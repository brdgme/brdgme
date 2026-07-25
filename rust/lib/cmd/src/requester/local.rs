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

        if !output.status.success() {
            return Err(RequestError::ChildExit {
                status: output.status,
            });
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failing_child_reports_exit_status_not_json_error() {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join("brdgme_cmd_local_requester_fail.sh");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"#!/bin/sh\ncat >/dev/null\nexit 3\n").unwrap();
        }
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();

        let mut requester = LocalRequester::new(path.as_os_str());
        let err = requester.request(&Request::PlayerCounts).unwrap_err();
        let _ = std::fs::remove_file(&path);
        match err {
            RequestError::ChildExit { status } => assert_eq!(Some(3), status.code()),
            e => panic!("expected ChildExit, got {:?}", e),
        }
    }
}
