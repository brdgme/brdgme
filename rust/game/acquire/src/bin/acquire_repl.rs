use acquire::Game;
use brdgme_cmd::repl;
use brdgme_cmd::requester;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
