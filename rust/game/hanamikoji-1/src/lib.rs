use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod card;
mod command;
mod render;

use brdgme_color::NamedColor;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Stat, Status, placings_log};
use brdgme_markup::Node as N;
use rand::prelude::*;
use rand::seq::SliceRandom;

use crate::card::Geisha;
use crate::command::Command;

const GEISHA: usize = 7;
const WIN_GEISHA: usize = 4;
const WIN_CHARM: i32 = 11;

#[derive(Default, PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    ChooseAction,
    OpponentChoose,
    Finished,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Pending {
    Gift {
        actor: usize,
        cards: Vec<Geisha>,
    },
    Competition {
        actor: usize,
        sets: [Vec<Geisha>; 2],
    },
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    players: usize,
    marker: Vec<Option<usize>>,
    faceup: Vec<[u32; 2]>,
    hands: Vec<Vec<Geisha>>,
    secret: Vec<Option<Geisha>>,
    traded: Vec<Vec<Geisha>>,
    used: Vec<[bool; 4]>,
    deck: Vec<Geisha>,
    round: u32,
    starting: usize,
    current: usize,
    phase: Phase,
    pending: Option<Pending>,
    winner: Option<usize>,
    #[serde(default = "GameRng::from_entropy")]
    rng: GameRng,
}

/// Spectator view. Structurally omits hidden info: the deck order, hand
/// contents, secret contents and trade-off contents are simply not present -
/// only counts and public consequences (face-up cards, victory markers and the
/// face-up pending choice) are exposed.
#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players (always 2).
    pub players: usize,
    /// The current round number.
    pub round: u32,
    /// True once a winner has been decided.
    pub finished: bool,
    /// Which step of the turn the game is in.
    pub phase: Phase,
    /// The actor of the current real turn.
    pub current: usize,
    /// The player(s) expected to act next.
    pub whose_turn: Vec<usize>,
    /// The player who starts this round.
    pub starting: usize,
    /// Cards left in the draw pile (order hidden).
    pub deck_remaining: usize,
    /// Victory marker position per geisha: None = contested, Some(p) = controlled by p.
    pub marker: Vec<Option<usize>>,
    /// Face-up card counts per geisha per player.
    pub faceup: Vec<[u32; 2]>,
    /// Which of the four action markers each player has used this round.
    pub used: Vec<[bool; 4]>,
    /// Number of cards in each player's hand (contents hidden).
    pub hand_counts: Vec<usize>,
    /// Whether each player has a face-down secret card (identity hidden).
    pub has_secret: Vec<bool>,
    /// Number of cards each player has set aside for trade-off (identities hidden).
    pub traded_counts: Vec<usize>,
    /// The face-up pending choice, if any (gift cards or competition sets).
    pub pending: Option<Pending>,
    /// Number of geisha each player currently controls.
    pub geisha_counts: Vec<usize>,
    /// Charm points each player currently controls.
    pub charms: Vec<i32>,
    /// The winner, if the game is over.
    pub winner: Option<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// This player's hand.
    pub hand: Vec<Geisha>,
    /// This player's face-down secret card, if any.
    pub secret: Option<Geisha>,
    /// This player's face-down trade-off discard.
    pub traded: Vec<Geisha>,
}

impl Game {
    fn sort_hand(&mut self, player: usize) {
        if let Some(hand) = self.hands.get_mut(player) {
            hand.sort_by_key(|g| g.index());
        }
    }

    fn can_remove(&self, player: usize, cards: &[Geisha]) -> bool {
        let hand = match self.hands.get(player) {
            Some(hand) => hand,
            None => return false,
        };
        let mut need = [0i32; GEISHA];
        for c in cards {
            need[c.index()] += 1;
        }
        let mut have = [0i32; GEISHA];
        for c in hand {
            have[c.index()] += 1;
        }
        (0..GEISHA).all(|i| have[i] >= need[i])
    }

    fn remove_from_hand(&mut self, player: usize, geisha: Geisha) -> bool {
        let Some(hand) = self.hands.get_mut(player) else {
            return false;
        };
        let Some(pos) = hand.iter().position(|c| *c == geisha) else {
            return false;
        };
        hand.remove(pos);
        true
    }

    fn remove_cards(&mut self, player: usize, cards: &[Geisha]) {
        for c in cards {
            self.remove_from_hand(player, *c);
        }
    }

    fn geisha_counts(&self) -> [usize; 2] {
        let mut counts = [0usize; 2];
        for m in &self.marker {
            if let Some(p) = m
                && *p < 2
            {
                counts[*p] += 1;
            }
        }
        counts
    }

    fn charms(&self) -> [i32; 2] {
        let mut counts = [0i32; 2];
        for (i, m) in self.marker.iter().enumerate() {
            if let Some(p) = m
                && *p < 2
                && let Some(g) = Geisha::ALL.get(i)
            {
                counts[*p] += g.charm();
            }
        }
        counts
    }

    fn whose_turn_calc(&self) -> Vec<usize> {
        match self.phase {
            Phase::ChooseAction => vec![self.current],
            Phase::OpponentChoose => vec![1 - self.current],
            Phase::Finished => vec![],
        }
    }

    fn deal_round(&mut self) {
        let mut pool = Geisha::full_deck();
        pool.shuffle(&mut self.rng);
        pool.pop();
        self.hands = vec![vec![]; self.players];
        for _ in 0..6 {
            if let Some(c) = pool.pop() {
                self.hands[0].push(c);
            }
        }
        for _ in 0..6 {
            if let Some(c) = pool.pop() {
                self.hands[1].push(c);
            }
        }
        for p in 0..self.players {
            self.sort_hand(p);
        }
        self.deck = pool;
        self.secret = vec![None; self.players];
        self.traded = vec![vec![]; self.players];
    }

    fn begin_turn(&mut self, player: usize) -> Vec<Log> {
        let mut logs = vec![];
        if let Some(card) = self.deck.pop() {
            if let Some(hand) = self.hands.get_mut(player) {
                hand.push(card);
            }
            self.sort_hand(player);
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" drew a card, "),
                N::Bold(vec![N::text(self.deck.len().to_string())]),
                N::text(" remaining in the deck"),
            ]));
            logs.push(Log::private(
                vec![N::text("You drew "), render::geisha_node(card)],
                vec![player],
            ));
        }
        self.current = player;
        self.phase = Phase::ChooseAction;
        logs.push(Log::public(vec![
            N::text("It is "),
            N::Player(player),
            N::text("'s turn"),
        ]));
        logs
    }

    fn advance_turn(&mut self) -> Vec<Log> {
        let all_used = (0..self.players).all(|p| {
            self.used
                .get(p)
                .copied()
                .unwrap_or([false; 4])
                .iter()
                .all(|u| *u)
        });
        if all_used {
            self.score_round()
        } else {
            self.begin_turn(1 - self.current)
        }
    }

    fn decide_winner(geisha: &[usize; 2], charm: &[i32; 2]) -> Option<usize> {
        let reaches = |p: usize| geisha[p] >= WIN_GEISHA || charm[p] >= WIN_CHARM;
        match (reaches(0), reaches(1)) {
            (false, false) => None,
            (true, false) => Some(0),
            (false, true) => Some(1),
            // Both reached a goal: the rulebook awards it to the player who
            // reached 11+ charm. Only one can (charm sums to 21), so this is
            // unambiguous; the fallback is defensive only.
            (true, true) => Some(if charm[0] >= WIN_CHARM { 0 } else { 1 }),
        }
    }

    fn score_round(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        for p in 0..self.players {
            if let Some(g) = self.secret.get(p).copied().flatten() {
                if let Some(f) = self.faceup.get_mut(g.index()) {
                    f[p] += 1;
                }
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(" revealed a secret "),
                    render::geisha_node(g),
                ]));
            }
        }
        for i in 0..self.marker.len() {
            let f = self.faceup.get(i).copied().unwrap_or([0, 0]);
            if f[0] > f[1] {
                self.marker[i] = Some(0);
            } else if f[1] > f[0] {
                self.marker[i] = Some(1);
            }
        }
        self.secret = vec![None; self.players];

        let geisha = self.geisha_counts();
        let charm = self.charms();

        let mut summary: Vec<N> = vec![N::Bold(vec![N::text(format!(
            "Round {} scored",
            self.round
        ))])];
        for g in Geisha::ALL {
            summary.push(N::text("\n  "));
            summary.push(render::geisha_node(g));
            summary.push(N::text(format!(" ({}): ", g.charm())));
            match self.marker.get(g.index()).copied().unwrap_or(None) {
                Some(p) => summary.push(N::Player(p)),
                None => summary.push(N::Fg(NamedColor::Grey.into(), vec![N::text("contested")])),
            }
        }
        summary.push(N::text("\n"));
        summary.push(N::Player(0));
        summary.push(N::text(format!(
            " controls {} geisha ({} charm), ",
            geisha[0], charm[0]
        )));
        summary.push(N::Player(1));
        summary.push(N::text(format!(
            " controls {} geisha ({} charm)",
            geisha[1], charm[1]
        )));
        logs.push(Log::public(summary));

        if let Some(w) = Self::decide_winner(&geisha, &charm) {
            self.winner = Some(w);
            self.phase = Phase::Finished;
            logs.push(Log::public(vec![
                N::Bold(vec![N::text("Game over - ")]),
                N::Player(w),
                N::Bold(vec![N::text(" wins!")]),
            ]));
        } else {
            self.used = vec![[false; 4]; self.players];
            self.faceup = vec![[0, 0]; self.marker.len()];
            self.starting = 1 - self.starting;
            self.round += 1;
            self.deal_round();
            logs.push(Log::public(vec![
                N::Bold(vec![N::text(format!("Round {} begins", self.round))]),
                N::text(" - "),
                N::Player(self.starting),
                N::text(" goes first"),
            ]));
            logs.extend(self.begin_turn(self.starting));
        }
        logs
    }

    fn assert_action(&self, player: usize) -> Result<(), GameError> {
        self.assert_not_finished()?;
        if self.phase != Phase::ChooseAction {
            return Err(GameError::invalid_input(
                "you must wait for the pending choice to be resolved",
            ));
        }
        if self.current != player {
            return Err(GameError::NotYourTurn);
        }
        Ok(())
    }

    fn already_used(&self, player: usize, action: usize) -> bool {
        self.used.get(player).map(|u| u[action]).unwrap_or(true)
    }

    fn secret(&mut self, player: usize, geisha: Geisha) -> Result<Vec<Log>, GameError> {
        self.assert_action(player)?;
        if self.already_used(player, 0) {
            return Err(GameError::invalid_input(
                "you have already used your secret action this round",
            ));
        }
        if !self.can_remove(player, &[geisha]) {
            return Err(GameError::invalid_input(
                "you do not have that card in your hand",
            ));
        }
        self.remove_cards(player, &[geisha]);
        if let Some(s) = self.secret.get_mut(player) {
            *s = Some(geisha);
        }
        if let Some(u) = self.used.get_mut(player) {
            u[0] = true;
        }
        let mut logs = vec![
            Log::public(vec![
                N::Player(player),
                N::text(" played a "),
                N::Bold(vec![N::text("secret")]),
                N::text(" card"),
            ]),
            Log::private(
                vec![
                    N::text("You played "),
                    render::geisha_node(geisha),
                    N::text(" as your secret card"),
                ],
                vec![player],
            ),
        ];
        logs.extend(self.advance_turn());
        Ok(logs)
    }

    fn trade(&mut self, player: usize, a: Geisha, b: Geisha) -> Result<Vec<Log>, GameError> {
        self.assert_action(player)?;
        if self.already_used(player, 1) {
            return Err(GameError::invalid_input(
                "you have already used your trade-off action this round",
            ));
        }
        if !self.can_remove(player, &[a, b]) {
            return Err(GameError::invalid_input(
                "you do not have those cards in your hand",
            ));
        }
        self.remove_cards(player, &[a, b]);
        if let Some(t) = self.traded.get_mut(player) {
            t.push(a);
            t.push(b);
        }
        if let Some(u) = self.used.get_mut(player) {
            u[1] = true;
        }
        let mut logs = vec![
            Log::public(vec![
                N::Player(player),
                N::text(" set aside two cards for a "),
                N::Bold(vec![N::text("trade-off")]),
            ]),
            Log::private(
                vec![
                    N::text("You set aside "),
                    render::geisha_node(a),
                    N::text(" and "),
                    render::geisha_node(b),
                ],
                vec![player],
            ),
        ];
        logs.extend(self.advance_turn());
        Ok(logs)
    }

    fn gift(&mut self, player: usize, cards: [Geisha; 3]) -> Result<Vec<Log>, GameError> {
        self.assert_action(player)?;
        if self.already_used(player, 2) {
            return Err(GameError::invalid_input(
                "you have already used your gift action this round",
            ));
        }
        if !self.can_remove(player, &cards) {
            return Err(GameError::invalid_input(
                "you do not have those cards in your hand",
            ));
        }
        self.remove_cards(player, &cards);
        self.pending = Some(Pending::Gift {
            actor: player,
            cards: cards.to_vec(),
        });
        self.phase = Phase::OpponentChoose;
        let mut line = vec![
            N::Player(player),
            N::text(" played a "),
            N::Bold(vec![N::text("gift")]),
            N::text(" offering "),
        ];
        line.extend(render::comma_geisha(&cards));
        line.push(N::text("; "));
        line.push(N::Player(1 - player));
        line.push(N::text(" chooses one"));
        Ok(vec![Log::public(line)])
    }

    fn compete(&mut self, player: usize, cards: [Geisha; 4]) -> Result<Vec<Log>, GameError> {
        self.assert_action(player)?;
        if self.already_used(player, 3) {
            return Err(GameError::invalid_input(
                "you have already used your competition action this round",
            ));
        }
        if !self.can_remove(player, &cards) {
            return Err(GameError::invalid_input(
                "you do not have those cards in your hand",
            ));
        }
        self.remove_cards(player, &cards);
        let sets = [vec![cards[0], cards[1]], vec![cards[2], cards[3]]];
        self.pending = Some(Pending::Competition {
            actor: player,
            sets: sets.clone(),
        });
        self.phase = Phase::OpponentChoose;
        let mut line = vec![
            N::Player(player),
            N::text(" played a "),
            N::Bold(vec![N::text("competition")]),
            N::text(" offering set 1 { "),
        ];
        line.extend(render::comma_geisha(&sets[0]));
        line.push(N::text(" } and set 2 { "));
        line.extend(render::comma_geisha(&sets[1]));
        line.push(N::text(" }; "));
        line.push(N::Player(1 - player));
        line.push(N::text(" chooses a set"));
        Ok(vec![Log::public(line)])
    }

    fn choose_gift(&mut self, player: usize, geisha: Geisha) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if self.phase != Phase::OpponentChoose {
            return Err(GameError::invalid_input(
                "there is no pending choice to make",
            ));
        }
        let (actor, cards) = match self.pending.clone() {
            Some(Pending::Gift { actor, cards }) => (actor, cards),
            _ => {
                return Err(GameError::invalid_input("there is no gift to choose from"));
            }
        };
        if player != 1 - actor {
            return Err(GameError::NotYourTurn);
        }
        if !cards.contains(&geisha) {
            return Err(GameError::invalid_input(
                "that card is not part of the gift",
            ));
        }
        let mut others = cards;
        if let Some(pos) = others.iter().position(|c| *c == geisha) {
            others.remove(pos);
        }
        if let Some(f) = self.faceup.get_mut(geisha.index()) {
            f[player] += 1;
        }
        for c in &others {
            if let Some(f) = self.faceup.get_mut(c.index()) {
                f[actor] += 1;
            }
        }
        if let Some(u) = self.used.get_mut(actor) {
            u[2] = true;
        }
        self.pending = None;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            render::geisha_node(geisha),
            N::text(" from the gift; "),
            N::Player(actor),
            N::text(" placed the rest"),
        ])];
        logs.extend(self.advance_turn());
        Ok(logs)
    }

    fn choose_competition(&mut self, player: usize, set_idx: usize) -> Result<Vec<Log>, GameError> {
        self.assert_not_finished()?;
        if self.phase != Phase::OpponentChoose {
            return Err(GameError::invalid_input(
                "there is no pending choice to make",
            ));
        }
        let (actor, sets) = match self.pending.clone() {
            Some(Pending::Competition { actor, sets }) => (actor, sets),
            _ => {
                return Err(GameError::invalid_input(
                    "there is no competition to choose from",
                ));
            }
        };
        if player != 1 - actor {
            return Err(GameError::NotYourTurn);
        }
        if set_idx >= 2 {
            return Err(GameError::invalid_input("choose set 1 or set 2"));
        }
        for c in &sets[set_idx] {
            if let Some(f) = self.faceup.get_mut(c.index()) {
                f[player] += 1;
            }
        }
        for c in &sets[1 - set_idx] {
            if let Some(f) = self.faceup.get_mut(c.index()) {
                f[actor] += 1;
            }
        }
        if let Some(u) = self.used.get_mut(actor) {
            u[3] = true;
        }
        self.pending = None;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took set "),
            N::Bold(vec![N::text((set_idx + 1).to_string())]),
            N::text(" from the competition; "),
            N::Player(actor),
            N::text(" placed the other"),
        ])];
        logs.extend(self.advance_turn());
        Ok(logs)
    }

    fn placings(&self) -> Vec<usize> {
        let mut metrics = vec![vec![0i32]; self.players];
        if let Some(w) = self.winner
            && w < self.players
        {
            metrics[w] = vec![1];
        }
        gen_placings(&metrics)
    }

    fn finished_stats(&self) -> Vec<HashMap<String, Stat>> {
        let geisha = self.geisha_counts();
        let charm = self.charms();
        (0..self.players)
            .map(|p| {
                let mut m = HashMap::new();
                m.insert("geisha".to_string(), Stat::Int(geisha[p] as i32));
                m.insert("charm".to_string(), Stat::Int(charm[p]));
                m
            })
            .collect()
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != 2 {
            return Err(GameError::PlayerCount {
                min: 2,
                max: 2,
                given: players,
            });
        }
        let mut g = Game {
            players: 2,
            marker: vec![None; GEISHA],
            faceup: vec![[0, 0]; GEISHA],
            hands: vec![vec![]; 2],
            secret: vec![None; 2],
            traded: vec![vec![]; 2],
            used: vec![[false; 4]; 2],
            round: 1,
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        g.starting = g.rng.random_range(0..2);
        let mut logs = vec![Log::public(vec![
            N::Bold(vec![N::text("A new game of Hanamikoji started")]),
            N::text(" - "),
            N::Player(g.starting),
            N::text(" goes first"),
        ])];
        g.deal_round();
        logs.extend(g.begin_turn(g.starting));
        Ok((g, logs))
    }

    fn validate(&self) -> Result<(), GameError> {
        if self.players != 2 {
            return Err(GameError::internal("hanamikoji-1: players must be 2"));
        }
        if self.hands.len() != 2 {
            return Err(GameError::internal("hanamikoji-1: hands length mismatch"));
        }
        if self.secret.len() != 2 {
            return Err(GameError::internal("hanamikoji-1: secret length mismatch"));
        }
        if self.traded.len() != 2 {
            return Err(GameError::internal("hanamikoji-1: traded length mismatch"));
        }
        if self.used.len() != 2 {
            return Err(GameError::internal("hanamikoji-1: used length mismatch"));
        }
        if self.marker.len() != GEISHA {
            return Err(GameError::internal("hanamikoji-1: marker length mismatch"));
        }
        if self.faceup.len() != GEISHA {
            return Err(GameError::internal("hanamikoji-1: faceup length mismatch"));
        }
        for m in &self.marker {
            if let Some(p) = m
                && *p >= 2
            {
                return Err(GameError::internal(
                    "hanamikoji-1: marker owner out of range",
                ));
            }
        }
        if self.current >= 2 {
            return Err(GameError::internal("hanamikoji-1: current out of range"));
        }
        if self.starting >= 2 {
            return Err(GameError::internal("hanamikoji-1: starting out of range"));
        }
        if self.winner.is_some_and(|w| w >= 2) {
            return Err(GameError::internal("hanamikoji-1: winner out of range"));
        }
        if let Some(pending) = &self.pending {
            match pending {
                Pending::Gift { actor, cards } => {
                    if *actor >= 2 || cards.is_empty() {
                        return Err(GameError::internal("hanamikoji-1: inconsistent gift"));
                    }
                }
                Pending::Competition { actor, sets } => {
                    if *actor >= 2 || sets[0].is_empty() || sets[1].is_empty() {
                        return Err(GameError::internal(
                            "hanamikoji-1: inconsistent competition",
                        ));
                    }
                }
            }
        }
        match self.phase {
            Phase::OpponentChoose => {
                let Some(pending) = &self.pending else {
                    return Err(GameError::internal(
                        "hanamikoji-1: opponent choice phase requires a pending choice",
                    ));
                };
                let actor = match pending {
                    Pending::Gift { actor, .. } | Pending::Competition { actor, .. } => *actor,
                };
                if actor != self.current {
                    return Err(GameError::internal(
                        "hanamikoji-1: pending choice actor does not match current",
                    ));
                }
            }
            Phase::ChooseAction | Phase::Finished => {
                if self.pending.is_some() {
                    return Err(GameError::internal(
                        "hanamikoji-1: pending choice outside opponent choice phase",
                    ));
                }
            }
        }
        Ok(())
    }

    fn status(&self) -> Status {
        if self.winner.is_some() {
            Status::Finished {
                placings: self.placings(),
                stats: self.finished_stats(),
            }
        } else {
            Status::Active {
                whose_turn: self.whose_turn_calc(),
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        let geisha = self.geisha_counts();
        let charm = self.charms();
        PubState {
            players: self.players,
            round: self.round,
            finished: self.winner.is_some(),
            phase: self.phase,
            current: self.current,
            whose_turn: self.whose_turn_calc(),
            starting: self.starting,
            deck_remaining: self.deck.len(),
            marker: self.marker.clone(),
            faceup: self.faceup.clone(),
            used: self.used.clone(),
            hand_counts: self.hands.iter().map(|h| h.len()).collect(),
            has_secret: self.secret.iter().map(|s| s.is_some()).collect(),
            traded_counts: self.traded.iter().map(|t| t.len()).collect(),
            pending: self.pending.clone(),
            geisha_counts: geisha.to_vec(),
            charms: charm.to_vec(),
            winner: self.winner,
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: self.hands.get(player).cloned().unwrap_or_default(),
            secret: self.secret.get(player).copied().flatten(),
            traded: self.traded.get(player).cloned().unwrap_or_default(),
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
        let was_finished = self.is_finished();
        let (mut logs, remaining_input) = match output {
            Ok(ParseOutput {
                value: Command::Secret(g),
                remaining,
                ..
            }) => (self.secret(player, g)?, remaining.to_string()),
            Ok(ParseOutput {
                value: Command::Trade(a, b),
                remaining,
                ..
            }) => (self.trade(player, a, b)?, remaining.to_string()),
            Ok(ParseOutput {
                value: Command::Gift(a, b, c),
                remaining,
                ..
            }) => (self.gift(player, [a, b, c])?, remaining.to_string()),
            Ok(ParseOutput {
                value: Command::Compete(a, b, c, d),
                remaining,
                ..
            }) => (self.compete(player, [a, b, c, d])?, remaining.to_string()),
            Ok(ParseOutput {
                value: Command::ChooseCard(g),
                remaining,
                ..
            }) => (self.choose_gift(player, g)?, remaining.to_string()),
            Ok(ParseOutput {
                value: Command::ChooseSet(i),
                remaining,
                ..
            }) => (self.choose_competition(player, i)?, remaining.to_string()),
            Err(e) => return Err(GameError::invalid_input(e.to_string())),
        };
        if !was_finished && self.is_finished() {
            let charm = self.charms();
            let scores: Vec<(usize, i32)> = (0..self.players).map(|p| (p, charm[p])).collect();
            logs.push(placings_log(&self.placings(), Some(&scores)));
        }
        Ok(CommandResponse {
            logs,
            can_undo: false,
            remaining_input,
        })
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        match self.winner {
            Some(w) => (0..self.players)
                .map(|p| if p == w { 1.0 } else { 0.0 })
                .collect(),
            None => vec![0.0; self.players],
        }
    }

    fn player_counts() -> Vec<usize> {
        vec![2]
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
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec!["player0".to_string(), "player1".to_string()]
    }

    #[test]
    fn test_full_deck() {
        let deck = Geisha::full_deck();
        assert_eq!(21, deck.len());
        for g in Geisha::ALL {
            assert_eq!(3, deck.iter().filter(|c| **c == g).count());
        }
    }

    #[test]
    fn test_charm_sum() {
        let total: i32 = Geisha::ALL.iter().map(|g| g.charm()).sum();
        assert_eq!(21, total);
    }

    #[test]
    fn test_start_wrong_player_count() {
        assert!(matches!(
            Game::start(3, 1),
            Err(GameError::PlayerCount { .. })
        ));
        assert!(Game::start(2, 1).is_ok());
    }

    #[test]
    fn test_start_deals() {
        let (g, _) = Game::start(2, 1).unwrap();
        assert_eq!(1, g.round);
        assert_eq!(7, g.deck.len());
        assert_eq!(7, g.hands[g.starting].len());
        assert_eq!(6, g.hands[1 - g.starting].len());
        assert!(g.starting < 2);
        assert_eq!(g.starting, g.current);
    }

    #[test]
    fn test_start_is_deterministic() {
        let (a, _) = Game::start(2, 7).unwrap();
        let (b, _) = Game::start(2, 7).unwrap();
        assert_eq!(a.hands, b.hands);
        assert_eq!(a.deck, b.deck);
        assert_eq!(a.starting, b.starting);
    }

    #[test]
    fn test_whose_turn() {
        let (g, _) = Game::start(2, 1).unwrap();
        assert_eq!(vec![g.current], g.whose_turn());
    }

    #[test]
    fn test_secret_action() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        let card = g.hands[cur][0];
        let before = g.hands[cur].len();
        g.secret(cur, card).unwrap();
        assert_eq!(Some(card), g.secret[cur]);
        assert!(g.used[cur][0]);
        assert_eq!(before - 1, g.hands[cur].len());
        assert_ne!(cur, g.current);
    }

    #[test]
    fn test_trade_action() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        g.hands[cur] = vec![Geisha::Flute, Geisha::Koto, Geisha::Fan];
        g.trade(cur, Geisha::Flute, Geisha::Koto).unwrap();
        assert!(g.used[cur][1]);
        assert_eq!(vec![Geisha::Flute, Geisha::Koto], g.traded[cur]);
        assert_eq!(vec![Geisha::Fan], g.hands[cur]);
    }

    #[test]
    fn test_cannot_reuse_action() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        g.hands[cur] = vec![Geisha::Flute, Geisha::Koto];
        g.used[cur][0] = true;
        assert!(g.secret(cur, Geisha::Flute).is_err());
    }

    #[test]
    fn test_gift_command_flow() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let actor = g.current;
        let opp = 1 - actor;
        g.hands[actor] = vec![Geisha::Flute, Geisha::Koto, Geisha::Fan, Geisha::Tea];
        g.command(actor, "gift flute koto fan", &names()).unwrap();
        assert_eq!(Phase::OpponentChoose, g.phase);
        assert!(matches!(g.pending, Some(Pending::Gift { .. })));
        g.command(opp, "choose flute", &names()).unwrap();
        assert_eq!(1, g.faceup[Geisha::Flute.index()][opp]);
        assert_eq!(1, g.faceup[Geisha::Koto.index()][actor]);
        assert_eq!(1, g.faceup[Geisha::Fan.index()][actor]);
        assert!(g.used[actor][2]);
        assert!(g.pending.is_none());
    }

    #[test]
    fn test_competition_command_flow() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let actor = g.current;
        let opp = 1 - actor;
        g.hands[actor] = vec![
            Geisha::Flute,
            Geisha::Koto,
            Geisha::Fan,
            Geisha::Tea,
            Geisha::Taiko,
        ];
        g.command(actor, "compete flute koto fan tea", &names())
            .unwrap();
        assert_eq!(Phase::OpponentChoose, g.phase);
        assert!(matches!(g.pending, Some(Pending::Competition { .. })));
        g.command(opp, "choose 2", &names()).unwrap();
        assert_eq!(1, g.faceup[Geisha::Fan.index()][opp]);
        assert_eq!(1, g.faceup[Geisha::Tea.index()][opp]);
        assert_eq!(1, g.faceup[Geisha::Flute.index()][actor]);
        assert_eq!(1, g.faceup[Geisha::Koto.index()][actor]);
        assert!(g.used[actor][3]);
        assert!(g.pending.is_none());
    }

    #[test]
    fn test_decide_winner_geisha() {
        assert_eq!(Some(0), Game::decide_winner(&[4, 3], &[10, 10]));
        assert_eq!(Some(1), Game::decide_winner(&[3, 4], &[10, 10]));
    }

    #[test]
    fn test_decide_winner_charm() {
        assert_eq!(Some(0), Game::decide_winner(&[3, 3], &[11, 10]));
        assert_eq!(Some(1), Game::decide_winner(&[3, 3], &[10, 11]));
    }

    #[test]
    fn test_decide_winner_charm_beats_geisha() {
        assert_eq!(Some(1), Game::decide_winner(&[4, 3], &[10, 11]));
        assert_eq!(Some(0), Game::decide_winner(&[3, 4], &[11, 10]));
    }

    #[test]
    fn test_decide_winner_none() {
        assert_eq!(None, Game::decide_winner(&[3, 3], &[10, 10]));
    }

    #[test]
    fn test_score_round_win() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.faceup = vec![[0, 0]; GEISHA];
        g.faceup[0] = [1, 0];
        g.faceup[1] = [1, 0];
        g.faceup[2] = [1, 0];
        g.faceup[3] = [1, 0];
        g.secret = vec![None, None];
        g.score_round();
        assert_eq!(Some(0), g.winner);
        assert_eq!(Phase::Finished, g.phase);
        assert!(g.is_finished());
        assert_eq!(vec![1, 2], g.placings());
    }

    #[test]
    fn test_score_round_no_winner_starts_new_round() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.faceup = vec![[0, 0]; GEISHA];
        g.faceup[0] = [1, 1];
        g.secret = vec![None, None];
        let old_starting = g.starting;
        let old_round = g.round;
        g.score_round();
        assert!(g.winner.is_none());
        assert_eq!(old_round + 1, g.round);
        assert_eq!(1 - old_starting, g.starting);
        assert_eq!(Phase::ChooseAction, g.phase);
        assert_eq!(g.starting, g.current);
    }

    #[test]
    fn test_secret_revealed_on_scoring() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.faceup = vec![[0, 0]; GEISHA];
        g.secret = vec![Some(Geisha::Tea), None];
        g.score_round();
        assert_eq!(Some(0), g.marker[Geisha::Tea.index()]);
        assert_eq!(vec![None, None], g.secret);
    }

    #[test]
    fn test_validate() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.validate().is_ok());

        g.current = 2;
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
        g.current = 0;

        g.marker.push(None);
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
        g.marker.pop();

        g.hands.push(vec![]);
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
        g.hands.pop();

        g.players = 3;
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
        g.players = 2;

        g.winner = Some(5);
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_rejects_opponent_choose_without_pending() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::OpponentChoose;
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_rejects_gift_pending_outside_opponent_choose() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.pending = Some(Pending::Gift {
            actor: 0,
            cards: vec![Geisha::Flute, Geisha::Koto, Geisha::Fan],
        });
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_rejects_finished_with_pending() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Finished;
        g.pending = Some(Pending::Gift {
            actor: 0,
            cards: vec![Geisha::Flute, Geisha::Koto, Geisha::Fan],
        });
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_rejects_competition_pending_outside_opponent_choose() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.pending = Some(Pending::Competition {
            actor: 0,
            sets: [
                vec![Geisha::Flute, Geisha::Koto],
                vec![Geisha::Fan, Geisha::Tea],
            ],
        });
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_rejects_pending_actor_mismatch() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::OpponentChoose;
        g.pending = Some(Pending::Gift {
            actor: 1 - g.current,
            cards: vec![Geisha::Flute, Geisha::Koto, Geisha::Fan],
        });
        assert!(matches!(g.validate(), Err(GameError::Internal { .. })));
    }

    #[test]
    fn test_validate_accepts_gift_flow_invariant() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let actor = g.current;
        let opp = 1 - actor;
        g.hands[actor] = vec![Geisha::Flute, Geisha::Koto, Geisha::Fan, Geisha::Tea];
        g.command(actor, "gift flute koto fan", &names()).unwrap();
        assert_eq!(Phase::OpponentChoose, g.phase);
        assert!(matches!(
            g.pending,
            Some(Pending::Gift { actor: a, .. }) if a == actor
        ));
        assert!(g.validate().is_ok());
        g.command(opp, "choose flute", &names()).unwrap();
        assert!(g.pending.is_none());
        assert!(g.validate().is_ok());
    }

    #[test]
    fn test_redaction() {
        let (g, _) = Game::start(2, 1).unwrap();
        let ps = g.pub_state();
        let mut counts = ps.hand_counts.clone();
        counts.sort();
        assert_eq!(vec![6, 7], counts);
        assert_eq!(7, ps.deck_remaining);
        assert_eq!(vec![false, false], ps.has_secret);
        assert_eq!(vec![0, 0], ps.traded_counts);
        let p0 = g.player_state(0);
        assert_eq!(g.hands[0], p0.hand);
    }

    #[test]
    fn test_garbage_command_is_user_error() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let err = g
            .command(0, "!!! not a command @@@ ###", &names())
            .unwrap_err();
        assert!(matches!(err, GameError::InvalidInput { .. }));
    }

    fn legal_command(g: &Game) -> (usize, String) {
        let player = g.whose_turn()[0];
        match g.phase {
            Phase::ChooseAction => {
                let hand = &g.hands[player];
                let used = g.used[player];
                if !used[0] && !hand.is_empty() {
                    (player, format!("secret {}", hand[0].name().to_lowercase()))
                } else if !used[1] && hand.len() >= 2 {
                    (
                        player,
                        format!(
                            "trade {} {}",
                            hand[0].name().to_lowercase(),
                            hand[1].name().to_lowercase()
                        ),
                    )
                } else if !used[2] && hand.len() >= 3 {
                    (
                        player,
                        format!(
                            "gift {} {} {}",
                            hand[0].name().to_lowercase(),
                            hand[1].name().to_lowercase(),
                            hand[2].name().to_lowercase()
                        ),
                    )
                } else if !used[3] && hand.len() >= 4 {
                    (
                        player,
                        format!(
                            "compete {} {} {} {}",
                            hand[0].name().to_lowercase(),
                            hand[1].name().to_lowercase(),
                            hand[2].name().to_lowercase(),
                            hand[3].name().to_lowercase()
                        ),
                    )
                } else {
                    panic!("no legal action available");
                }
            }
            Phase::OpponentChoose => match g.pending.as_ref().expect("pending choice") {
                Pending::Gift { cards, .. } => {
                    (player, format!("choose {}", cards[0].name().to_lowercase()))
                }
                Pending::Competition { .. } => (player, "choose 1".to_string()),
            },
            Phase::Finished => panic!("game already finished"),
        }
    }

    fn has_epilogue(logs: &[Log]) -> bool {
        logs.iter()
            .any(|l| brdgme_markup::to_string(&l.content).contains("Final scores:"))
    }

    fn drive_to_finish(g: &mut Game) -> (CommandResponse, Vec<Log>) {
        let mut non_final: Vec<Log> = vec![];
        let mut iterations = 0;
        loop {
            iterations += 1;
            assert!(iterations < 200, "game did not terminate");
            let (player, cmd) = legal_command(g);
            let resp = g.command(player, &cmd, &names()).unwrap();
            if g.is_finished() {
                return (resp, non_final);
            }
            non_final.extend(resp.logs);
        }
    }

    #[test]
    fn test_play_full_game_to_finish() {
        for seed in [1u64, 42, 999] {
            let (mut g, _) = Game::start(2, seed).unwrap();
            let mut iterations = 0;
            while !g.is_finished() {
                iterations += 1;
                assert!(iterations < 200, "game did not terminate for seed {seed}");
                let (player, cmd) = legal_command(&g);
                g.command(player, &cmd, &names()).unwrap();
            }
            assert!(g.is_finished());
            assert!(g.winner.is_some());
            let mut placings = g.placings();
            assert_eq!(2, placings.len());
            placings.sort();
            assert_eq!(vec![1, 2], placings);
            assert!(g.validate().is_ok());
        }
    }

    #[test]
    fn test_finish_emits_epilogue_once() {
        let (mut g, _) = Game::start(2, 5).unwrap();
        let (final_resp, non_final) = drive_to_finish(&mut g);
        assert!(has_epilogue(&final_resp.logs));
        assert!(!has_epilogue(&non_final));
    }

    #[test]
    fn test_command_parse_round_trips() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        g.hands[cur] = vec![Geisha::Flute, Geisha::Koto, Geisha::Fan, Geisha::Tea];

        let parse = |g: &Game, player: usize, input: &str| {
            g.command_parser(player)
                .expect("parser available")
                .parse(input, &names())
                .expect("parses")
                .value
        };

        assert_eq!(
            Command::Secret(Geisha::Flute),
            parse(&g, cur, "secret flute")
        );
        assert_eq!(
            Command::Trade(Geisha::Flute, Geisha::Koto),
            parse(&g, cur, "trade flute koto")
        );
        assert_eq!(
            Command::Gift(Geisha::Flute, Geisha::Koto, Geisha::Fan),
            parse(&g, cur, "gift flute koto fan")
        );
        assert_eq!(
            Command::Compete(Geisha::Flute, Geisha::Koto, Geisha::Fan, Geisha::Tea),
            parse(&g, cur, "compete flute koto fan tea")
        );

        assert_eq!(
            Command::Secret(Geisha::Flute),
            parse(&g, cur, "SECRET FLUTE")
        );
        assert_eq!(
            Command::Secret(Geisha::Flute),
            parse(&g, cur, "Secret Flute")
        );

        let opp = 1 - cur;
        g.phase = Phase::OpponentChoose;
        g.pending = Some(Pending::Gift {
            actor: cur,
            cards: vec![Geisha::Flute, Geisha::Koto, Geisha::Fan],
        });
        assert_eq!(
            Command::ChooseCard(Geisha::Flute),
            parse(&g, opp, "choose flute")
        );

        g.pending = Some(Pending::Competition {
            actor: cur,
            sets: [
                vec![Geisha::Flute, Geisha::Koto],
                vec![Geisha::Fan, Geisha::Tea],
            ],
        });
        assert_eq!(Command::ChooseSet(0), parse(&g, opp, "choose 1"));
        assert_eq!(Command::ChooseSet(1), parse(&g, opp, "choose 2"));
    }

    #[test]
    fn test_command_spec_availability() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        assert!(g.command_spec(cur).is_some());
        assert!(g.command_spec(1 - cur).is_none());

        g.winner = Some(0);
        assert!(g.command_spec(0).is_none());
        assert!(g.command_spec(1).is_none());
    }

    #[test]
    fn test_marker_persists_across_rounds() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.faceup = vec![[0, 0]; GEISHA];
        g.faceup[0] = [1, 0];
        g.secret = vec![None, None];
        let old_round = g.round;
        g.score_round();
        assert!(g.winner.is_none());
        assert_eq!(old_round + 1, g.round);
        assert_eq!(Some(0), g.marker[0]);
        for f in &g.faceup {
            assert_eq!([0, 0], *f);
        }
    }

    #[test]
    fn test_multibyte_and_hostile_input() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let cur = g.current;
        g.hands[cur] = vec![Geisha::Tea, Geisha::Taiko];
        let hostile = [
            "\u{a0}secret",
            "secret\u{a0}flute",
            "secret \u{3000}flute",
            "secret caf\u{e9}",
            "secret \u{1f004}",
            "e\u{301}",
        ];
        for input in hostile {
            let result = g.command(cur, input, &names());
            assert!(
                matches!(result, Err(GameError::InvalidInput { .. })),
                "expected InvalidInput for {input:?}"
            );
        }
    }
}
