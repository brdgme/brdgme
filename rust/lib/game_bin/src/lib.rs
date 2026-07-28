use std::env;
use std::fmt::Debug;
use std::io;
use std::net::SocketAddr;

use brdgme_cmd::cli::cli;
use brdgme_cmd::http;
use brdgme_cmd::requester;
use brdgme_game::Gamer;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn cli_main<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>() {
    cli(
        &mut requester::gamer::new::<G>(),
        io::stdin(),
        &mut io::stdout(),
    );
}

pub fn http_main<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>() {
    http_main_inner::<G>();
}

#[tokio::main]
async fn http_main_inner<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>() {
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or("0.0.0.0:8080".to_string())
        .parse()
        .expect("Invalid socket address");
    http::serve::<G>(addr).await
}

pub fn fuzz_main<G: Gamer + Debug + Clone + Serialize + DeserializeOwned + 'static>() {
    brdgme_fuzz::fuzz_gamer::<G>();
}
