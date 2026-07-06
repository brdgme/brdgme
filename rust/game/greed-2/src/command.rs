use brdgme_game::command::parser::*;

use crate::Game;
use crate::{Die, Score};

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
            parsers.push(Box::new(score_parser(crate::available_scores(
                &self.remaining_dice,
            ))));
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

pub fn score_parser(available: Vec<Score>) -> impl Parser<T = Command> {
    let available: Vec<Box<dyn Parser<T = Vec<Die>>>> = available
        .into_iter()
        .map(|s| Box::new(score_dice_parser(s)) as Box<dyn Parser<T = Vec<Die>>>)
        .collect();
    Map::new(
        Chain2::new(
            Doc::name_desc("score", "score dice", Token::new("score")),
            AfterSpace::new(Doc::name_desc(
                "dice",
                "the dice to score",
                OneOf::new(available),
            )),
        ),
        |(_, dice)| Command::Score { dice },
    )
}

pub fn score_dice_parser(score: Score) -> impl Parser<T = Vec<Die>> {
    let name: String = score.dice.iter().map(|d| d.name()).collect();
    let desc = format!("{} points", score.value);
    let token_name = name.clone();
    Map::new(
        Doc::name_desc(name, desc, Token::new(token_name)),
        move |_| score.dice.clone(),
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
