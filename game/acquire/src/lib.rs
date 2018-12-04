pub mod board;
mod command;
pub mod corp;
mod render;
mod stats;

use rand::{thread_rng, Rng};
use serde_derive::{Serialize, Deserialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use std::collections::HashMap;

use crate::board::{Board, Loc, Tile};
use crate::command::Command;
use crate::corp::Corp;
use crate::stats::Stats;

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 6;
pub const STARTING_MONEY: usize = 6000;
pub const STARTING_SHARES: usize = 25;
pub const TILE_HAND_SIZE: usize = 6;
pub const BONUS_ROUNDING: usize = 100;
pub const DUMMY_PLAYER_OFFSET: usize = 999;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Phase {
    Play(usize),
    Found {
        player: usize,
        at: Loc,
    },
    Buy {
        player: usize,
        remaining: usize,
    },
    ChooseMerger {
        player: usize,
        at: Loc,
    },
    SellOrTrade {
        player: usize,
        corp: Corp,
        into: Corp,
        at: Loc,
        turn_player: usize,
    },
}

impl Phase {
    pub fn whose_turn(&self) -> usize {
        match *self {
            Phase::Play(player)
            | Phase::Found { player, .. }
            | Phase::Buy { player, .. }
            | Phase::ChooseMerger { player, .. }
            | Phase::SellOrTrade { player, .. } => player,
        }
    }

    pub fn main_turn_player(&self) -> usize {
        match *self {
            Phase::Play(player)
            | Phase::Found { player, .. }
            | Phase::Buy { player, .. }
            | Phase::ChooseMerger { player, .. } => player,
            Phase::SellOrTrade { turn_player, .. } => turn_player,
        }
    }
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Play(0)
    }
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PubState {
    pub phase: Phase,
    pub players: Vec<PubPlayer>,
    pub board: Board,
    pub shares: HashMap<Corp, usize>,
    pub remaining_tiles: usize,
    pub last_turn: bool,
    pub finished: bool,
}

impl PubState {
    pub fn can_end(&self) -> CanEnd {
        if self.finished {
            return CanEnd::Finished;
        }
        if self.last_turn {
            return CanEnd::Triggered;
        }
        let mut largest: usize = 0;
        let mut has_safe: bool = false;
        let mut unsafe_count: usize = 0;
        for corp in Corp::iter() {
            let size = self.board.corp_size(corp);
            if size > largest {
                largest = size;
            }
            if size >= corp::SAFE_SIZE {
                has_safe = true;
            }
            if size > 0 && size < corp::SAFE_SIZE {
                unsafe_count += 1;
            }
        }
        if largest >= corp::GAME_END_SIZE || has_safe && unsafe_count == 0 {
            return CanEnd::True;
        }
        CanEndFalse {
            largest,
            has_safe,
            unsafe_count,
        }.into()
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub tiles: Vec<Loc>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub phase: Phase,
    pub players: Vec<Player>,
    pub board: Board,
    pub draw_tiles: Vec<Loc>,
    pub shares: HashMap<Corp, usize>,
    pub last_turn: bool,
    pub finished: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            phase: Phase::Play(0),
            players: vec![],
            board: Board::default(),
            draw_tiles: vec![],
            shares: corp_hash_map(STARTING_SHARES),
            last_turn: false,
            finished: false,
        }
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn new(players: usize) -> Result<(Self, Vec<Log>), GameError> {
        let mut g = Game::default();
        if players < MIN_PLAYERS || players > MAX_PLAYERS {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }

        // Shuffle up the draw tiles.
        let mut tiles = Loc::all();
        thread_rng().shuffle(tiles.as_mut_slice());
        g.draw_tiles = tiles;

        // Place initial tiles onto the board.
        for l in g.draw_tiles.drain(0..players) {
            g.board.set_tile(&l, Tile::Unincorporated);
        }

        // Setup for each player.
        for _ in 0..players {
            let mut player = Player::default();
            player.tiles = g.draw_tiles.drain(0..TILE_HAND_SIZE).collect();
            g.players.push(player);
        }

        // Set the start player.
        let start_player = (thread_rng().next_u32() as usize) % players;
        g.phase = Phase::Play(start_player);

        let mut logs: Vec<Log> = vec![];
        if players == 2 {
            // 2 players gets a dummy shareholder, output details.
            logs.push(Log::public(vec![N::Bold(vec![
                N::text(
                    "\
2 player special rule: a dummy player is added for shareholder bonuses. A dice (D6) is rolled to \
determine the dummy player's shares. The money for the dummy player is not tracked and it is not \
able to win the game."
                ),
            ])]))
        }
        logs.push(Log::public(vec![
            N::Player(start_player),
            N::text(" will start the game"),
        ]));

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
                whose_turn: vec![self.phase.whose_turn()],
                eliminated: vec![],
            }
        }
    }

    fn placings(&self) -> Vec<usize> {
        gen_placings(
            self.players
                .iter()
                .map(|p| vec![p.money as i32])
                .collect::<Vec<Vec<i32>>>()
                .as_ref(),
        )
    }

    fn pub_state(&self) -> Self::PubState {
        self.to_owned().into()
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            tiles: self.players[player].tiles.to_owned(),
        }
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let parser = self.command_parser(player).ok_or_else::<GameError, _>(|| {
            GameError::InvalidInput {
                message: "not your turn".to_string(),
            }
        })?;
        let output = parser.parse(input, players)?;
        match output.value {
            Command::Play(loc) => self.handle_play_command(player, &loc),
            Command::Found(corp) => self.handle_found_command(player, &corp),
            Command::Buy(n, corp) => self.handle_buy_command(player, n, corp),
            Command::Done => self.handle_done_command(player).map(|l| (l, false)),
            Command::Merge(corp, into) => self.handle_merge_command(player, &corp, &into),
            Command::Sell(n) => self.handle_sell_command(player, n),
            Command::Trade(n) => self.handle_trade_command(player, n),
            Command::Keep => self.handle_keep_command(player),
            Command::End => self.handle_end_command(player).map(|l| (l, false)),
        }.map(|(logs, can_undo)| CommandResponse {
            logs,
            can_undo,
            remaining_input: output.remaining.to_string(),
        })
    }

    fn player_count(&self) -> usize {
        self.players.len()
    }

    fn player_counts() -> Vec<usize> {
        (2..6).collect()
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|p| p.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.players.len())
            .map(|p| self.player_score(p) as f32)
            .collect()
    }
}

#[derive(Debug, PartialEq)]
pub struct CanEndFalse {
    largest: usize,
    has_safe: bool,
    unsafe_count: usize,
}

impl Into<CanEnd> for CanEndFalse {
    fn into(self) -> CanEnd {
        CanEnd::False(self)
    }
}

#[derive(Debug, PartialEq)]
pub enum CanEnd {
    Triggered,
    Finished,
    True,
    False(CanEndFalse),
}

struct BonusPlayers {
    major: Vec<usize>,
    minor: Vec<usize>,
    dummy_shares: usize,
}

impl Game {
    pub fn can_play(&self, player: usize) -> bool {
        match self.phase {
            Phase::Play(p) if p == player => true,
            _ => false,
        }
    }

    fn draw_replacement_tiles(&mut self, player: usize) -> Result<(Vec<Log>, bool), GameError> {
        // Discard permanently unplayable tiles.
        let (mut keep, discard): (Vec<Loc>, Vec<Loc>) = self.players[player]
            .tiles
            .iter()
            .partition(|loc| !self.board.loc_neighbours_multiple_safe_corps(loc));
        let mut logs: Vec<Log> = vec![];
        if !discard.is_empty() {
            self.board.set_discarded(&discard);
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" discarded "),
                N::Group(
                    discard
                        .iter()
                        .enumerate()
                        .flat_map(|(i, d)| {
                            let mut ns: Vec<N> = vec![];
                            if i > 0 {
                                ns.push(N::text(", "));
                            }
                            ns.push(d.render());
                            ns
                        })
                        .collect(),
                ),
            ]))
        }
        let remaining = TILE_HAND_SIZE - keep.len();
        if self.draw_tiles.len() < remaining {
            // End of game
            logs.extend(self.end()?);
            return Ok((logs, true));
        }
        let new_tiles: Vec<Loc> = self.draw_tiles.drain(0..remaining).collect();
        logs.push(Log::private(
            vec![
                N::text("You drew "),
                N::Group(
                    new_tiles
                        .iter()
                        .enumerate()
                        .flat_map(|(i, d)| {
                            let mut ns: Vec<N> = vec![];
                            if i > 0 {
                                ns.push(N::text(", "));
                            }
                            ns.push(d.render());
                            ns
                        })
                        .collect(),
                ),
            ],
            vec![player],
        ));
        keep.extend(new_tiles);
        self.players[player].tiles = keep;
        Ok((logs, false))
    }

    pub fn handle_play_command(
        &mut self,
        player: usize,
        loc: &Loc,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;

        let mut can_undo = true;

        if !self.can_play(player) {
            return Err(GameError::InvalidInput {
                message: "You can't play a tile right now".to_string(),
            });
        }
        let pos = match self.players[player].tiles.iter().position(|l| l == loc) {
            Some(p) => p,
            None => {
                return Err(GameError::InvalidInput {
                    message: "You don't have that tile".to_string(),
                });
            }
        };
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            N::Bold(vec![N::text(format!("{}", loc))]),
        ])];
        let neighbouring_corps = self.board.neighbouring_corps(loc);
        match neighbouring_corps.len() {
            1 => {
                let n_corp = neighbouring_corps.iter().next().unwrap();
                self.board.extend_corp(loc, n_corp);
                logs.push(Log::public(vec![
                    n_corp.render(),
                    N::text(" increased in size to "),
                    N::Bold(vec![N::text(format!("{}", self.board.corp_size(n_corp)))]),
                ]));
                self.buy_phase(player);
            }
            0 => {
                let has_unincorporated_neighbour = loc.neighbours()
                    .iter()
                    .any(|n_loc| self.board.get_tile(n_loc) == Tile::Unincorporated);
                if has_unincorporated_neighbour {
                    if self.board.available_corps().is_empty() {
                        return Err(GameError::InvalidInput {
                            message: "there aren't any corporations available to found".to_string(),
                        });
                    }
                    self.found_phase(player, loc.to_owned());
                } else {
                    self.buy_phase(player);
                }
                // Set the tile last as errors can be thrown above.
                self.board.set_tile(loc, Tile::Unincorporated);
            }
            _ => {
                let safe_corp_count = neighbouring_corps.iter().fold(0, |acc, corp| {
                    if self.board.corp_is_safe(corp) {
                        acc + 1
                    } else {
                        acc
                    }
                });
                if safe_corp_count > 1 {
                    return Err(GameError::InvalidInput {
                        message: "can't merge safe corporations together".to_string(),
                    });
                }
                self.board.set_tile(loc, Tile::Unincorporated);
                let (new_logs, new_can_undo) = self.choose_merger_phase(player, *loc)?;
                logs.extend(new_logs);
                can_undo = new_can_undo
            }
        }
        self.players[player].tiles.swap_remove(pos);
        Ok((logs, can_undo))
    }

    fn buy_phase(&mut self, player: usize) {
        self.phase = Phase::Buy {
            player: player,
            remaining: 3,
        };
    }

    fn found_phase(&mut self, player: usize, loc: Loc) {
        self.phase = Phase::Found {
            player: player,
            at: loc,
        }
    }

    fn choose_merger_phase(
        &mut self,
        player: usize,
        loc: Loc,
    ) -> Result<(Vec<Log>, bool), GameError> {
        let (from, into) = self.board.merge_candidates(&loc);
        if from.is_empty() {
            // No mergers, go to buy phase.
            self.buy_phase(player);
            return Ok((vec![], true));
        }
        // We set the phase as the merge function validates this.
        self.phase = Phase::ChooseMerger {
            player: player,
            at: loc,
        };
        if from.len() == 1 && into.len() == 1 && from[0] != into[0] {
            // There's no ambiguity, automatically make the merge.
            self.handle_merge_command(player, &from[0], &into[0])
        } else {
            // Stay in this phase so the player can choose.
            Ok((vec![], true))
        }
    }

    pub fn handle_found_command(
        &mut self,
        player: usize,
        corp: &Corp,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        let at = match self.phase {
            Phase::Found { at, .. } => at,
            _ => {
                return Err(GameError::InvalidInput {
                    message: "not able to found a corporation at the moment".to_string(),
                });
            }
        };
        if !self.board.available_corps().contains(corp) {
            return Err(GameError::InvalidInput {
                message: format!("{} is already on the board", corp),
            });
        }
        self.players[player].stats.founds.push(*corp);
        self.board.extend_corp(&at, corp);
        {
            let corp_shares = self.shares.entry(*corp).or_insert(STARTING_SHARES);
            if *corp_shares > 0 {
                let player_shares = self.players[player].shares.entry(*corp).or_insert(0);
                *player_shares += 1;
                *corp_shares -= 1;
            }
        }
        self.buy_phase(player);
        Ok((
            vec![Log::public(vec![
                N::Player(player),
                N::text(" founded "),
                corp.render(),
            ])],
            match self.phase {
                Phase::Buy { .. } => true,
                _ => false,
            },
        ))
    }

    pub fn handle_buy_command(
        &mut self,
        player: usize,
        n: usize,
        corp: Corp,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        if n == 0 {
            return Err(GameError::InvalidInput {
                message: "can't buy 0 shares".to_string(),
            });
        }
        match self.phase {
            Phase::Buy { remaining, .. } => {
                if n > remaining {
                    return Err(GameError::InvalidInput {
                        message: format!("can only buy {} more", remaining),
                    });
                }
                let corp_size = self.board.corp_size(&corp);
                if corp_size == 0 {
                    return Err(GameError::InvalidInput {
                        message: format!("{} is not on the board", corp),
                    });
                }
                let corp_shares = self.shares.get(&corp).cloned().unwrap_or(0);
                if n > corp_shares {
                    return Err(GameError::InvalidInput {
                        message: format!("{} has {} left", corp, corp_shares),
                    });
                }
                let price = corp.value(corp_size) * n;
                let player_money = self.players[player].money;
                if price > player_money {
                    return Err(GameError::InvalidInput {
                        message: format!("costs ${}, you only have ${}", price, player_money),
                    });
                }
                self.players[player].money -= price;
                self.take_shares(player, n, &corp)?;
                self.players[player].stats.buy_sum += price;
                self.players[player].stats.buys += n;

                self.phase = Phase::Buy {
                    player,
                    remaining: remaining - n,
                };
                Ok((
                    vec![Log::public(vec![
                        N::Player(player),
                        N::text(" bought "),
                        N::Bold(vec![N::text(format!("{} ", n))]),
                        corp.render(),
                        N::text(" for "),
                        N::Bold(vec![N::text(format!("${}", price))]),
                    ])],
                    true,
                ))
            }
            _ => Err(GameError::InvalidInput {
                message: "can't buy shares at the moment".to_string(),
            }),
        }
    }

    pub fn handle_done_command(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        match self.phase {
            Phase::Buy { .. } => self.end_turn(),
            _ => Err(GameError::InvalidInput {
                message: "can't end your turn at the moment".to_string(),
            }),
        }
    }

    fn end(&mut self) -> Result<Vec<Log>, GameError> {
        let mut logs: Vec<Log> = vec![];
        self.finished = true;
        // Pay all bonuses on the board.
        for corp in Corp::iter() {
            let size = self.board.corp_size(corp);
            if size > 0 {
                logs.push(Log::public(vec![N::Bold(vec![
                    N::text("Paying shareholder bonuses for "),
                    corp.render(),
                ])]));
                logs.extend(self.pay_bonuses(corp));
                for player in 0..self.players.len() {
                    let p_shares = *self.players[player]
                        .shares
                        .get(corp)
                        .expect("could not get player shares");
                    if p_shares > 0 {
                        logs.extend(self.sell(player, p_shares, corp)?);
                    }
                }
            }
        }
        Ok(logs)
    }

    fn start_turn(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        // If all tiles are unplayable, we get new tiles.
        if self.players[player]
            .tiles
            .iter()
            .find(|loc| self.board.assert_loc_playable(loc).is_ok())
            .is_none()
            {
                let (mut logs, has_ended) = self.redraw_hand(player)?;
                if !has_ended {
                    logs.extend(self.start_turn(player)?);
                }
                return Ok(logs);
            }
        self.phase = Phase::Play(player);
        Ok(vec![])
    }

    fn redraw_hand(&mut self, player: usize) -> Result<(Vec<Log>, bool), GameError> {
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::Player(player),
            N::text(" has no playable tiles and will draw a new hand, discarded "),
            N::Group(
                self.players[player]
                    .tiles
                    .iter()
                    .enumerate()
                    .flat_map(|(i, loc)| {
                        let mut ns: Vec<N> = vec![];
                        if i > 0 {
                            ns.push(N::text(", "));
                        }
                        ns.push(loc.render());
                        ns
                    })
                    .collect(),
            ),
        ])];
        self.board.set_discarded(&self.players[player].tiles);
        self.players[player].tiles = vec![];
        let (rep_logs, has_finished) = self.draw_replacement_tiles(player)?;
        logs.extend(rep_logs);
        Ok((logs, has_finished))
    }

    fn end_turn(&mut self) -> Result<Vec<Log>, GameError> {
        if self.last_turn {
            // End the game
            return self.end();
        }
        let current_player = self.phase.whose_turn();
        let (mut logs, has_ended) = self.draw_replacement_tiles(current_player)?;
        if !has_ended {
            let next_player = self.next_player(current_player);
            logs.extend(self.start_turn(next_player)?);
        }
        Ok(logs)
    }

    fn next_player(&self, player: usize) -> usize {
        (player + 1) % self.players.len()
    }

    pub fn handle_merge_command(
        &mut self,
        player: usize,
        from: &Corp,
        into: &Corp,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        let mut can_undo = true;
        let at = match self.phase {
            Phase::ChooseMerger { at, .. } => at,
            _ => {
                return Err(GameError::InvalidInput {
                    message: "can't choose a merger at the moment".to_string(),
                });
            }
        };
        if from == into {
            return Err(GameError::InvalidInput {
                message: "can't merge the same corp into itself".to_string(),
            });
        }
        let (from_candidates, into_candidates) = self.board.merge_candidates(&at);
        if from_candidates.is_empty() || into_candidates.is_empty() {
            return Err(GameError::Internal {
                message: "merge was called with an empty from or into candidates".to_string(),
            });
        }
        if !from_candidates.contains(from) {
            return Err(GameError::InvalidInput {
                message: format!("{} is not a valid corporation to be merged", from),
            });
        }
        if !into_candidates.contains(into) {
            return Err(GameError::InvalidInput {
                message: format!("{} is not a valid corporation to merge into", into),
            });
        }
        if self.board.get_tile(at) == Tile::Unincorporated {
            // We just give the tile to the big corp now to make it visually obvious.
            self.board.set_tile(at, Tile::Corp(*into));
            // Make sure we also consume any unincorporated tiles if required.
            self.board.extend_corp(&at, into);
        }
        let mut logs = vec![Log::public(vec![
            from.render(),
            N::text(" is merging into "),
            into.render(),
        ])];
        self.players[player].stats.merges += 1;
        logs.extend(self.pay_bonuses(from));
        self.phase = Phase::SellOrTrade {
            player,
            corp: *from,
            into: *into,
            at,
            turn_player: player,
        };
        if self.players[player].shares.get(from).cloned().unwrap_or(0) == 0 {
            // The player has none of the shares anyway, just skip them.
            let (new_logs, new_can_undo) = self.next_player_sell_trade()?;
            logs.extend(new_logs);
            can_undo = new_can_undo;
        }
        // Can't undo if it's two player as a dice is rolled.
        Ok((logs, can_undo && self.players.len() > 2))
    }

    fn pay_bonuses(&mut self, corp: &Corp) -> Vec<Log> {
        let BonusPlayers {
            major,
            minor,
            dummy_shares,
        } = self.bonus_players(corp);

        let mut logs: Vec<Log> = vec![];
        if dummy_shares > 0 {
            logs.push(Log::public(vec![
                N::text("The dummy player rolled "),
                N::Bold(vec![N::text(format!("{}", dummy_shares))]),
            ]));
        }

        let major_len = major.len();
        let minor_len = minor.len();
        if major_len == 0 {
            panic!("expected some major bonus players");
        }
        let corp_size = self.board.corp_size(corp);
        let mut major_bonus = corp.major_bonus(corp_size);
        let minor_bonus = corp.minor_bonus(corp_size);
        if minor_len == 0 {
            // There are multiple majors so they also get the minor bonus
            major_bonus += minor_bonus;
        }
        // Round up to the nearest 100
        let major_per = (major_bonus / BONUS_ROUNDING + major_len - 1) / major_len * BONUS_ROUNDING;
        logs.push(Game::bonus_log(&major, "Major", major_per));
        for p in &major {
            if *p == DUMMY_PLAYER_OFFSET {
                continue;
            }
            self.players[*p].money += major_per;
            self.players[*p].stats.major_bonus_sum += major_per;
            self.players[*p].stats.major_bonuses += 1;
        }
        if minor_len > 0 {
            // Round up to the nearest 100
            let minor_per =
                (minor_bonus / BONUS_ROUNDING + minor_len - 1) / minor_len * BONUS_ROUNDING;
            logs.push(Game::bonus_log(&minor, "Minor", minor_per));
            for p in &minor {
                if *p == DUMMY_PLAYER_OFFSET {
                    continue;
                }
                self.players[*p].money += minor_per;
                self.players[*p].stats.minor_bonus_sum += minor_per;
                self.players[*p].stats.minor_bonuses += 1;
            }
        }
        logs
    }

    fn bonus_log(players: &[usize], kind: &str, bonus: usize) -> Log {
        let mut content: Vec<N> = vec![
            N::text(format!("{} bonus of ", kind)),
            N::Bold(vec![N::text(format!("${}", bonus))]),
            N::text(" to "),
        ];
        content.extend(players.iter().enumerate().flat_map(|(i, p)| {
            let mut player_content: Vec<N> = vec![];
            if i > 0 {
                player_content.push(N::text(", "));
            }
            player_content.push(match *p {
                DUMMY_PLAYER_OFFSET => N::Bold(vec![N::text("dummy player")]),
                _ => N::Player(*p),
            });
            player_content
        }));
        Log::public(content)
    }

    fn bonus_players(&self, corp: &Corp) -> BonusPlayers {
        let mut major: Vec<usize> = vec![];
        let mut major_count: usize = 0;
        let mut dummy_shares: usize = 0;
        if self.players.len() == 2 {
            dummy_shares = (thread_rng().gen::<usize>() % 5) + 1;
            major.push(DUMMY_PLAYER_OFFSET);
            major_count = dummy_shares;
        }
        let mut minor: Vec<usize> = vec![];
        let mut minor_count: usize = 0;
        for (player, state) in self.players.iter().enumerate() {
            let shares = state.shares.get(corp).cloned().unwrap_or(0);
            if shares == 0 {
                continue;
            }
            if shares > major_count {
                minor = major;
                minor_count = major_count;
                major = vec![];
                major_count = shares;
            }
            if shares == major_count {
                major.push(player);
            } else {
                if shares > minor_count {
                    minor = vec![];
                    minor_count = shares;
                }
                if shares == minor_count {
                    minor.push(player);
                }
            }
        }
        if major.len() > 1 {
            // If there are multiple majors, they share the minor bonus too
            minor = vec![];
        }
        BonusPlayers {
            major,
            minor,
            dummy_shares,
        }
    }

    fn next_player_sell_trade(&mut self) -> Result<(Vec<Log>, bool), GameError> {
        let (mut player, corp, into, at, turn_player) = match self.phase {
            Phase::SellOrTrade {
                player,
                corp,
                into,
                at,
                turn_player,
            } => (player, corp, into, at, turn_player),
            _ => panic!("must be Phase::SellOrTrade"),
        };
        player = self.next_player(player);
        if player == turn_player {
            // Everyone has had a turn.
            return self.end_sell_trade_phase();
        }
        self.phase = Phase::SellOrTrade {
            player,
            corp,
            into,
            at,
            turn_player,
        };
        if self.players[player].shares.get(&corp).cloned().unwrap_or(0) == 0 {
            return self.next_player_sell_trade();
        }
        Ok((vec![], true))
    }

    fn end_sell_trade_phase(&mut self) -> Result<(Vec<Log>, bool), GameError> {
        let (corp, into, at, turn_player) = match self.phase {
            Phase::SellOrTrade {
                corp,
                into,
                at,
                turn_player,
                ..
            } => (corp, into, at, turn_player),
            _ => panic!("must be Phase::SellOrTrade"),
        };
        self.board.convert_corp(&corp, &into);
        self.choose_merger_phase(turn_player, at)
    }

    pub fn handle_sell_command(
        &mut self,
        player: usize,
        n: usize,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        let mut can_undo = true;
        let corp = match self.phase {
            Phase::SellOrTrade { corp, .. } => corp,
            _ => {
                return Err(GameError::InvalidInput {
                    message: "can't sell or trade at the moment".to_string(),
                });
            }
        };
        let mut logs = self.sell(player, n, &corp)?;
        if *self.players[player]
            .shares
            .get(&corp)
            .expect("could not get player shares") == 0
            {
                let (new_logs, new_can_undo) = self.next_player_sell_trade()?;
                logs.extend(new_logs);
                can_undo = new_can_undo;
            }
        Ok((logs, can_undo))
    }

    fn sell(&mut self, player: usize, n: usize, corp: &Corp) -> Result<Vec<Log>, GameError> {
        if n == 0 {
            return Err(GameError::InvalidInput {
                message: "you must sell an amount greater than 0".to_string(),
            });
        }
        let money = corp.value(self.board.corp_size(corp)) * n;
        let player_shares = *self.players[player]
            .shares
            .get(corp)
            .expect("could not get player shares");
        if n > player_shares {
            return Err(GameError::InvalidInput {
                message: "you don't have that many shares".to_string(),
            });
        }
        self.return_shares(player, n, corp)?;
        self.players[player].money += money;
        self.players[player].stats.sell_sum += money;
        self.players[player].stats.sells += n;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" sold "),
            N::Bold(vec![N::text(format!("{} ", n))]),
            corp.render(),
            N::text(" for "),
            N::Bold(vec![N::text(format!("${}", money))]),
        ])])
    }

    pub fn handle_trade_command(
        &mut self,
        player: usize,
        n: usize,
    ) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        // Validate
        let (corp, into) = match self.phase {
            Phase::SellOrTrade { corp, into, .. } => (corp, into),
            _ => {
                return Err(GameError::InvalidInput {
                    message: "not currently in a sell or trade phase".to_string(),
                });
            }
        };
        if n == 0 {
            return Err(GameError::InvalidInput {
                message: "you must specify an amount to trade greater than 0".to_string(),
            });
        }
        if n % 2 != 0 {
            return Err(GameError::InvalidInput {
                message: "you can only trade multiples of 2, trades are 2-for-1".to_string(),
            });
        }
        let corp_shares = self.players[player]
            .shares
            .get(&corp)
            .cloned()
            .expect("could not get player shares");
        if corp_shares < n {
            return Err(GameError::InvalidInput {
                message: format!("you only have {} {}", corp_shares, corp),
            });
        }
        let receive = n / 2;
        let into_shares = self.shares
            .get(&into)
            .cloned()
            .expect("could not get into shares");
        if receive > into_shares {
            return Err(GameError::InvalidInput {
                message: format!("{} only has {} remaining", into, into_shares),
            });
        }

        let mut can_undo = true;
        self.players[player].stats.trades += receive;
        self.players[player].stats.trade_loss_sum += n * corp.value(self.board.corp_size(&corp));
        self.players[player].stats.trade_gain_sum +=
            receive * into.value(self.board.corp_size(&into));
        self.return_shares(player, n, &corp)?;
        self.take_shares(player, receive, &into)?;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" traded "),
            N::Bold(vec![N::text(format!("{} ", n))]),
            corp.render(),
            N::text(" for "),
            N::Bold(vec![N::text(format!("{} ", receive))]),
            into.render(),
        ])];
        if n == corp_shares {
            let (new_logs, new_can_undo) = self.next_player_sell_trade()?;
            logs.extend(new_logs);
            can_undo = new_can_undo;
        }
        Ok((logs, can_undo))
    }

    fn take_shares(&mut self, player: usize, n: usize, corp: &Corp) -> Result<(), GameError> {
        let corp_shares = *self.shares
            .get(corp)
            .expect("could not get corp share count");
        if corp_shares < n {
            return Err(GameError::InvalidInput {
                message: format!("{} only has {} left", corp, corp_shares),
            });
        }
        let player_shares = self.players[player].shares.entry(*corp).or_insert(0);
        *player_shares += n;
        let corp_shares = self.shares.entry(*corp).or_insert(STARTING_SHARES);
        *corp_shares -= n;
        Ok(())
    }

    fn return_shares(&mut self, player: usize, n: usize, corp: &Corp) -> Result<(), GameError> {
        let player_shares = *self.players[player]
            .shares
            .get(corp)
            .expect("could not get player share count");
        if player_shares < n {
            return Err(GameError::InvalidInput {
                message: format!("only has {} left", player_shares),
            });
        }
        let player_shares = self.players[player].shares.entry(*corp).or_insert(0);
        *player_shares -= n;
        let corp_shares = self.shares.entry(*corp).or_insert(STARTING_SHARES);
        *corp_shares += n;
        Ok(())
    }

    pub fn handle_keep_command(&mut self, player: usize) -> Result<(Vec<Log>, bool), GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        let corp = match self.phase {
            Phase::SellOrTrade { corp, .. } => corp,
            _ => {
                return Err(GameError::InvalidInput {
                    message: "not currently in a sell or trade phase".to_string(),
                });
            }
        };
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::Player(player),
            N::text(" kept "),
            N::Bold(vec![N::text(format!(
                "{} ",
                *self.players[player].shares.entry(corp).or_insert(0)
            ))]),
            corp.render(),
        ])];
        let (new_logs, can_undo) = self.next_player_sell_trade()?;
        logs.extend(new_logs);
        Ok((logs, can_undo))
    }

    pub fn handle_end_command(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if self.phase.main_turn_player() != player {
            return Err(GameError::InvalidInput {
                message: "can't end the game during another player's turn".to_string(),
            });
        }
        if self.pub_state().can_end() != CanEnd::True {
            return Err(GameError::InvalidInput {
                message: "can't end the game at the moment".to_string(),
            });
        }
        self.last_turn = true;
        Ok(vec![Log::public(vec![N::Bold(vec![
            N::Player(player),
            N::text(" triggered the end of the game at the end of their turn"),
        ])])])
    }

    fn player_score(&self, player: usize) -> usize {
        self.players[player].money
    }

    fn player_can_end(&self, player: usize) -> bool {
        self.phase.main_turn_player() == player && self.pub_state().can_end() == CanEnd::True
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub money: usize,
    pub shares: HashMap<Corp, usize>,
    pub tiles: Vec<Loc>,
    pub stats: Stats,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            money: STARTING_MONEY,
            shares: corp_hash_map(0),
            tiles: vec![],
            stats: Stats::default(),
        }
    }
}

fn corp_hash_map(initial: usize) -> HashMap<Corp, usize> {
    let mut hm: HashMap<Corp, usize> = HashMap::new();
    for corp in Corp::iter() {
        hm.insert(*corp, initial);
    }
    hm
}

impl Into<PubState> for Game {
    fn into(self) -> PubState {
        PubState {
            phase: self.phase,
            players: self.players.iter().map(|v| v.to_owned().into()).collect(),
            board: self.board,
            shares: self.shares,
            remaining_tiles: self.draw_tiles.len(),
            last_turn: self.last_turn,
            finished: self.finished,
        }
    }
}

impl Into<PubPlayer> for Player {
    fn into(self) -> PubPlayer {
        PubPlayer {
            money: self.money,
            shares: self.shares,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PubPlayer {
    pub money: usize,
    pub shares: HashMap<Corp, usize>,
}

#[cfg(test)]
impl<'a> From<&'a str> for Game {
    fn from(s: &'a str) -> Self {
        let mut players: Vec<Player> = vec![Player::default(), Player::default()];
        for (row, line) in s.trim().lines().enumerate() {
            for (col, ch) in line.trim().chars().enumerate() {
                if ch >= '0' && ch <= '9' {
                    let p = ((ch as u8) - b'0') as usize;
                    while players.len() <= p {
                        players.push(Player::default());
                    }
                    players[p].tiles.push(Loc { row, col });
                }
            }
        }
        let mut g = Game::new(players.len()).expect("expected new game").0;
        g.phase = Phase::Play(0);
        g.players = players;
        g.board = s.into();
        g.draw_tiles = vec![];
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_from_str_is_deterministic() {
        let g1: Game = "...012".into();
        let g2: Game = "...012".into();
        assert_eq!(g1, g2);
    }

    #[test]
    fn play_works() {
        let players = vec!["mick".to_string(), "steve".to_string()];
        let mut g: Game = "...
                           .0.
                           ..."
            .into();
        g.command(0, "play b2", &players)
            .expect("expected playing tile to work");
        assert_eq!(
            g.board,
            "...
             .#.
             ..."
                .into()
        );
    }

    #[test]
    fn found_works() {
        let players = vec!["mick".to_string(), "steve".to_string()];
        let mut g: Game = "...
                           #0.
                           ..."
            .into();
        g.command(0, "play b2", &players)
            .expect("expected playing tile to work");
        g.command(0, "found fe", &players)
            .expect("expected founding to work");
        assert_eq!(
            g.board,
            "...
             FF.
             ..."
                .into()
        );
    }

    #[test]
    fn merge_works() {
        let players = vec!["mick".to_string(), "steve".to_string()];
        let mut g: Game = "FF0
                           ..A
                           ..A"
            .into();
        g.players[0].shares.insert(Corp::American, 9);
        g.players[1].shares.insert(Corp::American, 8);
        g.command(0, "play a3", &players)
            .expect("expected 'play a3' to work");
        g.command(0, "merge am into fe", &players)
            .expect("expected 'merge am into fe' to work");
        assert_eq!(STARTING_MONEY + 3000, g.players[0].money);
        assert_eq!(STARTING_MONEY + 1500, g.players[1].money);
        g.command(0, "trade 8", &players)
            .expect("expected 'trade 8' to work");
        g.command(0, "sell 1", &players)
            .expect("expected 'sell 1' to work");
        assert_eq!(Some(&0), g.players[0].shares.get(&Corp::American));
        assert_eq!(Some(&4), g.players[0].shares.get(&Corp::Festival));
        assert_eq!(STARTING_MONEY + 3300, g.players[0].money);
        g.command(1, "sell 8", &players)
            .expect("expected 'sell 8' to work");
        assert_eq!(Some(&0), g.players[1].shares.get(&Corp::American));
        assert_eq!(STARTING_MONEY + 3900, g.players[1].money);
        assert_eq!(
            g.board,
            "FFF
             ..F
             ..F"
                .into()
        );
    }
}
