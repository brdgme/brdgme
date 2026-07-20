use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    Blue,
    Green,
    Red,
    Yellow,
}

impl Currency {
    pub const ALL: [Currency; 4] = [
        Currency::Blue,
        Currency::Green,
        Currency::Red,
        Currency::Yellow,
    ];

    pub fn abbr(&self) -> &'static str {
        match self {
            Currency::Blue => "B",
            Currency::Green => "G",
            Currency::Red => "R",
            Currency::Yellow => "Y",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Currency::Blue => "blue",
            Currency::Green => "green",
            Currency::Red => "red",
            Currency::Yellow => "yellow",
        }
    }

    pub fn from_abbr(s: &str) -> Option<Currency> {
        match s.to_uppercase().as_str() {
            "B" => Some(Currency::Blue),
            "G" => Some(Currency::Green),
            "R" => Some(Currency::Red),
            "Y" => Some(Currency::Yellow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub currency: Currency,
    pub value: i32,
}

impl Card {
    pub fn new(currency: Currency, value: i32) -> Self {
        Card { currency, value }
    }

    pub fn parse(input: &str) -> Option<Card> {
        let input = input.trim();
        if input.len() < 2 {
            return None;
        }
        let (letter, rest) = input.split_at(1);
        let currency = Currency::from_abbr(letter)?;
        let value: i32 = rest.parse().ok()?;
        if value < 1 {
            return None;
        }
        Some(Card { currency, value })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.currency.abbr(), self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckCard {
    Money(Card),
    Scoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileType {
    Empty,
    Fountain,
    Pavillion,
    Seraglio,
    Arcades,
    Chambers,
    Garden,
    Tower,
}

impl TileType {
    pub fn abbr(&self) -> &'static str {
        match self {
            TileType::Empty => "   ",
            TileType::Fountain => " F ",
            TileType::Pavillion => "Pav",
            TileType::Seraglio => "Ser",
            TileType::Arcades => "Arc",
            TileType::Chambers => "Cha",
            TileType::Garden => "Gar",
            TileType::Tower => "Tow",
        }
    }

    pub fn is_scoring(&self) -> bool {
        matches!(
            self,
            TileType::Pavillion
                | TileType::Seraglio
                | TileType::Arcades
                | TileType::Chambers
                | TileType::Garden
                | TileType::Tower
        )
    }
}

pub const SCORING_TILE_TYPES: [TileType; 6] = [
    TileType::Pavillion,
    TileType::Seraglio,
    TileType::Arcades,
    TileType::Chambers,
    TileType::Garden,
    TileType::Tower,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Right, Dir::Down, Dir::Left];

    pub fn inverse(&self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Right => Dir::Left,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
        }
    }

    pub fn vect(&self) -> Vect {
        match self {
            Dir::Up => Vect { x: 0, y: -1 },
            Dir::Right => Vect { x: 1, y: 0 },
            Dir::Down => Vect { x: 0, y: 1 },
            Dir::Left => Vect { x: -1, y: 0 },
        }
    }

    pub fn rot(&self, n: i32) -> Dir {
        let idx = match self {
            Dir::Up => 0,
            Dir::Right => 1,
            Dir::Down => 2,
            Dir::Left => 3,
        };
        let new_idx = ((idx + n).rem_euclid(4)) as usize;
        Dir::ALL[new_idx]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vect {
    pub x: i32,
    pub y: i32,
}

impl Vect {
    pub fn add(&self, other: Vect) -> Vect {
        Vect {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    pub fn rot_all(&self, n: i32) -> Vect {
        const DIRS_ALL: [Vect; 8] = [
            Vect { x: 0, y: -1 },
            Vect { x: 1, y: -1 },
            Vect { x: 1, y: 0 },
            Vect { x: 1, y: 1 },
            Vect { x: 0, y: 1 },
            Vect { x: -1, y: 1 },
            Vect { x: -1, y: 0 },
            Vect { x: -1, y: -1 },
        ];
        for (i, d) in DIRS_ALL.iter().enumerate() {
            if self == d {
                let ni = (i as i32 + n).rem_euclid(8) as usize;
                return DIRS_ALL[ni];
            }
        }
        panic!("Can only call rot_all on unit vector")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub tile_type: TileType,
    pub cost: i32,
    pub walls: HashMap<Dir, bool>,
}

impl Tile {
    pub fn new(tile_type: TileType, cost: i32, walls: &[Dir]) -> Self {
        let mut wall_map = HashMap::new();
        for w in walls {
            wall_map.insert(*w, true);
        }
        Tile {
            tile_type,
            cost,
            walls: wall_map,
        }
    }

    pub fn empty() -> Self {
        Tile {
            tile_type: TileType::Empty,
            cost: 0,
            walls: HashMap::new(),
        }
    }

    pub fn has_wall(&self, dir: Dir) -> bool {
        self.walls.get(&dir).copied().unwrap_or(false)
    }
}

pub fn all_tiles() -> Vec<Tile> {
    use Dir::*;
    use TileType::*;
    vec![
        Tile::new(Pavillion, 6, &[Up]),
        Tile::new(Pavillion, 4, &[Right, Down]),
        Tile::new(Pavillion, 8, &[]),
        Tile::new(Pavillion, 3, &[Down, Left]),
        Tile::new(Pavillion, 5, &[Up, Left]),
        Tile::new(Pavillion, 2, &[Up, Right, Left]),
        Tile::new(Pavillion, 7, &[Right]),
        Tile::new(Seraglio, 5, &[Down, Left]),
        Tile::new(Seraglio, 6, &[Right, Down]),
        Tile::new(Seraglio, 7, &[Left]),
        Tile::new(Seraglio, 8, &[Down]),
        Tile::new(Seraglio, 3, &[Right, Down, Left]),
        Tile::new(Seraglio, 4, &[Up, Right]),
        Tile::new(Seraglio, 9, &[]),
        Tile::new(Arcades, 9, &[]),
        Tile::new(Arcades, 4, &[Up, Right, Down]),
        Tile::new(Arcades, 10, &[]),
        Tile::new(Arcades, 8, &[Up]),
        Tile::new(Arcades, 7, &[Right, Down]),
        Tile::new(Arcades, 6, &[Down, Left]),
        Tile::new(Arcades, 6, &[Up, Right]),
        Tile::new(Arcades, 5, &[Up, Left]),
        Tile::new(Arcades, 8, &[Right]),
        Tile::new(Chambers, 6, &[Right, Down]),
        Tile::new(Chambers, 7, &[Down, Left]),
        Tile::new(Chambers, 10, &[]),
        Tile::new(Chambers, 5, &[Up, Down, Left]),
        Tile::new(Chambers, 7, &[Up, Right]),
        Tile::new(Chambers, 9, &[Left]),
        Tile::new(Chambers, 8, &[Up, Left]),
        Tile::new(Chambers, 11, &[]),
        Tile::new(Chambers, 9, &[Down]),
        Tile::new(Garden, 9, &[Right]),
        Tile::new(Garden, 7, &[Up, Down, Left]),
        Tile::new(Garden, 8, &[Up, Right]),
        Tile::new(Garden, 11, &[]),
        Tile::new(Garden, 8, &[Down, Left]),
        Tile::new(Garden, 10, &[Up]),
        Tile::new(Garden, 6, &[Right, Down, Left]),
        Tile::new(Garden, 8, &[Up, Left]),
        Tile::new(Garden, 10, &[]),
        Tile::new(Garden, 12, &[Down]),
        Tile::new(Garden, 10, &[Left]),
        Tile::new(Tower, 8, &[Up, Right, Down]),
        Tile::new(Tower, 9, &[Up, Left]),
        Tile::new(Tower, 13, &[Right]),
        Tile::new(Tower, 9, &[Right, Down]),
        Tile::new(Tower, 7, &[Up, Right, Left]),
        Tile::new(Tower, 11, &[Up]),
        Tile::new(Tower, 9, &[Up, Right]),
        Tile::new(Tower, 11, &[Down]),
        Tile::new(Tower, 12, &[]),
        Tile::new(Tower, 11, &[]),
        Tile::new(Tower, 10, &[Left]),
    ]
}

pub type Grid = HashMap<Vect, Tile>;

pub mod grid_serde {
    use super::{Grid, Tile, Vect};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(grid: &Grid, serializer: S) -> Result<S::Ok, S::Error> {
        let entries: Vec<(&Vect, &Tile)> = grid.iter().collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Grid, D::Error> {
        let entries: Vec<(Vect, Tile)> = Vec::deserialize(deserializer)?;
        Ok(entries.into_iter().collect())
    }
}

pub fn new_grid() -> Grid {
    let mut g = HashMap::new();
    g.insert(Vect { x: 0, y: 0 }, Tile::new(TileType::Fountain, 0, &[]));
    g
}

pub fn grid_tile_at(g: &Grid, v: Vect) -> Tile {
    g.get(&v).cloned().unwrap_or_else(Tile::empty)
}

pub fn grid_fountain_loc(g: &Grid) -> Option<Vect> {
    g.iter()
        .find(|(_, t)| t.tile_type == TileType::Fountain)
        .map(|(v, _)| *v)
}

pub fn grid_bounds(g: &Grid) -> (Vect, Vect) {
    let mut min = Vect {
        x: i32::MAX,
        y: i32::MAX,
    };
    let mut max = Vect {
        x: i32::MIN,
        y: i32::MIN,
    };
    for v in g.keys() {
        if v.x < min.x {
            min.x = v.x;
        }
        if v.y < min.y {
            min.y = v.y;
        }
        if v.x > max.x {
            max.x = v.x;
        }
        if v.y > max.y {
            max.y = v.y;
        }
    }
    (min, max)
}

pub const GRID_INVALID_NO_FOUNTAIN: &str = "must not be missing the fountain tile";
pub const GRID_INVALID_WALL: &str =
    "adjoining tile sides must match, either both walls or both not walls";
pub const GRID_INVALID_CANNOT_WALK: &str =
    "must be able to walk from the fountain to all other tiles";
pub const GRID_INVALID_GAP: &str = "not allowed to create empty gaps";

pub fn grid_is_valid(g: &Grid) -> (bool, String) {
    let fv = match grid_fountain_loc(g) {
        Some(v) => v,
        None => return (false, GRID_INVALID_NO_FOUNTAIN.to_string()),
    };

    let mut walk_stack = vec![fv];
    let mut in_walk_stack: HashMap<Vect, bool> = HashMap::new();
    let mut connected: HashMap<Vect, bool> = HashMap::new();

    while let Some(next) = walk_stack.first().copied() {
        walk_stack.remove(0);
        let next_tile = grid_tile_at(g, next);
        in_walk_stack.insert(next, false);
        connected.insert(next, true);

        for dir in Dir::ALL {
            let dv = next.add(dir.vect());
            let dv_tile = grid_tile_at(g, dv);

            if dv_tile.tile_type == TileType::Empty {
                continue;
            }

            if next_tile.has_wall(dir) {
                if !dv_tile.has_wall(dir.inverse()) {
                    return (false, GRID_INVALID_WALL.to_string());
                }
                continue;
            }

            if in_walk_stack.contains_key(&dv) || connected.contains_key(&dv) {
                continue;
            }

            walk_stack.push(dv);
            in_walk_stack.insert(dv, true);
        }
    }

    for (v, t) in g.iter() {
        if t.tile_type != TileType::Empty && !connected.contains_key(v) {
            return (false, GRID_INVALID_CANNOT_WALK.to_string());
        }
    }

    let (min, max) = grid_bounds(g);
    let start = min.add(Vect { x: -1, y: -1 });
    let mut walk_stack = vec![start];
    let mut in_walk_stack: HashMap<Vect, bool> = HashMap::new();
    let mut connected: HashMap<Vect, bool> = HashMap::new();

    while let Some(next) = walk_stack.first().copied() {
        walk_stack.remove(0);
        in_walk_stack.insert(next, false);
        connected.insert(next, true);

        for dir in Dir::ALL {
            let dv = next.add(dir.vect());
            let dv_tile = grid_tile_at(g, dv);

            if dv_tile.tile_type != TileType::Empty
                || in_walk_stack.contains_key(&dv)
                || connected.contains_key(&dv)
                || dv.x < min.x - 1
                || dv.x > max.x + 1
                || dv.y < min.y - 1
                || dv.y > max.y + 1
            {
                continue;
            }

            walk_stack.push(dv);
            in_walk_stack.insert(dv, true);
        }
    }

    for x in min.x..=max.x {
        for y in min.y..max.y {
            let v = Vect { x, y };
            if grid_tile_at(g, v).tile_type == TileType::Empty && !connected.contains_key(&v) {
                return (false, GRID_INVALID_GAP.to_string());
            }
        }
    }

    (true, String::new())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VectDir {
    pub vect: Vect,
    pub dir: Dir,
}

pub fn grid_is_wall(g: &Grid, vd: VectDir) -> bool {
    grid_tile_at(g, vd.vect).has_wall(vd.dir)
}

pub fn grid_is_internal_wall(g: &Grid, vd: VectDir) -> bool {
    let adjacent = vd.vect.add(vd.dir.vect());
    grid_tile_at(g, adjacent).has_wall(vd.dir.inverse())
}

pub fn grid_longest_ext_wall(g: &Grid) -> i32 {
    let mut visited: HashMap<VectDir, bool> = HashMap::new();
    let mut longest = 0;

    for (v, t) in g.iter() {
        for d in Dir::ALL {
            if !t.has_wall(d) {
                continue;
            }
            let vd = VectDir { vect: *v, dir: d };
            if visited.contains_key(&vd) || grid_is_internal_wall(g, vd) {
                continue;
            }

            visited.insert(vd, true);
            let mut wall = 1;

            for rot_dir in [1i32, -1i32] {
                let mut cur = vd;
                loop {
                    let pivot = cur.vect.add(cur.dir.vect());
                    let mut found = false;
                    for rot_num in 0..3i32 {
                        let next_wall = VectDir {
                            vect: pivot.add(cur.dir.vect().rot_all((rot_num + 2) * rot_dir)),
                            dir: cur.dir.rot((rot_num - 1) * rot_dir),
                        };
                        if grid_tile_at(g, next_wall.vect).tile_type == TileType::Empty {
                            continue;
                        }
                        if !visited.contains_key(&next_wall)
                            && grid_is_wall(g, next_wall)
                            && !grid_is_internal_wall(g, next_wall)
                        {
                            wall += 1;
                            visited.insert(next_wall, true);
                            found = true;
                            cur = next_wall;
                        }
                        break;
                    }
                    if !found {
                        break;
                    }
                }
            }

            if wall > longest {
                longest = wall;
            }
        }
    }

    longest
}

pub fn grid_parse_coord(g: &Grid, input: &str) -> Result<Vect, String> {
    let input = input.trim();
    if let Some(v) = parse_coord_alpha_num(g, input) {
        return Ok(v);
    }
    if let Some(v) = parse_coord_num_alpha(g, input) {
        return Ok(v);
    }
    Err("coord must be numbers and letters, like a4 or 4a".to_string())
}

fn parse_coord_alpha_num(g: &Grid, input: &str) -> Option<Vect> {
    let input = input.to_lowercase();
    let letter = input.chars().next()?;
    if !letter.is_ascii_lowercase() {
        return None;
    }
    let num_str = &input[1..];
    let n: i32 = num_str.parse().ok()?;
    let (min, _) = grid_bounds(g);
    Some(Vect {
        x: (letter as u8 - b'a') as i32 + min.x - 1,
        y: n + min.y - 2,
    })
}

fn parse_coord_num_alpha(g: &Grid, input: &str) -> Option<Vect> {
    let input = input.to_lowercase();
    let letter = input.chars().last()?;
    if !letter.is_ascii_lowercase() {
        return None;
    }
    let num_str = &input[..input.len() - 1];
    let n: i32 = num_str.parse().ok()?;
    let (min, _) = grid_bounds(g);
    Some(Vect {
        x: (letter as u8 - b'a') as i32 + min.x - 1,
        y: n + min.y - 2,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBoard {
    #[serde(with = "grid_serde")]
    pub grid: Grid,
    pub reserve: Vec<Tile>,
    pub cards: Vec<Card>,
    pub place: Vec<Tile>,
    pub points: i32,
}

impl Default for PlayerBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerBoard {
    pub fn new() -> Self {
        PlayerBoard {
            grid: new_grid(),
            reserve: vec![],
            cards: vec![],
            place: vec![],
            points: 0,
        }
    }

    pub fn tile_counts(&self) -> HashMap<TileType, i32> {
        let mut counts = HashMap::new();
        for t in self.grid.values() {
            if t.tile_type != TileType::Empty {
                *counts.entry(t.tile_type).or_insert(0) += 1;
            }
        }
        counts
    }

    pub fn currency_value(&self, currency: Currency) -> i32 {
        self.cards
            .iter()
            .filter(|c| c.currency == currency)
            .map(|c| c.value)
            .sum()
    }
}

pub fn build_deck(players: usize) -> Vec<DeckCard> {
    let n = if players == 2 { 2 } else { 3 };
    let mut deck = vec![];
    for c in Currency::ALL {
        for v in 1..=9 {
            for _ in 0..n {
                deck.push(DeckCard::Money(Card::new(c, v)));
            }
        }
    }
    deck
}

pub fn round_scores(tile_type: TileType) -> [i32; 3] {
    match tile_type {
        TileType::Pavillion => [1, 8, 16],
        TileType::Seraglio => [2, 9, 17],
        TileType::Arcades => [3, 10, 18],
        TileType::Chambers => [4, 11, 19],
        TileType::Garden => [5, 12, 20],
        TileType::Tower => [6, 13, 21],
        _ => [0, 0, 0],
    }
}

pub fn not_empty(tiles: &[Tile]) -> Vec<Tile> {
    tiles
        .iter()
        .filter(|t| t.tile_type != TileType::Empty)
        .cloned()
        .collect()
}
