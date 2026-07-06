use brdgme_game::command::parser::*;

use crate::Die;
use crate::Game;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Score { dice: Vec<Die> },
    Roll,
    Done,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_score(player) {
            parsers.push(Box::new(score_parser()));
        }
        if self.can_roll(player) {
            parsers.push(Box::new(roll_parser()));
        }
        if self.can_done(player) {
            parsers.push(Box::new(done_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

/// `score <dice>` where `<dice>` is one or more space-separated integers
/// 1..=6; the selection is validated against the score table at action time.
/// Faithful port of the Go farkle score parser.
pub fn score_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("score", "score dice", Token::new("score")),
            AfterSpace::new(Doc::name_desc(
                "dice",
                "the dice to score",
                Many::some_spaced(Int::bounded(1, 6)),
            )),
        ),
        |(_, dice_i32): (_, Vec<i32>)| Command::Score {
            dice: dice_i32.into_iter().map(|d| d as Die).collect(),
        },
    )
}

pub fn roll_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("roll", "roll the dice", Token::new("roll")),
        |_| Command::Roll,
    )
}

pub fn done_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("done", "finish your turn", Token::new("done")),
        |_| Command::Done,
    )
}
