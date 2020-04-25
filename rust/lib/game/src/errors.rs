use std::fmt;

use failure::Fail;

use crate::command::parser::comma_list_or;

#[derive(Debug, Fail)]
pub enum GameError {
    PlayerCount {
        min: usize,
        max: usize,
        given: usize,
    },
    InvalidInput {
        message: String,
    },
    NotYourTurn,
    Finished,
    Internal {
        message: String,
    },
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

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GameError::PlayerCount { given, min, max } => write!(
                f,
                "not for {} players, expected {}",
                given,
                player_range_output(min, max)
            ),
            GameError::InvalidInput { ref message } => write!(f, "{}", message),
            GameError::NotYourTurn => write!(f, "not your turn"),
            GameError::Finished => write!(f, "game is already finished"),
            GameError::Internal { ref message } => write!(f, "internal error: {}", message),
            GameError::Parse {
                ref message,
                ref expected,
                ..
            } => write!(
                f,
                "{}expected {}",
                message
                    .as_ref()
                    .map(|m| format!("{}, ", m))
                    .unwrap_or_else(|| "".to_string()),
                comma_list_or(expected)
            ),
        }
    }
}

fn player_range_output(min: usize, max: usize) -> String {
    if min == max {
        format!("{}", min)
    } else {
        format!("{} to {}", min, max)
    }
}
