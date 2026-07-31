use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use brdgme_session_store::PostgresStore;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use serial_test::serial;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use tower_sessions::{Expiry, Session};
use uuid::Uuid;

use web::auth::session::set_user_session;
use web::db::{self, CreateGameOpts};
use web::game::server_fns::BotSlot;
use web::models::user::User;
use web::router::build_router;
use web::state::AppState;
use web::websocket::GameBroadcaster;

async fn make_state(pool: PgPool) -> (AppState, GameBroadcaster) {
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let nats_client = async_nats::connect(&nats_url).await.expect("nats connect");
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    web::nats::ensure_stream_and_consumers(&jetstream)
        .await
        .expect("nats stream/consumers");
    let broadcaster = GameBroadcaster::new(nats_client);

    let state = AppState {
        leptos_options: leptos::config::LeptosOptions::builder()
            .output_name("web")
            .build(),
        pool,
        broadcaster: broadcaster.clone(),
        http_client: reqwest::Client::new(),
        resend: None,
        jetstream,
    };
    (state, broadcaster)
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

async fn login_cookie(pool: &PgPool, user: &User, email: &str) -> String {
    let (cookie, _) = login_cookie_with_token(pool, user, email).await;
    cookie
}

async fn login_cookie_with_token(pool: &PgPool, user: &User, email: &str) -> (String, Uuid) {
    let store = PostgresStore::new(pool.clone());
    store.migrate().await.unwrap();

    let auth_token_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
        auth_token_id,
        user.id
    )
    .execute(pool)
    .await
    .unwrap();

    let session = Session::new(
        None,
        std::sync::Arc::new(store),
        Some(Expiry::OnInactivity(
            tower_sessions::cookie::time::Duration::days(30),
        )),
    );
    set_user_session(&session, user, email, auth_token_id)
        .await
        .unwrap();
    session.save().await.unwrap();
    let id = session.id().expect("session id assigned after save");
    (format!("id={}", id), auth_token_id)
}

async fn make_game_version(pool: &PgPool) -> Uuid {
    let game_type_id: Uuid = sqlx::query_scalar(
        "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("Test Game {}", Uuid::new_v4()))
    .bind(vec![2i32])
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
         VALUES ($1, $2, $3, true, false) RETURNING id",
    )
    .bind(game_type_id)
    .bind("test-v1")
    .bind("http://localhost:0/mock")
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_game(pool: &PgPool, gv: Uuid, creator: Uuid, opponents: &[Uuid]) -> Uuid {
    let bot_slots: Vec<BotSlot> = if opponents.is_empty() {
        vec![BotSlot {
            name: "Bot".to_string(),
            bot_name: "easy".to_string(),
        }]
    } else {
        vec![]
    };
    let game = db::create_game_with_users(
        pool,
        CreateGameOpts {
            game_version_id: gv,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: creator,
            opponent_ids: opponents,
            opponent_emails: &[],
            bot_slots: &bot_slots,
            chat_id: None,
            game_state: "state",
            all_accepted: false,
        },
    )
    .await
    .unwrap();
    game.id
}

async fn seed_proposal(pool: &PgPool, gv: Uuid, owner: Uuid, player: Uuid) -> Uuid {
    let proposal_id: Uuid = sqlx::query_scalar(
        "INSERT INTO game_proposals (game_version_id, owner_user_id, status) VALUES ($1,$2,'open') RETURNING id",
    )
    .bind(gv)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO game_proposal_players (proposal_id, position, user_id, bot_name, response) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(proposal_id)
    .bind(0i32)
    .bind(owner)
    .bind(Option::<&str>::None)
    .bind("accepted")
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO game_proposal_players (proposal_id, position, user_id, bot_name, response) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(proposal_id)
    .bind(1i32)
    .bind(player)
    .bind(Option::<&str>::None)
    .bind("pending")
    .execute(pool)
    .await
    .unwrap();
    proposal_id
}

async fn read_sse_text(body: &mut Body, duration: Duration) -> String {
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + duration;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), body.frame()).await {
            Ok(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    collected.push_str(&String::from_utf8_lossy(data));
                }
            }
            Ok(Some(Err(_))) | Ok(None) => break,
            Err(_) => continue,
        }
    }
    collected
}

async fn sse_request(app: Router, path: &str, cookie: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let req = builder.body(Body::empty()).unwrap();
    app.oneshot(req).await.unwrap()
}

async fn serve_router(app: Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}

// The `sse_connections` gauge is recorded through the global `metrics` facade,
// which is a no-op until a recorder is installed. `PrometheusMetricLayer::pair()`
// installs the recorder (and panics if called twice), so it is guarded by a
// process-wide `OnceLock`; the returned handle renders the current gauge value.
fn metrics_handle() -> &'static axum_prometheus::metrics_exporter_prometheus::PrometheusHandle {
    static HANDLE: std::sync::OnceLock<
        axum_prometheus::metrics_exporter_prometheus::PrometheusHandle,
    > = std::sync::OnceLock::new();
    HANDLE.get_or_init(|| {
        let (_layer, handle) = axum_prometheus::PrometheusMetricLayer::pair();
        handle
    })
}

fn gauge_value(
    handle: &axum_prometheus::metrics_exporter_prometheus::PrometheusHandle,
    name: &str,
) -> f64 {
    for line in handle.render().lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some(name)
            && let Some(val) = parts.next()
        {
            return val.parse().unwrap_or(f64::NAN);
        }
    }
    0.0
}

// NATS publishes its monitoring endpoint on the client port + 4000 (CI maps
// 4222->8222, the local test script 14222->18222; both run `nats-server -m 8222`).
fn nats_monitor_base() -> String {
    let url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let stripped = url.strip_prefix("nats://").unwrap_or(&url);
    let (host, port) = match stripped.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(4222)),
        None => (stripped.to_string(), 4222),
    };
    format!("http://{}:{}", host, port + 4000)
}

async fn nats_game_subjects() -> Vec<String> {
    let url = format!("{}/subsz?subs=1", nats_monitor_base());
    let body = reqwest::get(&url)
        .await
        .unwrap_or_else(|e| panic!("NATS monitoring endpoint {url} unreachable: {e}"))
        .text()
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("could not parse NATS subsz JSON from {url}: {e}\nbody: {body}"));
    let mut subjects = Vec::new();
    if let Some(subs) = json.get("subscriptions").and_then(|s| s.as_array()) {
        for sub in subs {
            if let Some(subject) = sub.get("subject").and_then(|s| s.as_str())
                && subject.starts_with("game.")
            {
                subjects.push(subject.to_string());
            }
        }
    }
    subjects
}

// --- Group 1: Rejection cases ---

#[sqlx::test]
#[serial]
async fn public_events_zero_topics_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
#[serial]
async fn public_events_unknown_topic_kind_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(
        app,
        &format!("/events/public?topic=tournament:{}", Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
#[serial]
async fn public_events_malformed_uuid_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public?topic=game:not-a-uuid", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
#[serial]
async fn public_events_no_colon_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public?topic=gameonly", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
#[serial]
async fn public_events_over_cap_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let topics: Vec<String> = (0..17)
        .map(|_| format!("topic=game:{}", Uuid::new_v4()))
        .collect();
    let resp = sse_request(app, &format!("/events/public?{}", topics.join("&")), None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- Group 2: Anonymous /events returns 200 ---

#[sqlx::test]
#[serial]
async fn anonymous_events_returns_200_event_stream(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    assert!(ct.contains("text/event-stream"), "content-type was {ct}");
}

// --- Group 3: Frame delivery ---

#[sqlx::test]
#[serial]
async fn public_game_frame_reaches_anonymous_events(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let creator = make_user(&pool, "pub-creator").await;
    let opponent = make_user(&pool, "pub-opponent").await;
    let game_id = seed_game(&pool, gv, creator.id, &[opponent.id]).await;

    let (state, broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events", None).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut body = resp.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;
    broadcaster.broadcast_game_update(game_id).await;

    let text = read_sse_text(&mut body, Duration::from_secs(5)).await;
    assert!(
        text.contains("event: game") && text.contains(&game_id.to_string()),
        "expected game frame for {game_id}, got: {text}"
    );
}

#[sqlx::test]
#[serial]
async fn private_game_frame_does_not_reach_non_participant(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let player_a = make_user(&pool, "priv-a").await;
    let player_b = make_user(&pool, "priv-b").await;
    let observer = make_user(&pool, "priv-observer").await;

    sqlx::query("UPDATE users SET game_visibility = 'private' WHERE id = $1")
        .bind(player_a.id)
        .execute(&pool)
        .await
        .unwrap();

    let private_game_id = seed_game(&pool, gv, player_a.id, &[player_b.id]).await;
    let observer_game_id = seed_game(&pool, gv, observer.id, &[]).await;

    let cookie = login_cookie(&pool, &observer, "priv-observer@example.com").await;
    let (state, broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events", Some(&cookie)).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut body = resp.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;
    broadcaster.broadcast_game_update(private_game_id).await;

    let text = read_sse_text(&mut body, Duration::from_secs(2)).await;
    assert!(
        !text.contains(&private_game_id.to_string()),
        "private game frame leaked to non-participant: {text}"
    );

    broadcaster.broadcast_game_update(observer_game_id).await;
    let text = read_sse_text(&mut body, Duration::from_secs(5)).await;
    assert!(
        text.contains("event: game") && text.contains(&observer_game_id.to_string()),
        "expected liveness frame for observer's own game, got: {text}"
    );
}

#[sqlx::test]
#[serial]
async fn proposal_frame_reaches_participant_but_not_anonymous(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let owner = make_user(&pool, "prop-owner").await;
    let player = make_user(&pool, "prop-player").await;
    let proposal_id = seed_proposal(&pool, gv, owner.id, player.id).await;

    let cookie = login_cookie(&pool, &player, "prop-player@example.com").await;
    let (state, broadcaster) = make_state(pool).await;

    let app = build_router(state.clone()).await;
    let resp = sse_request(app, "/events", Some(&cookie)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let mut body = resp.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;
    broadcaster.broadcast_proposal_update(proposal_id).await;

    let text = read_sse_text(&mut body, Duration::from_secs(5)).await;
    assert!(
        text.contains("event: proposal") && text.contains(&proposal_id.to_string()),
        "participant should receive proposal frame, got: {text}"
    );

    let app2 = build_router(state).await;
    let resp2 = sse_request(app2, "/events", None).await;
    assert_eq!(resp2.status(), StatusCode::OK);
    let mut body2 = resp2.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;
    broadcaster.broadcast_proposal_update(proposal_id).await;

    let text2 = read_sse_text(&mut body2, Duration::from_secs(2)).await;
    assert!(
        !text2.contains(&proposal_id.to_string()),
        "anonymous should NOT receive proposal frame, got: {text2}"
    );
}

#[sqlx::test]
#[serial]
async fn public_events_receives_matching_game_only(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let u1 = make_user(&pool, "pe-u1").await;
    let u2 = make_user(&pool, "pe-u2").await;
    let u3 = make_user(&pool, "pe-u3").await;
    let u4 = make_user(&pool, "pe-u4").await;

    let game_a = seed_game(&pool, gv, u1.id, &[u2.id]).await;
    let game_b = seed_game(&pool, gv, u3.id, &[u4.id]).await;

    let priv_player = make_user(&pool, "pe-priv").await;
    sqlx::query("UPDATE users SET game_visibility = 'private' WHERE id = $1")
        .bind(priv_player.id)
        .execute(&pool)
        .await
        .unwrap();
    let private_game = seed_game(&pool, gv, priv_player.id, &[]).await;

    let (state, broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, &format!("/events/public?topic=game:{game_a}"), None).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut body = resp.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;

    broadcaster.broadcast_game_update(game_a).await;
    let text = read_sse_text(&mut body, Duration::from_secs(5)).await;
    assert!(
        text.contains("event: game") && text.contains(&game_a.to_string()),
        "expected frame for game_a, got: {text}"
    );

    broadcaster.broadcast_game_update(game_b).await;
    let text = read_sse_text(&mut body, Duration::from_secs(2)).await;
    assert!(
        !text.contains(&game_b.to_string()),
        "should not receive frame for non-subscribed game_b: {text}"
    );

    broadcaster.broadcast_game_update(private_game).await;
    let text = read_sse_text(&mut body, Duration::from_secs(2)).await;
    assert!(
        !text.contains(&private_game.to_string()),
        "should not receive frame for private game: {text}"
    );
}

#[sqlx::test]
#[serial]
async fn public_events_multiple_topics_all_deliver(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let u1 = make_user(&pool, "mt-u1").await;
    let u2 = make_user(&pool, "mt-u2").await;
    let u3 = make_user(&pool, "mt-u3").await;
    let u4 = make_user(&pool, "mt-u4").await;

    let game_a = seed_game(&pool, gv, u1.id, &[u2.id]).await;
    let game_b = seed_game(&pool, gv, u3.id, &[u4.id]).await;

    let (state, broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(
        app,
        &format!("/events/public?topic=game:{game_a}&topic=game:{game_b}"),
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut body = resp.into_body();
    tokio::time::sleep(Duration::from_millis(200)).await;

    broadcaster.broadcast_game_update(game_a).await;
    broadcaster.broadcast_game_update(game_b).await;

    let text = read_sse_text(&mut body, Duration::from_secs(5)).await;
    assert!(
        text.contains(&game_a.to_string()),
        "expected frame for game_a, got: {text}"
    );
    assert!(
        text.contains(&game_b.to_string()),
        "expected frame for game_b, got: {text}"
    );
}

// --- Group 4: Keepalive ---

#[sqlx::test]
#[serial]
async fn sse_stream_survives_past_request_timeout_with_keepalive(pool: PgPool) {
    use std::net::SocketAddr;

    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let mut stream = response.bytes_stream();
    let start = std::time::Instant::now();
    let mut saw_keepalive = false;

    while start.elapsed() < Duration::from_secs(32) {
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                let text = String::from_utf8_lossy(&chunk);
                if text.contains(": ") || text.starts_with(':') {
                    saw_keepalive = true;
                }
            }
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => panic!("stream ended before 32s"),
            Err(_) => continue,
        }
    }
    assert!(saw_keepalive, "expected at least one keepalive comment");
}

// --- Group 5: Graceful shutdown ---

#[sqlx::test]
#[serial]
async fn graceful_shutdown_ends_sse_stream_and_server_completes(pool: PgPool) {
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    let (state, broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown({
            let broadcaster = broadcaster.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                broadcaster.begin_shutdown();
            }
        })
        .await
        .unwrap();
    });

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    let mut stream = response.bytes_stream();
    let mut stream_ended = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(_))) | Ok(None) => {
                stream_ended = true;
                break;
            }
            Err(_) => continue,
        }
    }
    assert!(
        stream_ended,
        "SSE stream did not end after graceful shutdown"
    );

    let server_result = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
    assert!(
        server_result.is_ok(),
        "server task did not complete within 5s of shutdown"
    );
}

// --- Group 6: Authorization lifetime and task hygiene (R-10) ---

// F-158: the auth handler validates the session token once at connect and never
// re-validates, so a revoked token keeps delivering events on the open stream.
// Drives the real handler over a real listener and asserts the stream TERMINATES
// after the token is revoked mid-stream. Pre-fix the viewer is captured once and
// nothing re-runs `validate_session_token`, so the stream never ends and the
// bounded deadline below is exhausted (test fails). Post-fix a periodic
// re-validation arm breaks the loop and the stream ends promptly.
#[sqlx::test]
#[serial]
async fn auth_stream_terminates_after_token_revocation(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let creator = make_user(&pool, "rev-creator").await;
    let opponent = make_user(&pool, "rev-opponent").await;
    let game_id = seed_game(&pool, gv, creator.id, &[opponent.id]).await;

    let (cookie, auth_token_id) =
        login_cookie_with_token(&pool, &creator, "rev-creator@example.com").await;

    let (state, broadcaster) = make_state(pool.clone()).await;
    let app = build_router(state).await;
    let addr = serve_router(app).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/events"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let mut stream = response.bytes_stream();

    tokio::time::sleep(Duration::from_millis(200)).await;
    broadcaster.broadcast_game_update(game_id).await;
    let mut saw_frame = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                if String::from_utf8_lossy(&chunk).contains(&game_id.to_string()) {
                    saw_frame = true;
                    break;
                }
            }
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => panic!("stream ended before delivering any frame"),
            Err(_) => continue,
        }
    }
    assert!(
        saw_frame,
        "authenticated stream did not deliver a visible game frame"
    );

    web::auth::session::invalidate_auth_token(&pool, auth_token_id)
        .await
        .unwrap();

    // Keep broadcasting visible events so a handler that re-checks per visible
    // event also terminates; a time-based re-validation arm terminates regardless.
    // The bound must exceed the implementation's re-validation period.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut stream_ended = false;
    while tokio::time::Instant::now() < deadline {
        broadcaster.broadcast_game_update(game_id).await;
        match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(_))) | Ok(None) => {
                stream_ended = true;
                break;
            }
            Err(_) => continue,
        }
    }
    assert!(
        stream_ended,
        "SSE stream stayed open after auth token revocation; handler never re-validated the session"
    );
}

// F-159: an idle anonymous connection (no visible games, so `tx.send` is never
// reached) leaks its spawned task and NATS subscription past client disconnect -
// the task only exits on a visible-event send failure or global shutdown. Opens a
// real connection, drops the client, and asserts the `sse_connections` gauge (the
// metric the finding names as hiding the leak, decremented on the task guard's
// Drop) falls back within a bounded deadline. Pre-fix the anonymous task lives
// until process shutdown, so the gauge never falls and the deadline is exhausted
// (test fails). Post-fix a per-connection cancellation token fires on stream drop
// and the task (with its subscription) goes away promptly.
#[sqlx::test]
#[serial]
async fn idle_anonymous_connection_releases_task_on_disconnect(pool: PgPool) {
    let handle = metrics_handle();
    let (state, _broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let addr = serve_router(app).await;

    let before = gauge_value(handle, "sse_connections");

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let stream = response.bytes_stream();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let peak = gauge_value(handle, "sse_connections");
    assert!(
        peak >= before + 1.0,
        "expected sse_connections to rise by 1 for the open connection (before={before}, peak={peak})"
    );

    drop(stream);
    drop(client);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut released = false;
    while tokio::time::Instant::now() < deadline {
        if gauge_value(handle, "sse_connections") <= peak - 1.0 {
            released = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        released,
        "sse_connections did not fall back after client disconnect; the anonymous SSE task leaked (before={before}, peak={peak}, now={})",
        gauge_value(handle, "sse_connections")
    );
}

// F-160: the public handler subscribes to the `game.>` firehose and filters
// in-process instead of subscribing to the specific `game.{id}` subjects it
// parsed. Opens a real `/events/public?topic=game:{A}` connection and asserts,
// via the NATS monitoring `subsz` endpoint, that the server holds a subscription
// on the specific `game.{A}` subject and NONE on the `game.>` wildcard. Pre-fix
// the handler subscribes to `game.>`, so the wildcard assertion fails. The
// frame-level filtering itself is already covered by
// `public_events_receives_matching_game_only`; the cached visibility predicate is
// covered by the `VisibilityCache` unit tests.
#[sqlx::test]
#[serial]
async fn public_handler_subscribes_per_game_not_firehose(pool: PgPool) {
    let gv = make_game_version(&pool).await;
    let u1 = make_user(&pool, "sub-u1").await;
    let u2 = make_user(&pool, "sub-u2").await;
    let game_a = seed_game(&pool, gv, u1.id, &[u2.id]).await;

    let (state, _broadcaster) = make_state(pool).await;
    let app = build_router(state).await;
    let addr = serve_router(app).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/events/public?topic=game:{game_a}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let _stream = response.bytes_stream();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let subjects = nats_game_subjects().await;
    assert!(
        !subjects.iter().any(|s| s == "game.>"),
        "public handler must not subscribe to the game.> firehose; active game.* subscriptions: {subjects:?}"
    );
    assert!(
        subjects.iter().any(|s| s == &format!("game.{game_a}")),
        "public handler should subscribe to the specific game.{game_a} subject; active game.* subscriptions: {subjects:?}"
    );
}
