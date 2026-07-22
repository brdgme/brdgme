use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};

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

pub fn placings_log(placings: &[usize], scores: Option<&[(usize, i32)]>) -> Log {
    let winners: Vec<usize> = placings
        .iter()
        .enumerate()
        .filter(|&(_, &p)| p == 1)
        .map(|(i, _)| i)
        .collect();

    let mut content: Vec<Node> = match winners.len() {
        1 => vec![
            Node::Player(winners[0]),
            Node::Bold(vec![Node::text(" wins!")]),
        ],
        2 => vec![
            Node::Player(winners[0]),
            Node::text(" and "),
            Node::Player(winners[1]),
            Node::Bold(vec![Node::text(" tie!")]),
        ],
        _ => vec![Node::Bold(vec![Node::text("It's a tie!")])],
    };

    if let Some(scores) = scores {
        content.push(Node::text(" Final scores: "));
        for (i, &(player, score)) in scores.iter().enumerate() {
            if i > 0 {
                content.push(Node::text(", "));
            }
            content.push(Node::Player(player));
            content.push(Node::text(": "));
            content.push(Node::Bold(vec![Node::text(score.to_string())]));
        }
    }

    Log::public(content)
}
