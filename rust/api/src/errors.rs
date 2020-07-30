use diesel;
use hyper::Method;
use rocket::http::hyper::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, AUTHORIZATION, CONTENT_TYPE,
};
use rocket::http::{ContentType, Status};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use std::io::Cursor;

#[derive(thiserror::Error, Debug)]
pub enum ControllerError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },
    #[error("Internal error: {inner}")]
    Internal { inner: Box<dyn std::error::Error> },
}

impl ControllerError {
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        ControllerError::BadRequest {
            message: message.into(),
        }
    }
}

impl From<diesel::result::Error> for ControllerError {
    fn from(error: diesel::result::Error) -> Self {
        ControllerError::Internal {
            inner: Box::new(error),
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ControllerError {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        match self {
            ControllerError::BadRequest { ref message } => Ok(Response::build()
                .status(Status::BadRequest)
                .header(ContentType::Plain)
                .raw_header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .raw_header(
                    ACCESS_CONTROL_ALLOW_METHODS,
                    vec![
                        Method::Get,
                        Method::Post,
                        Method::Put,
                        Method::Delete,
                        Method::Options,
                    ],
                )
                .raw_header(
                    ACCESS_CONTROL_ALLOW_HEADERS,
                    vec![AUTHORIZATION, CONTENT_TYPE],
                )
                .raw_header(ACCESS_CONTROL_ALLOW_CREDENTIALS, true)
                .sized_body(Cursor::new(message.to_owned()))
                .finalize()),
            ControllerError::Internal { inner } => {
                error!("{}, {}", inner.cause(), inner.backtrace());
                Err(Status::InternalServerError)
            }
        }
    }
}
