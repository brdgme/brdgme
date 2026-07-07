use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Roll,
    Keep,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.finished {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_roll(player) {
            parsers.push(Box::new(roll_parser()));
        }
        if self.can_keep(player) {
            parsers.push(Box::new(keep_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn roll_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "roll",
            "push your luck and roll the dice",
            Token::new("roll"),
        ),
        |_| Command::Roll,
    )
}

pub fn keep_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "keep",
            "be a coward and keep your brains",
            Token::new("keep"),
        ),
        |_| Command::Keep,
    )
}
