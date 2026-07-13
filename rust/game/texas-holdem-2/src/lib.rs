//! `texas-holdem-2`: Rust port of `brdgme-go/texas_holdem_1`.
//!
//! See `card.rs` (ported from `brdgme-go/libcard`) and `poker.rs` (ported
//! from `brdgme-go/libpoker/hand.go`) for the card/hand-evaluation building
//! blocks. This module ports `texas_holdem.go` itself: game state, betting
//! rules, phase progression and status/points.

pub mod card;
mod command;
pub mod poker;
mod render;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::{Align as A, Node as N, Row, table_with_gap};

use card::Deck;
use command::Command;
use rand::RngExt;

const STARTING_MONEY: i32 = 100;
const STARTING_MINIMUM_BET: i32 = 10;
const HANDS_PER_BLINDS_INCREASE: i32 = 5;
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 9;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub current_dealer: usize,
    pub player_hands: Vec<Deck>,
    pub community_cards: Deck,
    pub deck: Deck,
    pub player_money: Vec<i32>,
    pub bets: Vec<i32>,
    pub folded_players: Vec<bool>,
    pub minimum_bet: i32,
    pub largest_raise: i32,
    pub hands_since_blinds_increase: i32,
    pub first_betting_player: usize,
    pub everyone_has_bet_once: bool,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub community_cards: Deck,
    pub pot: i32,
    pub current_dealer: usize,
    pub current_player: usize,
    pub player_money: Vec<i32>,
    pub bets: Vec<i32>,
    pub folded_players: Vec<bool>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub hand: Deck,
}

/// Port of `RenderCash`, used from log messages (see `render.rs` for the
/// version used by table rendering - same output, kept as separate helpers
/// mirroring the Go source's single `RenderCash` used from both contexts).
fn render_cash(amount: i32) -> N {
    render::cash(amount)
}

fn render_player_name(player: usize) -> N {
    N::Player(player)
}

impl Game {
    /// Port of `RemainingPlayers`.
    pub fn remaining_players(&self) -> Vec<usize> {
        (0..self.players)
            .filter(|&p| self.player_money[p] > 0 || self.bets[p] > 0)
            .collect()
    }

    /// Port of `ActivePlayers`.
    pub fn active_players(&self) -> Vec<usize> {
        self.remaining_players()
            .into_iter()
            .filter(|&p| !self.folded_players[p])
            .collect()
    }

    /// Port of `BettingPlayers`.
    pub fn betting_players(&self) -> Vec<usize> {
        self.active_players()
            .into_iter()
            .filter(|&p| self.player_money[p] > 0)
            .collect()
    }

    /// Port of `RequiringCallPlayers`.
    pub fn requiring_call_players(&self) -> Vec<usize> {
        let current_bet = self.current_bet();
        self.betting_players()
            .into_iter()
            .filter(|&p| self.bets[p] < current_bet)
            .collect()
    }

    fn next_active_player_num_from(&self, player_num: usize) -> usize {
        self.next_player_in_set(player_num, &self.active_players())
    }

    fn next_betting_player_num_from(&self, player_num: usize) -> usize {
        self.next_player_in_set(player_num, &self.betting_players())
    }

    fn next_remaining_player_num_from(&self, player_num: usize) -> usize {
        self.next_player_in_set(player_num, &self.remaining_players())
    }

    /// Port of `NextPlayerInSet`. Panics if `set` is empty, matching Go's
    /// `panic("No players in set")`.
    fn next_player_in_set(&self, player_num: usize, set: &[usize]) -> usize {
        assert!(!set.is_empty(), "No players in set");
        for i in 0..self.players {
            let next_player_num = (player_num + i + 1) % self.players;
            if set.contains(&next_player_num) {
                return next_player_num;
            }
        }
        panic!("Could not find any valid players");
    }

    /// Port of `BetUpTo`.
    fn bet_up_to(&mut self, player_num: usize, amount: i32) -> i32 {
        let bet_amount = amount.min(self.player_money[player_num]);
        self.bet(player_num, bet_amount)
            .expect("BetUpTo always bets an affordable amount");
        bet_amount
    }

    /// Port of `Bet`.
    fn bet(&mut self, player_num: usize, amount: i32) -> Result<(), GameError> {
        if self.player_money[player_num] < amount {
            return Err(GameError::invalid_input("Not enough money"));
        }
        let raise_amount = self.bets[player_num] + amount - self.current_bet();
        self.bets[player_num] += amount;
        self.player_money[player_num] -= amount;
        self.largest_raise = raise_amount.max(self.largest_raise);
        Ok(())
    }

    /// Port of `CanCheck`.
    pub fn can_check(&self, player: usize) -> bool {
        let current_bet = self.current_bet();
        self.current_player == player && self.bets[player] == current_bet && !self.is_finished()
    }

    /// Port of `Check`.
    pub fn check(&mut self, player_num: usize) -> Result<Vec<Log>, GameError> {
        if self.is_finished() || self.current_player != player_num {
            return Err(GameError::invalid_input("Not your turn"));
        }
        if self.current_bet() > self.bets[player_num] {
            return Err(GameError::invalid_input(
                "Cannot check because you are below the bet",
            ));
        }
        let mut logs = vec![Log::public(vec![
            render_player_name(player_num),
            N::text(" checked"),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    /// Port of `CanFold`.
    pub fn can_fold(&self, player: usize) -> bool {
        let current_bet = self.current_bet();
        self.current_player == player && self.bets[player] < current_bet && !self.is_finished()
    }

    /// Port of `Fold`.
    pub fn fold(&mut self, player_num: usize) -> Result<Vec<Log>, GameError> {
        if self.is_finished() || self.current_player != player_num {
            return Err(GameError::invalid_input("Not your turn"));
        }
        let mut logs = vec![Log::public(vec![
            render_player_name(player_num),
            N::text(" folded"),
        ])];
        self.folded_players[player_num] = true;
        if self.active_players().len() == 1 {
            // Everyone folded. Go's `for _, p := range g.ActivePlayers() { ...
            // return }` only ever runs its body once because exactly one
            // active player remains at this point; ported directly as a
            // single-element access (clippy's `never_loop` rejects a Rust
            // `for` loop that unconditionally returns on its first
            // iteration) rather than as a loop.
            let p = self.active_players()[0];
            logs.push(Log::public(vec![
                render_player_name(p),
                N::text(" took "),
                render_cash(self.pot()),
            ]));
            self.player_money[p] += self.pot();
            logs.extend(self.new_hand());
            return Ok(logs);
        } else {
            logs.extend(self.next_player());
        }
        Ok(logs)
    }

    /// Port of `CanCall`.
    pub fn can_call(&self, player: usize) -> bool {
        let current_bet = self.current_bet();
        self.current_player == player
            && self.bets[player] < current_bet
            && self.player_money[player] > current_bet - self.bets[player]
            && !self.is_finished()
    }

    /// Port of `Call`.
    pub fn call(&mut self, player_num: usize) -> Result<Vec<Log>, GameError> {
        if self.is_finished() || self.current_player != player_num {
            return Err(GameError::invalid_input("Not your turn"));
        }
        let difference = self.current_bet() - self.bets[player_num];
        if self.player_money[player_num] < difference {
            return Err(GameError::invalid_input(
                "You don't have enough to call, you can only go allin",
            ));
        }
        if difference <= 0 {
            return Err(GameError::invalid_input(
                "You are already at the current bet, you may check if you don't want to raise",
            ));
        }
        self.bet(player_num, difference)?;
        let mut logs = vec![Log::public(vec![
            render_player_name(player_num),
            N::text(" called"),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    /// Port of `MinRaise`.
    pub fn min_raise(&self) -> i32 {
        self.minimum_bet.max(self.largest_raise)
    }

    /// Port of `Raise`.
    pub fn raise(&mut self, player_num: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if self.is_finished() || self.current_player != player_num {
            return Err(GameError::invalid_input("Not your turn"));
        }
        let min_raise = self.min_raise();
        let difference = self.current_bet() - self.bets[player_num];
        if amount < min_raise {
            return Err(GameError::invalid_input(format!(
                "Your raise must be at least {}",
                min_raise
            )));
        }
        self.bet(player_num, difference + amount)?;
        let mut logs = vec![Log::public(vec![
            render_player_name(player_num),
            N::text(" raised by "),
            render_cash(amount),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    /// Port of `CanRaise`.
    ///
    /// Go quirk preserved: the local variable is named `minRaise` but is
    /// assigned `g.LargestRaise`, not `g.MinRaise()` - so the guard doesn't
    /// actually use the minimum-bet floor. Kept verbatim per the porting
    /// correctness rule.
    pub fn can_raise(&self, player: usize) -> bool {
        let current_bet = self.current_bet();
        let min_raise = self.largest_raise;
        self.current_player == player
            && self.player_money[player] > current_bet - self.bets[player] + min_raise
            && !self.is_finished()
    }

    /// Port of `AllIn`.
    pub fn all_in(&mut self, player_num: usize) -> Result<Vec<Log>, GameError> {
        if self.is_finished() || self.current_player != player_num {
            return Err(GameError::invalid_input("Not your turn"));
        }
        let amount = self.player_money[player_num];
        self.bet(player_num, amount)?;
        let mut logs = vec![Log::public(vec![
            render_player_name(player_num),
            N::text(" went all in with "),
            render_cash(amount),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    /// Port of `CanAllIn`.
    pub fn can_all_in(&self, player: usize) -> bool {
        self.current_player == player && self.player_money[player] > 0 && !self.is_finished()
    }

    /// Port of `NextPlayer`.
    fn next_player(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let requiring_call_players = self.requiring_call_players();
        let betting_players = self.betting_players();
        if !betting_players.is_empty() {
            let next_player = self.next_player_in_set(self.current_player, &betting_players);
            if !self.everyone_has_bet_once {
                // Check if we've passed the first player, using i32 for the
                // signed distance arithmetic (mirrors Go's int math).
                let players = self.players as i32;
                let mut distance_to_first =
                    self.first_betting_player as i32 - self.current_player as i32;
                if distance_to_first <= 0 {
                    distance_to_first += players;
                }
                let mut distance_to_next_player = next_player as i32 - self.current_player as i32;
                if distance_to_next_player <= 0 {
                    distance_to_next_player += players;
                }
                if distance_to_next_player >= distance_to_first {
                    self.everyone_has_bet_once = true;
                }
            }
            if requiring_call_players.is_empty() && self.everyone_has_bet_once {
                logs.extend(self.next_phase());
            } else {
                self.current_player = next_player;
            }
        } else {
            logs.extend(self.next_phase());
        }
        logs
    }

    /// Port of `NextPhase`.
    fn next_phase(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let betting_players_count = self.betting_players().len();
        match self.community_cards.len() {
            0 => {
                logs.extend(self.flop());
                if betting_players_count < 2 {
                    logs.extend(self.next_phase());
                }
            }
            3 => {
                logs.extend(self.turn());
                if betting_players_count < 2 {
                    logs.extend(self.next_phase());
                }
            }
            4 => {
                logs.extend(self.river());
                if betting_players_count < 2 {
                    logs.extend(self.next_phase());
                }
            }
            5 => {
                logs.extend(self.showdown());
            }
            _ => {}
        }
        logs
    }

    /// Port of `Flop`.
    fn flop(&mut self) -> Vec<Log> {
        self.new_community_cards(3);
        let logs = vec![Log::public(vec![
            N::text("Flop cards are "),
            N::Bold(render::cards(&self.community_cards)),
        ])];
        self.new_betting_round();
        logs
    }

    /// Port of `Turn`.
    fn turn(&mut self) -> Vec<Log> {
        self.new_community_cards(1);
        let logs = vec![Log::public(vec![
            N::text("Turn card is "),
            N::Bold(vec![self.community_cards[3].render_standard_52()]),
        ])];
        self.new_betting_round();
        logs
    }

    /// Port of `River`.
    fn river(&mut self) -> Vec<Log> {
        self.new_community_cards(1);
        let logs = vec![Log::public(vec![
            N::text("River card is "),
            N::Bold(vec![self.community_cards[4].render_standard_52()]),
        ])];
        self.new_betting_round();
        logs
    }

    /// Port of `Showdown`.
    fn showdown(&mut self) -> Vec<Log> {
        let mut content: Vec<N> = vec![N::Bold(vec![N::text("Showdown")]), N::text("\n")];
        while self.pot() > 0 {
            let smallest = self.smallest_bet();
            let mut pot = 0;
            let mut hand_results: HashMap<usize, poker::HandResult> = HashMap::new();
            let mut hands_table: Vec<Row> = vec![];
            for player_num in 0..self.players {
                let b = self.bets[player_num];
                if b == 0 {
                    continue;
                }
                let contribution = b.min(smallest);
                pot += contribution;
                self.bets[player_num] -= contribution;
                if !self.folded_players[player_num] {
                    let full_hand =
                        card::push_many(&self.player_hands[player_num], &self.community_cards);
                    let hand_result = poker::result(&full_hand);
                    hands_table.push(vec![
                        (A::Left, vec![render_player_name(player_num)]),
                        (A::Left, render::cards(&self.player_hands[player_num])),
                        (A::Left, vec![N::text(hand_result.name.clone())]),
                        (A::Left, render::cards(&hand_result.cards)),
                    ]);
                    hand_results.insert(player_num, hand_result);
                }
            }
            if hand_results.len() > 1 {
                // Multiple people for this pot, showdown.
                content.push(N::text("Showdown for pot of "));
                content.push(render_cash(pot));
                content.push(N::text("\n"));
                content.push(table_with_gap(&hands_table, 2));
                content.push(N::text("\n"));
                let hand_results_i32: HashMap<i32, poker::HandResult> = hand_results
                    .iter()
                    .map(|(&p, hr)| (p as i32, hr.clone()))
                    .collect();
                let winners = poker::winning_hand_result(&hand_results_i32);
                let pot_per_player = pot / winners.len() as i32;
                for &winner in &winners {
                    let winner = winner as usize;
                    content.push(render_player_name(winner));
                    content.push(N::text(" took "));
                    content.push(render_cash(pot_per_player));
                    content.push(N::text(" ("));
                    content.push(N::text(hand_results[&winner].name.clone()));
                    content.push(N::text(")\n"));
                    self.player_money[winner] += pot_per_player;
                }
                let remainder = pot - pot_per_player * winners.len() as i32;
                if remainder > 0 {
                    let remainder_player = self.next_remaining_player_num_from(self.current_dealer);
                    content.push(render_player_name(remainder_player));
                    content.push(N::text(" took "));
                    content.push(render_cash(remainder));
                    content.push(N::text(" due to uneven split"));
                    self.player_money[remainder_player] += remainder;
                }
            } else {
                // Only one player left for the pot, give it to them.
                for (&player_num, hand_result) in hand_results.iter() {
                    content.push(render_player_name(player_num));
                    content.push(N::text(" took remaining "));
                    content.push(render_cash(pot));
                    content.push(N::text(" ("));
                    content.push(N::text(hand_result.name.clone()));
                    content.push(N::text(")\n"));
                    self.player_money[player_num] += pot;
                }
            }
        }
        let mut logs = vec![Log::public(content)];
        if !self.is_finished() {
            logs.extend(self.new_hand());
        }
        logs
    }

    /// Port of `CurrentBet`.
    pub fn current_bet(&self) -> i32 {
        self.bets.iter().cloned().max().unwrap_or(0)
    }

    /// Port of `Pot`.
    pub fn pot(&self) -> i32 {
        self.bets.iter().sum()
    }

    /// Port of `SmallestBet`.
    fn smallest_bet(&self) -> i32 {
        let mut bet = 0;
        let mut first_run = true;
        for p in self.active_players() {
            if self.bets[p] != 0 && (first_run || self.bets[p] < bet) {
                bet = self.bets[p];
                first_run = false;
            }
        }
        bet
    }

    /// Port of `NewCommunityCards`.
    fn new_community_cards(&mut self, n: usize) {
        let (popped, remaining) = card::pop_n(&self.deck, n);
        self.deck = remaining;
        self.community_cards = card::push_many(&self.community_cards, &popped);
    }

    /// Port of `NewBettingRound`.
    fn new_betting_round(&mut self) {
        if !self.betting_players().is_empty() {
            self.current_player = self.next_betting_player_num_from(self.current_dealer);
        } else {
            self.current_player = self.current_dealer;
        }
        self.first_betting_player = self.current_player;
        self.everyone_has_bet_once = false;
    }

    /// Port of `NewHand`.
    fn new_hand(&mut self) -> Vec<Log> {
        // Reset values.
        self.folded_players = vec![false; self.players];
        self.bets = vec![0; self.players];
        self.largest_raise = 0;
        self.everyone_has_bet_once = false;
        self.new_betting_round();
        let active_players = self.active_players();
        let num_active_players = active_players.len();
        let mut logs = vec![];
        // Raise blinds if we need to.
        if self.hands_since_blinds_increase >= HANDS_PER_BLINDS_INCREASE {
            self.hands_since_blinds_increase = 0;
            self.minimum_bet *= 2;
            logs.push(Log::public(vec![
                N::text("Minimum bet increased to "),
                render_cash(self.minimum_bet),
            ]));
        } else {
            self.hands_since_blinds_increase += 1;
        }
        // Set a new active dealer.
        self.current_dealer = self.next_active_player_num_from(self.current_dealer);
        logs.push(Log::public(vec![
            render_player_name(self.current_dealer),
            N::text(" is the new dealer"),
        ]));
        // Blinds.
        let small_blind_player = if num_active_players == 2 {
            // Special head-to-head rules for 2 player.
            // https://en.wikipedia.org/wiki/Texas_hold_'em#Betting_structures
            self.current_dealer
        } else {
            self.next_active_player_num_from(self.current_dealer)
        };
        let amount = self.bet_up_to(small_blind_player, self.minimum_bet / 2);
        logs.push(Log::public(vec![
            render_player_name(small_blind_player),
            N::text(" posted a small blind of "),
            render_cash(amount),
        ]));
        let big_blind_player = self.next_active_player_num_from(small_blind_player);
        let amount = self.bet_up_to(big_blind_player, self.minimum_bet);
        logs.push(Log::public(vec![
            render_player_name(big_blind_player),
            N::text(" posted a big blind of "),
            render_cash(amount),
        ]));
        // Shuffle and deal two cards to each player.
        self.community_cards = Deck::new();
        self.deck = card::shuffle(&card::standard_52_deck_ace_high(), &mut self.rng);
        for &p in &active_players {
            let (hand, remaining) = card::pop_n(&self.deck, 2);
            self.deck = remaining;
            self.player_hands[p] = card::sort(&hand);
        }
        if !self.betting_players().is_empty() {
            // Make the current player the one next to the big blind.
            self.current_player = self.next_betting_player_num_from(big_blind_player);
            self.first_betting_player = self.current_player;
        } else {
            // Nobody has money! Just go to next phase.
            logs.extend(self.next_phase());
        }
        logs
    }

    /// Port of `IsFinished`.
    pub fn is_finished(&self) -> bool {
        self.remaining_players().len() < 2
    }

    /// Port of `EliminatedPlayerList`.
    pub fn eliminated_player_list(&self) -> Vec<usize> {
        (0..self.players)
            .filter(|&p| self.player_money[p] == 0 && self.bets[p] == 0)
            .collect()
    }

    /// Port of `PlayerTotalMoney`.
    pub fn player_total_money(&self, player: usize) -> i32 {
        self.bets[player] + self.player_money[player]
    }

    /// Port of `Placings`.
    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_total_money(p)])
            .collect();
        gen_placings(&metrics)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    /// Port of `New`.
    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let mut rng = GameRng::seed_from_u64(seed);
        // Pick a random starting player.
        let current_dealer = rng.random_range(0..players);
        let mut g = Game {
            players,
            player_hands: vec![Deck::new(); players],
            player_money: vec![STARTING_MONEY; players],
            minimum_bet: STARTING_MINIMUM_BET,
            current_dealer,
            rng,
            ..Game::default()
        };
        let logs = g.new_hand();
        Ok((g, logs))
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            community_cards: self.community_cards.clone(),
            pot: self.pot(),
            current_dealer: self.current_dealer,
            current_player: self.current_player,
            player_money: self.player_money.clone(),
            bets: self.bets.clone(),
            folded_players: self.folded_players.clone(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: self.player_hands[player].clone(),
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
                value: Command::AllIn,
                remaining,
                ..
            }) => self.all_in(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Call,
                remaining,
                ..
            }) => self.call(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Check,
                remaining,
                ..
            }) => self.check(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Fold,
                remaining,
                ..
            }) => self.fold(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Raise(amount),
                remaining,
                ..
            }) => self.raise(player, amount).map(|logs| CommandResponse {
                logs,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![HashMap::new(); self.players],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: self.eliminated_player_list(),
            }
        }
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
    }

    fn points(&self) -> Vec<f32> {
        (0..self.players)
            .map(|p| self.player_total_money(p) as f32)
            .collect()
    }

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const BJ: usize = 2;

    fn mock_game() -> Game {
        Game::start(2, 1).unwrap().0
    }

    /// Renders a `Log`'s content to plain text for assertions. Used for the
    /// showdown pot-distribution tests below: `showdown()` always recurses
    /// into `new_hand()` when the game isn't finished (mirroring Go's
    /// `Showdown`/`NewHand` mutual recursion - see the porting notes on
    /// `new_hand`), which immediately re-bets blinds out of the same
    /// `player_money` the showdown just paid into. So the pot-distribution
    /// outcome itself is only observable in the log text `showdown()`
    /// returns, not in post-call `player_money`.
    fn log_text(logs: &[Log]) -> String {
        logs.iter()
            .map(|l| brdgme_markup::plain(&brdgme_markup::transform(&l.content, &[])))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_start() {
        let g = mock_game();
        assert_ne!(0, g.players);
    }

    #[test]
    fn test_next_phase_on_initial_fold() {
        let mut g = Game::start(3, 1).unwrap().0;
        // First player folds.
        g.fold(g.current_player).unwrap();
        assert_eq!(0, g.community_cards.len(), "Cards were already drawn");
        // Next two players call and check, should flop.
        g.call(g.current_player).unwrap();
        assert_eq!(0, g.community_cards.len(), "Cards were already drawn");
        g.check(g.current_player).unwrap();
        assert_eq!(3, g.community_cards.len(), "No flop");
    }

    #[test]
    fn test_dealer_raise_when_last_player() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_dealer = MICK;
        g.current_player = MICK;
        g.first_betting_player = MICK;
        g.bets = vec![5, 10, 0];
        g.call(MICK).unwrap();
        assert_eq!(0, g.community_cards.len(), "Flopped too early");
        g.check(STEVE).unwrap();
        assert_eq!(0, g.community_cards.len(), "Flopped too early");
        g.call(BJ).unwrap();
        assert_eq!(3, g.community_cards.len(), "Flop didn't happen");
        g.check(STEVE).unwrap();
        assert_eq!(3, g.community_cards.len(), "Turn happened too early");
        g.check(BJ).unwrap();
        assert_eq!(3, g.community_cards.len(), "Turn happened too early");
        g.raise(MICK, 10).unwrap();
        assert_eq!(3, g.community_cards.len(), "Turn happened too early");
        g.call(STEVE).unwrap();
        assert_eq!(3, g.community_cards.len(), "Turn happened too early");
        g.call(BJ).unwrap();
        assert_eq!(4, g.community_cards.len(), "Turn didn't happen");
    }

    #[test]
    fn test_all_in_above_other_player() {
        // https://github.com/Miniand/brdg.me/issues/3
        let mut g = Game::start(2, 1).unwrap().0;
        let (popped, remaining) = card::pop_n(&g.deck, 3);
        g.community_cards = popped;
        g.deck = remaining;
        g.current_dealer = 0;
        g.current_player = 0;
        g.first_betting_player = 0;
        g.bets = vec![5, 10];
        g.player_money = vec![10, 20];
        // Go all in with MICK.
        g.all_in(MICK).unwrap();
        assert_eq!(3, g.community_cards.len());
        assert_eq!(
            1, g.current_player,
            "Game progressed without letting Steve call"
        );
    }

    #[test]
    fn test_all_players_all_in_when_blinds_bigger_than_cash() {
        let mut g = Game::start(2, 1).unwrap().0;
        g.player_money = vec![3, 3];
        // Reseeded so the shuffled deck gives the two players hands that
        // don't tie at showdown (a tie would split the pot evenly and leave
        // both players non-eliminated, which would fail the assertion below
        // for a reason unrelated to what this test is checking - the
        // all-in-below-blinds path forcing the hand to a conclusion).
        g.rng = GameRng::seed_from_u64(2);
        g.new_hand();
        assert!(
            g.is_finished(),
            "Game didn't finish when players had lower money than blinds"
        );
    }

    #[test]
    fn test_next_player_is_skipped_on_next_phase_when_no_money() {
        // https://github.com/Miniand/brdg.me/issues/5
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_dealer = 0;
        g.current_player = 0;
        g.first_betting_player = 0;
        g.bets = vec![10, 6, 10];
        g.player_money = vec![10, 0, 100];
        // Skip to next phase manually.
        g.next_phase();
        assert_eq!(
            2, g.current_player,
            "Didn't skip over Mick on new phase even though he is all in"
        );
    }

    #[test]
    fn test_eliminated_players() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.player_money[0] = 0;
        g.player_money[1] = 0;
        g.bets[0] = 0;
        g.bets[1] = 5;
        let eliminated = g.eliminated_player_list();
        assert_eq!(
            vec![MICK],
            eliminated,
            "Expected only Mick to be eliminated"
        );
    }

    // Additional baseline tests (the Go suite is thin on command-guard and
    // status coverage - see docs/porting/GAME_PORTING.md step 8).

    #[test]
    fn can_check_true_when_at_current_bet() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = 0;
        g.bets = vec![10, 10, 10];
        assert!(g.can_check(0));
        assert!(!g.can_check(1));
    }

    #[test]
    fn can_fold_true_when_behind_bet() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = 0;
        g.bets = vec![5, 10, 10];
        assert!(g.can_fold(0));
        assert!(!g.can_fold(1));
    }

    #[test]
    fn can_call_requires_enough_money_and_behind_bet() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = 0;
        g.bets = vec![5, 10, 10];
        g.player_money = vec![20, 0, 0];
        assert!(g.can_call(0));
        // Not enough money to call (would need to go allin instead).
        g.player_money[0] = 5;
        assert!(!g.can_call(0));
    }

    #[test]
    fn can_raise_requires_enough_money_above_largest_raise() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = 0;
        g.bets = vec![0, 10, 0];
        g.largest_raise = 10;
        g.player_money = vec![25, 0, 0];
        assert!(g.can_raise(0));
        g.player_money[0] = 19;
        assert!(!g.can_raise(0));
    }

    #[test]
    fn can_all_in_requires_money_and_turn() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = 0;
        g.player_money = vec![5, 0, 0];
        assert!(g.can_all_in(0));
        assert!(!g.can_all_in(1));
        g.player_money[0] = 0;
        assert!(!g.can_all_in(0));
    }

    #[test]
    fn fold_to_one_player_awards_pot_and_starts_new_hand() {
        let mut g = Game::start(2, 1).unwrap().0;
        let hand_before = g.hands_since_blinds_increase;
        g.fold(g.current_player).unwrap();
        // A new hand should have started: community cards reset and a new
        // round of blinds posted (or all-in resolution recursed further).
        assert_ne!(hand_before, g.hands_since_blinds_increase);
    }

    #[test]
    fn blinds_increase_after_five_hands() {
        let mut g = Game::start(2, 1).unwrap().0;
        let starting_minimum_bet = g.minimum_bet;
        for _ in 0..HANDS_PER_BLINDS_INCREASE {
            // Force through a hand quickly by folding until a new hand
            // starts (2-player fold immediately awards the pot and starts
            // a new hand).
            let p = g.current_player;
            g.fold(p).unwrap();
        }
        assert_eq!(starting_minimum_bet * 2, g.minimum_bet);
    }

    #[test]
    fn showdown_splits_pot_between_tied_hands() {
        let mut g = Game::start(2, 1).unwrap().0;
        g.community_cards = vec![
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_2,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_3,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_4,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_5,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_6,
            },
        ];
        // Both players get the exact same (unusable, off-suit) pair so their
        // best hand is identical (the straight flush on the board).
        g.player_hands[0] = vec![
            card::Card {
                suit: card::Suit::Diamonds,
                rank: card::RANK_8,
            },
            card::Card {
                suit: card::Suit::Hearts,
                rank: card::RANK_9,
            },
        ];
        g.player_hands[1] = vec![
            card::Card {
                suit: card::Suit::Diamonds,
                rank: card::RANK_10,
            },
            card::Card {
                suit: card::Suit::Hearts,
                rank: card::RANK_JACK,
            },
        ];
        g.bets = vec![10, 10];
        g.folded_players = vec![false, false];
        g.player_money = vec![0, 0];
        let logs = g.showdown();
        let text = log_text(&logs);
        assert!(text.contains("Showdown for pot of $20"), "{}", text);
        // Both players tie, so both should show up as taking $10.
        assert_eq!(2, text.matches("took $10").count(), "{}", text);
    }

    #[test]
    fn showdown_awards_uneven_split_remainder_to_next_remaining_from_dealer() {
        // 4 equal bets of $10 (pot $40); the first 3 players tie on the
        // board's straight flush (their hole cards are irrelevant kickers),
        // the 4th has a strictly worse hand. $40 / 3 winners doesn't divide
        // evenly, so the $1 remainder goes to whoever is
        // `next_remaining_player_num_from(current_dealer)`.
        let mut g = Game::start(4, 1).unwrap().0;
        g.current_dealer = 0;
        // No straight/flush possible: ranks 2,7,9,J,K are too spread out and
        // only 2 of the 5 cards share a suit.
        g.community_cards = vec![
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_2,
            },
            card::Card {
                suit: card::Suit::Diamonds,
                rank: 7,
            },
            card::Card {
                suit: card::Suit::Hearts,
                rank: 9,
            },
            card::Card {
                suit: card::Suit::Spades,
                rank: card::RANK_JACK,
            },
            card::Card {
                suit: card::Suit::Diamonds,
                rank: card::RANK_KING,
            },
        ];
        // Players 0-2 all hold pocket aces (unrealistic as an actual dealt
        // deck, but `poker::result` only looks at the 7 cards it's given -
        // giving three players identical hole cards is the simplest way to
        // force an exact 3-way tie for this test).
        for p in 0..3 {
            g.player_hands[p] = vec![
                card::Card {
                    suit: card::Suit::Spades,
                    rank: card::RANK_ACE_HIGH,
                },
                card::Card {
                    suit: card::Suit::Hearts,
                    rank: card::RANK_ACE_HIGH,
                },
            ];
        }
        // Player 3 has no pair and no help from the board - clearly worse
        // than the others' pair of aces.
        g.player_hands[3] = vec![
            card::Card {
                suit: card::Suit::Diamonds,
                rank: 3,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: 4,
            },
        ];
        g.bets = vec![10, 10, 10, 10];
        g.folded_players = vec![false, false, false, false];
        g.player_money = vec![0, 0, 0, 0];
        let logs = g.showdown();
        let text = log_text(&logs);
        assert!(text.contains("Showdown for pot of $40"), "{}", text);
        assert_eq!(3, text.matches("took $13").count(), "{}", text);
        assert!(text.contains("took $1 due to uneven split"), "{}", text);
    }

    #[test]
    fn side_pot_awarded_separately_when_one_player_is_all_in_below_bet() {
        let mut g = Game::start(3, 1).unwrap().0;
        // No pairs/straights/flushes from the board alone: ranks 2,7,9,J,K,
        // only 2 shared suits.
        g.community_cards = vec![
            card::Card {
                suit: card::Suit::Clubs,
                rank: card::RANK_2,
            },
            card::Card {
                suit: card::Suit::Diamonds,
                rank: 7,
            },
            card::Card {
                suit: card::Suit::Hearts,
                rank: 9,
            },
            card::Card {
                suit: card::Suit::Spades,
                rank: card::RANK_JACK,
            },
            card::Card {
                suit: card::Suit::Diamonds,
                rank: card::RANK_KING,
            },
        ];
        // Player 0 has the best hand outright (a pair of aces) but is
        // all-in for less than the other two, so their winnings are capped
        // at the smaller main pot they're eligible for.
        g.player_hands[0] = vec![
            card::Card {
                suit: card::Suit::Spades,
                rank: card::RANK_ACE_HIGH,
            },
            card::Card {
                suit: card::Suit::Hearts,
                rank: card::RANK_ACE_HIGH,
            },
        ];
        // Player 1: high card only, kicker 5 (no pair/straight with the
        // board).
        g.player_hands[1] = vec![
            card::Card {
                suit: card::Suit::Diamonds,
                rank: 4,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: 5,
            },
        ];
        // Player 2: high card only, kicker 8 - beats player 1's kicker of 5
        // for the side pot, but still loses to player 0's pair overall.
        g.player_hands[2] = vec![
            card::Card {
                suit: card::Suit::Diamonds,
                rank: 6,
            },
            card::Card {
                suit: card::Suit::Clubs,
                rank: 8,
            },
        ];
        g.bets = vec![5, 20, 20];
        g.folded_players = vec![false, false, false];
        g.player_money = vec![0, 0, 0];
        let logs = g.showdown();
        let text = log_text(&logs);
        // Main pot (5 * 3 = $15, all three eligible) goes to player 0 (best
        // hand overall).
        assert!(text.contains("Showdown for pot of $15"), "{}", text);
        assert!(text.contains("took $15"), "{}", text);
        // Side pot ($15 * 2 = $30, between players 1 & 2 only) goes to
        // player 2, the better of the two remaining hands.
        assert!(text.contains("Showdown for pot of $30"), "{}", text);
        assert!(text.contains("took $30"), "{}", text);
    }

    #[test]
    fn pub_state_does_not_leak_hands_or_deck() {
        let g = mock_game();
        let json = serde_json::to_value(g.pub_state()).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("player_hands"));
        assert!(!obj.contains_key("deck"));
        assert!(obj.contains_key("community_cards"));
        assert!(obj.contains_key("player_money"));
    }

    #[test]
    fn player_state_carries_own_hand() {
        let g = mock_game();
        let ps = g.player_state(0);
        assert_eq!(g.player_hands[0], ps.hand);
    }

    #[test]
    fn finished_status_reports_placings() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.player_money = vec![0, 300, 0];
        g.bets = vec![0, 0, 0];
        match g.status() {
            Status::Finished { placings, .. } => {
                assert_eq!(vec![2, 1, 2], placings);
            }
            _ => panic!("expected finished status"),
        }
    }

    #[test]
    fn points_reflect_total_money() {
        let mut g = Game::start(2, 1).unwrap().0;
        g.player_money = vec![40, 60];
        g.bets = vec![10, 0];
        assert_eq!(vec![50.0, 60.0], g.points());
    }
}
