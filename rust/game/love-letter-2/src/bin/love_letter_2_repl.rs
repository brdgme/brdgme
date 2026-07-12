use brdgme_cmd::repl;
use brdgme_cmd::requester;
use love_letter_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
