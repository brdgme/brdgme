use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use tower_sessions::{Expiry, Session};
use tower_sessions_sqlx_store::PostgresStore;
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
    format!("id={}", id)
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
    .bind("1.0.0")
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

// --- Group 1: Rejection cases ---

#[sqlx::test]
async fn public_events_zero_topics_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
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
async fn public_events_malformed_uuid_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public?topic=game:not-a-uuid", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn public_events_no_colon_returns_400(pool: PgPool) {
    let (state, _) = make_state(pool).await;
    let app = build_router(state).await;
    let resp = sse_request(app, "/events/public?topic=gameonly", None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
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
#[ignore = "takes 32+ seconds"]
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
