use std::fmt::Debug;
use std::net::SocketAddr;

use serde::{de::DeserializeOwned, Serialize};
use warp::reject::Reject;
use warp::Filter;

use brdgme_game::Gamer;

use crate::api::Request;
use crate::requester;
use crate::requester::error::RequestError;
use crate::requester::gamer::GameRequester;
use crate::requester::Requester;

impl Reject for RequestError {}

pub async fn serve<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    addr: impl Into<SocketAddr>,
) {
    env_logger::init();
    let handler = warp::post().and(warp::body::json()).map(|req: Request| {
        let mut g: GameRequester<G> = requester::gamer::new();
        warp::reply::json(&g.request(&req).unwrap())
    });
    warp::serve(handler).run(addr).await
}
