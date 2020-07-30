use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("failed to parse request")]
    Parse {
        #[from]
        source: serde_json::Error,
    },
    #[error("IO error")]
    IO {
        #[from]
        source: io::Error,
    },
    #[error("Failed to get stdin")]
    Stdin,
}

#[derive(Debug, Error)]
pub enum ParseArgsError {
    #[error("expected type argument, one of 'local'")]
    TypeMissing,
    #[error("path argument missing")]
    PathMissing,
}
