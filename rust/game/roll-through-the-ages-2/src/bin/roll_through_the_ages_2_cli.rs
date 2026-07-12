use std::io;

use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;
use roll_through_the_ages_2::Game;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
