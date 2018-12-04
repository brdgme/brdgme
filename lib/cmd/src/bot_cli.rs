use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_derive::{Serialize, Deserialize};
use serde_json;

use brdgme_game::Gamer;
use brdgme_game::bot::Botter;
use brdgme_game::command::Spec as CommandSpec;

use std::fmt::Debug;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub player: usize,
    pub player_state: String,
    pub players: Vec<String>,
    pub command_spec: CommandSpec,
    pub game_id: Option<String>,
}

pub type Response = Vec<String>;

pub fn cli<G, B, I, O>(bot: &mut B, input: I, output: &mut O)
where
    G: Gamer + Debug + Clone + Serialize + DeserializeOwned,
    B: Botter<G>,
    I: Read,
    O: Write,
{
    let request = serde_json::from_reader::<_, Request>(input).unwrap();
    let player_state: G::PlayerState = serde_json::from_str(&request.player_state).unwrap();
    writeln!(
        output,
        "{}",
        serde_json::to_string(&bot.commands(
            request.player,
            &player_state,
            &request.players,
            &request.command_spec,
            request.game_id,
        ),).unwrap()
    ).unwrap();
}
