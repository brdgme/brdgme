use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::de::{Error as DeError, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use brdgme_game::Log;
use brdgme_game::rng::GameRng;
use brdgme_markup::Node as N;

use crate::casino::Casino;
use crate::roll;
use crate::tile::TILES;

pub const BLOCK_WIDTH: usize = 3;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Block {
    A,
    B,
    C,
    D,
    E,
    F,
}

pub static BLOCKS: &[Block] = &[Block::A, Block::B, Block::C, Block::D, Block::E, Block::F];

impl Block {
    pub fn max_lot(self) -> Lot {
        match self {
            Block::A | Block::B | Block::E => 6,
            Block::C => 12,
            Block::D | Block::F => 9,
        }
    }
}

impl Block {
    fn parse_char(value: char) -> Result<Self, String> {
        match value {
            'A' => Ok(Block::A),
            'B' => Ok(Block::B),
            'C' => Ok(Block::C),
            'D' => Ok(Block::D),
            'E' => Ok(Block::E),
            'F' => Ok(Block::F),
            _ => Err("expected block character A-F".to_string()),
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Block::A => "A",
                Block::B => "B",
                Block::C => "C",
                Block::D => "D",
                Block::E => "E",
                Block::F => "F",
            }
        )
    }
}

pub type Lot = usize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub struct Loc {
    pub block: Block,
    pub lot: Lot,
}

impl Loc {
    fn parse_str(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("Loc string is empty".to_string());
        }
        let mut chars = value.chars();
        let block = Block::parse_char(chars.next().unwrap())?;
        let lot_str: String = chars.collect();
        let lot: Lot = lot_str
            .parse()
            .map_err(|_| "Loc lot must be a number".to_string())?;
        if lot < 1 || lot > block.max_lot() {
            return Err(format!("Loc lot must be between 1 and {}", block.max_lot()));
        }
        Ok((block, lot).into())
    }
}

impl From<(Block, Lot)> for Loc {
    fn from((block, lot): (Block, Lot)) -> Self {
        Loc { block, lot }
    }
}

impl Loc {
    pub fn neighbours(&self) -> Vec<Loc> {
        let mut n: Vec<Loc> = vec![];
        if self.lot > BLOCK_WIDTH {
            n.push((self.block, self.lot - BLOCK_WIDTH).into());
        }
        if self.lot % BLOCK_WIDTH != 1 {
            n.push((self.block, self.lot - 1).into());
        }
        if !self.lot.is_multiple_of(BLOCK_WIDTH) {
            n.push((self.block, self.lot + 1).into());
        }
        if self.lot <= self.block.max_lot() - BLOCK_WIDTH {
            n.push((self.block, self.lot + BLOCK_WIDTH).into());
        }
        n
    }

    pub fn render(&self) -> N {
        N::Bold(vec![N::text(format!("{}", self))])
    }
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.block, self.lot)
    }
}

// We use custom serialisation for `Loc` as it needs to become a string type to be used in JSON maps

impl Serialize for Loc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

struct LocVisitor;

impl Visitor<'_> for LocVisitor {
    type Value = Loc;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a loc such as B2")
    }

    fn visit_str<E>(self, value: &str) -> Result<Loc, E>
    where
        E: DeError,
    {
        Loc::parse_str(value).map_err(|e| DeError::invalid_value(Unexpected::Other(&e), &self))
    }
}

impl<'de> Deserialize<'de> for Loc {
    fn deserialize<D>(deserializer: D) -> Result<Loc, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LocVisitor)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileOwner {
    pub player: usize,
    pub die: usize,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
pub enum BoardTile {
    #[default]
    Unowned,
    Owned {
        player: usize,
    },
    Built {
        owner: Option<TileOwner>,
        casino: Casino,
        height: usize,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Board(HashMap<Loc, BoardTile>);

#[derive(Default, Copy, Clone)]
pub struct UsedResources {
    pub dice: usize,
    pub tokens: usize,
}

impl Board {
    pub fn get(&self, loc: &Loc) -> BoardTile {
        self.0.get(loc).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, loc: Loc, bt: BoardTile) {
        self.0.insert(loc, bt);
    }

    pub fn used_resources(&self, p: usize) -> UsedResources {
        let mut used = UsedResources::default();
        for bt in self.0.values() {
            match *bt {
                BoardTile::Owned { player } if player == p => used.tokens += 1,
                BoardTile::Built {
                    owner: Some(TileOwner { player, .. }),
                    ..
                } if player == p => used.dice += 1,
                _ => {}
            }
        }
        used
    }

    pub fn casino_tile_count(&self, c: Casino) -> usize {
        self.0.iter().fold(0, |acc, (_, bt)| match *bt {
            BoardTile::Built { casino, .. } if casino == c => acc + 1,
            _ => acc,
        })
    }

    pub fn player_locs(&self, p: usize) -> Vec<Loc> {
        self.0
            .iter()
            .filter_map(|(l, bt)| match *bt {
                BoardTile::Owned { player } if player == p => Some(*l),
                _ => None,
            })
            .collect()
    }

    pub fn casino_at(&self, loc: &Loc) -> Option<BoardCasino> {
        let (casino, height) = match self.get(loc) {
            BoardTile::Built { casino, height, .. } => (casino, height),
            _ => return None,
        };

        let mut queue: BTreeSet<Loc> = BTreeSet::new();
        queue.insert(*loc);
        let mut visited: HashSet<Loc> = HashSet::new();
        let mut tiles: Vec<CasinoTile> = vec![];

        while let Some(next) = queue.pop_first() {
            visited.insert(next);
            match self.get(&next) {
                BoardTile::Built {
                    casino: c,
                    owner,
                    height: h,
                } if c == casino && h == height => {
                    tiles.push(CasinoTile { loc: next, owner });
                    for n in next.neighbours() {
                        if !visited.contains(&n) {
                            queue.insert(n);
                        }
                    }
                }
                _ => {}
            }
        }

        Some(BoardCasino {
            casino,
            height,
            tiles,
        })
    }

    pub fn casinos(&self) -> Vec<BoardCasino> {
        let mut visited: HashSet<Loc> = HashSet::new();
        let mut casinos: Vec<BoardCasino> = vec![];
        let mut locs: Vec<Loc> = TILES.keys().cloned().collect();
        locs.sort();
        for loc in &locs {
            if visited.contains(loc) {
                continue;
            }
            if let Some(bc) = self.casino_at(loc) {
                visited.extend(bc.tiles.iter().map(|t| t.loc));
                casinos.push(bc);
            }
        }
        casinos
    }

    pub fn reroll_at(&mut self, loc: &Loc, rng: &mut GameRng) -> Option<usize> {
        let t = self.get(loc);
        match t {
            BoardTile::Built {
                casino,
                owner: Some(TileOwner { player, .. }),
                height,
                ..
            } => {
                let die = roll(rng);
                self.set(
                    *loc,
                    BoardTile::Built {
                        casino,
                        owner: Some(TileOwner { player, die }),
                        height,
                    },
                );
                Some(die)
            }
            _ => None,
        }
    }

    pub fn resolve_boss_ties(&mut self, rng: &mut GameRng) -> Option<Vec<Log>> {
        let mut boss_tie = false;
        let mut logs: Vec<Log> = vec![];

        for bc in self.casinos() {
            let boss_tiles = bc.boss_tiles();
            let bosses: HashSet<usize> = HashSet::from_iter(
                boss_tiles
                    .iter()
                    .filter_map(|bt| bt.owner.map(|to| to.player)),
            );
            if bosses.len() <= 1 {
                // There is no boss tie.
                continue;
            }
            boss_tie = true;
            for bt in &boss_tiles {
                if let Some(die) = self.reroll_at(&bt.loc, rng)
                    && let Some(TileOwner { player, .. }) = bt.owner
                {
                    logs.push(Log::public(vec![
                        N::text("Boss tie at "),
                        bc.casino.render(),
                        N::text(": "),
                        N::Player(player),
                        N::text("'s die at "),
                        bt.loc.render(),
                        N::text(" rerolled to "),
                        N::Bold(vec![N::text(die.to_string())]),
                    ]));
                }
            }
        }

        if boss_tie {
            // Do another pass, we may have created a new boss tie.
            if let Some(new_logs) = self.resolve_boss_ties(rng) {
                logs.extend(new_logs);
            }
            Some(logs)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct CasinoTile {
    pub loc: Loc,
    pub owner: Option<TileOwner>,
}

#[derive(PartialEq, Debug)]
pub struct BoardCasino {
    pub casino: Casino,
    pub height: usize,
    pub tiles: Vec<CasinoTile>,
}

impl BoardCasino {
    pub fn boss_tiles(&self) -> Vec<CasinoTile> {
        let mut highest: usize = 0;
        let mut bosses: Vec<CasinoTile> = vec![];
        for t in &self.tiles {
            if let Some(TileOwner { die, .. }) = t.owner {
                if die > highest {
                    highest = die;
                    bosses = vec![];
                }
                if die == highest {
                    bosses.push(*t);
                }
            }
        }
        bosses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_neighbours<I: Into<Loc>>(l: I, n: Vec<I>) {
        let mut expected = n.into_iter().map(|n| n.into()).collect::<Vec<Loc>>();
        expected.sort();
        let mut actual = l.into().neighbours();
        actual.sort();
        assert_eq!(expected, actual);
    }

    #[test]
    fn loc_neighbours_works() {
        use self::Block::*;

        assert_neighbours((A, 1), vec![(A, 2), (A, 4)]);
        assert_neighbours((A, 2), vec![(A, 1), (A, 3), (A, 5)]);
        assert_neighbours((A, 3), vec![(A, 2), (A, 6)]);
        assert_neighbours((A, 4), vec![(A, 1), (A, 5)]);
        assert_neighbours((A, 5), vec![(A, 2), (A, 4), (A, 6)]);
        assert_neighbours((A, 6), vec![(A, 3), (A, 5)]);
        assert_neighbours((C, 8), vec![(C, 5), (C, 7), (C, 9), (C, 11)]);
    }

    #[test]
    fn loc_parse_str_rejects_out_of_bounds_lot() {
        assert!(Loc::parse_str("a0").is_err());
        assert!(Loc::parse_str("A7").is_err());
    }

    #[test]
    fn test_board_casino_at_works() {
        let mut b = Board::default();
        assert_eq!(None, b.casino_at(&(Block::A, 1).into()));

        b.set((Block::A, 1).into(), BoardTile::Owned { player: 0 });
        assert_eq!(None, b.casino_at(&(Block::A, 1).into()));

        b.set(
            (Block::A, 1).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 3, player: 0 }),
                height: 1,
            },
        );
        assert_eq!(
            Some(BoardCasino {
                casino: Casino::Albion,
                height: 1,
                tiles: vec![CasinoTile {
                    loc: (Block::A, 1).into(),
                    owner: Some(TileOwner { die: 3, player: 0 }),
                },],
            }),
            b.casino_at(&(Block::A, 1).into())
        );
        assert_eq!(
            vec![CasinoTile {
                loc: (Block::A, 1).into(),
                owner: Some(TileOwner { die: 3, player: 0 }),
            },],
            b.casino_at(&(Block::A, 1).into()).unwrap().boss_tiles()
        );

        // Set a diagonal and make sure it doesn't get included.
        b.set(
            (Block::A, 5).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 5, player: 0 }),
                height: 1,
            },
        );
        assert_eq!(
            Some(BoardCasino {
                casino: Casino::Albion,
                height: 1,
                tiles: vec![CasinoTile {
                    loc: (Block::A, 1).into(),
                    owner: Some(TileOwner { die: 3, player: 0 }),
                },],
            }),
            b.casino_at(&(Block::A, 1).into())
        );

        // Join the diagonal in and make sure it is.
        b.set(
            (Block::A, 2).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 2, player: 1 }),
                height: 1,
            },
        );
        assert_eq!(
            Some(BoardCasino {
                casino: Casino::Albion,
                height: 1,
                tiles: vec![
                    CasinoTile {
                        loc: (Block::A, 1).into(),
                        owner: Some(TileOwner { die: 3, player: 0 }),
                    },
                    CasinoTile {
                        loc: (Block::A, 2).into(),
                        owner: Some(TileOwner { die: 2, player: 1 }),
                    },
                    CasinoTile {
                        loc: (Block::A, 5).into(),
                        owner: Some(TileOwner { die: 5, player: 0 }),
                    },
                ],
            }),
            b.casino_at(&(Block::A, 1).into())
        );
        assert_eq!(
            vec![CasinoTile {
                loc: (Block::A, 5).into(),
                owner: Some(TileOwner { die: 5, player: 0 }),
            },],
            b.casino_at(&(Block::A, 1).into()).unwrap().boss_tiles()
        );
    }

    #[test]
    fn test_board_casinos_works() {
        let mut b = Board::default();
        assert_eq!(0, b.casinos().len());

        b.set((Block::A, 1).into(), BoardTile::Owned { player: 0 });
        assert_eq!(0, b.casinos().len());

        b.set(
            (Block::A, 1).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 3, player: 0 }),
                height: 1,
            },
        );
        assert_eq!(1, b.casinos().len());

        // Set a diagonal and make sure it doesn't get included.
        b.set(
            (Block::A, 5).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 5, player: 0 }),
                height: 1,
            },
        );
        assert_eq!(2, b.casinos().len());

        // Join the diagonal in and make sure it is.
        b.set(
            (Block::A, 2).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: Some(TileOwner { die: 2, player: 1 }),
                height: 1,
            },
        );
        assert_eq!(1, b.casinos().len());
    }

    #[test]
    fn resolve_boss_ties_is_deterministic_per_seed() {
        // d F2: identical board + identical seed must produce identical
        // reroll outcomes on every run. Pre-fix, the BFS queue is a fresh
        // HashSet per call whose iteration order varies per instance, so the
        // reroll order (and via re-tie cascades, the draw count) diverges.
        use self::Block::*;

        let locs: Vec<Loc> = vec![(A, 1).into(), (A, 2).into(), (A, 3).into(), (A, 6).into()];
        let dice_at = |b: &Board| -> Vec<Option<usize>> {
            locs.iter()
                .map(|l| match b.get(l) {
                    BoardTile::Built {
                        owner: Some(TileOwner { die, .. }),
                        ..
                    } => Some(die),
                    _ => None,
                })
                .collect()
        };

        let mut outcomes: Vec<Vec<Option<usize>>> = vec![];
        for _ in 0..100 {
            let mut b = Board::default();
            for (i, l) in locs.iter().enumerate() {
                // A1-A2-A3-A6 is one contiguous Albion casino (A3 and A6 are
                // vertical neighbours); four distinct players tied on die 4.
                b.set(
                    *l,
                    BoardTile::Built {
                        casino: Casino::Albion,
                        owner: Some(TileOwner { die: 4, player: i }),
                        height: 1,
                    },
                );
            }
            let mut rng = GameRng::seed_from_u64(7);
            assert!(
                b.resolve_boss_ties(&mut rng).is_some(),
                "a four-way boss tie must trigger resolution"
            );
            outcomes.push(dice_at(&b));
        }
        for o in &outcomes[1..] {
            assert_eq!(
                &outcomes[0], o,
                "same board and seed must produce identical rerolls (d F2)"
            );
        }
    }

    #[test]
    fn resolve_boss_ties_logs_each_reroll() {
        // d F3: rerolls are player-facing (RULES.md) and disable undo, so
        // every rerolled die must appear in the logs.
        let mut b = Board::default();
        for (lot, player) in [(1, 0), (2, 1)] {
            b.set(
                (Block::A, lot).into(),
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: Some(TileOwner { die: 5, player }),
                    height: 1,
                },
            );
        }
        let mut rng = GameRng::seed_from_u64(1);
        let logs = b
            .resolve_boss_ties(&mut rng)
            .expect("a two-way boss tie must trigger resolution");
        assert!(
            logs.len() >= 2,
            "each rerolled die must be logged, got {} logs",
            logs.len()
        );
        let text: String = logs
            .iter()
            .map(|l| brdgme_markup::plain(&brdgme_markup::transform(&l.content, &[])))
            .collect::<Vec<String>>()
            .join("\n");
        assert!(text.contains("rerolled"), "got: {}", text);
        assert!(
            text.contains("A1") && text.contains("A2"),
            "both rerolled locations must be named, got: {}",
            text
        );
    }

    #[test]
    fn casinos_iterates_in_sorted_loc_order() {
        // Lock-in for d F2: casinos() must scan the board in Loc order so the
        // reroll pass is deterministic across processes (TILES is a HashMap
        // whose key order differs per process).
        let mut b = Board::default();
        b.set(
            (Block::F, 9).into(),
            BoardTile::Built {
                casino: Casino::Tivoli,
                owner: None,
                height: 1,
            },
        );
        b.set(
            (Block::A, 1).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: None,
                height: 1,
            },
        );
        let cs = b.casinos();
        assert_eq!(2, cs.len());
        assert_eq!(Casino::Albion, cs[0].casino, "A1 casino must come first");
        assert_eq!(Casino::Tivoli, cs[1].casino);
    }
}
