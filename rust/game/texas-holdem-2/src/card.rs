//! Ported from `brdgme-go/libcard` (suit_rank.go, standard.go, deck.go),
//! scoped down to the subset `texas_holdem_1` actually uses (see
//! `docs/porting/GAME_PORTING.md` / splendor-2 precedent for inlining a
//! library as crate-local modules rather than a shared crate).
//!
//! Unported from the Go library (not used by texas_holdem_1, so left out):
//! - `Standard52Deck` (low-ace deck), `Standard52DeckWithJokers`, the
//!   `STANDARD_52_SUIT_JOKER` suit and low `STANDARD_52_RANK_ACE`/joker rank
//!   handling in rendering.
//! - `Deck.Contains`, `Deck.Remove`, `Deck.Push`, `Deck.Unshift`,
//!   `Deck.UnshiftMany`, `Deck.Shift`, `Deck.ShiftN`, `Deck.Pop` (single-card
//!   variants/helpers texas_holdem_1 never calls; `PopN`/`PushMany`/`Sort` are
//!   the ones it uses).
//! - `RenderStandard52Hidden` / `RenderStandard52HiddenFixedWidth` (rendering
//!   a face-down card; texas_holdem_1 doesn't hide cards this way).

use brdgme_color::Color;
use brdgme_markup::Node as N;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

/// Rank as the raw Go int value (2-10, then 11=Jack, 12=Queen, 13=King,
/// 14=AceHigh), matching `STANDARD_52_RANK_*` exactly so poker.rs's rank
/// arithmetic (ace-high straights, `CardsByRank` indexing) ports 1:1.
pub const RANK_2: u8 = 2;
pub const RANK_3: u8 = 3;
pub const RANK_4: u8 = 4;
pub const RANK_5: u8 = 5;
pub const RANK_6: u8 = 6;
pub const RANK_7: u8 = 7;
pub const RANK_8: u8 = 8;
pub const RANK_9: u8 = 9;
pub const RANK_10: u8 = 10;
pub const RANK_JACK: u8 = 11;
pub const RANK_QUEEN: u8 = 12;
pub const RANK_KING: u8 = 13;
pub const RANK_ACE_HIGH: u8 = 14;

// Field order matters: sort by suit first, then rank, mirroring libcard's
// `Card.Compare`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: u8,
}

pub type Deck = Vec<Card>;

/// Port of `Standard52DeckAceHigh` - the only deck constructor texas_holdem_1
/// uses (aces rank above kings).
pub fn standard_52_deck_ace_high() -> Deck {
    let mut d = Deck::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        for rank in RANK_2..=RANK_ACE_HIGH {
            d.push(Card { suit, rank });
        }
    }
    d
}

/// Port of `Deck.Sort` - returns a new sorted deck, leaving the input
/// untouched (Go's version copies before calling `sort.Sort`).
pub fn sort(deck: &[Card]) -> Deck {
    let mut d = deck.to_vec();
    d.sort();
    d
}

/// Port of `Deck.Shuffle`. Go seeds from `time.Now()`; per
/// `docs/authoring/GAME_DEVELOPMENT.md` this takes the game's `GameRng`
/// instead of ambient time-based randomness so shuffles are deterministic
/// given the game seed.
pub fn shuffle<R: rand::Rng + ?Sized>(deck: &[Card], rng: &mut R) -> Deck {
    use rand::seq::SliceRandom;
    let mut d = deck.to_vec();
    d.shuffle(rng);
    d
}

/// Port of `Deck.PushMany` - returns a new deck with `cards` appended to the
/// end.
pub fn push_many(deck: &[Card], cards: &[Card]) -> Deck {
    let mut d = deck.to_vec();
    d.extend_from_slice(cards);
    d
}

/// Port of `Deck.PopN`.
///
/// Go quirk preserved exactly: `PopN` takes cards from the **end** (top) of
/// the deck, not the front - `d[d.Len()-n:]` is the popped portion and
/// `d[:d.Len()-n]` is what remains. Returned order is unchanged (the popped
/// slice keeps the deck's existing order; it is not reversed).
///
/// Panics if `deck.len() < n`, matching Go's `panic("Not enough cards to pop")`.
pub fn pop_n(deck: &[Card], n: usize) -> (Deck, Deck) {
    let len = deck.len();
    if len < n {
        panic!("Not enough cards to pop");
    }
    (deck[len - n..].to_vec(), deck[..len - n].to_vec())
}

impl Suit {
    fn symbol(self) -> &'static str {
        match self {
            Suit::Clubs => "♣",
            Suit::Diamonds => "♦",
            Suit::Hearts => "♥",
            Suit::Spades => "♠",
        }
    }

    fn color(self) -> Color {
        match self {
            Suit::Clubs | Suit::Spades => brdgme_color::BLACK,
            Suit::Diamonds | Suit::Hearts => brdgme_color::RED,
        }
    }
}

fn rank_str(rank: u8) -> String {
    match rank {
        1 | RANK_ACE_HIGH => "A".to_string(),
        RANK_JACK => "J".to_string(),
        RANK_QUEEN => "Q".to_string(),
        RANK_KING => "K".to_string(),
        r => format!("{r}"),
    }
}

impl Card {
    /// Port of `Card.RenderStandard52`.
    pub fn render_standard_52(self) -> N {
        N::Fg(
            self.suit.color().into(),
            vec![N::text(format!(
                "{}{}",
                self.suit.symbol(),
                rank_str(self.rank)
            ))],
        )
    }

    /// Port of `Card.RenderStandard52FixedWidth`.
    ///
    /// Go quirk preserved: padding is decided by `c.Rank != 10`, a numeric
    /// comparison, not by the rendered string's length. Every rank other
    /// than 10 gets a trailing space, including "A" (ace-high, rank 14),
    /// even though "A" is the same width as "J"/"Q"/"K" which also get
    /// padded - i.e. the padding logic isn't actually about width at all,
    /// it just happens to line up because only "10" is two characters.
    pub fn render_standard_52_fixed_width(self) -> N {
        if self.rank == RANK_10 {
            self.render_standard_52()
        } else {
            N::Group(vec![self.render_standard_52(), N::text(" ")])
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;

    use super::*;

    fn node_text(n: &N) -> String {
        brdgme_markup::plain(&brdgme_markup::transform(std::slice::from_ref(n), &[]))
    }

    #[test]
    fn test_ace_high() {
        // Port of libcard's TestAceHigh.
        let d = standard_52_deck_ace_high();
        assert!(d[d.len() - 1].rank > RANK_KING);
    }

    #[test]
    fn test_shuffle() {
        // Port of libcard's TestShuffle. Seed chosen arbitrarily; the point
        // is just that the deterministic RNG still produces a different
        // order than the input.
        let d = standard_52_deck_ace_high();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        assert_ne!(d, shuffle(&d, &mut rng));
    }

    #[test]
    fn test_sort() {
        // Port of libcard's TestSort.
        let d = standard_52_deck_ace_high();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let shuffled = shuffle(&d, &mut rng);
        let d_clone = shuffled.clone();
        let sorted = sort(&shuffled);
        assert_eq!(sorted, d);
        // Sorting doesn't mutate the input.
        assert_eq!(shuffled, d_clone);
    }

    #[test]
    fn test_push_many() {
        // Port of libcard's TestPushMany.
        let d = standard_52_deck_ace_high();
        let cards = vec![
            Card {
                suit: Suit::Clubs,
                rank: 50,
            },
            Card {
                suit: Suit::Diamonds,
                rank: 51,
            },
        ];
        let new_d = push_many(&d, &cards);
        assert_eq!(52, d.len(), "PushMany modified original deck");
        assert_eq!(54, new_d.len());
        assert_eq!(new_d[52], cards[0]);
        assert_eq!(new_d[53], cards[1]);
    }

    #[test]
    fn test_pop_n() {
        // Port of libcard's TestPopN, adjusted for the ace-high deck (the
        // only deck constructor this crate ports): last two cards are King
        // and Ace of Spades.
        let d = standard_52_deck_ace_high();
        let (cards, new_d) = pop_n(&d, 2);
        assert_eq!(52, d.len(), "PopN modified original deck");
        assert_eq!(50, new_d.len());
        assert_eq!(2, cards.len());
        assert_eq!(
            cards[0],
            Card {
                suit: Suit::Spades,
                rank: RANK_KING,
            }
        );
        assert_eq!(
            cards[1],
            Card {
                suit: Suit::Spades,
                rank: RANK_ACE_HIGH,
            }
        );
    }

    #[test]
    #[should_panic(expected = "Not enough cards to pop")]
    fn test_pop_n_panics_when_not_enough_cards() {
        let d = standard_52_deck_ace_high();
        pop_n(&d, d.len() + 1);
    }

    #[test]
    fn example_deal() {
        // Port of libcard's ExampleDeal.
        let d = standard_52_deck_ace_high();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let d = shuffle(&d, &mut rng);
        let (player1_hand, d) = pop_n(&d, 5);
        let player1_hand = sort(&player1_hand);
        let (player2_hand, d) = pop_n(&d, 5);
        let player2_hand = sort(&player2_hand);
        let (player3_hand, d) = pop_n(&d, 5);
        let player3_hand = sort(&player3_hand);
        assert_eq!(5, player1_hand.len());
        assert_eq!(5, player2_hand.len());
        assert_eq!(5, player3_hand.len());
        assert_eq!(37, d.len());
    }

    #[test]
    fn test_render_standard_52() {
        // Port of the non-hidden-card assertions in libcard's
        // TestRenderStandard52.
        let c = Card {
            suit: Suit::Clubs,
            rank: 1, // Go's STANDARD_52_RANK_ACE; renders as "A" like ace-high.
        };
        assert_eq!("♣A", node_text(&c.render_standard_52()));
        assert_eq!("♣A ", node_text(&c.render_standard_52_fixed_width()));

        let c = Card {
            suit: Suit::Diamonds,
            rank: RANK_10,
        };
        assert_eq!("♦10", node_text(&c.render_standard_52()));

        let c = Card {
            suit: Suit::Hearts,
            rank: RANK_KING,
        };
        assert_eq!("♥K ", node_text(&c.render_standard_52_fixed_width()));
    }
}
