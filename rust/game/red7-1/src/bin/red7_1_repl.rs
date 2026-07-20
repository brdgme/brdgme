use brdgme_cmd::repl;
use brdgme_cmd::requester;
use red7_1::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
