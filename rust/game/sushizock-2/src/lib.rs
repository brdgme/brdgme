use std::collections::HashSet;

use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;
use rand::prelude::*;

use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 5;
const START_DICE: usize = 5;
const START_ROLLS: i32 = 2;

#[derive(Default, Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DieFace {
    #[default]
    Sushi,
    BlueChopsticks,
    Bones,
    RedChopsticks,
}

const DIE_FACES: [DieFace; 6] = [
    DieFace::Sushi,
    DieFace::Sushi,
    DieFace::Bones,
    DieFace::Bones,
    DieFace::BlueChopsticks,
    DieFace::RedChopsticks,
];

#[derive(Default, Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TileType {
    #[default]
    Blue,
    Red,
}

impl std::fmt::Display for TileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TileType::Blue => write!(f, "blue"),
            TileType::Red => write!(f, "red"),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Tile {
    pub kind: TileType,
    pub value: i32,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub blue_tiles: Vec<Tile>,
    pub red_tiles: Vec<Tile>,
    pub player_blue_tiles: Vec<Vec<Tile>>,
    pub player_red_tiles: Vec<Vec<Tile>>,
    pub rolled_dice: Vec<DieFace>,
    pub kept_dice: Vec<DieFace>,
    pub remaining_rolls: i32,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PubState {
    pub players: usize,
    pub current_player: usize,
    pub blue_tiles: Vec<Tile>,
    pub red_tiles: Vec<Tile>,
    pub player_blue_tiles: Vec<Vec<Tile>>,
    pub player_red_tiles: Vec<Vec<Tile>>,
    pub rolled_dice: Vec<DieFace>,
    pub kept_dice: Vec<DieFace>,
    pub remaining_rolls: i32,
    pub finished: bool,
    pub final_scores: Vec<i32>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct DiceCounts {
    sushi: usize,
    blue_chopsticks: usize,
    bones: usize,
    red_chopsticks: usize,
}

pub fn dice_counts_pub(pub_state: &PubState) -> DiceCounts {
    let mut d = pub_state.rolled_dice.clone();
    d.extend(&pub_state.kept_dice);
    dice_counts(&d)
}

fn dice_counts(dice: &[DieFace]) -> DiceCounts {
    let mut c = DiceCounts::default();
    for d in dice {
        match d {
            DieFace::Sushi => c.sushi += 1,
            DieFace::BlueChopsticks => c.blue_chopsticks += 1,
            DieFace::Bones => c.bones += 1,
            DieFace::RedChopsticks => c.red_chopsticks += 1,
        }
    }
    c
}

fn all_dice(rolled: &[DieFace], kept: &[DieFace]) -> Vec<DieFace> {
    let mut d = rolled.to_vec();
    d.extend_from_slice(kept);
    d
}

fn roll_dice(n: usize) -> Vec<DieFace> {
    let mut rng = rand::rng();
    (0..n)
        .map(|_| *DIE_FACES.choose(&mut rng).unwrap())
        .collect()
}

fn blue_tiles() -> Vec<Tile> {
    vec![
        Tile {
            kind: TileType::Blue,
            value: 1,
        },
        Tile {
            kind: TileType::Blue,
            value: 1,
        },
        Tile {
            kind: TileType::Blue,
            value: 2,
        },
        Tile {
            kind: TileType::Blue,
            value: 2,
        },
        Tile {
            kind: TileType::Blue,
            value: 3,
        },
        Tile {
            kind: TileType::Blue,
            value: 3,
        },
        Tile {
            kind: TileType::Blue,
            value: 4,
        },
        Tile {
            kind: TileType::Blue,
            value: 4,
        },
        Tile {
            kind: TileType::Blue,
            value: 5,
        },
        Tile {
            kind: TileType::Blue,
            value: 5,
        },
        Tile {
            kind: TileType::Blue,
            value: 6,
        },
        Tile {
            kind: TileType::Blue,
            value: 6,
        },
    ]
}

fn red_tiles() -> Vec<Tile> {
    vec![
        Tile {
            kind: TileType::Red,
            value: -1,
        },
        Tile {
            kind: TileType::Red,
            value: -1,
        },
        Tile {
            kind: TileType::Red,
            value: -1,
        },
        Tile {
            kind: TileType::Red,
            value: -1,
        },
        Tile {
            kind: TileType::Red,
            value: -1,
        },
        Tile {
            kind: TileType::Red,
            value: -2,
        },
        Tile {
            kind: TileType::Red,
            value: -2,
        },
        Tile {
            kind: TileType::Red,
            value: -2,
        },
        Tile {
            kind: TileType::Red,
            value: -2,
        },
        Tile {
            kind: TileType::Red,
            value: -3,
        },
        Tile {
            kind: TileType::Red,
            value: -3,
        },
        Tile {
            kind: TileType::Red,
            value: -4,
        },
    ]
}

fn score(blue: &[Tile], red: &[Tile]) -> i32 {
    let mut s: i32 = red.iter().map(|t| t.value).sum();
    for (i, b) in blue.iter().enumerate() {
        if i < red.len() {
            s += b.value;
        }
    }
    s
}

impl Game {
    pub fn player_score(&self, player: usize) -> i32 {
        score(
            &self.player_blue_tiles[player],
            &self.player_red_tiles[player],
        )
    }

    pub fn dice_counts_all(&self) -> DiceCounts {
        dice_counts(&all_dice(&self.rolled_dice, &self.kept_dice))
    }

    pub fn is_finished(&self) -> bool {
        self.blue_tiles.is_empty() && self.red_tiles.is_empty()
    }

    pub fn whose_turn_inner(&self) -> Vec<usize> {
        if self.is_finished() {
            vec![]
        } else {
            vec![self.current_player]
        }
    }

    pub fn can_roll(&self, player: usize) -> bool {
        self.current_player == player && self.remaining_rolls > 0 && self.rolled_dice.len() > 1
    }

    pub fn can_take_blue(&self, player: usize) -> bool {
        if player != self.current_player {
            return false;
        }
        let c = self.dice_counts_all();
        c.sushi > 0 && self.blue_tiles.len() >= c.sushi
    }

    pub fn can_take_red(&self, player: usize) -> bool {
        if player != self.current_player {
            return false;
        }
        let c = self.dice_counts_all();
        c.bones > 0 && self.red_tiles.len() >= c.bones
    }

    pub fn can_take(&self, player: usize) -> bool {
        self.can_take_blue(player) || self.can_take_red(player)
    }

    pub fn another_player_has_blue(&self, player: usize) -> bool {
        (0..self.players).any(|p| p != player && !self.player_blue_tiles[p].is_empty())
    }

    pub fn another_player_has_red(&self, player: usize) -> bool {
        (0..self.players).any(|p| p != player && !self.player_red_tiles[p].is_empty())
    }

    pub fn can_steal_blue(&self, player: usize) -> bool {
        self.current_player == player
            && self.another_player_has_blue(player)
            && self.dice_counts_all().blue_chopsticks >= 3
    }

    pub fn can_steal_red(&self, player: usize) -> bool {
        self.current_player == player
            && self.another_player_has_red(player)
            && self.dice_counts_all().red_chopsticks >= 3
    }

    pub fn can_steal_blue_n(&self, player: usize) -> bool {
        self.current_player == player
            && self.another_player_has_blue(player)
            && self.dice_counts_all().blue_chopsticks >= 4
    }

    pub fn can_steal_red_n(&self, player: usize) -> bool {
        self.current_player == player
            && self.another_player_has_red(player)
            && self.dice_counts_all().red_chopsticks >= 4
    }

    pub fn can_steal(&self, player: usize) -> bool {
        self.can_steal_blue(player) || self.can_steal_red(player)
    }

    pub fn start_turn(&mut self) -> Vec<Log> {
        self.rolled_dice = roll_dice(START_DICE);
        self.kept_dice = vec![];
        self.remaining_rolls = START_ROLLS;
        vec![Log::public(vec![
            N::Player(self.current_player),
            N::text(" rolled  "),
            render::bold_dice(&self.rolled_dice),
        ])]
    }

    pub fn next_player(&mut self) -> Vec<Log> {
        if self.is_finished() {
            return self.log_game_end();
        }
        self.current_player = (self.current_player + 1) % self.players;
        self.start_turn()
    }

    pub fn log_game_end(&self) -> Vec<Log> {
        use brdgme_markup::{Align as A, Row};
        let mut rows: Vec<Row> = vec![vec![
            (A::Left, vec![N::Bold(vec![N::text("Player")])]),
            (A::Left, vec![N::Bold(vec![N::text("Tiles")])]),
            (A::Left, vec![N::Bold(vec![N::text("Score")])]),
        ]];
        for p in 0..self.players {
            let mut all_tiles: Vec<Tile> = self.player_blue_tiles[p].clone();
            all_tiles.extend(self.player_red_tiles[p].clone());
            rows.push(vec![
                (A::Left, vec![N::Player(p)]),
                (A::Left, vec![render::bold_tile_list(&all_tiles)]),
                (
                    A::Left,
                    vec![N::Bold(vec![N::text(format!(
                        "{} points",
                        self.player_score(p)
                    ))])],
                ),
            ]);
        }
        vec![Log::public(vec![
            N::Bold(vec![N::text(
                "The game is now finished, scores are as follows:\n",
            )]),
            N::Table(rows),
        ])]
    }

    pub fn take_blue(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_take_blue(player) {
            return Err(GameError::invalid_input(
                "unable to take blue at the moment",
            ));
        }
        let idx = self.dice_counts_all().sushi - 1;
        let t = self.blue_tiles.remove(idx);
        self.player_blue_tiles[player].push(t);
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            N::Bold(vec![render::tile(&t)]),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn take_red(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_take_red(player) {
            return Err(GameError::invalid_input("unable to take red at the moment"));
        }
        let idx = self.dice_counts_all().bones - 1;
        let t = self.red_tiles.remove(idx);
        self.player_red_tiles[player].push(t);
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            N::Bold(vec![render::tile(&t)]),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn steal_blue(
        &mut self,
        player: usize,
        target: usize,
        n: Option<i32>,
    ) -> Result<Vec<Log>, GameError> {
        let n = n.unwrap_or(1);
        if n == 1 {
            if !self.can_steal_blue(player) {
                return Err(GameError::invalid_input(
                    "can't steal a blue tile at the moment",
                ));
            }
        } else if !self.can_steal_blue_n(player) {
            return Err(GameError::invalid_input(
                "can't steal a hidden blue tile at the moment",
            ));
        }
        if player == target {
            return Err(GameError::invalid_input("can't steal from yourself"));
        }
        if self.player_blue_tiles[target].is_empty() {
            return Err(GameError::invalid_input(
                "they don't have any blue tiles to steal",
            ));
        }
        let len = self.player_blue_tiles[target].len();
        let index = len as i32 - n;
        if index < 0 || index as usize >= len {
            return Err(GameError::invalid_input(format!(
                "invalid tile number, you need to pick something between 1 and {}",
                len
            )));
        }
        let idx = index as usize;
        let t = self.player_blue_tiles[target].remove(idx);
        self.player_blue_tiles[player].push(t);
        let mut logs = self.steal_log(player, target, &t);
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn steal_red(
        &mut self,
        player: usize,
        target: usize,
        n: Option<i32>,
    ) -> Result<Vec<Log>, GameError> {
        let n = n.unwrap_or(1);
        if n == 1 {
            if !self.can_steal_red(player) {
                return Err(GameError::invalid_input(
                    "can't steal a red tile at the moment",
                ));
            }
        } else if !self.can_steal_red_n(player) {
            return Err(GameError::invalid_input(
                "can't steal a hidden red tile at the moment",
            ));
        }
        if player == target {
            return Err(GameError::invalid_input("can't steal from yourself"));
        }
        if self.player_red_tiles[target].is_empty() {
            return Err(GameError::invalid_input(
                "they don't have any red tiles to steal",
            ));
        }
        let len = self.player_red_tiles[target].len();
        let index = len as i32 - n;
        if index < 0 || index as usize >= len {
            return Err(GameError::invalid_input(format!(
                "invalid tile number, you need to pick something between 1 and {}",
                len
            )));
        }
        let idx = index as usize;
        let t = self.player_red_tiles[target].remove(idx);
        self.player_red_tiles[player].push(t);
        let mut logs = self.steal_log(player, target, &t);
        logs.extend(self.next_player());
        Ok(logs)
    }

    fn steal_log(&self, player: usize, target: usize, tile: &Tile) -> Vec<Log> {
        vec![Log::public(vec![
            N::Player(player),
            N::text(" stole "),
            N::Bold(vec![render::tile(tile)]),
            N::text(" from "),
            N::Player(target),
        ])]
    }

    pub fn take_worst(&mut self) -> Vec<Log> {
        let player = self.current_player;
        if !self.red_tiles.is_empty() {
            let mut min_idx = 0;
            let mut min_val = self.red_tiles[0].value;
            for (i, t) in self.red_tiles.iter().enumerate() {
                if t.value < min_val {
                    min_val = t.value;
                    min_idx = i;
                }
            }
            let t = self.red_tiles.remove(min_idx);
            self.player_red_tiles[player].push(t);
            let mut logs = vec![Log::public(vec![
                N::Player(player),
                N::text(" is forced to take "),
                N::Bold(vec![render::tile(&t)]),
            ])];
            logs.extend(self.next_player());
            logs
        } else {
            let mut min_idx = 0;
            let mut min_val = self.blue_tiles[0].value;
            for (i, t) in self.blue_tiles.iter().enumerate() {
                if t.value < min_val {
                    min_val = t.value;
                    min_idx = i;
                }
            }
            let t = self.blue_tiles.remove(min_idx);
            self.player_blue_tiles[player].push(t);
            let mut logs = vec![Log::public(vec![
                N::Player(player),
                N::text(" is forced to take "),
                N::Bold(vec![render::tile(&t)]),
            ])];
            logs.extend(self.next_player());
            logs
        }
    }

    pub fn roll_dice_cmd(
        &mut self,
        player: usize,
        dice_nums: &[i32],
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_roll(player) {
            return Err(GameError::invalid_input("unable to roll at the moment"));
        }
        let max = self.rolled_dice.len() as i32;
        let mut roll_set: HashSet<usize> = HashSet::new();
        for &d in dice_nums {
            if d < 1 || d > max {
                return Err(GameError::invalid_input(format!(
                    "{} is not a valid die number",
                    d
                )));
            }
            roll_set.insert((d - 1) as usize);
        }
        if roll_set.len() == self.rolled_dice.len() {
            return Err(GameError::invalid_input("you must keep at least one die"));
        }
        let mut kept: Vec<DieFace> = vec![];
        let mut rolled: Vec<DieFace> = vec![];
        for (i, d) in self.rolled_dice.iter().enumerate() {
            if roll_set.contains(&i) {
                rolled.push(*d);
            } else {
                kept.push(*d);
            }
        }
        self.kept_dice.extend(kept);
        self.rolled_dice = roll_dice(roll_set.len());
        self.remaining_rolls -= 1;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" rolled  "),
            render::dice_row_bold_then_normal(&self.rolled_dice, &self.kept_dice),
        ])];
        if self.remaining_rolls == 0 || self.rolled_dice.len() == 1 {
            self.kept_dice.extend(self.rolled_dice.clone());
            self.rolled_dice = vec![];
            self.remaining_rolls = 0;
            if !self.can_take(player) && !self.can_steal(player) {
                logs.extend(self.take_worst());
            }
        }
        Ok(logs)
    }

    pub fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_score(p)])
            .collect();
        gen_placings(&metrics)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let mut g = Game {
            players,
            blue_tiles: blue_tiles(),
            red_tiles: red_tiles(),
            player_blue_tiles: vec![vec![]; players],
            player_red_tiles: vec![vec![]; players],
            ..Game::default()
        };
        g.blue_tiles.shuffle(&mut rand::rng());
        g.red_tiles.shuffle(&mut rand::rng());
        let logs = g.start_turn();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: self.whose_turn_inner(),
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        let finished = self.is_finished();
        PubState {
            players: self.players,
            current_player: self.current_player,
            blue_tiles: self.blue_tiles.clone(),
            red_tiles: self.red_tiles.clone(),
            player_blue_tiles: self.player_blue_tiles.clone(),
            player_red_tiles: self.player_red_tiles.clone(),
            rolled_dice: self.rolled_dice.clone(),
            kept_dice: self.kept_dice.clone(),
            remaining_rolls: self.remaining_rolls,
            finished,
            final_scores: if finished {
                (0..self.players).map(|p| self.player_score(p)).collect()
            } else {
                vec![]
            },
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
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
                value: Command::Roll(dice),
                ..
            }) => {
                let logs = self.roll_dice_cmd(player, &dice)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Take(kind),
                ..
            }) => {
                let logs = match kind {
                    TileType::Blue => self.take_blue(player)?,
                    TileType::Red => self.take_red(player)?,
                };
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Steal { target, kind, num },
                ..
            }) => {
                let logs = match kind {
                    TileType::Blue => self.steal_blue(player, target, num)?,
                    TileType::Red => self.steal_red(player, target, num)?,
                };
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
            .map(|p| self.player_score(p) as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4, 5]
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
    const BJ: usize = 2;

    fn names() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()]
    }

    // --- 1:1 Go test ports ---

    #[test]
    fn test_new() {
        let g = Game::start(2);
        assert!(g.is_ok());
    }

    #[test]
    fn test_roll() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.command(MICK, "roll 1 2 5", &n).unwrap();
        g.command(MICK, "roll 2 3", &n).unwrap();
        assert_eq!(0, g.remaining_rolls);
    }

    #[test]
    fn test_take_blue() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.rolled_dice = vec![
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::BlueChopsticks,
            DieFace::RedChopsticks,
            DieFace::Bones,
        ];
        let target = g.blue_tiles[1];
        g.command(MICK, "take b", &n).unwrap();
        assert_eq!(vec![target], g.player_blue_tiles[MICK]);
    }

    #[test]
    fn test_take_red() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.rolled_dice = vec![
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::BlueChopsticks,
            DieFace::RedChopsticks,
            DieFace::Bones,
        ];
        let target = g.red_tiles[0];
        g.command(MICK, "take r", &n).unwrap();
        assert_eq!(vec![target], g.player_red_tiles[MICK]);
    }

    #[test]
    fn test_force_take_most_negative_red() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.rolled_dice = vec![
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::BlueChopsticks,
        ];
        g.blue_tiles = vec![];
        g.red_tiles = vec![
            Tile {
                kind: TileType::Red,
                value: -2,
            },
            Tile {
                kind: TileType::Red,
                value: -4,
            },
            Tile {
                kind: TileType::Red,
                value: -3,
            },
        ];
        let target = g.red_tiles[1];
        g.command(MICK, "roll 5", &n).unwrap();
        assert_eq!(vec![target], g.player_red_tiles[MICK]);
    }

    #[test]
    fn test_force_take_lowest_blue() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.rolled_dice = vec![
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::BlueChopsticks,
        ];
        g.blue_tiles = vec![
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
        ];
        g.red_tiles = vec![];
        let target = g.blue_tiles[1];
        g.command(MICK, "roll 5", &n).unwrap();
        assert_eq!(vec![target], g.player_blue_tiles[MICK]);
    }

    #[test]
    fn test_steal_blue() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_blue_tiles[BJ] = vec![
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
        ];
        g.rolled_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        let target = g.player_blue_tiles[BJ][2];
        g.command(MICK, "roll 5", &n).unwrap();
        g.command(MICK, "steal bj blue", &n).unwrap();
        assert_eq!(vec![target], g.player_blue_tiles[MICK]);
        assert_eq!(
            vec![
                Tile {
                    kind: TileType::Blue,
                    value: 3
                },
                Tile {
                    kind: TileType::Blue,
                    value: 1
                },
            ],
            g.player_blue_tiles[BJ]
        );
    }

    #[test]
    fn test_steal_red() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_red_tiles[STEVE] = vec![
            Tile {
                kind: TileType::Red,
                value: -3,
            },
            Tile {
                kind: TileType::Red,
                value: -1,
            },
            Tile {
                kind: TileType::Red,
                value: -2,
            },
        ];
        g.rolled_dice = vec![
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        let target = g.player_red_tiles[STEVE][2];
        g.command(MICK, "roll 5", &n).unwrap();
        g.command(MICK, "steal ste r", &n).unwrap();
        assert_eq!(vec![target], g.player_red_tiles[MICK]);
        assert_eq!(
            vec![
                Tile {
                    kind: TileType::Red,
                    value: -3
                },
                Tile {
                    kind: TileType::Red,
                    value: -1
                },
            ],
            g.player_red_tiles[STEVE]
        );
    }

    #[test]
    fn test_steal_red_n_not_allowed() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_red_tiles[STEVE] = vec![
            Tile {
                kind: TileType::Red,
                value: -3,
            },
            Tile {
                kind: TileType::Red,
                value: -1,
            },
            Tile {
                kind: TileType::Red,
                value: -2,
            },
        ];
        g.command(MICK, "roll 5", &n).unwrap();
        g.kept_dice = vec![
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        assert!(g.command(MICK, "steal ste r 2", &n).is_err());
    }

    #[test]
    fn test_steal_blue_n() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_blue_tiles[BJ] = vec![
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
        ];
        g.rolled_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Bones,
        ];
        let target = g.player_blue_tiles[BJ][0];
        g.command(MICK, "roll 5", &n).unwrap();
        g.command(MICK, "steal bj blue 3", &n).unwrap();
        assert_eq!(vec![target], g.player_blue_tiles[MICK]);
        assert_eq!(
            vec![
                Tile {
                    kind: TileType::Blue,
                    value: 1
                },
                Tile {
                    kind: TileType::Blue,
                    value: 2
                },
            ],
            g.player_blue_tiles[BJ]
        );
    }

    #[test]
    fn test_steal_red_n() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_red_tiles[STEVE] = vec![
            Tile {
                kind: TileType::Red,
                value: -3,
            },
            Tile {
                kind: TileType::Red,
                value: -1,
            },
            Tile {
                kind: TileType::Red,
                value: -2,
            },
        ];
        g.rolled_dice = vec![
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::Bones,
        ];
        let target = g.player_red_tiles[STEVE][1];
        g.command(MICK, "roll 5", &n).unwrap();
        g.command(MICK, "steal ste r 2", &n).unwrap();
        assert_eq!(vec![target], g.player_red_tiles[MICK]);
        assert_eq!(
            vec![
                Tile {
                    kind: TileType::Red,
                    value: -3
                },
                Tile {
                    kind: TileType::Red,
                    value: -2
                },
            ],
            g.player_red_tiles[STEVE]
        );
    }

    // --- tile_test.go 1:1 port ---

    #[test]
    fn test_tiles_remove() {
        let tiles = vec![
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
            Tile {
                kind: TileType::Blue,
                value: 4,
            },
            Tile {
                kind: TileType::Blue,
                value: 5,
            },
        ];
        let mut remaining = tiles.clone();
        let actual = remaining.remove(2);
        let expected = Tile {
            kind: TileType::Blue,
            value: 3,
        };
        assert_eq!(expected, actual);
        let expected_remaining = vec![
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
            Tile {
                kind: TileType::Blue,
                value: 4,
            },
            Tile {
                kind: TileType::Blue,
                value: 5,
            },
        ];
        assert_eq!(expected_remaining, remaining);
    }

    // --- baseline tests per step 8's thin-suite rule ---

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2, 3, 4, 5], Game::player_counts());
        assert!(Game::start(1).is_err());
        assert!(Game::start(6).is_err());
        assert!(Game::start(2).is_ok());
        assert!(Game::start(5).is_ok());
    }

    #[test]
    fn test_start_state() {
        let (g, logs) = Game::start(3).unwrap();
        assert!(!logs.is_empty());
        assert_eq!(3, g.players);
        assert_eq!(12, g.blue_tiles.len());
        assert_eq!(12, g.red_tiles.len());
        assert_eq!(START_DICE, g.rolled_dice.len());
        assert!(g.kept_dice.is_empty());
        assert_eq!(START_ROLLS, g.remaining_rolls);
        assert_eq!(0, g.current_player);
        assert!(!g.is_finished());
        assert_eq!(vec![0], g.whose_turn());
    }

    #[test]
    fn test_tile_decks() {
        let bt = blue_tiles();
        assert_eq!(12, bt.len());
        assert_eq!(1, bt[0].value);
        assert_eq!(6, bt[11].value);
        assert!(bt.iter().all(|t| t.kind == TileType::Blue));
        let rt = red_tiles();
        assert_eq!(12, rt.len());
        assert_eq!(-1, rt[0].value);
        assert_eq!(-4, rt[11].value);
        assert!(rt.iter().all(|t| t.kind == TileType::Red));
    }

    #[test]
    fn test_scoring() {
        // No tiles -> 0
        assert_eq!(0, score(&[], &[]));
        // 1 red, 1 blue -> red + blue
        assert_eq!(
            4,
            score(
                &[Tile {
                    kind: TileType::Blue,
                    value: 5
                }],
                &[Tile {
                    kind: TileType::Red,
                    value: -1
                }]
            )
        );
        // 2 red, 3 blue -> sum of 2 red + sum of first 2 blue (third blue doesn't score)
        assert_eq!(
            1 + 2 + (-1) + (-2),
            score(
                &[
                    Tile {
                        kind: TileType::Blue,
                        value: 1
                    },
                    Tile {
                        kind: TileType::Blue,
                        value: 2
                    },
                    Tile {
                        kind: TileType::Blue,
                        value: 99
                    },
                ],
                &[
                    Tile {
                        kind: TileType::Red,
                        value: -1
                    },
                    Tile {
                        kind: TileType::Red,
                        value: -2
                    },
                ]
            )
        );
        // 0 red, 3 blue -> 0 (no red means no blue scores)
        assert_eq!(
            0,
            score(
                &[
                    Tile {
                        kind: TileType::Blue,
                        value: 1
                    },
                    Tile {
                        kind: TileType::Blue,
                        value: 2
                    },
                    Tile {
                        kind: TileType::Blue,
                        value: 3
                    },
                ],
                &[]
            )
        );
    }

    #[test]
    fn test_dice_counts() {
        let c = dice_counts(&[
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::Bones,
            DieFace::BlueChopsticks,
            DieFace::RedChopsticks,
        ]);
        assert_eq!(2, c.sushi);
        assert_eq!(1, c.bones);
        assert_eq!(1, c.blue_chopsticks);
        assert_eq!(1, c.red_chopsticks);
    }

    #[test]
    fn test_can_roll() {
        let (mut g, _) = Game::start(2).unwrap();
        assert!(g.can_roll(0));
        assert!(!g.can_roll(1));
        // Can't roll with 1 die
        g.rolled_dice = vec![DieFace::Sushi];
        assert!(!g.can_roll(0));
        // Can't roll with 0 remaining rolls
        g.rolled_dice = vec![DieFace::Sushi, DieFace::Bones];
        g.remaining_rolls = 0;
        assert!(!g.can_roll(0));
    }

    #[test]
    fn test_can_take_guards() {
        let (mut g, _) = Game::start(2).unwrap();
        // 2 sushi, 3 bones, full tiles -> can take both
        g.rolled_dice = vec![
            DieFace::Sushi,
            DieFace::Sushi,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
        ];
        assert!(g.can_take_blue(0));
        assert!(g.can_take_red(0));
        // Not your turn
        assert!(!g.can_take_blue(1));
        assert!(!g.can_take_red(1));
        // Sushi count > blue tiles available
        g.blue_tiles = vec![Tile {
            kind: TileType::Blue,
            value: 1,
        }];
        assert!(!g.can_take_blue(0)); // 2 sushi > 1 blue tile
        // Bones count > red tiles available
        g.red_tiles = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        g.rolled_dice = vec![
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
        ];
        assert!(!g.can_take_red(0)); // 5 bones > 1 red tile
        // No sushi at all
        g.rolled_dice = vec![
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
        ];
        g.blue_tiles = vec![
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 2,
            },
        ];
        assert!(!g.can_take_blue(0)); // 0 sushi
    }

    #[test]
    fn test_can_steal_guards() {
        let (mut g, _) = Game::start(3).unwrap();
        // No one has tiles -> can't steal even with chopsticks
        g.kept_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        g.rolled_dice = vec![];
        assert!(!g.can_steal_blue(0)); // no one has blue tiles
        g.player_blue_tiles[BJ] = vec![Tile {
            kind: TileType::Blue,
            value: 3,
        }];
        assert!(g.can_steal_blue(0));
        assert!(!g.can_steal_blue_n(0)); // only 3 chopsticks
        g.kept_dice.push(DieFace::BlueChopsticks);
        assert!(g.can_steal_blue_n(0)); // now 4 chopsticks
        // Not your turn
        assert!(!g.can_steal_blue(1));
        // Only 2 chopsticks
        g.kept_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
            DieFace::Bones,
        ];
        assert!(!g.can_steal_blue(0));
    }

    #[test]
    fn test_roll_must_keep_one() {
        let (mut g, _) = Game::start(2).unwrap();
        let n = names();
        // Rolling all 5 dice is not allowed
        assert!(g.command(0, "roll 1 2 3 4 5", &n).is_err());
    }

    #[test]
    fn test_roll_invalid_die_number() {
        let (mut g, _) = Game::start(2).unwrap();
        let n = names();
        assert!(g.command(0, "roll 6", &n).is_err());
        assert!(g.command(0, "roll 0", &n).is_err());
    }

    #[test]
    fn test_roll_wrong_player() {
        let (mut g, _) = Game::start(2).unwrap();
        let n = names();
        assert!(g.command(1, "roll 1", &n).is_err());
    }

    #[test]
    fn test_command_after_finished() {
        let (mut g, _) = Game::start(2).unwrap();
        let n = names();
        g.blue_tiles = vec![];
        g.red_tiles = vec![];
        assert!(g.is_finished());
        assert!(g.command(0, "roll 1", &n).is_err());
        assert!(g.command(0, "take blue", &n).is_err());
        assert!(g.command(0, "steal 1 blue", &n).is_err());
    }

    #[test]
    fn test_command_unknown() {
        let (mut g, _) = Game::start(2).unwrap();
        let n = names();
        assert!(g.command(0, "frobnicate", &n).is_err());
    }

    #[test]
    fn test_steal_from_self() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_blue_tiles[MICK] = vec![Tile {
            kind: TileType::Blue,
            value: 3,
        }];
        g.kept_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        g.rolled_dice = vec![];
        // "steal mick blue" -> can't steal from yourself
        assert!(g.command(MICK, "steal mick blue", &n).is_err());
    }

    #[test]
    fn test_steal_from_empty_player() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        // Steve has no tiles
        g.kept_dice = vec![
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        g.rolled_dice = vec![];
        assert!(g.command(MICK, "steal ste r", &n).is_err());
    }

    #[test]
    fn test_take_advances_turn() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.rolled_dice = vec![
            DieFace::Sushi,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
            DieFace::Bones,
        ];
        g.command(MICK, "take b", &n).unwrap();
        assert_eq!(STEVE, g.current_player);
        assert_eq!(START_ROLLS, g.remaining_rolls);
        assert_eq!(START_DICE, g.rolled_dice.len());
    }

    #[test]
    fn test_steal_advances_turn() {
        let (mut g, _) = Game::start(3).unwrap();
        let n = names();
        g.player_blue_tiles[BJ] = vec![Tile {
            kind: TileType::Blue,
            value: 3,
        }];
        g.rolled_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Sushi,
            DieFace::Bones,
        ];
        g.kept_dice = vec![];
        // After roll 5 -> all dice kept, then steal
        g.command(MICK, "roll 5", &n).unwrap();
        g.command(MICK, "steal bj blue", &n).unwrap();
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn test_placings() {
        let (mut g, _) = Game::start(3).unwrap();
        // Mick: 5, Steve: 3, BJ: 7 -> placings [2, 3, 1]
        g.player_blue_tiles[MICK] = vec![Tile {
            kind: TileType::Blue,
            value: 5,
        }];
        g.player_red_tiles[MICK] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        // score = -1 + 5 = 4... let me set up clearer scores
        g.player_blue_tiles[MICK] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[MICK] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        // Mick score = -1 + 6 = 5
        g.player_blue_tiles[STEVE] = vec![Tile {
            kind: TileType::Blue,
            value: 4,
        }];
        g.player_red_tiles[STEVE] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        // Steve score = -1 + 4 = 3
        g.player_blue_tiles[BJ] = vec![Tile {
            kind: TileType::Blue,
            value: 8,
        }];
        g.player_red_tiles[BJ] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        // BJ score = -1 + 8 = 7
        g.blue_tiles = vec![];
        g.red_tiles = vec![];
        assert_eq!(vec![2, 3, 1], g.placings());
    }

    #[test]
    fn test_placings_tie_standard_competition() {
        let (mut g, _) = Game::start(3).unwrap();
        // Mick and Steve tied at 5; BJ at 3
        g.player_blue_tiles[MICK] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[MICK] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        g.player_blue_tiles[STEVE] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[STEVE] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        g.player_blue_tiles[BJ] = vec![Tile {
            kind: TileType::Blue,
            value: 4,
        }];
        g.player_red_tiles[BJ] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        g.blue_tiles = vec![];
        g.red_tiles = vec![];
        // Rust gen_placings uses standard-competition: two tied at top -> [1, 1, 3]
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_points_always_current() {
        let (mut g, _) = Game::start(3).unwrap();
        g.player_blue_tiles[0] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[0] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        // Points reflect current score even when not finished
        let pts = g.points();
        assert_eq!(5.0, pts[0]);
        assert_eq!(0.0, pts[1]);
        assert_eq!(0.0, pts[2]);
    }

    #[test]
    fn test_finished_placings() {
        let (mut g, _) = Game::start(3).unwrap();
        g.blue_tiles = vec![];
        g.red_tiles = vec![];
        g.player_blue_tiles[0] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[0] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        assert!(g.is_finished());
        let s = g.status();
        match s {
            Status::Finished { placings, .. } => {
                assert_eq!(1, placings[0]);
            }
            _ => panic!("expected finished"),
        }
    }

    #[test]
    fn test_pub_state_no_hidden_info() {
        let (g, _) = Game::start(3).unwrap();
        let ps = g.pub_state();
        assert_eq!(g.blue_tiles, ps.blue_tiles);
        assert_eq!(g.red_tiles, ps.red_tiles);
        assert_eq!(g.player_blue_tiles, ps.player_blue_tiles);
        assert_eq!(g.player_red_tiles, ps.player_red_tiles);
        assert_eq!(g.rolled_dice, ps.rolled_dice);
        assert_eq!(g.kept_dice, ps.kept_dice);
        assert!(!ps.finished);
        assert!(ps.final_scores.is_empty());
    }

    #[test]
    fn test_finished_pub_state_has_scores() {
        let (mut g, _) = Game::start(3).unwrap();
        g.blue_tiles = vec![];
        g.red_tiles = vec![];
        g.player_blue_tiles[0] = vec![Tile {
            kind: TileType::Blue,
            value: 6,
        }];
        g.player_red_tiles[0] = vec![Tile {
            kind: TileType::Red,
            value: -1,
        }];
        let ps = g.pub_state();
        assert!(ps.finished);
        assert_eq!(vec![5, 0, 0], ps.final_scores);
    }

    #[test]
    fn test_take_worst_red_picks_minimum() {
        let (mut g, _) = Game::start(2).unwrap();
        g.red_tiles = vec![
            Tile {
                kind: TileType::Red,
                value: -1,
            },
            Tile {
                kind: TileType::Red,
                value: -4,
            },
            Tile {
                kind: TileType::Red,
                value: -2,
            },
        ];
        g.blue_tiles = vec![Tile {
            kind: TileType::Blue,
            value: 1,
        }];
        let logs = g.take_worst();
        assert!(!logs.is_empty());
        // Should have taken -4 (the minimum)
        assert_eq!(1, g.player_red_tiles[0].len());
        assert_eq!(-4, g.player_red_tiles[0][0].value);
        // Red tiles should have -1 and -2 remaining
        assert_eq!(2, g.red_tiles.len());
    }

    #[test]
    fn test_take_worst_blue_when_no_red() {
        let (mut g, _) = Game::start(2).unwrap();
        g.red_tiles = vec![];
        g.blue_tiles = vec![
            Tile {
                kind: TileType::Blue,
                value: 5,
            },
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
        ];
        let logs = g.take_worst();
        assert!(!logs.is_empty());
        // Should have taken 1 (the minimum blue)
        assert_eq!(1, g.player_blue_tiles[0].len());
        assert_eq!(1, g.player_blue_tiles[0][0].value);
    }
}
