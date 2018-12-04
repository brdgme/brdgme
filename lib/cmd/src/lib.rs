#![recursion_limit = "1024"]

mod repl;
pub use crate::repl::repl;

pub mod api;
pub mod bot_cli;
pub mod cli;
pub mod requester;
