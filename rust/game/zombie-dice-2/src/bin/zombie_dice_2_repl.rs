use brdgme_cmd::repl;
use brdgme_cmd::requester;
use zombie_dice_2::Game;

fn main() {
    repl(&mut requester::gamer::new::<Game>());
}
