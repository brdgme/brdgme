#[cfg(feature = "ssr")]
pub use brdgme_game_client as client;
#[cfg(feature = "ssr")]
pub mod export;
#[cfg(feature = "ssr")]
pub mod import;
pub mod placing;
pub mod server_fns;

/// The fields of a game service `Status` used to update
/// `game_players`/`games` rows, split out by `status_fields`.
#[cfg(feature = "ssr")]
pub struct StatusUpdate {
    pub is_finished: bool,
    pub whose_turn: Vec<usize>,
    pub eliminated: Vec<usize>,
    pub placings: Vec<usize>,
}

/// Splits a game service `Status` into the `StatusUpdate` fields used to
/// update `game_players`/`games` rows. Shared by every command flow that
/// calls the game service and then writes the resulting status back to the
/// DB.
#[cfg(feature = "ssr")]
pub fn status_fields(status: brdgme_game::Status) -> StatusUpdate {
    use brdgme_game::Status;
    match status {
        Status::Active {
            whose_turn,
            eliminated,
        } => StatusUpdate {
            is_finished: false,
            whose_turn,
            eliminated,
            placings: vec![],
        },
        Status::Finished { placings, .. } => StatusUpdate {
            is_finished: true,
            whose_turn: vec![],
            eliminated: vec![],
            placings,
        },
    }
}

/// Broadcasts the skinny game-update signal and triggers any bots whose turn
/// it now is. Shared epilogue for every command flow that mutates a game and
/// then needs to notify watchers/bots. The broadcast is unconditional; only
/// the bot trigger depends on a DB read.
#[cfg(feature = "ssr")]
pub async fn broadcast_and_trigger(
    pool: &sqlx::PgPool,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
) {
    // R-20 / I2: test-only tap recording that the broadcast (and thus the
    // `bot.turn` publish that can trigger a fast bot move) happened. Ordered
    // against the mail-send tap in `email::outbound::test_events`, this lets a
    // test prove notify-before-broadcast directly - the guard that stops the
    // start-path notify and the bot-command notify from double-mailing.
    #[cfg(all(test, feature = "ssr"))]
    crate::email::outbound::test_events::record_broadcast(game_id);
    broadcaster.broadcast_game_update(game_id).await;
    trigger_bot_turns(pool, jetstream, game_id).await;
}

/// Distinguishes a stale-state conflict (the game changed under the bot
/// between validation and commit - the caller should re-publish `bot.turn`
/// with an incremented attempt counter) from every other failure (the
/// caller should give up and log).
#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum ExecuteCommandError {
    #[error("stale state conflict")]
    Conflict,
    /// The game rejected the command (e.g. "expected buy or done") - user
    /// input error, not a server fault. submit_command renders it inline.
    #[error("{0}")]
    UserError(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// On success returns the pre-command `GameExtended` it loaded for its own
/// guards, so the caller can diff it in `email::notify::notify_game_emails`
/// without a second read that could silently fail (wd F8, wfe F42).
#[cfg(feature = "ssr")]
pub async fn execute_command(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
    player_position: usize,
    command: String,
) -> Result<crate::db::GameExtended, ExecuteCommandError> {
    use brdgme_cmd::api::{Request, Response};

    let ge = crate::db::find_game_extended(pool, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Game not found"))?;

    if ge.game.is_finished {
        return Err(anyhow::anyhow!("Game is already finished").into());
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.position as usize == player_position)
        .ok_or_else(|| anyhow::anyhow!("Invalid player position"))?;

    if !player.game_player.is_turn {
        return Err(anyhow::anyhow!("Not your turn").into());
    }

    let names: Vec<String> = ge
        .game_players
        .iter()
        .map(|p| p.name().to_string())
        .collect();

    let resp = client::request(
        http_client,
        &ge.game_version.uri,
        &ge.game_version.name,
        &Request::Play {
            player: player.game_player.position as usize,
            game: ge.game.game_state.clone(),
            command,
            names,
        },
    )
    .await
    .map_err(|e| match e {
        client::GameClientError::UserError { message } => ExecuteCommandError::UserError(message),
        e => anyhow::Error::from(e).into(),
    })?;

    let (game_response, logs, can_undo, remaining_input) = match resp {
        Response::Play {
            game,
            logs,
            can_undo,
            remaining_input,
            ..
        } => (game, logs, can_undo, remaining_input),
        Response::UserError { message } => return Err(ExecuteCommandError::UserError(message)),
        _ => return Err(anyhow::anyhow!("Unexpected response from game service").into()),
    };

    if !remaining_input.trim().is_empty() {
        return Err(ExecuteCommandError::UserError(format!(
            "Unexpected input: {}",
            remaining_input.trim()
        )));
    }

    let prev_game_state = ge.game.game_state.clone();
    let status = status_fields(game_response.status);

    if let Err(e) = crate::db::update_game_command_success(
        pool,
        game_id,
        player.game_player.id,
        &prev_game_state,
        &game_response.state,
        can_undo,
        &status,
        &game_response.points,
        ge.game.updated_at,
        logs,
    )
    .await
    {
        if e.downcast_ref::<crate::db::StaleStateConflict>().is_some() {
            return Err(ExecuteCommandError::Conflict);
        }
        return Err(e.into());
    }

    broadcast_and_trigger(pool, broadcaster, jetstream, game_id).await;
    Ok(ge)
}

/// Publishes a `bot.turn` event (attempt 0) for every bot player whose turn
/// it currently is. The bot picks these up from the `bot-turn` durable
/// consumer; the monolith never talks to the bot directly. Gives up with a
/// warn log if the bot-turn query fails.
#[cfg(feature = "ssr")]
pub async fn trigger_bot_turns(
    pool: &sqlx::PgPool,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
) {
    match crate::db::find_bot_turns(pool, game_id).await {
        Ok(turns) => publish_bot_turns(jetstream, game_id, &turns, 0).await,
        Err(e) => {
            tracing::error!(%game_id, "Failed to query bot turns: {}", e);
            axum_prometheus::metrics::counter!("bot_turn_publish_failures_total").increment(1);
        }
    }
}

/// Stable `Nats-Msg-Id` for a `bot.turn` publish. Derived from the turn state
/// plus the retry `attempt` so identical re-publishes of the same
/// (game, position, updated_at, attempt) collapse inside the stream's duplicate
/// window, while a real turn change bumps `updated_at` and a deliberate retry
/// (the invalid-command re-publish, which writes nothing and so leaves
/// `updated_at` unchanged) bumps `attempt` - each getting a fresh id so the
/// retry is delivered rather than deduped away (R-15 / F-105, F-2).
#[cfg(feature = "ssr")]
fn bot_turn_message_id(
    game_id: uuid::Uuid,
    position: i32,
    updated_at: &str,
    attempt: i32,
) -> String {
    format!("{}:{}:{}:{}", game_id, position, updated_at, attempt)
}

/// Shared by `trigger_bot_turns` (attempt 0, fresh turns) and the
/// `bot.command` consumer (attempt N, re-publish after a stale-state
/// conflict).
#[cfg(feature = "ssr")]
pub(crate) async fn publish_bot_turns(
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
    turns: &[crate::db::BotTurn],
    attempt: i32,
) {
    for turn in turns {
        tracing::info!(
            %game_id,
            position = turn.position,
            bot_name = %turn.bot_name,
            attempt,
            "Publishing bot.turn"
        );
        let event = crate::nats::BotTurnEvent {
            game_id,
            player_position: turn.position,
            bot_name: turn.bot_name.clone(),
            attempt,
        };
        let payload = match serde_json::to_vec(&event) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(%game_id, "Failed to serialize bot.turn event: {}", e);
                continue;
            }
        };
        let mut headers = async_nats::HeaderMap::new();
        sentry::configure_scope(|scope| {
            if let Some(span) = scope.get_span() {
                for (k, v) in span.iter_headers() {
                    headers.insert(k, v);
                }
            }
        });
        // `games.updated_at` is stored without an offset (`PrimitiveDateTime`)
        // and treated as UTC throughout, so attach UTC to satisfy `Iso8601`'s
        // offset component - a bare `PrimitiveDateTime` is a compile-time error
        // here, not a runtime `Result`.
        let updated_at = turn
            .updated_at
            .assume_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_default();
        let message = async_nats::jetstream::message::PublishMessage::build()
            .payload(payload.into())
            .headers(headers)
            .message_id(bot_turn_message_id(
                game_id,
                turn.position,
                &updated_at,
                attempt,
            ));
        match jetstream
            .send_publish(crate::nats::SUBJECT_TURN, message)
            .await
        {
            // The outer `.await` only confirms the message was sent; the
            // inner one waits for JetStream's persistence ack so a publish
            // that returns `Ok` is actually durable in the stream.
            Ok(ack) => {
                if let Err(e) = ack.await {
                    tracing::error!(%game_id, "bot.turn publish not acked: {}", e);
                    axum_prometheus::metrics::counter!("bot_turn_publish_failures_total")
                        .increment(1);
                }
            }
            Err(e) => {
                tracing::error!(%game_id, "Failed to publish bot.turn: {}", e);
                axum_prometheus::metrics::counter!("bot_turn_publish_failures_total").increment(1);
            }
        }
    }
}

/// Pulls `bot.command` events one at a time from the durable `bot-command`
/// consumer and applies them via `execute_command`. Runs until `shutdown` is
/// cancelled (R-11 / F-109) or the message stream ends; multiple monolith
/// replicas can run this concurrently since JetStream hands each message to
/// exactly one fetcher.
#[cfg(feature = "ssr")]
pub async fn run_bot_command_consumer(
    pool: sqlx::PgPool,
    http_client: reqwest::Client,
    broadcaster: crate::websocket::GameBroadcaster,
    jetstream: async_nats::jetstream::Context,
    resend: Option<resend_rs::Resend>,
    shutdown: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    let consumer: async_nats::jetstream::consumer::PullConsumer = jetstream
        .get_consumer_from_stream(crate::nats::CONSUMER_COMMAND, crate::nats::STREAM_NAME)
        .await?;
    let messages = consumer.messages().await?;

    let handler = {
        let pool = pool.clone();
        let http_client = http_client.clone();
        let broadcaster = broadcaster.clone();
        let jetstream = jetstream.clone();
        let resend = resend.clone();
        move |message: async_nats::jetstream::Message| {
            let pool = pool.clone();
            let http_client = http_client.clone();
            let broadcaster = broadcaster.clone();
            let jetstream = jetstream.clone();
            let resend = resend.clone();
            async move {
                process_bot_command_message(
                    message,
                    &pool,
                    &http_client,
                    &broadcaster,
                    &jetstream,
                    &resend,
                )
                .await
            }
        }
    };

    run_bot_command_consume_loop(shutdown, messages, handler).await
}

/// The bot-command consume loop, generic over the message stream so the
/// shutdown path can be exercised without a live NATS connection (R-11). Each
/// pulled message is handed to `handle`; the loop returns promptly when
/// `shutdown` is cancelled - including while parked waiting for the next
/// message - or when the stream ends.
#[cfg(feature = "ssr")]
async fn run_bot_command_consume_loop<S, E, H, HFut>(
    shutdown: tokio_util::sync::CancellationToken,
    mut messages: S,
    mut handle: H,
) -> anyhow::Result<()>
where
    S: futures_util::Stream<Item = Result<async_nats::jetstream::Message, E>> + Unpin + Send,
    E: std::fmt::Display + Send,
    H: FnMut(async_nats::jetstream::Message) -> HFut + Send,
    HFut: std::future::Future<Output = ()> + Send + 'static,
{
    use futures_util::StreamExt;

    loop {
        let next = tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("bot-command consumer shutdown signalled; stopping consume loop");
                return Ok(());
            }
            next = messages.next() => next,
        };
        let Some(message) = next else {
            return Ok(());
        };
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to pull bot.command message: {}", e);
                continue;
            }
        };
        handle(message).await;
    }
}

/// Parses, applies, and acks/terms a single pulled `bot.command` message
/// (the per-message body of `run_bot_command_consumer`, split out so the
/// consume loop can stay generic over the stream - R-11).
#[cfg(feature = "ssr")]
async fn process_bot_command_message(
    message: async_nats::jetstream::Message,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    resend: &Option<resend_rs::Resend>,
) {
    let event: crate::nats::BotCommandEvent = match serde_json::from_slice(&message.payload) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to parse bot.command payload: {}", e);
            if let Err(e) = message.ack().await {
                tracing::warn!("Failed to ack unparseable bot.command message: {}", e);
            }
            return;
        }
    };

    let outcome =
        handle_bot_command_event(pool, http_client, broadcaster, jetstream, resend, &event).await;

    match outcome {
        // `handle_bot_command_event` never actually returns
        // `Conflict` (it resolves conflicts internally by re-publishing
        // `bot.turn` or, on exhaustion, giving up), but ack it too if it
        // ever did - nothing more is going to happen with this message.
        Ok(()) | Err(ExecuteCommandError::Conflict) => {
            if let Err(e) = message.ack().await {
                tracing::warn!(game_id = %event.game_id, "Failed to ack bot.command message: {}", e);
            }
        }
        Err(ExecuteCommandError::UserError(_)) => {
            // Unreachable for the `handle_bot_command_event` path: it now
            // resolves `UserError` internally (re-publishing `bot.turn` or,
            // on exhaustion, giving up) and returns `Ok(())`. Kept so any
            // future caller that does surface a `UserError` still acks it -
            // a game-rejected command never succeeds on redelivery.
            tracing::warn!(
                game_id = %event.game_id,
                "Acking bot.command message rejected by the game (not transient)"
            );
            if let Err(e) = message.ack().await {
                tracing::warn!(game_id = %event.game_id, "Failed to ack bot.command message: {}", e);
            }
        }
        Err(ExecuteCommandError::Other(_)) => match message.info() {
            Ok(info) if info.delivered >= crate::nats::MAX_DELIVER => {
                if let Err(e) = message.ack_with(async_nats::jetstream::AckKind::Term).await {
                    tracing::warn!(game_id = %event.game_id, "Failed to term bot.command message: {}", e);
                }
                tracing::error!(
                    game_id = %event.game_id,
                    delivered = info.delivered,
                    "Terminating bot.command message after exhausting max_deliver"
                );
                axum_prometheus::metrics::counter!("bot_command_terminated_total").increment(1);
            }
            Ok(info) => {
                // Transient failure: Nak with a short exponential backoff so
                // the server redelivers in seconds instead of waiting out the
                // full 5-minute ack_wait. `delivered` is 1-based, so
                // 2^delivered yields 2s then 4s for deliveries 1 and 2;
                // delivery 3 hits the term branch above (R-15 / F-101).
                let backoff = std::time::Duration::from_secs(
                    2u64.saturating_pow(info.delivered.max(1) as u32),
                );
                tracing::warn!(
                    game_id = %event.game_id,
                    delivered = info.delivered,
                    backoff_secs = backoff.as_secs(),
                    "Naking bot.command message for delayed redelivery after transient failure"
                );
                if let Err(e) = message
                    .ack_with(async_nats::jetstream::AckKind::Nak(Some(backoff)))
                    .await
                {
                    tracing::warn!(game_id = %event.game_id, "Failed to nak bot.command message: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!(
                    game_id = %event.game_id,
                    "Leaving bot.command message unacked for redelivery (failed to read delivery info: {})",
                    e
                );
            }
        },
    }
}

/// Applies a single `bot.command` event: run `execute_command`, and on a
/// stale-state conflict re-publish `bot.turn` with the attempt counter
/// incremented (up to `MAX_TURN_ATTEMPTS` re-publishes total - `event.attempt`
/// echoes the `bot.turn` event's own counter, so this survives across the
/// bot round-trip rather than resetting to 0 every time). Split out from
/// `run_bot_command_consumer` so it can be exercised directly in tests
/// without needing to drive the full pull loop.
#[cfg(feature = "ssr")]
pub async fn handle_bot_command_event(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    resend: &Option<resend_rs::Resend>,
    event: &crate::nats::BotCommandEvent,
) -> Result<(), ExecuteCommandError> {
    let attempt = event.attempt;
    let result = execute_command(
        pool,
        http_client,
        broadcaster,
        jetstream,
        event.game_id,
        event.player_position as usize,
        event.command.clone(),
    )
    .await;

    match result {
        Ok(before) => {
            tracing::info!(game_id = %event.game_id, position = event.player_position, "Bot command applied");
            crate::email::notify::notify_game_emails(
                resend.as_ref(),
                pool,
                http_client,
                event.game_id,
                Some(before),
            )
            .await;
            Ok(())
        }
        Err(ExecuteCommandError::Conflict) => {
            if attempt >= crate::nats::MAX_TURN_ATTEMPTS {
                tracing::error!(
                    game_id = %event.game_id,
                    position = event.player_position,
                    attempt,
                    "Bot turn exhausted state-conflict retries, giving up"
                );
                axum_prometheus::metrics::counter!("bot_turn_wedge_total").increment(1);
                // Nothing more will happen for this game/attempt, so treat
                // exhaustion as a successful outcome for acking purposes.
                return Ok(());
            }
            tracing::warn!(
                game_id = %event.game_id,
                position = event.player_position,
                attempt,
                "Stale state conflict applying bot command, re-publishing bot.turn"
            );
            match crate::db::find_bot_turns(pool, event.game_id).await {
                Ok(turns) => {
                    let conflicting: Vec<crate::db::BotTurn> = turns
                        .into_iter()
                        .filter(|t| t.position == event.player_position)
                        .collect();
                    publish_bot_turns(jetstream, event.game_id, &conflicting, attempt + 1).await;
                }
                Err(e) => {
                    tracing::error!(game_id = %event.game_id, "Failed to query bot turns while re-publishing bot.turn: {}", e);
                    axum_prometheus::metrics::counter!("bot_turn_publish_failures_total")
                        .increment(1);
                }
            }
            // Conflict is re-published as a fresh bot.turn; the original
            // bot.command message is done, so ack it.
            Ok(())
        }
        Err(ExecuteCommandError::UserError(msg)) => {
            tracing::warn!(
                game_id = %event.game_id,
                position = event.player_position,
                "Bot command rejected by game: {}",
                msg
            );
            if attempt >= crate::nats::MAX_TURN_ATTEMPTS {
                tracing::error!(
                    game_id = %event.game_id,
                    position = event.player_position,
                    attempt,
                    "Bot turn exhausted user-error retries, giving up"
                );
                axum_prometheus::metrics::counter!("bot_turn_wedge_total").increment(1);
                return Ok(());
            }
            match crate::db::find_bot_turns(pool, event.game_id).await {
                Ok(turns) => {
                    let conflicting: Vec<crate::db::BotTurn> = turns
                        .into_iter()
                        .filter(|t| t.position == event.player_position)
                        .collect();
                    publish_bot_turns(jetstream, event.game_id, &conflicting, attempt + 1).await;
                }
                Err(e) => {
                    tracing::error!(game_id = %event.game_id, "Failed to query bot turns while re-publishing bot.turn: {}", e);
                    axum_prometheus::metrics::counter!("bot_turn_publish_failures_total")
                        .increment(1);
                }
            }
            Ok(())
        }
        Err(ExecuteCommandError::Other(e)) => {
            tracing::warn!(
                game_id = %event.game_id,
                position = event.player_position,
                "Bot command rejected: {}",
                e
            );
            Err(ExecuteCommandError::Other(e))
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::{self, CreateGameOpts};
    use crate::models::user::User;
    use axum::{Json, Router, routing::post};
    use brdgme_cmd::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};
    use sqlx::PgPool;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use uuid::Uuid;

    fn now() -> time::PrimitiveDateTime {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    }

    /// F-2: the `bot.turn` dedup key must include the retry `attempt`. A
    /// deliberate retry after an invalid bot command re-publishes the identical
    /// (game, position, updated_at) at `attempt + 1` with no DB write in
    /// between, so an attempt-less key would dedup the retry away and wedge the
    /// game. Two publishes of the identical event (same attempt) must collide,
    /// while a higher attempt of the same turn state must get a fresh key.
    #[test]
    fn bot_turn_message_id_differs_by_attempt() {
        let game_id = Uuid::new_v4();
        let updated_at = "2026-08-01T12:34:56.789000000Z";
        let attempt0_a = bot_turn_message_id(game_id, 1, updated_at, 0);
        let attempt0_b = bot_turn_message_id(game_id, 1, updated_at, 0);
        let attempt1 = bot_turn_message_id(game_id, 1, updated_at, 1);
        assert_eq!(
            attempt0_a, attempt0_b,
            "identical events (same attempt) must share a key so duplicates dedup"
        );
        assert_ne!(
            attempt0_a, attempt1,
            "a retry at a higher attempt must get a fresh key so it is delivered"
        );
    }

    /// Starts an in-process mock game service that answers every request with
    /// whatever `handler` returns; mirrors the pattern in `brdgme_game_client`'s tests.
    pub(crate) async fn spawn_mock_game_service<F>(handler: F) -> String
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| {
                let handler = handler.clone();
                async move { Json(handler(payload)) }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    async fn make_user(pool: &PgPool, name: &str) -> User {
        sqlx::query_as!(
            User,
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
            Uuid::new_v4(),
            name,
            &Vec::<String>::new()
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn make_game_version(pool: &PgPool, uri: &str) -> Uuid {
        let game_type_id = sqlx::query_scalar!(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
            format!("Test Game {}", Uuid::new_v4()),
            &vec![2, 3, 4]
        )
        .fetch_one(pool)
        .await
        .unwrap();

        sqlx::query_scalar!(
            r#"INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
               VALUES ($1, $2, $3, true, false) RETURNING id"#,
            game_type_id,
            "test-v1",
            uri
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn make_broadcaster() -> crate::websocket::GameBroadcaster {
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = async_nats::connect(&nats_url).await.unwrap();
        crate::websocket::GameBroadcaster::new(client)
    }

    /// Connects to a real NATS server with the `BOT` stream/consumers ensured,
    /// mirroring the Postgres/Redis convention of pointing tests at a real
    /// service via an env var (defaults to the local dev NATS).
    pub(crate) async fn make_jetstream() -> async_nats::jetstream::Context {
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let js = crate::nats::connect(&nats_url).await.unwrap();
        crate::nats::ensure_stream_and_consumers(&js).await.unwrap();
        js
    }

    /// Two human players (position 0, 1), player 0 on turn, pointed at `uri`.
    async fn make_two_player_game(pool: &PgPool, uri: &str) -> (Uuid, User, User) {
        let p0 = make_user(pool, "p0").await;
        let p1 = make_user(pool, "p1").await;
        let game_version_id = make_game_version(pool, uri).await;
        let game = db::create_game_with_users(
            pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: p0.id,
                opponent_ids: &[p1.id],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        (game.id, p0, p1)
    }

    /// One human player (position 0, on turn) plus one bot player (position
    /// 1), pointed at `uri`.
    async fn make_game_with_human_and_bot(pool: &PgPool, uri: &str) -> (Uuid, User) {
        let p0 = make_user(pool, "p0").await;
        let game_version_id = make_game_version(pool, uri).await;
        let game = db::create_game_with_users(
            pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: p0.id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[db::BotSlot {
                    name: "Bot 0".to_string(),
                    bot_name: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();
        (game.id, p0)
    }

    fn play_response(state: &str, whose_turn: Vec<usize>, can_undo: bool) -> Response {
        Response::Play {
            game: GameResponse {
                state: state.to_string(),
                points: vec![0.0, 0.0],
                status: brdgme_game::Status::Active {
                    whose_turn,
                    eliminated: vec![],
                },
            },
            logs: vec![CliLog {
                content: "did a thing".to_string(),
                at: now(),
                public: true,
                to: vec![],
            }],
            can_undo,
            remaining_input: String::new(),
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: vec![
                PlayerRender {
                    player_state: "p0".to_string(),
                    render: "p0render".to_string(),
                    command_spec: None,
                },
                PlayerRender {
                    player_state: "p1".to_string(),
                    render: "p1render".to_string(),
                    command_spec: None,
                },
            ],
        }
    }

    #[sqlx::test]
    async fn happy_path_saves_state_and_advances_turn(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "new_state");
        assert!(!ge.game.is_finished);

        let logs = db::get_all_game_logs(&pool, game_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].body, "did a thing");

        let player0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let player1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(!player0.game_player.is_turn);
        assert!(player1.game_player.is_turn);
        assert_eq!(
            player0.game_player.undo_game_state.as_deref(),
            Some("initial_state")
        );
        assert!(player1.game_player.undo_game_state.is_none());
    }

    // wd F8 / wfe F42: the notification diff baseline must come from the load
    // execute_command already does, not from a second best-effort read whose
    // failure silently becomes "brand-new game".
    #[sqlx::test]
    async fn execute_command_returns_the_pre_command_snapshot(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let before = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        // The returned snapshot is the state BEFORE the command...
        assert_eq!(before.game.game_state, "initial_state");
        let before_p0 = before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert!(before_p0.game_player.is_turn, "p0 held the turn before");
        let before_p1 = before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(!before_p1.game_player.is_turn, "p1 did not hold it before");

        // ...while the DB now holds the state after it.
        let after = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.game.game_state, "new_state");
        assert!(
            after
                .game_players
                .iter()
                .find(|p| p.game_player.position == 1)
                .unwrap()
                .game_player
                .is_turn
        );
    }

    #[sqlx::test]
    async fn concurrent_write_conflict_returns_err_and_preserves_first_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        // Simulate two concurrent requests both reading the game before either
        // writes: capture the stale `updated_at` here, then let the first
        // request (a normal execute_command) win the race and land its write.
        let stale_ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        // The second request now tries to write using the state it read
        // before the first request's write landed - its expected_updated_at
        // is stale, so it must be rejected as a conflict rather than
        // silently overwriting the first write.
        let played_player_id = stale_ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap()
            .game_player
            .id;
        let result = db::update_game_command_success(
            &pool,
            game_id,
            played_player_id,
            "initial_state",
            "concurrent_conflict_state",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[0.0, 0.0],
            stale_ge.game.updated_at,
            vec![CliLog {
                content: "should never be persisted".to_string(),
                at: now(),
                public: true,
                to: vec![],
            }],
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "new_state");

        // The conflicting update's log insert must not have committed
        // outside the failed transaction: only the first, successful
        // execute_command's log should be present.
        let logs = db::get_all_game_logs(&pool, game_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].body, "did a thing");
    }

    #[sqlx::test]
    async fn not_players_turn_returns_err_and_leaves_game_unchanged(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            1, // not player 1's turn
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn finished_game_returns_err_and_leaves_game_unchanged(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        // Force the game to already be finished.
        sqlx::query!("UPDATE games SET is_finished = true WHERE id = $1", game_id)
            .execute(&pool)
            .await
            .unwrap();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn user_error_propagated_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::UserError {
            message: "invalid command".to_string(),
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        let err = result.unwrap_err();
        match err {
            ExecuteCommandError::UserError(msg) => {
                assert_eq!("invalid command", msg);
            }
            e => panic!("expected UserError, got {:?}", e),
        }
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn system_error_propagated_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::SystemError {
            message: "boom".to_string(),
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn remaining_input_returns_err_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| {
            let mut resp = play_response("new_state", vec![1], true);
            if let Response::Play {
                ref mut remaining_input,
                ..
            } = resp
            {
                *remaining_input = "extra".to_string();
            }
            resp
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn finished_status_persists_placings(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::Play {
            game: GameResponse {
                state: "final_state".to_string(),
                points: vec![1.0, 0.0],
                status: brdgme_game::Status::Finished {
                    placings: vec![0, 1],
                    stats: vec![],
                },
            },
            logs: vec![],
            can_undo: false,
            remaining_input: String::new(),
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: vec![
                PlayerRender {
                    player_state: "p0".to_string(),
                    render: "p0render".to_string(),
                    command_spec: None,
                },
                PlayerRender {
                    player_state: "p1".to_string(),
                    render: "p1render".to_string(),
                    command_spec: None,
                },
            ],
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert!(ge.game.is_finished);
        assert!(ge.game.finished_at.is_some());

        let player0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let player1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(player0.game_player.place, Some(0));
        assert_eq!(player1.game_player.place, Some(1));

        // Both players started at the DB default rating (1200), so the
        // winner (place 0) gains and the loser (place 1) loses the same
        // amount (K=32, equal ratings => +-16).
        assert_eq!(player0.game_player.rating_change, Some(16));
        assert_eq!(player1.game_player.rating_change, Some(-16));

        let winner_rating = sqlx::query_scalar!(
            "SELECT rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            ge.game_type.id,
            player0.user.as_ref().unwrap().id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let loser_rating = sqlx::query_scalar!(
            "SELECT rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            ge.game_type.id,
            player1.user.as_ref().unwrap().id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(winner_rating, 1216);
        assert_eq!(loser_rating, 1184);
    }

    #[sqlx::test]
    async fn finished_game_with_bot_player_is_not_rated(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::Play {
            game: GameResponse {
                state: "final_state".to_string(),
                points: vec![1.0, 0.0],
                status: brdgme_game::Status::Finished {
                    placings: vec![0, 1],
                    stats: vec![],
                },
            },
            logs: vec![],
            can_undo: false,
            remaining_input: String::new(),
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: vec![
                PlayerRender {
                    player_state: "p0".to_string(),
                    render: "p0render".to_string(),
                    command_spec: None,
                },
                PlayerRender {
                    player_state: "bot".to_string(),
                    render: "botrender".to_string(),
                    command_spec: None,
                },
            ],
        })
        .await;
        let (game_id, p0) = make_game_with_human_and_bot(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert!(ge.game.is_finished);

        // Two-player game with a bot: only one human player, so no pairwise
        // rating is possible. rating_change stays NULL for everyone.
        for p in &ge.game_players {
            assert_eq!(p.game_player.rating_change, None);
        }

        // create_game_with_users eagerly creates a game_type_users row for
        // every human player at the default rating (1200); a bot game must
        // leave that rating untouched rather than applying an ELO change.
        let rating = sqlx::query_scalar!(
            "SELECT rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            ge.game_type.id,
            p0.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rating, 1200);
    }

    #[sqlx::test]
    async fn trigger_bot_turns_noop_when_no_bot_players(pool: PgPool) {
        let jetstream = make_jetstream().await;
        let uri = spawn_mock_game_service(|_req| play_response("s", vec![0], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;

        // No-op, no panic: nothing to assert beyond "returns".
        trigger_bot_turns(&pool, &jetstream, game_id).await;
    }

    #[sqlx::test]
    #[ignore = "flaky NATS timing; see docs/changes/27-web-simplification/plan.md deferred item 2"]
    async fn broadcast_and_trigger_publishes_signal_for_missing_game(pool: PgPool) {
        use futures_util::StreamExt;

        let broadcaster = make_broadcaster().await;
        let jetstream = make_jetstream().await;
        let game_id = Uuid::new_v4();

        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = async_nats::connect(&nats_url).await.unwrap();
        let mut game_sub = client.subscribe(format!("game.{}", game_id)).await.unwrap();
        client.flush().await.unwrap();

        // The game id doesn't exist in the DB: the skinny signal must still
        // publish unconditionally, with only the bot trigger no-oping.
        broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;

        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), game_sub.next())
            .await
            .expect("timed out waiting for game.{id} message")
            .expect("game.{id} subscription ended unexpectedly");
        assert_eq!(msg.subject.as_str(), format!("game.{}", game_id));
    }

    #[tokio::test]
    async fn bot_command_consume_loop_exits_on_shutdown() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio_util::sync::CancellationToken;

        let shutdown = CancellationToken::new();
        // A stream that never yields: the loop parks in `messages.next()`, so
        // the shutdown arm is the only way out - this exercises the active
        // consume loop's shutdown path, not just the supervisor restart gap.
        let messages = futures_util::stream::pending::<
            Result<async_nats::jetstream::Message, async_nats::Error>,
        >();
        let handled = Arc::new(AtomicUsize::new(0));
        let handled_clone = handled.clone();
        let handler = move |_message: async_nats::jetstream::Message| {
            let handled = handled_clone.clone();
            async move {
                handled.fetch_add(1, Ordering::SeqCst);
            }
        };
        let task = tokio::spawn(run_bot_command_consume_loop(
            shutdown.clone(),
            messages,
            handler,
        ));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(handled.load(Ordering::SeqCst), 0);
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .expect("consume loop did not exit after shutdown")
            .unwrap()
            .unwrap();
        assert_eq!(
            handled.load(Ordering::SeqCst),
            0,
            "no message should be handled once shutdown is signalled"
        );
    }
}
