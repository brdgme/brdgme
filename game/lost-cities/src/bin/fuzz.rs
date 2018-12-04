use std::io::stdout;

use brdgme_rand_bot::fuzz;
use lost_cities::Game;

fn main() {
    fuzz::<Game, _>(&mut stdout());
}
