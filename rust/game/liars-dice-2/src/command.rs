use brdgme_game::Gamer;
use brdgme_game::command::parser::*;

use crate::Game;
use crate::{MAX_BID_VALUE, MIN_BID_QUANTITY, MIN_BID_VALUE};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Bid { quantity: i32, value: i32 },
    Call,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_bid(player) {
            parsers.push(Box::new(bid_parser()));
        }
        if self.can_call(player) {
            parsers.push(Box::new(call_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn bid_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain3::new(
            Doc::name_desc(
                "bid",
                "bid the number of dice under all players' cups",
                Token::new("bid"),
            ),
            AfterSpace::new(Doc::name_desc(
                "quantity",
                "the quantity of dice to bid",
                Int {
                    min: Some(MIN_BID_QUANTITY),
                    max: None,
                },
            )),
            AfterSpace::new(Doc::name_desc(
                "value",
                "the face value of dice to bid, including wild dice (1)",
                Int {
                    min: Some(MIN_BID_VALUE),
                    max: Some(MAX_BID_VALUE),
                },
            )),
        ),
        |(_, quantity, value)| Command::Bid { quantity, value },
    )
}

pub fn call_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("call", "call that the bid is too high", Token::new("call")),
        |_| Command::Call,
    )
}
