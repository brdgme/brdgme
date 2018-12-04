use serde_derive::{Serialize, Deserialize};

use std::fmt;
use std::collections::HashMap;

use brdgme_color;

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Value {
    Investment,
    N(usize),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Expedition {
    Red,
    Green,
    White,
    Blue,
    Yellow,
}

impl Expedition {
    pub fn color(&self) -> brdgme_color::Color {
        match *self {
            Expedition::Red => brdgme_color::RED,
            Expedition::Green => brdgme_color::GREEN,
            Expedition::White => brdgme_color::GREY,
            Expedition::Blue => brdgme_color::BLUE,
            Expedition::Yellow => brdgme_color::AMBER,
        }
    }

    fn abbrev(&self) -> String {
        match *self {
            Expedition::Red => "R".to_string(),
            Expedition::Green => "G".to_string(),
            Expedition::White => "W".to_string(),
            Expedition::Blue => "B".to_string(),
            Expedition::Yellow => "Y".to_string(),
        }
    }
}

impl fmt::Display for Expedition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.abbrev())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Value::Investment => "X".to_string(),
                Value::N(n) => format!("{}", n),
            }
        )
    }
}

pub fn expeditions() -> Vec<Expedition> {
    vec![
        Expedition::Red,
        Expedition::Green,
        Expedition::White,
        Expedition::Blue,
        Expedition::Yellow,
    ]
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Card {
    pub expedition: Expedition,
    pub value: Value,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.expedition, self.value)
    }
}

impl From<(Expedition, Value)> for Card {
    fn from((e, v): (Expedition, Value)) -> Self {
        Self {
            expedition: e,
            value: v,
        }
    }
}

pub fn by_expedition(cards: &[Card]) -> HashMap<Expedition, Vec<Card>> {
    let mut output: HashMap<Expedition, Vec<Card>> = HashMap::new();
    for e in expeditions() {
        output.insert(e, of_expedition(cards, e));
    }
    output
}

pub fn of_expedition(cards: &[Card], expedition: Expedition) -> Vec<Card> {
    cards
        .iter()
        .filter(|c| c.expedition == expedition)
        .cloned()
        .collect()
}

pub fn last_expedition(cards: &[Card], expedition: Expedition) -> Option<Card> {
    cards
        .iter()
        .rev()
        .find(|c| c.expedition == expedition)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn value_cmp_works() {
        assert_eq!(Ordering::Less, Value::Investment.cmp(&Value::N(2)));
        assert_eq!(Ordering::Less, Value::N(2).cmp(&Value::N(3)));
    }
}
