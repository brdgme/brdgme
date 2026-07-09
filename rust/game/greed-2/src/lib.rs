use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_color as color;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;
use rand::prelude::*;
use std::sync::LazyLock;

use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 6;
pub const DICE_COUNT: usize = 6;
pub const WIN_SCORE: i32 = 5000;

#[repr(usize)]
#[derive(Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Die {
    #[default]
    Dollar = 0,
    G = 1,
    R = 2,
    E1 = 3,
    E2 = 4,
    D = 5,
}

pub const DIE_FACES: [Die; 6] = [Die::Dollar, Die::G, Die::R, Die::E1, Die::E2, Die::D];

impl Die {
    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn from_idx(i: usize) -> Die {
        DIE_FACES[i]
    }

    pub fn name(self) -> &'static str {
        match self {
            Die::Dollar => "$",
            Die::G => "G",
            Die::R => "R",
            Die::E1 => "E",
            Die::E2 => "e",
            Die::D => "D",
        }
    }

    pub fn color(self) -> color::Color {
        match self {
            Die::Dollar => color::GREY,
            Die::G => color::YELLOW,
            Die::R => color::RED,
            Die::E1 => color::BLACK,
            Die::E2 => color::GREEN,
            Die::D => color::CYAN,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Score {
    pub dice: Vec<Die>,
    pub value: i32,
}

pub static SCORES: LazyLock<Vec<Score>> = LazyLock::new(|| {
    vec![
        Score {
            dice: vec![Die::Dollar; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::G; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::R; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::E1; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::E2; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::D; 6],
            value: 5000,
        },
        Score {
            dice: vec![Die::D; 4],
            value: 1000,
        },
        Score {
            dice: vec![Die::Dollar, Die::G, Die::R, Die::E1, Die::E2, Die::D],
            value: 1000,
        },
        Score {
            dice: vec![Die::Dollar; 3],
            value: 600,
        },
        Score {
            dice: vec![Die::G; 3],
            value: 500,
        },
        Score {
            dice: vec![Die::R; 3],
            value: 400,
        },
        Score {
            dice: vec![Die::E1; 3],
            value: 300,
        },
        Score {
            dice: vec![Die::E2; 3],
            value: 300,
        },
        Score {
            dice: vec![Die::D],
            value: 100,
        },
        Score {
            dice: vec![Die::G],
            value: 50,
        },
    ]
});

pub fn scores() -> &'static [Score] {
    &SCORES
}

pub fn dice_in_dice(search: &[Die], in_dice: &[Die]) -> Option<Vec<Die>> {
    let mut in_counts = [0usize; 6];
    for d in in_dice {
        in_counts[d.idx()] += 1;
    }
    let mut search_counts = [0usize; 6];
    for d in search {
        search_counts[d.idx()] += 1;
    }
    let mut remaining: Vec<Die> = vec![];
    for (i, &s) in search_counts.iter().enumerate() {
        if s > in_counts[i] {
            return None;
        }
        for _ in 0..(in_counts[i] - s) {
            remaining.push(Die::from_idx(i));
        }
    }
    Some(remaining)
}

pub fn dice_equals(a: &[Die], b: &[Die]) -> bool {
    match dice_in_dice(a, b) {
        Some(r) => r.is_empty(),
        None => false,
    }
}

pub fn available_scores(dice: &[Die]) -> Vec<Score> {
    scores()
        .iter()
        .filter(|s| dice_in_dice(&s.dice, dice).is_some())
        .cloned()
        .collect()
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub first_player: usize,
    pub current_player: usize,
    pub scores: Vec<i32>,
    pub turn_score: i32,
    pub remaining_dice: Vec<Die>,
    pub taken_this_roll: bool,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub current_player: usize,
    pub first_player: usize,
    pub scores: Vec<i32>,
    pub turn_score: i32,
    pub remaining_dice: Vec<Die>,
    pub finished: bool,
    pub placings: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
}

impl Game {
    pub fn can_score(&self, player: usize) -> bool {
        player == self.current_player
            && !self.finished()
            && !available_scores(&self.remaining_dice).is_empty()
    }

    pub fn can_roll(&self, player: usize) -> bool {
        player == self.current_player && self.taken_this_roll && !self.finished()
    }

    pub fn can_done(&self, player: usize) -> bool {
        player == self.current_player && !self.finished()
    }

    /// Direct port of Go `IsFinished`: the round closes when play returns to
    /// the first player AND someone has reached the target score.
    fn finished(&self) -> bool {
        self.current_player == self.first_player && self.scores.iter().any(|&s| s >= WIN_SCORE)
    }

    fn random_dice(rng: &mut GameRng, n: usize) -> Vec<Die> {
        let mut dice: Vec<Die> = (0..n)
            .map(|_| DIE_FACES[rng.random_range(0..6)])
            .collect();
        dice.sort();
        dice
    }

    /// Handles a "no scoring dice" bust: logs the loss and advances play to
    /// the next player.
    fn bust(&mut self) -> Vec<Log> {
        let logs = vec![Log::public(vec![
            N::Player(self.current_player),
            N::text(" rolled no scoring dice and lost "),
            N::Bold(vec![N::text(self.turn_score.to_string())]),
            N::text(" points!"),
        ])];
        self.current_player = (self.current_player + 1) % self.players;
        logs
    }

    /// Begins the current player's turn: resets turn score, rolls the dice,
    /// and cascades through any "no scoring dice" busts until a roll leaves
    /// scoring dice or the game finishes.
    fn start_turn(&mut self) -> Vec<Log> {
        let mut logs: Vec<Log> = vec![];
        loop {
            logs.push(Log::public(vec![
                N::text("It is now "),
                N::Player(self.current_player),
                N::text("'s turn"),
            ]));
            self.turn_score = 0;
            self.taken_this_roll = false;
            self.remaining_dice = Self::random_dice(&mut self.rng, DICE_COUNT);
            logs.push(Log::public(vec![
                N::Player(self.current_player),
                N::text(" rolled "),
                render::render_dice(&self.remaining_dice, " "),
            ]));
            if available_scores(&self.remaining_dice).is_empty() {
                logs.extend(self.bust());
                if self.finished() {
                    break;
                }
                continue;
            }
            break;
        }
        logs
    }

    pub fn score(&mut self, player: usize, dice: &[Die]) -> Result<Vec<Log>, GameError> {
        let _ = player;
        let value = scores()
            .iter()
            .find(|s| dice_equals(dice, &s.dice))
            .map(|s| s.value)
            .ok_or_else(|| GameError::invalid_input("That doesn't score any points"))?;
        let remaining = match dice_in_dice(dice, &self.remaining_dice) {
            Some(r) => r,
            None => {
                return Err(GameError::invalid_input("You don't have those dice"));
            }
        };
        self.turn_score += value;
        self.taken_this_roll = true;
        self.remaining_dice = remaining;
        Ok(vec![Log::public(vec![
            N::Player(self.current_player),
            N::text(" scored "),
            render::render_dice(dice, ""),
            N::text(" for "),
            N::Bold(vec![N::text(value.to_string())]),
            N::text(" points"),
        ])])
    }

    pub fn player_roll(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_roll(player) {
            return Err(GameError::invalid_input("can't play at the moment"));
        }
        self.taken_this_roll = false;
        let n = if self.remaining_dice.is_empty() {
            DICE_COUNT
        } else {
            self.remaining_dice.len()
        };
        let mut logs: Vec<Log> = vec![];
        self.remaining_dice = Self::random_dice(&mut self.rng, n);
        logs.push(Log::public(vec![
            N::Player(self.current_player),
            N::text(" rolled "),
            render::render_dice(&self.remaining_dice, " "),
        ]));
        if available_scores(&self.remaining_dice).is_empty() {
            logs.extend(self.bust());
            if !self.finished() {
                logs.extend(self.start_turn());
            }
        }
        Ok(logs)
    }

    pub fn done(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_done(player) {
            return Err(GameError::invalid_input("can't call done at the moment"));
        }
        let mut logs: Vec<Log> = vec![];
        while let Some(s) = scores()
            .iter()
            .find(|s| dice_in_dice(&s.dice, &self.remaining_dice).is_some())
        {
            logs.extend(self.score(player, &s.dice.clone())?);
        }
        logs.push(Log::public(vec![
            N::Player(self.current_player),
            N::text(" took "),
            N::Bold(vec![N::text(self.turn_score.to_string())]),
            N::text(" points, now on "),
            N::Bold(vec![N::text(
                (self.scores[player] + self.turn_score).to_string(),
            )]),
        ]));
        self.scores[player] += self.turn_score;
        self.current_player = (self.current_player + 1) % self.players;
        if !self.finished() {
            logs.extend(self.start_turn());
        }
        Ok(logs)
    }

    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players).map(|p| vec![self.scores[p]]).collect();
        gen_placings(&metrics)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let mut rng = GameRng::seed_from_u64(seed);
        let current_player = rng.random_range(0..players);
        let mut g = Game {
            players,
            first_player: current_player,
            current_player,
            scores: vec![0; players],
            rng,
            ..Game::default()
        };
        let logs = g.start_turn();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            current_player: self.current_player,
            first_player: self.first_player,
            scores: self.scores.clone(),
            turn_score: self.turn_score,
            remaining_dice: self.remaining_dice.clone(),
            finished: self.finished(),
            placings: if self.finished() {
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
                value: Command::Score { dice },
                ..
            }) => {
                let logs = self.score(player, &dice)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: true,
                    remaining_input: remaining.to_string(),
                })
            }
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
                value: Command::Done,
                ..
            }) => {
                let logs = self.done(player)?;
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
        vec![2, 3, 4, 5, 6]
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

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    #[test]
    fn test_game() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![
            Die::Dollar,
            Die::Dollar,
            Die::Dollar,
            Die::E1,
            Die::E2,
            Die::D,
        ];
        let p = vec![];
        g.command(g.current_player, "score $$$", &p).unwrap();
    }

    #[test]
    fn test_done_takes_remaining_scoring_dice() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        g.remaining_dice = vec![Die::G, Die::G, Die::G, Die::G, Die::R, Die::D];
        let p = vec![];
        g.command(current, "done", &p).unwrap();
        assert_eq!(
            650, g.scores[current],
            "We should have scored GGG, D and G for 650 points"
        );
    }

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2, 3, 4, 5, 6], Game::player_counts());
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(7, 1).is_err());
        assert!(Game::start(2, 1).is_ok());
        assert!(Game::start(6, 1).is_ok());
    }

    #[test]
    fn test_dice_in_dice() {
        assert!(dice_in_dice(&[Die::D], &[Die::D]).is_some());
        assert!(dice_in_dice(&[Die::D, Die::D], &[Die::D]).is_none());
        let remaining = dice_in_dice(&[Die::D], &[Die::D, Die::G, Die::D]).unwrap();
        assert_eq!(vec![Die::G, Die::D], remaining);
        assert!(dice_equals(&[Die::D, Die::D], &[Die::D, Die::D]));
        assert!(!dice_equals(&[Die::D, Die::D], &[Die::D]));
        assert!(dice_equals(
            &[Die::Dollar, Die::G, Die::R, Die::E1, Die::E2, Die::D],
            &[Die::D, Die::E2, Die::E1, Die::R, Die::G, Die::Dollar],
        ));
    }

    #[test]
    fn test_available_scores() {
        assert!(available_scores(&[Die::R, Die::R]).is_empty());
        let dollar3 = available_scores(&[Die::Dollar, Die::Dollar, Die::Dollar]);
        assert!(dollar3.iter().any(|s| s.value == 600));
        let straight = available_scores(&[Die::Dollar, Die::G, Die::R, Die::E1, Die::E2, Die::D]);
        assert!(straight.iter().any(|s| s.value == 1000));
        let six_d = available_scores(&[Die::D; 6]);
        assert!(six_d.iter().any(|s| s.value == 5000));
        assert!(six_d.iter().any(|s| s.value == 1000));
    }

    #[test]
    fn test_score_accumulates() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        g.remaining_dice = vec![Die::Dollar, Die::Dollar, Die::Dollar, Die::D];
        g.score(current, &[Die::Dollar, Die::Dollar, Die::Dollar])
            .unwrap();
        assert_eq!(600, g.turn_score);
        assert_eq!(vec![Die::D], g.remaining_dice);
        assert!(g.taken_this_roll);
    }

    #[test]
    fn test_score_no_score_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![Die::R, Die::R];
        let err = g.score(g.current_player, &[Die::R, Die::R]).unwrap_err();
        assert!(format!("{err}").contains("doesn't score any points"));
    }

    #[test]
    fn test_score_not_in_dice_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![Die::D];
        let err = g
            .score(g.current_player, &[Die::Dollar, Die::Dollar, Die::Dollar])
            .unwrap_err();
        assert!(format!("{err}").contains("don't have those dice"));
    }

    #[test]
    fn test_can_commands() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        let other = 1 - current;
        // No scoring dice -> can_score false, done true, roll false.
        g.remaining_dice = vec![Die::R, Die::R];
        assert!(!g.can_score(current));
        assert!(g.can_done(current));
        assert!(!g.can_roll(current));
        // Scoring dice but not yet taken this roll -> can_score true, roll false.
        g.remaining_dice = vec![Die::D, Die::R, Die::R];
        assert!(g.can_score(current));
        assert!(!g.can_roll(current));
        // After scoring, taken_this_roll true -> can_roll true.
        g.score(current, &[Die::D]).unwrap();
        assert!(g.can_roll(current));
        // Wrong player cannot act.
        assert!(!g.can_score(other));
        assert!(!g.can_roll(other));
        assert!(!g.can_done(other));
    }

    #[test]
    fn test_finished_and_placings() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Round only closes when play returns to first player with a score >= 5000.
        g.scores = vec![5000, 3000, 1000];
        g.current_player = g.first_player + 1;
        assert!(!g.finished());
        g.current_player = g.first_player;
        assert!(g.finished());
        assert_eq!(vec![1, 2, 3], g.placings());
        // Standard-competition tie ranking (Rust gen_placings): [1, 1, 3].
        g.scores = vec![5000, 5000, 1000];
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_start_rolls_and_logs() {
        let (g, logs) = Game::start(2, 1).unwrap();
        assert!(!g.remaining_dice.is_empty());
        assert_eq!(DICE_COUNT, g.remaining_dice.len());
        // At least the "It is now X's turn" and "X rolled ..." logs.
        assert!(logs.len() >= 2);
    }

    #[test]
    fn test_full_turn_flow() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        let p = players(2);
        // Score three dollars; the leftover R,R,E1 has no scoring combo.
        g.remaining_dice = vec![
            Die::Dollar,
            Die::Dollar,
            Die::Dollar,
            Die::R,
            Die::R,
            Die::E1,
        ];
        g.command(current, "score $$$", &p).unwrap();
        assert_eq!(600, g.turn_score);
        assert_eq!(vec![Die::R, Die::R, Die::E1], g.remaining_dice);
        assert!(g.can_roll(current));
        // done banks 600 (no remaining scoring dice to auto-take).
        let before = g.scores[current];
        let resp = g.command(current, "done", &p).unwrap();
        assert_eq!(before + 600, g.scores[current]);
        // Turn passed to the other player. Asserted via the logs instead of
        // current_player: the other player's automatic opening roll can bust,
        // cascading the turn straight back to the original player.
        let other = 1 - current;
        assert!(
            resp.logs
                .iter()
                .any(|l| l.content.contains(&N::Player(other)))
        );
    }

    #[test]
    fn test_score_case_insensitive_e1_e2_collision() {
        // Die::E1's name is "E" and Die::E2's name is "e", so score_dice_parser
        // builds token "EEE" for the E1 triple and "eee" for the E2 triple.
        // Token::parse compares case-insensitively (UniCase), so both tokens
        // match either "EEE" or "eee" input, and OneOf::parse is first-Ok-wins.
        // scores() lists the E1 triple before the E2 triple at every tier, so
        // it wins regardless of the case actually typed - this is parity with
        // the Go original, not a regression. This test pins that resolution so
        // a future reordering of scores() is a deliberate, test-visible change
        // rather than a silent flip in which physical dice get consumed.
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        g.remaining_dice = vec![Die::E1, Die::E1, Die::E1, Die::E2, Die::E2, Die::E2];
        let p = vec![];
        g.command(current, "score eee", &p).unwrap();
        // The E1 triple (listed first in scores()) was consumed, leaving E2.
        assert_eq!(vec![Die::E2, Die::E2, Die::E2], g.remaining_dice);
    }

    #[test]
    fn test_done_auto_scores_remaining_combos() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        let p = vec![];
        // 3x$ (600) plus 3xR (400) -> done auto-takes both for 1000.
        g.remaining_dice = vec![
            Die::Dollar,
            Die::Dollar,
            Die::Dollar,
            Die::R,
            Die::R,
            Die::R,
        ];
        g.command(current, "done", &p).unwrap();
        assert_eq!(1000, g.scores[current]);
    }
}
