use brdgme_cmd::repl;
use brdgme_cmd::requester;
use tic_tac_toe_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
