use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Play(crate::Card),
    Choose(usize),
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_choose(player) {
            parsers.push(Box::new(self.choose_parser()));
        }
        if self.can_play(player) {
            parsers.push(Box::new(self.play_parser(player)));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    pub fn choose_parser(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("choose", "choose the row to take", Token::new("choose")),
                AfterSpace::new(Doc::name_desc(
                    "row",
                    "the row to take",
                    Int::bounded(1, crate::ROWS as i32),
                )),
            ),
            |(_, row)| Command::Choose(row as usize),
        )
    }

    pub fn play_parser(&self, player: usize) -> impl Parser<T = Command> {
        let mut cards = self.hands.get(player).cloned().unwrap_or_default();
        cards.sort();
        cards.dedup();
        Map::new(
            Chain2::new(
                Doc::name_desc("play", "play a card", Token::new("play")),
                AfterSpace::new(Doc::name_desc(
                    "card",
                    "the card to play",
                    Enum::exact(cards),
                )),
            ),
            |(_, card)| Command::Play(card),
        )
    }
}
