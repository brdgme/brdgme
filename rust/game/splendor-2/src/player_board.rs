use serde::{Deserialize, Serialize};

use crate::card::{Card, Noble};
use crate::cost::{self, Cost};

/// Ported from `brdgme-go/splendor_1/player_board.go`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBoard {
    pub cards: Vec<Card>,
    pub reserve: Vec<Card>,
    pub nobles: Vec<Noble>,
    pub tokens: Cost,
}

impl PlayerBoard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bonuses(&self) -> Cost {
        let mut bonuses = Cost::new();
        for c in &self.cards {
            let entry = bonuses.0.entry(c.resource).or_insert(0);
            *entry += 1;
        }
        bonuses
    }

    pub fn buying_power(&self) -> Cost {
        self.bonuses().add(&self.tokens)
    }

    pub fn can_afford(&self, cost: &Cost) -> bool {
        cost::can_afford(&self.buying_power(), cost)
    }

    pub fn prestige(&self) -> i32 {
        let mut prestige = 0;
        for c in &self.cards {
            prestige += c.prestige;
        }
        for n in &self.nobles {
            prestige += n.prestige;
        }
        prestige
    }
}
