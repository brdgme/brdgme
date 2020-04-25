use failure::{bail, Error};

use crate::api::{Request, Response};

pub mod gamer;
pub mod local;

pub trait Requester {
    fn request(&mut self, req: &Request) -> Result<Response, Error>;
}

pub fn parse_args(args: &[String]) -> Result<impl Requester, Error> {
    let args_len = args.len();
    if args_len < 2 {
        bail!("expected a type argument of 'local'");
    }
    Ok(match args[1].as_ref() {
        "local" => {
            if args_len < 3 {
                bail!("expected a path argument");
            }
            local::LocalRequester::new(&args[2])
        }
        _ => panic!("expected one of 'local'"),
    })
}
