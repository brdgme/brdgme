use brdgme_game::command::parser::*;

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
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        parsers.push(Box::new(take_parser()));
        parsers.push(Box::new(sell_parser()));
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}

fn good_parser() -> impl Parser<T = Good> {
    Enum::partial(Good::all_goods().to_vec())
}

fn trade_good_parser() -> impl Parser<T = Good> {
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
                let p2: Box<dyn Parser<T = Command>> = Box::new(Map::new(
                    Many::some_spaced(trade_good_parser()),
                    |goods: Vec<Good>| {
                        let good = goods.first().copied().unwrap_or(Good::Diamond);
                        Command::Sell {
                            good,
                            quantity: goods.len(),
                        }
                    },
                ));
                OneOf::new(vec![p1, p2])
            }),
        ),
        |(_, cmd)| cmd,
    )
}
