//! Ported from `brdgme-go/splendor_1/command.go`, `take_command.go`,
//! `buy_command.go`, `reserve_command.go`, `discard_command.go`,
//! `visit_command.go`.

use brdgme_game::command::parser::*;

use crate::Game;
use crate::card::{GEMS, Resource};

/// Ported from `command.go`'s `ParsedLoc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedLoc {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Buy(ParsedLoc),
    Discard(Vec<Resource>),
    Reserve(ParsedLoc),
    Take(Vec<Resource>),
    Visit(usize),
}

/// A loc choice paired with its positional letter+row name, for use with the
/// `Enum` parser. Port of `LocParser`'s `EnumValue`s (`command.go`).
#[derive(Debug, Clone, Copy)]
struct LocChoice {
    loc: ParsedLoc,
    name: [u8; 2],
}

impl std::fmt::Display for LocChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.name[0] as char, self.name[1] as char)
    }
}

/// A resource token choice paired with its `ResourceStrings` name, for use
/// with the `Enum` parser. Port of `TokenParser`'s `EnumValue`s
/// (`command.go`).
#[derive(Debug, Clone, Copy)]
struct TokenChoice {
    resource: Resource,
}

impl std::fmt::Display for TokenChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.resource.name())
    }
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        // Fixed order per `CommandParser` (`command.go`): buy, discard,
        // reserve, take, visit.
        if self.can_buy(player) {
            parsers.push(Box::new(self.buy_parser(player)));
        }
        if self.can_discard(player) {
            parsers.push(Box::new(discard_parser()));
        }
        if self.can_reserve(player) {
            parsers.push(Box::new(self.reserve_parser(player)));
        }
        if self.can_take(player) {
            parsers.push(Box::new(take_parser()));
        }
        if self.can_visit(player) {
            parsers.push(Box::new(self.visit_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    /// Port of `LocParser` (`command.go`): board locations named
    /// `{'A'+col}{row+1}`, then the player's own reserve locations named
    /// `{'A'+col}4`. Computed fresh from current board/reserve state on
    /// every call - column letters are positional, not stable ids.
    fn loc_parser(&self, player: usize) -> impl Parser<T = ParsedLoc> {
        let mut values: Vec<LocChoice> = vec![];
        for (row, cards) in self.board.iter().enumerate() {
            for col in 0..cards.len() {
                values.push(LocChoice {
                    loc: ParsedLoc { row, col },
                    name: [b'A' + col as u8, b'1' + row as u8],
                });
            }
        }
        for col in 0..self.player_boards[player].reserve.len() {
            values.push(LocChoice {
                loc: ParsedLoc { row: 3, col },
                name: [b'A' + col as u8, b'4'],
            });
        }
        Map::new(Enum::exact(values), |c: LocChoice| c.loc)
    }

    pub fn buy_parser(&self, player: usize) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("buy", "buy a card", Token::new("buy")),
                AfterSpace::new(Doc::name_desc(
                    "card",
                    "the card to buy",
                    self.loc_parser(player),
                )),
            ),
            |(_, loc)| Command::Buy(loc),
        )
    }

    pub fn reserve_parser(&self, player: usize) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "reserve",
                    "reserve a card and take a gold",
                    Token::new("reserve"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "card",
                    "the card to reserve",
                    self.loc_parser(player),
                )),
            ),
            |(_, loc)| Command::Reserve(loc),
        )
    }

    pub fn visit_parser(&self) -> impl Parser<T = Command> {
        // Port of `VisitParser` (`command.go`): accepts any of `len(Nobles)`
        // by 1-based index, not just currently-affordable ones (quirk 1).
        Map::new(
            Chain2::new(
                Doc::name_desc("visit", "visit a noble", Token::new("visit")),
                AfterSpace::new(Doc::name_desc(
                    "noble",
                    "the noble to visit",
                    Int {
                        min: Some(1),
                        max: Some(self.nobles.len() as i32),
                    },
                )),
            ),
            |(_, noble)| Command::Visit((noble - 1) as usize),
        )
    }
}

/// Port of `TokenParser` (`command.go`).
fn token_parser(include_gold: bool) -> impl Parser<T = Resource> {
    let mut values: Vec<TokenChoice> = GEMS.iter().map(|&r| TokenChoice { resource: r }).collect();
    if include_gold {
        values.push(TokenChoice {
            resource: Resource::Gold,
        });
    }
    Map::new(Enum::exact(values), |c: TokenChoice| c.resource)
}

/// Port of `TokensParser` (`command.go`): one-or-more space-delimited tokens.
fn tokens_parser(include_gold: bool) -> impl Parser<T = Vec<Resource>> {
    Many::some_spaced(token_parser(include_gold))
}

pub fn take_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "take",
                "take 3 different tokens, or 2 of the same token",
                Token::new("take"),
            ),
            AfterSpace::new(Doc::name_desc(
                "tokens",
                "the tokens to take",
                tokens_parser(false),
            )),
        ),
        |(_, tokens)| Command::Take(tokens),
    )
}

pub fn discard_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "discard",
                "discard tokens back down to 10",
                Token::new("discard"),
            ),
            AfterSpace::new(Doc::name_desc(
                "tokens",
                "the tokens to discard",
                tokens_parser(true),
            )),
        ),
        |(_, tokens)| Command::Discard(tokens),
    )
}
