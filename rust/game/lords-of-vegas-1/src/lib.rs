use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::board::{Board, BoardTile, Loc, TileOwner};
use crate::card::{Card, render_cards, shuffled_deck};
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

pub static POINT_STOPS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 23, 26, 29, 32, 36, 40, 44, 49, 54, 60, 66,
    73, 81, 90,
];

#[derive(Serialize, Deserialize)]
pub struct PubState {
    /// Per-player public info (cash and points), indexed by player number.
    pub players: Vec<Player>,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// Number of cards left in the draw deck.
    pub remaining_deck: usize,
    /// Location cards that have been dealt so far. Each is a Loc card; the hidden GameEnd card is never shown here.
    pub played: Vec<Card>,
    /// The state of every lot on the strip, keyed by location.
    pub board: Board,
    /// True when the game is over.
    pub finished: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerState {
    /// Which player this private state belongs to.
    pub player: usize,
    /// This player's own cash and points.
    pub state: Option<Player>,
    /// The full public game state.
    pub pub_state: PubState,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Player {
    /// Cash currently on hand, used to pay build costs.
    pub cash: usize,
    /// Points index into POINT_STOPS. Currently always 0 as scoring is not yet implemented.
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
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

pub fn roll(rng: &mut GameRng) -> usize {
    rng.random_range(DIE_MIN..=DIE_MAX)
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(2..=6).contains(&players) {
            return Err(GameError::PlayerCount {
                min: 2,
                max: 6,
                given: players,
            });
        }
        let mut rng = GameRng::seed_from_u64(seed);
        let mut logs: Vec<Log> = vec![];
        let mut board = Board::default();
        let mut deck = shuffled_deck(players, &mut rng);
        let mut played: Vec<Card> = vec![];
        let current_player = rng.random_range(0..players);
        let players: Vec<Player> = (0..players)
            .map(|p| {
                let cards: Vec<Card> = deck.drain(..STARTING_CARDS).collect();
                let cash = cards.iter().fold(0, |acc, c| match *c {
                    Card::Loc { loc } => {
                        board.set(loc, BoardTile::Owned { player: p });
                        acc + TILES[&loc].starting_cash
                    }
                    // shuffled_deck inserts GameEnd in the last quarter of the
                    // deck (position >= 38 even for 2 players; see card.rs),
                    // while starting hands drain at most 12 cards from the
                    // front, so GameEnd can never be dealt here.
                    Card::GameEnd => unreachable!("GameEnd cannot be in a starting hand"),
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
                rng,
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

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = self.command_parser(player).parse(input, players)?;
        let (logs, can_undo) = self.dispatch(player, output.value)?;
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

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }

    fn data_docs() -> String {
        include_str!("../DATA_DOCS.md").to_string()
    }

    fn basic_strategy() -> String {
        include_str!("../BASIC_STRATEGY.md").to_string()
    }

    fn advanced_strategy() -> String {
        include_str!("../ADVANCED_STRATEGY.md").to_string()
    }
}

impl Game {
    fn can_build(&self, player: usize) -> bool {
        player == self.current_player
    }

    fn dispatch(&mut self, player: usize, cmd: Command) -> Result<(Vec<Log>, bool), GameError> {
        match cmd {
            Command::Build { loc, casino } => self.build(player, &loc, casino),
            Command::Remodel { .. } => Err(GameError::InvalidInput {
                message: "remodel is not implemented yet".to_string(),
            }),
            Command::Reorg { .. } => Err(GameError::InvalidInput {
                message: "reorg is not implemented yet".to_string(),
            }),
            Command::Sprawl { .. } => Err(GameError::InvalidInput {
                message: "sprawl is not implemented yet".to_string(),
            }),
            Command::Gamble { .. } => Err(GameError::InvalidInput {
                message: "gamble is not implemented yet".to_string(),
            }),
            Command::Raise { .. } => Err(GameError::InvalidInput {
                message: "raise is not implemented yet".to_string(),
            }),
            Command::Done => self.done(player),
        }
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
        if self.board.casino_tile_count(casino) >= CASINO_TILES {
            return Err(GameError::InvalidInput {
                message: format!("there are no {} tiles remaining", casino),
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
        if let Some(resolve_logs) = self.board.resolve_boss_ties(&mut self.rng) {
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
        let game = Game::start(3, 1)
            .expect("could not create game with 3 players")
            .0;
        serde_json::to_string(&game).expect("could not serialise game to JSON");
    }

    #[test]
    fn unimplemented_commands_error_instead_of_panicking() {
        // d F1: the five unwired command arms must return a GameError, not
        // panic the process, so that wiring their parsers in later can never
        // turn a valid player command into a crash.
        use crate::board::Block;
        use crate::command::Command;

        let mut g = Game::start(2, 1).expect("could not start game").0;
        let p = g.current_player;
        let loc: Loc = (Block::A, 1).into();
        let cmds = vec![
            Command::Remodel {
                loc,
                casino: Casino::Albion,
            },
            Command::Reorg { loc },
            Command::Sprawl {
                from: loc,
                to: (Block::A, 2).into(),
            },
            Command::Gamble {
                player: (p + 1) % 2,
                amount: 5,
            },
            Command::Raise { loc },
        ];
        for cmd in cmds {
            match g.dispatch(p, cmd) {
                Err(GameError::InvalidInput { message }) => assert!(
                    message.contains("not implemented"),
                    "unexpected message: {}",
                    message
                ),
                Ok(_) => panic!("expected InvalidInput error, got Ok"),
                Err(e) => panic!("expected InvalidInput error, got: {}", e),
            }
        }
    }

    #[test]
    fn build_rejects_when_casino_tile_supply_exhausted() {
        // d F4: there are only 9 tiles per casino colour; the 10th build of a
        // colour must be rejected instead of corrupting the board.
        use crate::board::Block;

        let mut board = Board::default();
        for loc in [
            (Block::A, 1),
            (Block::A, 2),
            (Block::A, 3),
            (Block::A, 4),
            (Block::A, 5),
            (Block::A, 6),
            (Block::B, 1),
            (Block::B, 2),
            (Block::B, 3),
        ] {
            board.set(
                loc.into(),
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: None,
                    height: 1,
                },
            );
        }
        board.set((Block::F, 5).into(), BoardTile::Owned { player: 0 });
        let mut g = Game {
            players: vec![
                Player {
                    cash: 100,
                    points: 0,
                },
                Player {
                    cash: 100,
                    points: 0,
                },
            ],
            current_player: 0,
            deck: vec![],
            played: vec![],
            board,
            finished: false,
            rng: GameRng::seed_from_u64(1),
        };
        match g.build(0, &(Block::F, 5).into(), Casino::Albion) {
            Err(GameError::InvalidInput { message }) => assert!(
                message.contains("no Albion tiles remaining"),
                "unexpected message: {}",
                message
            ),
            Ok(_) => panic!("10th Albion build must fail: the supply is 9 tiles"),
            Err(e) => panic!("expected InvalidInput, got: {}", e),
        }
        // A different colour still has tiles and must build fine.
        assert!(
            g.build(0, &(Block::F, 5).into(), Casino::Vega).is_ok(),
            "other colours must be unaffected by an exhausted Albion supply"
        );
    }

    #[test]
    fn render_saturates_when_supplies_exceeded() {
        // d F4: legacy saved states can already exceed the supplies; the
        // renderer must saturate at zero instead of underflowing usize.
        // 13 built Albion tiles owned by player 0 exceed both the 9-tile
        // casino supply and the 12-die player supply.
        use crate::board::Block;

        let mut board = Board::default();
        let mut locs: Vec<Loc> = vec![(Block::C, 1).into()];
        for lot in 1..=6 {
            locs.push((Block::A, lot).into());
            locs.push((Block::B, lot).into());
        }
        for loc in &locs {
            board.set(
                *loc,
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: Some(TileOwner { die: 1, player: 0 }),
                    height: 1,
                },
            );
        }
        let ps = PubState {
            players: vec![Player::default()],
            current_player: 0,
            remaining_deck: 0,
            played: vec![],
            board,
            finished: false,
        };
        // Pre-fix both of these panic in debug builds with
        // "attempt to subtract with overflow".
        let player_table = ps.render_player_table(0);
        let casino_table = ps.render_casino_table();
        assert!(matches!(player_table, N::Table(_)));
        assert!(matches!(casino_table, N::Table(_)));
    }
}
