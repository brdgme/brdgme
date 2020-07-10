use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("parse error, {message}")]
    Parse { message: String },
}
