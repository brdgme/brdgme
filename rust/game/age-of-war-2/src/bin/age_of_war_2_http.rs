use std::{env, net::SocketAddr};

use age_of_war_2::Game;
use brdgme_cmd::http;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or("0.0.0.0:80".to_string())
        .parse()
        .expect("Invalid socket address");
    http::serve::<Game>(addr).await
}
