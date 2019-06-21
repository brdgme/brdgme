use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::board::{Board, BoardTile, Loc, TileOwner};
use crate::card::{render_cards, shuffled_deck, Card};
use crate::casino::Casino;
use crate::command::Command;
use crate::render::render_cash;
use crate::tile::TILES;

pub mod board;
pub mod card;
pub mod casino;
mod command;
pub mod render;
pub mod tile;

pub const STARTING_CARDS: usize = 2;
pub const PLAYER_DICE: usize = 12;
pub const PLAYER_OWNER_TOKENS: usize = 10;
pub const CASINO_CARDS: usize = 9;
pub const CASINO_TILES: usize = 9;
pub const CASINO_DEFAULT_HEIGHT: usize = 1;

pub const DIE_MIN: usize = 1;
pub const DIE_MAX: usize = 6;

pub static POINT_STOPS: &'static [usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 23, 26, 29, 32, 36, 40, 44, 49, 54, 60, 66,
    73, 81, 90,
];

#[derive(Serialize, Deserialize)]
pub struct PubState {
    pub players: Vec<Player>,
    pub current_player: usize,
    pub remaining_deck: usize,
    pub played: Vec<Card>,
    pub board: Board,
    pub finished: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerState {
    pub player: usize,
    pub state: Option<Player>,
    pub pub_state: PubState,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Player {
    pub cash: usize,
    pub points: usize,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Game {
    pub players: Vec<Player>,
    pub current_player: usize,
    pub deck: Vec<Card>,
    pub played: Vec<Card>,
    pub board: Board,
    pub finished: bool,
}

pub fn roll() -> usize {
    rand::thread_rng().gen::<usize>() % (DIE_MAX - DIE_MIN) + DIE_MIN
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize) -> Result<(Self, Vec<Log>), GameError> {
        if players < 2 || players > 6 {
            return Err(GameError::PlayerCount {
                min: 2,
                max: 6,
                given: players,
            });
        }
        let mut logs: Vec<Log> = vec![];
        let mut board = Board::default();
        let mut deck = shuffled_deck(players);
        let mut played: Vec<Card> = vec![];
        let current_player = rand::thread_rng().gen::<usize>() % players;
        let players: Vec<Player> = (0..players)
            .map(|p| {
                let cards: Vec<Card> = deck.drain(..STARTING_CARDS).collect();
                let cash = cards.iter().fold(0, |acc, c| match *c {
                    Card::Loc { loc } => {
                        board.set(loc, BoardTile::Owned { player: p });
                        acc + TILES[&loc].starting_cash
                    }
                    Card::GameEnd => unreachable!(),
                });
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(" drew "),
                    N::Group(render_cards(&cards)),
                    N::text(" and will start with "),
                    render_cash(cash),
                ]));
                let player = Player {
                    cash,
                    ..Player::default()
                };
                played.extend(cards);
                player
            })
            .collect();
        logs.push(Log::public(vec![
            N::Player(current_player),
            N::text(" will start the game"),
        ]));
        Ok((
            Game {
                players,
                current_player,
                board,
                deck,
                played,
                finished: false,
            },
            logs,
        ))
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players.clone(),
            current_player: self.current_player,
            remaining_deck: self.deck.len(),
            played: self.played.clone(),
            board: self.board.clone(),
            finished: self.finished,
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            player,
            state: self.players.get(player).cloned(),
            pub_state: self.pub_state(),
        }
    }

    #[allow(unused_variables)]
    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = self.command_parser(player).parse(input, players)?;
        let (logs, can_undo) = match output.value {
            Command::Build { loc, casino } => self.build(player, &loc, casino)?,
            Command::Remodel { loc, casino } => unimplemented!(),
            Command::Reorg { loc } => unimplemented!(),
            Command::Sprawl { from, to } => unimplemented!(),
            Command::Gamble { player, amount } => unimplemented!(),
            Command::Raise { loc } => unimplemented!(),
            Command::Done => self.done(player)?,
        };
        Ok(CommandResponse {
            logs,
            can_undo,
            remaining_input: output.remaining.to_string(),
        })
    }

    fn status(&self) -> Status {
        if self.finished {
            Status::Finished {
                placings: gen_placings(
                    &self
                        .players
                        .iter()
                        .map(|p| vec![p.points as i32, p.cash as i32])
                        .collect::<Vec<Vec<i32>>>(),
                ),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.whose_turn().into_iter().find(|&p| p == player)?;
        Some(self.command_parser(player).to_spec())
    }

    fn player_count(&self) -> usize {
        self.players.len()
    }

    fn player_counts() -> Vec<usize> {
        (2..7).collect()
    }
}

impl Game {
    fn can_build(&self, player: usize) -> bool {
        player == self.current_player
    }

    fn build(
        &mut self,
        p: usize,
        loc: &Loc,
        casino: Casino,
    ) -> Result<(Vec<Log>, bool), GameError> {
        if !self.can_build(p) {
            return Err(GameError::InvalidInput {
                message: "can't build at the moment".to_string(),
            });
        }

        if !TILES.contains_key(loc) {
            return Err(GameError::InvalidInput {
                message: "not a valid location".to_string(),
            });
        }
        match self.board.get(loc) {
            BoardTile::Owned { player } if player == p => {}
            BoardTile::Built { .. } => {
                return Err(GameError::InvalidInput {
                    message: "that location has already been built".to_string(),
                });
            }
            _ => {
                return Err(GameError::InvalidInput {
                    message: "you don't own that location".to_string(),
                });
            }
        }
        if self.players[p].cash < TILES[loc].build_cost {
            return Err(GameError::InvalidInput {
                message: "you don't have enough cash".to_string(),
            });
        }
        self.players[p].cash -= TILES[loc].build_cost;
        self.board.set(
            *loc,
            BoardTile::Built {
                casino,
                owner: Some(TileOwner {
                    die: TILES[loc].die,
                    player: p,
                }),
                height: CASINO_DEFAULT_HEIGHT,
            },
        );
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::Player(p),
            N::text(" built "),
            casino.render(),
            N::text(" at "),
            loc.render(),
        ])];
        let mut can_undo = true;

        // Building can trigger boss ties.
        if let Some(resolve_logs) = self.board.resolve_boss_ties() {
            logs.extend(resolve_logs);
            can_undo = false;
        }

        Ok((logs, can_undo))
    }

    fn can_done(&self, player: usize) -> bool {
        player == self.current_player
    }

    fn done(&mut self, player: usize) -> Result<(Vec<Log>, bool), GameError> {
        if !self.can_done(player) {
            return Err(GameError::InvalidInput {
                message: "can't end turn at the moment".to_string(),
            });
        }

        Ok(self.next_player())
    }

    fn next_player(&mut self) -> (Vec<Log>, bool) {
        self.current_player = (self.current_player + 1) % self.players.len();
        (vec![], false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_counts_works() {
        assert_eq!(Game::player_counts(), vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn json_works() {
        use serde_json;
        let game = Game::start(3)
            .expect("could not create game with 3 players")
            .0;
        serde_json::to_string(&game).expect("could not serialise game to JSON");
    }
}
