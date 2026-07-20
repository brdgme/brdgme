//! Port of `brdgme-go/roll_through_the_ages_1`.
//!
//! Task 2 provided the full `Game` struct + `Gamer` impl (start/status/
//! points/whose_turn/pub+player state shapes), the complete phase/turn
//! cascade engine (including resolve-phase disasters), and the full
//! `command_parser` (all 11 gated sub-parsers), with `next`/`roll`/
//! `preserve` dispatching to real actions. Task 3 wires up the remaining 8
//! command variants (`build`/`trade`/`buy`/`take`/`discard`/`invade`/
//! `sell`/`swap`), finishing `command()`'s dispatch.

pub mod development;
pub mod dice;
pub mod good;
pub mod monument;
pub mod player_board;
pub mod take;

mod command;
mod render;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use development::DevelopmentId;
use dice::Die;
use good::{GOODS, Good, good_maximum, good_value};
use monument::{MONUMENTS, MonumentId};
use player_board::PlayerBoard;
use take::TakeAction;

pub use command::{BuildTarget, BuyGoods, Command};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

/// Phase enum, ported from `game.go`'s `Phase` iota.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Preserve,
    Roll,
    ExtraRoll,
    Collect,
    Resolve,
    Invade,
    Build,
    Trade,
    Buy,
    Discard,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub phase: Phase,
    pub boards: Vec<PlayerBoard>,

    pub rolled_dice: Vec<Die>,
    pub kept_dice: Vec<Die>,
    pub remaining_rolls: i32,
    pub remaining_workers: i32,
    pub remaining_ships: i32,
    pub remaining_coins: i32,

    pub final_round: bool,
    pub finished: bool,

    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            players: 0,
            current_player: 0,
            phase: Phase::Preserve,
            boards: vec![],
            rolled_dice: vec![],
            kept_dice: vec![],
            remaining_rolls: 0,
            remaining_workers: 0,
            remaining_ships: 0,
            remaining_coins: 0,
            final_round: false,
            finished: false,
            rng: GameRng::default(),
        }
    }
}

/// No hidden information in this game (`PlayerState`/`PubState` both return
/// `nil` in Go; `PubRender()` is literally `PlayerRender(CurrentPlayer)`).
/// Both states carry a full clone of the game so `render.rs` can port
/// `PlayerRender` verbatim; `PubState`'s render uses `game.current_player`,
/// matching Go's `PubRender() = PlayerRender(CurrentPlayer)` exactly.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PubState {
    /// The full game state: every player's board, the current dice, phase,
    /// turn supplies, and round/finish flags. This game has no hidden
    /// information, so the public state is a complete clone of the game.
    pub game: Game,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full game state, identical to the public state since there is no
    /// hidden information.
    pub game: Game,
    /// Which player index (0 through players-1) this state is being shown to.
    pub player: usize,
}

impl Game {
    // ---------------------------------------------------------------
    // Guard functions (Can*), ported from each `*_command.go`. Each Go file
    // is read directly, not assumed uniform; see the plan's Global
    // Constraints re `can_undo` for why this matters.
    // ---------------------------------------------------------------

    /// Port of `CanNext` (next_command.go).
    pub fn can_next(&self, player: usize) -> bool {
        player == self.current_player
            && matches!(
                self.phase,
                Phase::Preserve
                    | Phase::Roll
                    | Phase::ExtraRoll
                    | Phase::Invade
                    | Phase::Build
                    | Phase::Trade
                    | Phase::Buy
            )
    }

    /// Port of `CanRoll` (roll_command.go). Preserves Go's exact operator
    /// precedence: `(Phase==Roll && (RemainingRolls>0 && len(RolledDice)>0))
    /// || Phase==ExtraRoll`.
    pub fn can_roll(&self, player: usize) -> bool {
        if self.current_player != player {
            return false;
        }
        (self.phase == Phase::Roll && (self.remaining_rolls > 0 && !self.rolled_dice.is_empty()))
            || self.phase == Phase::ExtraRoll
    }

    /// Port of `CanPreserve` (preserve_command.go).
    pub fn can_preserve(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Preserve
            && b.developments.contains(&DevelopmentId::Preservation)
            && b.goods.get(&Good::Pottery).copied().unwrap_or(0) > 0
            && b.food > 0
    }

    /// Port of `CanBuildBuilding` (build_command.go).
    pub fn can_build_building(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Build && self.remaining_workers > 0
    }

    /// Port of `CanBuildShip` (build_command.go).
    pub fn can_build_ship(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Build
            && b.developments.contains(&DevelopmentId::Shipping)
            && b.goods.get(&Good::Wood).copied().unwrap_or(0) > 0
            && b.goods.get(&Good::Cloth).copied().unwrap_or(0) > 0
    }

    /// Port of `CanBuild` (build_command.go).
    pub fn can_build(&self, player: usize) -> bool {
        self.can_build_building(player) || self.can_build_ship(player)
    }

    /// Port of `CanTrade` (trade_command.go). Note the Go quirk this
    /// comment preserves: this checks `Phase == PhaseBuild`, not a
    /// dedicated trade phase - see the plan's Global Constraints.
    pub fn can_trade(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Build
            && b.developments.contains(&DevelopmentId::Engineering)
            && b.goods.get(&Good::Stone).copied().unwrap_or(0) > 0
    }

    /// Port of `CanBuildOrTrade` (build_command.go).
    pub fn can_build_or_trade(&self, player: usize) -> bool {
        self.can_build(player) || self.can_trade(player)
    }

    /// Port of `CanBuy` (buy_command.go).
    pub fn can_buy(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Buy
    }

    /// Port of `CanTake` (take_command.go).
    pub fn can_take(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Collect
    }

    /// Port of `CanDiscard` (discard_command.go).
    pub fn can_discard(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Discard
            && self.boards[self.current_player].goods_over_limit() > 0
    }

    /// Port of `CanInvade` (invade_command.go).
    pub fn can_invade(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Invade
            && b.developments.contains(&DevelopmentId::Smithing)
            && b.goods.get(&Good::Spearhead).copied().unwrap_or(0) > 0
    }

    /// Port of `CanSell` (sell_command.go).
    pub fn can_sell(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Buy
            && b.developments.contains(&DevelopmentId::Granaries)
            && b.food > 0
    }

    /// Port of `CanSwap` (swap_command.go).
    pub fn can_swap(&self, player: usize) -> bool {
        let b = &self.boards[player];
        self.current_player == player
            && self.phase == Phase::Trade
            && b.developments.contains(&DevelopmentId::Shipping)
            && b.goods_num() > 0
    }

    // ---------------------------------------------------------------
    // Phase/turn cascade engine, ported from `game.go`.
    // ---------------------------------------------------------------

    /// Port of `StartTurn`.
    fn start_turn(&mut self) -> Vec<Log> {
        self.remaining_coins = 0;
        self.remaining_workers = 0;
        self.preserve_phase()
    }

    /// Port of `NextPhase`.
    fn next_phase(&mut self) -> Vec<Log> {
        match self.phase {
            Phase::Preserve => self.roll_phase(),
            Phase::Roll => self.roll_extra_phase(),
            Phase::ExtraRoll => self.collect_phase(),
            Phase::Collect => self.phase_resolve(),
            Phase::Resolve | Phase::Invade => self.build_phase(),
            Phase::Build => self.trade_phase(),
            Phase::Trade => self.buy_phase(),
            Phase::Buy => self.discard_phase(),
            Phase::Discard => self.next_turn(),
        }
    }

    /// Port of `PreservePhase`.
    fn preserve_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Preserve;
        if !self.can_preserve(self.current_player) {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `RollPhase`.
    fn roll_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Roll;
        let cities = self.boards[self.current_player].cities();
        let logs = self.new_roll(cities);
        self.remaining_rolls = 2;
        logs
    }

    /// Port of `RollExtraPhase`.
    fn roll_extra_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::ExtraRoll;
        // Can reroll anything.
        let mut kept = std::mem::take(&mut self.kept_dice);
        self.rolled_dice.append(&mut kept);
        self.kept_dice = vec![];
        if !self.boards[self.current_player]
            .developments
            .contains(&DevelopmentId::Leadership)
        {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `CollectPhase`.
    fn collect_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Collect;
        // Go: `g.KeptDice = append(g.RolledDice, g.KeptDice...)`.
        let mut new_kept = std::mem::take(&mut self.rolled_dice);
        new_kept.append(&mut self.kept_dice);
        self.kept_dice = new_kept;
        self.rolled_dice = vec![];

        let cp = self.current_player;
        let mut has_food_or_workers_dice = false;
        let mut goods = 0i32;
        for &d in self.kept_dice.clone().iter() {
            match d {
                Die::Food => {
                    let modifier = self.boards[cp].food_modifier();
                    self.boards[cp].food += 3 + modifier;
                }
                Die::Good => goods += 1,
                Die::Skull => goods += 2,
                Die::Workers => {
                    let modifier = self.boards[cp].worker_modifier();
                    self.remaining_workers += 3 + modifier;
                }
                Die::FoodOrWorkers => has_food_or_workers_dice = true,
                Die::Coins => {
                    self.remaining_coins += self.boards[cp].coins_die_value();
                }
            }
        }
        self.boards[cp].gain_goods(goods);
        if !has_food_or_workers_dice {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `PhaseResolve`.
    fn phase_resolve(&mut self) -> Vec<Log> {
        self.phase = Phase::Resolve;
        let cp = self.current_player;
        let players = self.players;
        let mut logs: Vec<Log> = vec![];

        // Check food isn't over maximum.
        if self.boards[cp].food > 15 {
            logs.push(Log::public(vec![
                N::Player(cp),
                N::text(" had their food reduced from "),
                N::Bold(vec![N::text(self.boards[cp].food.to_string())]),
                N::text(" to the maximum of "),
                N::Bold(vec![N::text("15")]),
            ]));
            self.boards[cp].food = 15;
        }

        // Feed cities.
        let cities = self.boards[cp].cities();
        if self.boards[cp].food >= cities {
            self.boards[cp].food -= cities;
            logs.push(Log::public(vec![
                N::Player(cp),
                N::text(" fed "),
                N::Bold(vec![N::text(cities.to_string())]),
                N::text(" cities"),
            ]));
        } else {
            let famine = cities - self.boards[cp].food;
            self.boards[cp].food = 0;
            self.boards[cp].disasters += famine;
            logs.push(Log::public(vec![
                N::text("Famine! "),
                N::Player(cp),
                N::text(" takes "),
                N::Bold(vec![N::text(format!("{} disaster points", famine))]),
            ]));
        }

        // Resolve disasters.
        let skulls = self.kept_dice.iter().filter(|&&d| d == Die::Skull).count();
        match skulls {
            0 | 1 => {}
            2 => {
                if self.boards[cp]
                    .developments
                    .contains(&DevelopmentId::Irrigation)
                {
                    logs.push(Log::public(vec![
                        N::Player(cp),
                        N::text(" avoids a drought with their irrigation development"),
                    ]));
                } else {
                    self.boards[cp].disasters += 2;
                    logs.push(Log::public(vec![
                        N::text("Drought! "),
                        N::Player(cp),
                        N::text(" takes "),
                        N::Bold(vec![N::text("2 disaster points")]),
                    ]));
                }
            }
            3 => {
                let mut content: Vec<N> = vec![N::text("Pestilence!")];
                for p in 0..players {
                    if p == cp {
                        continue;
                    }
                    if self.boards[p]
                        .developments
                        .contains(&DevelopmentId::Medicine)
                    {
                        content.push(N::text("\n  "));
                        content.push(N::Player(p));
                        content.push(N::text(
                            " avoids pestilence with their medicine development",
                        ));
                    } else {
                        self.boards[p].disasters += 3;
                        content.push(N::text("\n  "));
                        content.push(N::Player(p));
                        content.push(N::text(" takes "));
                        content.push(N::Bold(vec![N::text("3 disaster points")]));
                    }
                }
                logs.push(Log::public(content));
            }
            4 => {
                if self.boards[cp]
                    .developments
                    .contains(&DevelopmentId::Smithing)
                {
                    let mut content: Vec<N> = vec![
                        N::text("Invasion! "),
                        N::Player(cp),
                        N::text(" has the smithing development, so "),
                        N::Bold(vec![N::text("all other players are invaded")]),
                    ];
                    for p in 0..players {
                        if p == cp {
                            continue;
                        }
                        if self.boards[p].has_built(MonumentId::GreatWall) {
                            content.push(N::text("\n  "));
                            content.push(N::Player(p));
                            content.push(N::text(" avoids an invasion with their wall"));
                        } else {
                            self.boards[p].disasters += 4;
                            content.push(N::text("\n  "));
                            content.push(N::Player(p));
                            content.push(N::text(" takes "));
                            content.push(N::Bold(vec![N::text("4 disaster points")]));
                        }
                    }
                    logs.push(Log::public(content));
                    logs.extend(self.invade_phase());
                    return logs;
                } else if self.boards[cp].has_built(MonumentId::GreatWall) {
                    logs.push(Log::public(vec![
                        N::Player(cp),
                        N::text(" avoids an invasion with their wall"),
                    ]));
                } else {
                    self.boards[cp].disasters += 4;
                    logs.push(Log::public(vec![
                        N::text("Invasion! "),
                        N::Player(cp),
                        N::text(" takes "),
                        N::Bold(vec![N::text("4 disaster points")]),
                    ]));
                }
            }
            _ => {
                if self.boards[cp]
                    .developments
                    .contains(&DevelopmentId::Religion)
                {
                    for p in 0..players {
                        if p == cp {
                            continue;
                        }
                        for &good in GOODS.iter() {
                            self.boards[p].goods.insert(good, 0);
                        }
                    }
                    logs.push(Log::public(vec![
                        N::text("Revolt! "),
                        N::Player(cp),
                        N::text(" has the religion development, so "),
                        N::Bold(vec![N::text("all other players")]),
                        N::text(" lose "),
                        N::Bold(vec![N::text("all of their goods")]),
                    ]));
                } else {
                    for &good in GOODS.iter() {
                        self.boards[cp].goods.insert(good, 0);
                    }
                    logs.push(Log::public(vec![
                        N::text("Revolt! "),
                        N::Player(cp),
                        N::text(" loses "),
                        N::Bold(vec![N::text("all of their goods")]),
                    ]));
                }
            }
        }
        logs.extend(self.next_phase());
        logs
    }

    /// Port of `InvadePhase`.
    fn invade_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Invade;
        if !self.can_invade(self.current_player) {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `BuildPhase`.
    fn build_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Build;
        if !self.can_build_or_trade(self.current_player) {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `TradePhase` (the ship-based goods-swapping phase, `Swap`,
    /// NOT the stone-for-workers `Trade` command - see the plan's Global
    /// Constraints re the `PhaseBuild`/`PhaseTrade` naming trap).
    fn trade_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Trade;
        let b = &self.boards[self.current_player];
        self.remaining_ships = b.ships;
        if b.ships == 0 || b.goods_num() == 0 {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `BuyPhase`.
    fn buy_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Buy;
        let b = &self.boards[self.current_player];
        let mut buying_power = self.remaining_coins + b.goods_value();
        if b.developments.contains(&DevelopmentId::Granaries) {
            buying_power += b.food * 6;
        }
        if buying_power < 10 {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `DiscardPhase`.
    fn discard_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Discard;
        let b = &self.boards[self.current_player];
        if b.goods_num() <= 6 || b.developments.contains(&DevelopmentId::Caravans) {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `NextTurn`.
    fn next_turn(&mut self) -> Vec<Log> {
        self.current_player = (self.current_player + 1) % self.players;
        if self.current_player == 0 && self.final_round {
            self.finished = true;
        }
        if !self.finished {
            return self.start_turn();
        }
        vec![]
    }

    /// Port of `CheckGameEndTriggered`. Called from both `BuildMonument` on
    /// monument completion and `BuyDevelopment` on development purchase
    /// (wired up in Task 3); the phase-engine plumbing here is what those
    /// callers will invoke.
    pub fn check_game_end_triggered(&mut self, player: usize) -> Vec<Log> {
        if self.final_round {
            return vec![];
        }
        // Go's comment says "5th development built" but the check is
        // `len(Developments) >= 7` - a stale comment with zero behavioural
        // effect (Go quirk #6). Ported threshold: 7, not 5.
        if self.boards[player].developments.len() >= 7 {
            return self.trigger_game_end();
        }
        // Every monument built (by anyone).
        for &m in MONUMENTS.iter() {
            let built = self.boards.iter().any(|b| b.has_built(m));
            if !built {
                return vec![];
            }
        }
        self.trigger_game_end()
    }

    /// Port of `TriggerGameEnd`.
    pub fn trigger_game_end(&mut self) -> Vec<Log> {
        self.final_round = true;
        vec![Log::public(vec![N::Bold(vec![N::text(
            "Game end has been triggered, the game will be finished after the last player has their turn",
        )])])]
    }

    /// Port of `AvailableMonuments`.
    pub fn available_monuments(&self, player: usize) -> Vec<MonumentId> {
        MONUMENTS
            .iter()
            .copied()
            .filter(|&m| {
                self.boards[player].monuments.get(&m).copied().unwrap_or(0) < m.value().size
            })
            .collect()
    }

    /// Port of `AvailableDevelopments`.
    pub fn available_developments(&self, player: usize) -> Vec<DevelopmentId> {
        development::DEVELOPMENTS
            .iter()
            .copied()
            .filter(|d| !self.boards[player].developments.contains(d))
            .collect()
    }

    /// Port of `WhoseTurn`.
    pub fn whose_turn(&self) -> Vec<usize> {
        vec![self.current_player]
    }

    // ---------------------------------------------------------------
    // Dice/roll mechanics, ported from `roll_command.go`.
    // ---------------------------------------------------------------

    fn roll_n(&mut self, n: usize) -> Vec<Die> {
        (0..n)
            .map(|_| {
                let idx = self.rng.random_range(0..dice::DICE_FACES.len());
                dice::DICE_FACES[idx]
            })
            .collect()
    }

    /// Port of `NewRoll`.
    fn new_roll(&mut self, n: i32) -> Vec<Log> {
        self.rolled_dice = self.roll_n(n.max(0) as usize);
        let mut logs = vec![self.log_roll(self.rolled_dice.clone(), vec![])];
        self.kept_dice = vec![];
        logs.extend(self.keep_skulls());
        logs
    }

    /// Port of `KeepSkulls`.
    fn keep_skulls(&mut self) -> Vec<Log> {
        // Go: "You can reroll skulls in single player" - `PlayerCount()==1`
        // guard. Dead in this platform (`PlayerCounts()` is `[2,3,4]`), but
        // ported anyway for source fidelity.
        if self.players == 1 {
            return vec![];
        }
        let mut kept_skulls = 0;
        self.rolled_dice.retain(|&d| {
            if d == Die::Skull {
                kept_skulls += 1;
                false
            } else {
                true
            }
        });
        for _ in 0..kept_skulls {
            self.kept_dice.push(Die::Skull);
        }
        if self.rolled_dice.is_empty()
            && !(self.phase == Phase::ExtraRoll
                && self.boards[self.current_player]
                    .developments
                    .contains(&DevelopmentId::Leadership))
        {
            return self.next_phase();
        }
        vec![]
    }

    /// Port of `LogRoll`.
    fn log_roll(&self, new_dice: Vec<Die>, old_dice: Vec<Die>) -> Log {
        let mut content: Vec<N> = vec![N::Player(self.current_player), N::text(" rolled  ")];
        for (i, d) in new_dice.iter().chain(old_dice.iter()).enumerate() {
            if i > 0 {
                content.push(N::text("  "));
            }
            if i < new_dice.len() {
                content.push(N::Bold(vec![N::text(d.face_string())]));
            } else {
                content.push(N::text(d.face_string()));
            }
        }
        Log::public(content)
    }

    /// Port of `Roll`.
    fn roll(&mut self, player: usize, dice_num: Vec<i32>) -> Result<Vec<Log>, GameError> {
        if !self.can_roll(player) {
            return Err(GameError::invalid_input("you can't roll at the moment"));
        }
        if dice_num.is_empty() {
            return Err(GameError::invalid_input(
                "you must specify which dice to roll",
            ));
        }
        if self.phase == Phase::ExtraRoll && dice_num.len() > 1 {
            return Err(GameError::invalid_input(
                "you may only roll one dice on the extra roll",
            ));
        }
        let l = self.rolled_dice.len() as i32;
        for &n in dice_num.iter() {
            if n < 0 || n > l {
                return Err(GameError::invalid_input(format!(
                    "dice number must be between 1 and {}",
                    l
                )));
            }
        }
        let mut kept = vec![];
        for (i, &d) in self.rolled_dice.iter().enumerate() {
            if !dice_num.contains(&((i + 1) as i32)) {
                kept.push(d);
            }
        }
        let rolled = self.roll_n(self.rolled_dice.len() - kept.len());
        let old_dice: Vec<Die> = kept.iter().cloned().chain(self.kept_dice.clone()).collect();
        let mut logs = vec![self.log_roll(rolled.clone(), old_dice)];
        self.rolled_dice = rolled.into_iter().chain(kept).collect();
        logs.extend(self.keep_skulls());
        match self.phase {
            Phase::Roll => {
                self.remaining_rolls -= 1;
                if self.remaining_rolls == 0 {
                    logs.extend(self.next_phase());
                }
            }
            Phase::ExtraRoll => {
                logs.extend(self.next_phase());
            }
            _ => {}
        }
        Ok(logs)
    }

    /// Port of `RollCommand`. `CanUndo` is hardcoded `false` in Go - the
    /// only command in this game that is (genuine RNG); preserve verbatim.
    fn roll_command(
        &mut self,
        player: usize,
        dice: Vec<i32>,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.roll(player, dice)?;
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `next`/`preserve`, ported from `next_command.go`/`preserve_command.go`.
    // ---------------------------------------------------------------

    /// Port of `Next`.
    fn next(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_next(player) {
            return Err(GameError::invalid_input("you can't next at the moment"));
        }
        Ok(self.next_phase())
    }

    fn next_command(
        &mut self,
        player: usize,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.next(player)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    /// Port of `Preserve`.
    fn preserve(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_preserve(player) {
            return Err(GameError::invalid_input("you can't preserve at the moment"));
        }
        self.boards[player].food *= 2;
        *self.boards[player].goods.entry(Good::Pottery).or_insert(0) -= 1;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" used "),
            N::Bold(vec![N::text("preservation")]),
            N::text(" to double their food to "),
            N::Bold(vec![N::text(self.boards[player].food.to_string())]),
            N::text(" for "),
            N::Bold(vec![N::text("1 pottery")]),
        ])];
        logs.extend(self.next_phase());
        Ok(logs)
    }

    fn preserve_command(
        &mut self,
        player: usize,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.preserve(player)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `build`, ported from `build_command.go`.
    // ---------------------------------------------------------------

    /// Port of `BuildCity`.
    fn build_city(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_build_building(player) {
            return Err(GameError::invalid_input("you can't build at the moment"));
        }
        if amount < 1 {
            return Err(GameError::invalid_input("amount must be a positive number"));
        }
        if amount > self.remaining_workers {
            return Err(GameError::invalid_input(format!(
                "you only have {} workers left",
                self.remaining_workers
            )));
        }
        if self.boards[player].city_progress + amount > player_board::MAX_CITY_PROGRESS {
            return Err(GameError::invalid_input(
                "that is more than what remains to be built",
            ));
        }
        let initial_cities = self.boards[player].cities();
        self.remaining_workers -= amount;
        self.boards[player].city_progress += amount;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" used "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" workers on "),
            N::Bold(vec![N::text("cities")]),
        ])];
        let new_cities = self.boards[player].cities();
        if new_cities > initial_cities {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" now has "),
                N::Bold(vec![N::text(format!("{} cities", new_cities))]),
            ]));
        }
        if !self.can_build_or_trade(player) {
            logs.extend(self.next_phase());
        }
        Ok(logs)
    }

    /// Port of `BuildShip`.
    fn build_ship(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_build_ship(player) {
            return Err(GameError::invalid_input(
                "you can't build a ship at the moment",
            ));
        }
        if amount < 1 {
            return Err(GameError::invalid_input("amount must be a positive number"));
        }
        let wood = self.boards[player]
            .goods
            .get(&Good::Wood)
            .copied()
            .unwrap_or(0);
        if amount > wood {
            return Err(GameError::invalid_input(format!(
                "you only have {} wood left",
                wood
            )));
        }
        let cloth = self.boards[player]
            .goods
            .get(&Good::Cloth)
            .copied()
            .unwrap_or(0);
        if amount > cloth {
            return Err(GameError::invalid_input(format!(
                "you only have {} cloth left",
                cloth
            )));
        }
        if self.boards[player].ships + amount > 5 {
            return Err(GameError::invalid_input("you can only have 5 ships"));
        }
        self.boards[player].ships += amount;
        *self.boards[player].goods.entry(Good::Wood).or_insert(0) -= amount;
        *self.boards[player].goods.entry(Good::Cloth).or_insert(0) -= amount;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" built "),
            N::Bold(vec![N::text(format!("{} ships", amount))]),
        ])];
        if !self.can_build_or_trade(player) {
            logs.extend(self.next_phase());
        }
        Ok(logs)
    }

    /// Port of `BuildMonument`. Go's `ContainsInt(monument, Monuments)`
    /// validity check is always true here: `MonumentId` is a closed enum
    /// whose only possible values are the 7 real monuments (the parser's
    /// `Enum` choices are built from `available_monuments`, so an invalid
    /// variant can never reach this function) - not ported as a runtime
    /// check for that reason, matching how other closed-enum "invalid X"
    /// Go checks have been treated in prior Track B ports. The "who built
    /// it first" scan (Go quirk #5) is a simple flag scan, ported as-is;
    /// confirmed not a real defect since builds are strictly sequential per
    /// player (no two players can complete the same monument in the same
    /// call).
    fn build_monument(
        &mut self,
        player: usize,
        amount: i32,
        monument: MonumentId,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_build_building(player) {
            return Err(GameError::invalid_input("you can't build at the moment"));
        }
        if amount < 1 {
            return Err(GameError::invalid_input("amount must be a positive number"));
        }
        if amount > self.remaining_workers {
            return Err(GameError::invalid_input(format!(
                "you only have {} workers left",
                self.remaining_workers
            )));
        }
        let mv = monument.value();
        let cur = self.boards[player]
            .monuments
            .get(&monument)
            .copied()
            .unwrap_or(0);
        if cur + amount > mv.size {
            return Err(GameError::invalid_input(
                "that is more than what remains to be built",
            ));
        }
        self.remaining_workers -= amount;
        let new_progress = cur + amount;
        self.boards[player].monuments.insert(monument, new_progress);
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" used "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" workers on the "),
            N::Bold(vec![N::text(mv.name)]),
        ])];
        if new_progress >= mv.size {
            let first = !self
                .boards
                .iter()
                .any(|b| b.monument_built_first.contains(&monument));
            if first {
                self.boards[player].monument_built_first.insert(monument);
            }
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" completed the "),
                N::Bold(vec![N::text(mv.name)]),
            ]));
            logs.extend(self.check_game_end_triggered(player));
        }
        if !self.can_build_or_trade(player) {
            logs.extend(self.next_phase());
        }
        Ok(logs)
    }

    fn build_command(
        &mut self,
        player: usize,
        amount: i32,
        target: BuildTarget,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = match target {
            BuildTarget::City => self.build_city(player, amount),
            BuildTarget::Ship => self.build_ship(player, amount),
            BuildTarget::Monument(m) => self.build_monument(player, amount, m),
        }?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `trade`, ported from `trade_command.go` (stone -> workers, Build
    // phase, Engineering - NOT the goods `swap` command, see the plan's
    // Global Constraints on the `PhaseBuild`/`PhaseTrade` naming trap).
    // ---------------------------------------------------------------

    /// Port of `TradeStone`. Note Go has no `amount < 1` guard here (unlike
    /// most other commands) - preserved verbatim; the parser's
    /// `Int::bounded(1, max)` already prevents non-positive amounts from
    /// reaching this function in practice.
    fn trade_stone(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_trade(player) {
            return Err(GameError::invalid_input("you can't trade at the moment"));
        }
        let stone = self.boards[player]
            .goods
            .get(&Good::Stone)
            .copied()
            .unwrap_or(0);
        if amount > stone {
            return Err(GameError::invalid_input(format!(
                "you only have {} stone",
                stone
            )));
        }
        let workers = amount * 3;
        self.remaining_workers += workers;
        *self.boards[player].goods.entry(Good::Stone).or_insert(0) -= amount;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" traded "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" "),
            N::text(Good::Stone.name()),
            N::text(" for "),
            N::Bold(vec![N::text(format!("{} workers", workers))]),
        ])])
    }

    fn trade_command(
        &mut self,
        player: usize,
        amount: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.trade_stone(player, amount)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `buy`, ported from `buy_command.go`.
    // ---------------------------------------------------------------

    /// Port of `BuyDevelopment`. Go quirk #4 preserved verbatim: goods are
    /// selected by TYPE, not by unit - the entire held stack of each named
    /// good (deduped for totalling/suffix purposes) is spent/zeroed toward
    /// the cost, and the zeroing loop below deliberately iterates the
    /// original (possibly-duplicated) `goods` list rather than the deduped
    /// set, matching Go's `for _, good := range goods { ...Goods[good] = 0 }`.
    fn buy_development(
        &mut self,
        player: usize,
        development: DevelopmentId,
        goods: Vec<Good>,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_buy(player) {
            return Err(GameError::invalid_input("you can't buy at the moment"));
        }
        if self.boards[player].developments.contains(&development) {
            return Err(GameError::invalid_input(
                "you already have that development",
            ));
        }
        let dv = development.value();

        let mut total = self.remaining_coins;
        let mut used_goods: Vec<Good> = vec![];
        for &good in goods.iter() {
            if used_goods.contains(&good) {
                continue;
            }
            let n = self.boards[player].goods.get(&good).copied().unwrap_or(0);
            total += good_value(good, n);
            used_goods.push(good);
        }
        if total < dv.cost {
            return Err(GameError::invalid_input(format!(
                "you require {} but your coins and specified goods only amount to {}, you may need to add more goods",
                dv.cost, total
            )));
        }

        let mut suffix = String::new();
        if !used_goods.is_empty() {
            let parts: Vec<&str> = used_goods.iter().map(|g| g.name()).collect();
            suffix = format!(", using {}", parts.join(", "));
        }
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" bought the "),
            N::Bold(vec![N::text(format!("{} development", dv.name))]),
            N::text(suffix),
        ])];
        self.boards[player].developments.insert(development);
        for &good in goods.iter() {
            self.boards[player].goods.insert(good, 0);
        }

        logs.extend(self.check_game_end_triggered(player));
        logs.extend(self.next_phase());
        Ok(logs)
    }

    fn buy_command(
        &mut self,
        player: usize,
        development: DevelopmentId,
        goods: BuyGoods,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let good_list = if goods.all_goods {
            GOODS
                .iter()
                .copied()
                .filter(|g| self.boards[player].goods.get(g).copied().unwrap_or(0) > 0)
                .collect()
        } else {
            goods.goods
        };
        let logs = self.buy_development(player, development, good_list)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `take`, ported from `take_command.go`.
    // ---------------------------------------------------------------

    /// Port of `Take`. Go quirk: the "wrong number of actions" error
    /// message reports `len(actions)` (what the player actually supplied),
    /// not `numDice` (what was actually required) - re-read `take_command.go`
    /// closely: `if l := len(actions); l != numDice { return ...fmt.Errorf(
    /// "you must specify %d take actions...", l) }` uses `l`, which is
    /// `len(actions)`, in the message, even though the error only fires
    /// when `l != numDice` - so the message always echoes back the
    /// (wrong) count the player gave, not the count actually needed. This
    /// looks like a bug (the message should say `numDice`) but is
    /// preserved verbatim per the porting correctness rule.
    fn take(&mut self, player: usize, actions: Vec<TakeAction>) -> Result<Vec<Log>, GameError> {
        if !self.can_take(player) {
            return Err(GameError::invalid_input("you can't take at the moment"));
        }
        let num_dice = self
            .kept_dice
            .iter()
            .filter(|&&d| d == Die::FoodOrWorkers)
            .count();
        if actions.len() != num_dice {
            return Err(GameError::invalid_input(format!(
                "you must specify {} take actions after the take command",
                actions.len()
            )));
        }
        let cp = self.current_player;
        for a in actions {
            match a {
                TakeAction::Food => {
                    let modifier = self.boards[cp].food_modifier();
                    self.boards[cp].food += 2 + modifier;
                }
                TakeAction::Workers => {
                    let modifier = self.boards[cp].worker_modifier();
                    self.remaining_workers += 2 + modifier;
                }
            }
        }
        Ok(self.next_phase())
    }

    fn take_command(
        &mut self,
        player: usize,
        actions: Vec<TakeAction>,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.take(player, actions)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `discard`, ported from `discard_command.go`.
    // ---------------------------------------------------------------

    /// Port of `Discard`.
    fn discard(&mut self, player: usize, amount: i32, good: Good) -> Result<Vec<Log>, GameError> {
        if !self.can_discard(player) {
            return Err(GameError::invalid_input("you can't discard at the moment"));
        }
        if amount < 1 {
            return Err(GameError::invalid_input("amount must be a positive number"));
        }
        let num = self.boards[player].goods.get(&good).copied().unwrap_or(0);
        if amount > num {
            return Err(GameError::invalid_input(format!(
                "you only have {} {}",
                num,
                good.name()
            )));
        }
        let goods_over_limit = self.boards[player].goods_over_limit();
        if amount > goods_over_limit {
            return Err(GameError::invalid_input(format!(
                "you only need to discard {}",
                goods_over_limit
            )));
        }
        *self.boards[player].goods.entry(good).or_insert(0) -= amount;
        let logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" discarded "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" "),
            N::text(good.name()),
        ])];
        let mut logs = logs;
        if self.boards[player].goods_over_limit() <= 0 {
            logs.extend(self.next_turn());
        }
        Ok(logs)
    }

    fn discard_command(
        &mut self,
        player: usize,
        amount: i32,
        good: Good,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.discard(player, amount, good)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `invade`, ported from `invade_command.go`.
    // ---------------------------------------------------------------

    /// Port of `Invade`. Go quirk found while porting: the per-opponent
    /// log line reports `amount` disaster points, but the actual effect
    /// applied is `amount * 2` (`g.Boards[p].Disasters += amount * 2`
    /// vs. the log text `"%d disaster points", amount`) - the log
    /// understates the real damage by half. Preserved verbatim per the
    /// porting correctness rule (not in the plan's enumerated quirk list;
    /// recorded here and in the Task 3 report).
    fn invade(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_invade(player) {
            return Err(GameError::invalid_input("you can't invade at the moment"));
        }
        if amount <= 0 {
            return Err(GameError::invalid_input(
                "you must specify a positive amount of spearheads",
            ));
        }
        let sh = self.boards[player]
            .goods
            .get(&Good::Spearhead)
            .copied()
            .unwrap_or(0);
        if amount > sh {
            return Err(GameError::invalid_input(format!(
                "you only have {} spearheads",
                sh
            )));
        }
        *self.boards[player]
            .goods
            .entry(Good::Spearhead)
            .or_insert(0) -= amount;
        let mut content = vec![
            N::Player(player),
            N::text(" used "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" spearheads to cause extra damage"),
        ];
        for p in 0..self.players {
            if p == player {
                continue;
            }
            if self.boards[p].has_built(MonumentId::GreatWall) {
                content.push(N::text("\n  "));
                content.push(N::Player(p));
                content.push(N::text(" avoids the extra damage with their wall"));
            } else {
                self.boards[p].disasters += amount * 2;
                content.push(N::text("\n  "));
                content.push(N::Player(p));
                content.push(N::text(" takes "));
                content.push(N::Bold(vec![N::text(format!(
                    "{} disaster points",
                    amount
                ))]));
            }
        }
        let mut logs = vec![Log::public(content)];
        logs.extend(self.next_phase());
        Ok(logs)
    }

    fn invade_command(
        &mut self,
        player: usize,
        amount: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.invade(player, amount)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `sell`, ported from `sell_command.go`.
    // ---------------------------------------------------------------

    /// Port of `SellFood`. Note Go has no `amount < 1` guard here - only
    /// checks against held food - preserved verbatim; the parser already
    /// bounds `amount` to `1..=food`.
    fn sell_food(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_sell(player) {
            return Err(GameError::invalid_input("you can't sell at the moment"));
        }
        if amount > self.boards[player].food {
            return Err(GameError::invalid_input(format!(
                "you only have {} food",
                self.boards[player].food
            )));
        }
        let coins = amount * 6;
        self.remaining_coins += coins;
        self.boards[player].food -= amount;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" sold "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" food for "),
            N::Bold(vec![N::text(format!("{} coins", coins))]),
        ])])
    }

    fn sell_command(
        &mut self,
        player: usize,
        amount: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.sell_food(player, amount)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    // ---------------------------------------------------------------
    // `swap`, ported from `swap_command.go`.
    // ---------------------------------------------------------------

    /// Port of `Swap`.
    fn swap(
        &mut self,
        player: usize,
        from: Good,
        to: Good,
        amount: i32,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_swap(player) {
            return Err(GameError::invalid_input("you can't swap at the moment"));
        }
        if amount < 1 {
            return Err(GameError::invalid_input("amount must be positive"));
        }
        if from == to {
            return Err(GameError::invalid_input(
                "you must specify two different goods",
            ));
        }
        if amount > self.remaining_ships {
            return Err(GameError::invalid_input(format!(
                "you only have {} ships remaining",
                self.remaining_ships
            )));
        }
        let from_num = self.boards[player].goods.get(&from).copied().unwrap_or(0);
        if from_num < amount {
            return Err(GameError::invalid_input(format!(
                "you only have {} {} left",
                from_num,
                from.name()
            )));
        }
        let max = good_maximum(to);
        let to_num = self.boards[player].goods.get(&to).copied().unwrap_or(0);
        if to_num + amount > max {
            return Err(GameError::invalid_input(format!(
                "the you only have room for {} {}",
                max,
                to.name()
            )));
        }
        *self.boards[player].goods.entry(from).or_insert(0) -= amount;
        *self.boards[player].goods.entry(to).or_insert(0) += amount;
        self.remaining_ships -= amount;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" swapped "),
            N::Bold(vec![N::text(amount.to_string())]),
            N::text(" "),
            N::text(from.name()),
            N::text(" for "),
            N::text(to.name()),
        ])];
        if self.remaining_ships == 0 {
            logs.extend(self.next_phase());
        }
        Ok(logs)
    }

    fn swap_command(
        &mut self,
        player: usize,
        from: Good,
        to: Good,
        amount: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        let logs = self.swap(player, from, to, amount)?;
        Ok(CommandResponse {
            logs,
            can_undo: self.current_player == player,
            remaining_input: remaining.to_string(),
        })
    }

    /// Port of `Points` (per-turn running score, always computed directly
    /// from `Score()`, not zero-until-finished).
    fn scores(&self) -> Vec<i32> {
        self.boards.iter().map(|b| b.score()).collect()
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
        let mut g = Game {
            players,
            boards: (0..players).map(|_| PlayerBoard::default()).collect(),
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        let logs = g.start_turn();
        Ok((g, logs))
    }

    fn pub_state(&self) -> Self::PubState {
        PubState { game: self.clone() }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            game: self.clone(),
            player,
        }
    }

    /// Port of `Command`. Follows the borrow-order gotcha: `command_parser`
    /// is never bound to a `let` before the mutating action call - the
    /// parser-construction-then-parse expression is inlined so the
    /// immutable borrow of `self` ends before any `&mut self` action runs.
    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = match self.command_parser(player) {
            Some(cp) => cp,
            None => return Err(GameError::invalid_input("you have no commands available")),
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                value: Command::Next,
                remaining,
                ..
            }) => self.next_command(player, remaining),
            Ok(ParseOutput {
                value: Command::Roll { dice },
                remaining,
                ..
            }) => self.roll_command(player, dice, remaining),
            Ok(ParseOutput {
                value: Command::Preserve,
                remaining,
                ..
            }) => self.preserve_command(player, remaining),
            Ok(ParseOutput {
                value: Command::Build { amount, target },
                remaining,
                ..
            }) => self.build_command(player, amount, target, remaining),
            Ok(ParseOutput {
                value: Command::Trade { amount },
                remaining,
                ..
            }) => self.trade_command(player, amount, remaining),
            Ok(ParseOutput {
                value: Command::Buy { development, goods },
                remaining,
                ..
            }) => self.buy_command(player, development, goods, remaining),
            Ok(ParseOutput {
                value: Command::Take { actions },
                remaining,
                ..
            }) => self.take_command(player, actions, remaining),
            Ok(ParseOutput {
                value: Command::Discard { amount, good },
                remaining,
                ..
            }) => self.discard_command(player, amount, good, remaining),
            Ok(ParseOutput {
                value: Command::Invade { amount },
                remaining,
                ..
            }) => self.invade_command(player, amount, remaining),
            Ok(ParseOutput {
                value: Command::Sell { amount },
                remaining,
                ..
            }) => self.sell_command(player, amount, remaining),
            Ok(ParseOutput {
                value: Command::Swap { amount, from, to },
                remaining,
                ..
            }) => self.swap_command(player, from, to, amount, remaining),
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    /// Port of `Status`. `Winners()` (Go) implements its own two-stage
    /// tie-break (score, then goods value) but is confirmed dead code: it
    /// is not called anywhere else in `roll_through_the_ages_1` (grepped
    /// the whole package), not called anywhere else in `brdgme-go` (not
    /// part of the `brdgme.Gamer` interface either - `liars_dice_1` and
    /// `texas_holdem_1` each define their own unrelated `Winners()` too, so
    /// it's a per-game convention method, not framework plumbing), and
    /// `Status()` uses `brdgme.GenPlacings(scores)` with a plain
    /// single-value `[]int{Score()}` metric per player - no goods-value
    /// tiebreaker at all. So this port intentionally does NOT carry over
    /// `Winners()`'s extra goods-value tiebreaker; `gen_placings` here is
    /// given the single-value score metric, matching what `Status()`
    /// actually uses.
    fn status(&self) -> Status {
        if self.finished {
            let metrics: Vec<Vec<i32>> = self.scores().into_iter().map(|s| vec![s]).collect();
            Status::Finished {
                placings: gen_placings(&metrics),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: self.whose_turn(),
                eliminated: vec![],
            }
        }
    }

    fn points(&self) -> Vec<f32> {
        self.scores().into_iter().map(|s| s as f32).collect()
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4]
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
pub(crate) mod test_helpers {
    use super::*;

    pub const MICK: usize = 0;
    pub const STEVE: usize = 1;
    pub const BJ: usize = 2;

    pub fn test_players() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()]
    }

    /// Port of the shared `NewBlank(players)` Go test helper (`game_test.go`):
    /// bypasses the normal `start()`/`StartTurn` dice roll, sets
    /// `current_player=Mick(0)`, `phase=Roll`, `remaining_rolls=2` directly,
    /// so tests stay deterministic without needing to seed/mock the RNG for
    /// every case.
    pub fn new_blank(players: usize) -> Game {
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&players));
        Game {
            players,
            boards: (0..players).map(|_| PlayerBoard::default()).collect(),
            current_player: MICK,
            phase: Phase::Roll,
            remaining_rolls: 2,
            ..Game::default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::test_helpers::*;
    use super::*;

    // ---------------------------------------------------------------
    // Baseline: start()/player_counts.
    // ---------------------------------------------------------------

    #[test]
    fn start_rejects_invalid_player_counts() {
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(5, 1).is_err());
    }

    #[test]
    fn start_ok_for_valid_player_counts_and_initial_state() {
        for n in 2..=4 {
            let (g, _) = Game::start(n, 1).unwrap();
            assert_eq!(n, g.boards.len());
            assert_eq!(0, g.current_player);
            for b in g.boards.iter() {
                assert_eq!(3, b.food);
                assert!(b.developments.is_empty());
                assert!(b.monuments.is_empty());
                assert!(b.goods.is_empty());
            }
        }
    }

    // Port of TestGame_KeepSkulls_allDisasterSkip (game_test.go).
    #[test]
    fn test_game_keep_skulls_all_disaster_skip() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Skull, Die::Skull, Die::Skull];
        let res = g.command(MICK, "next", &test_players());
        assert!(res.is_ok());
        assert_eq!(MICK, g.current_player);
        assert_eq!(Phase::Buy, g.phase);
    }

    // Port of TestGame_KeepSkulls_allDisasterLeadership (game_test.go).
    #[test]
    fn test_game_keep_skulls_all_disaster_leadership() {
        let mut g = new_blank(3);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        g.rolled_dice = vec![Die::Skull, Die::Skull, Die::Skull];
        let res = g.command(MICK, "next", &test_players());
        assert!(res.is_ok());
        assert_eq!(MICK, g.current_player);
        assert_eq!(Phase::ExtraRoll, g.phase);
    }

    // Port of TestRollCommand (roll_command_test.go).
    #[test]
    fn test_roll_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Coins; 7];
        let res = g.command(MICK, "roll 2 4 7", &test_players());
        assert!(res.is_ok());
    }

    // Port of TestRollExtraCommand (roll_command_test.go).
    #[test]
    fn test_roll_extra_command() {
        let mut g = new_blank(3);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        g.rolled_dice = vec![Die::Coins; 4];
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull];
        let res = g.command(MICK, "roll 7", &test_players());
        assert!(res.is_err());
        let res = g.command(MICK, "next", &test_players());
        assert!(res.is_ok());
        let res = g.command(MICK, "roll 7", &test_players());
        assert!(res.is_ok());
    }

    // Port of TestNextCommandPreserve (next_command_test.go).
    #[test]
    fn test_next_command_preserve() {
        let mut g = new_blank(3);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Preservation);
        g.boards[MICK].goods.insert(Good::Pottery, 3);
        g.boards[MICK].food = 3;
        g.phase = Phase::Preserve;
        let res = g.command(MICK, "next", &test_players());
        assert!(res.is_ok());
        assert_ne!(Phase::Preserve, g.phase);
    }

    // Port of TestNextCommand (next_command_test.go).
    #[test]
    fn test_next_command() {
        let mut g = new_blank(3);
        // For reroll.
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        // For invade.
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        // For trade.
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].ships = 3;
        assert_eq!(Phase::Roll, g.phase);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull, Die::Skull, Die::Workers];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::ExtraRoll, g.phase);
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Invade, g.phase);
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Build, g.phase);
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Trade, g.phase);
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Buy, g.phase);
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Discard, g.phase);
    }

    // Port of TestPreserveCommand (preserve_command_test.go).
    #[test]
    fn test_preserve_command() {
        let mut g = new_blank(3);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Preservation);
        g.boards[MICK].goods.insert(Good::Pottery, 3);
        g.boards[MICK].food = 3;
        g.phase = Phase::Preserve;
        let res = g.command(MICK, "preserve", &test_players());
        assert!(res.is_ok());
        assert_eq!(6, g.boards[MICK].food);
    }

    // ---------------------------------------------------------------
    // Baseline: individual phase-skip conditions.
    // ---------------------------------------------------------------

    #[test]
    fn preserve_phase_skips_when_cannot_preserve() {
        let mut g = new_blank(2);
        g.phase = Phase::Discard; // arbitrary prior phase
        let logs = g.preserve_phase();
        assert_ne!(Phase::Preserve, g.phase);
        assert!(!logs.is_empty() || g.phase != Phase::Preserve);
    }

    #[test]
    fn roll_extra_phase_skips_without_leadership() {
        let mut g = new_blank(2);
        g.kept_dice = vec![Die::Coins];
        g.rolled_dice = vec![];
        g.phase = Phase::Roll;
        g.roll_extra_phase();
        assert_ne!(Phase::ExtraRoll, g.phase);
    }

    #[test]
    fn roll_extra_phase_stays_with_leadership() {
        let mut g = new_blank(2);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        g.kept_dice = vec![Die::Coins];
        g.rolled_dice = vec![];
        g.roll_extra_phase();
        assert_eq!(Phase::ExtraRoll, g.phase);
    }

    #[test]
    fn collect_phase_skips_without_food_or_workers_die() {
        let mut g = new_blank(2);
        g.rolled_dice = vec![Die::Coins, Die::Good];
        g.collect_phase();
        assert_ne!(Phase::Collect, g.phase);
    }

    #[test]
    fn collect_phase_stays_with_food_or_workers_die() {
        let mut g = new_blank(2);
        g.rolled_dice = vec![Die::FoodOrWorkers];
        g.collect_phase();
        assert_eq!(Phase::Collect, g.phase);
    }

    #[test]
    fn invade_phase_skips_when_cannot_invade() {
        let mut g = new_blank(2);
        g.invade_phase();
        assert_ne!(Phase::Invade, g.phase);
    }

    #[test]
    fn invade_phase_stays_when_can_invade() {
        let mut g = new_blank(2);
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        g.phase = Phase::Collect;
        g.invade_phase();
        assert_eq!(Phase::Invade, g.phase);
    }

    #[test]
    fn build_phase_skips_when_cannot_build_or_trade() {
        let mut g = new_blank(2);
        g.remaining_workers = 0;
        g.build_phase();
        assert_ne!(Phase::Build, g.phase);
    }

    #[test]
    fn build_phase_stays_with_workers() {
        let mut g = new_blank(2);
        g.remaining_workers = 1;
        g.build_phase();
        assert_eq!(Phase::Build, g.phase);
    }

    #[test]
    fn trade_phase_skips_without_ships_or_goods() {
        let mut g = new_blank(2);
        g.trade_phase();
        assert_ne!(Phase::Trade, g.phase);
    }

    #[test]
    fn trade_phase_stays_with_ships_and_goods() {
        let mut g = new_blank(2);
        g.boards[MICK].ships = 2;
        g.boards[MICK].goods.insert(Good::Wood, 1);
        g.phase = Phase::Build;
        g.trade_phase();
        assert_eq!(Phase::Trade, g.phase);
    }

    #[test]
    fn buy_phase_skips_when_buying_power_below_ten() {
        let mut g = new_blank(2);
        g.remaining_coins = 5;
        g.buy_phase();
        assert_ne!(Phase::Buy, g.phase);
    }

    #[test]
    fn buy_phase_stays_when_buying_power_at_least_ten() {
        let mut g = new_blank(2);
        g.remaining_coins = 10;
        g.buy_phase();
        assert_eq!(Phase::Buy, g.phase);
    }

    #[test]
    fn discard_phase_skips_at_or_under_limit() {
        let mut g = new_blank(2);
        g.discard_phase();
        assert_ne!(Phase::Discard, g.phase);
    }

    #[test]
    fn discard_phase_skips_with_caravans_over_limit() {
        let mut g = new_blank(2);
        g.boards[MICK].developments.insert(DevelopmentId::Caravans);
        g.boards[MICK].goods.insert(Good::Wood, 8);
        g.discard_phase();
        assert_ne!(Phase::Discard, g.phase);
    }

    #[test]
    fn discard_phase_stays_over_limit_without_caravans() {
        let mut g = new_blank(2);
        g.boards[MICK].goods.insert(Good::Wood, 8);
        g.phase = Phase::Buy;
        g.discard_phase();
        assert_eq!(Phase::Discard, g.phase);
    }

    // ---------------------------------------------------------------
    // Baseline: resolve-phase disaster-dice logic.
    // ---------------------------------------------------------------

    #[test]
    fn resolve_no_disaster_for_zero_or_one_skull() {
        let mut g = new_blank(2);
        g.kept_dice = vec![Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[MICK].disasters);
    }

    #[test]
    fn resolve_drought_without_irrigation() {
        let mut g = new_blank(2);
        g.kept_dice = vec![Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(2, g.boards[MICK].disasters);
    }

    #[test]
    fn resolve_drought_avoided_with_irrigation() {
        let mut g = new_blank(2);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Irrigation);
        g.kept_dice = vec![Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[MICK].disasters);
    }

    #[test]
    fn resolve_pestilence_hits_other_players_without_medicine() {
        let mut g = new_blank(3);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[MICK].disasters);
        assert_eq!(3, g.boards[STEVE].disasters);
        assert_eq!(3, g.boards[BJ].disasters);
    }

    #[test]
    fn resolve_pestilence_avoided_per_player_with_medicine() {
        let mut g = new_blank(3);
        g.boards[STEVE].developments.insert(DevelopmentId::Medicine);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[STEVE].disasters);
        assert_eq!(3, g.boards[BJ].disasters);
    }

    #[test]
    fn resolve_invasion_hits_self_without_smithing_or_wall() {
        let mut g = new_blank(2);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(4, g.boards[MICK].disasters);
    }

    #[test]
    fn resolve_invasion_avoided_self_with_wall() {
        let mut g = new_blank(2);
        g.boards[MICK].monuments.insert(MonumentId::GreatWall, 13);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[MICK].disasters);
    }

    #[test]
    fn resolve_invasion_with_smithing_hits_all_others() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        // Mick also needs spearheads for `can_invade` to keep the cascade
        // parked on `PhaseInvade` (i.e. an *attacker* with no spearheads to
        // spend auto-skips straight through it, same as any other guard).
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[MICK].disasters);
        assert_eq!(4, g.boards[STEVE].disasters);
        assert_eq!(4, g.boards[BJ].disasters);
        assert_eq!(Phase::Invade, g.phase);
    }

    #[test]
    fn resolve_invasion_with_smithing_wall_blocks_per_opponent() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[STEVE].monuments.insert(MonumentId::GreatWall, 13);
        g.kept_dice = vec![Die::Skull, Die::Skull, Die::Skull, Die::Skull];
        g.phase_resolve();
        assert_eq!(0, g.boards[STEVE].disasters);
        assert_eq!(4, g.boards[BJ].disasters);
    }

    #[test]
    fn resolve_revolt_wipes_own_goods_without_religion() {
        let mut g = new_blank(2);
        g.boards[MICK].goods.insert(Good::Wood, 3);
        g.kept_dice = vec![Die::Skull; 5];
        g.phase_resolve();
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Wood));
    }

    #[test]
    fn resolve_revolt_wipes_all_others_goods_with_religion() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Religion);
        g.boards[MICK].goods.insert(Good::Wood, 3);
        g.boards[STEVE].goods.insert(Good::Wood, 2);
        g.boards[BJ].goods.insert(Good::Wood, 1);
        g.kept_dice = vec![Die::Skull; 5];
        g.phase_resolve();
        assert_eq!(Some(&3), g.boards[MICK].goods.get(&Good::Wood));
        assert_eq!(Some(&0), g.boards[STEVE].goods.get(&Good::Wood));
        assert_eq!(Some(&0), g.boards[BJ].goods.get(&Good::Wood));
    }

    // ---------------------------------------------------------------
    // Baseline: game-end triggering, next_turn, status/points/whose_turn.
    // ---------------------------------------------------------------

    #[test]
    fn check_game_end_triggered_on_seventh_development() {
        let mut g = new_blank(2);
        for d in [
            DevelopmentId::Leadership,
            DevelopmentId::Irrigation,
            DevelopmentId::Agriculture,
            DevelopmentId::Quarrying,
            DevelopmentId::Medicine,
            DevelopmentId::Preservation,
            DevelopmentId::Coinage,
        ] {
            g.boards[MICK].developments.insert(d);
        }
        assert!(!g.final_round);
        g.check_game_end_triggered(MICK);
        assert!(g.final_round);
    }

    #[test]
    fn check_game_end_not_triggered_under_seven_developments() {
        let mut g = new_blank(2);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        g.check_game_end_triggered(MICK);
        assert!(!g.final_round);
    }

    #[test]
    fn check_game_end_triggered_when_all_monuments_built() {
        let mut g = new_blank(2);
        for &m in MONUMENTS.iter() {
            g.boards[MICK].monuments.insert(m, m.value().size);
        }
        g.check_game_end_triggered(MICK);
        assert!(g.final_round);
    }

    #[test]
    fn check_game_end_not_triggered_when_one_monument_incomplete() {
        let mut g = new_blank(2);
        for &m in MONUMENTS.iter().take(MONUMENTS.len() - 1) {
            g.boards[MICK].monuments.insert(m, m.value().size);
        }
        g.check_game_end_triggered(MICK);
        assert!(!g.final_round);
    }

    #[test]
    fn check_game_end_already_triggered_is_a_no_op() {
        let mut g = new_blank(2);
        g.final_round = true;
        let logs = g.check_game_end_triggered(MICK);
        assert!(logs.is_empty());
    }

    #[test]
    fn next_turn_wraps_player_index() {
        let mut g = new_blank(3);
        g.current_player = BJ;
        g.next_turn();
        assert_eq!(MICK, g.current_player);
    }

    #[test]
    fn next_turn_finishes_game_on_wraparound_during_final_round() {
        let mut g = new_blank(2);
        g.current_player = 1;
        g.final_round = true;
        g.next_turn();
        assert_eq!(0, g.current_player);
        assert!(g.finished);
    }

    #[test]
    fn next_turn_does_not_finish_without_final_round() {
        let mut g = new_blank(2);
        g.current_player = 1;
        g.next_turn();
        assert_eq!(0, g.current_player);
        assert!(!g.finished);
    }

    #[test]
    fn whose_turn_and_status_active_during_play() {
        let g = new_blank(2);
        assert_eq!(vec![0], g.whose_turn());
        match g.status() {
            Status::Active { whose_turn, .. } => assert_eq!(vec![0], whose_turn),
            _ => panic!("expected Active status"),
        }
    }

    #[test]
    fn status_finished_uses_plain_score_metric_no_goods_tiebreak() {
        let mut g = new_blank(2);
        g.finished = true;
        // Tie on score but NOT on goods value - per the plan's dead-code
        // investigation, `Status()` does not use `Winners()`'s goods-value
        // tiebreaker, so both players should be tied for 1st here even
        // though their goods values differ.
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership); // 2 pts
        g.boards[STEVE]
            .developments
            .insert(DevelopmentId::Irrigation); // 2 pts
        g.boards[STEVE].goods.insert(Good::Spearhead, 4); // high goods value, irrelevant
        match g.status() {
            Status::Finished { placings, .. } => {
                assert_eq!(1, placings[MICK]);
                assert_eq!(1, placings[STEVE]);
            }
            _ => panic!("expected Finished status"),
        }
    }

    #[test]
    fn status_finished_places_higher_score_first() {
        let mut g = new_blank(2);
        g.finished = true;
        g.boards[MICK].developments.insert(DevelopmentId::Empire); // 10 pts + cities
        match g.status() {
            Status::Finished { placings, .. } => {
                assert_eq!(1, placings[MICK]);
                assert_eq!(2, placings[STEVE]);
            }
            _ => panic!("expected Finished status"),
        }
    }

    #[test]
    fn points_returns_raw_current_score_at_all_times() {
        let mut g = new_blank(2);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        let points = g.points();
        assert_eq!(2.0 + g.boards[MICK].cities() as f32 * 0.0, points[MICK]);
        assert_eq!(0.0, points[STEVE]);
        // Not finished, but points still reflects current score (unlike
        // for-sale-2's zero-until-finished convention).
        assert!(!g.finished);
    }

    // ---------------------------------------------------------------
    // Integration: a full deterministic multi-phase command() sequence.
    // ---------------------------------------------------------------

    #[test]
    fn integration_next_and_roll_cascade_through_multiple_phases() {
        let mut g = new_blank(2);
        let p = vec!["Mick".to_string(), "Steve".to_string()];
        // `new_blank` starts at PhaseRoll with no dice rolled, so `next` is
        // the only legal command (CanRoll requires a non-empty
        // RolledDice); a single kept Workers die drives the cascade
        // Roll -> ExtraRoll -> Collect (collects 3 workers) -> Resolve (no
        // skulls) -> Build (stays, since RemainingWorkers > 0), all without
        // ever touching the RNG (no fresh dice are rolled along the way).
        g.kept_dice = vec![Die::Workers];
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Build, g.phase);
        assert_eq!(3, g.remaining_workers); // 3 base + 0 masonry modifier
        assert_eq!(MICK, g.current_player);
    }

    // =================================================================
    // Task 3: build/buy/discard/invade/sell/swap/take/trade.
    // =================================================================

    // Port of TestBuildCityCommand (build_command_test.go).
    #[test]
    fn test_build_city_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Workers, Die::Workers, Die::FoodOrWorkers];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "take w", &p).is_ok());
        assert!(g.command(MICK, "build 8 city", &p).is_ok());
        assert_eq!(8, g.boards[MICK].city_progress);
    }

    // Port of TestBuildMonumentCommand (build_command_test.go).
    #[test]
    fn test_build_monument_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![
            Die::Workers,
            Die::Workers,
            Die::FoodOrWorkers,
            Die::FoodOrWorkers,
        ];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "take w w", &p).is_ok());
        assert!(g.command(MICK, "build 10 wall", &p).is_ok());
        assert_eq!(
            10,
            g.boards[MICK]
                .monuments
                .get(&MonumentId::GreatWall)
                .copied()
                .unwrap_or(0)
        );
    }

    // Port of TestBuildShipCommand (build_command_test.go).
    #[test]
    fn test_build_ship_command() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Cloth, 2);
        g.boards[MICK].goods.insert(Good::Wood, 3);
        g.rolled_dice = vec![
            Die::Workers,
            Die::Workers,
            Die::FoodOrWorkers,
            Die::FoodOrWorkers,
        ];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "take w f", &p).is_ok());
        assert!(g.command(MICK, "build 3 ship", &p).is_err());
        assert!(g.command(MICK, "build 2 ship", &p).is_ok());
        assert_eq!(2, g.boards[MICK].ships);
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Cloth));
        assert_eq!(Some(&1), g.boards[MICK].goods.get(&Good::Wood));
    }

    // Port of TestBuyCommandCoins (buy_command_test.go).
    #[test]
    fn test_buy_command_coins() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Coins, Die::Coins];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "buy leader", &p).is_ok());
        assert!(
            g.boards[MICK]
                .developments
                .contains(&DevelopmentId::Leadership)
        );
        assert_ne!(Phase::Buy, g.phase);
    }

    // Port of TestBuyCommandCoinsWithCoinage (buy_command_test.go).
    #[test]
    fn test_buy_command_coins_with_coinage() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Coins, Die::Coins];
        g.boards[MICK].developments.insert(DevelopmentId::Coinage);
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "buy cara", &p).is_ok());
        assert!(
            g.boards[MICK]
                .developments
                .contains(&DevelopmentId::Caravans)
        );
        assert_ne!(Phase::Buy, g.phase);
    }

    // Port of TestBuyCommandGoodsSpecific (buy_command_test.go).
    #[test]
    fn test_buy_command_goods_specific() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Good; 6];
        g.boards[MICK].developments.insert(DevelopmentId::Coinage);
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(
            g.command(MICK, "buy agri wood stone pot cloth spear", &p)
                .is_ok()
        );
        assert!(
            g.boards[MICK]
                .developments
                .contains(&DevelopmentId::Agriculture)
        );
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Wood));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Stone));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Pottery));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Cloth));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Spearhead));
        assert_ne!(Phase::Buy, g.phase);
    }

    // Port of TestBuyCommandGoodsAll (buy_command_test.go).
    #[test]
    fn test_buy_command_goods_all() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Good; 6];
        g.boards[MICK].developments.insert(DevelopmentId::Coinage);
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "buy agri all", &p).is_ok());
        assert!(
            g.boards[MICK]
                .developments
                .contains(&DevelopmentId::Agriculture)
        );
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Wood));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Stone));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Pottery));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Cloth));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Spearhead));
        assert_ne!(Phase::Buy, g.phase);
    }

    // Port of TestTakeCommand (take_command_test.go).
    #[test]
    fn test_take_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![
            Die::Workers,
            Die::Workers,
            Die::FoodOrWorkers,
            Die::FoodOrWorkers,
            Die::FoodOrWorkers,
        ];
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "take w w f", &p).is_ok());
        assert!(g.command(MICK, "build 10 city", &p).is_ok());
        assert_eq!(10, g.boards[MICK].city_progress);
    }

    // Port of TestDiscardCommand (discard_command_test.go).
    #[test]
    fn test_discard_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![
            Die::Skull,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
        ];
        let p = test_players();
        // Keep dice
        assert!(g.command(MICK, "next", &p).is_ok());
        // Skip buy phase
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Discard, g.phase);
        assert!(g.command(MICK, "discard 1 wood", &p).is_ok());
        assert!(g.command(MICK, "discard 1 spear", &p).is_ok());
        assert_eq!(Some(&1), g.boards[MICK].goods.get(&Good::Wood));
        assert_eq!(Some(&2), g.boards[MICK].goods.get(&Good::Stone));
        assert_eq!(Some(&2), g.boards[MICK].goods.get(&Good::Pottery));
        assert_eq!(Some(&1), g.boards[MICK].goods.get(&Good::Cloth));
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Spearhead));
    }

    // Port of TestInvadeCommand (invade_command_test.go).
    #[test]
    fn test_invade_command() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.rolled_dice = vec![
            Die::Skull,
            Die::Skull,
            Die::Skull,
            Die::Skull,
            Die::Good,
            Die::Good,
        ];
        let p = test_players();
        // Keep dice
        assert!(g.command(MICK, "next", &p).is_ok());
        assert_eq!(Phase::Invade, g.phase);
        assert_eq!(4, g.boards[STEVE].disasters);
        assert_eq!(4, g.boards[BJ].disasters);
        assert!(g.command(MICK, "invade 2", &p).is_ok());
        assert_eq!(8, g.boards[STEVE].disasters);
        assert_eq!(8, g.boards[BJ].disasters);
    }

    // Port of TestSellCommand (sell_command_test.go).
    #[test]
    fn test_sell_command() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Granaries);
        g.boards[MICK].food = 10; // Will need to feed 3 cities
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "sell 5", &p).is_ok());
        assert_eq!(2, g.boards[MICK].food);
        assert_eq!(30, g.remaining_coins);
    }

    // Port of TestSwapCommand (swap_command_test.go).
    #[test]
    fn test_swap_command() {
        let mut g = new_blank(3);
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].ships = 3;
        g.rolled_dice = vec![
            Die::Skull,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
            Die::Good,
        ];
        let p = test_players();
        // Keep dice
        assert!(g.command(MICK, "next", &p).is_ok());
        // Skip build
        assert!(g.command(MICK, "next", &p).is_ok());
        // Swap all wood for spearheads
        assert!(g.command(MICK, "swap 2 wood spear", &p).is_ok());
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Wood));
        assert_eq!(Some(&3), g.boards[MICK].goods.get(&Good::Spearhead));
    }

    // Port of TestTradeCommand (trade_command_test.go).
    #[test]
    fn test_trade_command() {
        let mut g = new_blank(3);
        g.rolled_dice = vec![Die::Food, Die::Food, Die::Food];
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Engineering);
        g.boards[MICK].goods.insert(Good::Stone, 3);
        let p = test_players();
        assert!(g.command(MICK, "next", &p).is_ok());
        assert!(g.command(MICK, "trade 3", &p).is_ok());
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Stone));
        assert!(g.command(MICK, "build 9 great", &p).is_ok());
        assert_eq!(
            9,
            g.boards[MICK]
                .monuments
                .get(&MonumentId::GreatPyramid)
                .copied()
                .unwrap_or(0)
        );
    }

    // -----------------------------------------------------------------
    // Baseline: guard functions (Can*) for the Task 3 commands.
    // -----------------------------------------------------------------

    #[test]
    fn can_build_building_requires_build_phase_and_workers() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 0;
        assert!(!g.can_build_building(MICK));
        g.remaining_workers = 1;
        assert!(g.can_build_building(MICK));
    }

    #[test]
    fn can_build_ship_requires_shipping_and_goods() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        assert!(!g.can_build_ship(MICK));
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        assert!(!g.can_build_ship(MICK)); // no wood/cloth yet
        g.boards[MICK].goods.insert(Good::Wood, 1);
        g.boards[MICK].goods.insert(Good::Cloth, 1);
        assert!(g.can_build_ship(MICK));
    }

    #[test]
    fn can_trade_requires_build_phase_not_trade_phase() {
        let mut g = new_blank(2);
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Engineering);
        g.boards[MICK].goods.insert(Good::Stone, 1);
        g.phase = Phase::Trade;
        assert!(!g.can_trade(MICK)); // trade command needs PhaseBuild, not PhaseTrade
        g.phase = Phase::Build;
        assert!(g.can_trade(MICK));
    }

    #[test]
    fn can_buy_requires_buy_phase() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        assert!(!g.can_buy(MICK));
        g.phase = Phase::Buy;
        assert!(g.can_buy(MICK));
    }

    #[test]
    fn can_take_requires_collect_phase() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        assert!(!g.can_take(MICK));
        g.phase = Phase::Collect;
        assert!(g.can_take(MICK));
    }

    #[test]
    fn can_discard_requires_over_limit() {
        let mut g = new_blank(2);
        g.phase = Phase::Discard;
        assert!(!g.can_discard(MICK));
        g.boards[MICK].goods.insert(Good::Wood, 8);
        assert!(g.can_discard(MICK));
    }

    #[test]
    fn can_invade_requires_smithing_and_spearheads() {
        let mut g = new_blank(2);
        g.phase = Phase::Invade;
        assert!(!g.can_invade(MICK));
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        assert!(!g.can_invade(MICK));
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        assert!(g.can_invade(MICK));
    }

    #[test]
    fn can_sell_requires_granaries_and_food() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        assert!(!g.can_sell(MICK));
        g.boards[MICK].developments.insert(DevelopmentId::Granaries);
        // PlayerBoard::default() sets food=3, so with Granaries now present
        // this already satisfies CanSell - explicitly zero it first to
        // exercise the food>0 guard.
        g.boards[MICK].food = 0;
        assert!(!g.can_sell(MICK));
        g.boards[MICK].food = 1;
        assert!(g.can_sell(MICK));
    }

    #[test]
    fn can_swap_requires_shipping_and_goods() {
        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        assert!(!g.can_swap(MICK));
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        assert!(!g.can_swap(MICK));
        g.boards[MICK].goods.insert(Good::Wood, 1);
        assert!(g.can_swap(MICK));
    }

    // -----------------------------------------------------------------
    // Baseline: build error paths.
    // -----------------------------------------------------------------

    #[test]
    fn build_city_rejects_over_max_progress() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        g.boards[MICK].city_progress = 10;
        assert!(g.build_city(MICK, 10).is_err());
    }

    #[test]
    fn build_city_rejects_more_than_remaining_workers() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 2;
        assert!(g.build_city(MICK, 3).is_err());
    }

    #[test]
    fn build_monument_rejects_over_size() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        assert!(g.build_monument(MICK, 4, MonumentId::StepPyramid).is_err()); // size 3
    }

    #[test]
    fn build_monument_first_flag_set_only_for_first_builder() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        g.build_monument(MICK, 3, MonumentId::StepPyramid).unwrap();
        assert!(
            g.boards[MICK]
                .monument_built_first
                .contains(&MonumentId::StepPyramid)
        );

        g.current_player = STEVE;
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        g.build_monument(STEVE, 3, MonumentId::StepPyramid).unwrap();
        assert!(
            !g.boards[STEVE]
                .monument_built_first
                .contains(&MonumentId::StepPyramid)
        );
    }

    #[test]
    fn build_monument_completion_triggers_game_end_check() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        for &m in MONUMENTS.iter().filter(|&&m| m != MonumentId::StepPyramid) {
            g.boards[MICK].monuments.insert(m, m.value().size);
        }
        assert!(!g.final_round);
        g.build_monument(MICK, 3, MonumentId::StepPyramid).unwrap();
        assert!(g.final_round);
    }

    #[test]
    fn build_ship_rejects_over_five_cap() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 6);
        g.boards[MICK].goods.insert(Good::Cloth, 6);
        g.boards[MICK].ships = 4;
        assert!(g.build_ship(MICK, 2).is_err());
    }

    #[test]
    fn build_ship_rejects_insufficient_wood_or_cloth() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 1);
        assert!(g.build_ship(MICK, 1).is_err());
    }

    // -----------------------------------------------------------------
    // Baseline: trade/buy/take/discard/invade/sell/swap error paths.
    // -----------------------------------------------------------------

    #[test]
    fn trade_stone_rejects_insufficient_stone() {
        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Engineering);
        g.boards[MICK].goods.insert(Good::Stone, 1);
        assert!(g.trade_stone(MICK, 2).is_err());
    }

    #[test]
    fn buy_development_rejects_already_owned() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Leadership);
        assert!(
            g.buy_development(MICK, DevelopmentId::Leadership, vec![])
                .is_err()
        );
    }

    #[test]
    fn buy_development_rejects_insufficient_total_with_exact_message() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        let res = g.buy_development(MICK, DevelopmentId::Leadership, vec![]);
        assert!(res.is_err());
        let msg = res.unwrap_err().to_string();
        assert!(msg.contains("you require 10"));
        assert!(msg.contains("only amount to 0"));
    }

    #[test]
    fn buy_development_mix_of_coins_and_goods() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        g.remaining_coins = 5;
        g.boards[MICK].goods.insert(Good::Wood, 4); // value 10
        assert!(
            g.buy_development(MICK, DevelopmentId::Leadership, vec![Good::Wood])
                .is_ok()
        );
        assert_eq!(Some(&0), g.boards[MICK].goods.get(&Good::Wood));
    }

    #[test]
    fn buy_development_completing_seventh_triggers_game_end() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        g.remaining_coins = 1000;
        for d in [
            DevelopmentId::Leadership,
            DevelopmentId::Irrigation,
            DevelopmentId::Agriculture,
            DevelopmentId::Quarrying,
            DevelopmentId::Medicine,
            DevelopmentId::Preservation,
        ] {
            g.boards[MICK].developments.insert(d);
        }
        assert!(!g.final_round);
        g.buy_development(MICK, DevelopmentId::Coinage, vec![])
            .unwrap();
        assert!(g.final_round);
    }

    #[test]
    fn take_rejects_wrong_action_count() {
        let mut g = new_blank(2);
        g.phase = Phase::Collect;
        g.kept_dice = vec![Die::FoodOrWorkers, Die::FoodOrWorkers];
        assert!(g.take(MICK, vec![TakeAction::Food]).is_err());
    }

    #[test]
    fn discard_rejects_exceeding_holdings() {
        let mut g = new_blank(2);
        g.phase = Phase::Discard;
        g.boards[MICK].goods.insert(Good::Wood, 8);
        assert!(g.discard(MICK, 10, Good::Wood).is_err());
    }

    #[test]
    fn discard_rejects_exceeding_over_limit_amount() {
        let mut g = new_blank(2);
        g.phase = Phase::Discard;
        g.boards[MICK].goods.insert(Good::Wood, 8); // 2 over limit
        assert!(g.discard(MICK, 8, Good::Wood).is_err());
    }

    #[test]
    fn discard_completes_turn_once_at_or_under_limit() {
        let mut g = new_blank(2);
        g.phase = Phase::Discard;
        g.boards[MICK].goods.insert(Good::Wood, 8); // 2 over limit
        g.current_player = MICK;
        g.discard(MICK, 2, Good::Wood).unwrap();
        // Under limit now, so NextTurn() should have advanced the player.
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn invade_rejects_non_positive_amount() {
        let mut g = new_blank(2);
        g.phase = Phase::Invade;
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        assert!(g.invade(MICK, 0).is_err());
    }

    #[test]
    fn invade_wall_blocks_per_opponent() {
        let mut g = new_blank(3);
        g.phase = Phase::Invade;
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[MICK].goods.insert(Good::Spearhead, 1);
        g.boards[STEVE].monuments.insert(MonumentId::GreatWall, 13);
        g.invade(MICK, 1).unwrap();
        assert_eq!(0, g.boards[STEVE].disasters);
        assert_eq!(2, g.boards[BJ].disasters);
    }

    #[test]
    fn sell_rejects_exceeding_food() {
        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        g.boards[MICK].developments.insert(DevelopmentId::Granaries);
        g.boards[MICK].food = 2;
        assert!(g.sell_food(MICK, 3).is_err());
    }

    #[test]
    fn swap_rejects_same_good() {
        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 2);
        g.remaining_ships = 2;
        assert!(g.swap(MICK, Good::Wood, Good::Wood, 1).is_err());
    }

    #[test]
    fn swap_rejects_exceeding_ship_budget() {
        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 5);
        g.remaining_ships = 1;
        assert!(g.swap(MICK, Good::Wood, Good::Stone, 2).is_err());
    }

    #[test]
    fn swap_rejects_exceeding_target_capacity() {
        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 5);
        g.boards[MICK].goods.insert(Good::Spearhead, 4); // cap is 4
        g.remaining_ships = 5;
        assert!(g.swap(MICK, Good::Wood, Good::Spearhead, 1).is_err());
    }

    #[test]
    fn swap_remaining_ships_reaching_zero_auto_advances() {
        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 2);
        g.remaining_ships = 2;
        g.swap(MICK, Good::Wood, Good::Stone, 2).unwrap();
        assert_ne!(Phase::Trade, g.phase);
    }

    // -----------------------------------------------------------------
    // Parser-level regression tests for the exact Go command strings.
    // -----------------------------------------------------------------

    #[test]
    fn parser_accepts_go_command_strings() {
        let p = test_players();

        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        match g.command_parser(MICK).unwrap().parse("build 8 city", &p) {
            Ok(out) => assert_eq!(
                Command::Build {
                    amount: 8,
                    target: BuildTarget::City
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.remaining_workers = 20;
        match g.command_parser(MICK).unwrap().parse("build 10 wall", &p) {
            Ok(out) => assert_eq!(
                Command::Build {
                    amount: 10,
                    target: BuildTarget::Monument(MonumentId::GreatWall)
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 3);
        g.boards[MICK].goods.insert(Good::Cloth, 3);
        match g.command_parser(MICK).unwrap().parse("build 2 ship", &p) {
            Ok(out) => assert_eq!(
                Command::Build {
                    amount: 2,
                    target: BuildTarget::Ship
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        match g.command_parser(MICK).unwrap().parse("buy leader", &p) {
            Ok(out) => assert_eq!(
                Command::Buy {
                    development: DevelopmentId::Leadership,
                    goods: BuyGoods::default()
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        match g.command_parser(MICK).unwrap().parse("buy cara", &p) {
            Ok(out) => assert_eq!(
                Command::Buy {
                    development: DevelopmentId::Caravans,
                    goods: BuyGoods::default()
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        match g
            .command_parser(MICK)
            .unwrap()
            .parse("buy agri wood stone pot cloth spear", &p)
        {
            Ok(out) => assert_eq!(
                Command::Buy {
                    development: DevelopmentId::Agriculture,
                    goods: BuyGoods {
                        all_goods: false,
                        goods: vec![
                            Good::Wood,
                            Good::Stone,
                            Good::Pottery,
                            Good::Cloth,
                            Good::Spearhead
                        ]
                    }
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        match g.command_parser(MICK).unwrap().parse("buy agri all", &p) {
            Ok(out) => assert_eq!(
                Command::Buy {
                    development: DevelopmentId::Agriculture,
                    goods: BuyGoods {
                        all_goods: true,
                        goods: vec![]
                    }
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Collect;
        g.kept_dice = vec![Die::FoodOrWorkers, Die::FoodOrWorkers, Die::FoodOrWorkers];
        match g.command_parser(MICK).unwrap().parse("take w w f", &p) {
            Ok(out) => assert_eq!(
                Command::Take {
                    actions: vec![TakeAction::Workers, TakeAction::Workers, TakeAction::Food]
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Discard;
        g.boards[MICK].goods.insert(Good::Wood, 8);
        match g.command_parser(MICK).unwrap().parse("discard 1 wood", &p) {
            Ok(out) => assert_eq!(
                Command::Discard {
                    amount: 1,
                    good: Good::Wood
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Invade;
        g.boards[MICK].developments.insert(DevelopmentId::Smithing);
        g.boards[MICK].goods.insert(Good::Spearhead, 2);
        match g.command_parser(MICK).unwrap().parse("invade 2", &p) {
            Ok(out) => assert_eq!(Command::Invade { amount: 2 }, out.value),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Trade;
        g.boards[MICK].developments.insert(DevelopmentId::Shipping);
        g.boards[MICK].goods.insert(Good::Wood, 2);
        g.remaining_ships = 5;
        match g
            .command_parser(MICK)
            .unwrap()
            .parse("swap 2 wood spear", &p)
        {
            Ok(out) => assert_eq!(
                Command::Swap {
                    amount: 2,
                    from: Good::Wood,
                    to: Good::Spearhead
                },
                out.value
            ),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Build;
        g.boards[MICK]
            .developments
            .insert(DevelopmentId::Engineering);
        g.boards[MICK].goods.insert(Good::Stone, 3);
        match g.command_parser(MICK).unwrap().parse("trade 3", &p) {
            Ok(out) => assert_eq!(Command::Trade { amount: 3 }, out.value),
            Err(e) => panic!("expected ok, got {}", e),
        }

        let mut g = new_blank(2);
        g.phase = Phase::Buy;
        g.boards[MICK].developments.insert(DevelopmentId::Granaries);
        g.boards[MICK].food = 5;
        match g.command_parser(MICK).unwrap().parse("sell 5", &p) {
            Ok(out) => assert_eq!(Command::Sell { amount: 5 }, out.value),
            Err(e) => panic!("expected ok, got {}", e),
        }
    }
}
