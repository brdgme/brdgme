use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_color as color;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;
use rand::prelude::*;

use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 8;
pub const WIN_SCORE: i32 = 13;
pub const ROLL_DICE_COUNT: usize = 3;
pub const BUST_SHOTGUN_COUNT: i32 = 3;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Colour {
    Green,
    Yellow,
    Red,
}

impl Colour {
    pub fn to_color(self) -> color::Color {
        match self {
            Colour::Green => color::GREEN,
            Colour::Yellow => color::YELLOW,
            Colour::Red => color::RED,
        }
    }
}

/// All dice faces in zombie dice. The `faces` for a colour are static and
/// deterministic; `Dice` only serializes the colour.
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dice {
    pub colour: Colour,
}

impl Dice {
    pub fn faces(self) -> &'static [Face] {
        match self.colour {
            Colour::Green => &[
                Face::Brain,
                Face::Brain,
                Face::Brain,
                Face::Footprints,
                Face::Footprints,
                Face::Shotgun,
            ],
            Colour::Yellow => &[
                Face::Brain,
                Face::Brain,
                Face::Footprints,
                Face::Footprints,
                Face::Shotgun,
                Face::Shotgun,
            ],
            Colour::Red => &[
                Face::Brain,
                Face::Footprints,
                Face::Footprints,
                Face::Shotgun,
                Face::Shotgun,
                Face::Shotgun,
            ],
        }
    }

    pub fn roll(self) -> Face {
        let faces = self.faces();
        let i = rand::rng().random_range(0..faces.len());
        faces[i]
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Face {
    Brain,
    Shotgun,
    Footprints,
}

impl Face {
    pub fn name(self) -> &'static str {
        match self {
            Face::Brain => "Brain",
            Face::Shotgun => "Shot",
            Face::Footprints => "Run",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DiceResult {
    pub dice: Dice,
    pub face: Face,
}

pub type DiceResultList = Vec<DiceResult>;

pub fn all_dice() -> Vec<Dice> {
    vec![
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Green,
        },
        Dice {
            colour: Colour::Yellow,
        },
        Dice {
            colour: Colour::Yellow,
        },
        Dice {
            colour: Colour::Yellow,
        },
        Dice {
            colour: Colour::Yellow,
        },
        Dice {
            colour: Colour::Red,
        },
        Dice {
            colour: Colour::Red,
        },
        Dice {
            colour: Colour::Red,
        },
    ]
}

pub fn roll_dice(dice: &[Dice]) -> DiceResultList {
    dice.iter()
        .map(|d| DiceResult {
            dice: *d,
            face: d.roll(),
        })
        .collect()
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_turn: usize,
    pub scores: Vec<i32>,
    pub cup: Vec<Dice>,
    /// Empty when no tie-breaker rolloff is active; otherwise the players
    /// still in the rolloff. Faithful port of Go's `map[int]bool` (where
    /// `nil` means no rolloff and an empty map never occurs because a
    /// rolloff always has >= 2 leaders).
    pub roll_off_players: Vec<usize>,
    pub finished: bool,
    pub current_roll: DiceResultList,
    pub kept: DiceResultList,
    pub round_brains: i32,
    pub round_shotguns: i32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub current_turn: usize,
    pub scores: Vec<i32>,
    pub cup: Vec<Dice>,
    pub current_roll: DiceResultList,
    pub kept: DiceResultList,
    pub round_brains: i32,
    pub round_shotguns: i32,
    pub finished: bool,
    pub placings: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
}

impl Game {
    pub fn can_roll(&self, player: usize) -> bool {
        self.current_turn == player && !self.finished
    }

    pub fn can_keep(&self, player: usize) -> bool {
        self.current_turn == player && !self.finished
    }

    /// True when a rolloff is active and `player` is NOT in it (so NextPlayer
    /// should skip them). Faithful port of Go's
    /// `g.RollOffPlayers != nil && !g.RollOffPlayers[player]` check.
    fn should_skip_in_rolloff(&self, player: usize) -> bool {
        !self.roll_off_players.is_empty() && !self.roll_off_players.contains(&player)
    }

    pub fn shake_cup(&mut self) {
        self.cup.shuffle(&mut rand::rng());
    }

    /// Faithful port of Go `TakeDice`. If the cup has fewer than `n` dice,
    /// returns all kept dice to the cup, reshuffles, then takes `n`.
    pub fn take_dice(&mut self, n: usize) -> (Vec<Dice>, Vec<Log>) {
        let mut logs: Vec<Log> = vec![];
        if n == 0 {
            return (vec![], logs);
        }
        if self.cup.len() < n {
            logs.push(Log::public(vec![N::text(
                "Not enough dice remaining, returning kept dice to the cup",
            )]));
            let returned: Vec<Dice> = self.kept.iter().map(|dr| dr.dice).collect();
            self.cup.extend(returned);
            self.kept = vec![];
            self.shake_cup();
        }
        let taken: Vec<Dice> = self.cup.drain(..n).collect();
        (taken, logs)
    }

    pub fn start_turn(&mut self) -> Vec<Log> {
        self.cup = all_dice();
        self.shake_cup();
        self.kept = vec![];
        self.current_roll = vec![];
        self.round_brains = 0;
        self.round_shotguns = 0;
        self.roll()
    }

    pub fn next_player(&mut self) -> Vec<Log> {
        let mut logs: Vec<Log> = vec![];
        self.current_turn = (self.current_turn + 1) % self.players;
        if self.current_turn == 0 {
            // Check for game end (round complete).
            let (score, leaders) = self.leaders();
            if score >= WIN_SCORE {
                if leaders.len() == 1 {
                    self.finished = true;
                    return logs;
                }
                // Roll off!
                self.roll_off_players = leaders.clone();
                let parts: Vec<N> = leaders.iter().map(|&l| N::Player(l)).collect();
                logs.push(Log::public(vec![
                    N::text("It's a tied score of "),
                    N::Bold(vec![N::text(score.to_string())]),
                    N::text(" between "),
                    render::comma_list(parts),
                    N::text(", tie breaker round!"),
                ]));
            }
        }
        if self.should_skip_in_rolloff(self.current_turn) {
            logs.extend(self.next_player());
        } else {
            logs.extend(self.start_turn());
        }
        logs
    }

    pub fn player_roll(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_roll(player) {
            return Err(GameError::invalid_input("can't roll at the moment"));
        }
        Ok(self.roll())
    }

    pub fn roll(&mut self) -> Vec<Log> {
        let mut logs: Vec<Log> = vec![];
        // Collect the footprint dice to re-roll.
        let dice: Vec<Dice> = self.current_roll.iter().map(|dr| dr.dice).collect();
        let dice_len = dice.len();
        let mut dice = dice;
        if dice_len < ROLL_DICE_COUNT {
            let (taken, take_logs) = self.take_dice(ROLL_DICE_COUNT - dice_len);
            logs.extend(take_logs);
            dice.extend(taken);
        }
        let drl = roll_dice(&dice);
        logs.push(Log::public(vec![
            N::Player(self.current_turn),
            N::text(" rolled "),
            render::render_dice_result_list(&drl),
        ]));

        let mut run: DiceResultList = vec![];
        let mut new_brains = 0;
        let mut was_shot = false;
        for dr in &drl {
            match dr.face {
                Face::Brain => {
                    new_brains += 1;
                    self.kept.push(*dr);
                }
                Face::Shotgun => {
                    self.round_shotguns += 1;
                    self.kept.push(*dr);
                    was_shot = true;
                }
                Face::Footprints => {
                    run.push(*dr);
                }
            }
        }
        if self.round_shotguns >= BUST_SHOTGUN_COUNT {
            logs.push(Log::public(vec![
                N::Player(self.current_turn),
                N::text(" got shot three times and lost "),
                N::Bold(vec![N::text(self.round_brains.to_string())]),
                N::text(" brains!"),
            ]));
            logs.extend(self.next_player());
            return logs;
        } else if was_shot {
            logs.push(Log::public(vec![
                N::Player(self.current_turn),
                N::text(" has "),
                N::Bold(vec![N::text(
                    (BUST_SHOTGUN_COUNT - self.round_shotguns).to_string(),
                )]),
                N::text(" health remaining"),
            ]));
        }
        self.round_brains += new_brains;
        self.current_roll = run;
        logs
    }

    pub fn keep(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_keep(player) {
            return Err(GameError::invalid_input("can't keep at the moment"));
        }
        self.scores[self.current_turn] += self.round_brains;
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::Player(self.current_turn),
            N::text(" kept "),
            N::Bold(vec![N::text(self.round_brains.to_string())]),
            N::text(" brains, now has "),
            N::Bold(vec![N::text(self.scores[self.current_turn].to_string())]),
            N::text("!"),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn leaders(&self) -> (i32, Vec<usize>) {
        let mut score = 0;
        let mut leaders: Vec<usize> = vec![];
        for p in 0..self.players {
            if self.scores[p] > score {
                score = self.scores[p];
                leaders = vec![];
            }
            if self.scores[p] == score {
                leaders.push(p);
            }
        }
        (score, leaders)
    }

    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players).map(|p| vec![self.scores[p]]).collect();
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
            scores: vec![0; players],
            ..Game::default()
        };
        let logs = g.start_turn();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.finished {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_turn],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            current_turn: self.current_turn,
            scores: self.scores.clone(),
            cup: self.cup.clone(),
            current_roll: self.current_roll.clone(),
            kept: self.kept.clone(),
            round_brains: self.round_brains,
            round_shotguns: self.round_shotguns,
            finished: self.finished,
            placings: if self.finished {
                self.placings()
            } else {
                vec![]
            },
        }
    }

    fn player_state(&self, _player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
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
                value: Command::Roll,
                ..
            }) => {
                let logs = self.player_roll(player)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Keep,
                ..
            }) => {
                let logs = self.keep(player)?;
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
        self.scores.iter().map(|&s| s as f32).collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4, 5, 6, 7, 8]
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

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2, 3, 4, 5, 6, 7, 8], Game::player_counts());
        assert!(Game::start(1).is_err());
        assert!(Game::start(9).is_err());
        assert!(Game::start(2).is_ok());
        assert!(Game::start(8).is_ok());
    }

    #[test]
    fn test_start_initial_state() {
        let (g, logs) = Game::start(2).unwrap();
        assert_eq!(0, g.current_turn);
        assert!(!g.finished);
        assert!(g.round_shotguns < BUST_SHOTGUN_COUNT);
        // 3 dice were rolled; brains + shotguns are kept, footprints are runners.
        assert_eq!(ROLL_DICE_COUNT, g.kept.len() + g.current_roll.len());
        // Cup has 13 - 3 taken = 10 dice remaining.
        assert_eq!(10, g.cup.len());
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_dice_face_counts() {
        // Green: 3B 2F 1S, Yellow: 2B 2F 2S, Red: 1B 2F 3S
        let green = Dice {
            colour: Colour::Green,
        };
        let yellow = Dice {
            colour: Colour::Yellow,
        };
        let red = Dice {
            colour: Colour::Red,
        };
        assert_eq!(6, green.faces().len());
        assert_eq!(6, yellow.faces().len());
        assert_eq!(6, red.faces().len());
        let count = |d: Dice, f: Face| d.faces().iter().filter(|&&x| x == f).count();
        assert_eq!(3, count(green, Face::Brain));
        assert_eq!(2, count(green, Face::Footprints));
        assert_eq!(1, count(green, Face::Shotgun));
        assert_eq!(2, count(yellow, Face::Brain));
        assert_eq!(2, count(yellow, Face::Footprints));
        assert_eq!(2, count(yellow, Face::Shotgun));
        assert_eq!(1, count(red, Face::Brain));
        assert_eq!(2, count(red, Face::Footprints));
        assert_eq!(3, count(red, Face::Shotgun));
    }

    #[test]
    fn test_all_dice_counts() {
        let cup = all_dice();
        assert_eq!(13, cup.len());
        let green = cup.iter().filter(|d| d.colour == Colour::Green).count();
        let yellow = cup.iter().filter(|d| d.colour == Colour::Yellow).count();
        let red = cup.iter().filter(|d| d.colour == Colour::Red).count();
        assert_eq!(6, green);
        assert_eq!(4, yellow);
        assert_eq!(3, red);
    }

    #[test]
    fn test_take_dice_basic() {
        let mut g = Game::start(2).unwrap().0;
        g.cup = all_dice();
        let (taken, logs) = g.take_dice(3);
        assert_eq!(3, taken.len());
        assert_eq!(10, g.cup.len());
        assert!(logs.is_empty());
    }

    #[test]
    fn test_take_dice_refills_from_kept_when_cup_low() {
        let mut g = Game::start(2).unwrap().0;
        g.cup = vec![];
        g.kept = vec![
            DiceResult {
                dice: Dice {
                    colour: Colour::Green,
                },
                face: Face::Brain,
            },
            DiceResult {
                dice: Dice {
                    colour: Colour::Red,
                },
                face: Face::Shotgun,
            },
        ];
        let (taken, logs) = g.take_dice(2);
        assert_eq!(2, taken.len());
        assert!(g.kept.is_empty());
        // 2 returned, 2 taken.
        assert_eq!(0, g.cup.len());
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_take_dice_zero_returns_empty() {
        let mut g = Game::start(2).unwrap().0;
        g.cup = all_dice();
        let (taken, logs) = g.take_dice(0);
        assert!(taken.is_empty());
        assert!(logs.is_empty());
        assert_eq!(13, g.cup.len());
    }

    #[test]
    fn test_roll_distributes_faces() {
        let mut g = Game::start(2).unwrap().0;
        g.cup = all_dice();
        g.kept = vec![];
        g.current_roll = vec![];
        g.round_brains = 0;
        g.round_shotguns = 0;
        let _ = g.roll();
        // 3 dice rolled: each is brain, shotgun, or footprints.
        assert_eq!(ROLL_DICE_COUNT, g.kept.len() + g.current_roll.len());
    }

    #[test]
    fn test_keep_banks_brains_and_advances() {
        let mut g = Game::start(2).unwrap().0;
        g.current_turn = 0;
        g.round_brains = 4;
        g.scores = vec![0, 0];
        let logs = g.keep(0).unwrap();
        assert_eq!(4, g.scores[0]);
        assert_eq!(1, g.current_turn);
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_keep_wrong_player_errors() {
        let mut g = Game::start(2).unwrap().0;
        g.current_turn = 0;
        assert!(g.keep(1).is_err());
    }

    #[test]
    fn test_can_roll_and_can_keep() {
        let mut g = Game::start(2).unwrap().0;
        g.current_turn = 0;
        g.finished = false;
        assert!(g.can_roll(0));
        assert!(g.can_keep(0));
        assert!(!g.can_roll(1));
        assert!(!g.can_keep(1));
        g.finished = true;
        assert!(!g.can_roll(0));
        assert!(!g.can_keep(0));
    }

    #[test]
    fn test_leaders() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![5, 7, 7];
        let (score, leaders) = g.leaders();
        assert_eq!(7, score);
        assert_eq!(vec![1, 2], leaders);
        g.scores = vec![5, 7, 6];
        let (score, leaders) = g.leaders();
        assert_eq!(7, score);
        assert_eq!(vec![1], leaders);
        // All zero scores: everyone ties at 0.
        g.scores = vec![0, 0, 0];
        let (score, leaders) = g.leaders();
        assert_eq!(0, score);
        assert_eq!(vec![0, 1, 2], leaders);
    }

    #[test]
    fn test_finished_unique_leader_at_threshold() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE, 5, 5];
        g.current_turn = 2; // advancing will wrap to 0.
        g.next_player();
        assert!(g.finished);
    }

    #[test]
    fn test_finished_not_triggered_below_threshold() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE - 1, 5, 5];
        g.current_turn = 2;
        g.next_player();
        assert!(!g.finished);
    }

    #[test]
    fn test_rolloff_starts_on_tie_at_threshold() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE, WIN_SCORE, 5];
        g.current_turn = 2;
        let logs = g.next_player();
        assert!(!g.finished);
        assert_eq!(vec![0, 1], g.roll_off_players);
        // The rolloff announcement is in the logs.
        let rendered: String = logs
            .iter()
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect::<Vec<String>>()
            .join("");
        assert!(rendered.contains("tied score"));
        assert!(rendered.contains("tie breaker round"));
    }

    #[test]
    fn test_rolloff_skips_non_rolloff_players() {
        let mut g = Game::start(4).unwrap().0;
        g.scores = vec![WIN_SCORE, 5, WIN_SCORE, 5];
        g.current_turn = 3;
        // After next_player: wraps to 0, sees tie, starts rolloff with [0, 2].
        let _ = g.next_player();
        assert_eq!(vec![0, 2], g.roll_off_players);
        // Player 0's turn now (rolloff participant). Keep some brains.
        g.round_brains = 1;
        let _ = g.keep(0).unwrap();
        // After player 0 keeps, next_player skips 1 (not in rolloff) and starts
        // player 2's turn.
        assert_eq!(2, g.current_turn);
        assert!(!g.finished);
    }

    #[test]
    fn test_rolloff_resolves_when_unique_leader() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE, WIN_SCORE, 5];
        g.roll_off_players = vec![0, 1];
        // Player 0 keeps 1 brain, player 1 keeps 0 brains, then wrap to 0:
        // 0 has WIN_SCORE+1, 1 has WIN_SCORE -> unique leader, finished.
        g.current_turn = 0;
        g.round_brains = 1;
        let _ = g.keep(0).unwrap();
        // Now player 1's turn.
        assert_eq!(1, g.current_turn);
        g.round_brains = 0;
        let _ = g.keep(1).unwrap();
        // Wrap to 0 -> check leaders -> 0 leads alone -> finished.
        assert!(g.finished);
    }

    #[test]
    fn test_placings_standard_competition_ties() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![10, 10, 5];
        assert_eq!(vec![1, 1, 3], g.placings());
        g.scores = vec![10, 5, 10];
        assert_eq!(vec![1, 3, 1], g.placings());
        g.scores = vec![5, 10, 10];
        assert_eq!(vec![3, 1, 1], g.placings());
        g.scores = vec![10, 10, 10];
        assert_eq!(vec![1, 1, 1], g.placings());
        g.scores = vec![10, 7, 5];
        assert_eq!(vec![1, 2, 3], g.placings());
    }

    #[test]
    fn test_command_roll_and_keep() {
        let (mut g, _) = Game::start(2).unwrap();
        let p = vec![];
        let current = g.current_turn;
        // `roll` should work for the current player and stay on their turn
        // (unless a 3-shotgun bust advances play - exercise it regardless).
        let resp = g.command(current, "roll", &p).unwrap();
        assert!(!resp.logs.is_empty());
        // `keep` should also work (either still current's turn, or the new
        // current's turn after a bust). Find whose turn it is and keep.
        let keeper = g.current_turn;
        let resp = g.command(keeper, "keep", &p).unwrap();
        assert!(!resp.logs.is_empty());
        assert_ne!(keeper, g.current_turn);
    }

    #[test]
    fn test_command_wrong_player_errors() {
        let (mut g, _) = Game::start(2).unwrap();
        let p = vec![];
        let current = g.current_turn;
        let other = 1 - current;
        assert!(g.command(other, "keep", &p).is_err());
        assert!(g.command(other, "roll", &p).is_err());
    }

    #[test]
    fn test_command_unknown_input_errors() {
        let (mut g, _) = Game::start(2).unwrap();
        let p = vec![];
        let current = g.current_turn;
        assert!(g.command(current, "fly", &p).is_err());
    }

    #[test]
    fn test_command_after_finished_errors() {
        let (mut g, _) = Game::start(2).unwrap();
        g.finished = true;
        let p = vec![];
        assert!(g.command(0, "roll", &p).is_err());
        assert!(g.command(0, "keep", &p).is_err());
    }

    #[test]
    fn test_cup_refill_returns_kept_to_cup() {
        let mut g = Game::start(2).unwrap().0;
        // Empty cup with some kept dice; rolling should trigger a refill.
        g.cup = vec![];
        g.kept = vec![
            DiceResult {
                dice: Dice {
                    colour: Colour::Green,
                },
                face: Face::Brain,
            },
            DiceResult {
                dice: Dice {
                    colour: Colour::Yellow,
                },
                face: Face::Shotgun,
            },
            DiceResult {
                dice: Dice {
                    colour: Colour::Red,
                },
                face: Face::Footprints,
            },
        ];
        g.current_roll = vec![];
        g.round_brains = 1;
        g.round_shotguns = 1;
        let logs = g.roll();
        // Refill announcement is logged.
        let refill_logged = logs
            .iter()
            .any(|l| brdgme_markup::to_string(&l.content).contains("Not enough dice remaining"));
        assert!(refill_logged);
        // 3 kept dice returned to cup, 3 taken for the roll; the new roll's 3
        // outcomes are partitioned between kept (brains+shotguns) and
        // current_roll (footprints).
        assert_eq!(ROLL_DICE_COUNT, g.kept.len() + g.current_roll.len());
    }

    #[test]
    fn test_pub_state_captures_rendered_fields() {
        let g = Game::start(2).unwrap().0;
        let ps = g.pub_state();
        assert_eq!(g.players, ps.players);
        assert_eq!(g.current_turn, ps.current_turn);
        assert_eq!(g.scores, ps.scores);
        assert_eq!(g.cup, ps.cup);
        assert_eq!(g.current_roll, ps.current_roll);
        assert_eq!(g.kept, ps.kept);
        assert_eq!(g.round_brains, ps.round_brains);
        assert_eq!(g.round_shotguns, ps.round_shotguns);
        assert_eq!(g.finished, ps.finished);
    }
}
