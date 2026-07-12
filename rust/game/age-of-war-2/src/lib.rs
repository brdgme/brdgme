use std::collections::HashSet;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

pub mod castle;
mod command;
mod render;

use castle::{ALL_CLANS, ALL_DICE, Clan, Die};
use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 6;

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,

    /// Indexed by castle (see `castle::castles()`).
    pub conquered: Vec<bool>,
    pub castle_owners: Vec<Option<usize>>,

    pub currently_attacking: Option<usize>,
    /// Line indices completed on the currently-attacked castle this turn.
    pub completed_lines: HashSet<usize>,
    pub current_roll: Vec<Die>,

    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub current_player: usize,
    pub conquered: Vec<bool>,
    pub castle_owners: Vec<Option<usize>>,
    pub currently_attacking: Option<usize>,
    pub completed_lines: Vec<usize>,
    pub current_roll: Vec<Die>,
    pub scores: Vec<u32>,
}

/// No hidden information in this game, so PlayerState is just PubState plus
/// the viewing player (mirrors Go's `PlayerRender` delegating to `PubRender`).
#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
}

impl Game {
    /// Port of Game.CanAttack (attack_command.go).
    pub fn can_attack(&self, player: usize) -> bool {
        self.current_player == player && self.currently_attacking.is_none()
    }

    /// Port of Game.CanLine (line_command.go).
    pub fn can_line(&self, player: usize) -> bool {
        self.current_player == player && self.currently_attacking.is_some()
    }

    /// Port of Game.CanRoll (roll_command.go).
    pub fn can_roll(&self, player: usize) -> bool {
        self.current_player == player
    }

    /// Port of Game.ClanConquered (game.go). Only the bool is used by any
    /// caller when it returns false; the player value on a false result
    /// mirrors Go's (possibly stale) return exactly.
    pub fn clan_conquered(&self, clan: Clan) -> (bool, Option<usize>) {
        let all_castles = castle::castles();
        let mut player: Option<usize> = None;
        for (i, c) in all_castles.iter().enumerate() {
            if c.clan != clan {
                continue;
            }
            if !self.conquered[i] {
                return (false, None);
            }
            match player {
                None => player = self.castle_owners[i],
                Some(p) => {
                    if self.castle_owners[i] != Some(p) {
                        return (false, player);
                    }
                }
            }
        }
        (true, player)
    }

    /// Port of Game.Scores (game.go).
    pub fn scores(&self) -> Vec<u32> {
        let all_castles = castle::castles();
        let mut scores = vec![0u32; self.players];
        let clan_status: Vec<(bool, Option<usize>)> = ALL_CLANS
            .iter()
            .map(|&cl| self.clan_conquered(cl))
            .collect();
        for (ci, &clan) in ALL_CLANS.iter().enumerate() {
            if let (true, Some(by)) = clan_status[ci] {
                scores[by] += clan.set_points();
            }
        }
        for (idx, c) in all_castles.iter().enumerate() {
            if !self.conquered[idx] {
                continue;
            }
            let clan_idx = ALL_CLANS.iter().position(|&cl| cl == c.clan).unwrap();
            if clan_status[clan_idx].0 {
                continue;
            }
            if let Some(owner) = self.castle_owners[idx] {
                scores[owner] += c.points;
            }
        }
        scores
    }

    fn calc_placings(&self) -> Vec<usize> {
        let mut clan_counts = vec![0i32; self.players];
        for &clan in &ALL_CLANS {
            if let (true, Some(by)) = self.clan_conquered(clan) {
                clan_counts[by] += 1;
            }
        }
        let scores = self.scores();
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![scores[p] as i32, clan_counts[p]])
            .collect();
        gen_placings(&metrics)
    }

    /// Port of Game.StartTurn (game.go).
    fn start_turn(&mut self) -> Log {
        self.currently_attacking = None;
        self.completed_lines.clear();
        self.roll(7)
    }

    /// Port of Game.NextTurn (game.go).
    fn next_turn(&mut self) -> Log {
        self.current_player = (self.current_player + 1) % self.players;
        self.start_turn()
    }

    /// Port of Game.Roll (dice.go), including its public log message.
    fn roll(&mut self, n: usize) -> Log {
        self.current_roll = (0..n)
            .map(|_| ALL_DICE[self.rng.random_range(0..6usize)])
            .collect();
        let mut content: Vec<N> = vec![N::Player(self.current_player), N::text(" rolled  ")];
        for (i, d) in self.current_roll.iter().enumerate() {
            if i > 0 {
                content.push(N::text("  "));
            }
            content.push(d.render());
        }
        Log::public(content)
    }

    /// Port of Game.FailedAttackMessage (game.go).
    fn failed_attack_message(&self) -> Log {
        let target = match self.currently_attacking {
            Some(idx) => castle::castles()[idx].render_name(),
            None => N::text("anything"),
        };
        Log::public(vec![
            N::Player(self.current_player),
            N::text(" failed to conquer "),
            target,
        ])
    }

    /// Port of Game.CheckEndOfTurn (game.go). Returns (ended_via_conquest,
    /// logs) exactly like the Go bool/log-slice pair.
    fn check_end_of_turn(&mut self) -> (bool, Vec<Log>) {
        let mut logs: Vec<Log> = vec![];
        let all_castles = castle::castles();
        if let Some(idx) = self.currently_attacking {
            let c = &all_castles[idx];
            let lines = c.calc_lines(self.conquered[idx]);

            let all_lines = (0..lines.len()).all(|l| self.completed_lines.contains(&l));
            if all_lines {
                let was_conquered = self.conquered[idx];
                let prior_owner = self.castle_owners[idx];
                let mut content: Vec<N> = vec![
                    N::Player(self.current_player),
                    N::text(" conquered the castle "),
                    c.render_name(),
                ];
                if was_conquered {
                    content.push(N::text(" from "));
                    content.push(N::Player(
                        prior_owner.expect("conquered castle has an owner"),
                    ));
                }
                logs.push(Log::public(content));
                self.conquered[idx] = true;
                self.castle_owners[idx] = Some(self.current_player);
                if self.clan_conquered(c.clan).0 {
                    logs.push(Log::public(vec![
                        N::Player(self.current_player),
                        N::text(" conquered the clan "),
                        c.clan.render(),
                    ]));
                }
                logs.push(self.next_turn());
                return (true, logs);
            }

            let mut req_dice = 0usize;
            let num_dice = self.current_roll.len();
            let mut can_afford_line = false;
            for (i, l) in lines.iter().enumerate() {
                if self.completed_lines.contains(&i) {
                    continue;
                }
                req_dice += l.min_dice();
                if req_dice > num_dice {
                    logs.push(self.failed_attack_message());
                    logs.push(self.next_turn());
                    return (false, logs);
                }
                if l.can_afford(&self.current_roll).0 {
                    can_afford_line = true;
                }
            }

            if req_dice == num_dice && !can_afford_line {
                logs.push(self.failed_attack_message());
                logs.push(self.next_turn());
                return (false, logs);
            }
        } else {
            for (i, c) in all_castles.iter().enumerate() {
                if self.conquered[i] && self.castle_owners[i] == Some(self.current_player) {
                    continue;
                }
                if self.clan_conquered(c.clan).0 {
                    continue;
                }
                let mut min_dice = c.min_dice();
                if self.conquered[i] {
                    min_dice += 1;
                }
                if min_dice <= self.current_roll.len() {
                    return (false, logs);
                }
            }
            logs.push(self.failed_attack_message());
            logs.push(self.next_turn());
            return (false, logs);
        }
        (false, logs)
    }

    /// Port of Game.Attack (attack_command.go).
    pub fn attack(
        &mut self,
        player: usize,
        castle_idx: usize,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        if !self.can_attack(player) {
            return Err(GameError::invalid_input(
                "unable to attack a castle right now",
            ));
        }
        let all_castles = castle::castles();
        if castle_idx >= all_castles.len() {
            return Err(GameError::invalid_input("that is not a valid castle"));
        }
        if self.conquered[castle_idx] && self.castle_owners[castle_idx] == Some(player) {
            return Err(GameError::invalid_input(
                "you have already conquered that castle",
            ));
        }
        if self.clan_conquered(all_castles[castle_idx].clan).0 {
            return Err(GameError::invalid_input("that clan is already conquered"));
        }
        self.currently_attacking = Some(castle_idx);
        let mut content: Vec<N> = vec![N::Player(player), N::text(" is attacking:\n")];
        content.push(render::render_castle(&self.pub_state(), castle_idx, &[]));
        let mut logs = vec![Log::public(content)];
        let (_, end_logs) = self.check_end_of_turn();
        logs.extend(end_logs);
        Ok(CommandResponse {
            logs,
            can_undo: true,
            remaining_input: remaining.to_string(),
        })
    }

    /// Port of Game.Line (line_command.go). `line` is 1-based, as typed by
    /// the player.
    pub fn line_action(
        &mut self,
        player: usize,
        line: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        if !self.can_line(player) {
            return Err(GameError::invalid_input(
                "unable to complete a line right now",
            ));
        }
        let idx = self
            .currently_attacking
            .expect("can_line implies currently_attacking");
        let all_castles = castle::castles();
        let lines = all_castles[idx].calc_lines(self.conquered[idx]);
        let line0 = line - 1;
        if line0 < 0 || line0 as usize >= lines.len() {
            return Err(GameError::invalid_input("that is not a valid line"));
        }
        let line0 = line0 as usize;
        if self.completed_lines.contains(&line0) {
            return Err(GameError::invalid_input(
                "that line has already been completed",
            ));
        }
        let (can_afford, using) = lines[line0].can_afford(&self.current_roll);
        if !can_afford {
            return Err(GameError::invalid_input("cannot afford that line"));
        }
        let plural = if using == 1 { "die" } else { "dice" };
        let mut content: Vec<N> = vec![N::Player(player), N::text(" completed ")];
        content.extend(lines[line0].render_row());
        content.push(N::text(" with "));
        content.push(N::Bold(vec![N::text(using.to_string())]));
        content.push(N::text(format!(" {}", plural)));
        let mut logs = vec![Log::public(content)];
        self.completed_lines.insert(line0);
        let (is_end, end_logs) = self.check_end_of_turn();
        logs.extend(end_logs);
        if !is_end {
            let roll_log = self.roll(self.current_roll.len() - using);
            logs.push(roll_log);
            let (_, end_logs2) = self.check_end_of_turn();
            logs.extend(end_logs2);
        }
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input: remaining.to_string(),
        })
    }

    /// Port of Game.RollForPlayer (roll_command.go).
    pub fn roll_action(
        &mut self,
        player: usize,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        if !self.can_roll(player) {
            return Err(GameError::invalid_input("unable to roll right now"));
        }
        let n = self.current_roll.len().saturating_sub(1);
        let mut logs = vec![self.roll(n)];
        let (_, end_logs) = self.check_end_of_turn();
        logs.extend(end_logs);
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input: remaining.to_string(),
        })
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
        let n_castles = castle::castles().len();
        let mut g = Game {
            players,
            conquered: vec![false; n_castles],
            castle_owners: vec![None; n_castles],
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        let log = g.start_turn();
        Ok((g, vec![log]))
    }

    fn status(&self) -> Status {
        if self.conquered.iter().all(|&c| c) {
            Status::Finished {
                placings: self.calc_placings(),
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
        let mut completed: Vec<usize> = self.completed_lines.iter().cloned().collect();
        completed.sort_unstable();
        PubState {
            players: self.players,
            current_player: self.current_player,
            conquered: self.conquered.clone(),
            castle_owners: self.castle_owners.clone(),
            currently_attacking: self.currently_attacking,
            completed_lines: completed,
            current_roll: self.current_roll.clone(),
            scores: self.scores(),
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
            None => return Err(GameError::invalid_input("not your turn")),
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                value: Command::Attack { castle },
                remaining,
                ..
            }) => self.attack(player, castle, remaining),
            Ok(ParseOutput {
                value: Command::Line { line },
                remaining,
                ..
            }) => self.line_action(player, line, remaining),
            Ok(ParseOutput {
                value: Command::Roll,
                remaining,
                ..
            }) => self.roll_action(player, remaining),
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        self.scores().into_iter().map(|s| s as f32).collect()
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

    // Port of TestGame_New (game_test.go).
    #[test]
    fn test_game_new() {
        assert!(Game::start(3, 1).is_ok());
    }

    // Port of TestGame_Attack (attack_test.go).
    #[test]
    fn test_game_attack() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let p = players(2);
        let cur = g.current_player;
        assert!(g.command(cur, "attack azu", &p).is_ok());
    }

    #[test]
    fn player_counts_2_to_6() {
        for n in 2..=6 {
            assert!(Game::start(n, 1).is_ok(), "expected {} players to be ok", n);
        }
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(7, 1).is_err());
    }

    #[test]
    fn start_rolls_seven_dice_with_public_log() {
        let (g, logs) = Game::start(3, 1).unwrap();
        assert_eq!(7, g.current_roll.len());
        assert_eq!(1, logs.len());
        assert!(logs[0].public);
    }

    #[test]
    fn command_gating_can_attack_can_line_can_roll() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.can_attack(0));
        assert!(!g.can_line(0));
        assert!(g.can_roll(0));
        assert!(!g.can_attack(1));

        g.currently_attacking = Some(0);
        assert!(!g.can_attack(0));
        assert!(g.can_line(0));
        assert!(g.can_roll(0));
    }

    #[test]
    fn attack_wrong_player_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let other = 1 - g.current_player;
        let p = players(2);
        assert!(g.command(other, "attack azu", &p).is_err());
    }

    #[test]
    fn attack_own_conquered_castle_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.conquered[0] = true;
        g.castle_owners[0] = Some(g.current_player);
        assert!(
            g.attack(g.current_player, 0, "")
                .unwrap_err()
                .to_string()
                .contains("already conquered")
        );
    }

    #[test]
    fn attack_conquered_clan_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        // Conquer all of Oda's 4 castles (indices 0-3) for player 1, then
        // player 0 (current) should be unable to attack any of them.
        for i in 0..4 {
            g.conquered[i] = true;
            g.castle_owners[i] = Some(1);
        }
        assert!(
            g.attack(g.current_player, 0, "")
                .unwrap_err()
                .to_string()
                .contains("already conquered")
        );
    }

    #[test]
    fn attack_sets_currently_attacking_and_can_undo() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        let resp = g.attack(cur, 0, "").unwrap();
        assert!(resp.can_undo);
        assert_eq!(Some(0), g.currently_attacking);
    }

    #[test]
    fn line_completes_and_rerolls_remainder() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        // Odani (index 2) has a single 10-infantry line; give a roll that
        // affords it (3+3+3+1=10, using 4 dice) with 3 dice left over.
        g.currently_attacking = Some(2);
        g.current_roll = vec![
            Die::Inf3,
            Die::Inf3,
            Die::Inf3,
            Die::Inf1,
            Die::Archery,
            Die::Cavalry,
            Die::Daimyo,
        ];
        let resp = g.line_action(cur, 1, "").unwrap();
        assert!(!resp.can_undo);
        // The castle is fully conquered by completing its only line, so the
        // turn advances rather than rerolling a remainder.
        assert!(g.conquered[2]);
    }

    #[test]
    fn line_reroll_when_castle_not_fully_conquered() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        // Azuchi (index 0) has 3 lines: [Archery], [Cavalry,Cavalry], {Infantry:5}.
        // Complete the archery line (1 die) with 6 left, leaving 2 lines
        // (min dice 2 + 2 = 4) so the turn continues (not enough to end via
        // insufficient dice, and not all lines complete).
        g.currently_attacking = Some(0);
        g.current_roll = vec![
            Die::Archery,
            Die::Inf1,
            Die::Inf1,
            Die::Inf1,
            Die::Inf1,
            Die::Inf1,
            Die::Inf1,
        ];
        let resp = g.line_action(cur, 1, "").unwrap();
        assert!(!resp.can_undo);
        assert!(!g.conquered[0]);
        assert_eq!(6, g.current_roll.len());
    }

    #[test]
    fn roll_discards_one_die_and_can_undo_false() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        let before = g.current_roll.len();
        let resp = g.roll_action(cur, "").unwrap();
        assert!(!resp.can_undo);
        assert_eq!(before - 1, g.current_roll.len());
    }

    #[test]
    fn conquer_castle_transfers_ownership_and_advances_turn() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        g.currently_attacking = Some(2); // Odani, single {Infantry: 10} line
        g.current_roll = vec![Die::Inf3, Die::Inf3, Die::Inf3, Die::Inf1];
        let resp = g.line_action(cur, 1, "").unwrap();
        assert!(g.conquered[2]);
        assert_eq!(Some(cur), g.castle_owners[2]);
        assert_ne!(cur, g.current_player);
        assert!(resp.logs.iter().any(|l| l.public));
    }

    #[test]
    fn steal_adds_daimyo_line() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.conquered[2] = true; // Odani conquered by the other player
        g.castle_owners[2] = Some(1 - g.current_player);
        g.currently_attacking = Some(2);
        let lines = castle::castles()[2].calc_lines(true);
        assert_eq!(2, lines.len());
        assert_eq!(vec![Die::Daimyo], lines[1].symbols);
    }

    #[test]
    fn clan_conquered_and_set_scoring() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        // Conquer all 4 Oda castles for player 0.
        for i in 0..4 {
            g.conquered[i] = true;
            g.castle_owners[i] = Some(0);
        }
        let (conquered, by) = g.clan_conquered(Clan::Oda);
        assert!(conquered);
        assert_eq!(Some(0), by);
        let scores = g.scores();
        assert_eq!(10, scores[0]); // Oda set points, not individual castle points
    }

    #[test]
    fn scores_use_individual_points_when_clan_not_conquered() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.conquered[0] = true; // Azuchi, 3 points
        g.castle_owners[0] = Some(0);
        let scores = g.scores();
        assert_eq!(3, scores[0]);
    }

    #[test]
    fn failed_attack_passes_turn() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current_player;
        // Gifu (index 3): lines [Daimyo],[Archery],[Cavalry] => min dice 3.
        g.currently_attacking = Some(3);
        g.current_roll = vec![Die::Inf1, Die::Inf1]; // only 2 dice, can't afford
        let (ended, _logs) = g.check_end_of_turn();
        assert!(!ended);
        assert_ne!(cur, g.current_player);
    }

    #[test]
    fn finished_when_all_castles_conquered() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = castle::castles().len();
        for i in 0..n {
            g.conquered[i] = true;
            g.castle_owners[i] = Some(i % 2);
        }
        assert!(matches!(g.status(), Status::Finished { .. }));
    }

    #[test]
    fn placings_standard_competition_tie() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Give players 0 and 1 identical single-castle scores (Odani, 1pt),
        // player 2 nothing. Rust gen_placings is standard-competition, so a
        // tie for first is [1, 1, 3], not Go's compact-ordinal [1, 1, 2].
        g.conquered[2] = true;
        g.castle_owners[2] = Some(0);
        g.conquered[3] = true;
        g.castle_owners[3] = Some(1);
        let placings = g.calc_placings();
        assert_eq!(vec![1, 1, 3], placings);
    }

    #[test]
    fn pub_state_carries_full_public_info() {
        let (g, _) = Game::start(4, 1).unwrap();
        let ps = g.pub_state();
        assert_eq!(g.players, ps.players);
        assert_eq!(g.current_player, ps.current_player);
        assert_eq!(g.current_roll, ps.current_roll);
        assert_eq!(g.conquered, ps.conquered);
    }

    // Go source quirk (preserved, not "fixed"): none of CanAttack/CanLine/
    // CanRoll check IsFinished, so a finished game (all 14 castles conquered)
    // still accepts commands from the current player rather than rejecting
    // them. This mirrors brdgme-go/age_of_war_1 exactly.
    #[test]
    fn command_after_finished_still_accepted_go_quirk() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = castle::castles().len();
        for i in 0..n {
            g.conquered[i] = true;
            g.castle_owners[i] = Some(0);
        }
        assert!(matches!(g.status(), Status::Finished { .. }));
        let p = players(2);
        assert!(g.command(g.current_player, "roll", &p).is_ok());
    }
}
