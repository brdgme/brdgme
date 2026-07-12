use brdgme_cmd::repl;
use brdgme_cmd::requester;
use modern_art_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
