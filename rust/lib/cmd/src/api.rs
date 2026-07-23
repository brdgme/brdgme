use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::PrimitiveDateTime;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::{Gamer, Log, Status};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    PlayerCounts,
    New {
        players: usize,
        #[serde(default)]
        seed: Option<u64>,
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
    Rules,
    DataDocs {
        game: String,
    },
    BasicStrategy {
        game: String,
        player: usize,
    },
    AdvancedStrategy {
        game: String,
        player: usize,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CliLog {
    pub content: String,
    #[serde(deserialize_with = "go_compat_datetime::deserialize")]
    pub at: PrimitiveDateTime,
    pub public: bool,
    pub to: Vec<usize>,
}

/// Rust V2 game services serialize log timestamps with `time`'s default serde
/// (a structured sequence), but Go V1 services emit an RFC 3339-ish string with
/// a `T` separator (e.g. `2026-07-23T07:53:50.928274053`, see
/// `brdgme-go/cmd/cli.go`) that the default deserializer rejects. Accept the
/// native form unchanged and parse the Go string with the well-known ISO 8601
/// format. Serialization is untouched, so deployed Rust services stay compatible.
mod go_compat_datetime {
    use serde::{Deserialize, Deserializer};
    use time::PrimitiveDateTime;
    use time::format_description::well_known::Iso8601;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Native(PrimitiveDateTime),
        Text(String),
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PrimitiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Repr::deserialize(deserializer)? {
            Repr::Native(dt) => Ok(dt),
            Repr::Text(raw) => {
                PrimitiveDateTime::parse(&raw, &Iso8601::DEFAULT).map_err(serde::de::Error::custom)
            }
        }
    }
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
        seed: u64,
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
    Rules {
        rules: String,
    },
    DataDocs {
        data_docs: String,
    },
    BasicStrategy {
        strategy: String,
    },
    AdvancedStrategy {
        strategy: String,
    },
    UserError {
        message: String,
    },
    SystemError {
        message: String,
    },
}

#[derive(Error, Debug)]
pub enum GameResponseError {
    #[error("failed to encode game state")]
    Encode {
        #[from]
        source: serde_json::Error,
    },
}

impl GameResponse {
    pub fn from_gamer<T: Gamer + Serialize>(gamer: &T) -> Result<GameResponse, GameResponseError> {
        Ok(GameResponse {
            state: serde_json::to_string(gamer)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_play_response_deserializes() {
        let body = include_str!("../testdata_go_play_resp.json");
        let resp: Response = serde_json::from_str(body).expect("Go Play response must deserialize");
        assert!(matches!(resp, Response::Play { .. }));
    }

    #[test]
    fn go_log_at_format_deserializes() {
        let json = r#"{"content":"x","at":"2026-07-23T07:53:50.928274053","public":true,"to":[]}"#;
        let log: CliLog = serde_json::from_str(json).expect("Go log `at` format must deserialize");
        assert_eq!(log.content, "x");
    }

    #[test]
    fn log_at_round_trips_through_default_serde() {
        let log = CliLog {
            content: "x".to_string(),
            at: time::macros::datetime!(2026-07-23 07:53:50.928274053),
            public: true,
            to: vec![],
        };
        let json = serde_json::to_string(&log).expect("serialize");
        let back: CliLog = serde_json::from_str(&json).expect("our own output must deserialize");
        assert_eq!(back.at, log.at);
    }
}
