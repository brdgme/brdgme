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
use web::game::server_fns::{BotSlot, RestartGameWithRoster, RestartOutcome};
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
        "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
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

// G-route ranking proof: static "/games/new" must outrank parametric
// "/games/:id" (GamePage would render "Error: Invalid Game ID" for "new").
#[sqlx::test]
async fn new_game_type_page_anonymous(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/new", None).await;
    assert_clean_html_body(status, &content_type, &body, "New Game");
    assert!(
        !body.contains("Invalid Game ID"),
        "static /games/new ranked below parametric /games/:id: {body}"
    );
}

#[sqlx::test]
async fn games_route_is_unused_returns_not_found(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games", None).await;
    // The Routes fallback renders "Page not found." with a 404 status.
    assert_eq!(status, StatusCode::NOT_FOUND, "body: {body}");
    assert!(
        content_type.starts_with("text/html"),
        "content-type: {content_type}"
    );
    assert!(
        body.contains("Page not found."),
        "expected fallback marker in body: {body}"
    );
    assert!(
        !body.to_lowercase().contains("panicked at"),
        "SSR body contains a panic message: {body}"
    );
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
                bot_name: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
            all_accepted: false,
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
                bot_name: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
            all_accepted: false,
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

#[sqlx::test]
async fn game_page_player_names_link_to_profiles_for_human_opponents(pool: PgPool) {
    let uri = spawn_mock_game_service().await;
    let game_version_id = make_game_version(&pool, &uri).await;
    let user = make_user(&pool, "link-test-viewer").await;
    let opponent = make_user(&pool, "link-test-opponent").await;
    let email = "link-test-viewer@example.com";
    let game = db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: user.id,
            opponent_ids: &[opponent.id],
            opponent_emails: &[],
            bot_slots: &[],
            chat_id: None,
            game_state: "state",
            all_accepted: false,
        },
    )
    .await
    .unwrap();

    let cookie = login_cookie(&pool, &user, email).await;
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/games/{}", game.id), Some(&cookie)).await;

    assert_clean_html_body(status, &content_type, &body, "mock render");
    assert!(
        body.contains(&format!("href=\"/players/{}\"", opponent.name)),
        "expected human opponent profile link in game-meta player list: {body}"
    );
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

/// POSTs to the real `RestartGameWithRoster` server-fn route. Args for this
/// server-fn shape are encoded as a url-encoded POST body, not JSON (unlike a
/// hand-rolled `reqwest` request to the game service). `roster_suffix` carries
/// the `opponent_ids[..]`/`bot_slots[..]` fields.
async fn restart_game_with_roster_via_http(
    app: Router,
    game_id: Uuid,
    game_version_id: Uuid,
    roster_suffix: &str,
    cookie: &str,
) -> (StatusCode, String) {
    let path = <RestartGameWithRoster as leptos::server_fn::ServerFn>::PATH;
    let body = format!("game_id={game_id}&game_version_id={game_version_id}{roster_suffix}");
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
                bot_name: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
            all_accepted: false,
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

    let (status, resp_text) = restart_game_with_roster_via_http(
        app,
        game.id,
        game_version_id,
        "&bot_slots[0][name]=Botty&bot_slots[0][bot_name]=easy",
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {resp_text}");

    let outcome: RestartOutcome = serde_json::from_str(&resp_text).unwrap();
    match outcome {
        RestartOutcome::Created(po) => assert!(
            po.game_id.is_some(),
            "solo-vs-bots restart creates a game directly"
        ),
        other => panic!("expected Created, got {other:?}"),
    }
}

// `restart_game_with_roster` does not auto-resolve the latest version - it
// restarts onto the `game_version_id` passed (validated to belong to the game
// type). Passing the newer, non-deprecated version creates the new game on it.
#[sqlx::test]
async fn restart_game_with_roster_uses_passed_version(pool: PgPool) {
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
                bot_name: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "state",
            all_accepted: false,
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

    let (status, resp_text) = restart_game_with_roster_via_http(
        app,
        game.id,
        new_game_version_id,
        "&bot_slots[0][name]=Botty&bot_slots[0][bot_name]=easy",
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {resp_text}");

    let outcome: RestartOutcome = serde_json::from_str(&resp_text).unwrap();
    let RestartOutcome::Created(po) = outcome else {
        panic!("expected Created, got {outcome:?}")
    };
    let new_game_id = po
        .game_id
        .expect("solo-vs-bots restart creates a game directly");
    let new_game_version_id_used = sqlx::query_scalar!(
        "SELECT game_version_id FROM games WHERE id = $1",
        new_game_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(new_game_version_id_used, new_game_version_id);
}

// --- player profile page (#29) ---

#[sqlx::test]
async fn players_page_existing_user_anonymous(pool: PgPool) {
    let user = make_user(&pool, "profile-guy").await;
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user.name), None).await;
    assert_clean_html_body(status, &content_type, &body, "Member since");
}

#[sqlx::test]
async fn players_page_unknown_user_renders_not_found_200(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/players/nosuchplayer", None).await;
    assert_clean_html_body(status, &content_type, &body, "No such player.");
}

#[sqlx::test]
async fn players_page_no_games_shows_empty_state(pool: PgPool) {
    let user = make_user(&pool, "no-games-guy").await;
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user.name), None).await;
    assert_clean_html_body(status, &content_type, &body, "No finished games yet.");
}

/// Inserts a game type with a fixed (non-random) name, plus a single game
/// version for it - used by tests that assert on the game type name showing
/// up in rendered HTML.
async fn make_game_type_with_fixed_name(pool: &PgPool, name: &str) -> (Uuid, Uuid) {
    let game_type_id = sqlx::query_scalar!(
        "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        name,
        &vec![2i32]
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let game_version_id = make_game_version_for_type(
        pool,
        game_type_id,
        "1.0.0",
        "http://localhost:0/mock",
        false,
    )
    .await;
    (game_type_id, game_version_id)
}

/// Inserts a finished 2-player game plus its `game_players` rows directly,
/// per the SQL shape in `web::stats::queries::fixtures::insert_game` (not
/// importable here since that module is `#[cfg(test)] pub(crate)` inside the
/// `web` crate, not exported to integration tests).
async fn insert_finished_two_player_game(
    pool: &PgPool,
    game_version_id: Uuid,
    players: &[(Uuid, i32, i32)],
) -> Uuid {
    let game_id = sqlx::query_scalar!(
        "INSERT INTO games (id, game_version_id, is_finished, finished_at, game_state)
         VALUES (uuid_generate_v4(), $1, true, now(), '')
         RETURNING id",
        game_version_id
    )
    .fetch_one(pool)
    .await
    .unwrap();

    const COLORS: [&str; 2] = ["Green", "Red"];
    for (i, (user_id, place, rating_change)) in players.iter().enumerate() {
        sqlx::query!(
            r#"INSERT INTO game_players
                (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
                 is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
               VALUES (uuid_generate_v4(), $1, $2, NULL, $3, $4, true, false, now(), now(), false, true, $5, $6)"#,
            game_id,
            user_id,
            i as i32,
            COLORS[i % COLORS.len()],
            place,
            rating_change
        )
        .execute(pool)
        .await
        .unwrap();
    }

    game_id
}

#[sqlx::test]
async fn players_page_renders_game_type_table(pool: PgPool) {
    let (game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Profile Table Game").await;
    let user_a = make_user(&pool, "table-player-a").await;
    let user_b = make_user(&pool, "table-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;

    sqlx::query!(
        "INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
         VALUES (uuid_generate_v4(), $1, $2, $3, $4)",
        game_type_id,
        user_a.id,
        1216,
        1216
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user_a.name), None).await;

    assert_clean_html_body(status, &content_type, &body, "profile-game-types");
    assert!(
        body.contains("Profile Table Game"),
        "expected game type name in body: {body}"
    );
    assert!(body.contains("1216"), "expected rating in body: {body}");
    assert!(
        body.contains("By game type"),
        "expected heading in body: {body}"
    );
}

/// Inserts an unfinished 2-player game plus its `game_players` rows directly,
/// modeled on `insert_finished_two_player_game` but with `is_finished =
/// false`, `finished_at` NULL, and no place/rating_change - the shape
/// `stats::queries::active_games` selects on.
async fn insert_unfinished_two_player_game(
    pool: &PgPool,
    game_version_id: Uuid,
    players: &[Uuid],
) -> Uuid {
    let game_id = sqlx::query_scalar!(
        "INSERT INTO games (id, game_version_id, is_finished, finished_at, game_state)
         VALUES (uuid_generate_v4(), $1, false, NULL, '')
         RETURNING id",
        game_version_id
    )
    .fetch_one(pool)
    .await
    .unwrap();

    const COLORS: [&str; 2] = ["Green", "Red"];
    for (i, user_id) in players.iter().enumerate() {
        sqlx::query!(
            r#"INSERT INTO game_players
                (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
                 is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
               VALUES (uuid_generate_v4(), $1, $2, NULL, $3, $4, true, false, now(), now(), false, true, NULL, NULL)"#,
            game_id,
            user_id,
            i as i32,
            COLORS[i % COLORS.len()],
        )
        .execute(pool)
        .await
        .unwrap();
    }

    game_id
}

#[sqlx::test]
async fn players_page_active_games_visible_to_anonymous_viewer(pool: PgPool) {
    let (_game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Active Games Test Game").await;
    let user_a = make_user(&pool, "active-player-a").await;
    let user_b = make_user(&pool, "active-player-b").await;

    insert_unfinished_two_player_game(&pool, game_version_id, &[user_a.id, user_b.id]).await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user_a.name), None).await;

    assert_clean_html_body(status, &content_type, &body, "profile-active-games");
    assert!(
        body.contains("Active Games Test Game"),
        "expected game type name in body: {body}"
    );
    assert!(
        body.contains(&user_b.name),
        "expected opponent name in body: {body}"
    );
    assert!(
        !body.contains("No active games."),
        "expected active games list, not empty state: {body}"
    );
}

#[sqlx::test]
async fn players_page_recent_games_render_with_opponent_links(pool: PgPool) {
    let (_game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Recent Games Test Game").await;
    let user_a = make_user(&pool, "recent-player-a").await;
    let user_b = make_user(&pool, "recent-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user_a.name), None).await;

    assert_clean_html_body(status, &content_type, &body, "profile-recent-games");
    assert!(
        body.contains("1st of 2"),
        "expected placing in body: {body}"
    );
    assert!(
        body.contains(&format!("/players/{}", user_b.name)),
        "expected opponent profile link in body: {body}"
    );
    assert!(
        body.contains("+16"),
        "expected rating change in body: {body}"
    );
}

#[sqlx::test]
async fn players_page_bots_toggle_changes_inclusion(pool: PgPool) {
    let (_game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Bots Toggle Test Game").await;
    let user = make_user(&pool, "bots-toggle-player").await;

    let game_id = sqlx::query_scalar!(
        "INSERT INTO games (id, game_version_id, is_finished, finished_at, game_state)
         VALUES (uuid_generate_v4(), $1, true, now(), '')
         RETURNING id",
        game_version_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let game_bot_id = sqlx::query_scalar!(
        "INSERT INTO game_bots (id, game_id, name, bot_name)
         VALUES (uuid_generate_v4(), $1, $2, 'medium')
         RETURNING id",
        game_id,
        "Botty"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"INSERT INTO game_players
            (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
             is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
           VALUES (uuid_generate_v4(), $1, $2, NULL, 0, 'Green', true, false, now(), now(), false, true, 1, 16)"#,
        game_id,
        user.id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"INSERT INTO game_players
            (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
             is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
           VALUES (uuid_generate_v4(), $1, NULL, $2, 1, 'Red', true, false, now(), now(), false, true, 2, NULL)"#,
        game_id,
        game_bot_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;

    let (status, content_type, body) =
        get(app.clone(), &format!("/players/{}", user.name), None).await;
    assert_clean_html_body(status, &content_type, &body, "profile-bots-toggle");
    assert!(
        body.contains("No finished games yet."),
        "expected empty state by default (single-human game excluded): {body}"
    );
    assert!(
        body.contains("?bots=1"),
        "expected bots toggle link in body: {body}"
    );

    let (status, content_type, body) =
        get(app, &format!("/players/{}?bots=1", user.name), None).await;
    assert_clean_html_body(status, &content_type, &body, "Bots Toggle Test Game");
}

#[sqlx::test]
async fn players_page_add_friend_button_shown_to_other_logged_in_viewer(pool: PgPool) {
    let user_a = make_user(&pool, "add-friend-target").await;
    let user_b = make_user(&pool, "add-friend-viewer").await;
    let cookie = login_cookie(&pool, &user_b, "add-friend-viewer@example.com").await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/players/{}", user_a.name), Some(&cookie)).await;

    assert_clean_html_body(status, &content_type, &body, "profile-add-friend");
    assert!(
        body.contains("Add friend"),
        "expected add-friend affordance in body: {body}"
    );
}

#[sqlx::test]
async fn players_page_add_friend_button_absent_for_own_profile(pool: PgPool) {
    let user_a = make_user(&pool, "self-viewer").await;
    let cookie = login_cookie(&pool, &user_a, "self-viewer@example.com").await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/players/{}", user_a.name), Some(&cookie)).await;

    assert_clean_html_body(status, &content_type, &body, "Member since");
    assert!(
        !body.contains("Add friend"),
        "did not expect add-friend affordance on own profile: {body}"
    );
}

#[sqlx::test]
async fn players_page_form_strip_renders_in_game_types_table(pool: PgPool) {
    let (game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Form Strip Test Game").await;
    let user_a = make_user(&pool, "form-strip-player-a").await;
    let user_b = make_user(&pool, "form-strip-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;
    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 2, -8), (user_b.id, 1, 8)],
    )
    .await;

    sqlx::query!(
        "INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
         VALUES (uuid_generate_v4(), $1, $2, $3, $4)",
        game_type_id,
        user_a.id,
        1208,
        1216
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/players/{}", user_a.name), None).await;

    assert_clean_html_body(status, &content_type, &body, "form-strip");
    assert!(
        body.contains("form-gold") && body.contains("form-silver"),
        "expected form-gold/form-silver cells in body: {body}"
    );
}

// --- per-game-type deep-dive page (#29) ---

#[sqlx::test]
async fn deep_dive_page_renders_chart_histogram_and_games(pool: PgPool) {
    let (game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Deep Dive Game").await;
    let user_a = make_user(&pool, "deep-dive-player-a").await;
    let user_b = make_user(&pool, "deep-dive-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;
    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 2, -8), (user_b.id, 1, 8)],
    )
    .await;

    sqlx::query!(
        "INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
         VALUES (uuid_generate_v4(), $1, $2, $3, $4)",
        game_type_id,
        user_a.id,
        1208,
        1216
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(
        app,
        &format!("/players/{}/{}", user_a.name, "Deep%20Dive%20Game"),
        None,
    )
    .await;

    assert_clean_html_body(status, &content_type, &body, "gt-rating-chart");
    assert!(
        body.contains("rating-chart") && body.contains("<polyline"),
        "expected rating chart svg in body: {body}"
    );
    assert!(
        body.contains("gt-histograms"),
        "expected gt-histograms marker: {body}"
    );
    assert!(
        body.contains("histogram-bar") && body.contains("<rect"),
        "expected histogram bars in body: {body}"
    );
    assert!(
        body.contains("gt-finished-games"),
        "expected gt-finished-games marker: {body}"
    );
    assert!(
        body.contains("1st of 2"),
        "expected placing in body: {body}"
    );
    assert!(
        body.contains("+16"),
        "expected rating change in body: {body}"
    );
    assert!(
        body.contains(&format!("/players/{}", user_b.name)),
        "expected opponent profile link in body: {body}"
    );
    assert!(
        body.contains("profile-bots-toggle") && body.contains("?bots=1"),
        "expected bots toggle link in body: {body}"
    );
}

#[sqlx::test]
async fn deep_dive_page_unknown_game_type_renders_not_found_200(pool: PgPool) {
    let user = make_user(&pool, "deep-dive-known-user").await;
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/players/{}/NoSuchGame", user.name), None).await;
    assert_clean_html_body(status, &content_type, &body, "No such player or game type.");
}

#[sqlx::test]
async fn deep_dive_page_unknown_player_renders_not_found_200(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/players/nosuchplayer/Whatever", None).await;
    assert_clean_html_body(status, &content_type, &body, "No such player or game type.");
}

#[sqlx::test]
async fn deep_dive_page_head_to_head_rows_render(pool: PgPool) {
    let (_game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Head To Head Game").await;
    let user_a = make_user(&pool, "h2h-player-a").await;
    let user_b = make_user(&pool, "h2h-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;
    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 2, -8), (user_b.id, 1, 8)],
    )
    .await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(
        app,
        &format!("/players/{}/{}", user_a.name, "Head%20To%20Head%20Game"),
        None,
    )
    .await;

    assert_clean_html_body(status, &content_type, &body, "gt-head-to-head");
    assert!(
        body.contains(&user_b.name),
        "expected opponent name in body: {body}"
    );
    assert!(
        body.contains(&format!("/players/{}", user_b.name)),
        "expected opponent profile link in body: {body}"
    );
    assert!(
        body.contains("Head-to-head"),
        "expected section heading in body: {body}"
    );
}

#[sqlx::test]
async fn history_page_renders_games_placings_and_pagination(pool: PgPool) {
    let (_game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "History Page Game").await;
    let user_a = make_user(&pool, "history-player-a").await;
    let user_b = make_user(&pool, "history-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) =
        get(app, &format!("/players/{}/history", user_a.name), None).await;

    assert_clean_html_body(status, &content_type, &body, "player-history");
    assert!(
        body.contains("1st of 2"),
        "expected viewer placing in body: {body}"
    );
    assert!(
        body.contains("/games/"),
        "expected clickable game link in body: {body}"
    );
    assert!(
        body.contains("Page 1 of 1"),
        "expected pagination marker in body: {body}"
    );
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
                bot_name: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "opaque_state_blob",
            all_accepted: false,
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

// --- rules page (#25) ---

#[sqlx::test]
async fn rules_page_anonymous_renders_shell(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, &format!("/rules/{}", Uuid::new_v4()), None).await;
    assert_clean_html_body(status, &content_type, &body, "Rules");
}

#[sqlx::test]
async fn create_proposal_without_opponent_emails_succeeds(pool: PgPool) {
    let uri = spawn_mock_new_game_service().await;
    let game_version_id = make_game_version(&pool, &uri).await;
    let user = make_user(&pool, "creator").await;
    let cookie = login_cookie(&pool, &user, "creator@example.com").await;
    let app = build_router(make_state(pool).await).await;

    let path = <web::proposals::CreateProposal as leptos::server_fn::ServerFn>::PATH;
    let body = format!(
        "game_version_id={game_version_id}&bot_slots[0][name]=Botty&bot_slots[0][bot_name]=easy"
    );
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
    let text = String::from_utf8_lossy(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .into_owned();
    assert_eq!(status, StatusCode::OK, "body: {text}");
    assert!(!text.contains("missing field"), "body: {text}");
    let outcome: web::proposals::ProposalOutcome = serde_json::from_str(&text).unwrap();
    assert!(
        outcome.game_id.is_some(),
        "solo-vs-bots creates a game directly"
    );
}

// --- game information page (#7) ---

#[sqlx::test]
async fn game_info_page_renders_for_existing_game_type(pool: PgPool) {
    let (game_type_id, game_version_id) =
        make_game_type_with_fixed_name(&pool, "Info Page Game").await;
    let user_a = make_user(&pool, "info-player-a").await;
    let user_b = make_user(&pool, "info-player-b").await;

    insert_finished_two_player_game(
        &pool,
        game_version_id,
        &[(user_a.id, 1, 16), (user_b.id, 2, -16)],
    )
    .await;

    for (user_id, rating) in [(user_a.id, 1216i32), (user_b.id, 1184i32)] {
        sqlx::query(
            "INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
             VALUES (uuid_generate_v4(), $1, $2, $3, $4)",
        )
        .bind(game_type_id)
        .bind(user_id)
        .bind(rating)
        .bind(rating)
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/type/Info%20Page%20Game", None).await;

    assert_clean_html_body(status, &content_type, &body, "game-info");
    assert!(
        body.contains("Info Page Game"),
        "expected game type name in body: {body}"
    );
    assert!(
        body.contains("Top players"),
        "expected ranking heading in body: {body}"
    );
    assert!(
        body.contains("Start a game"),
        "expected start-game link in body: {body}"
    );
    assert!(
        body.contains("/games/new/"),
        "expected start-game href in body: {body}"
    );
}

#[sqlx::test]
async fn game_info_page_unknown_game_type_renders_not_found_200(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/type/NoSuchGame", None).await;
    assert_clean_html_body(status, &content_type, &body, "No such game type.");
}

#[sqlx::test]
async fn game_info_page_case_insensitive_name(pool: PgPool) {
    let (_game_type_id, _game_version_id) =
        make_game_type_with_fixed_name(&pool, "Info Page Game").await;

    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/type/info%20page%20game", None).await;

    assert_clean_html_body(status, &content_type, &body, "game-info");
}

#[sqlx::test]
async fn new_game_setup_page_renders_shell(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/new/Some%20Game", None).await;
    assert_clean_html_body(status, &content_type, &body, "Back to games");
}

#[sqlx::test]
async fn new_game_setup_page_unknown_type_renders_shell(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, content_type, body) = get(app, "/games/new/NoSuchGame", None).await;
    assert_clean_html_body(status, &content_type, &body, "Back to games");
}
