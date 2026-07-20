use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::card::Card;
use crate::command::Command;

pub mod card;
mod command;
mod render;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 4;

/// Points required to win the game, keyed by player count - matches the Go
/// `endScores` map.
fn end_score(players: usize) -> usize {
    match players {
        2 => 7,
        3 => 5,
        4 => 4,
        _ => unreachable!(),
    }
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub round: usize,
    pub deck: Vec<Card>,
    pub removed: Vec<Card>,
    pub hands: Vec<Vec<Card>>,
    pub discards: Vec<Vec<Card>>,
    pub player_points: Vec<usize>,
    pub current_player: usize,
    pub eliminated: Vec<bool>,
    pub protected: Vec<bool>,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

/// Spectator view. Structurally omits hidden info: deck/removed card
/// identities and hands are simply not present, only their counts and
/// public consequences (discards, points, elimination/protection) are.
#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in the game, 2 through 4.
    pub players: usize,
    /// Number of cards left in the draw pile. When 0, the round ends after the current turn.
    pub deck_remaining: usize,
    /// Cards each player has discarded this round, indexed by player. Each inner vec is in discard order.
    pub discards: Vec<Vec<Card>>,
    /// Points accumulated toward winning, indexed by player.
    pub player_points: Vec<usize>,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// Whether each player is eliminated from the current round, indexed by player.
    pub eliminated: Vec<bool>,
    /// Whether each player is protected by the Handmaid until their next turn, indexed by player.
    pub protected: Vec<bool>,
    /// Points required to win the game, based on player count.
    pub end_score: usize,
    /// The highest point total held by any player.
    pub leader_points: usize,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// The cards currently in this player's hand.
    pub hand: Vec<Card>,
}

impl Game {
    fn start_round(&mut self) -> Vec<Log> {
        self.round += 1;
        self.eliminated = vec![false; self.players];
        self.protected = vec![false; self.players];
        let mut deck = card::initial_deck();
        deck.shuffle(&mut self.rng);
        let remove = if self.players == 2 { 4 } else { 1 };
        let mut logs = vec![Log::public(vec![
            N::text(format!("Starting round {}, ", self.round)),
            N::Bold(vec![N::text(format!(
                "removing {} {}",
                remove,
                plural(remove, "card")
            ))]),
        ])];
        self.removed = deck[..remove].to_vec();
        self.deck = deck[remove..].to_vec();
        self.hands = vec![vec![]; self.players];
        self.discards = vec![vec![]; self.players];
        for p in 0..self.players {
            logs.extend(self.draw_card(p));
        }
        logs.extend(self.start_turn());
        logs
    }

    fn start_turn(&mut self) -> Vec<Log> {
        self.protected[self.current_player] = false;
        if self.deck.is_empty() {
            self.end_round()
        } else {
            self.draw_card(self.current_player)
        }
    }

    fn next_player(&mut self) -> Vec<Log> {
        loop {
            self.current_player = (self.current_player + 1) % self.players;
            if !self.eliminated[self.current_player] {
                break;
            }
        }
        self.start_turn()
    }

    fn discard_card_log(&mut self, player: usize, card: Card) -> Vec<Log> {
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" discarded "),
            render::card(card),
        ])];
        logs.extend(self.discard_card(player, card));
        logs
    }

    /// Removes one instance of `card` from `player`'s hand (if present) and
    /// records it as discarded. Eliminates the player if it was the
    /// Princess. Mirrors Go `DiscardCard`.
    fn discard_card(&mut self, player: usize, card: Card) -> Vec<Log> {
        if let Some(pos) = self.hands[player].iter().position(|&c| c == card) {
            self.hands[player].remove(pos);
        }
        self.discards[player].push(card);
        if card == Card::Princess {
            self.eliminate(player)
        } else {
            vec![]
        }
    }

    fn eliminate(&mut self, player: usize) -> Vec<Log> {
        if self.eliminated[player] {
            return vec![];
        }
        self.eliminated[player] = true;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" has been eliminated from this round"),
        ])];
        while let Some(&card) = self.hands[player].first() {
            logs.extend(self.discard_card_log(player, card));
        }
        let num_remaining = (0..self.players).filter(|&p| !self.eliminated[p]).count();
        if num_remaining <= 1 {
            logs.extend(self.end_round());
        }
        logs
    }

    fn end_round(&mut self) -> Vec<Log> {
        let mut output: Vec<N> = vec![N::Bold(vec![N::text("It is the end of the round")])];
        let mut highest_card: u8 = 0;
        let mut highest_player: usize = 0;
        let mut discard_total: i64 = 0;
        for p in 0..self.players {
            if self.eliminated[p] {
                continue;
            }
            let c = self.hands[p][0];
            let discarded: u32 = self.discards[p].iter().map(|c| c.number() as u32).sum();
            output.push(N::text("\n"));
            output.extend(vec![
                N::Player(p),
                N::text(" had "),
                render::card(c),
                N::text(" (total "),
                N::Bold(vec![N::text(format!("{}", discarded))]),
                N::text(" discarded)"),
            ]);
            if c.number() > highest_card {
                highest_card = c.number();
                discard_total = -1;
            }
            if c.number() == highest_card && discarded as i64 > discard_total {
                discard_total = discarded as i64;
                highest_player = p;
            }
        }

        self.player_points[highest_player] += 1;
        output.push(N::text("\n"));
        output.extend(vec![
            N::Player(highest_player),
            N::text(" won the round and moved to "),
            N::Bold(vec![N::text(format!(
                "{} {}",
                self.player_points[highest_player],
                plural(self.player_points[highest_player], "point")
            ))]),
        ]);

        let is_finished = self.check_finished();
        if is_finished {
            output.push(N::text("\n"));
            output.push(N::Bold(vec![
                N::text("It is the end of the game, the winner is "),
                N::Player(self.leader()),
            ]));
        }
        let mut logs = vec![Log::public(output)];
        if !is_finished {
            self.current_player = highest_player;
            logs.extend(self.start_round());
        }
        logs
    }

    fn leader(&self) -> usize {
        let mut highest = 0;
        let mut player = 0;
        for p in 0..self.players {
            if self.player_points[p] > highest {
                player = p;
                highest = self.player_points[p];
            }
        }
        player
    }

    fn draw_card(&mut self, player: usize) -> Vec<Log> {
        let mut logs = vec![];
        let card = if !self.deck.is_empty() {
            let card = self.deck.remove(0);
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" drew a card from the draw pile, "),
                N::Bold(vec![N::text(format!("{}", self.deck.len()))]),
                N::text(" remaining"),
            ]));
            card
        } else {
            let card = self.removed[0];
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" drew a card from the removed cards"),
            ]));
            card
        };
        logs.push(Log::private(
            vec![N::text("You drew "), render::card(card)],
            vec![player],
        ));
        self.hands[player].push(card);
        logs
    }

    fn check_finished(&self) -> bool {
        self.player_points.iter().copied().max().unwrap_or(0) >= end_score(self.players)
    }

    fn available_targets(&self, for_player: usize) -> Vec<usize> {
        (0..self.players)
            .filter(|&p| p != for_player && !self.eliminated[p] && !self.protected[p])
            .collect()
    }

    /// Mirrors Go `AssertTarget` **exactly**, including its source defect:
    /// the second check re-tests `g.Eliminated[target]` instead of
    /// `g.Protected[target]` (clearly a copy-paste bug in the original), so
    /// targeting a protected player is *not* rejected here - protected
    /// players are only excluded from `available_targets`, which drives the
    /// "must target yourself" fallback when no unprotected targets remain.
    fn assert_target(&self, player: usize, inc_self: bool, target: usize) -> Result<(), GameError> {
        let available_targets = self.available_targets(player);
        if available_targets.is_empty() {
            if target == player {
                return Ok(());
            }
            return Err(GameError::invalid_input(
                "all other players are protected by the Handmaid, so you must target yourself",
            ));
        }

        if !inc_self && target == player {
            return Err(GameError::invalid_input(
                "you cannot target yourself if there are other players you can target",
            ));
        }

        if self.eliminated[target] {
            return Err(GameError::invalid_input("that player is eliminated"));
        }
        // NB: this is meant to be `self.protected[target]` (see Go
        // `AssertTarget`'s second `g.Eliminated[target]` check) but the
        // original has the same condition twice - preserved verbatim.
        if self.eliminated[target] {
            return Err(GameError::invalid_input(
                "that player is protected by the Handmaid",
            ));
        }

        Ok(())
    }

    fn assert_can_play(&self, player: usize) -> Result<(), GameError> {
        if self.current_player != player {
            return Err(GameError::NotYourTurn);
        }
        Ok(())
    }

    fn assert_must_not_play_countess(&self, player: usize) -> Result<(), GameError> {
        if self.hands[player].contains(&Card::Countess) {
            return Err(GameError::invalid_input("you must play the Countess"));
        }
        Ok(())
    }

    fn maybe_next_player(&mut self, cur_round: usize) -> Vec<Log> {
        if self.round == cur_round {
            self.next_player()
        } else {
            vec![]
        }
    }

    pub fn play_princess(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        let cur_round = self.round;

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Princess),
        ])];
        logs.extend(self.discard_card(player, Card::Princess));

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_countess(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::Countess);
        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" discarded "),
            render::card(Card::Countess),
            N::text(", they might have been forced to if they also had "),
            render::card(Card::King),
            N::text(" or "),
            render::card(Card::Prince),
        ]));

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_king(&mut self, player: usize, target: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        self.assert_must_not_play_countess(player)?;
        self.assert_target(player, false, target)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::King);

        if target == player {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" played "),
                render::card(Card::King),
                N::text(", but had nobody to target so just discarded the card"),
            ]));
            logs.extend(self.maybe_next_player(cur_round));
            return Ok(logs);
        }

        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::King),
            N::text(" and swapped hands with "),
            N::Player(target),
        ]));
        logs.push(Log::private(
            vec![
                N::text("You traded your "),
                render::card(self.hands[player][0]),
                N::text(" for "),
                render::card(self.hands[target][0]),
            ],
            vec![player],
        ));
        logs.push(Log::private(
            vec![
                N::text("You traded your "),
                render::card(self.hands[target][0]),
                N::text(" for "),
                render::card(self.hands[player][0]),
            ],
            vec![target],
        ));

        self.hands.swap(player, target);

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_prince(&mut self, player: usize, target: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        self.assert_must_not_play_countess(player)?;
        self.assert_target(player, true, target)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::Prince);

        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Prince),
            N::text(" and made "),
            N::Player(target),
            N::text(" discard their hand and draw a new card"),
        ]));

        let target_card = self.hands[target][0];
        logs.extend(self.discard_card_log(target, target_card));
        if self.round == cur_round && !self.eliminated[target] {
            logs.extend(self.draw_card(target));
        }

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_handmaid(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::Handmaid);

        self.protected[player] = true;
        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Handmaid),
            N::text(
                " and is immune to the effects of other players' cards until the start of their next turn",
            ),
        ]));

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_baron(&mut self, player: usize, target: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        self.assert_target(player, false, target)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::Baron);

        if target == player {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" played "),
                render::card(Card::Baron),
                N::text(", but had nobody to target so just discarded the card"),
            ]));
            logs.extend(self.maybe_next_player(cur_round));
            return Ok(logs);
        }

        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Baron),
            N::text(" and is comparing hands with "),
            N::Player(target),
            N::text(" to see who has a lower card"),
        ]));
        let player_card = self.hands[player][0];
        let target_card = self.hands[target][0];
        logs.push(Log::private(
            vec![
                N::text("You have "),
                render::card(player_card),
                N::text(", "),
                N::Player(target),
                N::text(" has "),
                render::card(target_card),
            ],
            vec![player],
        ));
        logs.push(Log::private(
            vec![
                N::text("You have "),
                render::card(target_card),
                N::text(", "),
                N::Player(player),
                N::text(" has "),
                render::card(player_card),
            ],
            vec![target],
        ));

        let mut eliminate: Option<usize> = None;
        let diff = player_card.number() as i32 - target_card.number() as i32;
        if diff < 0 {
            eliminate = Some(player);
            self.hands[player] = vec![player_card];
        } else if diff > 0 {
            eliminate = Some(target);
            self.hands[target] = vec![target_card];
        }

        match eliminate {
            None => {
                logs.push(Log::public(vec![N::text(
                    "The cards were equal, nobody is eliminated",
                )]));
            }
            Some(elim) => {
                logs.extend(self.eliminate(elim));
            }
        }

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_priest(&mut self, player: usize, target: usize) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        self.assert_target(player, false, target)?;
        let cur_round = self.round;

        let mut logs = self.discard_card(player, Card::Priest);

        if target == player {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" played "),
                render::card(Card::Priest),
                N::text(", but had nobody to target so just discarded the card"),
            ]));
            logs.extend(self.maybe_next_player(cur_round));
            return Ok(logs);
        }

        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Priest),
            N::text(" and looked at "),
            N::Player(target),
            N::text("'s hand"),
        ]));
        let mut private_log = vec![N::Player(target), N::text(" has ")];
        private_log.extend(render::comma_cards(&self.hands[target]));
        logs.push(Log::private(private_log, vec![player]));

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
    }

    pub fn play_guard(
        &mut self,
        player: usize,
        target: usize,
        card: Card,
    ) -> Result<Vec<Log>, GameError> {
        self.assert_can_play(player)?;
        self.assert_target(player, false, target)?;
        let cur_round = self.round;
        let mut logs = vec![];

        if target == player {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" played "),
                render::card(Card::Guard),
                N::text(", but had nobody to target so just discarded the card"),
            ]));
            logs.extend(self.discard_card(player, Card::Guard));
            logs.extend(self.maybe_next_player(cur_round));
            return Ok(logs);
        }

        if card == Card::Guard {
            return Err(GameError::invalid_input(
                "you can't use Guard against other Guards",
            ));
        }

        logs.extend(self.discard_card(player, Card::Guard));

        let mut prefix = vec![
            N::Player(player),
            N::text(" played "),
            render::card(Card::Guard),
            N::text(" and guessed that "),
            N::Player(target),
            N::text(" is a "),
            render::card(card),
            N::text(", "),
        ];

        if self.hands[target].contains(&card) {
            prefix.push(N::text("and was correct!"));
            logs.push(Log::public(prefix));
            logs.extend(self.eliminate(target));
        } else {
            prefix.push(N::text("but was incorrect"));
            logs.push(Log::public(prefix));
        }

        logs.extend(self.maybe_next_player(cur_round));
        Ok(logs)
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
            player_points: vec![0; players],
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        let logs = g.start_round();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.check_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![Default::default(); self.players],
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
            deck_remaining: self.deck.len(),
            discards: self.discards.clone(),
            player_points: self.player_points.clone(),
            current_player: self.current_player,
            eliminated: self.eliminated.clone(),
            protected: self.protected.clone(),
            end_score: end_score(self.players),
            leader_points: self.player_points.iter().copied().max().unwrap_or(0),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: self.hands.get(player).cloned().unwrap_or_default(),
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
                value: Command::Princess,
                remaining,
                ..
            }) => self.play_princess(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Countess,
                remaining,
                ..
            }) => self.play_countess(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::King(target),
                remaining,
                ..
            }) => self.play_king(player, target).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Prince(target),
                remaining,
                ..
            }) => self
                .play_prince(player, target)
                .map(|logs| CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                }),
            Ok(ParseOutput {
                value: Command::Handmaid,
                remaining,
                ..
            }) => self.play_handmaid(player).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Baron(target),
                remaining,
                ..
            }) => self.play_baron(player, target).map(|logs| CommandResponse {
                logs,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                value: Command::Priest(target),
                remaining,
                ..
            }) => self
                .play_priest(player, target)
                .map(|logs| CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                }),
            Ok(ParseOutput {
                value: Command::Guard(target, card),
                remaining,
                ..
            }) => self
                .play_guard(player, target, card)
                .map(|logs| CommandResponse {
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
        self.player_points.iter().map(|&p| p as f32).collect()
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

impl Game {
    fn placings(&self) -> Vec<usize> {
        gen_placings(
            &self
                .player_points
                .iter()
                .map(|&p| vec![p as i32])
                .collect::<Vec<Vec<i32>>>(),
        )
    }
}

pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        word.to_string()
    } else {
        format!("{}s", word)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const BJ: usize = 2;

    fn end_score_of(players: usize) -> usize {
        end_score(players)
    }

    #[test]
    fn test_game_is_finished() {
        let mut g = Game::start(2, 1).unwrap().0;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(2) - 1;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(2);
        assert!(g.check_finished());

        let mut g = Game::start(3, 1).unwrap().0;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(3) - 1;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(3);
        assert!(g.check_finished());

        let mut g = Game::start(4, 1).unwrap().0;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(4) - 1;
        assert!(!g.check_finished());
        g.player_points[MICK] = end_score_of(4);
        assert!(g.check_finished());
    }

    #[test]
    fn char_baron_play_win() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::King];
        g.hands[STEVE] = vec![Card::Prince];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "baron steve", &players).unwrap();
        assert_eq!(vec![Card::King], g.hands[MICK]);
        assert!(!g.eliminated[MICK]);
        assert!(g.eliminated[STEVE]);
    }

    #[test]
    fn char_baron_play_tie() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::Prince];
        g.hands[STEVE] = vec![Card::Prince];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "baron steve", &players).unwrap();
        assert_eq!(vec![Card::Prince], g.hands[MICK]);
        assert!(!g.eliminated[MICK]);
        assert!(!g.eliminated[STEVE]);
    }

    #[test]
    fn char_baron_play_lose() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::Prince];
        g.hands[STEVE] = vec![Card::King];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "baron steve", &players).unwrap();
        assert_eq!(Vec::<Card>::new(), g.hands[MICK]);
        assert!(g.eliminated[MICK]);
        assert!(!g.eliminated[STEVE]);
    }

    #[test]
    fn char_baron_play_double() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::Baron];
        g.hands[STEVE] = vec![Card::Guard];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "baron steve", &players).unwrap();
        assert_eq!(vec![Card::Baron], g.hands[MICK]);
        assert!(!g.eliminated[MICK]);
        assert!(g.eliminated[STEVE]);
    }

    #[test]
    fn char_prince_play_end() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Prince, Card::Princess];
        g.hands[STEVE] = vec![Card::Prince];
        g.protected[STEVE] = true;
        g.eliminated[BJ] = true;
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "prince mick", &players).unwrap();
        assert_eq!(2, g.round);
        assert_eq!(1, g.player_points[STEVE]);
        assert_eq!(STEVE, g.current_player);
        assert_eq!(1, g.hands[MICK].len());
        assert_eq!(2, g.hands[STEVE].len());
        assert_eq!(1, g.hands[BJ].len());
    }

    #[test]
    fn play_king_happy_path() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::King, Card::Guard];
        g.hands[STEVE] = vec![Card::Priest];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "king steve", &players).unwrap();
        assert_eq!(vec![Card::Priest], g.hands[MICK]);
        // Steve's held card (Priest) went to Mick, and Mick's remaining
        // Guard went to Steve, plus Steve draws for their turn.
        assert!(g.hands[STEVE].contains(&Card::Guard));
    }

    #[test]
    fn play_king_must_play_countess() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::King, Card::Countess];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        let err = g.command(MICK, "king steve", &players).unwrap_err();
        assert!(format!("{}", err).contains("you must play the Countess"));
    }

    #[test]
    fn play_prince_must_play_countess() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Prince, Card::Countess];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        let err = g.command(MICK, "prince steve", &players).unwrap_err();
        assert!(format!("{}", err).contains("you must play the Countess"));
    }

    #[test]
    fn play_countess_happy_path() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Countess, Card::King];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "countess", &players).unwrap();
        assert_eq!(vec![Card::King], g.hands[MICK]);
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn play_princess_happy_path() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Princess, Card::King];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "princess", &players).unwrap();
        assert!(g.eliminated[MICK]);
    }

    #[test]
    fn play_handmaid_happy_path() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Handmaid, Card::King];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "handmaid", &players).unwrap();
        // Protected got reset by start_turn for the *new* current player, but
        // Mick himself should have been protected at the time.
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn play_priest_happy_path() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Priest, Card::King];
        g.hands[STEVE] = vec![Card::Guard];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "priest steve", &players).unwrap();
        assert_eq!(STEVE, g.current_player);
    }

    #[test]
    fn play_guard_happy_path_correct() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Guard, Card::King];
        g.hands[STEVE] = vec![Card::Priest];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "guard steve priest", &players).unwrap();
        assert!(g.eliminated[STEVE]);
    }

    #[test]
    fn play_guard_happy_path_incorrect() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Guard, Card::King];
        g.hands[STEVE] = vec![Card::Priest];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        g.command(MICK, "guard steve baron", &players).unwrap();
        assert!(!g.eliminated[STEVE]);
    }

    #[test]
    fn play_guard_cannot_guess_guard() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Guard, Card::King];
        g.hands[STEVE] = vec![Card::Guard];
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        let err = g.command(MICK, "guard steve guard", &players).unwrap_err();
        assert!(format!("{}", err).contains("Guard against other Guards"));
    }

    #[test]
    fn handmaid_protection_forces_self_target_fallback() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::King];
        g.protected[STEVE] = true;
        g.eliminated[BJ] = true;
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        // No unprotected, non-eliminated targets other than self, so
        // targeting self is allowed even though `Baron` normally forbids
        // self-targeting when other targets exist.
        g.command(MICK, "baron mick", &players).unwrap();
        assert_eq!(vec![Card::King], g.hands[MICK]);
    }

    #[test]
    fn eliminated_target_is_rejected() {
        let mut g = Game::start(3, 1).unwrap().0;
        g.hands[MICK] = vec![Card::Baron, Card::King];
        g.eliminated[STEVE] = true;
        let players = vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()];
        let err = g.command(MICK, "baron steve", &players).unwrap_err();
        assert!(format!("{}", err).contains("eliminated"));
    }

    #[test]
    fn end_of_game_finishes_at_end_score() {
        // 2p ends at 7, 3p at 5, 4p at 4.
        let mut g = Game::start(2, 1).unwrap().0;
        g.player_points = vec![7, 0];
        assert!(g.is_finished());

        let mut g = Game::start(3, 1).unwrap().0;
        g.player_points = vec![5, 0, 0];
        assert!(g.is_finished());

        let mut g = Game::start(4, 1).unwrap().0;
        g.player_points = vec![4, 0, 0, 0];
        assert!(g.is_finished());
    }

    #[test]
    fn pub_state_does_not_leak_hidden_info() {
        let g = Game::start(3, 1).unwrap().0;
        let json = serde_json::to_value(g.pub_state()).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("hands"));
        assert!(!obj.contains_key("hand"));
        assert!(!obj.contains_key("deck"));
        assert!(!obj.contains_key("removed"));
        assert!(obj.contains_key("deck_remaining"));
        assert!(obj.contains_key("discards"));
        assert!(obj.contains_key("player_points"));
        assert!(obj.contains_key("eliminated"));
        assert!(obj.contains_key("protected"));
    }
}
