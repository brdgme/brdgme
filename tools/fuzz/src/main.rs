extern crate brdgme_cmd;
extern crate brdgme_fuzz;

use brdgme_cmd::requester;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    brdgme_fuzz::fuzz(move || requester::parse_args(&args).unwrap());
}
