use std::fmt::Debug;
use std::net::SocketAddr;

use serde::{Serialize, de::DeserializeOwned};
use tokio::signal::unix::{SignalKind, signal};
use warp::Filter;

use brdgme_game::Gamer;

use crate::api::{Request, Response};
use crate::requester;
use crate::requester::Requester;
use crate::requester::gamer::GameRequester;

const MAX_CONTENT_LENGTH: u64 = 16 * 1024 * 1024;

fn route<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>()
-> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::header::headers_cloned())
        .and(warp::body::content_length_limit(MAX_CONTENT_LENGTH))
        .and(warp::body::json())
        .map(|headers: warp::http::HeaderMap, req: Request| {
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
            let reply = warp::reply::json(&response);
            transaction.finish();
            reply
        })
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
    let handler = route::<G>();
    let shutdown = async {
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    warp::serve(handler)
        .bind(addr.into())
        .await
        .graceful(shutdown)
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Response;
    use crate::test_game::TestGame;

    #[tokio::test]
    async fn malformed_game_json_returns_system_error_not_panic() {
        let res = warp::test::request()
            .method("POST")
            .json(&Request::Status {
                game: "not valid json".to_string(),
            })
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(200, res.status());
        match serde_json::from_slice::<Response>(res.body()).unwrap() {
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
        let res = warp::test::request()
            .method("POST")
            .json(&Request::New {
                players: 2,
                seed: Some(1),
            })
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(200, res.status());
        match serde_json::from_slice::<Response>(res.body()).unwrap() {
            Response::New { player_renders, .. } => assert_eq!(2, player_renders.len()),
            r => panic!("expected New, got {:?}", r),
        }
    }

    #[tokio::test]
    async fn oversized_content_length_is_rejected() {
        let res = warp::test::request()
            .method("POST")
            .json(&Request::PlayerCounts)
            .header("content-length", (MAX_CONTENT_LENGTH + 1).to_string())
            .reply(&route::<TestGame>())
            .await;
        assert_eq!(413, res.status());
    }
}
