use brdgme_cmd::repl;
use brdgme_cmd::requester;
use lords_of_vegas_1::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
