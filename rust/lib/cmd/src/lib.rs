#![recursion_limit = "1024"]

pub use crate::repl::repl;

mod repl;

pub mod api;
pub mod bot_cli;
pub mod cli;
pub mod http;
pub mod requester;
