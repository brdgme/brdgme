use std::fmt::Debug;
use std::net::SocketAddr;

use serde::{Serialize, de::DeserializeOwned};
use tokio::signal::unix::{SignalKind, signal};
use warp::Filter;
use warp::reject::Reject;

use brdgme_game::Gamer;

use crate::api::Request;
use crate::requester;
use crate::requester::Requester;
use crate::requester::error::RequestError;
use crate::requester::gamer::GameRequester;

impl Reject for RequestError {}

pub async fn serve<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
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
    let handler = warp::post()
        .and(warp::header::headers_cloned())
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
            let reply = warp::reply::json(&g.request(&req).unwrap());
            transaction.finish();
            reply
        });
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
