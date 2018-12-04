use lost_cities::Game;
use brdgme_rand_bot::fuzz;

use std::io::stdout;

fn main() {
    fuzz::<Game, _>(&mut stdout());
}
