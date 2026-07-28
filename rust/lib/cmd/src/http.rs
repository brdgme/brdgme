use std::fmt::Debug;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Json};
use axum::http::HeaderMap;
use axum::routing::post;
use serde::{Serialize, de::DeserializeOwned};
use tokio::signal::unix::{SignalKind, signal};

use brdgme_game::Gamer;

use crate::api::{Request, Response};
use crate::requester;
use crate::requester::Requester;
use crate::requester::gamer::GameRequester;

const MAX_CONTENT_LENGTH: u64 = 16 * 1024 * 1024;

fn route<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>() -> Router {
    Router::new()
        .fallback_service(post(handle::<G>))
        .layer(DefaultBodyLimit::max(MAX_CONTENT_LENGTH as usize))
}

async fn handle<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>(
    headers: HeaderMap,
    Json(req): Json<Request>,
) -> Json<Response> {
    let header_pairs: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
        .collect();
    let ctx = sentry::TransactionContext::continue_from_headers(
        "game.request",
        "http.server",
        header_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
    );
    let transaction = sentry::start_transaction(ctx);
    sentry::configure_scope(|scope| {
        scope.set_span(Some(transaction.clone().into()));
    });
    let mut g: GameRequester<G> = requester::gamer::new();
    let response = g.request(&req).unwrap_or_else(|e| Response::SystemError {
        message: e.to_string(),
    });
    transaction.finish();
    Json(response)
}

pub async fn serve<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>(
    addr: impl Into<SocketAddr>,
) {
    env_logger::init();
    let _sentry_guard = std::env::var("SENTRY_DSN_SERVER").ok().map(|dsn| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: std::env::var("SENTRY_RELEASE")
                    .ok()
                    .map(std::borrow::Cow::Owned),
                send_default_pii: false,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });
    let shutdown = async {
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    let listener = tokio::net::TcpListener::bind(addr.into())
        .await
        .expect("failed to bind TCP listener");
    axum::serve(listener, route::<G>())
        .with_graceful_shutdown(shutdown)
        .await
        .expect("game HTTP server failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Response;
    use crate::test_game::TestGame;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    #[tokio::test]
    async fn malformed_game_json_returns_system_error_not_panic() {
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&Request::Status {
                    game: "not valid json".to_string(),
                })
                .unwrap(),
            ))
            .unwrap();
        let res = route::<TestGame>().oneshot(req).await.unwrap();
        assert_eq!(200, res.status());
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        match serde_json::from_slice::<Response>(&body).unwrap() {
            Response::SystemError { message } => assert!(
                message.contains("failed to parse request"),
                "got: {}",
                message
            ),
            r => panic!("expected SystemError, got {:?}", r),
        }
    }

    #[tokio::test]
    async fn valid_request_still_served() {
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&Request::New {
                    players: 2,
                    seed: Some(1),
                })
                .unwrap(),
            ))
            .unwrap();
        let res = route::<TestGame>().oneshot(req).await.unwrap();
        assert_eq!(200, res.status());
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        match serde_json::from_slice::<Response>(&body).unwrap() {
            Response::New { player_renders, .. } => assert_eq!(2, player_renders.len()),
            r => panic!("expected New, got {:?}", r),
        }
    }

    #[tokio::test]
    async fn oversized_content_length_is_rejected() {
        let big = vec![0u8; (MAX_CONTENT_LENGTH + 1) as usize];
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(big))
            .unwrap();
        let res = route::<TestGame>().oneshot(req).await.unwrap();
        assert_eq!(413, res.status());
    }
}
