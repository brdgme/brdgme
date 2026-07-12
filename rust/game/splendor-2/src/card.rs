use serde::{Deserialize, Serialize};

use crate::cost::Cost;

/// Ported from `brdgme-go/splendor_1/card.go`'s `Resources` iota constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    Diamond,
    Sapphire,
    Emerald,
    Ruby,
    Onyx,
    Gold,
    Prestige,
}

pub const RESOURCES: [Resource; 7] = [
    Resource::Diamond,
    Resource::Sapphire,
    Resource::Emerald,
    Resource::Ruby,
    Resource::Onyx,
    Resource::Gold,
    Resource::Prestige,
];

pub const GEMS: [Resource; 5] = [
    Resource::Diamond,
    Resource::Sapphire,
    Resource::Emerald,
    Resource::Ruby,
    Resource::Onyx,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub resource: Resource,
    pub prestige: i32,
    pub cost: Cost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noble {
    pub prestige: i32,
    pub cost: Cost,
}

/// Build a `Cost` from `(Resource, i32)` pairs, e.g. `cost!(Sapphire: 1, Emerald: 1)`.
macro_rules! cost {
    ($($r:ident: $n:expr),* $(,)?) => {
        Cost(std::collections::HashMap::from([
            $((Resource::$r, $n)),*
        ]))
    };
}

/// Transcribed verbatim from `brdgme-go/splendor_1/card.go`'s `Level1Cards`.
pub fn level_1_cards() -> Vec<Card> {
    use Resource::*;
    vec![
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Sapphire: 1, Emerald: 1, Ruby: 1, Onyx: 1),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Sapphire: 1, Emerald: 2, Ruby: 1, Onyx: 1),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Diamond: 3, Sapphire: 1, Onyx: 1),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Sapphire: 2, Emerald: 2, Onyx: 1),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Sapphire: 2, Onyx: 2),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Ruby: 2, Onyx: 1),
        },
        Card {
            resource: Diamond,
            prestige: 1,
            cost: cost!(Emerald: 4),
        },
        Card {
            resource: Diamond,
            prestige: 0,
            cost: cost!(Sapphire: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Diamond: 1, Emerald: 1, Ruby: 1, Onyx: 1),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Diamond: 1, Emerald: 1, Ruby: 2, Onyx: 1),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Sapphire: 1, Emerald: 3, Ruby: 1),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Diamond: 1, Emerald: 2, Ruby: 2),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Emerald: 2, Onyx: 2),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Diamond: 1, Onyx: 2),
        },
        Card {
            resource: Sapphire,
            prestige: 1,
            cost: cost!(Ruby: 4),
        },
        Card {
            resource: Sapphire,
            prestige: 0,
            cost: cost!(Onyx: 3),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 2, Emerald: 1, Ruby: 1),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 1, Emerald: 1, Ruby: 1),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Emerald: 1, Ruby: 3, Onyx: 1),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Diamond: 2, Sapphire: 2, Ruby: 1),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Diamond: 2, Emerald: 2),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Emerald: 2, Ruby: 1),
        },
        Card {
            resource: Onyx,
            prestige: 0,
            cost: cost!(Emerald: 3),
        },
        Card {
            resource: Onyx,
            prestige: 1,
            cost: cost!(Sapphire: 4),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 2, Sapphire: 1, Emerald: 1, Onyx: 1),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 1, Emerald: 1, Onyx: 1),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 1, Ruby: 1, Onyx: 3),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 2, Ruby: 2),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Sapphire: 2, Emerald: 1),
        },
        Card {
            resource: Ruby,
            prestige: 1,
            cost: cost!(Diamond: 4),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 3),
        },
        Card {
            resource: Ruby,
            prestige: 0,
            cost: cost!(Diamond: 2, Emerald: 1, Onyx: 2),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 1, Ruby: 1, Onyx: 1),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 1, Ruby: 1, Onyx: 2),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Diamond: 1, Sapphire: 3, Emerald: 1),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Sapphire: 1, Ruby: 2, Onyx: 2),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Sapphire: 2, Ruby: 2),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Diamond: 2, Sapphire: 1),
        },
        Card {
            resource: Emerald,
            prestige: 0,
            cost: cost!(Ruby: 3),
        },
        Card {
            resource: Emerald,
            prestige: 1,
            cost: cost!(Onyx: 4),
        },
    ]
}

/// Transcribed verbatim from `brdgme-go/splendor_1/card.go`'s `Level2Cards`.
pub fn level_2_cards() -> Vec<Card> {
    use Resource::*;
    vec![
        Card {
            resource: Diamond,
            prestige: 1,
            cost: cost!(Emerald: 3, Ruby: 2, Onyx: 2),
        },
        Card {
            resource: Diamond,
            prestige: 1,
            cost: cost!(Diamond: 2, Sapphire: 3, Ruby: 3),
        },
        Card {
            resource: Diamond,
            prestige: 2,
            cost: cost!(Emerald: 1, Ruby: 4, Onyx: 2),
        },
        Card {
            resource: Diamond,
            prestige: 2,
            cost: cost!(Ruby: 5, Onyx: 3),
        },
        Card {
            resource: Diamond,
            prestige: 2,
            cost: cost!(Ruby: 5),
        },
        Card {
            resource: Diamond,
            prestige: 3,
            cost: cost!(Diamond: 6),
        },
        Card {
            resource: Sapphire,
            prestige: 1,
            cost: cost!(Sapphire: 2, Emerald: 2, Ruby: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 1,
            cost: cost!(Sapphire: 2, Emerald: 3, Onyx: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 2,
            cost: cost!(Diamond: 5, Sapphire: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 2,
            cost: cost!(Diamond: 2, Ruby: 1, Onyx: 4),
        },
        Card {
            resource: Sapphire,
            prestige: 3,
            cost: cost!(Sapphire: 6),
        },
        Card {
            resource: Sapphire,
            prestige: 2,
            cost: cost!(Sapphire: 5),
        },
        Card {
            resource: Onyx,
            prestige: 1,
            cost: cost!(Diamond: 3, Emerald: 3, Onyx: 2),
        },
        Card {
            resource: Onyx,
            prestige: 2,
            cost: cost!(Sapphire: 1, Emerald: 4, Ruby: 2),
        },
        Card {
            resource: Onyx,
            prestige: 1,
            cost: cost!(Diamond: 3, Sapphire: 2, Emerald: 2),
        },
        Card {
            resource: Onyx,
            prestige: 2,
            cost: cost!(Emerald: 5, Ruby: 3),
        },
        Card {
            resource: Onyx,
            prestige: 2,
            cost: cost!(Diamond: 5),
        },
        Card {
            resource: Onyx,
            prestige: 3,
            cost: cost!(Onyx: 6),
        },
        Card {
            resource: Ruby,
            prestige: 1,
            cost: cost!(Diamond: 2, Ruby: 2, Onyx: 3),
        },
        Card {
            resource: Ruby,
            prestige: 1,
            cost: cost!(Sapphire: 3, Ruby: 2, Onyx: 3),
        },
        Card {
            resource: Ruby,
            prestige: 2,
            cost: cost!(Diamond: 1, Sapphire: 4, Emerald: 2),
        },
        Card {
            resource: Ruby,
            prestige: 2,
            cost: cost!(Diamond: 3, Onyx: 5),
        },
        Card {
            resource: Ruby,
            prestige: 2,
            cost: cost!(Onyx: 5),
        },
        Card {
            resource: Ruby,
            prestige: 3,
            cost: cost!(Ruby: 6),
        },
        Card {
            resource: Emerald,
            prestige: 2,
            cost: cost!(Emerald: 5),
        },
        Card {
            resource: Emerald,
            prestige: 2,
            cost: cost!(Sapphire: 5, Emerald: 3),
        },
        Card {
            resource: Emerald,
            prestige: 3,
            cost: cost!(Emerald: 6),
        },
        Card {
            resource: Emerald,
            prestige: 1,
            cost: cost!(Diamond: 2, Sapphire: 3, Onyx: 2),
        },
        Card {
            resource: Emerald,
            prestige: 1,
            cost: cost!(Diamond: 3, Emerald: 2, Ruby: 3),
        },
        Card {
            resource: Emerald,
            prestige: 2,
            cost: cost!(Diamond: 4, Sapphire: 2, Onyx: 1),
        },
    ]
}

/// Transcribed verbatim from `brdgme-go/splendor_1/card.go`'s `Level3Cards`.
pub fn level_3_cards() -> Vec<Card> {
    use Resource::*;
    vec![
        Card {
            resource: Diamond,
            prestige: 4,
            cost: cost!(Diamond: 3, Ruby: 3, Onyx: 6),
        },
        Card {
            resource: Diamond,
            prestige: 4,
            cost: cost!(Onyx: 7),
        },
        Card {
            resource: Diamond,
            prestige: 5,
            cost: cost!(Diamond: 3, Onyx: 7),
        },
        Card {
            resource: Diamond,
            prestige: 3,
            cost: cost!(Sapphire: 3, Emerald: 3, Ruby: 5, Onyx: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 5,
            cost: cost!(Diamond: 7, Sapphire: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 4,
            cost: cost!(Diamond: 6, Sapphire: 3, Onyx: 3),
        },
        Card {
            resource: Sapphire,
            prestige: 3,
            cost: cost!(Diamond: 3, Emerald: 3, Ruby: 3, Onyx: 5),
        },
        Card {
            resource: Sapphire,
            prestige: 4,
            cost: cost!(Diamond: 7),
        },
        Card {
            resource: Onyx,
            prestige: 4,
            cost: cost!(Ruby: 7),
        },
        Card {
            resource: Onyx,
            prestige: 4,
            cost: cost!(Emerald: 3, Ruby: 6, Onyx: 3),
        },
        Card {
            resource: Onyx,
            prestige: 5,
            cost: cost!(Ruby: 7, Onyx: 3),
        },
        Card {
            resource: Onyx,
            prestige: 3,
            cost: cost!(Diamond: 3, Sapphire: 3, Emerald: 5, Ruby: 3),
        },
        Card {
            resource: Ruby,
            prestige: 4,
            cost: cost!(Emerald: 7),
        },
        Card {
            resource: Ruby,
            prestige: 3,
            cost: cost!(Diamond: 3, Sapphire: 5, Emerald: 3, Onyx: 3),
        },
        Card {
            resource: Ruby,
            prestige: 5,
            cost: cost!(Emerald: 7, Ruby: 3),
        },
        Card {
            resource: Ruby,
            prestige: 4,
            cost: cost!(Sapphire: 3, Emerald: 6, Ruby: 3),
        },
        Card {
            resource: Emerald,
            prestige: 4,
            cost: cost!(Diamond: 3, Sapphire: 6, Emerald: 3),
        },
        Card {
            resource: Emerald,
            prestige: 4,
            cost: cost!(Sapphire: 7),
        },
        Card {
            resource: Emerald,
            prestige: 5,
            cost: cost!(Sapphire: 7, Emerald: 3),
        },
        Card {
            resource: Emerald,
            prestige: 3,
            cost: cost!(Diamond: 5, Sapphire: 3, Ruby: 3, Onyx: 3),
        },
    ]
}

/// Transcribed verbatim from `brdgme-go/splendor_1/noble.go`'s `NobleCards`.
pub fn noble_cards() -> Vec<Noble> {
    vec![
        Noble {
            prestige: 3,
            cost: cost!(Emerald: 3, Sapphire: 3, Diamond: 3),
        },
        Noble {
            prestige: 3,
            cost: cost!(Emerald: 3, Sapphire: 3, Ruby: 3),
        },
        Noble {
            prestige: 3,
            cost: cost!(Onyx: 3, Ruby: 3, Diamond: 3),
        },
        Noble {
            prestige: 3,
            cost: cost!(Onyx: 3, Sapphire: 3, Diamond: 3),
        },
        Noble {
            prestige: 3,
            cost: cost!(Onyx: 3, Ruby: 3, Emerald: 3),
        },
        Noble {
            prestige: 3,
            cost: cost!(Onyx: 4, Ruby: 4),
        },
        Noble {
            prestige: 3,
            cost: cost!(Onyx: 4, Diamond: 4),
        },
        Noble {
            prestige: 3,
            cost: cost!(Sapphire: 4, Diamond: 4),
        },
        Noble {
            prestige: 3,
            cost: cost!(Sapphire: 4, Emerald: 4),
        },
        Noble {
            prestige: 3,
            cost: cost!(Ruby: 4, Emerald: 4),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_1_cards() {
        assert_eq!(40, level_1_cards().len());
    }

    #[test]
    fn test_level_2_cards() {
        assert_eq!(30, level_2_cards().len());
    }

    #[test]
    fn test_level_3_cards() {
        assert_eq!(20, level_3_cards().len());
    }

    #[test]
    fn test_noble_cards() {
        let nobles = noble_cards();
        assert_eq!(10, nobles.len());
        for n in &nobles {
            assert_eq!(3, n.prestige);
        }
    }
}
