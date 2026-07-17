#[cfg(feature = "ssr")]
pub use brdgme_game_client as client;
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

#[cfg(feature = "ssr")]
pub async fn execute_command(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
    player_position: usize,
    command: String,
) -> Result<(), ExecuteCommandError> {
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
    .await?;

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
    Ok(())
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
        Err(e) => tracing::warn!(%game_id, "Failed to query bot turns: {}", e),
    }
}

/// Shared by `trigger_bot_turns` (attempt 0, fresh turns) and the
/// `bot.command` consumer (attempt N, re-publish after a stale-state
/// conflict).
#[cfg(feature = "ssr")]
async fn publish_bot_turns(
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
    turns: &[crate::db::BotTurn],
    attempt: i32,
) {
    for turn in turns {
        tracing::info!(
            %game_id,
            position = turn.position,
            difficulty = %turn.difficulty,
            attempt,
            "Publishing bot.turn"
        );
        let event = crate::nats::BotTurnEvent {
            game_id,
            player_position: turn.position,
            difficulty: turn.difficulty.clone(),
            attempt,
        };
        let payload = match serde_json::to_vec(&event) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(%game_id, "Failed to serialize bot.turn event: {}", e);
                continue;
            }
        };
        match jetstream
            .publish(crate::nats::SUBJECT_TURN, payload.into())
            .await
        {
            // The outer `.await` only confirms the message was sent; the
            // inner one waits for JetStream's persistence ack so a publish
            // that returns `Ok` is actually durable in the stream.
            Ok(ack) => {
                if let Err(e) = ack.await {
                    tracing::warn!(%game_id, "bot.turn publish not acked: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!(%game_id, "Failed to publish bot.turn: {}", e);
            }
        }
    }
}

/// Pulls `bot.command` events one at a time from the durable `bot-command`
/// consumer and applies them via `execute_command`. Runs for the lifetime of
/// the process; multiple monolith replicas can run this concurrently since
/// JetStream hands each message to exactly one fetcher.
#[cfg(feature = "ssr")]
pub async fn run_bot_command_consumer(
    pool: sqlx::PgPool,
    http_client: reqwest::Client,
    broadcaster: crate::websocket::GameBroadcaster,
    jetstream: async_nats::jetstream::Context,
) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let consumer: async_nats::jetstream::consumer::PullConsumer = jetstream
        .get_consumer_from_stream(crate::nats::CONSUMER_COMMAND, crate::nats::STREAM_NAME)
        .await?;
    let mut messages = consumer.messages().await?;

    while let Some(message) = messages.next().await {
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to pull bot.command message: {}", e);
                continue;
            }
        };
        let event: crate::nats::BotCommandEvent = match serde_json::from_slice(&message.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to parse bot.command payload: {}", e);
                if let Err(e) = message.ack().await {
                    tracing::warn!("Failed to ack unparseable bot.command message: {}", e);
                }
                continue;
            }
        };

        let outcome =
            handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &event).await;

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
                // A bot-issued command the game rejected as invalid will
                // never succeed on redelivery - ack it so it doesn't loop.
                tracing::warn!(
                    game_id = %event.game_id,
                    "Acking bot.command message rejected by the game (not transient)"
                );
                if let Err(e) = message.ack().await {
                    tracing::warn!(game_id = %event.game_id, "Failed to ack bot.command message: {}", e);
                }
            }
            Err(ExecuteCommandError::Other(_)) => {
                tracing::warn!(
                    game_id = %event.game_id,
                    "Leaving bot.command message unacked for redelivery after transient failure"
                );
            }
        }
    }

    Ok(())
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
        Ok(()) => {
            tracing::info!(game_id = %event.game_id, position = event.player_position, "Bot command applied");
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
                    publish_bot_turns(jetstream, event.game_id, &turns, attempt + 1).await;
                }
                Err(e) => {
                    tracing::warn!(game_id = %event.game_id, "Failed to query bot turns while re-publishing bot.turn: {}", e)
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
            Err(ExecuteCommandError::UserError(msg))
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
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING *",
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
            "1.0.0",
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
                    difficulty: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "initial_state",
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
        assert!(err.to_string().contains("invalid command"));
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

        // Games with a bot player are never rated: rating_change must stay
        // NULL for every player, human and bot alike.
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
    #[ignore = "flaky NATS timing; see docs/superpowers/plans/2026-07-07-27-web-simplification.md deferred item 2"]
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
}
