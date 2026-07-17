//! 11.6a: in-process SSR page tests. These hit the exact same Axum/Leptos
//! router `main.rs` builds (via `web::router::build_router`, factored out for
//! this purpose) with `tower::ServiceExt::oneshot`, so no browser or running
//! binary is needed. They catch SSR panics, route breakage, and server-fn
//! 500s in milliseconds.
//!
//! Authentication: rather than driving the `Login`/`ConfirmLogin` server
//! functions over HTTP (their routes carry a compile-time hash suffix that
//! isn't practical to hardcode here), tests insert a `tower-sessions` row
//! directly via the same `PostgresStore` the app uses, then attach the
//! resulting session ID as a `Cookie: id=...` header - equivalent to what a
//! browser would send after a real login, without re-driving the login flow
//! net effect.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::{Json, Router, routing::post};
use brdgme_cmd::api::{GameResponse as GameStateResponse, PlayerRender, PubRender};
use brdgme_cmd::api::{Request as GameRequest, Response as GameResponse};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceExt;
use tower_sessions::{Expiry, Session};
use tower_sessions_sqlx_store::PostgresStore;
use uuid::Uuid;

use web::auth::session::set_user_session;
use web::db::{self, CreateGameOpts};
use web::game::server_fns::{BotSlot, RestartGame};
use web::models::user::User;
use web::router::build_router;
use web::state::AppState;
use web::websocket::GameBroadcaster;

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

/// Inserts a real session row (matching what a browser cookie after login
/// would reference) and returns the `Cookie` header value for it.
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

/// Spawns an in-process mock game service answering `PlayerRender` requests,
/// per the `rust/web` convention (docs/CODING.md "Testing Conventions": never
/// call the real game service in a test).
async fn spawn_mock_game_service() -> String {
    let app = Router::new().route(
        "/",
        post(|Json(payload): Json<GameRequest>| async move {
            match payload {
                GameRequest::PlayerRender { .. } => Json(GameResponse::PlayerRender {
                    render: PlayerRender {
                        player_state: "state".to_string(),
                        render: "mock render".to_string(),
                        command_spec: None,
                    },
                }),
                _ => Json(GameResponse::SystemError {
                    message: "unsupported in mock".to_string(),
                }),
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn make_game_version(pool: &PgPool, uri: &str) -> Uuid {
    let game_type_id = sqlx::query_scalar!(
        "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        format!("Test Game {}", Uuid::new_v4()),
        &vec![2i32]
    )
    .fetch_one(pool)
    .await
    .unwrap();
    make_game_version_for_type(pool, game_type_id, "1.0.0", uri, false).await
}

/// Inserts a game version onto an existing game type - used to build up
/// multiple versions (e.g. a deprecated one plus a newer one) for the same
/// game type.
async fn make_game_version_for_type(
    pool: &PgPool,
    game_type_id: Uuid,
    name: &str,
    uri: &str,
    is_deprecated: bool,
) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
         VALUES ($1, $2, $3, true, $4) RETURNING id",
        game_type_id,
        name,
        uri,
        is_deprecated
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Body must be a real page render, not a Leptos SSR error/panic. Rust panics
/// caught by the SSR renderer, and framework error boundaries, both surface a
/// "panicked at" substring or a 5xx status - assert neither.
fn assert_clean_html_body(status: StatusCode, content_type: &str, body: &str, marker: &str) {
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(
        content_type.starts_with("text/html"),
        "content-type was {content_type}"
    );
    assert!(
        body.contains(marker),
        "expected marker {marker:?} in body: {body}"
    );
    assert!(
        !body.to_lowercase().contains("panicked at"),
        "SSR body contains a panic message: {body}"
    );
}

async fn get(app: Router, path: &str, cookie: Option<&str>) -> (StatusCode, String, String) {
    let mut builder = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        content_type,
        String::from_utf8(body.to_vec()).unwrap(),
    )
}

#[sqlx::test]
async fn healthz_returns_200(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, _content_type, body) = get(app, "/healthz", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "OK");
}

// WP2 (#28 abuse protection): the router's `RequestBodyLimitLayer` should
// reject a request whose declared `Content-Length` exceeds the 256 KiB cap
// before ever calling the matched route, mirroring tower-http's own
// documented pattern of pre-checking the header rather than requiring the
// oversized body to actually be sent.
#[sqlx::test]
async fn oversized_post_body_rejected_with_413(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header(header::CONTENT_LENGTH, (256 * 1024 + 1).to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test]
async fn home_page_anonymous(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/", None).await;
    assert_clean_html_body(status, &content_type, &body, "Welcome to brdg.me");
}

#[sqlx::test]
async fn login_page_anonymous(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/login", None).await;
    assert_clean_html_body(
        status,
        &content_type,
        &body,
        "Enter your email address to start",
    );
}

#[sqlx::test]
async fn dashboard_page_anonymous(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/dashboard", None).await;
    assert_clean_html_body(status, &content_type, &body, "Dashboard");
}

#[sqlx::test]
async fn games_page_anonymous(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games", None).await;
    assert_clean_html_body(status, &content_type, &body, "New Game");
}

#[sqlx::test]
async fn game_page_anonymous_visitor_gets_clean_error_not_panic(pool: PgPool) {
    let uri = spawn_mock_game_service().await;
    let game_version_id = make_game_version(&pool, &uri).await;
    let owner = make_user(&pool, "owner").await;
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: owner.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
        },
    )
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/games/{}", game.id), None).await;
    // Anonymous visitors are not authenticated for get_game_details, so the
    // page renders its own in-app error state - still a clean 200, not a
    // framework panic.
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert!(content_type.starts_with("text/html"));
    assert!(!body.to_lowercase().contains("panicked at"), "body: {body}");
}

#[sqlx::test]
async fn game_page_logged_in_player_renders_game(pool: PgPool) {
    let uri = spawn_mock_game_service().await;
    let game_version_id = make_game_version(&pool, &uri).await;
    let user = make_user(&pool, "player-one").await;
    let email = "player-one@example.com";
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: user.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
        },
    )
    .await
    .unwrap();

    let cookie = login_cookie(&pool, &user, email).await;
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/games/{}", game.id), Some(&cookie)).await;
    // `Resource::new_blocking` streams the resolved `get_game_details` payload
    // as a serialized resource chunk for client-side hydration rather than
    // inlining the resolved `<div class="game-render">` markup synchronously
    // in this leptos version's SSR stream, so the mock game service's render
    // output showing up here is the marker that the authenticated
    // request (auth + DB lookup + game-service render) round-tripped
    // correctly, rather than erroring out as the anonymous case does.
    assert_clean_html_body(status, &content_type, &body, "mock render");
}

/// Spawns an in-process mock game service answering `New` requests, for
/// exercising `restart_game` without calling a real game service (per
/// docs/CODING.md "Testing Conventions").
async fn spawn_mock_new_game_service() -> String {
    let app = Router::new().route(
        "/",
        post(|Json(payload): Json<GameRequest>| async move {
            match payload {
                GameRequest::New { players, .. } => Json(GameResponse::New {
                    game: GameStateResponse {
                        state: "mock_state".to_string(),
                        points: vec![0.0; players],
                        status: brdgme_game::Status::Active {
                            whose_turn: vec![0],
                            eliminated: vec![],
                        },
                    },
                    logs: vec![],
                    public_render: PubRender {
                        pub_state: "pub".to_string(),
                        render: "mock render".to_string(),
                    },
                    player_renders: vec![],
                    seed: 0,
                }),
                _ => Json(GameResponse::SystemError {
                    message: "unsupported in mock".to_string(),
                }),
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

/// POSTs to the real `RestartGame` server-fn route. Args for this server-fn
/// shape are encoded as a url-encoded POST body, not JSON (unlike a
/// hand-rolled `reqwest` request to the game service).
async fn restart_game_via_http(app: Router, game_id: Uuid, cookie: &str) -> (StatusCode, String) {
    let path = <RestartGame as leptos::server_fn::ServerFn>::PATH;
    let body = format!("game_id={}", game_id);
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("cookie", cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let resp_body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&resp_body).into_owned())
}

// Regression test for the "restart 500 error" bug: `restart_game` sends its
// `Request::New` as a leptos server-fn call, which - unlike a hand-rolled
// `reqwest` request - encodes args as a url-encoded POST body, not JSON. This
// drives the real router/server-fn dispatch end to end (auth, DB lookup,
// game-service call, DB write) to prove that path works given a well-formed
// JSON response from the game service.
#[sqlx::test]
async fn restart_game_on_finished_game_succeeds(pool: PgPool) {
    let uri = spawn_mock_new_game_service().await;
    let game_version_id = make_game_version(&pool, &uri).await;
    let user = make_user(&pool, "player-one").await;
    let email = "player-one@example.com";
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &[],
            eliminated: &[],
            placings: &[0, 1],
            points: &[10.0, 5.0],
            creator_id: user.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
        },
    )
    .await
    .unwrap();
    sqlx::query!("UPDATE games SET is_finished = true WHERE id = $1", game.id)
        .execute(&pool)
        .await
        .unwrap();

    let cookie = login_cookie(&pool, &user, email).await;
    let app = build_router(make_state(pool).await).await;

    let (status, resp_text) = restart_game_via_http(app, game.id, &cookie).await;
    assert_eq!(status, StatusCode::OK, "body: {resp_text}");
}

// Regression test for the restart-onto-latest-version behaviour: when a
// newer, non-deprecated game_version exists for the same game type,
// restarting a finished game (played on an older/deprecated version) should
// create the new game on the newer version, not the original.
#[sqlx::test]
async fn restart_game_creates_new_game_on_latest_non_deprecated_version(pool: PgPool) {
    let old_uri = spawn_mock_new_game_service().await;
    let new_uri = spawn_mock_new_game_service().await;

    let old_game_version_id = make_game_version(&pool, &old_uri).await;
    let game_type_id = sqlx::query_scalar!(
        "SELECT game_type_id FROM game_versions WHERE id = $1",
        old_game_version_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    // Deprecate the original version and add a newer one pointing at a
    // different mock service, so we can tell which one restart used.
    sqlx::query!(
        "UPDATE game_versions SET is_deprecated = true WHERE id = $1",
        old_game_version_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let new_game_version_id =
        make_game_version_for_type(&pool, game_type_id, "2.0.0", &new_uri, false).await;

    let user = make_user(&pool, "player-one").await;
    let email = "player-one@example.com";
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id: old_game_version_id,
            whose_turn: &[],
            eliminated: &[],
            placings: &[0, 1],
            points: &[10.0, 5.0],
            creator_id: user.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
        },
    )
    .await
    .unwrap();
    sqlx::query!("UPDATE games SET is_finished = true WHERE id = $1", game.id)
        .execute(&pool)
        .await
        .unwrap();

    let cookie = login_cookie(&pool, &user, email).await;
    let app = build_router(make_state(pool.clone()).await).await;

    let (status, resp_text) = restart_game_via_http(app, game.id, &cookie).await;
    assert_eq!(status, StatusCode::OK, "body: {resp_text}");

    let new_game_id: Uuid = serde_json::from_str(&resp_text).unwrap();
    let new_game_version_id_used = sqlx::query_scalar!(
        "SELECT game_version_id FROM games WHERE id = $1",
        new_game_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(new_game_version_id_used, new_game_version_id);
}

// --- admin export route (#34, spec D4) ---

#[sqlx::test]
async fn admin_export_route_requires_login(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn admin_export_route_rejects_non_admin(pool: PgPool) {
    let user = make_user(&pool, "pleb").await;
    let cookie = login_cookie(&pool, &user, "pleb@example.com").await;
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn admin_export_route_returns_bundle_without_emails(pool: PgPool) {
    let admin = make_user(&pool, "boss").await;
    sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin.id)
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login_cookie(&pool, &admin, "boss@example.com").await;

    let game_version_id = make_game_version(&pool, "http://localhost:0/mock").await;
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: admin.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "opaque_state_blob",
        },
    )
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(
        app,
        &format!("/admin/games/{}/export", game.id),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("application/json"));
    let bundle: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(bundle["schema_version"], 1);
    assert_eq!(bundle["game"]["game_state"], "opaque_state_blob");
    assert_eq!(bundle["players"].as_array().unwrap().len(), 2);
    assert_eq!(bundle["bots"][0]["name"], "Botty");
    // Spec D4: no email addresses in the bundle, ever.
    assert!(!body.contains("boss@example.com"));
    assert!(!body.contains('@'));
}

#[sqlx::test]
async fn admin_export_route_missing_game_404s(pool: PgPool) {
    let admin = make_user(&pool, "boss2").await;
    sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin.id)
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login_cookie(&pool, &admin, "boss2@example.com").await;
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
