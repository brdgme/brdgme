use std::io;

use acquire::Game;
use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
