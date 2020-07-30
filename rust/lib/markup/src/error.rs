use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarkupError {
    #[error("failed to parse input")]
    Parse,
}
