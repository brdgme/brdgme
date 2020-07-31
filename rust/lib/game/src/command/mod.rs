use serde::{Deserialize, Serialize};

pub mod doc;
pub mod parser;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum Spec {
    Int {
        min: Option<i32>,
        max: Option<i32>,
    },
    Token(String),
    Enum {
        values: Vec<String>,
        exact: bool,
    },
    OneOf(Vec<Spec>),
    Chain(Vec<Spec>),
    Many {
        spec: Box<Spec>,
        min: Option<usize>,
        max: Option<usize>,
        delim: String,
    },
    Opt(Box<Spec>),
    Doc {
        name: String,
        desc: Option<String>,
        spec: Box<Spec>,
    },
    Player,
    Space,
}
