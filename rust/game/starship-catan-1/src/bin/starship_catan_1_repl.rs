use brdgme_cmd::repl;
use brdgme_cmd::requester;
use starship_catan_1::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
