use std::collections::HashMap;
use std::default::Default;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::card::{Card, Rank, Suit, suits};
use crate::command::Command;

pub mod card;
mod command;
mod render;

const MIN_PLAYERS: usize = 3;
const MAX_PLAYERS: usize = 5;
const INITIAL_MONEY: i32 = 100;
const ROUNDS: usize = 4;

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub enum State {
    #[default]
    PlayCard,
    Auction,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub player_money: Vec<i32>,
    pub player_hands: Vec<Vec<Card>>,
    pub player_purchases: Vec<Vec<Card>>,
    pub state: State,
    pub round: usize,
    pub deck: Vec<Card>,
    pub current_player: usize,
    pub value_board: Vec<HashMap<Suit, i32>>,
    pub finished: bool,
    pub currently_auctioning: Vec<Card>,
    pub bids: HashMap<usize, i32>,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub round: usize,
    pub finished: bool,
    pub is_auction: bool,
    pub current_player: usize,
    pub auctioning: Vec<Card>,
    pub auction_type: Option<Rank>,
    pub current_bid: Option<(usize, i32)>,
    pub purchases: Vec<Vec<Card>>,
    pub value_board: Vec<HashMap<Suit, i32>>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub money: i32,
    pub hand: Vec<Card>,
}

fn round_cards(players: usize, round: usize) -> usize {
    let table: [usize; 4] = match players {
        3 => [10, 6, 6, 0],
        4 => [9, 4, 4, 0],
        5 => [8, 3, 3, 0],
        _ => unreachable!(),
    };
    table[round]
}

impl Game {
    pub fn is_auction(&self) -> bool {
        self.state == State::Auction
    }

    pub fn auction_card(&self) -> Option<Card> {
        self.currently_auctioning.last().copied()
    }

    pub fn auction_type(&self) -> Option<Rank> {
        self.auction_card().map(|c| c.rank)
    }

    pub fn suit_cards_on_table(&self, suit: Suit) -> usize {
        let mut count = 0;
        for hand in &self.player_purchases {
            count += hand.iter().filter(|c| c.suit == suit).count();
        }
        count += self
            .currently_auctioning
            .iter()
            .filter(|c| c.suit == suit)
            .count();
        count
    }

    pub fn suit_value(&self, suit: Suit) -> i32 {
        self.value_board
            .iter()
            .map(|values| *values.get(&suit).unwrap_or(&0))
            .sum()
    }

    pub fn is_players_turn(&self, player: usize) -> bool {
        self.whose_turn_players().contains(&player)
    }

    /// Named distinctly from the `Gamer::whose_turn` default (which delegates
    /// to `status()`) since this is also needed internally by the auction
    /// state machine (settling, availability checks).
    pub fn whose_turn_players(&self) -> Vec<usize> {
        if self.finished {
            return vec![];
        }
        match self.state {
            State::PlayCard => vec![self.current_player],
            State::Auction => match self.auction_type() {
                Some(Rank::Open) => {
                    let (highest_bidder, _) = self.highest_bidder();
                    (0..self.players)
                        .filter(|&p| {
                            let bid = self.bids.get(&p);
                            p != highest_bidder && (bid.is_none() || *bid.unwrap() > 0)
                        })
                        .collect()
                }
                Some(Rank::FixedPrice) | Some(Rank::Double) => {
                    for i in 0..self.players {
                        let p = (i + self.current_player) % self.players;
                        if !self.bids.contains_key(&p) {
                            return vec![p];
                        }
                    }
                    vec![]
                }
                Some(Rank::OnceAround) => {
                    let mut highest_bid = 0;
                    for i in 0..self.players {
                        let p = (1 + i + self.current_player) % self.players;
                        if let Some(&bid) = self.bids.get(&p)
                            && bid > highest_bid
                        {
                            highest_bid = bid;
                        }
                        if !(self.bids.contains_key(&p)
                            || (highest_bid == 0 && p == self.current_player))
                        {
                            return vec![p];
                        }
                    }
                    vec![]
                }
                Some(Rank::Sealed) => (0..self.players)
                    .filter(|p| !self.bids.contains_key(p))
                    .collect(),
                None => vec![],
            },
        }
    }

    pub fn highest_bidder(&self) -> (usize, i32) {
        let mut player = 0;
        let mut bid = -1;
        for i in self.current_player..self.current_player + self.players {
            let p = i % self.players;
            let b = *self.bids.get(&p).unwrap_or(&0);
            if b > bid {
                player = p;
                bid = b;
            }
        }
        (player, bid)
    }

    pub fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % self.players;
    }

    pub fn can_play(&self, player: usize) -> bool {
        !self.finished && self.is_players_turn(player) && self.state == State::PlayCard
    }

    pub fn can_pass(&self, player: usize) -> bool {
        if self.is_auction() {
            match self.auction_type() {
                Some(Rank::Open)
                | Some(Rank::Sealed)
                | Some(Rank::Double)
                | Some(Rank::OnceAround) => self.is_players_turn(player),
                Some(Rank::FixedPrice) => {
                    player != self.current_player && self.is_players_turn(player)
                }
                None => false,
            }
        } else {
            false
        }
    }

    pub fn can_bid(&self, player: usize) -> bool {
        if self.is_auction() {
            matches!(
                self.auction_type(),
                Some(Rank::Open) | Some(Rank::Sealed) | Some(Rank::OnceAround)
            ) && self.is_players_turn(player)
                // If the player couldn't afford to beat the current bid, there's
                // no valid bid amount to offer - they can only pass. This keeps
                // the bid parser's Int bounds (min..=max) always valid.
                && self.min_bid() <= self.player_money[player]
        } else {
            false
        }
    }

    /// The minimum amount a bid must exceed to be valid: one more than the
    /// current highest bid, or 1 for sealed auctions (which have no visible
    /// highest bid to beat).
    pub fn min_bid(&self) -> i32 {
        if self.auction_type() == Some(Rank::Sealed) {
            1
        } else {
            let (_, highest) = self.highest_bidder();
            highest + 1
        }
    }

    pub fn can_add(&self, player: usize) -> bool {
        self.is_auction()
            && self.auction_type() == Some(Rank::Double)
            && self.is_players_turn(player)
            && !self.player_hands.get(player).unwrap_or(&vec![]).is_empty()
    }

    pub fn can_buy(&self, player: usize) -> bool {
        self.is_auction()
            && self.auction_type() == Some(Rank::FixedPrice)
            && self.is_players_turn(player)
            && self.current_player != player
    }

    pub fn can_set_price(&self, player: usize) -> bool {
        self.is_auction()
            && self.auction_type() == Some(Rank::FixedPrice)
            && self.is_players_turn(player)
            && self.current_player == player
    }

    fn start_round(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        self.state = State::PlayCard;
        let num_cards = round_cards(self.players, self.round);
        logs.push(Log::public(vec![N::text(format!(
            "Start of round {}",
            self.round + 1
        ))]));
        if num_cards > 0 {
            logs.push(Log::public(vec![N::text(format!(
                "Dealing {} cards to each player",
                num_cards
            ))]));
        }
        // Mirrors the Go quirk of resetting PlayerPurchases inside the loop -
        // net effect is all purchases are cleared at the start of the round.
        self.player_purchases = vec![vec![]; self.players];
        for p in 0..self.players {
            if num_cards > 0 {
                let n = num_cards.min(self.deck.len());
                let drawn: Vec<Card> = self.deck.drain(self.deck.len() - n..).collect();
                self.player_hands[p].extend(drawn);
                self.player_hands[p].sort();
            }
        }
        logs
    }

    fn end_round(&mut self) -> Vec<Log> {
        let mut logs = vec![Log::public(vec![N::Bold(vec![N::text(
            "It is the end of the round",
        )])])];
        self.currently_auctioning = vec![];
        let mut counts: HashMap<Suit, usize> = HashMap::new();
        for s in suits() {
            counts.insert(s, self.suit_cards_on_table(s));
        }
        let mut scored: HashMap<Suit, bool> = HashMap::new();
        let mut values: HashMap<Suit, i32> = HashMap::new();
        for &v in &[30, 20, 10] {
            let mut highest = suits()[0];
            let mut highest_count: i64 = -1;
            for s in suits() {
                if !*scored.get(&s).unwrap_or(&false) && counts[&s] as i64 > highest_count {
                    highest = s;
                    highest_count = counts[&s] as i64;
                }
            }
            scored.insert(highest, true);
            values.insert(highest, v);
            logs.push(Log::public(vec![
                N::text("Adding "),
                render::money(v),
                N::text(" to the value of "),
                render::suit(highest),
                N::text(format!(" ({} cards)", highest_count)),
            ]));
        }
        self.value_board.push(values);
        for p in 0..self.players {
            let mut p_total = 0;
            for c in &self.player_purchases[p] {
                p_total += self.suit_value(c.suit);
            }
            logs.push(Log::public(vec![
                N::text("Paying "),
                N::Player(p),
                N::text(" "),
                render::money(p_total),
                N::text(" for selling all their cards"),
            ]));
            self.player_money[p] += p_total;
        }
        if self.round == ROUNDS - 1 {
            let mut money_rows: Vec<brdgme_markup::Row> = vec![];
            for p in 0..self.players {
                money_rows.push(vec![
                    (brdgme_markup::Align::Left, vec![N::Player(p)]),
                    (
                        brdgme_markup::Align::Left,
                        vec![render::money(self.player_money[p])],
                    ),
                ]);
            }
            logs.push(Log::public(vec![
                N::Bold(vec![N::text("End of the game, final player money:")]),
                N::text("\n"),
                brdgme_markup::table_with_gap(&money_rows, 1),
            ]));
            self.finished = true;
        } else {
            self.round += 1;
            self.next_player();
            logs.extend(self.start_round());
        }
        logs
    }

    pub fn play_card(&mut self, player: usize, c: Card) -> Result<Vec<Log>, GameError> {
        if !self.can_play(player) {
            return Err(GameError::invalid_input(
                "You're not able to play a card at the moment",
            ));
        }
        self.currently_auctioning = vec![];
        self.add_card_to_auction(player, c)
    }

    pub fn add_card(&mut self, player: usize, c: Card) -> Result<Vec<Log>, GameError> {
        if !self.can_add(player) {
            return Err(GameError::invalid_input(
                "You're not able to add a card at the moment",
            ));
        }
        if self.auction_card().map(|ac| ac.suit) != Some(c.suit) {
            return Err(GameError::invalid_input(
                "The artist of the card must match the existing one",
            ));
        }
        if c.rank == Rank::Double {
            return Err(GameError::invalid_input(
                "You are not allowed to add a second double auction",
            ));
        }
        self.add_card_to_auction(player, c)
    }

    fn add_card_to_auction(&mut self, player: usize, c: Card) -> Result<Vec<Log>, GameError> {
        let hand = self
            .player_hands
            .get_mut(player)
            .ok_or_else(|| GameError::internal("invalid player number"))?;
        let index = hand
            .iter()
            .position(|&hc| hc == c)
            .ok_or_else(|| GameError::invalid_input("You do not have that card in your hand"))?;
        hand.remove(index);
        self.current_player = player;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card_name(c),
        ])];
        self.currently_auctioning.push(c);
        self.bids = HashMap::new();
        self.state = State::Auction;
        if self.suit_cards_on_table(c.suit) >= 5 {
            logs.extend(self.end_round());
        }
        Ok(logs)
    }

    fn settle_auction(&mut self, winner: usize, price: i32) -> Vec<Log> {
        let mut logs = vec![];
        self.player_money[winner] -= price;
        let cards = std::mem::take(&mut self.currently_auctioning);
        self.player_purchases[winner].extend(cards.clone());
        self.player_purchases[winner].sort();
        let paid_to = if winner != self.current_player {
            self.player_money[self.current_player] += price;
            N::Player(self.current_player)
        } else {
            N::text("the bank")
        };
        logs.push(Log::public(vec![
            N::Player(winner),
            N::text(" bought "),
            render::card_names(&cards),
            N::text(", paying "),
            render::money(price),
            N::text(" to "),
            paid_to,
        ]));
        self.state = State::PlayCard;
        self.next_player();
        while self.player_hands[self.current_player].is_empty() {
            logs.push(Log::public(vec![
                N::text("Skipping "),
                N::Player(self.current_player),
                N::text(" as they have no cards"),
            ]));
            self.next_player();
        }
        logs
    }

    pub fn pass(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_pass(player) {
            return Err(GameError::invalid_input(
                "You're not able to pass at the moment",
            ));
        }
        let mut logs = vec![];
        self.bids.insert(player, 0);
        if self.auction_type() != Some(Rank::Sealed) {
            logs.push(Log::public(vec![N::Player(player), N::text(" passed")]));
        }
        match self.auction_type() {
            Some(Rank::FixedPrice) => {
                if self.bids.len() == self.players {
                    let price = *self.bids.get(&self.current_player).unwrap_or(&0);
                    let cp = self.current_player;
                    logs.extend(self.settle_auction(cp, price));
                }
            }
            _ => {
                if self.whose_turn_players().is_empty() {
                    let (winner, bid) = self.highest_bidder();
                    logs.extend(self.settle_auction(winner, bid));
                }
            }
        }
        Ok(logs)
    }

    pub fn bid(&mut self, player: usize, amount: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_bid(player) {
            return Err(GameError::invalid_input(
                "You're not able to bid at the moment",
            ));
        }
        if amount > self.player_money[player] {
            return Err(GameError::invalid_input(format!(
                "You must not bid higher than the money you have, which is ${}",
                self.player_money[player]
            )));
        }
        if self.auction_type() != Some(Rank::Sealed) {
            let (_, highest_bid) = self.highest_bidder();
            if amount <= highest_bid {
                return Err(GameError::invalid_input(format!(
                    "You must bid higher than ${}",
                    highest_bid
                )));
            }
        }
        self.bids.insert(player, amount);
        let mut logs = vec![];
        if self.auction_type() != Some(Rank::Sealed) {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" bid "),
                render::money(amount),
            ]));
        }
        if self.whose_turn_players().is_empty() {
            let (winner, bid) = self.highest_bidder();
            logs.extend(self.settle_auction(winner, bid));
        }
        Ok(logs)
    }

    pub fn buy(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_buy(player) {
            return Err(GameError::invalid_input(
                "You're not able to buy the card at the moment",
            ));
        }
        let price = *self.bids.get(&self.current_player).unwrap_or(&0);
        if price > self.player_money[player] {
            return Err(GameError::invalid_input(
                "You don't have enough money to buy the card",
            ));
        }
        Ok(self.settle_auction(player, price))
    }

    pub fn set_price(&mut self, player: usize, price: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_set_price(player) {
            return Err(GameError::invalid_input(
                "You're not able to set the price at the moment",
            ));
        }
        if price <= 0 {
            return Err(GameError::invalid_input(
                "The price you set must be higher than 0",
            ));
        }
        if price > self.player_money[player] {
            return Err(GameError::invalid_input(
                "You can't set the price higher than your current money",
            ));
        }
        self.bids.insert(player, price);
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" set the price to "),
            render::money(price),
        ])])
    }

    fn placings(&self) -> Vec<usize> {
        gen_placings(
            &(0..self.players)
                .map(|p| vec![self.player_money[p]])
                .collect::<Vec<Vec<i32>>>(),
        )
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
        let mut deck = card::deck();
        let mut rng = GameRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);
        let mut g = Game {
            players,
            player_money: vec![INITIAL_MONEY; players],
            player_hands: vec![vec![]; players],
            player_purchases: vec![vec![]; players],
            deck,
            rng,
            ..Game::default()
        };
        let logs = g.start_round();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.finished {
            Status::Finished {
                placings: self.placings(),
                stats: vec![HashMap::new(); self.players],
            }
        } else {
            Status::Active {
                whose_turn: self.whose_turn_players(),
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            round: self.round,
            finished: self.finished,
            is_auction: self.is_auction(),
            current_player: self.current_player,
            auctioning: self.currently_auctioning.clone(),
            auction_type: self.auction_type(),
            current_bid: if self.is_auction() && self.auction_type() != Some(Rank::Sealed) {
                Some(self.highest_bidder())
            } else {
                None
            },
            purchases: self.player_purchases.clone(),
            value_board: self.value_board.clone(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            money: self.player_money[player],
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
                remaining,
                value: Command::Play(c),
                ..
            }) => self.play_card(player, c).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Add(c),
                ..
            }) => self.add_card(player, c).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Bid(amount),
                ..
            }) => self.bid(player, amount).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Buy,
                ..
            }) => self.buy(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Pass,
                ..
            }) => self.pass(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Price(amount),
                ..
            }) => self.set_price(player, amount).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Err(e) => Err(e),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        let mut points = vec![0.0; self.players];
        if !self.finished {
            // "Points" are cash, and cash is secret until the end of the game.
            return points;
        }
        for (p, money) in points.iter_mut().enumerate() {
            *money = self.player_money[p] as f32;
        }
        points
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
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
    use crate::card::{Rank, Suit};

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const BJ: usize = 2;
    const ELVA: usize = 3;

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    fn mock_game() -> Game {
        Game::start(4, 1).unwrap().0
    }

    #[test]
    fn test_deck() {
        let d = card::deck();
        assert_eq!(70, d.len());
        assert_eq!(12, d.iter().filter(|c| c.suit == Suit::LiteMetal).count());
        assert_eq!(13, d.iter().filter(|c| c.suit == Suit::Yoko).count());
        assert_eq!(14, d.iter().filter(|c| c.suit == Suit::ChristineP).count());
        assert_eq!(15, d.iter().filter(|c| c.suit == Suit::KarlGitter).count());
        assert_eq!(16, d.iter().filter(|c| c.suit == Suit::Krypto).count());
    }

    #[test]
    fn test_start() {
        let g = mock_game();
        assert_eq!(9, g.player_hands[0].len());
        assert_eq!(9, g.player_hands[1].len());
        assert_eq!(9, g.player_hands[2].len());
        assert_eq!(9, g.player_hands[3].len());
        assert_eq!(34, g.deck.len());
        assert_eq!(100, g.player_money[0]);
        assert_eq!(100, g.player_money[1]);
        assert_eq!(100, g.player_money[2]);
        assert_eq!(100, g.player_money[3]);
    }

    #[test]
    fn test_open_auction() {
        let p = players(4);
        // Give BJ a Lite Metal Open Auction card and let him play it.
        {
            let mut g = mock_game();
            g.current_player = BJ;
            g.player_hands[BJ].push(Card {
                suit: Suit::LiteMetal,
                rank: Rank::Open,
            });
            g.command(BJ, "play lmop", &p).unwrap();
            assert_eq!(State::Auction, g.state);
            assert_eq!(1, g.currently_auctioning.len());

            // Steve bids, everyone else passes.
            {
                let mut g = g.clone();
                g.command(STEVE, "bid 10", &p).unwrap();
                assert_eq!(State::Auction, g.state);
                g.command(MICK, "pass", &p).unwrap();
                g.command(BJ, "pass", &p).unwrap();
                g.command(ELVA, "pass", &p).unwrap();
                assert_eq!(State::PlayCard, g.state);
                assert_eq!(ELVA, g.current_player);
                assert_eq!(1, g.player_purchases[STEVE].len());
                assert_eq!(90, g.player_money[STEVE]);
                assert_eq!(110, g.player_money[BJ]);
            }

            // Nobody bids.
            {
                let mut g = g.clone();
                g.command(MICK, "pass", &p).unwrap();
                g.command(STEVE, "pass", &p).unwrap();
                g.command(ELVA, "pass", &p).unwrap();
                assert_eq!(State::PlayCard, g.state);
                assert_eq!(ELVA, g.current_player);
                assert_eq!(1, g.player_purchases[BJ].len());
                assert_eq!(100, g.player_money[BJ]);
            }
        }
    }

    #[test]
    fn test_fixed_price_auction() {
        let p = players(4);
        let mut g = mock_game();
        g.current_player = ELVA;
        g.player_hands[ELVA].push(Card {
            suit: Suit::ChristineP,
            rank: Rank::FixedPrice,
        });
        g.command(ELVA, "play cpfp", &p).unwrap();
        assert_eq!(State::Auction, g.state);
        assert_eq!(1, g.currently_auctioning.len());
        g.command(ELVA, "price 15", &p).unwrap();

        // Mick passes and Steve buys.
        {
            let mut g = g.clone();
            g.command(MICK, "pass", &p).unwrap();
            assert_eq!(State::Auction, g.state);
            g.command(STEVE, "buy", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(MICK, g.current_player);
            assert_eq!(1, g.player_purchases[STEVE].len());
            assert_eq!(85, g.player_money[STEVE]);
            assert_eq!(115, g.player_money[ELVA]);
        }

        // Nobody buys.
        {
            let mut g = g.clone();
            g.command(MICK, "pass", &p).unwrap();
            g.command(STEVE, "pass", &p).unwrap();
            g.command(BJ, "pass", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(MICK, g.current_player);
            assert_eq!(1, g.player_purchases[ELVA].len());
            assert_eq!(85, g.player_money[ELVA]);
        }
    }

    #[test]
    fn test_sealed_auction() {
        let p = players(4);
        let mut g = mock_game();
        g.current_player = ELVA;
        g.player_hands[ELVA].push(Card {
            suit: Suit::Krypto,
            rank: Rank::Sealed,
        });
        g.command(ELVA, "play krsl", &p).unwrap();
        assert_eq!(State::Auction, g.state);
        assert_eq!(1, g.currently_auctioning.len());

        // Everyone bids different amounts.
        {
            let mut g = g.clone();
            g.command(MICK, "bid 4", &p).unwrap();
            g.command(STEVE, "bid 5", &p).unwrap();
            g.command(BJ, "bid 3", &p).unwrap();
            g.command(ELVA, "bid 1", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(MICK, g.current_player);
            assert_eq!(1, g.player_purchases[STEVE].len());
            assert_eq!(95, g.player_money[STEVE]);
            assert_eq!(105, g.player_money[ELVA]);
        }

        // Nobody bids.
        {
            let mut g = g.clone();
            g.command(MICK, "pass", &p).unwrap();
            g.command(STEVE, "pass", &p).unwrap();
            g.command(ELVA, "pass", &p).unwrap();
            g.command(BJ, "pass", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(MICK, g.current_player);
            assert_eq!(1, g.player_purchases[ELVA].len());
            assert_eq!(100, g.player_money[ELVA]);
        }
    }

    #[test]
    fn test_double_auction() {
        let p = players(4);
        let mut g = mock_game();
        g.current_player = ELVA;
        g.player_hands[ELVA].push(Card {
            suit: Suit::KarlGitter,
            rank: Rank::Double,
        });
        g.player_hands[ELVA].push(Card {
            suit: Suit::KarlGitter,
            rank: Rank::Sealed,
        });
        g.player_hands[STEVE].push(Card {
            suit: Suit::KarlGitter,
            rank: Rank::Sealed,
        });
        g.command(ELVA, "play kgdb", &p).unwrap();
        assert_eq!(State::Auction, g.state);
        assert_eq!(1, g.currently_auctioning.len());

        g.command(ELVA, "pass", &p).unwrap();
        g.command(MICK, "pass", &p).unwrap();
        g.command(STEVE, "add kgsl", &p).unwrap();
        assert_eq!(STEVE, g.current_player);
        assert_eq!(2, g.currently_auctioning.len());
        assert!(g.is_auction());
        assert_eq!(Some(Rank::Sealed), g.auction_type());

        g.command(MICK, "bid 8", &p).unwrap();
        g.command(STEVE, "bid 5", &p).unwrap();
        g.command(BJ, "bid 3", &p).unwrap();
        g.command(ELVA, "bid 1", &p).unwrap();
        assert_eq!(State::PlayCard, g.state);
        assert_eq!(BJ, g.current_player);
        assert_eq!(2, g.player_purchases[MICK].len());
        assert_eq!(92, g.player_money[MICK]);
        assert_eq!(108, g.player_money[STEVE]);
        assert_eq!(100, g.player_money[ELVA]);
    }

    #[test]
    fn test_double_auction_ends_round() {
        let p = players(3);
        let mut g = Game::start(3, 1).unwrap().0;
        g.current_player = MICK;
        let lm_double = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Double,
        };
        let lm_open = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        };
        g.player_purchases = vec![vec![lm_double], vec![lm_double], vec![lm_double]];
        g.player_hands[MICK] = vec![lm_double, lm_double, lm_double, lm_double];
        g.player_hands[STEVE] = vec![lm_open, lm_open, lm_open, lm_open];
        g.command(MICK, "play lmdb", &p).unwrap();
        g.command(MICK, "pass", &p).unwrap();
        g.command(STEVE, "add lmop", &p).unwrap();
        assert_eq!(1, g.round);
        assert_eq!(BJ, g.current_player);
    }

    #[test]
    fn test_once_around_auction() {
        let p = players(4);
        let mut g = mock_game();
        g.current_player = MICK;
        g.player_hands[MICK].push(Card {
            suit: Suit::Yoko,
            rank: Rank::OnceAround,
        });
        g.command(MICK, "play yooa", &p).unwrap();
        assert_eq!(State::Auction, g.state);
        assert_eq!(1, g.currently_auctioning.len());

        // Some bids.
        {
            let mut g = g.clone();
            g.command(STEVE, "pass", &p).unwrap();
            g.command(BJ, "bid 5", &p).unwrap();
            g.command(ELVA, "bid 7", &p).unwrap();
            g.command(MICK, "pass", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(STEVE, g.current_player);
            assert_eq!(1, g.player_purchases[ELVA].len());
            assert_eq!(107, g.player_money[MICK]);
            assert_eq!(93, g.player_money[ELVA]);
        }

        // Everyone passes.
        {
            let mut g = g.clone();
            g.command(STEVE, "pass", &p).unwrap();
            g.command(BJ, "pass", &p).unwrap();
            g.command(ELVA, "pass", &p).unwrap();
            assert_eq!(State::PlayCard, g.state);
            assert_eq!(STEVE, g.current_player);
            assert_eq!(1, g.player_purchases[MICK].len());
            assert_eq!(100, g.player_money[MICK]);
        }
    }

    #[test]
    fn test_end_of_round() {
        let p = players(4);
        let g = mock_game();
        let lm_open = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Open,
        };
        let lm_double = Card {
            suit: Suit::LiteMetal,
            rank: Rank::Double,
        };

        // 3 Lite Metal already on the board; Mick plays a double, then adds
        // another Lite Metal.
        {
            let mut g = g.clone();
            g.player_purchases[MICK] = vec![lm_open, lm_open];
            g.player_purchases[STEVE] = vec![lm_open];
            g.player_hands[MICK].push(lm_double);
            g.command(MICK, "play lmdb", &p).unwrap();
            assert_eq!(0, g.round);

            g.player_hands[MICK].push(lm_open);
            g.command(MICK, "add lmop", &p).unwrap();
            assert_eq!(1, g.round);
            assert_eq!(30, g.suit_value(Suit::LiteMetal));
            assert_eq!(160, g.player_money[MICK]);
            assert_eq!(130, g.player_money[STEVE]);
            assert_eq!(100, g.player_money[BJ]);
            assert_eq!(100, g.player_money[ELVA]);
        }

        // 4 Lite Metal already on the board.
        {
            let mut g = g.clone();
            g.player_purchases[MICK] = vec![lm_open, lm_open];
            g.player_purchases[STEVE] = vec![lm_open];
            g.player_purchases[BJ] = vec![lm_open];

            // Mick plays a 5th to end the round immediately.
            {
                let mut g = g.clone();
                g.player_hands[MICK].push(lm_open);
                g.command(MICK, "play lmop", &p).unwrap();
                assert_eq!(1, g.round);
                assert_eq!(30, g.suit_value(Suit::LiteMetal));
                assert_eq!(160, g.player_money[MICK]);
                assert_eq!(130, g.player_money[STEVE]);
                assert_eq!(130, g.player_money[BJ]);
                assert_eq!(100, g.player_money[ELVA]);
            }

            // It is the final round; Mick plays a 5th to end the game.
            {
                let mut g = g.clone();
                g.round = 3;
                g.player_hands[MICK].push(lm_open);
                g.command(MICK, "play lmop", &p).unwrap();
                assert!(g.is_finished());
                assert_eq!(30, g.suit_value(Suit::LiteMetal));
                assert_eq!(160, g.player_money[MICK]);
                assert_eq!(130, g.player_money[STEVE]);
                assert_eq!(130, g.player_money[BJ]);
                assert_eq!(100, g.player_money[ELVA]);
                let placings = g.placings();
                assert_eq!(1, placings[MICK]);
            }
        }
    }

    #[test]
    fn test_pub_state_hides_sealed_bids_and_money() {
        let p = players(4);
        let mut g = mock_game();
        g.current_player = ELVA;
        g.player_hands[ELVA].push(Card {
            suit: Suit::Krypto,
            rank: Rank::Sealed,
        });
        g.command(ELVA, "play krsl", &p).unwrap();
        g.command(MICK, "bid 4", &p).unwrap();

        let pub_state = g.pub_state();
        let json = serde_json::to_string(&pub_state).unwrap();
        assert!(!json.contains("money"));
        assert!(!json.contains("hand"));
        assert!(!json.contains("current_bid") || json.contains("\"current_bid\":null"));
    }
}
