use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Suit {
    LiteMetal,
    Yoko,
    ChristineP,
    KarlGitter,
    Krypto,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Rank {
    Open,
    FixedPrice,
    Sealed,
    Double,
    OnceAround,
}

// Field order matters for the derived Ord below: suit first, then rank,
// mirroring libcard's Card.Compare (sort by suit, then by rank).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

pub fn suits() -> Vec<Suit> {
    vec![
        Suit::LiteMetal,
        Suit::Yoko,
        Suit::ChristineP,
        Suit::KarlGitter,
        Suit::Krypto,
    ]
}

pub fn ranks() -> Vec<Rank> {
    vec![
        Rank::Open,
        Rank::FixedPrice,
        Rank::Sealed,
        Rank::Double,
        Rank::OnceAround,
    ]
}

impl Suit {
    pub fn name(self) -> &'static str {
        match self {
            Suit::LiteMetal => "Lite Metal",
            Suit::Yoko => "Yoko",
            Suit::ChristineP => "Christine P",
            Suit::KarlGitter => "Karl Gitter",
            Suit::Krypto => "Krypto",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Suit::LiteMetal => "lm",
            Suit::Yoko => "yo",
            Suit::ChristineP => "cp",
            Suit::KarlGitter => "kg",
            Suit::Krypto => "kr",
        }
    }

    pub fn color(self) -> brdgme_color::NamedColor {
        match self {
            Suit::LiteMetal => brdgme_color::NamedColor::Yellow,
            Suit::Yoko => brdgme_color::NamedColor::Green,
            Suit::ChristineP => brdgme_color::NamedColor::Red,
            Suit::KarlGitter => brdgme_color::NamedColor::Blue,
            // Go used render.Brown.
            Suit::Krypto => brdgme_color::NamedColor::Brown,
        }
    }
}

impl Rank {
    pub fn name(self) -> &'static str {
        match self {
            Rank::Open => "Open",
            Rank::FixedPrice => "Fixed Price",
            Rank::Sealed => "Sealed",
            Rank::Double => "Double",
            Rank::OnceAround => "Once Around",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Rank::Open => "op",
            Rank::FixedPrice => "fp",
            Rank::Sealed => "sl",
            Rank::Double => "db",
            Rank::OnceAround => "oa",
        }
    }
}

impl Card {
    pub fn code(self) -> String {
        format!("{}{}", self.suit.code(), self.rank.code())
    }

    pub fn name(self) -> String {
        format!("{} - {}", self.suit.name(), self.rank.name())
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Number of cards of each suit/rank combination, mirroring Go's
/// cardDistribution table exactly.
fn card_count(suit: Suit, rank: Rank) -> usize {
    match (suit, rank) {
        (Suit::LiteMetal, Rank::Open) => 3,
        (Suit::LiteMetal, Rank::FixedPrice) => 2,
        (Suit::LiteMetal, Rank::Sealed) => 2,
        (Suit::LiteMetal, Rank::Double) => 2,
        (Suit::LiteMetal, Rank::OnceAround) => 3,

        (Suit::Yoko, Rank::Open) => 3,
        (Suit::Yoko, Rank::FixedPrice) => 3,
        (Suit::Yoko, Rank::Sealed) => 3,
        (Suit::Yoko, Rank::Double) => 2,
        (Suit::Yoko, Rank::OnceAround) => 2,

        (Suit::ChristineP, Rank::Open) => 3,
        (Suit::ChristineP, Rank::FixedPrice) => 3,
        (Suit::ChristineP, Rank::Sealed) => 3,
        (Suit::ChristineP, Rank::Double) => 2,
        (Suit::ChristineP, Rank::OnceAround) => 3,

        (Suit::KarlGitter, Rank::Open) => 3,
        (Suit::KarlGitter, Rank::FixedPrice) => 3,
        (Suit::KarlGitter, Rank::Sealed) => 3,
        (Suit::KarlGitter, Rank::Double) => 3,
        (Suit::KarlGitter, Rank::OnceAround) => 3,

        (Suit::Krypto, Rank::Open) => 4,
        (Suit::Krypto, Rank::FixedPrice) => 3,
        (Suit::Krypto, Rank::Sealed) => 3,
        (Suit::Krypto, Rank::Double) => 3,
        (Suit::Krypto, Rank::OnceAround) => 3,
    }
}

pub fn deck() -> Vec<Card> {
    let mut d = vec![];
    for suit in suits() {
        for rank in ranks() {
            for _ in 0..card_count(suit, rank) {
                d.push(Card { suit, rank });
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_has_70_cards() {
        assert_eq!(70, deck().len());
    }

    #[test]
    fn card_sort_order() {
        let mut cards = vec![
            Card {
                suit: Suit::Krypto,
                rank: Rank::Open,
            },
            Card {
                suit: Suit::LiteMetal,
                rank: Rank::OnceAround,
            },
            Card {
                suit: Suit::LiteMetal,
                rank: Rank::Open,
            },
        ];
        cards.sort();
        assert_eq!(
            vec![
                Card {
                    suit: Suit::LiteMetal,
                    rank: Rank::Open,
                },
                Card {
                    suit: Suit::LiteMetal,
                    rank: Rank::OnceAround,
                },
                Card {
                    suit: Suit::Krypto,
                    rank: Rank::Open,
                },
            ],
            cards
        );
    }
}
