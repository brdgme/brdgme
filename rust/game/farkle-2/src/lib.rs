use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_color::NamedColor;
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

/// Die face value, always 1..=6.
pub type Die = u8;

pub fn die_color(d: Die) -> NamedColor {
    match d {
        1 => NamedColor::Cyan,
        2 => NamedColor::Green,
        3 => NamedColor::Red,
        4 => NamedColor::Blue,
        5 => NamedColor::Yellow,
        6 => NamedColor::Purple,
        _ => NamedColor::Grey,
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Score {
    pub dice: Vec<Die>,
    pub value: i32,
}

/// Farkle scoring combinations. Order is irrelevant for matching - each
/// combination is a distinct multiset - but kept stable for tests.
pub static SCORES: LazyLock<Vec<Score>> = LazyLock::new(|| {
    vec![
        Score {
            dice: vec![5],
            value: 50,
        },
        Score {
            dice: vec![1],
            value: 100,
        },
        Score {
            dice: vec![2, 2, 2],
            value: 200,
        },
        Score {
            dice: vec![3, 3, 3],
            value: 300,
        },
        Score {
            dice: vec![4, 4, 4],
            value: 400,
        },
        Score {
            dice: vec![5, 5, 5],
            value: 500,
        },
        Score {
            dice: vec![6, 6, 6],
            value: 600,
        },
        Score {
            dice: vec![1, 1, 1],
            value: 1000,
        },
    ]
});

pub fn scores() -> &'static [Score] {
    &SCORES
}

/// Multiset subset check: returns the `in_dice` multiset minus `search` if
/// `search` is contained in `in_dice`, else `None`. Faithful port of Go
/// `libdie.DiceInDice` over die values 1..=6.
pub fn dice_in_dice(search: &[Die], in_dice: &[Die]) -> Option<Vec<Die>> {
    let mut in_counts = [0usize; 6];
    for &d in in_dice {
        if !(1..=6).contains(&d) {
            return None;
        }
        in_counts[(d - 1) as usize] += 1;
    }
    let mut search_counts = [0usize; 6];
    for &d in search {
        if !(1..=6).contains(&d) {
            return None;
        }
        search_counts[(d - 1) as usize] += 1;
    }
    let mut remaining: Vec<Die> = vec![];
    for (i, &s) in search_counts.iter().enumerate() {
        if s > in_counts[i] {
            return None;
        }
        for _ in 0..(in_counts[i] - s) {
            remaining.push((i + 1) as Die);
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
    /// Number of players in the game.
    pub players: usize,
    /// Index of the player currently taking their turn.
    pub current_player: usize,
    /// Index of the player who went first this game.
    pub first_player: usize,
    /// Banked score for each player.
    pub scores: Vec<i32>,
    /// Points accumulated in the current turn, not yet banked.
    pub turn_score: i32,
    /// Dice still available to roll this turn (values 1-6).
    pub remaining_dice: Vec<Die>,
    /// Whether the game has ended.
    pub finished: bool,
    /// Final standings once finished (empty while active).
    pub placings: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state; Farkle has no hidden information.
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
        player == self.current_player && self.taken_this_roll && !self.finished()
    }

    /// Direct port of Go `IsFinished`: the round closes when play returns to
    /// the first player AND someone has reached the target score.
    fn finished(&self) -> bool {
        self.current_player == self.first_player && self.scores.iter().any(|&s| s >= WIN_SCORE)
    }

    fn random_dice(rng: &mut GameRng, n: usize) -> Vec<Die> {
        let mut dice: Vec<Die> = (0..n).map(|_| rng.random_range(1..=6u8)).collect();
        dice.sort_unstable();
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
            render::render_dice(dice, " "),
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
    fn data_docs() -> String {
        include_str!("../DATA_DOCS.md").to_string()
    }
    fn basic_strategy() -> String {
        include_str!("../BASIC_STRATEGY.md").to_string()
    }
    fn advanced_strategy() -> String {
        include_str!("../ADVANCED_STRATEGY.md").to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    // 1:1 port of Go `TestGame`.
    #[test]
    fn test_game() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![1, 2, 3, 4, 5, 6];
        let p = vec![];
        g.command(g.current_player, "score 1", &p).unwrap();
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
        assert!(dice_in_dice(&[1], &[1]).is_some());
        assert!(dice_in_dice(&[1, 1], &[1]).is_none());
        let remaining = dice_in_dice(&[1], &[1, 2, 1]).unwrap();
        assert_eq!(vec![1, 2], remaining);
        assert!(dice_equals(&[1, 1], &[1, 1]));
        assert!(!dice_equals(&[1, 1], &[1]));
        // Port of Go libdie TestDiceInDice.
        let remaining = dice_in_dice(&[1, 1, 1], &[2, 4, 1, 3, 1, 1]).unwrap();
        assert!(dice_equals(&remaining, &[2, 4, 3]));
        let remaining = dice_in_dice(&[5], &[1, 4, 5, 5, 5, 3]).unwrap();
        assert!(dice_equals(&remaining, &[1, 4, 5, 5, 3]));
        assert!(dice_in_dice(&[6], &[1, 4, 5, 5, 5, 3]).is_none());
    }

    #[test]
    fn test_dice_in_dice_rejects_out_of_range() {
        // Persisted state could contain a corrupted die value (e.g. 0 or 7);
        // these must return None instead of panicking on underflow/OOB index.
        assert!(dice_in_dice(&[0], &[1]).is_none());
        assert!(dice_in_dice(&[1], &[0]).is_none());
        assert!(dice_in_dice(&[7], &[1]).is_none());
        assert!(dice_in_dice(&[1], &[7]).is_none());
    }

    #[test]
    fn test_available_scores() {
        assert!(available_scores(&[2, 3]).is_empty());
        let single_1 = available_scores(&[1, 2, 3]);
        assert!(single_1.iter().any(|s| s.value == 100));
        let triple_5 = available_scores(&[5, 5, 5, 2]);
        assert!(triple_5.iter().any(|s| s.value == 500));
        let triple_1 = available_scores(&[1, 1, 1]);
        assert!(triple_1.iter().any(|s| s.value == 1000));
        // Three 5s also contains two single 5s and one single 5.
        assert!(triple_5.iter().any(|s| s.value == 50));
    }

    #[test]
    fn test_score_accumulates() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        g.remaining_dice = vec![1, 5, 5, 5, 2, 3];
        g.score(current, &[5, 5, 5]).unwrap();
        assert_eq!(500, g.turn_score);
        assert_eq!(vec![1, 2, 3], g.remaining_dice);
        assert!(g.taken_this_roll);
    }

    #[test]
    fn test_score_no_score_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![1, 2];
        let err = g.score(g.current_player, &[2, 1]).unwrap_err();
        assert!(format!("{err}").contains("doesn't score any points"));
    }

    #[test]
    fn test_score_not_in_dice_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.remaining_dice = vec![1, 2];
        let err = g.score(g.current_player, &[5, 5, 5]).unwrap_err();
        assert!(format!("{err}").contains("don't have those dice"));
    }

    #[test]
    fn test_can_commands() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        let other = 1 - current;
        // No scoring dice -> can_score false, done false (no taken), roll false.
        g.remaining_dice = vec![2, 3];
        assert!(!g.can_score(current));
        assert!(!g.can_done(current));
        assert!(!g.can_roll(current));
        // Scoring dice but not yet scored this roll -> can_score true, roll/done false.
        g.remaining_dice = vec![1, 2, 3];
        assert!(g.can_score(current));
        assert!(!g.can_roll(current));
        assert!(!g.can_done(current));
        // After scoring, taken_this_roll true -> can_roll and can_done true.
        g.score(current, &[1]).unwrap();
        assert!(g.can_roll(current));
        assert!(g.can_done(current));
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
        // Score three 1s (1000); leftover 2,3,4 has no scoring combo.
        g.remaining_dice = vec![1, 1, 1, 2, 3, 4];
        g.command(current, "score 1 1 1", &p).unwrap();
        assert_eq!(1000, g.turn_score);
        assert_eq!(vec![2, 3, 4], g.remaining_dice);
        assert!(g.can_roll(current));
        // done banks 1000 (farkle does NOT auto-score remaining dice).
        let before = g.scores[current];
        let resp = g.command(current, "done", &p).unwrap();
        assert_eq!(before + 1000, g.scores[current]);
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
    fn test_done_banks_without_auto_scoring() {
        // Farkle-specific: done only banks the accumulated turn score; it does
        // NOT auto-score leftover dice (unlike greed-2). A single 5 left over
        // after scoring three 1s stays unscored.
        let (mut g, _) = Game::start(2, 1).unwrap();
        let current = g.current_player;
        let p = vec![];
        g.remaining_dice = vec![1, 1, 1, 5, 2, 3];
        g.command(current, "score 1 1 1", &p).unwrap();
        assert_eq!(1000, g.turn_score);
        let before = g.scores[current];
        g.command(current, "done", &p).unwrap();
        // Only the 1000 from the explicit score is banked, not the unused 50.
        assert_eq!(before + 1000, g.scores[current]);
    }
}
