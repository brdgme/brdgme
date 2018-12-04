use serde_json;

use std::io::{Read, Write};

use crate::api::{Request, Response};
use crate::requester::Requester;

pub fn cli<R: Requester, I: Read, O: Write>(requester: &mut R, input: I, output: &mut O) {
    writeln!(
        output,
        "{}",
        serde_json::to_string(&match serde_json::from_reader::<_, Request>(input) {
            Err(message) => Response::SystemError {
                message: message.to_string(),
            },
            Ok(r) => requester.request(&r).unwrap(),
        }).unwrap()
    ).unwrap();
}
