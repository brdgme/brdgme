use std::env;

use brdgme_cmd::repl;
use brdgme_cmd::requester;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut client = requester::parse_args(&args).unwrap();
    repl(&mut client);
}
