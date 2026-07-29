use brdgme_game::Gamer;
use brdgme_game::command::parser::*;

use crate::card::Geisha;
use crate::{Game, Pending, Phase};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Command {
    Secret(Geisha),
    Trade(Geisha, Geisha),
    Gift(Geisha, Geisha, Geisha),
    Compete(Geisha, Geisha, Geisha, Geisha),
    ChooseCard(Geisha),
    ChooseSet(usize),
}

pub fn geisha_parser() -> impl Parser<T = Geisha> {
    Enum::exact(Geisha::ALL.to_vec())
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        match self.phase {
            Phase::ChooseAction if player == self.current => {
                let hand = self.hands.get(player).map(Vec::len).unwrap_or(0);
                let used = self.used.get(player).copied().unwrap_or([true; 4]);
                if !used[0] && hand >= 1 {
                    parsers.push(Box::new(secret_parser()));
                }
                if !used[1] && hand >= 2 {
                    parsers.push(Box::new(trade_parser()));
                }
                if !used[2] && hand >= 3 {
                    parsers.push(Box::new(gift_parser()));
                }
                if !used[3] && hand >= 4 {
                    parsers.push(Box::new(compete_parser()));
                }
            }
            Phase::OpponentChoose if player == 1 - self.current => match &self.pending {
                Some(Pending::Gift { cards, .. }) => {
                    parsers.push(Box::new(choose_card_parser(cards)));
                }
                Some(Pending::Competition { .. }) => {
                    parsers.push(Box::new(choose_set_parser()));
                }
                None => {}
            },
            _ => {}
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

pub fn secret_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "secret",
                "play one card face-down; it is revealed and scored at the end of the round",
                Token::new("secret"),
            ),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the card to play face-down",
                geisha_parser(),
            )),
        ),
        |(_, g)| Command::Secret(g),
    )
}

pub fn trade_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain3::new(
            Doc::name_desc(
                "trade",
                "set two cards aside face-down, out of the round (not scored)",
                Token::new("trade"),
            ),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the first card to set aside",
                geisha_parser(),
            )),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the second card to set aside",
                geisha_parser(),
            )),
        ),
        |(_, a, b)| Command::Trade(a, b),
    )
}

pub fn gift_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain4::new(
            Doc::name_desc(
                "gift",
                "offer three cards face-up; your opponent takes one, you place the other two",
                Token::new("gift"),
            ),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the first gift card",
                geisha_parser(),
            )),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the second gift card",
                geisha_parser(),
            )),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the third gift card",
                geisha_parser(),
            )),
        ),
        |(_, a, b, c)| Command::Gift(a, b, c),
    )
}

pub fn compete_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Chain3::new(
                Doc::name_desc(
                    "compete",
                    "play four cards as two face-up pairs; your opponent takes one pair, you place the other",
                    Token::new("compete"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "geisha",
                    "first card of your first pair",
                    geisha_parser(),
                )),
                AfterSpace::new(Doc::name_desc(
                    "geisha",
                    "second card of your first pair",
                    geisha_parser(),
                )),
            ),
            Chain2::new(
                AfterSpace::new(Doc::name_desc(
                    "geisha",
                    "first card of your second pair",
                    geisha_parser(),
                )),
                AfterSpace::new(Doc::name_desc(
                    "geisha",
                    "second card of your second pair",
                    geisha_parser(),
                )),
            ),
        ),
        |((_, a, b), (c, d))| Command::Compete(a, b, c, d),
    )
}

pub fn choose_card_parser(cards: &[Geisha]) -> impl Parser<T = Command> {
    let mut distinct = cards.to_vec();
    distinct.sort_by_key(|g| g.index());
    distinct.dedup();
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "choose",
                "choose one of the gift cards to place on your side",
                Token::new("choose"),
            ),
            AfterSpace::new(Doc::name_desc(
                "geisha",
                "the gift card to take",
                Enum::exact(distinct),
            )),
        ),
        |(_, g)| Command::ChooseCard(g),
    )
}

pub fn choose_set_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "choose",
                "choose one of the two pairs to place on your side",
                Token::new("choose"),
            ),
            AfterSpace::new(Doc::name_desc(
                "set",
                "the pair to take, 1 or 2",
                Int::bounded(1, 2),
            )),
        ),
        |(_, n)| Command::ChooseSet((n - 1) as usize),
    )
}
