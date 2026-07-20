use brdgme_game::command::parser::*;

use crate::Game;
use crate::card::{Module, Resource, SectorCard};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Choose {
        module: Module,
    },
    Gain {
        resource: Resource,
    },
    Put {
        num: usize,
        on: PutWhere,
    },
    Sector {
        sector: i32,
    },
    Build {
        resource: Resource,
    },
    Upgrade {
        module: Module,
    },
    Buy {
        amount: i32,
        resource: Option<Resource>,
    },
    Sell {
        amount: i32,
        resource: Option<Resource>,
    },
    Take {
        resource: Resource,
    },
    Done,
    Next,
    End,
    Found,
    Fight,
    Pay,
    Lose {
        module: Module,
    },
    Complete {
        adventure: usize,
    },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PutWhere {
    Top,
    Bottom,
}

fn found_parser() -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Doc::name_desc(
            "found",
            "found a colony or trading post here",
            Token::new("found"),
        ),
        |_| Command::Found,
    ))
}

fn fight_parser() -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Doc::name_desc("fight", "fight the pirate", Token::new("fight")),
        |_| Command::Fight,
    ))
}

fn pay_parser() -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Doc::name_desc("pay", "pay the pirate ransom", Token::new("pay")),
        |_| Command::Pay,
    ))
}

fn lose_parser() -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Chain2::new(
            Doc::name_desc(
                "lose",
                "choose which module was destroyed, eg. lose sensor",
                Token::new("lose"),
            ),
            AfterSpace::new(Enum::partial(Module::ALL.to_vec())),
        ),
        |(_, module): (String, Module)| Command::Lose { module },
    ))
}

fn complete_parser() -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Chain2::new(
            Doc::name_desc(
                "complete",
                "complete an adventure, eg. complete 2",
                Token::new("complete"),
            ),
            AfterSpace::new(Int::positive()),
        ),
        |(_, adventure): (String, i32)| Command::Complete {
            adventure: adventure as usize,
        },
    ))
}

fn buy_parser(tradable: Vec<Resource>) -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Chain3::new(
            Doc::name_desc(
                "buy",
                "buy resources, eg. buy 3 or buy 3 food",
                Token::new("buy"),
            ),
            AfterSpace::new(Int::positive()),
            Opt::new(AfterSpace::new(Enum::partial(tradable))),
        ),
        |(_, amount, resource): (String, i32, Option<Resource>)| Command::Buy { amount, resource },
    ))
}

fn sell_parser(tradable: Vec<Resource>) -> Box<dyn Parser<T = Command>> {
    Box::new(Map::new(
        Chain3::new(
            Doc::name_desc(
                "sell",
                "sell resources, eg. sell 3 or sell 3 food",
                Token::new("sell"),
            ),
            AfterSpace::new(Int::positive()),
            Opt::new(AfterSpace::new(Enum::partial(tradable))),
        ),
        |(_, amount, resource): (String, i32, Option<Resource>)| Command::Sell { amount, resource },
    ))
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<dyn Parser<T = Command> + '_>> {
        if !self.whose_turn().contains(&player) {
            return None;
        }
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];

        if self.gain_resources.is_none() && !self.flight_cards.is_empty() && !self.card_finished {
            match self.flight_cards.last() {
                Some(SectorCard::Colony { .. }) if self.can_found_colony(player) => {
                    parsers.push(found_parser());
                }
                Some(SectorCard::Trade { .. }) if self.can_found_trading_post(player) => {
                    parsers.push(found_parser());
                }
                Some(SectorCard::Pirate { .. }) => {
                    if self.can_fight(player) {
                        parsers.push(fight_parser());
                    }
                    if self.can_pay_ransom(player) {
                        parsers.push(pay_parser());
                    }
                    if self.can_lose_module(player) {
                        parsers.push(lose_parser());
                    }
                }
                Some(SectorCard::Median) if self.can_found_trading_post(player) => {
                    parsers.push(found_parser());
                }
                Some(SectorCard::AdventurePlanet { .. }) if self.can_complete(player) => {
                    parsers.push(complete_parser());
                }
                _ => {}
            }
        }

        if self.can_choose(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "choose",
                        "choose which module to start with, eg. choose lo",
                        Token::new("choose"),
                    ),
                    AfterSpace::new(Enum::partial(Module::ALL.to_vec())),
                ),
                |(_, module): (String, Module)| Command::Choose { module },
            )));
        }

        if self.can_gain(player)
            && let Some(gr) = self.gain_resources.clone()
        {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "gain",
                        "gain a resource of your choice, eg. gain sci",
                        Token::new("gain"),
                    ),
                    AfterSpace::new(Enum::partial(gr)),
                ),
                |(_, resource): (String, Resource)| Command::Gain { resource },
            )));
        }

        if self.can_put(player) {
            parsers.push(Box::new(Map::new(
                Chain3::new(
                    Doc::name_desc(
                        "put",
                        "put a peeked card on the top or bottom of the pile, eg. put 1 bottom",
                        Token::new("put"),
                    ),
                    AfterSpace::new(Int::positive()),
                    AfterSpace::new(Enum::partial(vec!["top".to_string(), "bottom".to_string()])),
                ),
                |(_, num, on): (String, i32, String)| Command::Put {
                    num: num as usize,
                    on: if on == "top" {
                        PutWhere::Top
                    } else {
                        PutWhere::Bottom
                    },
                },
            )));
        }

        if self.can_sector(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "sector",
                        "choose which sector to travel through, between 1 and 4, eg. sector 3",
                        Token::new("sector"),
                    ),
                    AfterSpace::new(Int::bounded(1, 4)),
                ),
                |(_, sector): (String, i32)| Command::Sector { sector },
            )));
        }

        if self.can_build(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "build",
                        "build a trade ship, colony ship, cannon or booster, eg. build colony",
                        Token::new("build"),
                    ),
                    AfterSpace::new(Enum::partial(Resource::BUILDABLES.to_vec())),
                ),
                |(_, resource): (String, Resource)| Command::Build { resource },
            )));
        }

        if self.can_upgrade(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "upgrade",
                        "upgrade a module, eg. upgrade logistics",
                        Token::new("upgrade"),
                    ),
                    AfterSpace::new(Enum::partial(Module::ALL.to_vec())),
                ),
                |(_, module): (String, Module)| Command::Upgrade { module },
            )));
        }

        let tradable = self.tradable_resources();
        if self.can_buy(player) {
            parsers.push(buy_parser(tradable.clone()));
        }
        if self.can_sell(player) {
            parsers.push(sell_parser(tradable.clone()));
        }

        if self.can_take(player) {
            parsers.push(Box::new(Map::new(
                Chain2::new(
                    Doc::name_desc(
                        "take",
                        "take a good from your opponent for $2, eg. take carbon",
                        Token::new("take"),
                    ),
                    AfterSpace::new(Enum::partial(Resource::GOODS.to_vec())),
                ),
                |(_, resource): (String, Resource)| Command::Take { resource },
            )));
        }

        if self.can_done(player) {
            parsers.push(Box::new(Map::new(
                Doc::name_desc("done", "end your turn", Token::new("done")),
                |_| Command::Done,
            )));
        }

        if self.can_next(player) {
            parsers.push(Box::new(Map::new(
                Doc::name_desc("next", "advance to the next card", Token::new("next")),
                |_| Command::Next,
            )));
        }

        if self.can_end(player) {
            parsers.push(Box::new(Map::new(
                Doc::name_desc("end", "end the flight early", Token::new("end")),
                |_| Command::End,
            )));
        }

        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }
}
