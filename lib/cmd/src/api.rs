use chrono::NaiveDateTime;
use failure::{Error, format_err};
use serde::Serialize;
use serde_derive::{Serialize, Deserialize};
use serde_json;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::{Gamer, Log, Status};
use brdgme_markup;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    PlayerCounts,
    New {
        players: usize,
    },
    Status {
        game: String,
    },
    Play {
        player: usize,
        command: String,
        names: Vec<String>,
        game: String,
    },
    PubRender {
        game: String,
    },
    PlayerRender {
        player: usize,
        game: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CliLog {
    pub content: String,
    pub at: NaiveDateTime,
    pub public: bool,
    pub to: Vec<usize>,
}

impl CliLog {
    fn from_log(log: &Log) -> CliLog {
        CliLog {
            content: brdgme_markup::to_string(&log.content),
            at: log.at,
            public: log.public,
            to: log.to.clone(),
        }
    }

    pub fn from_logs(logs: &[Log]) -> Vec<CliLog> {
        logs.iter().map(CliLog::from_log).collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameResponse {
    pub state: String,
    pub points: Vec<f32>,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PubRender {
    pub pub_state: String,
    pub render: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerRender {
    pub player_state: String,
    pub render: String,
    pub command_spec: Option<CommandSpec>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    PlayerCounts {
        player_counts: Vec<usize>,
    },
    New {
        game: GameResponse,
        logs: Vec<CliLog>,
        public_render: PubRender,
        player_renders: Vec<PlayerRender>,
    },
    Status {
        game: GameResponse,
        public_render: PubRender,
        player_renders: Vec<PlayerRender>,
    },
    Play {
        game: GameResponse,
        logs: Vec<CliLog>,
        can_undo: bool,
        remaining_input: String,
        public_render: PubRender,
        player_renders: Vec<PlayerRender>,
    },
    PubRender {
        render: PubRender,
    },
    PlayerRender {
        render: PlayerRender,
    },
    UserError {
        message: String,
    },
    SystemError {
        message: String,
    },
}

impl GameResponse {
    pub fn from_gamer<T: Gamer + Serialize>(gamer: &T) -> Result<GameResponse, Error> {
        Ok(GameResponse {
            state: serde_json::to_string(gamer)
                .map_err(|e| format_err!("unable to encode game state: {}", e))?,
            points: gamer.points(),
            status: gamer.status(),
        })
    }
}

impl From<GameError> for Response {
    fn from(e: GameError) -> Self {
        match e {
            GameError::Internal { message } => Response::SystemError { message },
            e => Response::UserError {
                message: e.to_string(),
            },
        }
    }
}
