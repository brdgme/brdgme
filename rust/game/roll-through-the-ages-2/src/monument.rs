//! Port of `brdgme-go/roll_through_the_ages_1/monument.go`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MonumentId {
    StepPyramid,
    StoneCircle,
    Temple,
    Obelisk,
    HangingGardens,
    GreatWall,
    GreatPyramid,
}

pub const MONUMENTS: [MonumentId; 7] = [
    MonumentId::StepPyramid,
    MonumentId::StoneCircle,
    MonumentId::Temple,
    MonumentId::Obelisk,
    MonumentId::HangingGardens,
    MonumentId::GreatWall,
    MonumentId::GreatPyramid,
];

pub struct Monument {
    pub name: &'static str,
    pub size: i32,
    pub points: i32,
    pub subsequent_points: i32,
    pub effect: &'static str,
}

impl MonumentId {
    pub fn value(self) -> Monument {
        match self {
            MonumentId::StepPyramid => Monument {
                name: "Step Pyramid",
                size: 3,
                points: 1,
                subsequent_points: 0,
                effect: "",
            },
            MonumentId::StoneCircle => Monument {
                name: "Stone Circle",
                size: 5,
                points: 2,
                subsequent_points: 1,
                effect: "",
            },
            MonumentId::Temple => Monument {
                name: "Temple",
                size: 7,
                points: 4,
                subsequent_points: 3,
                effect: "",
            },
            MonumentId::Obelisk => Monument {
                name: "Obelisk",
                size: 9,
                points: 6,
                subsequent_points: 4,
                effect: "",
            },
            MonumentId::HangingGardens => Monument {
                name: "Hanging Gardens",
                size: 11,
                points: 8,
                subsequent_points: 5,
                effect: "",
            },
            // Go source note: "Changed from Great Wall to help string
            // matching" - preserved verbatim, this is the display name, not
            // a rename to fix.
            MonumentId::GreatWall => Monument {
                name: "Wall",
                size: 13,
                points: 10,
                subsequent_points: 6,
                effect: "invasion has no effect",
            },
            MonumentId::GreatPyramid => Monument {
                name: "Great Pyramid",
                size: 15,
                points: 12,
                subsequent_points: 8,
                effect: "",
            },
        }
    }
}
