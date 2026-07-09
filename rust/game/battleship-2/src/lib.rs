use std::fmt;

use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use command::Command;

const BOARD_SIZE: usize = 10;
const NUM_PLAYERS: usize = 2;

pub type Board = [[Cell; BOARD_SIZE]; BOARD_SIZE];

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cell {
    #[default]
    Empty,
    Carrier,
    Battleship,
    Cruiser,
    Submarine,
    Destroyer,
    Hit,
    Miss,
}

impl Cell {
    fn is_ship(self) -> bool {
        self.to_ship().is_some()
    }

    fn to_ship(self) -> Option<Ship> {
        match self {
            Cell::Carrier => Some(Ship::Carrier),
            Cell::Battleship => Some(Ship::Battleship),
            Cell::Cruiser => Some(Ship::Cruiser),
            Cell::Submarine => Some(Ship::Submarine),
            Cell::Destroyer => Some(Ship::Destroyer),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ship {
    Carrier,
    Battleship,
    Cruiser,
    Submarine,
    Destroyer,
}

impl Ship {
    pub fn all() -> &'static [Ship] {
        &[
            Ship::Carrier,
            Ship::Battleship,
            Ship::Cruiser,
            Ship::Submarine,
            Ship::Destroyer,
        ]
    }

    pub fn size(self) -> usize {
        match self {
            Ship::Carrier => 5,
            Ship::Battleship => 4,
            Ship::Cruiser => 3,
            Ship::Submarine => 3,
            Ship::Destroyer => 2,
        }
    }

    pub fn to_cell(self) -> Cell {
        match self {
            Ship::Carrier => Cell::Carrier,
            Ship::Battleship => Cell::Battleship,
            Ship::Cruiser => Cell::Cruiser,
            Ship::Submarine => Cell::Submarine,
            Ship::Destroyer => Cell::Destroyer,
        }
    }
}

impl fmt::Display for Ship {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Ship::Carrier => "carrier",
            Ship::Battleship => "battleship",
            Ship::Cruiser => "cruiser",
            Ship::Submarine => "submarine",
            Ship::Destroyer => "destroyer",
        };
        write!(f, "{}", s)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    pub fn all() -> Vec<Direction> {
        vec![
            Direction::Up,
            Direction::Right,
            Direction::Down,
            Direction::Left,
        ]
    }

    pub fn modifiers(self) -> (i32, i32) {
        match self {
            Direction::Up => (-1, 0),
            Direction::Right => (0, 1),
            Direction::Down => (1, 0),
            Direction::Left => (0, -1),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Direction::Up => "up",
            Direction::Right => "right",
            Direction::Down => "down",
            Direction::Left => "left",
        };
        write!(f, "{}", s)
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    #[default]
    Placing,
    Shooting,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Loc {
    pub y: usize,
    pub x: usize,
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", (b'A' + self.y as u8) as char, self.x + 1)
    }
}

pub fn all_locations() -> Vec<Loc> {
    let mut locs = vec![];
    for y in 0..BOARD_SIZE {
        for x in 0..BOARD_SIZE {
            locs.push(Loc { y, x });
        }
    }
    locs
}

fn is_valid_location(y: i32, x: i32) -> bool {
    (0..BOARD_SIZE as i32).contains(&y) && (0..BOARD_SIZE as i32).contains(&x)
}

fn locations_in_direction(y: i32, x: i32, dir: Direction, dist: i32) -> Vec<(i32, i32)> {
    let (y_mod, x_mod) = dir.modifiers();
    (0..=dist).map(|i| (y + i * y_mod, x + i * x_mod)).collect()
}

fn other_player(p: usize) -> usize {
    (p + 1) % NUM_PLAYERS
}

fn redact_board(board: &Board) -> Board {
    let mut redacted = *board;
    for row in redacted.iter_mut() {
        for cell in row.iter_mut() {
            if cell.is_ship() {
                *cell = Cell::Empty;
            }
        }
    }
    redacted
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub phase: Phase,
    pub boards: Vec<Board>,
    pub left_to_place: Vec<Vec<Ship>>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub phase: Phase,
    pub current_player: usize,
    pub boards: Vec<Board>,
    pub left_to_place_counts: Vec<usize>,
    pub finished: bool,
    pub placings: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub board: Board,
    pub left_to_place: Vec<Ship>,
}

impl Game {
    pub fn can_place(&self, player: usize) -> bool {
        !self.is_finished()
            && self.phase == Phase::Placing
            && self
                .left_to_place
                .get(player)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
    }

    pub fn can_shoot(&self, player: usize) -> bool {
        !self.is_finished() && self.phase == Phase::Shooting && self.current_player == player
    }

    pub fn place_ship(
        &mut self,
        player: usize,
        ship: Ship,
        loc: Loc,
        dir: Direction,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_place(player) {
            return Err(GameError::invalid_input(
                "you are not allowed to place a ship at the moment",
            ));
        }
        let found_at = self.left_to_place[player]
            .iter()
            .position(|&s| s == ship)
            .ok_or_else(|| {
                GameError::invalid_input("you don't have any of that type of ship to place")
            })?;
        let locs = locations_in_direction(loc.y as i32, loc.x as i32, dir, ship.size() as i32 - 1);
        for (y, x) in &locs {
            if !is_valid_location(*y, *x) {
                return Err(GameError::invalid_input(
                    "can't place there because it would go off the board",
                ));
            }
            if self.boards[player][*y as usize][*x as usize] != Cell::Empty {
                return Err(GameError::invalid_input(
                    "can't place there because there's a ship in the way",
                ));
            }
        }
        for (y, x) in &locs {
            self.boards[player][*y as usize][*x as usize] = ship.to_cell();
        }
        let mut logs = vec![];
        if self.left_to_place[player].len() == 1 {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" finished placing their ships"),
            ]));
            self.left_to_place[player] = vec![];
            if self.left_to_place[other_player(player)].is_empty() {
                self.phase = Phase::Shooting;
            }
        } else {
            self.left_to_place[player].remove(found_at);
        }
        Ok(logs)
    }

    pub fn shoot(&mut self, player: usize, loc: Loc) -> Result<Vec<Log>, GameError> {
        if !self.can_shoot(player) {
            return Err(GameError::invalid_input(
                "you are not allowed to shoot at the moment",
            ));
        }
        let Loc { y, x } = loc;
        let op = other_player(player);
        let mut logs = vec![];
        match self.boards[op][y][x] {
            Cell::Hit | Cell::Miss => {
                return Err(GameError::invalid_input(
                    "you have already shot there previously",
                ));
            }
            Cell::Empty => {
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(format!(" shot at {} and missed", loc)),
                ]));
                self.boards[op][y][x] = Cell::Miss;
            }
            ship_cell => {
                let ship = ship_cell.to_ship().expect("cell is a ship");
                self.boards[op][y][x] = Cell::Hit;
                if self.player_ship_hits_remaining(op, ship) == 0 {
                    logs.push(Log::public(vec![N::Bold(vec![
                        N::Player(player),
                        N::text(format!(" shot at {} and sunk a {}!", loc, ship)),
                    ])]));
                } else {
                    logs.push(Log::public(vec![
                        N::Player(player),
                        N::text(format!(" shot at {} and hit a ship", loc)),
                    ]));
                }
            }
        }
        self.current_player = other_player(self.current_player);
        Ok(logs)
    }

    pub fn player_hits_remaining(&self, player: usize) -> i32 {
        let mut remaining = 0;
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if self.boards[player][y][x].is_ship() {
                    remaining += 1;
                }
            }
        }
        remaining
    }

    pub fn player_ship_hits_remaining(&self, player: usize, ship: Ship) -> i32 {
        let mut remaining = 0;
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                if self.boards[player][y][x] == ship.to_cell() {
                    remaining += 1;
                }
            }
        }
        remaining
    }

    pub fn is_finished(&self) -> bool {
        if self.phase == Phase::Placing {
            return false;
        }
        for p in 0..self.players {
            if self.player_hits_remaining(p) == 0 {
                return true;
            }
        }
        false
    }

    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_hits_remaining(p)])
            .collect();
        gen_placings(&metrics)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, _seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != NUM_PLAYERS {
            return Err(GameError::PlayerCount {
                min: NUM_PLAYERS,
                max: NUM_PLAYERS,
                given: players,
            });
        }
        let g = Game {
            players,
            current_player: 0,
            phase: Phase::Placing,
            boards: vec![[[Cell::Empty; BOARD_SIZE]; BOARD_SIZE]; NUM_PLAYERS],
            left_to_place: vec![Ship::all().to_vec(), Ship::all().to_vec()],
        };
        Ok((g, vec![]))
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else if self.phase == Phase::Placing {
            let whose: Vec<usize> = (0..self.players)
                .filter(|&p| !self.left_to_place[p].is_empty())
                .collect();
            Status::Active {
                whose_turn: whose,
                eliminated: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        let finished = self.is_finished();
        let boards = if finished {
            self.boards.clone()
        } else {
            self.boards.iter().map(redact_board).collect()
        };
        PubState {
            players: self.players,
            phase: self.phase,
            current_player: self.current_player,
            boards,
            left_to_place_counts: self.left_to_place.iter().map(|v| v.len()).collect(),
            finished,
            placings: if finished { self.placings() } else { vec![] },
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            board: self.boards.get(player).copied().unwrap_or_default(),
            left_to_place: self.left_to_place.get(player).cloned().unwrap_or_default(),
        }
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = match self.command_parser(player) {
            Some(cp) => cp,
            None => {
                return Err(GameError::invalid_input(
                    "not expecting any commands at the moment",
                ));
            }
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                remaining,
                value: Command::Place { ship, loc, dir },
                ..
            }) => {
                let logs = self.place_ship(player, ship, loc, dir)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Shoot { loc },
                ..
            }) => {
                let logs = self.shoot(player, loc)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.players)
            .map(|p| self.player_hits_remaining(p) as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![NUM_PLAYERS]
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const MICK: usize = 0;
    const STEVE: usize = 1;

    fn players() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string()]
    }

    fn mock_game() -> Game {
        Game::start(2, 1).unwrap().0
    }

    fn place_all(g: &mut Game, player: usize) {
        let cmds = [
            "place sub b3 right",
            "place car c3 right",
            "place des d3 right",
            "place cru e3 right",
            "place bat f3 right",
        ];
        let p = players();
        for cmd in &cmds {
            g.command(player, cmd, &p).unwrap();
        }
    }

    #[test]
    fn test_game() {
        let mut g = mock_game();
        assert_eq!(2, g.whose_turn().len());
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        g.command(MICK, "place car c3 right", &p).unwrap();
        g.command(MICK, "place des d3 right", &p).unwrap();
        g.command(MICK, "place cru e3 right", &p).unwrap();
        g.command(MICK, "place bat f3 right", &p).unwrap();
        g.command(STEVE, "place sub b3 right", &p).unwrap();
        g.command(STEVE, "place car c3 right", &p).unwrap();
        g.command(STEVE, "place des d3 right", &p).unwrap();
        g.command(STEVE, "place cru e3 right", &p).unwrap();
        g.command(STEVE, "place bat f3 right", &p).unwrap();
        g.command(MICK, "shoot b3", &p).unwrap();
    }

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2], Game::player_counts());
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(3, 1).is_err());
        assert!(Game::start(2, 1).is_ok());
    }

    #[test]
    fn test_start_initial_state() {
        let g = mock_game();
        assert_eq!(2, g.players);
        assert_eq!(Phase::Placing, g.phase);
        assert_eq!(0, g.current_player);
        for p in 0..2 {
            assert_eq!(5, g.left_to_place[p].len());
            assert_eq!(
                vec![
                    Ship::Carrier,
                    Ship::Battleship,
                    Ship::Cruiser,
                    Ship::Submarine,
                    Ship::Destroyer
                ],
                g.left_to_place[p]
            );
            for y in 0..BOARD_SIZE {
                for x in 0..BOARD_SIZE {
                    assert_eq!(Cell::Empty, g.boards[p][y][x]);
                }
            }
        }
        assert_eq!(2, g.whose_turn().len());
        assert!(!g.is_finished());
    }

    #[test]
    fn test_ship_sizes() {
        assert_eq!(5, Ship::Carrier.size());
        assert_eq!(4, Ship::Battleship.size());
        assert_eq!(3, Ship::Cruiser.size());
        assert_eq!(3, Ship::Submarine.size());
        assert_eq!(2, Ship::Destroyer.size());
    }

    #[test]
    fn test_loc_display() {
        assert_eq!("A1", Loc { y: 0, x: 0 }.to_string());
        assert_eq!("B3", Loc { y: 1, x: 2 }.to_string());
        assert_eq!("J10", Loc { y: 9, x: 9 }.to_string());
    }

    #[test]
    fn test_can_place_and_can_shoot() {
        let mut g = mock_game();
        assert!(g.can_place(MICK));
        assert!(g.can_place(STEVE));
        assert!(!g.can_shoot(MICK));
        assert!(!g.can_shoot(STEVE));
        place_all(&mut g, MICK);
        assert!(!g.can_place(MICK));
        assert!(g.can_place(STEVE));
        assert!(!g.can_shoot(MICK));
        place_all(&mut g, STEVE);
        assert!(!g.can_place(MICK));
        assert!(!g.can_place(STEVE));
        assert!(g.can_shoot(MICK));
        assert!(!g.can_shoot(STEVE));
    }

    #[test]
    fn test_place_removes_ship_from_left_to_place() {
        let mut g = mock_game();
        let p = players();
        assert_eq!(5, g.left_to_place[MICK].len());
        g.command(MICK, "place sub b3 right", &p).unwrap();
        assert_eq!(4, g.left_to_place[MICK].len());
        assert!(!g.left_to_place[MICK].contains(&Ship::Submarine));
    }

    #[test]
    fn test_place_marks_board() {
        let mut g = mock_game();
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        assert_eq!(Cell::Submarine, g.boards[MICK][1][2]);
        assert_eq!(Cell::Submarine, g.boards[MICK][1][3]);
    }

    #[test]
    fn test_place_logs_finished() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        let resp = g.command(MICK, "", &p);
        assert!(resp.is_err());
        place_all(&mut g, STEVE);
        let resp = g.command(STEVE, "", &p);
        assert!(resp.is_err());
        assert_eq!(Phase::Shooting, g.phase);
    }

    #[test]
    fn test_place_finished_log_message() {
        let mut g = mock_game();
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        g.command(MICK, "place car c3 right", &p).unwrap();
        g.command(MICK, "place des d3 right", &p).unwrap();
        g.command(MICK, "place cru e3 right", &p).unwrap();
        let resp = g.command(MICK, "place bat f3 right", &p).unwrap();
        assert_eq!(1, resp.logs.len());
    }

    #[test]
    fn test_place_off_board_errors() {
        let mut g = mock_game();
        let p = players();
        assert!(g.command(MICK, "place sub a10 right", &p).is_err());
        assert!(g.command(MICK, "place car j8 down", &p).is_err());
    }

    #[test]
    fn test_place_overlapping_errors() {
        let mut g = mock_game();
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        assert!(g.command(MICK, "place des b3 right", &p).is_err());
        assert!(g.command(MICK, "place car b2 right", &p).is_err());
    }

    #[test]
    fn test_place_already_placed_errors() {
        let mut g = mock_game();
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        assert!(g.command(MICK, "place sub d3 right", &p).is_err());
    }

    #[test]
    fn test_place_wrong_player_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        assert!(!g.can_place(MICK));
        assert!(g.command(MICK, "place sub a1 right", &p).is_err());
    }

    #[test]
    fn test_shoot_miss() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        let resp = g.command(MICK, "shoot a1", &p).unwrap();
        assert_eq!(1, resp.logs.len());
        assert_eq!(Cell::Miss, g.boards[STEVE][0][0]);
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn test_shoot_hit() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        let resp = g.command(MICK, "shoot b3", &p).unwrap();
        assert_eq!(1, resp.logs.len());
        assert_eq!(Cell::Hit, g.boards[STEVE][1][2]);
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn test_shoot_sunk() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.command(MICK, "shoot b3", &p).unwrap();
        let resp = g.command(STEVE, "shoot a1", &p).unwrap();
        assert_eq!(1, resp.logs.len());
        g.command(MICK, "shoot b4", &p).unwrap();
        g.command(STEVE, "shoot a2", &p).unwrap();
        let resp = g.command(MICK, "shoot b5", &p).unwrap();
        assert_eq!(1, resp.logs.len());
        assert_eq!(0, g.player_ship_hits_remaining(STEVE, Ship::Submarine));
    }

    #[test]
    fn test_shoot_already_shot_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.command(MICK, "shoot a1", &p).unwrap();
        g.command(STEVE, "shoot a1", &p).unwrap();
        assert!(g.command(MICK, "shoot a1", &p).is_err());
    }

    #[test]
    fn test_shoot_wrong_player_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        assert!(g.command(STEVE, "shoot a1", &p).is_err());
    }

    #[test]
    fn test_shoot_before_placing_done_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        assert!(g.command(MICK, "shoot a1", &p).is_err());
    }

    #[test]
    fn test_shoot_after_finished_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(g.is_finished());
        assert!(g.command(MICK, "shoot a1", &p).is_err());
    }

    #[test]
    fn test_player_hits_remaining() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        let total: usize = Ship::all().iter().map(|s| s.size()).sum();
        assert_eq!(total as i32, g.player_hits_remaining(MICK));
        g.boards[MICK][1][2] = Cell::Hit;
        assert_eq!(total as i32 - 1, g.player_hits_remaining(MICK));
    }

    #[test]
    fn test_player_ship_hits_remaining() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        assert_eq!(3, g.player_ship_hits_remaining(MICK, Ship::Submarine));
        assert_eq!(5, g.player_ship_hits_remaining(MICK, Ship::Carrier));
        g.boards[MICK][1][2] = Cell::Hit;
        assert_eq!(2, g.player_ship_hits_remaining(MICK, Ship::Submarine));
    }

    #[test]
    fn test_finished_when_all_ships_sunk() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        assert!(!g.is_finished());
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(g.is_finished());
        g.boards[STEVE][0][0] = Cell::Carrier;
        assert!(!g.is_finished());
    }

    #[test]
    fn test_finished_not_during_placing() {
        let mut g = mock_game();
        assert!(!g.is_finished());
        g.boards[MICK] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(!g.is_finished());
    }

    #[test]
    fn test_placings() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(g.is_finished());
        let placings = g.placings();
        assert_eq!(vec![1, 2], placings);
    }

    #[test]
    fn test_placings_standard_competition_ties() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.boards[MICK] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(g.is_finished());
        assert_eq!(vec![1, 1], g.placings());
    }

    #[test]
    fn test_placings_three_way_tie() {
        let g = Game {
            players: 3,
            current_player: 0,
            phase: Phase::Shooting,
            boards: vec![
                [[Cell::Carrier; BOARD_SIZE]; BOARD_SIZE],
                [[Cell::Carrier; BOARD_SIZE]; BOARD_SIZE],
                [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE],
            ],
            left_to_place: vec![vec![], vec![], vec![]],
        };
        assert!(g.is_finished());
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_points() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        let total: usize = Ship::all().iter().map(|s| s.size()).sum();
        assert_eq!(vec![total as f32, total as f32], g.points());
        g.boards[STEVE][1][2] = Cell::Hit;
        assert_eq!(vec![total as f32, (total - 1) as f32], g.points());
    }

    #[test]
    fn test_command_unknown_input_errors() {
        let mut g = mock_game();
        let p = players();
        assert!(g.command(MICK, "fly", &p).is_err());
    }

    #[test]
    fn test_command_after_finished_errors() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        assert!(g.command(MICK, "shoot a1", &p).is_err());
        assert!(g.command(MICK, "place sub a1 right", &p).is_err());
    }

    #[test]
    fn test_pub_state_redacts_ships() {
        let g = mock_game();
        let ps = g.pub_state();
        assert!(!ps.finished);
        for p in 0..2 {
            for y in 0..BOARD_SIZE {
                for x in 0..BOARD_SIZE {
                    assert_eq!(Cell::Empty, ps.boards[p][y][x]);
                }
            }
        }
    }

    #[test]
    fn test_pub_state_shows_ships_when_finished() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        g.boards[STEVE] = [[Cell::Hit; BOARD_SIZE]; BOARD_SIZE];
        let ps = g.pub_state();
        assert!(ps.finished);
        assert_eq!(g.boards[MICK], ps.boards[MICK]);
        assert_eq!(g.boards[STEVE], ps.boards[STEVE]);
    }

    #[test]
    fn test_pub_state_captures_fields() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        let ps = g.pub_state();
        assert_eq!(g.players, ps.players);
        assert_eq!(g.phase, ps.phase);
        assert_eq!(g.current_player, ps.current_player);
        assert_eq!(
            g.left_to_place.iter().map(|v| v.len()).collect::<Vec<_>>(),
            ps.left_to_place_counts
        );
        assert!(!ps.finished);
        assert!(ps.placings.is_empty());
    }

    #[test]
    fn test_player_state_includes_own_board() {
        let mut g = mock_game();
        place_all(&mut g, MICK);
        let ps = g.player_state(MICK);
        assert_eq!(MICK, ps.player);
        assert_eq!(g.boards[MICK], ps.board);
        assert_eq!(g.left_to_place[MICK], ps.left_to_place);
    }

    #[test]
    fn test_player_state_board_has_ships() {
        let mut g = mock_game();
        let p = players();
        g.command(MICK, "place sub b3 right", &p).unwrap();
        let ps = g.player_state(MICK);
        assert_eq!(Cell::Submarine, ps.board[1][2]);
        assert_eq!(Cell::Submarine, ps.board[1][3]);
    }

    #[test]
    fn test_alternating_turns() {
        let mut g = mock_game();
        let p = players();
        place_all(&mut g, MICK);
        place_all(&mut g, STEVE);
        assert_eq!(MICK, g.current_player);
        g.command(MICK, "shoot a1", &p).unwrap();
        assert_eq!(STEVE, g.current_player);
        g.command(STEVE, "shoot a1", &p).unwrap();
        assert_eq!(MICK, g.current_player);
    }
}
