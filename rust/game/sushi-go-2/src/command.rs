use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Play(Vec<usize>),
    Dummy(usize),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_play(player) {
            parsers.push(Box::new(play_parser(self.hands[player].len())));
        }
        if self.can_dummy(player) {
            parsers.push(Box::new(dummy_parser(self.hands[player].len())));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn play_parser(max: usize) -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "play",
                "play a card, or two cards if you have previously played chopsticks",
                Token::new("play"),
            ),
            AfterSpace::new(Doc::name_desc(
                "card",
                "the card to play",
                Many::bounded_spaced(Int::bounded(1, max as i32), 1, 2),
            )),
        ),
        |(_, nums): (String, Vec<i32>)| {
            Command::Play(nums.iter().map(|n| (*n - 1) as usize).collect())
        },
    )
}

pub fn dummy_parser(max: usize) -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "dummy",
                "play a card for the dummy player",
                Token::new("dummy"),
            ),
            AfterSpace::new(Doc::name_desc(
                "card",
                "the card to play",
                Int::bounded(1, max as i32),
            )),
        ),
        |(_, n): (String, i32)| Command::Dummy((n - 1) as usize),
    )
}
