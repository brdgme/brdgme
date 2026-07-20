//! Phase 13: integration tests for the NATS/JetStream bot eventing flow,
//! against a real NATS server with JetStream (see docs/superpowers/plans/2026-07-05-13-nats-bot-eventing.md).
//! The LLM is out of scope here (the bot process owns that call) - these
//! tests exercise the monolith side: publishing `bot.turn`, consuming
//! `bot.command` -> `execute_command` -> DB commit, the stale-state-conflict
//! re-publish path, the turn-level attempt limit, and exactly-once delivery
//! across two fetchers on the same durable consumer.
//!
//! Tests share the real `BOT` stream/`bot-turn`/`bot-command` durable
//! consumers (JetStream forbids a second consumer with an overlapping
//! filter, so each test can't get its own isolated stream the way
//! `sqlx::test` gives an isolated DB) - `#[serial]` forces them to run one at
//! a time, and each test only ever asserts on messages matching its own
//! game_id(s), discarding (acking) anything else as stale leftovers from a
//! prior run.

use axum::{Json, Router, routing::post};
use brdgme_cmd::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};
use futures_util::StreamExt;
use serial_test::serial;
use sqlx::PgPool;
use std::collections::HashSet;
use std::future::Future;
use std::time::Duration;
use tokio::net::TcpListener;
use uuid::Uuid;

use web::db::{self, CreateGameOpts};
use web::game::{handle_bot_command_event, trigger_bot_turns};
use web::models::user::User;
use web::nats::{self, BotCommandEvent, BotTurnEvent};
use web::websocket::GameBroadcaster;

fn now() -> time::PrimitiveDateTime {
    let t = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(t.date(), t.time())
}

async fn make_jetstream() -> async_nats::jetstream::Context {
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let js = nats::connect(&nats_url).await.expect("nats connect");
    nats::ensure_stream_and_consumers(&js)
        .await
        .expect("nats stream/consumers");
    js
}

async fn make_broadcaster() -> GameBroadcaster {
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let client = async_nats::connect(&nats_url).await.unwrap();
    GameBroadcaster::new(client)
}

/// Async variant of the mock game service (the in-tree unit tests only need
/// a sync handler; the conflict test needs the handler to perform its own
/// DB write between execute_command's read and its write).
async fn spawn_async_mock_game_service<F, Fut>(handler: F) -> String
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    let handler = std::sync::Arc::new(handler);
    let app = Router::new().route(
        "/",
        post(move |Json(payload): Json<Request>| {
            let handler = handler.clone();
            async move { Json(handler(payload).await) }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_mock_game_service<F>(handler: F) -> String
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    spawn_async_mock_game_service(move |req| {
        let resp = handler(req);
        async move { resp }
    })
    .await
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
        "1.0.0",
        uri
    )
    .fetch_one(pool)
    .await
    .unwrap()
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

/// One human player (position 0, on turn) plus one bot player (position 1),
/// pointed at `uri`.
async fn make_game_with_human_and_bot(pool: &PgPool, uri: &str) -> Uuid {
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
        },
    )
    .await
    .unwrap();
    game.id
}

/// `create_game_with_users_tx` shuffles seating order (`slots.shuffle`), so
/// the bot player's position in a `make_game_with_human_and_bot` game is
/// random - callers that need to address the bot specifically must look its
/// position up rather than assuming it's 1.
async fn bot_position(pool: &PgPool, game_id: Uuid) -> i32 {
    sqlx::query_scalar!(
        "SELECT position FROM game_players WHERE game_id = $1 AND game_bot_id IS NOT NULL",
        game_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
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

/// Fetches up to `max` messages from a `bot-turn` pull consumer within
/// `timeout`, acking every one (whether or not it belongs to this test) and
/// returning only the `BotTurnEvent`s for `game_id` - other tests' stale
/// leftovers are discarded rather than left to accumulate.
async fn drain_bot_turn_events(
    consumer: &async_nats::jetstream::consumer::PullConsumer,
    game_id: Uuid,
    max: usize,
    timeout: Duration,
) -> Vec<BotTurnEvent> {
    let mut matched = Vec::new();
    let mut messages = consumer
        .batch()
        .max_messages(max)
        .expires(timeout)
        .messages()
        .await
        .unwrap();
    while let Some(Ok(message)) = messages.next().await {
        let event: BotTurnEvent = serde_json::from_slice(&message.payload).unwrap();
        message.ack().await.unwrap();
        if event.game_id == game_id {
            matched.push(event);
        }
    }
    matched
}

#[sqlx::test]
#[serial]
async fn bot_turn_published_on_turn_change(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let uri = spawn_mock_game_service(|_req| play_response("s", vec![0], true)).await;
    let game_id = make_game_with_human_and_bot(&pool, &uri).await;
    let bot_pos = bot_position(&pool, game_id).await;

    // Flip the bot onto turn directly, mirroring what a real command
    // execution would leave behind.
    sqlx::query!(
        "UPDATE game_players SET is_turn = (position = $2) WHERE game_id = $1",
        game_id,
        bot_pos
    )
    .execute(&pool)
    .await
    .unwrap();

    trigger_bot_turns(&pool, &jetstream, game_id).await;

    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer = stream
        .get_or_create_consumer(
            nats::CONSUMER_TURN,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(nats::CONSUMER_TURN.to_string()),
                filter_subject: nats::SUBJECT_TURN.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let events = drain_bot_turn_events(&consumer, game_id, 20, Duration::from_secs(5)).await;
    assert_eq!(events.len(), 1, "expected exactly one bot.turn event");
    assert_eq!(events[0].game_id, game_id);
    assert_eq!(events[0].player_position, bot_pos);
    assert_eq!(events[0].bot_name, "easy");
    assert_eq!(events[0].attempt, 0);
}

#[sqlx::test]
#[serial]
async fn bot_command_consumed_executes_and_commits(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;
    let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
    let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;

    let event = BotCommandEvent {
        game_id,
        player_position: 0,
        command: "abc".to_string(),
        attempt: 0,
    };
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &event).await;

    let ge = db::find_game_extended(&pool, game_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ge.game.game_state, "new_state");
    let player1 = ge
        .game_players
        .iter()
        .find(|p| p.game_player.position == 1)
        .unwrap();
    assert!(player1.game_player.is_turn);
}

#[sqlx::test]
#[serial]
async fn stale_conflict_republishes_bot_turn_with_incremented_attempt(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let uri = format!("http://{}", addr);
    // publish_bot_turns only re-publishes for players with a bot attached, so
    // the conflicting command has to be attributed to the bot player - a
    // real conflict can only ever originate from a `bot.command` in the
    // first place.
    let game_id = make_game_with_human_and_bot(&pool, &uri).await;
    let bot_pos = bot_position(&pool, game_id).await;
    sqlx::query!(
        "UPDATE game_players SET is_turn = (position = $2) WHERE game_id = $1",
        game_id,
        bot_pos
    )
    .execute(&pool)
    .await
    .unwrap();

    // The mock game service simulates another writer landing a change to the
    // game between execute_command's read and its own write, so
    // update_game_command_success's optimistic-concurrency check fails.
    let pool_for_handler = pool.clone();
    let app = Router::new().route(
        "/",
        post(move |Json(_req): Json<Request>| {
            let pool = pool_for_handler.clone();
            async move {
                sqlx::query!("UPDATE games SET updated_at = NOW() WHERE id = $1", game_id)
                    .execute(&pool)
                    .await
                    .unwrap();
                Json(play_response("new_state", vec![0], true))
            }
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let event = BotCommandEvent {
        game_id,
        player_position: bot_pos,
        command: "abc".to_string(),
        attempt: 0,
    };
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &event).await;

    // The game must be untouched (the conflicting write was rejected)...
    let ge = db::find_game_extended(&pool, game_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ge.game.game_state, "initial_state");

    // ...and bot.turn must have been re-published with attempt incremented.
    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer = stream
        .get_or_create_consumer(
            nats::CONSUMER_TURN,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(nats::CONSUMER_TURN.to_string()),
                filter_subject: nats::SUBJECT_TURN.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let events = drain_bot_turn_events(&consumer, game_id, 20, Duration::from_secs(5)).await;
    assert_eq!(
        events.len(),
        1,
        "expected exactly one re-published bot.turn event"
    );
    assert_eq!(events[0].attempt, 1);
}

#[sqlx::test]
#[serial]
async fn attempt_limit_exhaustion_gives_up(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let uri = format!("http://{}", addr);
    let game_id = make_game_with_human_and_bot(&pool, &uri).await;
    let bot_pos = bot_position(&pool, game_id).await;
    sqlx::query!(
        "UPDATE game_players SET is_turn = (position = $2) WHERE game_id = $1",
        game_id,
        bot_pos
    )
    .execute(&pool)
    .await
    .unwrap();

    // Always conflicts, no matter how many times it's called.
    let pool_for_handler = pool.clone();
    let app = Router::new().route(
        "/",
        post(move |Json(_req): Json<Request>| {
            let pool = pool_for_handler.clone();
            async move {
                sqlx::query!("UPDATE games SET updated_at = NOW() WHERE id = $1", game_id)
                    .execute(&pool)
                    .await
                    .unwrap();
                Json(play_response("new_state", vec![0], true))
            }
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer = stream
        .get_or_create_consumer(
            nats::CONSUMER_TURN,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(nats::CONSUMER_TURN.to_string()),
                filter_subject: nats::SUBJECT_TURN.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Drive the conflict/re-publish cycle by hand: each call's resulting
    // bot.turn attempt feeds the next simulated bot.command's attempt,
    // exactly like the bot round-tripping the attempt counter would.
    let mut attempt = 0;
    for _ in 0..(nats::MAX_TURN_ATTEMPTS + 1) {
        let event = BotCommandEvent {
            game_id,
            player_position: bot_pos,
            command: "abc".to_string(),
            attempt,
        };
        let _ =
            handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &event).await;

        if attempt >= nats::MAX_TURN_ATTEMPTS {
            // Final attempt: must give up, no further bot.turn published.
            let events =
                drain_bot_turn_events(&consumer, game_id, 20, Duration::from_secs(2)).await;
            assert!(
                events.is_empty(),
                "expected no bot.turn re-publish after exhausting attempts, got {:?}",
                events
            );
            break;
        }

        let events = drain_bot_turn_events(&consumer, game_id, 20, Duration::from_secs(5)).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attempt, attempt + 1);
        attempt = events[0].attempt;
    }
}

#[sqlx::test]
#[serial]
async fn bot_command_delivered_exactly_once_across_two_fetchers(pool: PgPool) {
    let _pool = pool; // unused - this test only needs JetStream.
    let jetstream = make_jetstream().await;

    let marker = Uuid::new_v4();
    const N: usize = 10;
    let mut expected_game_ids = HashSet::new();
    for _ in 0..N {
        let game_id = Uuid::new_v4();
        expected_game_ids.insert(game_id);
        let event = BotCommandEvent {
            game_id,
            player_position: 0,
            command: format!("marker:{}", marker),
            attempt: 0,
        };
        let payload = serde_json::to_vec(&event).unwrap();
        jetstream
            .publish(nats::SUBJECT_COMMAND, payload.into())
            .await
            .unwrap()
            .await
            .unwrap();
    }

    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer_a = stream
        .get_or_create_consumer(
            nats::CONSUMER_COMMAND,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(nats::CONSUMER_COMMAND.to_string()),
                filter_subject: nats::SUBJECT_COMMAND.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let consumer_b = stream
        .get_or_create_consumer::<async_nats::jetstream::consumer::pull::Config>(
            nats::CONSUMER_COMMAND,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(nats::CONSUMER_COMMAND.to_string()),
                filter_subject: nats::SUBJECT_COMMAND.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn fetch_matching(
        consumer: async_nats::jetstream::consumer::PullConsumer,
        expected: HashSet<Uuid>,
        marker: Uuid,
    ) -> Vec<Uuid> {
        let marker_tag = format!("marker:{}", marker);
        let mut seen = Vec::new();
        // Several short fetch rounds rather than one long one, so both
        // fetchers get a fair chance to compete for messages concurrently.
        for _ in 0..20 {
            if seen.len() >= expected.len() {
                break;
            }
            let mut messages = consumer
                .batch()
                .max_messages(expected.len())
                .expires(Duration::from_millis(500))
                .messages()
                .await
                .unwrap();
            while let Some(Ok(message)) = messages.next().await {
                let event: BotCommandEvent = serde_json::from_slice(&message.payload).unwrap();
                message.ack().await.unwrap();
                if event.command == marker_tag && expected.contains(&event.game_id) {
                    seen.push(event.game_id);
                }
            }
        }
        seen
    }

    let (a, b) = tokio::join!(
        fetch_matching(consumer_a, expected_game_ids.clone(), marker),
        fetch_matching(consumer_b, expected_game_ids.clone(), marker)
    );

    let mut all: Vec<Uuid> = a.into_iter().chain(b).collect();
    all.sort();
    let mut expected_sorted: Vec<Uuid> = expected_game_ids.into_iter().collect();
    expected_sorted.sort();
    assert_eq!(
        all, expected_sorted,
        "every published bot.command must be delivered exactly once across both fetchers"
    );
}
