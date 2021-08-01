use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use log::error;
use std::io::Cursor;

#[derive(thiserror::Error, Debug)]
pub enum ControllerError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },
    #[error("Database error")]
    Database(#[from] diesel::result::Error),
    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

impl ControllerError {
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        ControllerError::BadRequest {
            message: message.into(),
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ControllerError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        match self {
            ControllerError::BadRequest { ref message } => Ok(Response::build()
                .status(Status::BadRequest)
                .header(ContentType::Plain)
                .raw_header("Access-Control-Allow-Origin", "*")
                .raw_header(
                    "Access-Control-Allow-Methods",
                    "GET, POST, PUT, DELETE, OPTIONS",
                )
                .raw_header(
                    "Access-Control-Allow-Headers",
                    "Authorization, Content-Type",
                )
                .raw_header("Access-Control-Allow-Credentials", "true")
                .sized_body(message.len(), Cursor::new(message.to_owned()))
                .finalize()),
            ControllerError::Internal(inner) => {
                error!("Internal error: {}", inner);
                Err(Status::InternalServerError)
            }
            ControllerError::Database(inner) => {
                error!("Database error: {}", inner);
                Err(Status::InternalServerError)
            }
        }
    }
}
