use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::{CommandResponse, Gamer, Log, Renderer, Status};
use brdgme_markup::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGame {
    pub players: usize,
    pub plays: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TestState;

impl Renderer for TestState {
    fn render(&self) -> Vec<Node> {
        vec![Node::text("test")]
    }
}

impl Gamer for TestGame {
    type PubState = TestState;
    type PlayerState = TestState;

    fn start(players: usize, _seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !Self::player_counts().contains(&players) {
            return Err(GameError::PlayerCount {
                min: 1,
                max: 4,
                given: players,
            });
        }
        Ok((TestGame { players, plays: 0 }, vec![]))
    }

    fn pub_state(&self) -> TestState {
        TestState
    }

    fn player_state(&self, _player: usize) -> TestState {
        TestState
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        _players: &[String],
    ) -> Result<CommandResponse, GameError> {
        if player != 0 {
            return Err(GameError::NotYourTurn);
        }
        match input.trim().strip_prefix("play") {
            Some(rest) => {
                self.plays += 1;
                Ok(CommandResponse {
                    logs: vec![],
                    can_undo: false,
                    remaining_input: rest.to_string(),
                })
            }
            None => Err(GameError::invalid_input("expected 'play'")),
        }
    }

    fn status(&self) -> Status {
        Status::Active {
            whose_turn: vec![0],
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
        vec![1, 2, 3, 4]
    }

    fn validate(&self) -> Result<(), GameError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BrokenRenderGame {
    pub players: usize,
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BrokenState {
    pub map: HashMap<(u8, u8), u8>,
}

impl Renderer for BrokenState {
    fn render(&self) -> Vec<Node> {
        vec![Node::text("broken")]
    }
}

impl Gamer for BrokenRenderGame {
    type PubState = BrokenState;
    type PlayerState = BrokenState;

    fn start(players: usize, _seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        Ok((BrokenRenderGame { players }, vec![]))
    }

    fn pub_state(&self) -> BrokenState {
        BrokenState {
            map: HashMap::from([((0, 0), 0)]),
        }
    }

    fn player_state(&self, _player: usize) -> BrokenState {
        self.pub_state()
    }

    fn command(
        &mut self,
        _player: usize,
        _input: &str,
        _players: &[String],
    ) -> Result<CommandResponse, GameError> {
        Err(GameError::invalid_input("no commands"))
    }

    fn status(&self) -> Status {
        Status::Active {
            whose_turn: vec![0],
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
        vec![2]
    }

    fn validate(&self) -> Result<(), GameError> {
        Ok(())
    }
}
