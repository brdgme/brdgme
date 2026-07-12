use brdgme_cmd::repl;
use brdgme_cmd::requester;
use roll_through_the_ages_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
