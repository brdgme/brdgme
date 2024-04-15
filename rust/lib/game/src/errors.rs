use thiserror::Error;

use crate::command::parser::comma_list_or;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid player count, expected {}, got {given}", player_range_output(.min, .max))]
    PlayerCount {
        min: usize,
        max: usize,
        given: usize,
    },
    #[error("invalid input, {message}")]
    InvalidInput { message: String },
    #[error("not your turn")]
    NotYourTurn,
    #[error("game is already finished")]
    Finished,
    #[error("internal error: {message}")]
    Internal { message: String },
    #[error("{}expected {}", parse_error_message(.message), comma_list_or(.expected))]
    Parse {
        message: Option<String>,
        expected: Vec<String>,
        offset: usize,
    },
}

impl GameError {
    pub fn invalid_input<I: Into<String>>(message: I) -> GameError {
        GameError::InvalidInput {
            message: message.into(),
        }
    }

    pub fn internal<I: Into<String>>(message: I) -> GameError {
        GameError::Internal {
            message: message.into(),
        }
    }
}

fn parse_error_message(message: &Option<String>) -> String {
    message
        .as_ref()
        .map(|m| format!("{}, ", m))
        .unwrap_or_default()
}

fn player_range_output(min: &usize, max: &usize) -> String {
    if min == max {
        format!("{}", min)
    } else {
        format!("{} to {}", min, max)
    }
}
