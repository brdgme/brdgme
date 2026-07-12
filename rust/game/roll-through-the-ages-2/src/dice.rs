//! Port of `brdgme-go/roll_through_the_ages_1/dice.go`.
//!
//! `Roll()`/`RollN()` in Go draw from a package-level `rand.Rand` seeded
//! from wall-clock time; per this repo's deterministic-RNG convention (see
//! `docs/authoring/GAME_DEVELOPMENT.md` and the `greed-2`/`farkle-2`
//! precedent), rolling here instead draws from the game's own `rng` field,
//! wired up in Task 2.

use brdgme_color::{self as color, Color};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Die {
    Food,
    Good,
    Skull,
    Workers,
    FoodOrWorkers,
    Coins,
}

pub const DICE_FACES: [Die; 6] = [
    Die::Food,
    Die::Good,
    Die::Skull,
    Die::Workers,
    Die::FoodOrWorkers,
    Die::Coins,
];

impl Die {
    pub fn face_string(self) -> &'static str {
        match self {
            Die::Food => "FFF",
            Die::Good => "G",
            Die::Skull => "GXG",
            Die::Workers => "WWW",
            Die::FoodOrWorkers => "FF/WW",
            Die::Coins => "C",
        }
    }
}

/// Port of `DiceValueColours`, keyed by the single-letter symbol used in a
/// die's face string.
pub fn dice_value_colour(symbol: char) -> Option<Color> {
    match symbol {
        'F' => Some(color::GREEN),
        'G' => Some(color::PURPLE),
        'X' => Some(color::RED),
        'W' => Some(color::CYAN),
        'C' => Some(color::YELLOW),
        _ => None,
    }
}
