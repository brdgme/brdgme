use brdgme_game::command::parser::*;

use crate::Game;
use crate::card::Card;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Add(Card),
    Bid(i32),
    Buy,
    Pass,
    Play(Card),
    Price(i32),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_add(player) {
            parsers.push(Box::new(self.add_parser(player)));
        }
        if self.can_bid(player) {
            parsers.push(Box::new(self.bid_parser(player)));
        }
        if self.can_buy(player) {
            parsers.push(Box::new(buy_parser()));
        }
        if self.can_pass(player) {
            parsers.push(Box::new(pass_parser()));
        }
        if self.can_play(player) {
            parsers.push(Box::new(self.play_parser(player)));
        }
        if self.can_set_price(player) {
            parsers.push(Box::new(price_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    fn cards_parser(&self, player: usize) -> impl Parser<T = Card> {
        let mut hand = self.player_hands[player].clone();
        hand.sort();
        Enum::exact(hand)
    }

    pub fn add_parser(&self, player: usize) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "add",
                    "add a card from the same artist to the auction",
                    Token::new("add"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "card",
                    "the card to add",
                    self.cards_parser(player),
                )),
            ),
            |(_, c)| Command::Add(c),
        )
    }

    pub fn bid_parser(&self, player: usize) -> impl Parser<T = Command> {
        let min = self.min_bid();
        let max = self.player_money[player];
        Map::new(
            Chain2::new(
                Doc::name_desc("bid", "bid for an artwork", Token::new("bid")),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "the amount to bid",
                    Int {
                        min: Some(min),
                        max: Some(max),
                    },
                )),
            ),
            |(_, amount)| Command::Bid(amount),
        )
    }

    pub fn play_parser(&self, player: usize) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "play",
                    "play a card from your hand and put it up for auction",
                    Token::new("play"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "card",
                    "the card to play",
                    self.cards_parser(player),
                )),
            ),
            |(_, c)| Command::Play(c),
        )
    }
}

pub fn buy_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "buy",
        "buy the painting for the asking price",
        Map::new(Token::new("buy"), |_| Command::Buy),
    )
}

pub fn pass_parser() -> impl Parser<T = Command> {
    Doc::name_desc(
        "pass",
        "pass and leave the auction",
        Map::new(Token::new("pass"), |_| Command::Pass),
    )
}

pub fn price_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "price",
                "set the asking price for the artwork",
                Token::new("price"),
            ),
            AfterSpace::new(Doc::name_desc(
                "amount",
                "the amount to set as the asking price",
                Int {
                    min: Some(1),
                    max: None,
                },
            )),
        ),
        |(_, amount)| Command::Price(amount),
    )
}
