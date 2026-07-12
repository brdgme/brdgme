//! Port of `brdgme-go/roll_through_the_ages_1/command.go`.
//!
//! All 11 gated sub-parsers were wired up in Task 2, matching
//! `CommandParser`'s `OneOf` construction. Task 3 wires the remaining 8
//! `Command` variants (`Build`/`Trade`/`Buy`/`Take`/`Discard`/`Invade`/
//! `Sell`/`Swap`) to real actions in `lib.rs`'s `command()` dispatch.

use brdgme_game::command::parser::*;

use crate::development::DevelopmentId;
use crate::good::{GOODS, Good};
use crate::monument::MonumentId;
use crate::take::TakeAction;
use crate::{Game, player_board};

#[derive(Debug, PartialEq, Clone)]
pub enum BuildTarget {
    City,
    Ship,
    Monument(MonumentId),
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct BuyGoods {
    pub all_goods: bool,
    pub goods: Vec<Good>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Next,
    Roll {
        dice: Vec<i32>,
    },
    Preserve,
    Build {
        amount: i32,
        target: BuildTarget,
    },
    Trade {
        amount: i32,
    },
    Buy {
        development: DevelopmentId,
        goods: BuyGoods,
    },
    Take {
        actions: Vec<TakeAction>,
    },
    Discard {
        amount: i32,
        good: Good,
    },
    Invade {
        amount: i32,
    },
    Sell {
        amount: i32,
    },
    Swap {
        amount: i32,
        from: Good,
        to: Good,
    },
}

/// Wrapper so `DevelopmentId`/`MonumentId`/`Good`/`TakeAction` (which don't
/// implement `Display` themselves, to keep the domain modules free of
/// parser-layer concerns) can be used with the `Enum` parser, which
/// requires `ToString + Clone`. Port of the ad-hoc `brdgme.EnumValue`
/// name/value pairs built in each Go `*Parser` function.
#[derive(Debug, Clone, Copy)]
struct Choice<T> {
    name: &'static str,
    value: T,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn good_choices(goods: &[Good]) -> Vec<Choice<Good>> {
    goods
        .iter()
        .map(|&g| Choice {
            name: g.name(),
            value: g,
        })
        .collect()
}

/// Port of `GoodParser`.
fn good_parser() -> impl Parser<T = Good> {
    Map::new(Enum::partial(good_choices(&GOODS)), |c: Choice<Good>| {
        c.value
    })
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_build(player) {
            parsers.push(Box::new(self.build_parser(player)));
        }
        if self.can_buy(player) {
            parsers.push(Box::new(self.buy_parser(player)));
        }
        if self.can_trade(player) {
            parsers.push(Box::new(self.trade_parser(player)));
        }
        if self.can_next(player) {
            parsers.push(Box::new(next_parser()));
        }
        if self.can_take(player) {
            parsers.push(Box::new(self.take_parser()));
        }
        if self.can_discard(player) {
            parsers.push(Box::new(self.discard_parser(player)));
        }
        if self.can_invade(player) {
            parsers.push(Box::new(self.invade_parser(player)));
        }
        if self.can_roll(player) {
            parsers.push(Box::new(self.roll_parser()));
        }
        if self.can_sell(player) {
            parsers.push(Box::new(self.sell_parser(player)));
        }
        if self.can_preserve(player) {
            parsers.push(Box::new(preserve_parser()));
        }
        if self.can_swap(player) {
            parsers.push(Box::new(swap_parser()));
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    /// Port of `BuildParser`/`BuildTargetWorkerParser`/
    /// `BuildTargetShipParser`/`BuildTargetMonumentParser`.
    fn build_parser(&self, player: usize) -> impl Parser<T = Command> {
        let b = &self.boards[player];

        // BuildTargetWorkerParser: `<amount> city|<monument>`.
        let worker_variant: Option<Box<dyn Parser<T = Command>>> =
            if self.can_build_building(player) {
                let mut target_opts: Vec<Box<dyn Parser<T = BuildTarget>>> = vec![];
                if b.city_progress < player_board::MAX_CITY_PROGRESS {
                    target_opts.push(Box::new(Map::new(Token::new("city"), |_| {
                        BuildTarget::City
                    })));
                }
                let available_monuments = self.available_monuments(player);
                if !available_monuments.is_empty() {
                    let choices: Vec<Choice<MonumentId>> = available_monuments
                        .iter()
                        .map(|&m| Choice {
                            name: m.value().name,
                            value: m,
                        })
                        .collect();
                    target_opts.push(Box::new(Map::new(
                        Doc::name_desc("monument", "the monument to build", Enum::partial(choices)),
                        |c: Choice<MonumentId>| BuildTarget::Monument(c.value),
                    )));
                }
                let max = self.remaining_workers;
                Some(Box::new(Map::new(
                    Chain2::new(
                        Doc::name_desc("amount", "the amount to build", Int::bounded(1, max)),
                        AfterSpace::new(OneOf::new(target_opts)),
                    ),
                    |(amount, target): (i32, BuildTarget)| Command::Build { amount, target },
                )))
            } else {
                None
            };

        // BuildTargetShipParser: `<amount> ship`.
        let ship_variant: Option<Box<dyn Parser<T = Command>>> = if self.can_build_ship(player) {
            let wood = b.goods.get(&Good::Wood).copied().unwrap_or(0);
            let cloth = b.goods.get(&Good::Cloth).copied().unwrap_or(0);
            let max = wood.max(cloth);
            Some(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "amount",
                        "the amount of ships to build",
                        Int::bounded(1, max),
                    ),
                    AfterSpace::new(Token::new("ship")),
                ),
                |(amount, _): (i32, String)| Command::Build {
                    amount,
                    target: BuildTarget::Ship,
                },
            )))
        } else {
            None
        };

        let mut opts: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if let Some(p) = worker_variant {
            opts.push(p);
        }
        if let Some(p) = ship_variant {
            opts.push(p);
        }

        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "build",
                    "build a city, monument or ship",
                    Token::new("build"),
                ),
                AfterSpace::new(OneOf::new(opts)),
            ),
            |(_, cmd): (String, Command)| cmd,
        )
    }

    /// Port of `TradeParser`.
    fn trade_parser(&self, player: usize) -> impl Parser<T = Command> {
        let max = self.boards[player]
            .goods
            .get(&Good::Stone)
            .copied()
            .unwrap_or(0);
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "trade",
                    "trade stone for 3 workers each",
                    Token::new("trade"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "the amount of stone to trade",
                    Int::bounded(1, max),
                )),
            ),
            |(_, amount): (String, i32)| Command::Trade { amount },
        )
    }

    /// Port of `BuyParser`/`BuyDevelopmentParser`/`BuyGoodParser`.
    fn buy_parser(&self, player: usize) -> impl Parser<T = Command> {
        let available = self.available_developments(player);
        let choices: Vec<Choice<DevelopmentId>> = available
            .iter()
            .map(|&d| Choice {
                name: d.value().name,
                value: d,
            })
            .collect();
        let development_parser = Doc::name_desc(
            "development",
            "the development to buy",
            Enum::partial(choices),
        );

        let all_variant: Box<dyn Parser<T = BuyGoods>> =
            Box::new(Map::new(Token::new("all"), |_| BuyGoods {
                all_goods: true,
                goods: vec![],
            }));
        let goods_variant: Box<dyn Parser<T = BuyGoods>> = Box::new(Map::new(
            Many::bounded_spaced(good_parser(), 1, GOODS.len()),
            |goods: Vec<Good>| BuyGoods {
                all_goods: false,
                goods,
            },
        ));
        let goods_parser = OneOf::new(vec![all_variant, goods_variant]);

        Map::new(
            Chain3::new(
                Doc::name_desc("buy", "buy a development", Token::new("buy")),
                AfterSpace::new(development_parser),
                Opt::new(AfterSpace::new(goods_parser)),
            ),
            |(_, development, goods): (String, Choice<DevelopmentId>, Option<BuyGoods>)| {
                Command::Buy {
                    development: development.value,
                    goods: goods.unwrap_or_default(),
                }
            },
        )
    }

    /// Port of `TakeParser`.
    fn take_parser(&self) -> impl Parser<T = Command> {
        let max = self
            .kept_dice
            .iter()
            .filter(|&&d| d == crate::dice::Die::FoodOrWorkers)
            .count();
        let choices = vec![
            Choice {
                name: "food",
                value: TakeAction::Food,
            },
            Choice {
                name: "workers",
                value: TakeAction::Workers,
            },
        ];
        Map::new(
            Chain2::new(
                Doc::name_desc("take", "take food or workers from dice", Token::new("take")),
                AfterSpace::new(Many::bounded_spaced(Enum::partial(choices), 1, max)),
            ),
            |(_, actions): (String, Vec<Choice<TakeAction>>)| Command::Take {
                actions: actions.into_iter().map(|c| c.value).collect(),
            },
        )
    }

    /// Port of `DiscardParser`.
    fn discard_parser(&self, player: usize) -> impl Parser<T = Command> {
        let max = self.boards[player].goods_over_limit();
        Map::new(
            Chain3::new(
                Doc::name_desc(
                    "discard",
                    "discard goods down to the limit",
                    Token::new("discard"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "amount of goods to discard",
                    Int::bounded(1, max),
                )),
                AfterSpace::new(Doc::name_desc(
                    "good",
                    "type of good to discard",
                    good_parser(),
                )),
            ),
            |(_, amount, good): (String, i32, Good)| Command::Discard { amount, good },
        )
    }

    /// Port of `InvadeParser`.
    fn invade_parser(&self, player: usize) -> impl Parser<T = Command> {
        let max = self.boards[player]
            .goods
            .get(&Good::Spearhead)
            .copied()
            .unwrap_or(0);
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "invade",
                    "invade other players by spending spearheads for -2 points each",
                    Token::new("invade"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "amount of spearheads to spend",
                    Int::bounded(1, max),
                )),
            ),
            |(_, amount): (String, i32)| Command::Invade { amount },
        )
    }

    /// Port of `RollParser`.
    fn roll_parser(&self) -> impl Parser<T = Command> {
        let max_i = self.rolled_dice.len() as i32;
        let max_u = if self.phase == crate::Phase::ExtraRoll {
            1
        } else {
            max_i as usize
        };
        Map::new(
            Chain2::new(
                Doc::name_desc("roll", "roll dice", Token::new("roll")),
                AfterSpace::new(Doc::name_desc(
                    "dice",
                    "list of dice numbers to roll, separated by spaces",
                    Many::bounded_spaced(Int::bounded(1, max_i), 1, max_u),
                )),
            ),
            |(_, dice): (String, Vec<i32>)| Command::Roll { dice },
        )
    }

    /// Port of `SellParser`.
    fn sell_parser(&self, player: usize) -> impl Parser<T = Command> {
        let max = self.boards[player].food;
        Map::new(
            Chain2::new(
                Doc::name_desc("sell", "sell food for 6 coins each", Token::new("sell")),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "amount of food to sell",
                    Int::bounded(1, max),
                )),
            ),
            |(_, amount): (String, i32)| Command::Sell { amount },
        )
    }
}

/// Port of `NextParser`.
fn next_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "next",
            "continue to the next phase of your turn",
            Token::new("next"),
        ),
        |_| Command::Next,
    )
}

/// Port of `PreserveParser`.
fn preserve_parser() -> impl Parser<T = Command> {
    Map::new(
        Doc::name_desc(
            "preserve",
            "use 1 pottery to double your food",
            Token::new("preserve"),
        ),
        |_| Command::Preserve,
    )
}

/// Port of `SwapParser`.
fn swap_parser() -> impl Parser<T = Command> {
    Map::new(
        Chain4::new(
            Doc::name_desc(
                "swap",
                "swap one type of goods for a different type using shipping",
                Token::new("swap"),
            ),
            AfterSpace::new(Doc::name_desc(
                "amount",
                "amount of goods to swap",
                Int::bounded(1, i32::MAX),
            )),
            AfterSpace::new(Doc::name_desc(
                "from",
                "type of good to swap away",
                good_parser(),
            )),
            AfterSpace::new(Doc::name_desc("to", "type of good to gain", good_parser())),
        ),
        |(_, amount, from, to): (String, i32, Good, Good)| Command::Swap { amount, from, to },
    )
}
