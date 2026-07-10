use brdgme_game::command::parser::*;

use crate::{Game, Loc, all_locations};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Play(Loc),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        self.can_play(player)
            .then(|| Box::new(play_parser()) as Box<dyn Parser<T = Command>>)
    }
}

pub fn play_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("play", "play in a square", Token::new("play")),
            AfterSpace::new(Doc::name_desc(
                "square",
                "the square to play in",
                Enum::exact(all_locations()),
            )),
        ),
        |(_, loc): (String, Loc)| Command::Play(loc),
    )
}
