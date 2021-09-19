use brdgme_game::command::parser::*;

use crate::board::Loc;
use crate::casino::{Casino, CASINOS};
use crate::tile::TILES;
use crate::Game;

pub enum Command {
    Build { loc: Loc, casino: Casino },
    Sprawl { from: Loc, to: Loc },
    Remodel { loc: Loc, casino: Casino },
    Reorg { loc: Loc },
    Gamble { player: usize, amount: usize },
    Raise { loc: Loc },
    Done,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Box<dyn Parser<T = Command>> {
        let mut parsers: Vec<Box<dyn Parser<T = Command>>> = vec![];
        if self.can_build(player) {
            parsers.push(Box::new(self.build_parser(player)));
        }
        if self.can_done(player) {
            parsers.push(Box::new(done_parser()));
        }
        Box::new(OneOf::new(parsers))
    }

    pub fn build_parser(&self, player: usize) -> impl Parser<T = Command> {
        Map::new(
            Chain3::new(
                Doc::name_desc("build", "build a casino at a location", Token::new("build")),
                AfterSpace::new(Doc::name_desc(
                    "loc",
                    "the location to build at",
                    loc_parser(self.board.player_locs(player)),
                )),
                AfterSpace::new(Doc::name_desc(
                    "casino",
                    "the casino to build",
                    casino_parser(),
                )),
            ),
            |(_, loc, casino)| Command::Build { loc, casino },
        )
    }

    pub fn sprawl_parser(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain3::new(
                Doc::name_desc(
                    "sprawl",
                    "sprawl a casino you are the boss of to an adjacent location",
                    Token::new("sprawl"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "from",
                    "the casino to sprawl from",
                    loc_parser(TILES.keys().cloned().collect()),
                )),
                AfterSpace::new(Doc::name_desc(
                    "to",
                    "the empty location to sprawl to",
                    loc_parser(TILES.keys().cloned().collect()),
                )),
            ),
            |(_, from, to)| Command::Sprawl { from, to },
        )
    }

    pub fn remodel_action(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain3::new(
                Doc::name_desc(
                    "remodel",
                    "remodel a casino you are the boss of to a different color",
                    Token::new("remodel"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "loc",
                    "a location of the casino to remodel",
                    loc_parser(TILES.keys().cloned().collect()),
                )),
                AfterSpace::new(Doc::name_desc(
                    "casino",
                    "the color to remodel to",
                    casino_parser(),
                )),
            ),
            |(_, loc, casino)| Command::Remodel { loc, casino },
        )
    }

    pub fn reorg_parser(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "reorg",
                    "reroll all the dice in a casino that you have a dice in",
                    Token::new("reorg"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "loc",
                    "a location of the casino to reorg",
                    loc_parser(TILES.keys().cloned().collect()),
                )),
            ),
            |(_, loc)| Command::Reorg { loc },
        )
    }

    pub fn gamble_parser(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain3::new(
                Doc::name_desc(
                    "gamble",
                    "gamble at an opponent's casino",
                    Token::new("gamble"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "player",
                    "the player whose casino you want to gamble at",
                    Player {},
                )),
                AfterSpace::new(Doc::name_desc(
                    "amount",
                    "the amount to gamble, maximum $5 per tile",
                    money_parser(),
                )),
            ),
            |(_, player, amount)| Command::Gamble { player, amount },
        )
    }

    pub fn raise_parser(&self) -> impl Parser<T = Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc(
                    "raise",
                    "raise a casino you are the boss of by a level",
                    Token::new("raise"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "loc",
                    "a location of the casino to raise",
                    loc_parser(TILES.keys().cloned().collect()),
                )),
            ),
            |(_, loc)| Command::Raise { loc },
        )
    }
}

fn loc_parser(mut locs: Vec<Loc>) -> impl Parser<T = Loc> {
    locs.sort();
    Enum::exact(locs)
}

fn casino_parser() -> impl Parser<T = Casino> {
    Enum::partial(CASINOS.to_owned())
}

fn money_parser() -> impl Parser<T = usize> {
    Map::new(Int::positive(), |i| i as usize)
}

fn done_parser() -> impl Parser<T = Command> {
    Map::new(Token::new("done"), |_| Command::Done)
}
