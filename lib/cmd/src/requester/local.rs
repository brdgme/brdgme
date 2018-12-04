use failure::{Error, format_err};
use serde_json;

use std::ffi::OsString;
use std::io::{BufWriter, Write};
use std::process::{Command, Stdio};

use crate::api::{Request, Response};
use crate::requester::Requester;

pub struct LocalRequester {
    path: OsString,
}

impl LocalRequester {
    pub fn new<I: Into<OsString>>(path: I) -> Self {
        LocalRequester { path: path.into() }
    }
}

impl Requester for LocalRequester {
    fn request(&mut self, req: &Request) -> Result<Response, Error> {
        let mut cmd = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let mut wr = cmd.stdin
                .as_mut()
                .ok_or(format_err!("failed to get stdin"))?;
            let mut bufwr = BufWriter::new(&mut wr);

            bufwr.write_all(serde_json::to_string(req)?.as_bytes())?;
            bufwr.flush()?;
        }

        let output = cmd.wait_with_output()?;

        serde_json::from_slice(&output.stdout).map_err(|e| {
            format_err!(
                "failed to parse JSON: {}\n\nChild process stderr:\n{}\n\nChild process stdout:\n{}\n\n",
                e,
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }
}
