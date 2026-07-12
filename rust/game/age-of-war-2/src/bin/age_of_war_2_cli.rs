use std::io;

use age_of_war_2::Game;
use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
