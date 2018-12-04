use brdgme_game::command::parser::*;
use brdgme_game::Gamer;

use crate::board::Loc;
use crate::corp::{Corp, CORPS};
use crate::Game;
use crate::Phase;

use std::usize;

pub enum Command {
    Play(Loc),
    Found(Corp),
    Buy(usize, Corp),
    Done,
    Merge(Corp, Corp),
    Sell(usize),
    Trade(usize),
    Keep,
    End,
}

impl Game {
    pub fn command_parser(&self, player: usize) -> Option<Box<Parser<Command>>> {
        if self.is_finished() {
            return None;
        }
        let mut parsers: Vec<Box<Parser<Command>>> = vec![];
        if self.phase.whose_turn() == player {
            match self.phase {
                Phase::Play(_) => {
                    parsers.push(Box::new(self.play_parser(player)));
                }
                Phase::Found { .. } => {
                    parsers.push(Box::new(self.found_parser(
                        self.board.available_corps().into_iter().collect(),
                    )));
                }
                Phase::Buy { remaining, .. } => {
                    if remaining > 0 {
                        parsers.push(Box::new(self.buy_parser(player, remaining)));
                    }
                    parsers.push(Box::new(done_parser()));
                }
                Phase::ChooseMerger { at, .. } => {
                    parsers.push(Box::new(
                        self.merge_parser(&self.board
                            .neighbouring_corps(&at)
                            .into_iter()
                            .collect::<Vec<Corp>>()),
                    ));
                }
                Phase::SellOrTrade { player, corp, .. } => {
                    parsers.push(Box::new(self.sell_parser(player, corp)));
                    if self.players[player]
                        .shares
                        .get(&corp)
                        .expect("could not get player shares") >= &2
                    {
                        parsers.push(Box::new(self.trade_parser(player, corp)));
                    }
                    parsers.push(Box::new(keep_parser()));
                }
            }
            if self.player_can_end(player) {
                parsers.push(Box::new(end_parser()));
            }
        }
        if parsers.is_empty() {
            None
        } else {
            Some(Box::new(OneOf::new(parsers)))
        }
    }

    fn play_parser(&self, player: usize) -> impl Parser<Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("play", "play a tile to the board", Token::new("play")),
                AfterSpace::new(Doc::name(
                    "tile",
                    Enum::exact(
                        self.players
                            .get(player)
                            .map(|p| p.tiles.clone())
                            .unwrap_or_else(|| vec![]),
                    ),
                )),
            ),
            |(_, loc)| Command::Play(loc),
        )
    }

    fn found_parser(&self, corps: Vec<Corp>) -> impl Parser<Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("found", "found a new corporation", Token::new("found")),
                AfterSpace::new(Doc::name_desc(
                    "corp",
                    "the corporation to found",
                    Enum::partial(corps),
                )),
            ),
            |(_, corp)| Command::Found(corp),
        )
    }

    fn buy_parser(&self, _player: usize, remaining: usize) -> impl Parser<Command> {
        Map::new(
            Chain3::new(
                Doc::name_desc("buy", "buy shares", Token::new("buy")),
                AfterSpace::new(Doc::name_desc(
                    "#",
                    "number of shares to buy",
                    Int::bounded(1, remaining as i32),
                )),
                AfterSpace::new(Doc::name_desc(
                    "corp",
                    "the corporation to buy shares in",
                    Enum::partial(CORPS.to_vec()),
                )),
            ),
            |(_, n, corp)| Command::Buy(n as usize, corp),
        )
    }

    fn sell_parser(&self, player: usize, corp: Corp) -> impl Parser<Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("sell", "sell shares", Token::new("sell")),
                AfterSpace::new(Doc::name_desc(
                    "#",
                    "number of shares to sell",
                    self.player_shares_parser(player, corp),
                )),
            ),
            |(_, n)| Command::Sell(n as usize),
        )
    }

    fn trade_parser(&self, player: usize, corp: Corp) -> impl Parser<Command> {
        Map::new(
            Chain2::new(
                Doc::name_desc("trade", "trade shares, two-for-one", Token::new("trade")),
                AfterSpace::new(Doc::name_desc(
                    "#",
                    "number of shares to trade, two-for-one",
                    self.player_shares_parser(player, corp),
                )),
            ),
            |(_, n)| Command::Trade(n as usize),
        )
    }

    fn player_shares_parser(&self, player: usize, corp: Corp) -> impl Parser<i32> {
        Int::bounded(
            1,
            self.players
                .get(player)
                .and_then(|p| p.shares.get(&corp).cloned())
                .expect("could not et player shares") as i32,
        )
    }

    fn merge_parser(&self, corps: &[Corp]) -> impl Parser<Command> {
        Map::new(
            Chain4::new(
                Doc::name_desc(
                    "merge",
                    "choose which corporation to merge into another",
                    Token::new("merge"),
                ),
                AfterSpace::new(Doc::name_desc(
                    "corp",
                    "the corporation to merge into another",
                    Enum::partial(corps.to_owned()),
                )),
                AfterSpace::new(Token::new("into")),
                AfterSpace::new(Doc::name_desc(
                    "corp",
                    "the corporation to be merged into",
                    Enum::partial(corps.to_owned()),
                )),
            ),
            |(_, from, _, into)| Command::Merge(from, into),
        )
    }
}

fn end_parser() -> impl Parser<Command> {
    Doc::name_desc(
        "end",
        "trigger the end of the game at the end of your turn",
        Map::new(Token::new("end"), |_| Command::End),
    )
}

fn done_parser() -> impl Parser<Command> {
    Doc::name_desc(
        "done",
        "finish buying shares and end your turn",
        Map::new(Token::new("done"), |_| Command::Done),
    )
}

fn keep_parser() -> impl Parser<Command> {
    Doc::name_desc(
        "keep",
        "finish selling and trading shares",
        Map::new(Token::new("keep"), |_| Command::Keep),
    )
}
