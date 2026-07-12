use std::{env, net::SocketAddr};

use brdgme_cmd::http;
use roll_through_the_ages_2::Game;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = env::var("ADDR")
        .unwrap_or("0.0.0.0:80".to_string())
        .parse()
        .expect("Invalid socket address");
    http::serve::<Game>(addr).await
}
