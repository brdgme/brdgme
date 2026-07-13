//! Ported from `brdgme-go/libpoker/hand.go`, scoped down to the subset
//! `texas_holdem_1` uses directly (`Result`, `WinningHandResult`,
//! `HandResult{Category, Cards, Name}`). The other functions here
//! (`is_straight`, `find_multiple`, `cards_by_suit`, ...) are private
//! implementation details in Go that `Result` itself depends on, so they are
//! ported too even though texas_holdem_1 never calls them directly.
//!
//! Not ported: none - every function in hand.go is either used directly by
//! texas_holdem_1 or is a transitive dependency of `Result`.

use std::collections::HashMap;

use crate::card::{Deck, RANK_2, RANK_ACE_HIGH, Suit};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Category {
    None,
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
}

#[derive(Clone, Debug, Default)]
pub struct HandResult {
    pub category: Option<Category>,
    pub cards: Deck,
    pub name: String,
}

impl HandResult {
    /// Port of `HandResult.HandScore`.
    pub fn hand_score(&self) -> Vec<i32> {
        let mut score = vec![self.category.unwrap_or(Category::None) as i32];
        score.extend(self.cards.iter().map(|c| c.rank as i32));
        score
    }
}

/// Port of `Result`.
pub fn result(hand: &Deck) -> HandResult {
    let cards_by_suit = cards_by_suit(hand);
    let mut res = HandResult::default();
    // Straight flush.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let suit_cards = cards_by_suit.get(&suit).cloned().unwrap_or_default();
        let (ok, cards) = is_straight(&suit_cards);
        if ok
            && (res.category.unwrap_or(Category::None) < Category::StraightFlush
                || cards[0].rank > res.cards[0].rank)
        {
            res.category = Some(Category::StraightFlush);
            res.cards = cards;
        }
    }
    if res.category.is_some() {
        res.name = "straight flush".to_string();
        return res;
    }
    // Four of a kind.
    let (ok, cards) = is_four_of_a_kind(hand);
    if ok {
        res.category = Some(Category::FourOfAKind);
        res.cards = cards;
        res.name = "four of a kind".to_string();
        return res;
    }
    // Full house.
    let (ok, cards) = is_full_house(hand);
    if ok {
        res.category = Some(Category::FullHouse);
        res.cards = cards;
        res.name = "full house".to_string();
        return res;
    }
    // Flush.
    let (ok, cards) = is_flush(hand);
    if ok {
        res.category = Some(Category::Flush);
        res.cards = cards;
        res.name = "flush".to_string();
        return res;
    }
    // Straight.
    let (ok, cards) = is_straight(hand);
    if ok {
        res.category = Some(Category::Straight);
        res.cards = cards;
        res.name = "straight".to_string();
        return res;
    }
    // Three of a kind.
    let (ok, cards) = is_three_of_a_kind(hand);
    if ok {
        res.category = Some(Category::ThreeOfAKind);
        res.cards = cards;
        res.name = "three of a kind".to_string();
        return res;
    }
    // Two pair.
    let (ok, cards) = is_two_pair(hand);
    if ok {
        res.category = Some(Category::TwoPair);
        res.cards = cards;
        res.name = "two pair".to_string();
        return res;
    }
    // One pair.
    let (ok, cards) = is_one_pair(hand);
    if ok {
        res.category = Some(Category::OnePair);
        res.cards = cards;
        res.name = "one pair".to_string();
        return res;
    }
    // High card.
    res.category = Some(Category::HighCard);
    let (cards, _) = find_highest_rank(hand, 5);
    res.cards = cards;
    res.name = "high card".to_string();
    res
}

/// Port of `IsStraight`.
pub fn is_straight(hand: &Deck) -> (bool, Deck) {
    if hand.len() < 5 {
        return (false, Deck::new());
    }
    let by_rank = cards_by_rank(hand);
    let mut ok = false;
    let mut cards: Deck = Deck::new();
    let mut rank = RANK_ACE_HIGH as i32;
    while rank >= 2 {
        let r = rank as u8;
        if !by_rank[&r].is_empty() {
            cards.push(by_rank[&r][0]);
            if cards.len() == 5 {
                ok = true;
                break;
            }
        } else {
            cards = Deck::new();
        }
        rank -= 1;
    }
    if cards.len() == 4 && !by_rank[&RANK_ACE_HIGH].is_empty() {
        // Ace also counts as low.
        ok = true;
        cards.push(by_rank[&RANK_ACE_HIGH][0]);
    }
    (ok, cards)
}

/// Port of `IsFourOfAKind`.
pub fn is_four_of_a_kind(hand: &Deck) -> (bool, Deck) {
    let (ok, mut cards, remaining) = find_multiple(hand, 4);
    if ok {
        let (kicker, _) = find_highest_rank(&remaining, 1);
        cards.extend(kicker);
    }
    (ok, cards)
}

/// Port of `IsFullHouse`.
pub fn is_full_house(hand: &Deck) -> (bool, Deck) {
    let (ok, mut cards, remaining) = find_multiple(hand, 3);
    if ok {
        let (pair_ok, pair, _) = find_multiple(&remaining, 2);
        if pair_ok {
            cards.extend(pair);
        }
        return (pair_ok, cards);
    }
    (false, cards)
}

/// Port of `IsFlush`.
pub fn is_flush(hand: &Deck) -> (bool, Deck) {
    let mut hand_results: HashMap<i32, HandResult> = HashMap::new();
    let mut i = 0;
    let by_suit = cards_by_suit(hand);
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let suit_cards = by_suit.get(&suit).cloned().unwrap_or_default();
        if suit_cards.len() >= 5 {
            let (flush, _) = find_highest_rank(&suit_cards, 5);
            hand_results.insert(
                i,
                HandResult {
                    category: Some(Category::Flush),
                    cards: flush,
                    name: String::new(),
                },
            );
            i += 1;
        }
    }
    if !hand_results.is_empty() {
        let winning_hand = winning_hand_result(&hand_results);
        if !winning_hand.is_empty() {
            return (true, hand_results[&winning_hand[0]].cards.clone());
        }
    }
    (false, Deck::new())
}

/// Port of `IsThreeOfAKind`.
pub fn is_three_of_a_kind(hand: &Deck) -> (bool, Deck) {
    let (ok, mut cards, remaining) = find_multiple(hand, 3);
    if ok {
        let (kickers, _) = find_highest_rank(&remaining, 2);
        cards.extend(kickers);
    }
    (ok, cards)
}

/// Port of `IsTwoPair`.
pub fn is_two_pair(hand: &Deck) -> (bool, Deck) {
    let (ok, mut cards, remaining) = find_multiple(hand, 2);
    if ok {
        let (pair_ok, pair, remaining) = find_multiple(&remaining, 2);
        if pair_ok {
            cards.extend(pair);
            let (kicker, _) = find_highest_rank(&remaining, 1);
            cards.extend(kicker);
        }
        return (pair_ok, cards);
    }
    (false, cards)
}

/// Port of `IsOnePair`.
pub fn is_one_pair(hand: &Deck) -> (bool, Deck) {
    let (ok, mut cards, remaining) = find_multiple(hand, 2);
    if ok {
        let (kickers, _) = find_highest_rank(&remaining, 3);
        cards.extend(kickers);
    }
    (ok, cards)
}

/// Port of `FindMultiple`.
pub fn find_multiple(hand: &Deck, n: usize) -> (bool, Deck, Deck) {
    let mut remaining = hand.clone();
    let by_rank = cards_by_rank(&remaining);
    let mut ok = false;
    let mut cards: Deck = Deck::new();
    let mut rank = RANK_ACE_HIGH as i32;
    while rank >= 0 {
        let r = rank as u8;
        if by_rank.get(&r).map(|v| v.len()).unwrap_or(0) >= n {
            ok = true;
            cards = by_rank[&r][..n].to_vec();
            for c in &cards {
                // Remove one instance of each card in `cards` from `remaining`,
                // mirroring Go's `remaining.Remove(c, 1)`.
                if let Some(pos) = remaining.iter().position(|x| x == c) {
                    remaining.remove(pos);
                }
            }
            break;
        }
        rank -= 1;
    }
    (ok, cards, remaining)
}

/// Port of `FindHighestRank`.
pub fn find_highest_rank(hand: &Deck, n: usize) -> (Deck, Deck) {
    let remaining = hand.clone();
    let by_rank = cards_by_rank(&remaining);
    let mut highest: Deck = Deck::new();
    let mut rank = RANK_ACE_HIGH as i32;
    while rank >= 0 {
        let r = rank as u8;
        let bucket = by_rank.get(&r).cloned().unwrap_or_default();
        let mut take = n - highest.len();
        if bucket.len() < take {
            take = bucket.len();
        }
        highest.extend(bucket[..take].to_vec());
        if highest.len() == n {
            break;
        }
        rank -= 1;
    }
    (highest, remaining)
}

/// Port of `CardsBySuit`. Breaks down a deck to cards by suit, sorted by
/// rank ascending.
///
/// Go quirk preserved: both the initialisation loop and the sort loop use
/// `for i := CLUBS; i < SPADES; i++` (strictly less than), so the `Spades`
/// bucket is never explicitly pre-initialised and, more importantly, is
/// never run through `.Sort()`. Spades cards still end up in the map (Go's
/// `ranksBySuit[s] = ranksBySuit[s].Push(c)` creates the entry on first
/// write, and reading a missing key returns the zero value), but in
/// whatever order they were encountered in `hand`, not rank order. Every
/// caller of `CardsBySuit` (`IsFlush`, `Result`'s straight-flush loop)
/// re-derives rank order itself via `CardsByRank`/`FindHighestRank` before
/// using the cards, so this appears to have no observable effect - but it
/// is preserved verbatim rather than "fixed" per the porting correctness
/// rule.
pub fn cards_by_suit(hand: &Deck) -> HashMap<Suit, Deck> {
    let mut ranks_by_suit: HashMap<Suit, Deck> = HashMap::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        ranks_by_suit.insert(suit, Deck::new());
    }
    for c in hand {
        ranks_by_suit.entry(c.suit).or_default().push(*c);
    }
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if let Some(d) = ranks_by_suit.get_mut(&suit) {
            d.sort();
        }
    }
    ranks_by_suit
}

/// Port of `CardsByRank`.
///
/// Go's version only pre-initialises ranks `RANK_2..RANK_ACE_HIGH`
/// (exclusive of ace-high) because Go map lookups on a missing key return
/// the zero value rather than panicking; `RANK_ACE_HIGH` still ends up
/// populated whenever an ace is actually in the hand (Go assigns into the
/// map on categorise), and reads of a still-missing `RANK_ACE_HIGH` key
/// silently return an empty slice. Rust's `HashMap` indexing panics on a
/// missing key, so this pre-initialises the full `RANK_2..=RANK_ACE_HIGH`
/// range to preserve the same observable behaviour without relying on
/// `HashMap::get`/`unwrap_or_default` at every call site.
pub fn cards_by_rank(hand: &Deck) -> HashMap<u8, Deck> {
    let mut suits_by_rank: HashMap<u8, Deck> = HashMap::new();
    for r in RANK_2..=RANK_ACE_HIGH {
        suits_by_rank.insert(r, Deck::new());
    }
    for c in hand {
        suits_by_rank.entry(c.rank).or_default().push(*c);
    }
    suits_by_rank
}

/// Port of `WinningHandResult`. Takes a map keyed by an arbitrary hand index
/// (mirroring Go's `map[int]HandResult`) and returns the indices of the
/// winning hand(s).
pub fn winning_hand_result(hand_results: &HashMap<i32, HandResult>) -> Vec<i32> {
    let mut hand_scores: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut next_pass: Vec<i32> = vec![];
    for (id, hr) in hand_results {
        if hr.category.is_some() {
            hand_scores.insert(*id, hr.hand_score());
            next_pass.push(*id);
        }
    }
    let mut val_index = 0usize;
    while next_pass.len() > 1 {
        let mut leaders: Vec<i32> = vec![];
        let mut highest = 0;
        for &hand_index in &next_pass {
            if hand_scores[&hand_index].len() <= val_index {
                // Run out of cards, call it a tie.
                return next_pass;
            }
            if hand_scores[&hand_index][val_index] > highest {
                leaders = vec![];
                highest = hand_scores[&hand_index][val_index];
            }
            if hand_scores[&hand_index][val_index] == highest {
                leaders.push(hand_index);
            }
        }
        val_index += 1;
        next_pass = leaders;
    }
    next_pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, RANK_10, RANK_JACK, RANK_KING, RANK_QUEEN};

    fn c(suit: Suit, rank: u8) -> Card {
        Card { suit, rank }
    }

    fn build_hand_by_ranks(ranks: &[u8]) -> Deck {
        ranks.iter().map(|&r| c(Suit::Clubs, r)).collect()
    }

    #[test]
    fn test_cards_by_suit() {
        // Note: uses rank 1 (Go's low `STANDARD_52_RANK_ACE`, the smallest
        // rank value), not ace-high - ascending sort puts it first.
        let hand = vec![
            c(Suit::Diamonds, RANK_KING),
            c(Suit::Diamonds, 1),
            c(Suit::Diamonds, 4),
            c(Suit::Spades, 8),
        ];
        let by_suit = cards_by_suit(&hand);
        assert_eq!(3, by_suit[&Suit::Diamonds].len());
        assert_eq!(1, by_suit[&Suit::Diamonds][0].rank);
    }

    #[test]
    fn test_is_straight() {
        let hand = build_hand_by_ranks(&[2, 6, 3, 8, 6]);
        let (ok, _) = is_straight(&hand);
        assert!(!ok, "Detected as straight but isn't");

        let hand = build_hand_by_ranks(&[2, 6, 3, 4, 5]);
        let (ok, cards) = is_straight(&hand);
        assert!(ok, "Didn't detect as straight");
        assert_eq!(5, cards.len());
        assert_eq!(6, cards[0].rank);

        let hand = build_hand_by_ranks(&[2, 6, 3, 4, 5, 4]);
        let (ok, cards) = is_straight(&hand);
        assert!(ok, "Didn't detect as straight");
        assert_eq!(5, cards.len());
        assert_eq!(6, cards[0].rank);

        // Ace as low card.
        let hand = build_hand_by_ranks(&[2, 14, 3, 5, 4]);
        let (ok, cards) = is_straight(&hand);
        assert!(ok, "Didn't detect as straight");
        assert_eq!(5, cards.len());
        assert_eq!(5, cards[0].rank);

        // Ace as high card.
        let hand = build_hand_by_ranks(&[11, 10, 13, 12, 14]);
        let (ok, cards) = is_straight(&hand);
        assert!(ok, "Didn't detect as straight");
        assert_eq!(5, cards.len());
        assert_eq!(14, cards[0].rank);
    }

    #[test]
    fn test_straight_flush() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 7),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Diamonds, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::StraightFlush), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(7, hand_result.cards[0].rank);
    }

    #[test]
    fn test_four_of_a_kind() {
        let hand_result = result(&vec![
            c(Suit::Hearts, 3),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, 3),
            c(Suit::Diamonds, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::FourOfAKind), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(3, hand_result.cards[0].rank);
        assert_eq!(6, hand_result.cards[4].rank);
    }

    #[test]
    fn test_full_house() {
        let hand_result = result(&vec![
            c(Suit::Hearts, 3),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, 3),
            c(Suit::Diamonds, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 6),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::FullHouse), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(3, hand_result.cards[0].rank);
        assert_eq!(6, hand_result.cards[3].rank);
    }

    #[test]
    fn test_flush() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 7),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Diamonds, RANK_JACK),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::Flush), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(RANK_JACK, hand_result.cards[0].rank);
        assert_eq!(7, hand_result.cards[1].rank);
        assert_eq!(5, hand_result.cards[2].rank);
        assert_eq!(4, hand_result.cards[3].rank);
        assert_eq!(3, hand_result.cards[4].rank);
    }

    #[test]
    fn test_straight() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Spades, RANK_ACE_HIGH),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::Straight), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(5, hand_result.cards[0].rank);
    }

    #[test]
    fn test_three_of_a_kind() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Spades, 3),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, 5),
        ]);
        assert_eq!(Some(Category::ThreeOfAKind), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(3, hand_result.cards[0].rank);
    }

    #[test]
    fn test_two_pair() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Spades, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 3),
            c(Suit::Diamonds, RANK_KING),
        ]);
        assert_eq!(Some(Category::TwoPair), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(RANK_KING, hand_result.cards[0].rank);
        assert_eq!(3, hand_result.cards[2].rank);
        assert_eq!(6, hand_result.cards[4].rank);
    }

    #[test]
    fn test_one_pair() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Spades, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 9),
            c(Suit::Diamonds, RANK_KING),
        ]);
        assert_eq!(Some(Category::OnePair), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(RANK_KING, hand_result.cards[0].rank);
        assert_eq!(9, hand_result.cards[2].rank);
        assert_eq!(6, hand_result.cards[3].rank);
        assert_eq!(4, hand_result.cards[4].rank);
    }

    #[test]
    fn test_high_card() {
        let hand_result = result(&vec![
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Spades, RANK_KING),
            c(Suit::Spades, 6),
            c(Suit::Diamonds, 4),
            c(Suit::Clubs, 9),
            c(Suit::Diamonds, RANK_QUEEN),
        ]);
        assert_eq!(Some(Category::HighCard), hand_result.category);
        assert_eq!(5, hand_result.cards.len());
        assert_eq!(RANK_KING, hand_result.cards[0].rank);
        assert_eq!(RANK_QUEEN, hand_result.cards[1].rank);
        assert_eq!(9, hand_result.cards[2].rank);
        assert_eq!(6, hand_result.cards[3].rank);
        assert_eq!(4, hand_result.cards[4].rank);
    }

    #[test]
    fn test_hand_score() {
        let hr = HandResult {
            category: Some(Category::Straight),
            cards: vec![c(Suit::Clubs, 3), c(Suit::Clubs, 4), c(Suit::Clubs, 5)],
            name: String::new(),
        };
        let hs = hr.hand_score();
        assert_eq!(4, hs.len());
        assert_eq!(Category::Straight as i32, hs[0]);
        assert_eq!(3, hs[1]);
        assert_eq!(4, hs[2]);
        assert_eq!(5, hs[3]);
    }

    #[test]
    fn test_winning_hand_result() {
        let mut hand_results: HashMap<i32, HandResult> = HashMap::new();
        // 0 is a pair.
        hand_results.insert(
            0,
            result(&vec![
                c(Suit::Diamonds, 2),
                c(Suit::Diamonds, 3),
                c(Suit::Spades, RANK_KING),
                c(Suit::Spades, 6),
                c(Suit::Diamonds, 4),
                c(Suit::Clubs, 9),
                c(Suit::Diamonds, RANK_KING),
            ]),
        );
        // 1 is full house.
        hand_results.insert(
            1,
            result(&vec![
                c(Suit::Hearts, 3),
                c(Suit::Diamonds, 3),
                c(Suit::Spades, 3),
                c(Suit::Diamonds, 6),
                c(Suit::Diamonds, 4),
                c(Suit::Clubs, 6),
                c(Suit::Diamonds, 5),
            ]),
        );
        // 2 is the same full house.
        hand_results.insert(
            2,
            result(&vec![
                c(Suit::Hearts, 3),
                c(Suit::Diamonds, 3),
                c(Suit::Spades, 3),
                c(Suit::Diamonds, 6),
                c(Suit::Diamonds, 4),
                c(Suit::Clubs, 6),
                c(Suit::Diamonds, 5),
            ]),
        );
        hand_results.insert(3, HandResult::default());
        let winning_results = winning_hand_result(&hand_results);
        assert_eq!(2, winning_results.len());
        assert!(winning_results.contains(&1));
        assert!(winning_results.contains(&2));
    }

    #[test]
    fn test_ace_is_in_flush_result() {
        // https://github.com/Miniand/brdg.me/issues/4
        let hand = vec![
            c(Suit::Diamonds, RANK_QUEEN),
            c(Suit::Diamonds, RANK_ACE_HIGH),
        ];
        let community_cards = vec![
            c(Suit::Diamonds, RANK_10),
            c(Suit::Spades, RANK_QUEEN),
            c(Suit::Diamonds, 4),
            c(Suit::Diamonds, 7),
            c(Suit::Spades, 4),
        ];
        let mut all = hand;
        all.extend(community_cards);
        let hand_result = result(&all);
        assert_eq!(5, hand_result.cards.len());
    }
}
