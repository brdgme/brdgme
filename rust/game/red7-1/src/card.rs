use std::fmt;

use brdgme_color::NamedColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Violet,
    Indigo,
    Blue,
    Green,
    Yellow,
    Orange,
    Red,
}

impl Suit {
    pub const ALL: [Suit; 7] = [
        Suit::Violet,
        Suit::Indigo,
        Suit::Blue,
        Suit::Green,
        Suit::Yellow,
        Suit::Orange,
        Suit::Red,
    ];

    pub fn abbr(self) -> &'static str {
        match self {
            Suit::Violet => "V",
            Suit::Indigo => "I",
            Suit::Blue => "B",
            Suit::Green => "G",
            Suit::Yellow => "Y",
            Suit::Orange => "O",
            Suit::Red => "R",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Suit::Violet => "Violet",
            Suit::Indigo => "Indigo",
            Suit::Blue => "Blue",
            Suit::Green => "Green",
            Suit::Yellow => "Yellow",
            Suit::Orange => "Orange",
            Suit::Red => "Red",
        }
    }

    pub fn rule_str(self) -> &'static str {
        match self {
            Suit::Red => "Highest card",
            Suit::Orange => "Same number",
            Suit::Yellow => "Same color",
            Suit::Green => "Even cards",
            Suit::Blue => "Most colors",
            Suit::Indigo => "In a row",
            Suit::Violet => "Below 4",
        }
    }

    pub fn color(self) -> NamedColor {
        match self {
            Suit::Violet => NamedColor::Purple,
            Suit::Indigo => NamedColor::Blue,
            Suit::Blue => NamedColor::Cyan,
            Suit::Green => NamedColor::Green,
            Suit::Yellow => NamedColor::Yellow,
            Suit::Orange => NamedColor::Orange,
            Suit::Red => NamedColor::Red,
        }
    }

    pub fn from_abbr(c: char) -> Option<Suit> {
        match c.to_ascii_uppercase() {
            'V' => Some(Suit::Violet),
            'I' => Some(Suit::Indigo),
            'B' => Some(Suit::Blue),
            'G' => Some(Suit::Green),
            'Y' => Some(Suit::Yellow),
            'O' => Some(Suit::Orange),
            'R' => Some(Suit::Red),
            _ => None,
        }
    }

    fn ordinal(self) -> u8 {
        match self {
            Suit::Violet => 0,
            Suit::Indigo => 1,
            Suit::Blue => 2,
            Suit::Green => 3,
            Suit::Yellow => 4,
            Suit::Orange => 5,
            Suit::Red => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: u8,
}

impl Card {
    pub fn new(suit: Suit, rank: u8) -> Self {
        Card { suit, rank }
    }

    pub fn parse(input: &str) -> Option<Card> {
        let chars: Vec<char> = input.chars().collect();
        if chars.len() != 2 {
            return None;
        }
        let suit = Suit::from_abbr(chars[0])?;
        let rank = chars[1].to_digit(10)? as u8;
        if !(1..=7).contains(&rank) {
            return None;
        }
        Some(Card { suit, rank })
    }

    pub fn points(self) -> u32 {
        self.rank as u32
    }

    pub fn sort_key(self) -> (u8, u8) {
        (self.suit.ordinal(), self.rank)
    }

    pub fn rank_key(self) -> (u8, u8) {
        (self.rank, self.suit.ordinal())
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.suit.abbr(), self.rank)
    }
}

pub fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(49);
    for suit in Suit::ALL {
        for rank in 1..=7 {
            deck.push(Card::new(suit, rank));
        }
    }
    deck
}

pub fn points(cards: &[Card]) -> u32 {
    cards.iter().map(|c| c.points()).sum()
}

pub fn sort_by_suit(cards: &mut [Card]) {
    cards.sort_by_key(|c| c.sort_key());
}

pub fn highest_card(cards: &[Card]) -> Vec<Card> {
    cards
        .iter()
        .copied()
        .max_by_key(|c| c.rank_key())
        .map(|c| vec![c])
        .unwrap_or_default()
}

pub fn cards_of_one_number(cards: &[Card]) -> Vec<Card> {
    let mut sorted: Vec<Card> = cards.to_vec();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.rank_key()));

    let mut most: Vec<Card> = vec![];
    let mut cur: Vec<Card> = vec![];
    let mut cur_rank: Option<u8> = None;

    for c in &sorted {
        if Some(c.rank) != cur_rank {
            if cur.len() > most.len() {
                most = cur.clone();
            }
            cur = vec![*c];
            cur_rank = Some(c.rank);
        } else {
            cur.push(*c);
        }
    }
    if cur.len() > most.len() {
        most = cur;
    }
    most
}

pub fn cards_of_one_color(cards: &[Card]) -> Vec<Card> {
    let mut sorted: Vec<Card> = cards.to_vec();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.rank_key()));

    let mut by_suit: Vec<Vec<Card>> = vec![];
    let mut n = 0usize;

    for c in &sorted {
        let idx = by_suit.iter().position(|s| s[0].suit == c.suit);
        match idx {
            Some(i) => {
                by_suit[i].push(*c);
                if by_suit[i].len() > n {
                    n = by_suit[i].len();
                }
            }
            None => {
                by_suit.push(vec![*c]);
                if 1 > n {
                    n = 1;
                }
            }
        }
    }

    for s in &by_suit {
        if s.len() == n {
            return s.clone();
        }
    }
    vec![]
}

pub fn most_even_cards(cards: &[Card]) -> Vec<Card> {
    cards.iter().copied().filter(|c| c.rank % 2 == 0).collect()
}

pub fn cards_of_different_colors(cards: &[Card]) -> Vec<Card> {
    let mut sorted: Vec<Card> = cards.to_vec();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.rank_key()));

    let mut used_suits: Vec<Suit> = vec![];
    let mut matching: Vec<Card> = vec![];

    for c in &sorted {
        if !used_suits.contains(&c.suit) {
            used_suits.push(c.suit);
            matching.push(*c);
        }
    }
    matching
}

pub fn cards_that_form_a_run(cards: &[Card]) -> Vec<Card> {
    let mut sorted: Vec<Card> = cards.to_vec();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.rank_key()));

    let mut last_rank: u8 = 0;
    let mut cur: Vec<Card> = vec![];
    let mut longest: Vec<Card> = vec![];

    for c in &sorted {
        let rank = c.rank;
        if rank == last_rank {
            continue;
        } else if rank == last_rank.wrapping_sub(1) {
            cur.push(*c);
        } else {
            if cur.len() > longest.len() {
                longest = cur.clone();
            }
            cur = vec![*c];
        }
        last_rank = rank;
    }
    if cur.len() > longest.len() {
        longest = cur;
    }
    longest
}

pub fn most_cards_below_4(cards: &[Card]) -> Vec<Card> {
    let mut sorted: Vec<Card> = cards.to_vec();
    sorted.sort_by_key(|c| std::cmp::Reverse(c.rank_key()));

    sorted.into_iter().filter(|c| c.rank < 4).collect()
}

pub fn suit_rule(suit: Suit) -> fn(&[Card]) -> Vec<Card> {
    match suit {
        Suit::Red => highest_card,
        Suit::Orange => cards_of_one_number,
        Suit::Yellow => cards_of_one_color,
        Suit::Green => most_even_cards,
        Suit::Blue => cards_of_different_colors,
        Suit::Indigo => cards_that_form_a_run,
        Suit::Violet => most_cards_below_4,
    }
}

pub fn leader(palettes: &[Vec<Card>]) -> (usize, Vec<Card>) {
    if palettes.is_empty() {
        return (0, vec![]);
    }
    let mut leader_idx = 0;
    let mut leader_palette = palettes[0].clone();

    for (i, p) in palettes.iter().enumerate().skip(1) {
        let l_max = leader_palette
            .iter()
            .map(|c| c.rank_key())
            .max()
            .unwrap_or((0, 0));
        let i_max = p.iter().map(|c| c.rank_key()).max().unwrap_or((0, 0));
        if p.len() > leader_palette.len() || (p.len() == leader_palette.len() && i_max > l_max) {
            leader_idx = i;
            leader_palette = p.clone();
        }
    }
    (leader_idx, leader_palette)
}
