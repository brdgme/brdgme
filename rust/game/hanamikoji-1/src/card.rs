use std::fmt;

use brdgme_color::NamedColor;
use serde::{Deserialize, Serialize};

/// The seven geisha, ordered by ascending charm so that `index` is a stable
/// 0..7 board position. Charm values are the standard distribution
/// {2,2,2,3,3,4,5} (sum 21); the name-to-value mapping is cosmetic (the real
/// game uses Japanese art names), only the multiset and 3-copies-each matter.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Geisha {
    Flute,
    Koto,
    Fan,
    Shamisen,
    Umbrella,
    Taiko,
    Tea,
}

impl Geisha {
    pub const ALL: [Geisha; 7] = [
        Geisha::Flute,
        Geisha::Koto,
        Geisha::Fan,
        Geisha::Shamisen,
        Geisha::Umbrella,
        Geisha::Taiko,
        Geisha::Tea,
    ];

    pub fn charm(self) -> i32 {
        match self {
            Geisha::Flute => 2,
            Geisha::Koto => 2,
            Geisha::Fan => 2,
            Geisha::Shamisen => 3,
            Geisha::Umbrella => 3,
            Geisha::Taiko => 4,
            Geisha::Tea => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Geisha::Flute => "Flute",
            Geisha::Koto => "Koto",
            Geisha::Fan => "Fan",
            Geisha::Shamisen => "Shamisen",
            Geisha::Umbrella => "Umbrella",
            Geisha::Taiko => "Taiko",
            Geisha::Tea => "Tea",
        }
    }

    pub fn color(self) -> NamedColor {
        match self {
            Geisha::Flute => NamedColor::Cyan,
            Geisha::Koto => NamedColor::Green,
            Geisha::Fan => NamedColor::Pink,
            Geisha::Shamisen => NamedColor::Purple,
            Geisha::Umbrella => NamedColor::Blue,
            Geisha::Taiko => NamedColor::Orange,
            Geisha::Tea => NamedColor::Yellow,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Geisha::Flute => 0,
            Geisha::Koto => 1,
            Geisha::Fan => 2,
            Geisha::Shamisen => 3,
            Geisha::Umbrella => 4,
            Geisha::Taiko => 5,
            Geisha::Tea => 6,
        }
    }

    /// The full 21-card deck: three identical item cards per geisha.
    pub fn full_deck() -> Vec<Geisha> {
        let mut deck: Vec<Geisha> = vec![];
        for g in Geisha::ALL {
            for _ in 0..3 {
                deck.push(g);
            }
        }
        deck
    }
}

impl fmt::Display for Geisha {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
