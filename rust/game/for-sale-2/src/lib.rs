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
use rand::seq::SliceRandom;

const MIN_PLAYERS: usize = 3;
const MAX_PLAYERS: usize = 5;
pub const STARTING_CHIPS: i32 = 15;
const SELL_THRESHOLD: usize = 18;

#[derive(Default, Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    #[default]
    Buying,
    Selling,
    Finished,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub building_deck: Vec<i32>,
    pub cheque_deck: Vec<i32>,
    pub open_cards: Vec<i32>,
    pub hands: Vec<Vec<i32>>,
    pub cheques: Vec<Vec<i32>>,
    pub chips: Vec<i32>,
    pub bidding_player: usize,
    pub bids: Vec<i32>,
    pub finished_bidding: Vec<bool>,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in the game
    pub players: usize,
    /// Current game phase: Buying, Selling, or Finished
    pub phase: Phase,
    /// Whether the game has ended
    pub finished: bool,
    /// Property auctions remaining in the buying phase
    pub buy_rounds_remaining: usize,
    /// Cheque rounds remaining in the selling phase
    pub sell_rounds_remaining: usize,
    /// Cards currently face up (buildings or cheques depending on phase)
    pub open_cards: Vec<i32>,
    /// Index of the player whose turn it is to bid or play
    pub bidding_player: usize,
    /// Current bid from each player in the buying phase
    pub bids: Vec<i32>,
    /// Which players have dropped out of the current auction
    pub finished_bidding: Vec<bool>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state
    pub public: PubState,
    /// This player's seat index
    pub player: usize,
    /// Remaining chips (money)
    pub chips: i32,
    /// Property cards in hand (values 1-30)
    pub hand: Vec<i32>,
    /// Cheque values collected during the selling phase
    pub cheques: Vec<i32>,
}

fn building_deck() -> Vec<i32> {
    (1..=20).collect()
}

fn cheque_deck() -> Vec<i32> {
    (1..=20).map(|i| if i < 3 { 0 } else { i }).collect()
}

impl Game {
    pub fn current_phase(&self) -> Phase {
        if !self.building_deck.is_empty()
            || (!self.open_cards.is_empty() && self.cheque_deck.len() >= SELL_THRESHOLD)
        {
            Phase::Buying
        } else if !self.cheque_deck.is_empty() || !self.open_cards.is_empty() {
            Phase::Selling
        } else {
            Phase::Finished
        }
    }

    pub fn start_round(&mut self) -> Vec<Log> {
        match self.current_phase() {
            Phase::Buying => self.start_buying_round(),
            Phase::Selling => self.start_selling_round(),
            Phase::Finished => {
                use brdgme_markup::{Align as A, Row, table_with_gap};
                let mut rows: Vec<Row> = vec![];
                for p in 0..self.players {
                    rows.push(vec![
                        (A::Left, vec![N::Player(p)]),
                        (
                            A::Left,
                            vec![render::bold_num(Self::deck_value(&self.cheques[p]))],
                        ),
                    ]);
                }
                vec![Log::public(vec![
                    N::Bold(vec![N::text("The game has finished!  The scores are:")]),
                    N::text("\n"),
                    table_with_gap(&rows, 1),
                ])]
            }
        }
    }

    pub fn start_buying_round(&mut self) -> Vec<Log> {
        let n = self.players;
        self.open_cards = self.building_deck.split_off(self.building_deck.len() - n);
        self.open_cards.sort();
        self.clear_bids();
        vec![Log::public(vec![
            N::text("Drew new buildings: "),
            render::cards(&self.open_cards, true),
        ])]
    }

    pub fn start_selling_round(&mut self) -> Vec<Log> {
        let n = self.players;
        self.open_cards = self.cheque_deck.split_off(self.cheque_deck.len() - n);
        self.open_cards.sort();
        self.clear_bids();
        let mut logs = vec![Log::public(vec![
            N::text("Drew new cheques: "),
            render::cards(&self.open_cards, false),
        ])];
        if self.hands.first().is_some_and(|h| h.len() == 1) {
            for p in 0..self.players {
                let card = self.hands[p][0];
                if let Ok(play_logs) = self.play(p, card) {
                    logs.extend(play_logs);
                }
            }
        }
        logs
    }

    pub fn clear_bids(&mut self) {
        for p in 0..self.players {
            self.bids[p] = 0;
            self.finished_bidding[p] = false;
        }
    }

    pub fn deck_value(deck: &[i32]) -> i32 {
        deck.iter().sum()
    }

    pub fn whose_turn_inner(&self) -> Vec<usize> {
        match self.current_phase() {
            Phase::Buying => vec![self.bidding_player],
            Phase::Selling => (0..self.players)
                .filter(|p| !self.finished_bidding[*p])
                .collect(),
            Phase::Finished => vec![],
        }
    }

    pub fn can_bid(&self, player: usize) -> bool {
        self.current_phase() == Phase::Buying && self.bidding_player == player
    }

    pub fn can_pass(&self, player: usize) -> bool {
        self.can_bid(player)
    }

    pub fn can_play(&self, player: usize) -> bool {
        self.current_phase() == Phase::Selling
            && player < self.players
            && !self.finished_bidding[player]
    }

    pub fn bid(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_bid(player) {
            return Err(GameError::invalid_input(
                "you are not able to bid at the moment",
            ));
        }
        if amount > self.chips[player] {
            return Err(GameError::invalid_input(format!(
                "cannot bid {}, you only have {}",
                amount, self.chips[player]
            )));
        }
        let (_, highest) = self.highest_bid();
        if amount <= highest {
            return Err(GameError::invalid_input(format!(
                "you must bid higher than {}",
                highest
            )));
        }
        self.bids[player] = amount;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" bid "),
            N::Bold(vec![N::text(amount.to_string())]),
        ])];
        logs.extend(self.next_bidder());
        Ok(logs)
    }

    pub fn pass(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_pass(player) {
            return Err(GameError::invalid_input(
                "you are not able to pass at the moment",
            ));
        }
        let c = self.take_first_open_card(player);
        let half_bid = self.bids[player] / 2;
        self.chips[player] -= half_bid;
        self.finished_bidding[player] = true;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" passed, paying "),
            N::Bold(vec![N::text(half_bid.to_string())]),
            N::text(" for "),
            render::building(c),
        ])];
        logs.extend(self.next_bidder());
        Ok(logs)
    }

    pub fn play(&mut self, player: usize, building: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_play(player) {
            return Err(GameError::invalid_input(
                "you are not able to play a building card at the moment",
            ));
        }
        let idx = self.hands[player]
            .iter()
            .position(|c| *c == building)
            .ok_or_else(|| GameError::invalid_input("you don't have that card in your hand"))?;
        self.hands[player].remove(idx);
        self.bids[player] = building;
        self.finished_bidding[player] = true;
        let mut logs: Vec<Log> = vec![];
        if self.whose_turn_inner().is_empty() {
            let mut played: Vec<(i32, usize)> =
                (0..self.players).map(|p| (self.bids[p], p)).collect();
            played.sort();
            for (bldg, p) in played {
                let cheque = self.open_cards.remove(0);
                self.cheques[p].push(cheque);
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(" sold "),
                    render::building(bldg),
                    N::text(" for "),
                    render::cheque(cheque),
                ]));
            }
            logs.extend(self.start_round());
        }
        Ok(logs)
    }

    pub fn take_first_open_card(&mut self, player: usize) -> i32 {
        let c = self.open_cards.remove(0);
        self.hands[player].push(c);
        self.hands[player].sort();
        c
    }

    pub fn next_bidder(&mut self) -> Vec<Log> {
        let remaining = (0..self.players)
            .filter(|p| !self.finished_bidding[*p])
            .count();
        if remaining == 1 {
            let (player, amount) = self.highest_bid();
            let c = self.take_first_open_card(player);
            self.chips[player] -= amount;
            self.bidding_player = player;
            let mut logs = vec![Log::public(vec![
                N::Player(player),
                N::text(" is the last player, paying "),
                N::Bold(vec![N::text(amount.to_string())]),
                N::text(" for "),
                render::building(c),
            ])];
            logs.extend(self.start_round());
            return logs;
        }
        loop {
            self.bidding_player = (self.bidding_player + 1) % self.players;
            if !self.finished_bidding[self.bidding_player] {
                break;
            }
        }
        vec![]
    }

    pub fn highest_bid(&self) -> (usize, i32) {
        let mut player = 0;
        let mut amount: i32 = -1;
        for p in 0..self.players {
            if !self.finished_bidding[p] && self.bids[p] > amount {
                player = p;
                amount = self.bids[p];
            }
        }
        (player, amount)
    }

    pub fn player_points(&self, player: usize) -> i32 {
        Self::deck_value(&self.cheques[player]) + self.chips[player]
    }

    pub fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_points(p), self.chips[p]])
            .collect();
        gen_placings(&metrics)
    }

    pub fn points_int(&self) -> Vec<i32> {
        let finished = self.is_finished();
        (0..self.players)
            .map(|p| if finished { self.player_points(p) } else { 0 })
            .collect()
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
        let mut g = Game {
            players,
            building_deck: building_deck(),
            cheque_deck: cheque_deck(),
            hands: vec![vec![]; players],
            cheques: vec![vec![]; players],
            chips: vec![STARTING_CHIPS; players],
            bids: vec![0; players],
            finished_bidding: vec![false; players],
            bidding_player: 0,
            open_cards: vec![],
            rng: GameRng::seed_from_u64(seed),
        };
        g.building_deck.shuffle(&mut g.rng);
        g.cheque_deck.shuffle(&mut g.rng);
        let mut logs = vec![];
        if players == 3 {
            logs.push(Log::public(vec![N::text(
                "Removing two building and cheque cards for 3 player game",
            )]));
            let _ = g.building_deck.split_off(g.building_deck.len() - 2);
            let _ = g.cheque_deck.split_off(g.cheque_deck.len() - 2);
        }
        logs.extend(g.start_round());
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.open_cards.is_empty()
            && self.building_deck.is_empty()
            && self.cheque_deck.is_empty()
        {
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
        PubState {
            players: self.players,
            phase: self.current_phase(),
            finished: self.is_finished(),
            buy_rounds_remaining: self.building_deck.len() / self.players,
            sell_rounds_remaining: self.cheque_deck.len() / self.players,
            open_cards: self.open_cards.clone(),
            bidding_player: self.bidding_player,
            bids: self.bids.clone(),
            finished_bidding: self.finished_bidding.clone(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            chips: self.chips[player],
            hand: self.hands[player].clone(),
            cheques: self.cheques[player].clone(),
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
                value: Command::Bid(amount),
                ..
            }) => {
                let mut logs = self.bid(player, amount)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_points(p)))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: true,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Pass,
                ..
            }) => {
                let mut logs = self.pass(player)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_points(p)))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Play(building),
                ..
            }) => {
                let mut logs = self.play(player, building)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_points(p)))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
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
        self.points_int().iter().map(|p| *p as f32).collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![3, 4, 5]
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

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const BJ: usize = 2;

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    fn pop_n(deck: &[i32], n: usize) -> (Vec<i32>, Vec<i32>) {
        let len = deck.len();
        (deck[len - n..].to_vec(), deck[..len - n].to_vec())
    }

    #[test]
    fn test_full_game() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Set the state of the game to sorted decks
        let (_, bd) = pop_n(&building_deck(), 2);
        g.building_deck = bd;
        let (_, cd) = pop_n(&cheque_deck(), 2);
        g.cheque_deck = cd;
        let (open, bd) = pop_n(&g.building_deck, 3);
        g.open_cards = open;
        g.building_deck = bd;
        // Play a round of buying
        assert_eq!(vec![MICK], g.whose_turn());
        g.command(MICK, "bid 3", &players(3)).unwrap();
        assert_eq!(vec![STEVE], g.whose_turn());
        assert!(g.command(STEVE, "bid 3", &players(3)).is_err());
        g.command(STEVE, "bid 4", &players(3)).unwrap();
        assert_eq!(vec![BJ], g.whose_turn());
        g.command(BJ, "pass", &players(3)).unwrap();
        assert_eq!(vec![17, 18], g.open_cards);
        assert_eq!(vec![16], g.hands[BJ]);
        assert_eq!(15, g.chips[BJ]);
        assert_eq!(vec![MICK], g.whose_turn());
        g.command(MICK, "pass", &players(3)).unwrap();
        assert_eq!(14, g.chips[MICK]);
        assert_eq!(11, g.chips[STEVE]);
        assert_eq!(vec![17], g.hands[MICK]);
        assert_eq!(vec![18], g.hands[STEVE]);
        assert_eq!(vec![STEVE], g.whose_turn());
        // One more buying phase so each player has 2 buildings.
        g.command(STEVE, "pass", &players(3)).unwrap();
        g.command(BJ, "pass", &players(3)).unwrap();
        assert_eq!(vec![15, 17], g.hands[MICK]);
        assert_eq!(vec![13, 18], g.hands[STEVE]);
        assert_eq!(vec![14, 16], g.hands[BJ]);
        // End the buying phase early and shorten the selling phase.
        g.building_deck = vec![];
        let (_, cd) = pop_n(&g.cheque_deck, 12);
        g.cheque_deck = cd;
        g.open_cards = vec![];
        let _ = g.start_round();
        assert_eq!(vec![MICK, STEVE, BJ], g.whose_turn());
        // Play a round of selling
        assert!(g.command(BJ, "play 18", &players(3)).is_err());
        g.command(BJ, "play 16", &players(3)).unwrap();
        assert_eq!(vec![MICK, STEVE], g.whose_turn());
        g.command(STEVE, "play 18", &players(3)).unwrap();
        assert_eq!(vec![MICK], g.whose_turn());
        g.command(MICK, "play 17", &players(3)).unwrap();
        // Because there were only two cards each, assume that the last cards were
        // automatically played.
        assert_eq!(vec![5, 3], g.cheques[MICK]);
        assert_eq!(vec![6, 0], g.cheques[STEVE]);
        assert_eq!(vec![4, 0], g.cheques[BJ]);
        // Check the game ended
        assert!(g.is_finished());
        assert_eq!(1, g.placings()[0]);
    }

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![3, 4, 5], Game::player_counts());
        assert!(Game::start(2, 1).is_err());
        assert!(Game::start(6, 1).is_err());
        assert!(Game::start(3, 1).is_ok());
        assert!(Game::start(5, 1).is_ok());
    }

    #[test]
    fn test_start_state() {
        let (g, logs) = Game::start(3, 1).unwrap();
        assert!(!logs.is_empty());
        assert_eq!(3, g.players);
        assert_eq!(STARTING_CHIPS, g.chips[0]);
        assert_eq!(STARTING_CHIPS, g.chips[1]);
        assert_eq!(STARTING_CHIPS, g.chips[2]);
        // 3p removes 2 cards from each deck (20 -> 18)
        assert_eq!(15, g.building_deck.len());
        assert_eq!(18, g.cheque_deck.len());
        // First buying round draws one card per player
        assert_eq!(3, g.open_cards.len());
        // open cards sorted ascending
        assert!(g.open_cards.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(Phase::Buying, g.current_phase());
        assert_eq!(vec![0], g.whose_turn());
    }

    #[test]
    fn test_decks() {
        let bd = building_deck();
        assert_eq!(20, bd.len());
        assert_eq!(1, bd[0]);
        assert_eq!(20, bd[19]);
        let cd = cheque_deck();
        assert_eq!(20, cd.len());
        assert_eq!(0, cd[0]);
        assert_eq!(0, cd[1]);
        assert_eq!(3, cd[2]);
        assert_eq!(20, cd[19]);
    }

    #[test]
    fn test_can_bid_pass_play() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Buying phase: only the bidding player can bid/pass
        assert!(g.can_bid(0));
        assert!(g.can_pass(0));
        assert!(!g.can_bid(1));
        assert!(!g.can_pass(1));
        assert!(!g.can_play(0));
        g.bidding_player = 1;
        assert!(!g.can_bid(0));
        assert!(g.can_bid(1));
    }

    #[test]
    fn test_bid_errors() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        // Not your turn
        assert!(g.command(1, "bid 1", &p).is_err());
        // Must bid higher than 0 (the initial "no bid" highest is 0)
        assert!(g.command(0, "bid 0", &p).is_err());
        // Bid more than chips (15) - parser rejects via Int max
        assert!(g.command(0, "bid 16", &p).is_err());
        // Valid first bid
        g.command(0, "bid 1", &p).unwrap();
        // Steve must bid higher than 1
        assert!(g.command(1, "bid 1", &p).is_err());
        g.command(1, "bid 2", &p).unwrap();
    }

    #[test]
    fn test_pass_takes_lowest_pays_half() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        // Fix a known open set: [1, 2, 3] sorted
        g.open_cards = vec![1, 2, 3];
        g.bids[0] = 4;
        g.bidding_player = 0;
        g.command(0, "pass", &p).unwrap();
        // Took the lowest (1), paid floor(4/2)=2
        assert_eq!(vec![2, 3], g.open_cards);
        assert_eq!(vec![1], g.hands[0]);
        assert_eq!(STARTING_CHIPS - 2, g.chips[0]);
        assert!(g.finished_bidding[0]);
        // Turn advanced to next non-finished player
        assert_eq!(1, g.bidding_player);
    }

    #[test]
    fn test_last_bidder_pays_full_takes_highest() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        g.open_cards = vec![1, 2, 3];
        // BJ has a standing bid of 6; the other two pass, leaving BJ as the last
        // remaining bidder who auto-takes the highest building for the full price.
        g.bids[BJ] = 6;
        g.bidding_player = MICK;
        g.command(MICK, "pass", &p).unwrap();
        g.command(STEVE, "pass", &p).unwrap();
        // Passers took the low buildings (1, 2); BJ took the highest (3) for 6.
        assert_eq!(vec![1], g.hands[MICK]);
        assert_eq!(vec![2], g.hands[STEVE]);
        assert_eq!(vec![3], g.hands[BJ]);
        assert_eq!(STARTING_CHIPS - 6, g.chips[BJ]);
        assert_eq!(BJ, g.bidding_player);
    }

    #[test]
    fn test_play_resolves_cheques() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        // Force a selling round: cheques [4, 5, 6] already drawn, each player has
        // one building. (open_cards set directly so the hands-len-1 autoplay in
        // start_selling_round doesn't fire immediately.)
        g.building_deck = vec![];
        g.cheque_deck = vec![0, 0, 3];
        g.open_cards = vec![4, 5, 6];
        g.hands[MICK] = vec![17];
        g.hands[STEVE] = vec![18];
        g.hands[BJ] = vec![16];
        // Selling phase, all players can play
        assert_eq!(vec![MICK, STEVE, BJ], g.whose_turn());
        g.command(BJ, "play 16", &p).unwrap();
        g.command(MICK, "play 17", &p).unwrap();
        g.command(STEVE, "play 18", &p).unwrap();
        // Lowest building (16, BJ) gets lowest cheque (4); 17 (Mick) -> 5; 18 (Steve) -> 6
        assert_eq!(vec![4], g.cheques[BJ]);
        assert_eq!(vec![5], g.cheques[MICK]);
        assert_eq!(vec![6], g.cheques[STEVE]);
    }

    #[test]
    fn test_play_wrong_card_and_wrong_player() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        g.building_deck = vec![];
        g.cheque_deck = vec![0, 0, 3];
        g.open_cards = vec![4, 5, 6];
        g.hands[MICK] = vec![17];
        g.hands[STEVE] = vec![18];
        g.hands[BJ] = vec![16];
        // Don't have that card
        assert!(g.command(MICK, "play 99", &p).is_err());
        // After playing, can't play again
        g.command(MICK, "play 17", &p).unwrap();
        assert!(g.command(MICK, "play 17", &p).is_err());
        // Wrong player (Steve can act, but Mick cannot once finished)
        assert!(g.command(MICK, "play 17", &p).is_err());
    }

    #[test]
    fn test_command_after_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        // Force a finished state
        g.building_deck = vec![];
        g.cheque_deck = vec![];
        g.open_cards = vec![];
        g.hands = vec![vec![], vec![], vec![]];
        assert!(g.is_finished());
        assert!(g.command(0, "pass", &p).is_err());
        assert!(g.command(0, "bid 1", &p).is_err());
        assert!(g.command(0, "play 1", &p).is_err());
    }

    #[test]
    fn test_placings() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Mick: 22, Steve: 17, BJ: 19 -> placings [1, 3, 2]
        g.cheques[0] = vec![5, 3];
        g.chips[0] = 14;
        g.cheques[1] = vec![6, 0];
        g.chips[1] = 11;
        g.cheques[2] = vec![4, 0];
        g.chips[2] = 15;
        g.building_deck = vec![];
        g.cheque_deck = vec![];
        g.open_cards = vec![];
        assert_eq!(vec![1, 3, 2], g.placings());
    }

    #[test]
    fn test_placings_tie_standard_competition() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Two players (Mick, Steve) tied at the top with 20; BJ lower with 5.
        g.cheques[MICK] = vec![10];
        g.chips[MICK] = 10;
        g.cheques[STEVE] = vec![10];
        g.chips[STEVE] = 10;
        g.cheques[BJ] = vec![];
        g.chips[BJ] = 5;
        g.building_deck = vec![];
        g.cheque_deck = vec![];
        g.open_cards = vec![];
        // Rust gen_placings uses standard-competition: two tied at top -> [1, 1, 3]
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_points_zero_until_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.cheques[0] = vec![5];
        g.chips[0] = 14;
        assert_eq!(vec![0.0, 0.0, 0.0], g.points());
        g.building_deck = vec![];
        g.cheque_deck = vec![];
        g.open_cards = vec![];
        let pts = g.points();
        assert_eq!(19.0, pts[0]);
    }

    #[test]
    fn test_pub_state_redacts_hands_and_cheques() {
        let (g, _) = Game::start(3, 1).unwrap();
        let ps = g.pub_state();
        // PubState must not contain per-player hands/cheques (hidden info)
        assert!(!ps.finished);
        assert_eq!(g.open_cards, ps.open_cards);
        assert_eq!(g.bids, ps.bids);
        // PlayerState carries the player's own hand/cheques/chips
        let pls = g.player_state(0);
        assert_eq!(g.hands[0], pls.hand);
        assert_eq!(g.cheques[0], pls.cheques);
        assert_eq!(g.chips[0], pls.chips);
    }

    #[test]
    fn test_finished_pub_state_is_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.building_deck = vec![];
        g.cheque_deck = vec![];
        g.open_cards = vec![];
        let ps = g.pub_state();
        assert!(ps.finished);
    }
}
