use std::io;

use brdgme_cmd::cli::cli;
use brdgme_cmd::requester;
use liars_dice_2::Game;

fn main() {
    cli(
        &mut requester::gamer::new::<Game>(),
        io::stdin(),
        &mut io::stdout(),
    );
}
