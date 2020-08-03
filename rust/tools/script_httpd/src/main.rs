use hyper::server::{Handler, Request, Response, Server};

use std::env;
use std::io::{copy, Write};
use std::process::{Command, Stdio};

struct ScriptHandler {
    script: String,
}

impl Handler for ScriptHandler {
    fn handle(&self, mut req: Request, res: Response) {
        let mut child = Command::new(&self.script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        {
            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            copy(&mut req, stdin).expect("Failed to write request to stdin");
        }

        res.start()
            .unwrap()
            .write_all(
                &child
                    .wait_with_output()
                    .expect("Failed to read command output")
                    .stdout,
            )
            .expect("Failed to write response");
    }
}

fn main() {
    Server::http("0.0.0.0:80")
        .unwrap()
        .handle(ScriptHandler {
            script: env::args().nth(1).unwrap(),
        })
        .unwrap();
}
