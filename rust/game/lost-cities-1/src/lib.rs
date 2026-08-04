use serde::{Deserialize, Serialize};

pub mod card;
mod command;
mod render;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Stat, Status, placings_log};
use brdgme_markup::Node as N;

use std::collections::HashMap;
use std::default::Default;

use card::{Card, Expedition, Value, expeditions};
use command::Command;
use rand::prelude::*;

const INVESTMENTS: usize = 3;
pub const ROUNDS: usize = 3;
pub const START_ROUND: usize = 1;
const PLAYERS: usize = 2;
const MIN_VALUE: usize = 2;
const MAX_VALUE: usize = 10;
const HAND_SIZE: usize = 8;

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub enum Phase {
    #[default]
    PlayOrDiscard,
    DrawOrTake,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub plays: usize,
    pub discards: usize,
    pub takes: usize,
    pub draws: usize,
    pub turns: usize,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
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
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Current round number, 1 through 3.
    pub round: usize,
    /// True when all 3 rounds are complete and the game is over.
    pub is_finished: bool,
    /// Current turn phase: PlayOrDiscard or DrawOrTake.
    pub phase: Phase,
    /// Number of cards left in the draw pile. When 0, the round ends.
    pub deck_remaining: usize,
    /// Top card value on each expedition's shared discard pile. Only expeditions with discards appear.
    pub discards: HashMap<Expedition, Value>,
    /// Scores indexed by player (0 or 1), then by round. Sum across rounds for cumulative score.
    pub scores: Vec<Vec<isize>>,
    /// Cards played to expeditions, indexed by player (0 or 1). Each card has an expedition and value.
    pub expeditions: Vec<Vec<Card>>,
    /// Index (0 or 1) of the player whose turn it is.
    pub current_player: usize,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player (0 or 1) this private state belongs to.
    pub player: usize,
    /// Cards currently in this player's hand, sorted by expedition then value.
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
    fn start_round(&mut self) -> Result<Vec<Log>, GameError> {
        let mut logs: Vec<Log> = vec![Log::public(vec![N::text(format!(
            "Starting round {}",
            self.round
        ))])];
        // Grab a new deck and shuffle it.
        let mut deck = initial_deck();
        deck.shuffle(&mut self.rng);
        self.deck = deck;
        // Clear out discards, hands and expeditions.
        self.discards = vec![];
        self.hands = vec![];
        self.expeditions = vec![];
        // Initialise player hands and expedition and draw initial cards.
        for p in 0..PLAYERS {
            self.hands.push(vec![]);
            self.expeditions.push(vec![]);
            logs.extend(self.draw_hand_full(p)?);
        }
        if self.round > START_ROUND {
            // Player with the most points starts next, otherwise the next player.
            self.current_player = match self.player_score(0) - self.player_score(1) {
                0 => opponent(self.current_player),
                s if s > 0 => 0,
                _ => 1,
            }
        }
        self.start_turn();
        Ok(logs)
    }

    fn end_round(&mut self) -> Result<Vec<Log>, GameError> {
        self.round += 1;
        let mut logs: Vec<Log> = vec![];
        for p in 0..PLAYERS {
            let mut round_score: isize = 0;
            if let Some(p_exp) = self.expeditions.get(p) {
                round_score = score(p_exp);
            }
            if let Some(s) = self.scores.get_mut(p) {
                s.push(round_score)
            }
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
        let scores: [isize; 2] = [self.player_score(0), self.player_score(1)];
        let winners = self.winners();
        let mut log_text = vec![N::text("The game is over, ")];
        log_text.extend(match winners.as_slice() {
            w if w.len() == 1 => {
                let p = w[0];
                vec![
                    N::Player(p),
                    N::text(format!(
                        " won by {} points",
                        scores.get(p).unwrap_or(&0) - scores.get(opponent(p)).unwrap_or(&0)
                    )),
                ]
            }
            _ => vec![N::text(format!(
                "scores tied at {}",
                scores.first().unwrap_or(&0)
            ))],
        });
        Log::public(vec![N::Bold(log_text)])
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
        self.current_player = (self.current_player + 1) % PLAYERS;
        self.start_turn();
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
        if let Some(index) = self
            .discards
            .iter()
            .rposition(|&c| c.expedition == expedition)
        {
            let c = *self
                .discards
                .get(index)
                .ok_or_else(|| GameError::internal("could not find discard card".to_string()))?;
            self.hands
                .get_mut(player)
                .ok_or_else(|| GameError::internal("could not find player hand".to_string()))?
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
                "there are no discarded cards for that expedition".to_string(),
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
                let index = h
                    .iter()
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
                Value::N(n) => {
                    if n <= hn {
                        return Err(GameError::invalid_input(format!(
                            "you can't play {} as you've already played a higher card",
                            c
                        )));
                    }
                }
            }
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
                let mut num = HAND_SIZE.saturating_sub(hand.len());
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
            None => return Err(GameError::internal("invalid player number".to_string())),
        };
        if self.deck.is_empty() {
            logs.extend(self.end_round()?);
        }
        Ok(logs)
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
        gen_placings(&[
            vec![self.player_score(0) as i32],
            vec![self.player_score(1) as i32],
        ])
    }

    fn winners(&self) -> Vec<usize> {
        self.placings()
            .iter()
            .enumerate()
            .filter_map(|(player, place)| if *place == 1 { Some(player) } else { None })
            .collect()
    }
}

pub fn opponent(player: usize) -> usize {
    (player + 1) % PLAYERS
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != PLAYERS {
            return Err(GameError::PlayerCount {
                min: PLAYERS,
                max: PLAYERS,
                given: players,
            });
        }
        let mut g = Game {
            round: START_ROUND,
            stats: vec![Stats::default(), Stats::default()],
            scores: vec![vec![], vec![]],
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        let logs = g.start_round()?;
        Ok((g, logs))
    }

    fn validate(&self) -> Result<(), GameError> {
        if self.hands.len() != PLAYERS {
            return Err(GameError::internal("lost-cities-1: hands length mismatch"));
        }
        if self.scores.len() != PLAYERS {
            return Err(GameError::internal("lost-cities-1: scores length mismatch"));
        }
        if self.expeditions.len() != PLAYERS {
            return Err(GameError::internal(
                "lost-cities-1: expeditions length mismatch",
            ));
        }
        if self.stats.len() != PLAYERS {
            return Err(GameError::internal("lost-cities-1: stats length mismatch"));
        }
        if self.current_player >= PLAYERS {
            return Err(GameError::internal(
                "lost-cities-1: current_player out of range",
            ));
        }
        Ok(())
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
            // Documented (and DATA_DOCS.md) contract: sorted by expedition
            // then value, which is exactly Card's derived Ord. The hand is
            // fetched defensively so a short `hands` vector renders an empty
            // hand instead of panicking every viewer (F-60); validate()
            // rejects such states at the deserialization boundary.
            hand: {
                let mut hand = self.hands.get(player).cloned().unwrap_or_default();
                hand.sort();
                hand
            },
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
        let was_finished = self.is_finished();
        match output {
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
            }) => self.draw(player).map(|mut l| {
                if !was_finished && self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..PLAYERS)
                        .map(|p| (p, self.player_score(p) as i32))
                        .collect();
                    l.push(placings_log(&self.placings(), Some(&scores)));
                }
                CommandResponse {
                    logs: l,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                }
            }),
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..PLAYERS).map(|p| self.player_score(p) as f32).collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![PLAYERS]
    }

    fn player_count(&self) -> usize {
        PLAYERS
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

pub fn score(cards: &[Card]) -> isize {
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
        let Some(&cards) = exp_cards.get(&e) else {
            return acc;
        };
        acc + (exp_sum.get(&e).unwrap_or(&0) - 20) * (exp_inv.get(&e).unwrap_or(&0) + 1)
            + if cards >= 8 { 20 } else { 0 }
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
        let game = Game::start(2, 1).unwrap().0;
        assert_eq!(game.hands.len(), 2);
        assert_eq!(game.hands[0].len(), 8);
        assert_eq!(game.hands[1].len(), 8);
        assert_eq!(game.deck.len(), 44);
    }

    #[test]
    fn end_round_works() {
        let mut game = Game::start(2, 1).unwrap().0;
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
        let mut game = Game::start(2, 1).unwrap().0;
        for _ in 0..(44 * ROUNDS) {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            game.draw(p).unwrap();
        }
        assert!(game.is_finished());
    }

    #[test]
    fn play_works() {
        let mut game = Game::start(2, 1).unwrap().0;
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
        assert_eq!(0, score(&[]));
        assert_eq!(-17, score(&[(Expedition::Red, Value::N(3)).into()]));
        assert_eq!(
            -34,
            score(&[
                (Expedition::Red, Value::N(3)).into(),
                (Expedition::Green, Value::N(3)).into()
            ])
        );
        assert_eq!(
            -30,
            score(&[
                (Expedition::Red, Value::N(3)).into(),
                (Expedition::Green, Value::N(3)).into(),
                (Expedition::Green, Value::N(4)).into()
            ])
        );
        assert_eq!(
            -37,
            score(&[
                (Expedition::Green, Value::Investment).into(),
                (Expedition::Red, Value::N(3)).into(),
                (Expedition::Green, Value::N(4)).into(),
                (Expedition::Green, Value::N(6)).into()
            ])
        );
        assert_eq!(
            44,
            score(&[
                (Expedition::Green, Value::N(2)).into(),
                (Expedition::Green, Value::N(3)).into(),
                (Expedition::Green, Value::N(4)).into(),
                (Expedition::Green, Value::N(5)).into(),
                (Expedition::Green, Value::N(6)).into(),
                (Expedition::Green, Value::N(7)).into(),
                (Expedition::Green, Value::N(8)).into(),
                (Expedition::Green, Value::N(9)).into()
            ])
        );
    }

    #[test]
    fn placings_works() {
        let mut g = Game::start(2, 1).expect("expected to create game").0;
        g.scores = vec![vec![200, 0, 0], vec![100, 50, 40]];
        assert_eq!(vec![1, 2], g.placings());
        g.scores = vec![vec![100, 50, 40], vec![200, 0, 0]];
        assert_eq!(vec![2, 1], g.placings());
        g.scores = vec![vec![100, 50, 40], vec![100, 50, 40]];
        assert_eq!(vec![1, 1], g.placings());
    }

    #[test]
    fn final_draw_of_a_round_keeps_its_logs() {
        // e F37: draw_hand_full dropped its accumulated logs when the draw
        // emptied the deck, so the last draw of every round was invisible.
        // Pre-fix the returned logs start with the round-score log instead of
        // the draw log.
        let mut game = Game::start(2, 1).unwrap().0;
        let mut logs: Vec<Log> = vec![];
        for _ in 0..44 {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            logs = game.draw(p).unwrap();
        }
        assert_eq!(
            START_ROUND + 1,
            game.round,
            "the 44th draw must end the round"
        );
        let text: Vec<String> = logs
            .iter()
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect();
        assert!(
            text[0].contains("drew a card"),
            "the final draw's public log must come first, got: {:?}",
            text
        );
        assert!(
            logs.iter().any(|l| !l.public),
            "the final draw's private log must be present, got: {:?}",
            text
        );
    }

    #[test]
    fn player_state_hand_is_sorted_as_documented() {
        // e F38 / e F20: PlayerState.hand's rustdoc and DATA_DOCS.md both
        // promise "sorted by expedition then value"; player_state() returned
        // acquisition order. Card's derived Ord is (expedition, value) with
        // Red < Green < White < Blue < Yellow and Investment < N(_).
        let mut game = Game::start(2, 1).unwrap().0;
        game.hands[0] = vec![
            (Expedition::Yellow, Value::N(9)).into(),
            (Expedition::Red, Value::Investment).into(),
            (Expedition::Green, Value::N(2)).into(),
            (Expedition::Red, Value::N(4)).into(),
        ];
        let hand = game.player_state(0).hand;
        let mut expected = hand.clone();
        expected.sort();
        assert_eq!(expected, hand, "hand must be sorted");
        assert_eq!(
            vec!["RX", "R4", "G2", "Y9"],
            hand.iter().map(|c| c.to_string()).collect::<Vec<String>>()
        );
    }

    #[test]
    fn draw_hand_full_does_not_underflow_on_an_oversized_hand() {
        // e F41 / e F26: `HAND_SIZE - hand.len()` panics in debug builds if a
        // hand ever exceeds the hand size. Unreachable in normal play, so this
        // constructs the state directly.
        let mut game = Game::start(2, 1).unwrap().0;
        let extra = game.deck.pop().expect("deck must not be empty");
        game.hands[0].push(extra);
        let over = game.hands[0].len();
        let logs = game
            .draw_hand_full(0)
            .expect("drawing into an over-full hand must not error");
        assert_eq!(
            over,
            game.hands[0].len(),
            "no cards may be drawn into an over-full hand"
        );
        assert!(!logs.is_empty(), "the draw attempt must still be logged");
    }

    #[test]
    fn validate_works() {
        assert!(Game::start(2, 1).unwrap().0.validate().is_ok());

        let mut game = Game::start(2, 1).unwrap().0;
        game.hands.pop();
        assert!(matches!(game.validate(), Err(GameError::Internal { .. })));

        let mut game = Game::start(2, 1).unwrap().0;
        game.scores.pop();
        assert!(matches!(game.validate(), Err(GameError::Internal { .. })));

        let mut game = Game::start(2, 1).unwrap().0;
        game.expeditions.pop();
        assert!(matches!(game.validate(), Err(GameError::Internal { .. })));

        let mut game = Game::start(2, 1).unwrap().0;
        game.stats.pop();
        assert!(matches!(game.validate(), Err(GameError::Internal { .. })));

        let mut game = Game::start(2, 1).unwrap().0;
        game.current_player = PLAYERS;
        assert!(matches!(game.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn player_state_does_not_panic_on_short_hands() {
        // F-60: a persisted Game with `hands` shorter than the player index
        // (e.g. empty, which serde accepts) panicked player_state() for every
        // viewer because the render path indexed `self.hands[player]` raw.
        let mut game = Game::start(2, 1).unwrap().0;
        game.hands = vec![];
        assert!(game.player_state(0).hand.is_empty());
        assert!(game.player_state(1).hand.is_empty());
    }

    // --- R-32 (F-18): !was_finished epilogue gate ---

    fn is_placings_log(l: &Log) -> bool {
        l.content.contains(&N::text(" Final scores: "))
    }

    #[test]
    fn finish_path_twice_emits_one_placings_epilogue() {
        let names = vec!["a".to_string(), "b".to_string()];

        // A non-finishing play emits no placings log and keeps can_undo.
        let (mut game, _) = Game::start(2, 1).unwrap();
        let c = game.hands[0][0];
        let resp = game.command(0, &format!("play {}", c), &names).unwrap();
        assert!(resp.can_undo, "Play arm can_undo must be unchanged");
        assert!(
            !resp.logs.iter().any(is_placings_log),
            "a non-finishing play emits no placings log"
        );

        // Wind to the final round with an 8-card deck, so the next draw
        // empties it and ends the game.
        let (mut game, _) = Game::start(2, 1).unwrap();
        game.round = START_ROUND + ROUNDS - 1;
        game.phase = Phase::DrawOrTake;
        game.current_player = 0;
        game.hands = vec![vec![], vec![]];
        game.discards = vec![];
        game.deck = game.deck[..HAND_SIZE].to_vec();
        let resp = game.command(0, "draw", &names).unwrap();
        assert!(game.is_finished());
        assert!(!resp.can_undo, "Draw arm can_undo must be unchanged");
        assert_eq!(
            1,
            resp.logs.iter().filter(|l| is_placings_log(l)).count(),
            "exactly one placings epilogue"
        );
        assert!(
            is_placings_log(resp.logs.last().unwrap()),
            "placings log is last"
        );
        assert_eq!(vec![1, 1], game.placings());
        match game.status() {
            Status::Finished { placings, .. } => assert_eq!(vec![1, 1], placings),
            _ => panic!("game should be finished"),
        }

        // The finished parser rejects a second invocation, so no duplicate
        // epilogue is appended.
        assert!(game.command(0, "draw", &names).is_err());
    }

    // --- 5.5: serialization redaction + R-LOG Log::public coverage ---

    fn assert_no_internal_fields(json: &str) {
        // Game-internal state that must never reach clients. Keys are matched
        // with quotes so PubState's legitimate "deck_remaining" field name
        // cannot trip the "deck" check.
        for field in ["hands", "deck", "rng", "stats", "discarded_expedition"] {
            assert!(
                !json.contains(&format!("\"{field}\"")),
                "internal field {field} leaked into serialized state: {json}"
            );
        }
    }

    #[test]
    fn serialized_public_and_player_states_redact_private_hands() {
        // e 5.5 redaction: the states shipped to clients are PubState and
        // PlayerState. Their serde output must carry the public tableau but
        // never another player's hand, and must not smuggle Game-internal
        // fields (hands, deck, rng, stats). Hands are set so every card is
        // distinguishable from the public tableau by expedition and value.
        let mut game = Game::start(2, 1).unwrap().0;
        game.hands = vec![
            vec![(Expedition::Red, Value::N(2)).into()],
            vec![(Expedition::Green, Value::N(3)).into()],
        ];
        game.discards = vec![(Expedition::Blue, Value::N(4)).into()];
        game.expeditions = vec![vec![(Expedition::White, Value::N(5)).into()], vec![]];
        game.deck = vec![];

        // Public state carries the tableau but neither hand.
        let public = serde_json::to_string(&game.pub_state()).unwrap();
        for token in ["White", "Blue", "5", "4"] {
            assert!(
                public.contains(token),
                "public token {token} missing from pub state: {public}"
            );
        }
        for token in ["Red", "Green", "2", "3"] {
            assert!(
                !public.contains(token),
                "private hand token {token} leaked into pub state: {public}"
            );
        }
        assert_no_internal_fields(&public);

        // Player 0's state serializes their own hand only.
        let p0 = serde_json::to_string(&game.player_state(0)).unwrap();
        assert!(
            p0.contains("Red"),
            "player 0's own hand must serialize: {p0}"
        );
        assert!(
            !p0.contains("Green"),
            "player 1's hand leaked into player 0's state: {p0}"
        );
        assert_no_internal_fields(&p0);

        // Player 1's state serializes their own hand only.
        let p1 = serde_json::to_string(&game.player_state(1)).unwrap();
        assert!(
            p1.contains("Green"),
            "player 1's own hand must serialize: {p1}"
        );
        assert!(
            !p1.contains("Red"),
            "player 0's hand leaked into player 1's state: {p1}"
        );
        assert_no_internal_fields(&p1);
    }

    #[test]
    fn start_game_public_logs_do_not_expose_drawn_hand_cards() {
        // e R-LOG / 5.6: draw_hand_full logs drawn cards to the drawing player
        // privately; the public logs may announce only a count. Asserting on
        // pub_state fields would not catch a card identity leaking into the
        // rendered Log::public content, so this walks the real start path.
        let (game, logs) = Game::start(2, 1).unwrap();
        let drawn: Vec<String> = (0..PLAYERS)
            .flat_map(|p| game.hands[p].iter().map(|c| c.to_string()))
            .collect();
        assert!(!drawn.is_empty(), "both players must have drawn hands");
        let public: Vec<String> = logs
            .iter()
            .filter(|l| l.public)
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect();
        assert!(
            public.iter().any(|t| t.contains("drew")),
            "a public draw announcement must exist, got: {:?}",
            public
        );
        for text in &public {
            for code in &drawn {
                assert!(
                    !text.contains(code.as_str()),
                    "public log must not expose drawn card {}, got: {}",
                    code,
                    text
                );
            }
        }
        // Each player gets exactly one private draw log naming their cards.
        for p in 0..PLAYERS {
            let detail: Vec<String> = logs
                .iter()
                .filter(|l| {
                    !l.public
                        && l.to == vec![p]
                        && brdgme_markup::to_string(&l.content).contains("drew")
                })
                .map(|l| brdgme_markup::to_string(&l.content))
                .collect();
            assert_eq!(
                1,
                detail.len(),
                "each player must get exactly one private draw log, got: {:?}",
                detail
            );
            let expected: Vec<String> = game.hands[p].iter().map(|c| c.to_string()).collect();
            assert!(
                expected
                    .iter()
                    .all(|code| detail[0].contains(code.as_str())),
                "the private draw log must name every drawn card, got: {}",
                detail[0]
            );
        }
    }
}
