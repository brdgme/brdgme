//! WP2 (#28 abuse protection): proves the global `RequestBodyLimitLayer` /
//! `TimeoutLayer` hygiene layers added to `router::build_router` do not harm
//! `/ws` - the acceptance criterion is "a live websocket survives > 30s
//! idle-with-pings in dev". `tests/ssr_pages.rs`'s in-process
//! `tower::ServiceExt::oneshot` harness can't exercise this: a real HTTP
//! upgrade to a duplex socket needs an actual TCP connection (the upgrade
//! hijacks the underlying I/O from the hyper connection), so this test spins
//! up a real `axum::serve` listener, same as `main.rs`, and drives it with a
//! real websocket client.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use web::router::build_router;
use web::state::AppState;
use web::websocket::GameBroadcaster;

/// `REQUEST_TIMEOUT` in `router.rs` is 30s; wait comfortably past it so a
/// regression (the timeout layer wrongly bounding the socket's lifetime,
/// rather than just the handler future that returns immediately on upgrade)
/// would show up as the connection dying around the 30s mark.
const PAST_REQUEST_TIMEOUT: Duration = Duration::from_secs(32);

async fn spawn_app(pool: PgPool) -> (SocketAddr, GameBroadcaster) {
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
        login_rate_limiter: web::auth::rate_limit::build_login_rate_limiter(),
        confirm_rate_limiter: web::auth::rate_limit::build_confirm_rate_limiter(),
        jetstream,
    };

    let app = build_router(state).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    (addr, broadcaster)
}

#[sqlx::test]
async fn live_websocket_survives_idle_past_request_timeout(pool: PgPool) {
    let (addr, broadcaster) = spawn_app(pool).await;

    let (ws_stream, response) = timeout(
        Duration::from_secs(5),
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws")),
    )
    .await
    .expect("connect did not complete in time")
    .expect("websocket handshake failed");
    assert_eq!(
        response.status(),
        tokio_tungstenite::tungstenite::http::StatusCode::SWITCHING_PROTOCOLS
    );

    let (_write, mut read) = ws_stream.split();

    // Idle-with-pings: read whatever the server sends (its `ping_interval`
    // fires a `Message::Ping` every 30s) without the client sending
    // anything, for longer than `REQUEST_TIMEOUT`. A close frame or a
    // dropped stream here would mean the timeout layer reached into the
    // upgraded connection's lifetime.
    let start = Instant::now();
    let mut saw_ping = false;
    while start.elapsed() < PAST_REQUEST_TIMEOUT {
        let remaining = PAST_REQUEST_TIMEOUT - start.elapsed();
        match timeout(remaining, read.next()).await {
            Ok(Some(Ok(Message::Ping(_)))) => saw_ping = true,
            Ok(Some(Ok(Message::Close(frame)))) => {
                panic!("connection closed before {PAST_REQUEST_TIMEOUT:?} elapsed: {frame:?}")
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("websocket error before {PAST_REQUEST_TIMEOUT:?}: {e}"),
            Ok(None) => panic!("connection dropped before {PAST_REQUEST_TIMEOUT:?} elapsed"),
            // No message within the remaining window - that's fine, we're
            // just proving the socket wasn't torn down.
            Err(_) => break,
        }
    }
    assert!(
        start.elapsed() >= PAST_REQUEST_TIMEOUT,
        "test loop exited early after {:?}",
        start.elapsed()
    );
    assert!(
        saw_ping,
        "expected at least one keepalive ping from the server's 30s ping_interval"
    );

    // Prove the connection is still fully functional (not a half-dead
    // socket) by round-tripping a real game-update broadcast through it.
    let game_id = Uuid::new_v4();
    broadcaster.broadcast_game_update(game_id).await;
    let msg = timeout(Duration::from_secs(5), read.next())
        .await
        .expect("timed out waiting for broadcast message")
        .expect("stream ended")
        .expect("websocket error");
    let Message::Text(text) = msg else {
        panic!("expected a Text message, got {msg:?}");
    };
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["game_id"], game_id.to_string());
}
