use brdgme_game::command::parser::*;

use crate::Game;

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Build { card: usize },
    Free { card: usize },
    Wonder { card: usize },
    Discard { card: usize },
    Deal { deal: usize },
    Take { card: usize },
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.finished {
            return None;
        }

        if let Some(crate::Resolver::DrawDiscard { player: rp }) = self.to_resolve.first() {
            if *rp != player {
                return None;
            }
            let parsers: Vec<Box<dyn Parser<T = Command>>> = vec![Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "take",
                        "take a card from the discard pile for free, eg. take 1",
                        Token::new("take"),
                    ),
                    AfterSpace::new(Map::new(Int::positive(), |n: i32| (n - 1) as usize)),
                ),
                |(_, card): (String, usize)| Command::Take { card },
            ))];
            return Some(Box::new(OneOf::new(parsers)));
        }

        if let Some(crate::Action::Build { chosen: false, .. }) = &self.actions[player] {
            let parsers: Vec<Box<dyn Parser<T = Command>>> = vec![Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "deal",
                        "choose a trade deal for neighbor resources, eg. deal 1",
                        Token::new("deal"),
                    ),
                    AfterSpace::new(Map::new(Int::positive(), |n: i32| (n - 1) as usize)),
                ),
                |(_, deal): (String, usize)| Command::Deal { deal },
            ))];
            return Some(Box::new(OneOf::new(parsers)));
        }

        if self.actions[player].is_some() || self.hands[player].is_empty() {
            return None;
        }

        let hand_len = self.hands[player].len() as i32;
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];

        let any_buildable = (0..self.hands[player].len()).any(|i| self.can_build_card(player, i).0);
        if any_buildable {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "build",
                        "build a card from your hand, eg. build 1",
                        Token::new("build"),
                    ),
                    AfterSpace::new(Map::new(Int::bounded(1, hand_len), |n: i32| {
                        (n - 1) as usize
                    })),
                ),
                |(_, card): (String, usize)| Command::Build { card },
            )));
        }

        if self.has_free_build(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "free",
                        "build a card for free using a wonder ability, eg. free 2",
                        Token::new("free"),
                    ),
                    AfterSpace::new(Map::new(Int::bounded(1, hand_len), |n: i32| {
                        (n - 1) as usize
                    })),
                ),
                |(_, card): (String, usize)| Command::Free { card },
            )));
        }

        if self.can_build_wonder(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "wonder",
                        "build a wonder stage using a card from your hand, eg. wonder 3",
                        Token::new("wonder"),
                    ),
                    AfterSpace::new(Map::new(Int::bounded(1, hand_len), |n: i32| {
                        (n - 1) as usize
                    })),
                ),
                |(_, card): (String, usize)| Command::Wonder { card },
            )));
        }

        parsers.push(Box::new(Map::new(
            Chain2::new(
                Doc::name_desc(
                    "discard",
                    "discard a card from your hand for 3 coins, eg. discard 4",
                    Token::new("discard"),
                ),
                AfterSpace::new(Map::new(Int::bounded(1, hand_len), |n: i32| {
                    (n - 1) as usize
                })),
            ),
            |(_, card): (String, usize)| Command::Discard { card },
        )));

        Some(Box::new(OneOf::new(parsers)))
    }
}
