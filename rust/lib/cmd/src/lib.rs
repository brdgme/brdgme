#![recursion_limit = "1024"]

pub use crate::repl::repl;

mod repl;

pub mod api;
pub mod bot_cli;
pub mod cli;
#[cfg(feature = "http-server")]
pub mod http;
pub mod requester;
#[cfg(test)]
mod test_game;
#[cfg(feature = "test-support")]
pub mod test_support;
