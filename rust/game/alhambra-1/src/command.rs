use brdgme_game::command::Spec;
use brdgme_game::command::parser::*;
use brdgme_game::errors::GameError;

use crate::card::{Card, Grid, Vect, grid_parse_coord};
use crate::{Game, Phase};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Take { cards: Vec<Card> },
    Spend { cards: Vec<Card> },
    Place { tile: usize, coord: Vect },
    Swap { tile: usize, coord: Vect },
    Remove { coord: Vect },
    Done,
}

struct CardParser;

impl Parser for CardParser {
    type T = Card;

    fn parse<'a>(&self, input: &'a str, _names: &[String]) -> Result<Output<'a, Card>, GameError> {
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 2 || !chars[0].is_ascii_alphabetic() {
            return Err(GameError::Parse {
                message: Some(
                    "cards must be a letter followed by a number, such as R10".to_string(),
                ),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        let mut end = 1;
        while end < chars.len() && chars[end].is_ascii_digit() {
            end += 1;
        }
        if end == 1 {
            return Err(GameError::Parse {
                message: Some(
                    "cards must be a letter followed by a number, such as R10".to_string(),
                ),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        let byte_end: usize = chars[..end].iter().map(|c| c.len_utf8()).sum();
        match Card::parse(&input[..byte_end]) {
            Some(card) => Ok(Output {
                value: card,
                consumed: &input[..byte_end],
                remaining: &input[byte_end..],
            }),
            None => Err(GameError::Parse {
                message: Some(
                    "cards must be a letter followed by a number, such as R10".to_string(),
                ),
                expected: self.expected(_names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec!["a card like R10 or b3".to_string()]
    }

    fn to_spec(&self) -> Spec {
        Spec::Token("card".to_string())
    }
}

struct CoordParser {
    grid: Grid,
}

impl Parser for CoordParser {
    type T = Vect;

    fn parse<'a>(&self, input: &'a str, _names: &[String]) -> Result<Output<'a, Vect>, GameError> {
        let chars: Vec<char> = input.chars().collect();
        let mut end = 0;
        while end < chars.len() && chars[end].is_ascii_alphanumeric() {
            end += 1;
        }
        if end == 0 {
            return Err(GameError::Parse {
                message: Some("coord must be numbers and letters, like a4 or 4a".to_string()),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        let byte_end: usize = chars[..end].iter().map(|c| c.len_utf8()).sum();
        let token = &input[..byte_end];
        match grid_parse_coord(&self.grid, token) {
            Ok(v) => Ok(Output {
                value: v,
                consumed: &input[..byte_end],
                remaining: &input[byte_end..],
            }),
            Err(msg) => Err(GameError::Parse {
                message: Some(msg),
                expected: self.expected(_names),
                offset: 0,
            }),
        }
    }

    fn expected(&self, _names: &[String]) -> Vec<String> {
        vec!["a coordinate like a4 or 4a".to_string()]
    }

    fn to_spec(&self) -> Spec {
        Spec::Token("coord".to_string())
    }
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.phase == Phase::End || self.current_player != player {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];

        if self.can_spend(player) {
            let hand_cards: Vec<String> = self.boards[player]
                .cards
                .iter()
                .map(|c| c.to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "spend",
                        "spend cards of a single currency to buy a tile, eg. spend r3 r4",
                        Token::new("spend"),
                    ),
                    AfterSpace::new(Many::some_spaced(Map::new(
                        Enum::exact(hand_cards),
                        |s: String| Card::parse(&s).unwrap(),
                    ))),
                ),
                |(_, cards): (String, Vec<Card>)| Command::Spend { cards },
            )));
        }

        if self.can_take(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "take",
                        "take multiple cards up to the value of 5, or a single card over the value of 5, eg. take r2 b3",
                        Token::new("take"),
                    ),
                    AfterSpace::new(Many::some_spaced(CardParser)),
                ),
                |(_, cards): (String, Vec<Card>)| Command::Take { cards },
            )));
        }

        if self.can_place(player) {
            let grid = self.boards[player].grid.clone();
            parsers.push(Box::new(Map::new(
                Chain3::new(
                    Doc::name_desc(
                        "place",
                        "place a tile in your Alhambra, eg. place 1 b3",
                        Token::new("place"),
                    ),
                    AfterSpace::new(Map::new(Int::positive(), |n: i32| (n - 1) as usize)),
                    AfterSpace::new(CoordParser { grid }),
                ),
                |(_, tile, coord): (String, usize, Vect)| Command::Place { tile, coord },
            )));
        }

        if self.can_swap(player) {
            let grid = self.boards[player].grid.clone();
            parsers.push(Box::new(Map::new(
                Chain3::new(
                    Doc::name_desc(
                        "swap",
                        "swap a tile between your reserve and your Alhambra, eg. swap 2 b4",
                        Token::new("swap"),
                    ),
                    AfterSpace::new(Map::new(Int::positive(), |n: i32| (n - 1) as usize)),
                    AfterSpace::new(CoordParser { grid }),
                ),
                |(_, tile, coord): (String, usize, Vect)| Command::Swap { tile, coord },
            )));
        }

        if self.can_remove(player) {
            let grid = self.boards[player].grid.clone();
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "remove",
                        "remove a tile from your Alhambra to your reserve, eg. remove b4",
                        Token::new("remove"),
                    ),
                    AfterSpace::new(CoordParser { grid }),
                ),
                |(_, coord): (String, Vect)| Command::Remove { coord },
            )));
        }

        if self.can_done(player) {
            parsers.push(Box::new(Map::new(
                Doc::name_desc(
                    "done",
                    "end your turn and put all remaining placeable tiles in your reserve",
                    Token::new("done"),
                ),
                |_| Command::Done,
            )));
        }

        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}
