use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;

use command::Command;
use rand::prelude::*;
use rand::seq::SliceRandom;

const MIN_PLAYERS: usize = 3;
const MAX_PLAYERS: usize = 5;
pub const STARTING_CHIPS: i32 = 11;
const CARD_MIN: i32 = 3;
const CARD_MAX: i32 = 35;
const DECK_SIZE: usize = 24;

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub player_hands: Vec<Vec<i32>>,
    pub player_chips: Vec<i32>,
    pub centre_chips: i32,
    pub remaining_cards: Vec<i32>,
    pub currently_moving: usize,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in this game (3 to 5).
    pub players: usize,
    /// True when the deck is empty and the game is over.
    pub finished: bool,
    /// The current card on the table, or None if the game is finished.
    pub current_card: Option<i32>,
    /// Number of cards remaining in the deck after the current card.
    pub remaining_after: usize,
    /// Number of chips accumulated on the current card.
    pub centre_chips: i32,
    /// Cards collected by each player, indexed by player. Cards are numbered 3 to 35.
    pub hands: Vec<Vec<i32>>,
    /// Chips held by each player. Only populated when the game is finished; empty during play.
    pub chips: Vec<i32>,
    /// Final scores for each player. Only populated when the game is finished; empty during play.
    pub final_scores: Vec<i32>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// Number of chips this player currently holds.
    pub chips: i32,
}

impl Game {
    pub fn all_cards() -> Vec<i32> {
        (CARD_MIN..=CARD_MAX).collect()
    }

    pub fn init_cards(&mut self) {
        let mut pool: Vec<i32> = Self::all_cards();
        pool.shuffle(&mut self.rng);
        self.remaining_cards = pool[..DECK_SIZE].to_vec();
    }

    pub fn init_player_chips(&mut self) {
        self.player_chips = vec![STARTING_CHIPS; self.players];
    }

    pub fn peek_top_card(&self) -> i32 {
        *self.remaining_cards.last().expect("no cards remaining")
    }

    pub fn next_player(&mut self) {
        self.currently_moving = (self.currently_moving + 1) % self.players;
    }

    pub fn can_pass(&self, player: usize) -> bool {
        self.currently_moving == player
            && self.player_chips.get(player).copied().unwrap_or(0) > 0
            && !self.remaining_cards.is_empty()
    }

    pub fn can_take(&self, player: usize) -> bool {
        self.currently_moving == player && !self.remaining_cards.is_empty()
    }

    pub fn pass(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_pass(player) {
            return Err(GameError::invalid_input("can't pass at the moment"));
        }
        if self.player_chips[player] <= 0 {
            return Err(GameError::invalid_input(
                "You have no chips left, you must take the card",
            ));
        }
        self.player_chips[player] -= 1;
        self.centre_chips += 1;
        self.next_player();
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" passed on the "),
            N::Bold(vec![render::render_card(self.peek_top_card())]),
        ])])
    }

    pub fn take(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_take(player) {
            return Err(GameError::invalid_input("can't take at the moment"));
        }
        let card = self.peek_top_card();
        let chips_taken = self.centre_chips;
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took the "),
            N::Bold(vec![render::render_card(card)]),
            N::text(" and "),
            N::Bold(vec![render::render_chips(chips_taken)]),
            N::text(" chips"),
        ])];
        self.remaining_cards.pop();
        self.player_hands[player].push(card);
        if !self.remaining_cards.is_empty() {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" drew "),
                N::Bold(vec![render::render_card(self.peek_top_card())]),
                N::text(" as the new card"),
            ]));
        }
        self.player_chips[player] += chips_taken;
        self.centre_chips = 0;
        Ok(logs)
    }

    pub fn player_hand_sorted(&self, player: usize) -> Vec<i32> {
        let mut h = self.player_hands[player].clone();
        h.sort();
        h
    }

    pub fn player_hand_grouped(&self, player: usize) -> Vec<Vec<i32>> {
        let sorted = self.player_hand_sorted(player);
        let mut groups: Vec<Vec<i32>> = vec![];
        let mut cur: Vec<i32> = vec![];
        let mut last: Option<i32> = None;
        for c in sorted {
            if last == Some(c - 1) {
                cur.push(c);
            } else {
                if !cur.is_empty() {
                    groups.push(std::mem::take(&mut cur));
                }
                cur.push(c);
            }
            last = Some(c);
        }
        if !cur.is_empty() {
            groups.push(cur);
        }
        groups
    }

    pub fn player_hand_score(&self, player: usize) -> i32 {
        self.player_hand_grouped(player).iter().map(|g| g[0]).sum()
    }

    pub fn final_player_score(&self, player: usize) -> i32 {
        self.player_hand_score(player) - self.player_chips[player]
    }

    pub fn points_int(&self) -> Vec<i32> {
        (0..self.players)
            .map(|p| {
                if self.remaining_cards.is_empty() {
                    self.final_player_score(p)
                } else {
                    self.player_hand_score(p)
                }
            })
            .collect()
    }

    fn placings(&self) -> Vec<usize> {
        let points = self.points_int();
        let metrics: Vec<Vec<i32>> = (0..self.players).map(|p| vec![-points[p]]).collect();
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
        let mut g = Game {
            players,
            player_hands: vec![vec![]; players],
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        g.init_cards();
        g.init_player_chips();
        g.currently_moving = g.rng.random_range(0..players);
        Ok((g, vec![]))
    }

    fn status(&self) -> Status {
        if self.remaining_cards.is_empty() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.currently_moving],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        let finished = self.remaining_cards.is_empty();
        PubState {
            players: self.players,
            finished,
            current_card: if finished {
                None
            } else {
                Some(self.peek_top_card())
            },
            remaining_after: self.remaining_cards.len().saturating_sub(1),
            centre_chips: self.centre_chips,
            hands: self.player_hands.clone(),
            chips: if finished {
                self.player_chips.clone()
            } else {
                vec![]
            },
            final_scores: if finished {
                (0..self.players)
                    .map(|p| self.final_player_score(p))
                    .collect()
            } else {
                vec![]
            },
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            chips: self.player_chips[player],
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
                value: Command::Pass,
                ..
            }) => {
                let logs = self.pass(player)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Take,
                ..
            }) => {
                let logs = self.take(player)?;
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

    #[test]
    fn test_all_cards() {
        let cards = Game::all_cards();
        assert_eq!(33, cards.len());
        assert_eq!(3, cards[0]);
        assert_eq!(35, cards[32]);
    }

    #[test]
    fn test_init_cards() {
        let mut g = Game::default();
        g.init_cards();
        assert_eq!(24, g.remaining_cards.len());
        for c in &g.remaining_cards {
            assert!(*c >= 3 && *c <= 35);
        }
    }

    #[test]
    fn test_init_player_chips() {
        let mut g = Game::default();
        g.init_player_chips();
        for p in 0..g.players {
            assert_eq!(11, g.player_chips[p]);
        }
    }

    #[test]
    fn test_assert_turn() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.currently_moving = STEVE;
        assert!(!g.can_take(MICK));
        assert!(g.can_take(STEVE));
        g.remaining_cards = vec![];
        assert!(!g.can_take(STEVE));
    }

    #[test]
    fn test_is_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        assert!(!g.is_finished());
        g.remaining_cards = vec![];
        assert!(g.is_finished());
    }

    #[test]
    fn test_pass() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let initial_player = g.currently_moving;
        let initial_card_count = g.remaining_cards.len();
        let initial_centre_chips = g.centre_chips;
        let initial_player_chips = g.player_chips[initial_player];
        g.pass(initial_player).unwrap();
        assert_eq!(initial_card_count, g.remaining_cards.len());
        assert_eq!(initial_centre_chips + 1, g.centre_chips);
        assert_eq!(initial_player_chips - 1, g.player_chips[initial_player]);
        assert_ne!(initial_player, g.currently_moving);
    }

    #[test]
    fn test_take() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let initial_player = g.currently_moving;
        let initial_card_count = g.remaining_cards.len();
        g.centre_chips = 5;
        let initial_centre_chips = g.centre_chips;
        let initial_player_chips = g.player_chips[initial_player];
        let top_card = g.peek_top_card();
        g.take(initial_player).unwrap();
        assert_eq!(initial_card_count - 1, g.remaining_cards.len());
        assert_eq!(0, g.centre_chips);
        assert_eq!(
            initial_player_chips + initial_centre_chips,
            g.player_chips[initial_player]
        );
        assert_eq!(1, g.player_hands[initial_player].len());
        assert_eq!(top_card, g.player_hands[initial_player][0]);
        assert_eq!(initial_player, g.currently_moving);
    }

    #[test]
    fn test_player_hand_sorted() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_hands[MICK] = vec![5, 3, 6, 4, 87];
        let sorted = g.player_hand_sorted(MICK);
        assert!(sorted.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn test_player_hand_grouped() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_hands[MICK] = vec![5, 8, 3, 10, 9, 15, 6, 16];
        let grouping = g.player_hand_grouped(MICK);
        assert_eq!(4, grouping.len());
        assert_eq!(vec![3], grouping[0]);
        assert_eq!(vec![5, 6], grouping[1]);
        assert_eq!(vec![8, 9, 10], grouping[2]);
        assert_eq!(vec![15, 16], grouping[3]);
    }

    #[test]
    fn test_player_hand_score() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_hands[MICK] = vec![5, 8, 3, 10, 9, 15, 6, 16];
        g.player_chips[MICK] = 10;
        let expected = 3 + 5 + 8 + 15;
        assert_eq!(expected, g.player_hand_score(MICK));
    }

    #[test]
    fn test_final_player_score() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_hands[MICK] = vec![5, 8, 3, 10, 9, 15, 6, 16];
        g.player_chips[MICK] = 10;
        let expected = 3 + 5 + 8 + 15 - 10;
        assert_eq!(expected, g.final_player_score(MICK));
    }

    #[test]
    fn test_whose_turn() {
        let (g, _) = Game::start(3, 1).unwrap();
        let wt = g.whose_turn();
        assert_eq!(1, wt.len());
        assert_eq!(g.currently_moving, wt[0]);
    }

    #[test]
    fn test_winners() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_hands[BJ] = vec![5, 8, 3, 10, 9, 15, 6, 16];
        g.player_chips[BJ] = 3;
        g.player_hands[MICK] = vec![5, 8, 3, 10, 9, 15, 6, 16];
        g.player_chips[MICK] = 10;
        g.player_hands[STEVE] = vec![5, 8, 3, 10, 9, 6, 16, 17];
        g.player_chips[STEVE] = 11;
        g.remaining_cards = vec![];
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_pub_state_chips_hidden_until_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        assert!(!g.pub_state().finished);
        assert!(g.pub_state().chips.is_empty());
        g.remaining_cards = vec![];
        let ps = g.pub_state();
        assert!(ps.finished);
        assert_eq!(g.player_chips, ps.chips);
    }

    #[test]
    fn test_player_actions() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let p = players(3);
        g.currently_moving = STEVE;
        let top_card = g.peek_top_card();
        g.command(STEVE, "pass", &p).unwrap();
        assert_eq!(10, g.player_chips[STEVE]);
        g.command(BJ, "taKE", &p).unwrap();
        assert_eq!(vec![top_card], g.player_hands[BJ]);
        assert_eq!(12, g.player_chips[BJ]);
    }
}
