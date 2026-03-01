use time::{OffsetDateTime, PrimitiveDateTime};
use serde::{Deserialize, Serialize};

use brdgme_markup::Node;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Log {
    pub content: Vec<Node>,
    pub at: PrimitiveDateTime,
    pub public: bool,
    pub to: Vec<usize>,
}

impl Log {
    pub fn public(content: Vec<Node>) -> Log {
        let now = OffsetDateTime::now_utc();
        Log {
            content,
            at: PrimitiveDateTime::new(now.date(), now.time()),
            public: true,
            to: vec![],
        }
    }

    pub fn private(content: Vec<Node>, to: Vec<usize>) -> Log {
        let now = OffsetDateTime::now_utc();
        Log {
            content,
            at: PrimitiveDateTime::new(now.date(), now.time()),
            public: false,
            to,
        }
    }
}
