use brdgme_game::Gamer;
use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Bid(i32),
    Pass,
    Play(i32),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_bid(player) {
            parsers.push(Box::new(bid_parser(self.chips[player])));
        }
        if self.can_pass(player) {
            parsers.push(Box::new(pass_parser()));
        }
        if self.can_play(player) {
            parsers.push(Box::new(play_parser(self.hands[player].clone())));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn bid_parser(max: i32) -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("bid", "bid for a building", Token::new("bid")),
            AfterSpace::new(Doc::name_desc(
                "amount",
                "the amount to bid",
                Int {
                    min: None,
                    max: Some(max),
                },
            )),
        ),
        |(_, amount): (String, i32)| Command::Bid(amount),
    )
}

pub fn pass_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc("pass", "pass from further bidding", Token::new("pass")),
        |_| Command::Pass,
    )
}

pub fn play_parser(hand: Vec<i32>) -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc("play", "play a building card", Token::new("play")),
            AfterSpace::new(Doc::name_desc(
                "building",
                "the building card to play",
                Enum::exact(hand),
            )),
        ),
        |(_, building): (String, i32)| Command::Play(building),
    )
}
