//! Port of `brdgme-go/roll_through_the_ages_1/development.go`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DevelopmentId {
    Leadership,
    Irrigation,
    Agriculture,
    Quarrying,
    Medicine,
    Preservation,
    Coinage,
    Caravans,
    Shipping,
    Smithing,
    Religion,
    Granaries,
    Masonry,
    Engineering,
    Commerce,
    Architecture,
    Empire,
}

/// Go iota order, preserved for anything that needs to iterate all
/// developments deterministically (e.g. render tables).
pub const DEVELOPMENTS: [DevelopmentId; 17] = [
    DevelopmentId::Leadership,
    DevelopmentId::Irrigation,
    DevelopmentId::Agriculture,
    DevelopmentId::Quarrying,
    DevelopmentId::Medicine,
    DevelopmentId::Preservation,
    DevelopmentId::Coinage,
    DevelopmentId::Caravans,
    DevelopmentId::Shipping,
    DevelopmentId::Smithing,
    DevelopmentId::Religion,
    DevelopmentId::Granaries,
    DevelopmentId::Masonry,
    DevelopmentId::Engineering,
    DevelopmentId::Commerce,
    DevelopmentId::Architecture,
    DevelopmentId::Empire,
];

pub struct Development {
    pub name: &'static str,
    pub effect: &'static str,
    pub cost: i32,
    pub points: i32,
}

impl DevelopmentId {
    pub fn value(self) -> Development {
        match self {
            DevelopmentId::Leadership => Development {
                name: "leadership",
                effect: "reroll 1 die (after last roll)",
                cost: 10,
                points: 2,
            },
            DevelopmentId::Irrigation => Development {
                name: "irrigation",
                effect: "drought has no effect",
                cost: 10,
                points: 2,
            },
            DevelopmentId::Agriculture => Development {
                name: "agriculture",
                effect: "+1 food / food die",
                cost: 15,
                points: 3,
            },
            DevelopmentId::Quarrying => Development {
                name: "quarrying",
                effect: "+1 stone if collecting stone",
                cost: 15,
                points: 3,
            },
            DevelopmentId::Medicine => Development {
                name: "medicine",
                effect: "pestilence has no effect",
                cost: 20,
                points: 4,
            },
            DevelopmentId::Preservation => Development {
                name: "preservation",
                effect: "food x2 before roll for 1 pottery",
                cost: 20,
                points: 4,
            },
            DevelopmentId::Coinage => Development {
                name: "coinage",
                effect: "coin die results are worth 12",
                cost: 20,
                points: 4,
            },
            DevelopmentId::Caravans => Development {
                name: "caravans",
                effect: "no need to discard goods",
                cost: 20,
                points: 4,
            },
            DevelopmentId::Shipping => Development {
                name: "shipping",
                effect: "swap 1 good / ship",
                cost: 25,
                points: 5,
            },
            DevelopmentId::Smithing => Development {
                name: "smithing",
                effect: "invasion affects opponents",
                cost: 25,
                points: 5,
            },
            DevelopmentId::Religion => Development {
                name: "religion",
                effect: "revolt affects opponents",
                cost: 25,
                points: 7,
            },
            DevelopmentId::Granaries => Development {
                name: "granaries",
                effect: "sell food for 6 coins each",
                cost: 30,
                points: 6,
            },
            DevelopmentId::Masonry => Development {
                name: "masonry",
                effect: "+1 worker / worker die",
                cost: 30,
                points: 6,
            },
            DevelopmentId::Engineering => Development {
                name: "engineering",
                effect: "use stone for 3 workers each",
                cost: 40,
                points: 6,
            },
            DevelopmentId::Commerce => Development {
                name: "commerce",
                effect: "bonus pts: 1 / good",
                cost: 40,
                points: 8,
            },
            DevelopmentId::Architecture => Development {
                name: "architecture",
                effect: "bonus pts: 2 / monument",
                cost: 60,
                points: 8,
            },
            DevelopmentId::Empire => Development {
                name: "empire",
                effect: "bonus pts: 1 / city",
                cost: 70,
                points: 10,
            },
        }
    }
}
