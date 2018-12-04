use serde_derive::{Serialize, Deserialize};

use brdgme_game::errors::GameError;
use brdgme_markup::Node as N;

use std::iter::{self, FromIterator};
use std::ops::Range;
use std::fmt;
use std::collections::HashSet;

use crate::corp::{self, Corp};

pub const WIDTH: usize = 12;
pub const HEIGHT: usize = 9;
pub const SIZE: usize = WIDTH * HEIGHT;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Tile {
    Empty,
    Discarded,
    Unincorporated,
    Corp(Corp),
}

impl Default for Tile {
    fn default() -> Self {
        Tile::Empty
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board(pub Vec<Tile>);

impl Board {
    pub fn get_tile<T: Into<usize>>(&self, at: T) -> Tile {
        self.0.get(at.into()).cloned().unwrap_or_default()
    }

    pub fn set_tile<T: Into<usize>>(&mut self, at: T, t: Tile) {
        let len = self.0.len();
        let at_u = at.into();
        if len <= at_u {
            self.0
                .extend(iter::repeat(Tile::default()).take(at_u - len + 1))
        }
        self.0[at_u] = t;
    }

    pub fn corp_size(&self, c: &Corp) -> usize {
        self.0
            .iter()
            .filter(|t| match **t {
                Tile::Corp(tc) if tc == *c => true,
                _ => false,
            })
            .count()
    }

    pub fn corp_is_safe(&self, c: &Corp) -> bool {
        self.corp_size(c) >= corp::SAFE_SIZE
    }

    pub fn available_corps(&self) -> HashSet<Corp> {
        let mut corps: HashSet<Corp> = HashSet::from_iter(Corp::iter().cloned());
        for l in &Loc::all() {
            if let Tile::Corp(c) = self.get_tile(l) {
                corps.remove(&c);
            }
        }
        corps
    }

    pub fn neighbouring_corps(&self, loc: &Loc) -> HashSet<Corp> {
        let mut corps: HashSet<Corp> = HashSet::new();
        for n_loc in &loc.neighbours() {
            if let Tile::Corp(c) = self.get_tile(n_loc) {
                corps.insert(c);
            }
        }
        corps
    }

    /// Find the largest and second-largest merge candidates. The first value are the corporations
    /// which are being assimilated, and the second value are the target corporations being merged
    /// into.
    pub fn merge_candidates(&self, loc: &Loc) -> (Vec<Corp>, Vec<Corp>) {
        // The larger corporations eating up the smaller ones.
        let mut into: Vec<Corp> = vec![];
        let mut into_size: usize = 0;
        // The smaller corporations being eaten.
        let mut from: Vec<Corp> = vec![];
        let mut from_size: usize = 0;
        for corp in self.neighbouring_corps(loc) {
            let size = self.corp_size(&corp);
            if size > into_size {
                from = into;
                into = vec![];
                from_size = into_size;
                into_size = size;
            }
            if size == into_size {
                into.push(corp);
            } else {
                if size > from_size {
                    from = vec![];
                    from_size = size;
                }
                if size == from_size {
                    from.push(corp);
                }
            }
        }
        if into.len() > 1 {
            // Multiple equal max size, use as from as well.
            from = into.clone();
        }
        (from, into)
    }

    pub fn extend_corp(&mut self, loc: &Loc, corp: &Corp) {
        self.set_tile(loc, Tile::Corp(corp.to_owned()));
        for n_loc in &loc.neighbours() {
            if self.get_tile(n_loc) == Tile::Unincorporated {
                self.extend_corp(n_loc, corp);
            }
        }
    }

    pub fn convert_corp(&mut self, from: &Corp, into: &Corp) {
        for loc in &Loc::all() {
            match self.get_tile(loc) {
                Tile::Corp(c) if c == *from => self.set_tile(loc, Tile::Corp(*into)),
                _ => {}
            }
        }
    }

    pub fn assert_loc_playable(&self, loc: &Loc) -> Result<(), GameError> {
        if self.loc_neighbours_multiple_safe_corps(loc) {
            return Err(GameError::InvalidInput {
                message: "can't merge multiple safe corporations".to_string(),
            });
        }
        if self.loc_founds(loc) && self.available_corps().is_empty() {
            return Err(GameError::InvalidInput {
                message: "there are no available unincorporated corporations".to_string(),
            });
        }
        Ok(())
    }

    pub fn loc_founds(&self, loc: &Loc) -> bool {
        let mut has_unincorporated = false;
        for n_loc in &loc.neighbours() {
            match self.get_tile(n_loc) {
                Tile::Unincorporated => has_unincorporated = true,
                Tile::Corp(_) => return false,
                _ => {}
            }
        }
        has_unincorporated
    }

    pub fn loc_neighbours_multiple_safe_corps(&self, loc: &Loc) -> bool {
        let mut has_safe_corp = false;
        for corp in self.neighbouring_corps(loc) {
            if self.corp_is_safe(&corp) {
                if has_safe_corp {
                    return true;
                }
                has_safe_corp = true
            }
        }
        false
    }

    pub fn set_discarded(&mut self, locs: &[Loc]) {
        for loc in locs {
            self.set_tile(loc, Tile::Discarded);
        }
    }
}

#[cfg(test)]
impl<'a> From<&'a str> for Board {
    fn from(s: &'a str) -> Self {
        let mut board = Board::default();
        for (row, line) in s.trim().lines().enumerate() {
            for (col, ch) in line.trim().chars().enumerate() {
                board.set_tile(
                    Loc { row, col },
                    match ch {
                        'w' | 'W' => Tile::Corp(Corp::Worldwide),
                        's' | 'S' => Tile::Corp(Corp::Sackson),
                        'i' | 'I' => Tile::Corp(Corp::Imperial),
                        'f' | 'F' => Tile::Corp(Corp::Festival),
                        'a' | 'A' => Tile::Corp(Corp::American),
                        'c' | 'C' => Tile::Corp(Corp::Continental),
                        't' | 'T' => Tile::Corp(Corp::Tower),
                        'x' | 'X' => Tile::Discarded,
                        '#' => Tile::Unincorporated,
                        _ => Tile::Empty,
                    },
                );
            }
        }
        board
    }
}

impl Default for Board {
    fn default() -> Self {
        Board(iter::repeat(Tile::default()).take(SIZE).collect())
    }
}

pub fn rows() -> Range<usize> {
    0..HEIGHT
}

pub fn cols() -> Range<usize> {
    0..WIDTH
}

#[derive(Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Loc {
    pub row: usize,
    pub col: usize,
}

impl Loc {
    pub fn all() -> Vec<Loc> {
        rows()
            .flat_map(move |r| cols().map(move |c| Loc { row: r, col: c }))
            .collect()
    }

    pub fn neighbours(&self) -> Vec<Loc> {
        let mut n = vec![];
        if self.col > 0 {
            n.push(Loc {
                col: self.col - 1,
                ..*self
            });
        }
        if self.col < WIDTH - 1 {
            n.push(Loc {
                col: self.col + 1,
                ..*self
            });
        }
        if self.row > 0 {
            n.push(Loc {
                row: self.row - 1,
                ..*self
            });
        }
        if self.row < HEIGHT - 1 {
            n.push(Loc {
                row: self.row + 1,
                ..*self
            });
        }
        n
    }

    pub fn name(&self) -> String {
        format!("{}", self)
    }

    pub fn render(&self) -> N {
        N::Bold(vec![N::text(self.name())])
    }
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", (b'A' + self.row as u8) as char, self.col + 1)
    }
}

impl From<usize> for Loc {
    fn from(u: usize) -> Self {
        Loc {
            row: u / WIDTH,
            col: u % WIDTH,
        }
    }
}

impl<'a> From<&'a Loc> for usize {
    fn from(l: &Loc) -> Self {
        l.row * WIDTH + l.col
    }
}

impl From<Loc> for usize {
    fn from(l: Loc) -> Self {
        (&l).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corp::Corp;

    #[test]
    fn usize_into_loc_works() {
        assert_eq!(Loc::default(), 0.into());
        assert_eq!(Loc { row: 0, col: 8 }, 8.into());
        assert_eq!(Loc { row: 2, col: 3 }, 27.into());
        assert_eq!(Loc { row: 1, col: 11 }, 23.into());
    }

    #[test]
    fn loc_into_usize_works() {
        assert_eq!(0 as usize, Loc::default().into());
        assert_eq!(8 as usize, Loc { row: 0, col: 8 }.into());
        assert_eq!(27 as usize, Loc { row: 2, col: 3 }.into());
        assert_eq!(23 as usize, Loc { row: 1, col: 11 }.into());
    }

    #[test]
    fn board_get_tile_works() {
        let mut b = Board::default();
        b.set_tile(5usize, Tile::Discarded);
        assert_eq!(Tile::Discarded, b.get_tile(5usize));
        assert_eq!(Tile::Empty, b.get_tile(99999usize));
    }

    #[test]
    fn board_indexing_by_loc_works() {
        let b = Board::default();
        assert_eq!(Tile::Empty, b.get_tile(Loc { row: 5, col: 4 }));
    }

    #[test]
    fn board_set_tile_works() {
        let mut b = Board::default();
        b.set_tile(99999usize, Tile::Unincorporated);
    }

    #[test]
    fn board_corp_size_works() {
        let mut b = Board::default();
        b.set_tile(2usize, Tile::Corp(Corp::American));
        b.set_tile(3usize, Tile::Corp(Corp::American));
        b.set_tile(4usize, Tile::Corp(Corp::Sackson));
        assert_eq!(0, b.corp_size(&Corp::Continental));
        assert_eq!(1, b.corp_size(&Corp::Sackson));
        assert_eq!(2, b.corp_size(&Corp::American));
    }
}
