use brdgme_game::Gamer;
use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Pass,
    Take,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_pass(player) {
            parsers.push(Box::new(pass_parser()));
        }
        if self.can_take(player) {
            parsers.push(Box::new(take_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn pass_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("pass", "spend a chip to pass", Token::new("pass")),
        |_| Command::Pass,
    )
}

pub fn take_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "take",
            "take the card and all chips on it",
            Token::new("take"),
        ),
        |_| Command::Take,
    )
}
