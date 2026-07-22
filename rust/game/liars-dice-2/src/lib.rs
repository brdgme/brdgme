use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status, placings_log};
use brdgme_markup::Node as N;

use command::Command;
use rand::prelude::*;

pub const START_DICE_COUNT: usize = 5;
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 6;
pub const MIN_BID_QUANTITY: i32 = 1;
pub const MIN_BID_VALUE: i32 = 1;
pub const MAX_BID_VALUE: i32 = 6;

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub current_player: usize,
    pub player_dice: Vec<Vec<u8>>,
    pub bid_quantity: i32,
    pub bid_value: i32,
    pub bid_player: usize,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Total number of players in the game (including eliminated).
    pub players: usize,
    /// Index of the player whose turn it is to bid or call.
    pub current_player: usize,
    /// Quantity (number of dice) in the current bid; 0 if no bid yet.
    pub bid_quantity: i32,
    /// Face value (1-6) in the current bid; 0 if no bid yet.
    pub bid_value: i32,
    /// Index of the player who made the current bid.
    pub bid_player: usize,
    /// Number of dice each player still has, by seat index.
    pub remaining_dice: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state visible to all players.
    pub public: PubState,
    /// This player's seat index.
    pub player: usize,
    /// Values of this player's dice (private, each 1-6).
    pub dice: Vec<u8>,
}

impl Game {
    pub fn can_bid(&self, player: usize) -> bool {
        !self.is_finished() && self.current_player == player
    }

    pub fn can_call(&self, player: usize) -> bool {
        !self.is_finished() && self.current_player == player && self.bid_quantity != 0
    }

    pub fn active_players(&self) -> Vec<usize> {
        (0..self.players)
            .filter(|&p| !self.player_dice[p].is_empty())
            .collect()
    }

    pub fn eliminated_player_list(&self) -> Vec<usize> {
        (0..self.players)
            .filter(|&p| self.player_dice[p].is_empty())
            .collect()
    }

    pub fn next_active_player(&self, from: usize) -> usize {
        let mut next = (from + 1) % self.players;
        while self.player_dice[next].is_empty() && next != from {
            next = (next + 1) % self.players;
        }
        next
    }

    fn start_round(&mut self) {
        self.bid_quantity = 0;
        self.bid_value = 0;
        self.bid_player = 0;
        self.roll_dice();
    }

    fn roll_dice(&mut self) {
        for p in 0..self.players {
            for d in 0..self.player_dice[p].len() {
                let v = self.rng.random_range(1u8..=6);
                self.player_dice[p][d] = v;
            }
        }
    }

    pub fn bid(
        &mut self,
        player: usize,
        quantity: i32,
        value: i32,
        remaining: &str,
    ) -> Result<CommandResponse, GameError> {
        if !self.can_bid(player) {
            return Err(GameError::invalid_input("can't bid at the moment"));
        }
        if quantity < 1 {
            return Err(GameError::invalid_input(
                "quantity must be a positive number, eg. 5",
            ));
        }
        if quantity < self.bid_quantity {
            return Err(GameError::invalid_input(format!(
                "you can't reduce the quantity of the bid, it is currently at {}",
                self.bid_quantity
            )));
        }
        if !(1..=6).contains(&value) {
            return Err(GameError::invalid_input(
                "value must be a number between 1 and 6",
            ));
        }
        if quantity == self.bid_quantity && value <= self.bid_value {
            return Err(GameError::invalid_input(
                "if you don't increase the bid quantity, you must increase the bid value",
            ));
        }
        let verb = if self.bid_quantity == 0 {
            "set the starting bid to"
        } else {
            "increased the bid to"
        };
        self.bid_quantity = quantity;
        self.bid_value = value;
        self.bid_player = player;
        let logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" {} ", verb)),
            render::render_bid(quantity, value),
        ])];
        self.current_player = self.next_active_player(self.current_player);
        Ok(CommandResponse {
            logs,
            can_undo: true,
            remaining_input: remaining.to_string(),
        })
    }

    pub fn call(&mut self, player: usize, remaining: &str) -> Result<CommandResponse, GameError> {
        if !self.can_call(player) {
            return Err(GameError::invalid_input("can't call at the moment"));
        }
        let mut matching = 0;
        for pd in &self.player_dice {
            for d in pd {
                if *d as i32 == self.bid_value || *d == 1 {
                    matching += 1;
                }
            }
        }
        let (losing_player, result_text) = if matching < self.bid_quantity {
            (
                self.bid_player,
                vec![
                    N::Player(self.bid_player),
                    N::text(" bid too high and lost a die"),
                ],
            )
        } else {
            (
                player,
                vec![
                    N::Player(self.bid_player),
                    N::text(" bid correctly and "),
                    N::Player(player),
                    N::text(" lost a die"),
                ],
            )
        };
        let table = render::reveal_table(&self.player_dice, &self.active_players(), self.bid_value);
        if !self.player_dice[losing_player].is_empty() {
            self.player_dice[losing_player].remove(0);
        }
        let mut content: Vec<N> = vec![
            N::Player(player),
            N::text(" called the bid of "),
            render::render_bid(self.bid_quantity, self.bid_value),
            N::text(" by "),
            N::Player(self.bid_player),
            N::text("\nEveryone revealed the following dice:\n"),
            table,
            N::text("\n"),
        ];
        content.extend(result_text);
        let logs = vec![Log::public(content)];
        if !self.is_finished() {
            self.start_round();
            self.current_player = self.next_active_player(self.current_player);
        }
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input: remaining.to_string(),
        })
    }

    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_dice[p].len() as i32])
            .collect();
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
            current_player,
            player_dice: vec![vec![0u8; START_DICE_COUNT]; players],
            rng,
            ..Game::default()
        };
        g.start_round();
        Ok((g, vec![]))
    }

    fn status(&self) -> Status {
        let active = self.active_players();
        if active.len() < 2 {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: self.eliminated_player_list(),
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            current_player: self.current_player,
            bid_quantity: self.bid_quantity,
            bid_value: self.bid_value,
            bid_player: self.bid_player,
            remaining_dice: (0..self.players)
                .map(|p| self.player_dice[p].len())
                .collect(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        let dice = if player < self.player_dice.len() {
            self.player_dice[player].clone()
        } else {
            vec![]
        };
        PlayerState {
            public: self.pub_state(),
            player,
            dice,
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
            None => return Err(GameError::invalid_input("not your turn".to_string())),
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                value: Command::Bid { quantity, value },
                remaining,
                ..
            }) => self.bid(player, quantity, value, remaining),
            Ok(ParseOutput {
                value: Command::Call,
                remaining,
                ..
            }) => {
                let mut resp = self.call(player, remaining)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_dice[p].len() as i32))
                        .collect();
                    resp.logs
                        .push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(resp)
            }
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.players)
            .map(|p| self.player_dice[p].len() as f32)
            .collect()
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
    use brdgme_game::Gamer;

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    #[test]
    fn start_works() {
        let (g, _) = Game::start(3, 1).unwrap();
        let wt = g.whose_turn();
        assert_eq!(1, wt.len());
        assert!(wt[0] < 3);
        assert_eq!(3, g.player_dice.len());
        for i in 0..g.players {
            assert_eq!(START_DICE_COUNT, g.player_dice[i].len());
            for d in &g.player_dice[i] {
                assert!(*d >= 1 && *d <= 6);
            }
        }
    }

    #[test]
    fn example_round() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        g.player_dice = vec![vec![1, 3, 4, 4, 6], vec![2, 2, 3, 3, 3], vec![1]];
        g.current_player = 0;
        assert!(g.command(0, "call", &p).is_err());
        g.command(0, "bid 2 5", &p).unwrap();
        g.command(1, "bid 2 6", &p).unwrap();
        g.command(2, "bid 3 5", &p).unwrap();
        assert!(g.command(0, "bid 3 5", &p).is_err());
        assert!(g.command(0, "bid 3 3", &p).is_err());
        assert!(g.command(0, "bid 2 6", &p).is_err());
        assert!(g.command(0, "bid 3 7", &p).is_err());
        assert!(g.command(3, "bid 6 5", &p).is_err());
        g.command(0, "call", &p).unwrap();
        assert_eq!(0, g.player_dice[2].len());
        assert_eq!(5, g.player_dice[0].len());
        assert_eq!(5, g.player_dice[1].len());
        assert_eq!(1, g.current_player);
        assert_eq!(2, g.active_players().len());
    }

    #[test]
    fn player_elimination() {
        let (mut g, _) = Game::start(4, 1).unwrap();
        g.player_dice[0] = vec![];
        g.player_dice[2] = vec![];
        let eliminated = g.eliminated_player_list();
        assert_eq!(2, eliminated.len());
        assert_eq!(0, eliminated[0]);
        assert_eq!(2, eliminated[1]);
    }
}
