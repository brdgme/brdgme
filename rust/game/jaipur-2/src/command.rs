use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::*;
use brdgme_game::errors::GameError;

use crate::{Game, Good};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Take { take: Vec<Good>, give: Vec<Good> },
    Sell { good: Good, quantity: usize },
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if self.is_finished() || self.current_player != player {
            return None;
        }
        let parsers: Vec<Box<dyn Parser<T = Command>>> =
            vec![Box::new(take_parser()), Box::new(sell_parser())];
        Some(Box::new(OneOf::new(parsers)))
    }
}

fn good_parser() -> impl Parser<T = Good> {
    Enum::partial(Good::all_goods().to_vec())
}

fn trade_good_parser() -> Enum<Good> {
    Enum::partial(Good::trade_goods().to_vec())
}

#[allow(clippy::type_complexity)]
fn take_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain3::new(
            Doc::name_desc(
                "take",
                "take cards from the market, eg. take dia or take dia silv for camel spi",
                Token::new("take"),
            ),
            AfterSpace::new(Many::some_spaced(good_parser())),
            Opt::new(Chain2::new(
                AfterSpace::new(Token::new("for")),
                AfterSpace::new(Many::some_spaced(good_parser())),
            )),
        ),
        |(_, take_goods, opt_for): (String, Vec<Good>, Option<(String, Vec<Good>)>)| {
            let give_goods = opt_for.map(|(_, g)| g).unwrap_or_default();
            Command::Take {
                take: take_goods,
                give: give_goods,
            }
        },
    )
}

/// Parses the bare-goods form of a sell command, eg. `sell dia dia`.
///
/// This exists instead of a `Map` over `Many` because `Map`'s closure is
/// infallible, and two things here must be able to fail the parse:
/// mixed good types (which used to be silently truncated to the first type,
/// executing an unintended sale) and an empty list (which used to fall back
/// to `Good::Diamond`, hiding any parser regression).
struct SellGoodsParser {
    inner: Many<Enum<Good>, Space>,
}

impl Parser for SellGoodsParser {
    type T = Command;

    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Command>, GameError> {
        let out = self.inner.parse(input, names)?;
        let Some(&good) = out.value.first() else {
            // Unreachable while `inner` is a `some_spaced` Many, but stated as
            // an error rather than an unwrap or a default so a regression is
            // loud and never a panic on a player-reachable path.
            return Err(GameError::Parse {
                message: Some("you must name at least one good to sell".to_string()),
                expected: self.expected(names),
                offset: 0,
            });
        };
        if let Some(other) = out.value.iter().find(|&&g| g != good) {
            return Err(GameError::Parse {
                message: Some(format!(
                    "you can only sell one type of good at a time, got {good} and {other}"
                )),
                expected: self.expected(names),
                // A non-zero offset makes OneOf prefer this message over the
                // sibling "sell <n> <good>" parser's offset-0 failure.
                offset: out.consumed.len(),
            });
        }
        Ok(Output {
            value: Command::Sell {
                good,
                quantity: out.value.len(),
            },
            consumed: out.consumed,
            remaining: out.remaining,
        })
    }

    fn expected(&self, names: &[String]) -> Vec<String> {
        self.inner.expected(names)
    }

    fn to_spec(&self) -> CommandSpec {
        self.inner.to_spec()
    }
}

fn sell_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain2::new(
            Doc::name_desc(
                "sell",
                "sell goods for tokens, eg. sell 2 dia or sell dia dia",
                Token::new("sell"),
            ),
            AfterSpace::new({
                let p1: Box<dyn Parser<T = Command>> = Box::new(Map::new(
                    Chain2::new(Int::positive(), AfterSpace::new(trade_good_parser())),
                    |(q, good): (i32, Good)| Command::Sell {
                        good,
                        quantity: q as usize,
                    },
                ));
                let p2: Box<dyn Parser<T = Command>> = Box::new(SellGoodsParser {
                    inner: Many::some_spaced(trade_good_parser()),
                });
                OneOf::new(vec![p1, p2])
            }),
        ),
        |(_, cmd)| cmd,
    )
}
