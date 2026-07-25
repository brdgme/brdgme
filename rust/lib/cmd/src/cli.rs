use std::io::{Read, Write};

pub use crate::api::{Request, Response};
use crate::requester::Requester;

pub fn cli<R: Requester, I: Read, O: Write>(requester: &mut R, input: I, output: &mut O) {
    writeln!(
        output,
        "{}",
        serde_json::to_string(&match serde_json::from_reader::<_, Request>(input) {
            Err(message) => Response::SystemError {
                message: message.to_string(),
            },
            Ok(r) => requester
                .request(&r)
                .unwrap_or_else(|e| Response::SystemError {
                    message: e.to_string(),
                }),
        })
        .expect("failed to encode response JSON")
    )
    .expect("failed to write response to output");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requester::error::RequestError;

    struct FailingRequester;

    impl Requester for FailingRequester {
        fn request(&mut self, _req: &Request) -> Result<Response, RequestError> {
            Err(RequestError::Stdin)
        }
    }

    #[test]
    fn requester_error_becomes_system_error_json() {
        let input = serde_json::to_vec(&Request::PlayerCounts).unwrap();
        let mut out: Vec<u8> = vec![];
        cli(&mut FailingRequester, input.as_slice(), &mut out);
        match serde_json::from_slice::<Response>(&out).unwrap() {
            Response::SystemError { message } => assert_eq!("Failed to get stdin", message),
            r => panic!("expected SystemError, got {:?}", r),
        }
    }
}
