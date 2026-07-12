//! Port of `loc.go` and the `Board`/`Walk`/`Dir` pieces of `board.go` and
//! `command.go` that operate purely on coordinates and directions.

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

/// Port of `Dir` (`command.go`), a 4-bit direction flag enum.
pub type Dir = u8;

pub const DIR_UP: Dir = 1;
pub const DIR_RIGHT: Dir = 2;
pub const DIR_DOWN: Dir = 4;
pub const DIR_LEFT: Dir = 8;

/// Port of `OrthoDirs` (`command.go`).
pub const ORTHO_DIRS: [Dir; 4] = [DIR_UP, DIR_RIGHT, DIR_DOWN, DIR_LEFT];

/// Port of `DiagDirs` (`command.go`).
pub const DIAG_DIRS: [Dir; 4] = [
    DIR_UP | DIR_RIGHT,
    DIR_DOWN | DIR_RIGHT,
    DIR_DOWN | DIR_LEFT,
    DIR_UP | DIR_LEFT,
];

/// Port of `Dirs` (`command.go`), ortho then diagonal, 8 directions total.
pub fn dirs() -> Vec<Dir> {
    let mut d = ORTHO_DIRS.to_vec();
    d.extend_from_slice(&DIAG_DIRS);
    d
}

/// Port of `OrthoDirNames` (`command.go`).
pub fn ortho_dir_name(dir: Dir) -> &'static str {
    match dir {
        DIR_UP => "up",
        DIR_RIGHT => "right",
        DIR_DOWN => "down",
        DIR_LEFT => "left",
        _ => panic!("not an ortho dir: {}", dir),
    }
}

/// Port of `DirInv` (`game.go`).
pub fn dir_inv(dir: Dir) -> Dir {
    let mut inv: Dir = 0;
    if dir & DIR_UP > 0 {
        inv |= DIR_DOWN;
    }
    if dir & DIR_RIGHT > 0 {
        inv |= DIR_LEFT;
    }
    if dir & DIR_DOWN > 0 {
        inv |= DIR_UP;
    }
    if dir & DIR_LEFT > 0 {
        inv |= DIR_RIGHT;
    }
    inv
}

/// Port of `Loc` (`loc.go`). `x`/`y` are `i32` (not `usize`) because piece
/// position offsets can be negative before being translated onto the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Loc {
    pub x: i32,
    pub y: i32,
}

impl Loc {
    pub fn new(x: i32, y: i32) -> Self {
        Loc { x, y }
    }

    // Named `add`/`neg`/`sub` to mirror Go's `Loc.Add`/`Loc.Neg`/`Loc.Sub`
    // directly rather than implementing `std::ops::{Add,Neg,Sub}`.
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Loc) -> Loc {
        Loc::new(self.x + other.x, self.y + other.y)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn neg(self) -> Loc {
        Loc::new(-self.x, -self.y)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Loc) -> Loc {
        self.add(other.neg())
    }

    /// Port of `Loc.Neighbour`.
    pub fn neighbour(self, dir: Dir) -> Loc {
        self.add(unit_loc(dir))
    }

    /// Port of `Loc.Rotate`: a 90-degree rotation applied `n` times.
    pub fn rotate(self, n: i32) -> Loc {
        match n.cmp(&0) {
            std::cmp::Ordering::Greater => Loc::new(-self.y, self.x).rotate(n - 1),
            std::cmp::Ordering::Less => Loc::new(self.y, -self.x).rotate(n + 1),
            std::cmp::Ordering::Equal => self,
        }
    }

    /// Port of `Loc.Valid`.
    pub fn valid(self) -> bool {
        (0..=9).contains(&self.x) && (0..=9).contains(&self.y)
    }

    /// Port of `Loc.String`: row letter 'A'-'J' (Y), column number 1-10 (X).
    pub fn to_key(self) -> String {
        format!("{}{}", (b'A' + self.y as u8) as char, self.x + 1)
    }
}

impl std::fmt::Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_key())
    }
}

/// Port of `Locs.Rotate`.
pub fn rotate_locs(locs: &[Loc], n: i32) -> Vec<Loc> {
    locs.iter().map(|&l| l.rotate(n)).collect()
}

/// Port of `UnitLoc`.
pub fn unit_loc(dir: Dir) -> Loc {
    let mut l = Loc::new(0, 0);
    if dir & DIR_UP == DIR_UP {
        l.y -= 1;
    }
    if dir & DIR_RIGHT == DIR_RIGHT {
        l.x += 1;
    }
    if dir & DIR_DOWN == DIR_DOWN {
        l.y += 1;
    }
    if dir & DIR_LEFT == DIR_LEFT {
        l.x -= 1;
    }
    l
}

/// Port of the `AllLocs` package var (`board.go`): every board cell in
/// row-major order.
pub fn all_locs() -> Vec<Loc> {
    let mut locs = Vec::with_capacity(100);
    for y in 0..10 {
        for x in 0..10 {
            locs.push(Loc::new(x, y));
        }
    }
    locs
}

/// Port of the `LocsByRow` package var (`board.go`).
pub fn locs_by_row() -> Vec<Vec<Loc>> {
    (0..10)
        .map(|y| (0..10).map(|x| Loc::new(x, y)).collect())
        .collect()
}

/// Port of `ParseLoc` (`play_command.go`): case-insensitive `^[a-j]\d+$`.
pub fn parse_loc(input: &str) -> Option<Loc> {
    let mut chars = input.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let rest: String = chars.collect();
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let row = first.to_ascii_uppercase() as i32 - 'A' as i32;
    let col: i32 = rest.parse().ok()?;
    let loc = Loc::new(col - 1, row);
    if loc.valid() { Some(loc) } else { None }
}

pub const WALK_CONTINUE: i32 = 0;
pub const WALK_BLOCKED: i32 = 1;
pub const WALK_FINISH: i32 = 2;

/// Port of `Walk` (`board.go`): a BFS visiting `from` then neighbours in
/// `dirs`, guided by the callback's `WalkContinue`/`WalkBlocked`/`WalkFinish`
/// result.
pub fn walk<F: FnMut(Loc) -> i32>(from: Loc, dirs: &[Dir], mut cb: F) {
    let mut visited: HashSet<Loc> = HashSet::new();
    let mut queued: HashSet<Loc> = HashSet::new();
    queued.insert(from);
    let mut queue: VecDeque<Loc> = VecDeque::new();
    queue.push_back(from);
    while let Some(current) = queue.pop_front() {
        visited.insert(current);
        match cb(current) {
            WALK_FINISH => return,
            WALK_CONTINUE => {
                for &dir in dirs {
                    let next_loc = current.neighbour(dir);
                    // Note: Go's `queued` map is only ever populated with the
                    // starting `from` location and is never updated as nodes
                    // are queued, so duplicate queue entries for the same
                    // `Loc` are possible (and thus the callback can be
                    // invoked more than once for the same `Loc`). Preserved
                    // verbatim - callers guard against double-processing
                    // themselves via their own `visited` maps.
                    if !queued.contains(&next_loc)
                        && !visited.contains(&next_loc)
                        && next_loc.valid()
                    {
                        queue.push_back(next_loc);
                    }
                }
            }
            _ => {}
        }
    }
}
