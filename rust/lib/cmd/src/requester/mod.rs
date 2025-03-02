use crate::api::{Request, Response};

pub mod error;
pub mod gamer;
pub mod local;

use crate::requester::error::{ParseArgsError, RequestError};

pub trait Requester {
    fn request(&mut self, req: &Request) -> Result<Response, RequestError>;
}

pub fn parse_args(args: &[String]) -> Result<impl Requester + use<>, ParseArgsError> {
    let args_len = args.len();
    if args_len < 2 {
        return Err(ParseArgsError::TypeMissing);
    }
    Ok(match args[1].as_ref() {
        "local" => {
            if args_len < 3 {
                return Err(ParseArgsError::PathMissing);
            }
            local::LocalRequester::new(&args[2])
        }
        _ => return Err(ParseArgsError::TypeMissing),
    })
}
