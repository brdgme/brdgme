use std::{env, net::SocketAddr};

use brdgme_cmd::http;
use lords_of_vegas_1::Game;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or("0.0.0.0:80".to_string())
        .parse()
        .expect("Invalid socket address");
    http::serve::<Game>(addr).await
}
