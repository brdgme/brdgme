use std::fmt;

use brdgme_color::Color;
use serde::{Deserialize, Serialize};

/// Card values match the Go `love_letter_1` int constants: `Guard = 1` up to
/// `Princess = 8`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Card {
    Guard = 1,
    Priest = 2,
    Baron = 3,
    Handmaid = 4,
    Prince = 5,
    King = 6,
    Countess = 7,
    Princess = 8,
}

/// All eight cards, ordered from `Princess` down to `Guard` - matches the Go
/// `CardParserValues` iteration order (`for c := Princess; c >= Guard; c--`).
pub fn princess_to_guard() -> Vec<Card> {
    vec![
        Card::Princess,
        Card::Countess,
        Card::King,
        Card::Prince,
        Card::Handmaid,
        Card::Baron,
        Card::Priest,
        Card::Guard,
    ]
}

impl Card {
    pub fn number(self) -> u8 {
        self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Card::Guard => "Guard",
            Card::Priest => "Priest",
            Card::Baron => "Baron",
            Card::Handmaid => "Handmaid",
            Card::Prince => "Prince",
            Card::King => "King",
            Card::Countess => "Countess",
            Card::Princess => "Princess",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Card::Guard => "Guess another player's card to eliminate them, except for Guard",
            Card::Priest => "Look at another player's hand",
            Card::Baron => "Compare hands with another player, lowest card is eliminated",
            Card::Handmaid => "Immune to the effects of other players' cards until next turn",
            Card::Prince => "Choose a player (or yourself) to discard and draw a new card",
            Card::King => "Trade your hand with another player",
            Card::Countess => "Discard the Countess if you have the King or Prince in your hand",
            Card::Princess => "You are eliminated if you discard the Princess",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Card::Guard => brdgme_color::GREY,
            Card::Priest => brdgme_color::CYAN,
            Card::Baron => brdgme_color::GREEN,
            Card::Handmaid => brdgme_color::BLACK,
            Card::Prince => brdgme_color::PURPLE,
            Card::King => brdgme_color::BLUE,
            Card::Countess => brdgme_color::RED,
            Card::Princess => brdgme_color::YELLOW,
        }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// The initial 16 card deck, in the same order as the Go `Deck` var - order
/// only matters in that it is what gets shuffled.
pub fn initial_deck() -> Vec<Card> {
    vec![
        Card::Guard,
        Card::Guard,
        Card::Guard,
        Card::Guard,
        Card::Guard,
        Card::Priest,
        Card::Priest,
        Card::Baron,
        Card::Baron,
        Card::Handmaid,
        Card::Handmaid,
        Card::Prince,
        Card::Prince,
        Card::King,
        Card::Countess,
        Card::Princess,
    ]
}

/// Number of copies of `card` in the full deck, used by the help table.
pub fn deck_count(card: Card) -> usize {
    initial_deck().iter().filter(|&&c| c == card).count()
}
