use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::http::{ContentType, Status};
use rocket::http::hyper::header::{AccessControlAllowCredentials, AccessControlAllowHeaders,
                                  AccessControlAllowMethods, AccessControlAllowOrigin};
use hyper::method::Method;
use unicase::UniCase;
use failure::{Context, Error};
use diesel;

use std::io::Cursor;

#[derive(Fail, Debug)]
pub enum ControllerError {
    #[fail(display = "Bad request: {}", message)] BadRequest { message: String },
    #[fail(display = "Internal error: {}", inner)] Internal { inner: Error },
}

impl ControllerError {
    pub fn bad_request<T: Into<String>>(message: T) -> Self {
        ControllerError::BadRequest {
            message: message.into(),
        }
    }
}

impl From<Error> for ControllerError {
    fn from(error: Error) -> Self {
        ControllerError::Internal { inner: error }
    }
}

impl<'a> From<Context<&'a str>> for ControllerError {
    fn from(error: Context<&str>) -> Self {
        let err: Error = error.into();
        err.into()
    }
}

impl From<diesel::result::Error> for ControllerError {
    fn from(error: diesel::result::Error) -> Self {
        let err: Error = error.into();
        err.into()
    }
}

impl<'r> Responder<'r> for ControllerError {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        match self {
            ControllerError::BadRequest { ref message } => Ok(Response::build()
                .status(Status::BadRequest)
                .header(ContentType::Plain)
                .header(AccessControlAllowOrigin::Any)
                .header(AccessControlAllowMethods(vec![
                    Method::Get,
                    Method::Post,
                    Method::Put,
                    Method::Delete,
                    Method::Options,
                ]))
                .header(AccessControlAllowHeaders(vec![
                    UniCase("Authorization".to_string()),
                    UniCase("Content-Type".to_string()),
                ]))
                .header(AccessControlAllowCredentials)
                .sized_body(Cursor::new(message.to_owned()))
                .finalize()),
            ControllerError::Internal { inner } => {
                error!("{}, {}", inner.cause(), inner.backtrace());
                Err(Status::InternalServerError)
            }
        }
    }
}
