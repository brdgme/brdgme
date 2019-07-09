use brdgme_cmd::repl;
use brdgme_cmd::requester;
use lords_of_vegas::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
