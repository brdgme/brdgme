use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid player count, expected {min}-{max}, got {given}")]
    PlayerCount {
        min: usize,
        max: usize,
        given: usize,
    },
    #[error("{message}")]
    InvalidInput { message: String },
    #[error("not your turn")]
    NotYourTurn,
    #[error("game is already finished")]
    Finished,
    #[error("internal error: {message}")]
    Internal { message: String },
    #[error("{message:?}expected {expected:?}")]
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
