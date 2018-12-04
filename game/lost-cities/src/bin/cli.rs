use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;
use lost_cities::Game;

use std::io;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
