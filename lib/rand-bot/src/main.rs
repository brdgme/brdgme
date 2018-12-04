extern crate brdgme_rand_bot;

use std::io::{stdin, stdout};

fn main() {
    brdgme_rand_bot::cli(stdin(), &mut stdout());
}
