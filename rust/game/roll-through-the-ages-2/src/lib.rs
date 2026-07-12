//! Port of `brdgme-go/roll_through_the_ages_1`.
//!
//! This crate is being built up task-by-task per
//! `docs/superpowers/plans/2026-07-12-roll-through-the-ages-2-port.md`.
//! Task 1 provides only the domain types (`dice`, `good`, `development`,
//! `monument`, `player_board`) and a bare `Game` skeleton; the full
//! `Gamer` impl (phase engine, commands, render) lands in later tasks.

pub mod development;
pub mod dice;
pub mod good;
pub mod monument;
pub mod player_board;

use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::game::Renderer;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use player_board::PlayerBoard;

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

/// Phase enum, ported from `game.go`'s `Phase` iota. Full phase-engine
/// wiring lands in Task 2.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Preserve,
    Roll,
    ExtraRoll,
    Collect,
    Resolve,
    Invade,
    Build,
    Trade,
    Buy,
    Discard,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub phase: Phase,
    pub boards: Vec<PlayerBoard>,

    pub rolled_dice: Vec<dice::Die>,
    pub kept_dice: Vec<dice::Die>,
    pub remaining_rolls: i32,
    pub remaining_workers: i32,
    pub remaining_ships: i32,
    pub remaining_coins: i32,

    pub final_round: bool,

    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            players: 0,
            current_player: 0,
            phase: Phase::Preserve,
            boards: vec![],
            rolled_dice: vec![],
            kept_dice: vec![],
            remaining_rolls: 0,
            remaining_workers: 0,
            remaining_ships: 0,
            remaining_coins: 0,
            final_round: false,
            rng: GameRng::default(),
        }
    }
}

impl Game {
    /// The players whose turn it currently is. Full multi-phase engine
    /// lands in Task 2; for now this always names the single current
    /// player, matching this game's strict single-current-player turn
    /// structure.
    pub fn players(&self) -> Vec<String> {
        (0..self.players).map(|i| format!("player{}", i)).collect()
    }
}

/// No hidden information in this game (`PlayerState`/`PubState` both return
/// `nil` in Go); render still needs a concrete type to implement `Renderer`
/// against, wired up fully in Task 4.
#[derive(Default, Serialize, Deserialize)]
pub struct PubState;

impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        vec![]
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState;

impl Renderer for PlayerState {
    fn render(&self) -> Vec<N> {
        vec![]
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    /// Placeholder implementation so the crate's bin stubs compile in Task
    /// 1; the real phase-cascading `start()` lands in Task 2.
    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let g = Game {
            players,
            boards: (0..players).map(|_| PlayerBoard::default()).collect(),
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        Ok((g, vec![]))
    }

    fn pub_state(&self) -> Self::PubState {
        PubState
    }

    fn player_state(&self, _player: usize) -> Self::PlayerState {
        PlayerState
    }

    /// Placeholder: no commands are wired up until Task 2/3.
    fn command(
        &mut self,
        _player: usize,
        _input: &str,
        _players: &[String],
    ) -> Result<CommandResponse, GameError> {
        Err(GameError::invalid_input("not yet implemented"))
    }

    fn status(&self) -> Status {
        Status::Active {
            whose_turn: vec![self.current_player],
            eliminated: vec![],
        }
    }

    fn command_spec(&self, _player: usize) -> Option<CommandSpec> {
        None
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4]
    }

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn start_rejects_invalid_player_counts() {
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(5, 1).is_err());
    }

    #[test]
    fn start_ok_for_valid_player_counts() {
        for n in 2..=4 {
            assert!(Game::start(n, 1).is_ok());
        }
    }
}
