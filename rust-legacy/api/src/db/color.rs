use rand::{self, Rng};
use failure::Error;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use brdgme_color;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Color {
    Green,
    Red,
    Blue,
    Amber,
    Purple,
    Brown,
    BlueGrey,
}

pub static COLORS: &'static [Color] = &[
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::Amber,
    Color::Purple,
    Color::Brown,
    Color::BlueGrey,
];

impl Color {
    pub fn from_strings(from: &[String]) -> Result<Vec<Color>, Error> {
        let mut cols = vec![];
        for s in from {
            cols.push(Color::from_str(s)?)
        }
        Ok(cols)
    }
}

impl Into<brdgme_color::Color> for Color {
    fn into(self) -> brdgme_color::Color {
        match self {
            Color::Green => brdgme_color::GREEN,
            Color::Red => brdgme_color::RED,
            Color::Blue => brdgme_color::BLUE,
            Color::Amber => brdgme_color::AMBER,
            Color::Purple => brdgme_color::PURPLE,
            Color::Brown => brdgme_color::BROWN,
            Color::BlueGrey => brdgme_color::BLUE_GREY,
        }
    }
}

impl ToString for Color {
    fn to_string(&self) -> String {
        match *self {
            Color::Green => "Green",
            Color::Red => "Red",
            Color::Blue => "Blue",
            Color::Amber => "Amber",
            Color::Purple => "Purple",
            Color::Brown => "Brown",
            Color::BlueGrey => "BlueGrey",
        }.to_string()
    }
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(match s {
            "Green" => Color::Green,
            "Red" => Color::Red,
            "Blue" => Color::Blue,
            "Amber" => Color::Amber,
            "Purple" => Color::Purple,
            "Brown" => Color::Brown,
            "BlueGrey" => Color::BlueGrey,
            _ => bail!("Invalid color"),
        })
    }
}

type LocPref = (usize, Vec<Color>);

/// Choose chooses colors based on preferences. First it tries to assign all first preferences, then
/// all second, until all have had colors assigned or it runs out of colors.
pub fn choose(available: &HashSet<&Color>, prefs: &[Vec<Color>]) -> Vec<Color> {
    if available.is_empty() || prefs.is_empty() {
        return vec![];
    }
    let mut sub_prefs = prefs;
    let tail = if prefs.len() > available.len() {
        // There are more people than available colors, so we just repeat the colours for later
        // players.
        let extra = choose(available, &prefs[available.len()..]);
        sub_prefs = &prefs[..available.len()];
        extra
    } else {
        vec![]
    };
    let mut rng = rand::thread_rng();
    let mut remaining = available.clone();
    let mut assigned: HashMap<usize, Color> = HashMap::new();
    let mut rem_prefs = sub_prefs
        .iter()
        .enumerate()
        .map(|(l, pref)| (l, pref.clone()))
        .collect::<Vec<LocPref>>();
    rng.shuffle(&mut rem_prefs);
    'outer: loop {
        'inner: for &(pos, ref pref) in &rem_prefs.clone() {
            if assigned.contains_key(&pos) || pref.is_empty() {
                continue 'inner;
            }
            let want_color = pref[0];
            if remaining.contains(&want_color) {
                assigned.insert(pos, want_color);
                remaining.remove(&want_color);
            }
            if remaining.is_empty() {
                // No colors left
                break 'outer;
            }
        }
        if let Some(new_prefs) = remove_highest_prefs(&rem_prefs) {
            rem_prefs = new_prefs;
        } else {
            // No more preferences, exit
            break 'outer;
        }
    }
    let mut left = remaining.drain();
    let mut res = vec![];
    for p in 0..rem_prefs.len() {
        res.push(
            assigned
                .get(&p)
                .cloned()
                .unwrap_or_else(|| left.next().cloned().unwrap())
                .to_owned(),
        );
    }
    res.extend(tail);
    res
}

fn remove_highest_prefs(prefs: &[LocPref]) -> Option<Vec<LocPref>> {
    let mut some_remain = false;
    let new_prefs = prefs
        .iter()
        .map(|&(pos, ref pref)| {
            let new_pref = if pref.is_empty() {
                vec![]
            } else {
                let p = pref[1..].to_owned();
                if !some_remain && !p.is_empty() {
                    some_remain = true;
                }
                p
            };
            (pos, new_pref)
        })
        .collect::<Vec<LocPref>>();
    if some_remain {
        Some(new_prefs)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_works() {
        use std::iter::FromIterator;
        assert_eq!(
            vec![Color::Amber, Color::Blue, Color::Green],
            choose(
                &HashSet::from_iter(vec![Color::Amber, Color::Blue, Color::Green].iter()),
                &[vec![], vec![Color::Blue, Color::Green], vec![Color::Green]],
            )
        );
    }

    #[test]
    fn choose_with_extra_works() {
        use std::iter::FromIterator;
        assert_eq!(
            vec![Color::Amber, Color::Amber, Color::Amber],
            choose(
                &HashSet::from_iter(vec![Color::Amber].iter()),
                &[vec![], vec![Color::Blue, Color::Green], vec![Color::Green]],
            )
        );
    }
}
