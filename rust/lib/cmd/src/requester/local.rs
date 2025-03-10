use std::ffi::OsString;
use std::io::{BufWriter, Write, stderr};
use std::process::{Command, Stdio};

use crate::api::{Request, Response};
use crate::requester::Requester;
use crate::requester::error::RequestError;

pub struct LocalRequester {
    path: OsString,
}

impl LocalRequester {
    pub fn new<I: Into<OsString>>(path: I) -> Self {
        LocalRequester { path: path.into() }
    }
}

impl Requester for LocalRequester {
    fn request(&mut self, req: &Request) -> Result<Response, RequestError> {
        let mut cmd = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let mut wr = cmd.stdin.as_mut().ok_or(RequestError::Stdin)?;
            let mut bufwr = BufWriter::new(&mut wr);

            bufwr.write_all(serde_json::to_string(req)?.as_bytes())?;
            bufwr.flush()?;
        }

        let output = cmd.wait_with_output()?;

        if !output.stderr.is_empty() {
            stderr().write_all(&output.stderr)?;
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
