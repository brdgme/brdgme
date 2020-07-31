use diesel;
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use log::error;
use std::io::Cursor;

#[derive(thiserror::Error, Debug)]
pub enum ControllerError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },
    #[error("Internal error: {inner}")]
    Internal { inner: anyhow::Error },
}

impl ControllerError {
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        ControllerError::BadRequest {
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for ControllerError {
    fn from(error: anyhow::Error) -> Self {
        ControllerError::Internal { inner: error }
    }
}

impl From<diesel::result::Error> for ControllerError {
    fn from(error: diesel::result::Error) -> Self {
        ControllerError::Internal {
            inner: error.into(),
        }
    }
}

impl<'r> Responder<'r> for ControllerError {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
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
                .sized_body(Cursor::new(message.to_owned()))
                .finalize()),
            ControllerError::Internal { inner } => {
                error!("{}", inner);
                Err(Status::InternalServerError)
            }
        }
    }
}
