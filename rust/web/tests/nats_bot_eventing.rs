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
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use web::db::{self, CreateGameOpts};
use web::game::{handle_bot_command_event, run_bot_command_consumer, trigger_bot_turns};
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
        "test-v1",
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
            all_accepted: false,
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
            all_accepted: false,
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
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
        .await;

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
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
        .await;

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
            handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
                .await;

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

/// review ws F56: when a message exhausts max_deliver, the server emits a
/// MAX_DELIVERIES advisory on the subject our listener subscribes to, with
/// a payload our parser understands. Forces redeliveries with Nak (test-only;
/// production code never naks — WP-38 boundary).
#[sqlx::test]
#[serial]
async fn max_deliver_exhaustion_emits_parseable_advisory(pool: PgPool) {
    let _pool = pool;
    let jetstream = make_jetstream().await;
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let core_client = async_nats::connect(&nats_url).await.unwrap();
    let mut advisories = core_client
        .subscribe(nats::MAX_DELIVERIES_ADVISORY_SUBJECT)
        .await
        .unwrap();

    let marker_game_id = Uuid::new_v4();
    let event = BotCommandEvent {
        game_id: marker_game_id,
        player_position: 0,
        command: "advisory-test".to_string(),
        attempt: 0,
    };
    let ack = jetstream
        .publish(
            nats::SUBJECT_COMMAND,
            serde_json::to_vec(&event).unwrap().into(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
    let our_seq = ack.sequence;

    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer = stream
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
    let mut naks = 0;
    'outer: for _ in 0..10 {
        let mut messages = consumer
            .batch()
            .max_messages(20)
            .expires(Duration::from_millis(500))
            .messages()
            .await
            .unwrap();
        while let Some(Ok(message)) = messages.next().await {
            let ev: BotCommandEvent = serde_json::from_slice(&message.payload).unwrap();
            if ev.game_id == marker_game_id {
                message
                    .ack_with(async_nats::jetstream::AckKind::Nak(None))
                    .await
                    .unwrap();
                naks += 1;
                if naks >= 3 {
                    break 'outer;
                }
            } else {
                message.ack().await.unwrap();
            }
        }
    }
    assert_eq!(
        naks, 3,
        "expected to nak the marker message max_deliver times"
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let adv = loop {
        let remaining = deadline - tokio::time::Instant::now();
        let msg = tokio::time::timeout(remaining, advisories.next())
            .await
            .expect("timed out waiting for MAX_DELIVERIES advisory")
            .expect("advisory subscription ended");
        if let Some(adv) = nats::parse_max_deliveries_advisory(&msg.payload)
            && adv.stream_seq == our_seq
        {
            break adv;
        }
    };
    assert_eq!(adv.stream, nats::STREAM_NAME);
    assert_eq!(adv.consumer, nats::CONSUMER_COMMAND);
    assert_eq!(adv.deliveries, 3);

    let _ = stream.delete_message(our_seq).await;
}

/// One human (creator) plus two bot players, pointed at `uri`.
async fn make_game_with_two_bots(pool: &PgPool, uri: &str) -> Uuid {
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
            bot_slots: &[
                db::BotSlot {
                    name: "Bot A".to_string(),
                    bot_name: "easy".to_string(),
                },
                db::BotSlot {
                    name: "Bot B".to_string(),
                    bot_name: "easy".to_string(),
                },
            ],
            chat_id: None,
            game_state: "initial_state",
            all_accepted: false,
        },
    )
    .await
    .unwrap();
    game.id
}

#[sqlx::test]
#[serial]
async fn user_error_republishes_bot_turn_with_incremented_attempt(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    let uri = spawn_mock_game_service(|_req| Response::UserError {
        message: "invalid command".to_string(),
    })
    .await;
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

    let event = BotCommandEvent {
        game_id,
        player_position: bot_pos,
        command: "abc".to_string(),
        attempt: 0,
    };
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
        .await;

    let ge = db::find_game_extended(&pool, game_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ge.game.game_state, "initial_state");

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
    assert_eq!(events[0].player_position, bot_pos);
    assert_eq!(events[0].attempt, 1);
}

#[sqlx::test]
#[serial]
async fn user_error_attempt_limit_exhaustion_gives_up(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    let uri = spawn_mock_game_service(|_req| Response::UserError {
        message: "invalid command".to_string(),
    })
    .await;
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

    let mut attempt = 0;
    for _ in 0..(nats::MAX_TURN_ATTEMPTS + 1) {
        let event = BotCommandEvent {
            game_id,
            player_position: bot_pos,
            command: "abc".to_string(),
            attempt,
        };
        let _ =
            handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
                .await;

        if attempt >= nats::MAX_TURN_ATTEMPTS {
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

/// review wd F9: a stale-state conflict must re-publish bot.turn only for
/// the conflicting event's position, not fan out to every bot on turn.
#[sqlx::test]
#[serial]
async fn conflict_republish_targets_only_the_conflicting_bot(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let uri = format!("http://{}", addr);
    let game_id = make_game_with_two_bots(&pool, &uri).await;
    let bot_positions: Vec<i32> = sqlx::query_scalar!(
        "SELECT position FROM game_players WHERE game_id = $1 AND game_bot_id IS NOT NULL ORDER BY position",
        game_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(bot_positions.len(), 2);
    sqlx::query!(
        "UPDATE game_players SET is_turn = (position = ANY($2)) WHERE game_id = $1",
        game_id,
        &bot_positions
    )
    .execute(&pool)
    .await
    .unwrap();

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

    let conflicting_pos = bot_positions[0];
    let event = BotCommandEvent {
        game_id,
        player_position: conflicting_pos,
        command: "abc".to_string(),
        attempt: 0,
    };
    let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
        .await;

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
        "conflict must re-publish only the conflicting bot's turn, got {:?}",
        events
    );
    assert_eq!(events[0].player_position, conflicting_pos);
    assert_eq!(events[0].attempt, 1);
}

// ---------------------------------------------------------------------------
// R-15 (F-101 / F-102 / F-105): test-first coverage for the NATS delivery
// semantics remediation. These are written BEFORE the production fix and are
// EXPECTED TO FAIL against current behavior:
//
//   * `duplicate_bot_turn_publish_collapses_to_one_delivery` - today the same
//     turn published twice yields two stream messages (no `Nats-Msg-Id`, no
//     `duplicate_window`), so it observes two deliveries and fails the
//     "exactly one" assertion. Passes once publish sets a message id and the
//     stream sets a duplicate window.
//   * `transient_failure_redelivers_command_well_inside_ack_wait` - today a
//     transient (`Other`) failure leaves the message unacked, so redelivery
//     waits the full 5-minute `ack_wait`; the bounded wait sees a single
//     delivery and fails. Passes once the handler Naks with a short backoff.
//   * `ensure_stream_and_consumers_reconciles_drifted_config` - deliberately
//     drifts the live stream/consumer config away from the desired values, then
//     asserts `ensure_stream_and_consumers` restores them (including the
//     explicit 120s duplicate window). A `get_or_create_*` implementation would
//     leave the drifted values in place and fail; only real reconciliation
//     passes. This is a regression guard for the reconcile path, not a
//     red/green against the pre-fix default (the server already defaults the
//     window to 120s).
//
// Runtime verification is deferred to CI (a live NATS server is required); see
// docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md (R-15).
// ---------------------------------------------------------------------------

/// Best-effort drain of the shared `bot-command` consumer: fetches a bounded
/// batch and acks every message (matching or not) so leftovers from a prior
/// run don't accumulate. Mirrors `drain_bot_turn_events` for the command side.
async fn ack_all_bot_command(jetstream: &async_nats::jetstream::Context) {
    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let consumer = stream
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
    let mut messages = consumer
        .batch()
        .max_messages(100)
        .expires(Duration::from_millis(500))
        .messages()
        .await
        .unwrap();
    while let Some(Ok(message)) = messages.next().await {
        let _ = message.ack().await;
    }
}

/// F-105: the same bot turn can be published more than once for the identical
/// (game, position) - the `broadcast_and_trigger` path after a command and the
/// 15-minute reconciliation sweep both fire `trigger_bot_turns` before the bot
/// has moved. Without dedup each publish becomes a separate stream message and
/// a separate (expensive) LLM completion. Publishing the exact same turn twice
/// must collapse to exactly one delivery.
#[sqlx::test]
#[serial]
async fn duplicate_bot_turn_publish_collapses_to_one_delivery(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let uri = spawn_mock_game_service(|_req| play_response("s", vec![0], true)).await;
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

    // The identical turn is published twice: no command runs between the two
    // calls, so the game row (and thus any dedup key derived from it) is
    // unchanged - this is exactly the sweep-overlap duplicate.
    trigger_bot_turns(&pool, &jetstream, game_id).await;
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
    assert_eq!(
        events.len(),
        1,
        "the same bot turn published twice must collapse to exactly one delivery, got {:?}",
        events
    );
    assert_eq!(events[0].game_id, game_id);
    assert_eq!(events[0].player_position, bot_pos);
}

/// F-101: a `bot.command` that fails transiently (`ExecuteCommandError::Other`,
/// e.g. a game-service 5xx) must be retried on a cadence appropriate for prompt
/// processing - seconds, not the full 5-minute `ack_wait`. Drives the real
/// `run_bot_command_consumer` against a game service that always returns a
/// system error and asserts the single message is redelivered (processed more
/// than once) well inside the ack window.
#[sqlx::test]
#[serial]
async fn transient_failure_redelivers_command_well_inside_ack_wait(pool: PgPool) {
    let jetstream = make_jetstream().await;
    let http_client = reqwest::Client::new();
    let broadcaster = make_broadcaster().await;

    // Count every game-service call: one per delivery of the command, since a
    // system error performs no DB write and leaves position 0 on turn.
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_mock = calls.clone();
    let uri = spawn_mock_game_service(move |_req| {
        calls_for_mock.fetch_add(1, Ordering::SeqCst);
        Response::SystemError {
            message: "boom".to_string(),
        }
    })
    .await;
    let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;

    let shutdown = CancellationToken::new();
    let consumer_task = tokio::spawn(run_bot_command_consumer(
        pool.clone(),
        http_client,
        broadcaster,
        jetstream.clone(),
        None,
        shutdown.clone(),
    ));

    let event = BotCommandEvent {
        game_id,
        player_position: 0,
        command: "abc".to_string(),
        attempt: 0,
    };
    let ack = jetstream
        .publish(
            nats::SUBJECT_COMMAND,
            serde_json::to_vec(&event).unwrap().into(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
    let our_seq = ack.sequence;

    // Bound the wait well under the 5-minute ack_wait: a Nak-driven retry
    // redelivers in single-digit seconds, whereas an unacked message would not
    // reappear inside this window at all.
    const REDELIVERY_BOUND: Duration = Duration::from_secs(15);
    let deadline = tokio::time::Instant::now() + REDELIVERY_BOUND;
    let mut delivered = 0;
    while tokio::time::Instant::now() < deadline {
        delivered = calls.load(Ordering::SeqCst);
        if delivered >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Cleanup before asserting so it runs even on failure: stop the consumer,
    // delete our message by stream sequence (works whether it is pending-ack or
    // already termed), and best-effort drain any other leftovers.
    shutdown.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(5), consumer_task).await;
    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    let _ = stream.delete_message(our_seq).await;
    ack_all_bot_command(&jetstream).await;

    assert!(
        delivered >= 2,
        "a transiently failing bot.command must be redelivered well inside the \
         5-minute ack_wait (processed {} time(s) in {:?}); an unacked message \
         waiting out the full ack_wait is the F-101 failure mode",
        delivered,
        REDELIVERY_BOUND
    );
}

/// F-102 / F-105: `ensure_stream_and_consumers` must actively reconcile a
/// drifted server config back to the desired values, not merely create-if-absent
/// (a `get_or_create_*` would leave the drift in place). Deliberately skews the
/// live stream's `duplicate_window` and the `bot-turn` consumer's retry config,
/// re-runs `ensure_stream_and_consumers`, and asserts the explicit desired
/// values are restored - including the 120s duplicate window the dedup relies
/// on. Asserting the exact 120s (not just `> 0`) after forcing it to 1s is what
/// proves reconciliation rather than NATS 2.11's 2-minute server default.
#[sqlx::test]
#[serial]
async fn ensure_stream_and_consumers_reconciles_drifted_config(pool: PgPool) {
    let _pool = pool; // only JetStream is exercised here.
    let jetstream = make_jetstream().await; // establishes a known-good baseline

    // Deliberately drift the stream's duplicate window and one consumer's retry
    // config away from the desired values, simulating a stale config left by an
    // older deploy. Subjects/retention/filter match the desired config so only
    // the mutable fields under test change.
    jetstream
        .create_stream(async_nats::jetstream::stream::Config {
            name: nats::STREAM_NAME.to_string(),
            subjects: vec!["bot.>".to_string()],
            retention: async_nats::jetstream::stream::RetentionPolicy::WorkQueue,
            duplicate_window: Duration::from_secs(1),
            ..Default::default()
        })
        .await
        .unwrap();
    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    stream
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            durable_name: Some(nats::CONSUMER_TURN.to_string()),
            filter_subject: nats::SUBJECT_TURN.to_string(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(30),
            max_deliver: 1,
            ..Default::default()
        })
        .await
        .unwrap();

    // Sanity: the drift actually took effect, so the restore below is meaningful.
    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    assert_eq!(
        stream.cached_info().config.duplicate_window,
        Duration::from_secs(1),
        "test setup failed to drift the stream duplicate_window"
    );
    let drifted_turn: async_nats::jetstream::consumer::PullConsumer =
        stream.get_consumer(nats::CONSUMER_TURN).await.unwrap();
    assert_eq!(
        drifted_turn.cached_info().config.max_deliver, 1,
        "test setup failed to drift the bot-turn consumer max_deliver"
    );

    // Reconcile, then assert the desired config is restored.
    nats::ensure_stream_and_consumers(&jetstream).await.unwrap();

    let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
    assert_eq!(
        stream.cached_info().config.duplicate_window,
        Duration::from_secs(120),
        "ensure_stream_and_consumers must reconcile the stream duplicate_window \
         back to the explicit 120s desired value (F-105)"
    );

    for (name, subject) in [
        (nats::CONSUMER_TURN, nats::SUBJECT_TURN),
        (nats::CONSUMER_COMMAND, nats::SUBJECT_COMMAND),
    ] {
        let consumer: async_nats::jetstream::consumer::PullConsumer =
            stream.get_consumer(name).await.unwrap();
        let cfg = &consumer.cached_info().config;
        assert_eq!(
            cfg.filter_subject, subject,
            "{name} consumer filter_subject drifted from server"
        );
        assert_eq!(
            cfg.ack_wait,
            nats::ACK_WAIT,
            "{name} consumer ack_wait must be reconciled to the shared ACK_WAIT const"
        );
        assert_eq!(
            cfg.max_deliver,
            nats::MAX_DELIVER,
            "{name} consumer max_deliver must be reconciled to the shared MAX_DELIVER const"
        );
    }
}
