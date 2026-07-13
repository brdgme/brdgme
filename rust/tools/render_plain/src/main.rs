//! Renders a brdgme markup string (from stdin) to plain text, substituting
//! player names given as CLI args (index order = player number).
//!
//! Usage: render_plain <player0-name> <player1-name> ... < markup.txt
//!
//! Works on markup produced by either the Rust `_cli` binaries or the Go
//! `brdgme-go` binaries - both emit the same `{{...}}` tag syntax.

use std::env;
use std::io::{self, Read};

use brdgme_color::LIGHT;
use brdgme_markup::{Player, from_string, plain, transform};

fn main() {
    let players: Vec<Player> = env::args()
        .skip(1)
        .enumerate()
        .map(|(i, name)| Player {
            name,
            color: LIGHT.player_color(i),
        })
        .collect();

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read markup from stdin");

    let (nodes, _) = from_string(&input).expect("failed to parse markup");
    print!("{}", plain(&transform(&nodes, &players)));
}
