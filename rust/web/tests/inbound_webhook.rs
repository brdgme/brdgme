use axum::body::Body;
use axum::http::{Request, StatusCode};
use serial_test::serial;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use web::router::build_router;
use web::state::AppState;
use web::websocket::GameBroadcaster;

const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

async fn make_state(pool: PgPool) -> AppState {
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let nats_client = async_nats::connect(&nats_url).await.expect("nats connect");
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    web::nats::ensure_stream_and_consumers(&jetstream)
        .await
        .expect("nats stream/consumers");
    let broadcaster = GameBroadcaster::new(nats_client);

    AppState {
        leptos_options: leptos::config::LeptosOptions::builder()
            .output_name("web")
            .build(),
        pool,
        broadcaster,
        http_client: reqwest::Client::new(),
        resend: None,
        jetstream,
    }
}

fn sign_body(secret: &str, msg_id: &str, body: &[u8]) -> (String, String, String) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let wh = svix::webhooks::Webhook::new(secret).unwrap();
    let sig = wh.sign(msg_id, ts, body).unwrap();
    (msg_id.to_string(), ts.to_string(), sig)
}

async fn post_webhook(app: &axum::Router, secret: &str, msg_id: &str, body: &[u8]) -> StatusCode {
    let (id, ts, sig) = sign_body(secret, msg_id, body);
    let req = Request::builder()
        .method("POST")
        .uri("/api/webhooks/resend")
        .header("content-type", "application/json")
        .header("svix-id", id)
        .header("svix-timestamp", ts)
        .header("svix-signature", sig)
        .body(Body::from(body.to_vec()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    resp.status()
}

async fn marker_count(pool: &PgPool, event_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM processed_webhook_events WHERE event_id = $1")
        .bind(event_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_player_with_verified_email(pool: &PgPool, token: &str, email: &str) -> (Uuid, Uuid) {
    let game_type_id: Uuid = sqlx::query_scalar(
        "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("Test Game {}", Uuid::new_v4()))
    .bind(vec![2i32])
    .fetch_one(pool)
    .await
    .unwrap();
    let game_version_id: Uuid = sqlx::query_scalar(
        "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
         VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
    )
    .bind(game_type_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let game_id: Uuid = sqlx::query_scalar(
        "INSERT INTO games (game_version_id, is_finished, game_state)
         VALUES ($1, false, 'initial') RETURNING id",
    )
    .bind(game_version_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let user_id: Uuid =
        sqlx::query_scalar("INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id")
            .bind("player")
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO game_players
         (game_id, user_id, position, color, has_accepted, is_turn,
          is_turn_at, last_turn_at, is_eliminated, is_read, email_token)
     VALUES ($1, $2, 0, 'Green', true, false, NOW(), NOW(), false, false, $3)",
    )
    .bind(game_id)
    .bind(user_id)
    .bind(token)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
    )
    .bind(user_id)
    .bind(email)
    .execute(pool)
    .await
    .unwrap();
    (game_id, user_id)
}

#[sqlx::test]
#[serial]
async fn transient_failure_returns_5xx_without_marker(pool: PgPool) {
    unsafe {
        std::env::set_var("RESEND_WEBHOOK_SECRET", SECRET);
        std::env::remove_var("RESEND_API_KEY");
    }
    let _ = seed_player_with_verified_email(&pool, "transient", "player@test.com").await;
    let app = build_router(make_state(pool.clone()).await).await;

    let msg_id = "msg_transient_1";
    let body = br#"{"type":"email.received","data":{"email_id":"em_123","from":"player@test.com","to":["g-transient@brdg.me"],"received_for":[]}}"#;
    let status = post_webhook(&app, SECRET, msg_id, body).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(marker_count(&pool, msg_id).await, 0);
}

#[sqlx::test]
#[serial]
async fn retry_not_short_circuited_as_duplicate(pool: PgPool) {
    unsafe {
        std::env::set_var("RESEND_WEBHOOK_SECRET", SECRET);
        std::env::remove_var("RESEND_API_KEY");
    }
    let _ = seed_player_with_verified_email(&pool, "transient", "player@test.com").await;
    let app = build_router(make_state(pool.clone()).await).await;

    let msg_id = "msg_retry_1";
    let body = br#"{"type":"email.received","data":{"email_id":"em_123","from":"player@test.com","to":["g-transient@brdg.me"],"received_for":[]}}"#;

    let status = post_webhook(&app, SECRET, msg_id, body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(marker_count(&pool, msg_id).await, 0);

    let status = post_webhook(&app, SECRET, msg_id, body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(marker_count(&pool, msg_id).await, 0);
}

#[sqlx::test]
#[serial]
async fn success_marks_exactly_once(pool: PgPool) {
    unsafe {
        std::env::set_var("RESEND_WEBHOOK_SECRET", SECRET);
        std::env::remove_var("RESEND_API_KEY");
    }
    let app = build_router(make_state(pool.clone()).await).await;

    let msg_id = "msg_success_1";
    let body = br#"{"type":"email.received","data":{"email_id":"em_456","from":"nobody@test.com","to":["g-unknowntoken@brdg.me"],"received_for":[]}}"#;

    let status = post_webhook(&app, SECRET, msg_id, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(marker_count(&pool, msg_id).await, 1);

    let status = post_webhook(&app, SECRET, msg_id, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(marker_count(&pool, msg_id).await, 1);
}

#[sqlx::test]
#[serial]
async fn permanent_failure_marks_and_returns_200(pool: PgPool) {
    unsafe {
        std::env::set_var("RESEND_WEBHOOK_SECRET", SECRET);
        std::env::remove_var("RESEND_API_KEY");
    }
    let app = build_router(make_state(pool.clone()).await).await;

    let msg_id_a = "msg_perm_a";
    let status = post_webhook(&app, SECRET, msg_id_a, b"not json{{{").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(marker_count(&pool, msg_id_a).await, 1);

    let msg_id_b = "msg_perm_b";
    let body = br#"{"type":"email.bounced","data":{"email_id":"x","from":"a@b.com","to":[],"received_for":[]}}"#;
    let status = post_webhook(&app, SECRET, msg_id_b, body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(marker_count(&pool, msg_id_b).await, 1);
}
