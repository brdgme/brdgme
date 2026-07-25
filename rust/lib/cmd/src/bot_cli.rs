use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub player: usize,
    pub player_state: String,
    pub players: Vec<String>,
    pub command_spec: CommandSpec,
    pub game_id: Option<String>,
}
