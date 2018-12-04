pub mod card;
mod command;
mod render;

use serde_derive::{Serialize, Deserialize};
use rand::{thread_rng, Rng};

use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::{CommandResponse, Gamer, Log, Stat, Status};
use brdgme_markup::Node as N;

use std::collections::{HashMap, HashSet};
use std::default::Default;

use crate::card::{expeditions, Card, Expedition, Value};
use crate::command::Command;

const INVESTMENTS: usize = 3;
pub const ROUNDS: usize = 3;
pub const START_ROUND: usize = 1;
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 3;
const MIN_VALUE: usize = 2;
const MAX_VALUE: usize = 10;
const HAND_SIZE_2P: usize = 8;
const HAND_SIZE_3P: usize = 7;
const EXP_COST_2P: isize = 20;
const EXP_COST_3P: isize = 15;
const EXP_BONUS_SIZE_2P: isize = 8;
const EXP_BONUS_SIZE_3P: isize = 7;

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Phase {
    PlayOrDiscard,
    DrawOrTake,
}

impl Default for Phase {
    fn default() -> Phase {
        Phase::PlayOrDiscard
    }
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub plays: usize,
    pub discards: usize,
    pub takes: usize,
    pub draws: usize,
    pub turns: usize,
    pub investments: usize,
    pub expeditions: usize,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub round: usize,
    pub phase: Phase,
    pub deck: Vec<Card>,
    pub discards: Vec<Card>,
    pub hands: Vec<Vec<Card>>,
    pub scores: Vec<Vec<isize>>,
    pub expeditions: Vec<Vec<Card>>,
    pub current_player: usize,
    pub discarded_expedition: Option<Expedition>,
    pub stats: Vec<Stats>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub players: usize,
    pub round: usize,
    pub is_finished: bool,
    pub phase: Phase,
    pub deck_remaining: usize,
    pub discards: HashMap<Expedition, Value>,
    pub scores: Vec<Vec<isize>>,
    pub expeditions: Vec<Vec<Card>>,
    pub current_player: usize,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub hand: Vec<Card>,
}

fn initial_deck() -> Vec<Card> {
    let mut deck: Vec<Card> = vec![];
    for e in card::expeditions() {
        for _ in 0..INVESTMENTS {
            deck.push((e, Value::Investment).into());
        }
        for v in MIN_VALUE..MAX_VALUE + 1 {
            deck.push((e, Value::N(v)).into());
        }
    }
    deck
}

impl Game {
    fn leaders(&self) -> HashSet<usize> {
        let mut lead: HashSet<usize> = HashSet::new();
        let mut highest: isize = std::isize::MIN;
        for p in 0..self.players {
            let score = self.player_score(p);
            if score > highest {
                highest = score;
                lead = HashSet::new();
            }
            if score == highest {
                lead.insert(p);
            }
        }
        lead
    }

    fn start_round(&mut self) -> Result<Vec<Log>, GameError> {
        let mut logs: Vec<Log> = vec![Log::public(vec![N::text(format!(
            "Starting round {}",
            self.round
        ))])];
        // Grab a new deck and shuffle it.
        let mut deck = initial_deck();
        thread_rng().shuffle(deck.as_mut_slice());
        self.deck = deck;
        // Clear out discards, hands and expeditions.
        self.discards = vec![];
        self.hands = vec![];
        self.expeditions = vec![];
        // Initialise player hands and expedition and draw initial cards.
        for p in 0..self.players {
            self.hands.push(vec![]);
            self.expeditions.push(vec![]);
            logs.extend(self.draw_hand_full(p)?);
        }
        if self.round > START_ROUND {
            let leaders = self.leaders();
            loop {
                self.current_player = self.next_player_num(self.current_player);
                if leaders.contains(&self.current_player) {
                    break;
                }
            }
        }
        self.start_turn();
        Ok(logs)
    }

    fn end_round(&mut self) -> Result<Vec<Log>, GameError> {
        self.round += 1;
        let mut logs: Vec<Log> = vec![];
        for p in 0..self.players {
            let mut round_score: isize = 0;
            if let Some(p_exp) = self.expeditions.get(p) {
                round_score = score(self.players, p_exp);
            }
            self.scores.get_mut(p).map(|s| s.push(round_score));
            logs.push(Log::public(vec![
                N::Player(p),
                N::text(" scored "),
                N::Bold(vec![N::text(format!("{}", round_score))]),
                N::text(" points, now on "),
                N::Bold(vec![N::text(format!("{}", self.player_score(p)))]),
            ]));
        }
        if self.round < START_ROUND + ROUNDS {
            self.start_round().map(|l| {
                logs.extend(l);
                logs
            })
        } else {
            logs.push(self.game_over_log());
            Ok(logs)
        }
    }

    fn game_over_log(&self) -> Log {
        Log::public(vec![N::Bold(vec![N::text("The game is over.")])])
    }

    fn assert_phase(&self, phase: Phase) -> Result<(), GameError> {
        if phase == self.phase {
            Ok(())
        } else {
            Err(GameError::invalid_input("not the right phase"))
        }
    }

    pub fn draw(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        self.assert_phase(Phase::DrawOrTake)?;
        let r = self.round;
        let logs = self.draw_hand_full(player)?;
        if r == self.round {
            // Only run next phase if a new round wasn't started, if a new round
            // was started then everything will already be initialised.
            self.next_phase();
        }
        self.stats[player].draws += 1;
        self.stats[player].turns += 1;
        Ok(logs)
    }

    fn next_phase(&mut self) {
        match self.phase {
            Phase::PlayOrDiscard => {
                self.phase = Phase::DrawOrTake;
            }
            Phase::DrawOrTake => {
                self.next_player();
            }
        }
    }

    fn next_player(&mut self) {
        self.current_player = self.next_player_num(self.current_player);
        self.start_turn();
    }

    fn next_player_num(&self, from: usize) -> usize {
        next_player(from, self.players)
    }

    fn start_turn(&mut self) {
        self.phase = Phase::PlayOrDiscard;
        self.discarded_expedition = None;
    }

    pub fn take(&mut self, player: usize, expedition: Expedition) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        self.assert_phase(Phase::DrawOrTake)?;
        if self.discarded_expedition == Some(expedition) {
            return Err(GameError::invalid_input(
                "you can't take the same card you just discarded",
            ));
        }
        if let Some(index) = self.discards
            .iter()
            .rposition(|&c| c.expedition == expedition)
        {
            let c = *self.discards
                .get(index)
                .ok_or_else(|| GameError::internal("could not find discard card"))?;
            self.hands
                .get_mut(player)
                .ok_or_else(|| GameError::internal("could not find player hand"))?
                .push(c);
            self.discards.remove(index);
            self.next_phase();
            self.stats[player].takes += 1;
            self.stats[player].turns += 1;
            Ok(vec![Log::public(vec![
                N::Player(player),
                N::text(" took "),
                render::card(&c),
            ])])
        } else {
            Err(GameError::invalid_input(
                "there are no discarded cards for that expedition",
            ))
        }
    }

    pub fn available_discard(&self, expedition: Expedition) -> Option<Card> {
        self.discards
            .iter()
            .rev()
            .find(|c| c.expedition == expedition)
            .cloned()
    }

    pub fn remove_player_card(&mut self, player: usize, c: Card) -> Result<(), GameError> {
        self.hands
            .get_mut(player)
            .ok_or_else(|| {
                GameError::internal(format!("could not find player hand for player {}", player))
            })
            .and_then(|h| {
                let index = h.iter()
                    .position(|hc| c == *hc)
                    .ok_or_else(|| GameError::invalid_input(format!("you don't have {}", c)))?;
                h.remove(index);
                Ok(())
            })?;
        Ok(())
    }

    pub fn discard(&mut self, player: usize, c: Card) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        self.assert_phase(Phase::PlayOrDiscard)?;
        self.remove_player_card(player, c)?;
        self.discards.push(c);
        self.discarded_expedition = Some(c.expedition);
        self.next_phase();
        self.stats[player].discards += 1;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" discarded "),
            render::card(&c),
        ])])
    }

    fn assert_has_card(&self, player: usize, c: Card) -> Result<(), GameError> {
        self.hands
            .get(player)
            .ok_or_else(|| {
                GameError::internal(format!("could not find player hand for player {}", player))
            })
            .and_then(|h| {
                h.iter()
                    .position(|hc| c == *hc)
                    .ok_or_else(|| GameError::invalid_input(format!("you don't have {}", c)))
            })?;
        Ok(())
    }

    fn highest_value_in_expedition(&self, player: usize, expedition: Expedition) -> Option<usize> {
        self.expeditions.get(player).and_then(|e| {
            e.iter()
                .filter(|&c| c.expedition == expedition && c.value != Value::Investment)
                .map(|&c| if let Value::N(n) = c.value { n } else { 0 })
                .max()
        })
    }

    pub fn play(&mut self, player: usize, c: Card) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;
        self.assert_phase(Phase::PlayOrDiscard)?;
        self.assert_has_card(player, c)?;
        if let Some(hn) = self.highest_value_in_expedition(player, c.expedition) {
            match c.value {
                Value::Investment => {
                    return Err(GameError::invalid_input(format!(
                        "you can't play {} as you've already played a higher card",
                        c
                    )));
                }
                Value::N(n) => if n <= hn {
                    return Err(GameError::invalid_input(format!(
                        "you can't play {} as you've already played a higher card",
                        c
                    )));
                },
            }
        }
        if self.expeditions
            .get(player)
            .ok_or_else(|| {
                GameError::internal(format!(
                    "could not find player expedition for player {}",
                    player
                ))
            })?
            .is_empty()
        {
            self.stats[player].expeditions += 1;
        }
        self.remove_player_card(player, c)?;
        self.expeditions
            .get_mut(player)
            .ok_or_else(|| {
                GameError::internal(format!(
                    "could not find player expedition for player {}",
                    player
                ))
            })?
            .push(c);
        self.next_phase();
        self.stats[player].plays += 1;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(&c),
        ])])
    }

    fn draw_hand_full(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        let mut logs: Vec<Log> = vec![];
        match self.hands.get_mut(player) {
            Some(hand) => {
                let mut num = hand_size(self.players) - hand.len();
                let dl = self.deck.len();
                if num > dl {
                    num = dl;
                }
                let mut drawn: Vec<Card> = vec![];
                for c in self.deck.drain(..num) {
                    hand.push(c);
                    drawn.push(c);
                }
                drawn.sort();
                let d_len = drawn.len();
                let mut public_log: Vec<N> = vec![N::Player(player), N::text(" drew ")];
                if d_len == 1 {
                    public_log.append(&mut vec![N::text("a card")]);
                } else {
                    public_log.append(&mut vec![
                        N::Bold(vec![N::text(format!("{}", drawn.len()))]),
                        N::text(" cards"),
                    ]);
                }
                public_log.append(&mut vec![
                    N::text(", "),
                    N::Bold(vec![N::text(format!("{}", self.deck.len()))]),
                    N::text(" remaining"),
                ]);
                logs.push(Log::public(public_log));
                let mut private_log: Vec<N> = vec![N::text("You drew ")];
                private_log.append(&mut render::comma_cards(&drawn));
                logs.push(Log::private(private_log, vec![player]));
            }
            None => return Err(GameError::internal("invalid player number")),
        };
        if self.deck.is_empty() {
            self.end_round()
        } else {
            Ok(logs)
        }
    }

    fn player_score(&self, player: usize) -> isize {
        match self.scores.get(player) {
            Some(s) => s.iter().sum(),
            None => 0,
        }
    }

    fn player_stats(&self, player: usize) -> HashMap<String, Stat> {
        let mut stats = HashMap::new();
        if player >= self.stats.len() {
            return stats;
        }
        stats.insert(
            "Plays".to_string(),
            Stat::Fraction(
                self.stats[player].plays as i32,
                self.stats[player].turns as i32,
            ),
        );
        stats.insert(
            "Discards".to_string(),
            Stat::Fraction(
                self.stats[player].discards as i32,
                self.stats[player].turns as i32,
            ),
        );
        stats.insert(
            "Draws".to_string(),
            Stat::Fraction(
                self.stats[player].draws as i32,
                self.stats[player].turns as i32,
            ),
        );
        stats.insert(
            "Takes".to_string(),
            Stat::Fraction(
                self.stats[player].takes as i32,
                self.stats[player].turns as i32,
            ),
        );
        stats
    }

    fn placings(&self) -> Vec<usize> {
        gen_placings(&(0..self.players)
            .map(|p| vec![self.player_score(p) as i32])
            .collect::<Vec<Vec<i32>>>())
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn new(players: usize) -> Result<(Self, Vec<Log>), GameError> {
        if players < MIN_PLAYERS || players > MAX_PLAYERS {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let mut stats = vec![];
        let mut scores = vec![];
        for _ in 0..players {
            stats.push(Stats::default());
            scores.push(vec![]);
        }
        let mut g = Game {
            players,
            round: START_ROUND,
            stats,
            scores,
            ..Game::default()
        };
        let logs = g.start_round()?;
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.round >= START_ROUND + ROUNDS {
            Status::Finished {
                placings: self.placings(),
                stats: vec![self.player_stats(0), self.player_stats(1)],
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
            round: self.round,
            is_finished: self.is_finished(),
            phase: self.phase,
            deck_remaining: self.deck.len(),
            discards: {
                let mut d: HashMap<Expedition, Value> = HashMap::new();
                for e in card::expeditions() {
                    if let Some(c) = card::last_expedition(&self.discards, e) {
                        d.insert(e, c.value);
                    }
                }
                d
            },
            scores: self.scores.clone(),
            expeditions: self.expeditions.clone(),
            current_player: self.current_player,
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: self.hands[player].clone(),
        }
    }

    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let cp = match self.command_parser(player) {
            Some(cp) => cp,
            None => return Err(GameError::invalid_input("not your turn")),
        };
        match cp.parse(input, players) {
            Ok(ParseOutput {
                value: Command::Play(c),
                remaining,
                ..
            }) => self.play(player, c).map(|l| CommandResponse {
                logs: l,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Discard(c),
                remaining,
                ..
            }) => self.discard(player, c).map(|l| CommandResponse {
                logs: l,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Take(e),
                remaining,
                ..
            }) => self.take(player, e).map(|l| CommandResponse {
                logs: l,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Draw,
                remaining,
                ..
            }) => self.draw(player).map(|l| CommandResponse {
                logs: l,
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
        (0..self.players)
            .map(|p| self.player_score(p) as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..MAX_PLAYERS).collect()
    }

    fn player_count(&self) -> usize {
        self.players
    }
}

fn next_player(player: usize, players: usize) -> usize {
    (player + 1) % players
}

fn expedition_cost(players: usize) -> isize {
    match players {
        2 => EXP_COST_2P,
        3 => EXP_COST_3P,
        _ => unreachable!(),
    }
}

fn hand_size(players: usize) -> usize {
    match players {
        2 => HAND_SIZE_2P,
        3 => HAND_SIZE_3P,
        _ => unreachable!(),
    }
}

fn expedition_bonus_size(players: usize) -> isize {
    match players {
        2 => EXP_BONUS_SIZE_2P,
        3 => EXP_BONUS_SIZE_3P,
        _ => unreachable!(),
    }
}

pub fn score(players: usize, cards: &[Card]) -> isize {
    let exp_cost = expedition_cost(players);
    let exp_bonus_size = expedition_bonus_size(players);

    let mut exp_cards: HashMap<Expedition, isize> = HashMap::new();
    let mut exp_inv: HashMap<Expedition, isize> = HashMap::new();
    let mut exp_sum: HashMap<Expedition, isize> = HashMap::new();
    for c in cards {
        let cards = exp_cards.entry(c.expedition).or_insert(0);
        *cards += 1;
        match c.value {
            Value::Investment => {
                let inv = exp_inv.entry(c.expedition).or_insert(0);
                *inv += 1;
            }
            Value::N(n) => {
                let sum = exp_sum.entry(c.expedition).or_insert(0);
                *sum += n as isize;
            }
        }
    }
    expeditions().iter().fold(0, |acc, &e| {
        let cards = exp_cards.get(&e);
        if cards == None {
            return acc;
        }
        acc + (exp_sum.get(&e).unwrap_or(&0) - exp_cost) * (exp_inv.get(&e).unwrap_or(&0) + 1)
            + if cards.unwrap() >= &exp_bonus_size {
                exp_cost
            } else {
                0
            }
    })
}

#[cfg(test)]
mod test {
    use super::card::{Expedition, Value};
    use super::*;
    use brdgme_game::Gamer;

    fn discard_and_draw(game: &mut Game, player: usize) {
        let c = game.hands[player][0];
        game.discard(player, c).unwrap();
        game.draw(player).unwrap();
    }

    #[test]
    fn start_works() {
        let game = Game::new(2).unwrap().0;
        assert_eq!(game.hands.len(), 2);
        assert_eq!(game.hands[0].len(), 8);
        assert_eq!(game.hands[1].len(), 8);
        assert_eq!(game.deck.len(), 44);
    }

    #[test]
    fn end_round_works() {
        let mut game = Game::new(2).unwrap().0;
        for _ in 0..44 {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            assert_eq!(START_ROUND, game.round);
            game.draw(p).unwrap();
        }
        assert_eq!(START_ROUND + 1, game.round);
        assert_eq!(game.hands[0].len(), 8);
        assert_eq!(game.hands[1].len(), 8);
        assert_eq!(game.deck.len(), 44);
        assert_eq!(game.scores, vec![vec![0], vec![0]]);
    }

    #[test]
    fn game_end_works() {
        let mut game = Game::new(2).unwrap().0;
        for _ in 0..(44 * ROUNDS) {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            game.draw(p).unwrap();
        }
        assert_eq!(game.is_finished(), true);
    }

    #[test]
    fn play_works() {
        let mut game = Game::new(2).unwrap().0;
        game.hands[0] = vec![
            (Expedition::Green, Value::Investment).into(),
            (Expedition::Green, Value::Investment).into(),
            (Expedition::Green, Value::N(2)).into(),
            (Expedition::Green, Value::N(3)).into(),
            (Expedition::Yellow, Value::Investment).into(),
            (Expedition::Yellow, Value::Investment).into(),
            (Expedition::Yellow, Value::N(2)).into(),
            (Expedition::Yellow, Value::N(3)).into(),
        ];
        game.play(0, (Expedition::Green, Value::Investment).into())
            .unwrap();
        game.draw(0).unwrap();
        discard_and_draw(&mut game, 1);
        game.play(0, (Expedition::Green, Value::N(2)).into())
            .unwrap();
        game.draw(0).unwrap();
        discard_and_draw(&mut game, 1);
        // Shouldn't be able to play GX now.
        assert!(
            game.play(0, (Expedition::Green, Value::Investment).into())
                .is_err()
        );
        game.play(0, (Expedition::Green, Value::N(3)).into())
            .unwrap();
        game.draw(0).unwrap();
        discard_and_draw(&mut game, 1);
        game.play(0, (Expedition::Yellow, Value::N(3)).into())
            .unwrap();
        game.draw(0).unwrap();
        discard_and_draw(&mut game, 1);
        // Shouldn't be able to play Y2 now.
        assert!(
            game.play(0, (Expedition::Yellow, Value::N(2)).into())
                .is_err()
        );
    }

    #[test]
    fn score_works() {
        assert_eq!(0, score(2, &vec![]));
        assert_eq!(-17, score(2, &vec![(Expedition::Red, Value::N(3)).into()]));
        assert_eq!(
            -34,
            score(
                2,
                &vec![
                    (Expedition::Red, Value::N(3)).into(),
                    (Expedition::Green, Value::N(3)).into(),
                ]
            )
        );
        assert_eq!(
            -30,
            score(
                2,
                &vec![
                    (Expedition::Red, Value::N(3)).into(),
                    (Expedition::Green, Value::N(3)).into(),
                    (Expedition::Green, Value::N(4)).into(),
                ]
            )
        );
        assert_eq!(
            -37,
            score(
                2,
                &vec![
                    (Expedition::Green, Value::Investment).into(),
                    (Expedition::Red, Value::N(3)).into(),
                    (Expedition::Green, Value::N(4)).into(),
                    (Expedition::Green, Value::N(6)).into(),
                ]
            )
        );
        assert_eq!(
            44,
            score(
                2,
                &vec![
                    (Expedition::Green, Value::N(2)).into(),
                    (Expedition::Green, Value::N(3)).into(),
                    (Expedition::Green, Value::N(4)).into(),
                    (Expedition::Green, Value::N(5)).into(),
                    (Expedition::Green, Value::N(6)).into(),
                    (Expedition::Green, Value::N(7)).into(),
                    (Expedition::Green, Value::N(8)).into(),
                    (Expedition::Green, Value::N(9)).into(),
                ]
            )
        );
    }

    #[test]
    fn placings_works() {
        let mut g = Game::new(2).expect("expected to create game").0;
        g.scores = vec![vec![200, 0, 0], vec![100, 50, 40]];
        assert_eq!(vec![1, 2], g.placings());
        g.scores = vec![vec![100, 50, 40], vec![200, 0, 0]];
        assert_eq!(vec![2, 1], g.placings());
        g.scores = vec![vec![100, 50, 40], vec![100, 50, 40]];
        assert_eq!(vec![1, 1], g.placings());
    }
}
