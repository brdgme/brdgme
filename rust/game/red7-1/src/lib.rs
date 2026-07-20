use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::card::{Card, Suit, full_deck, leader, points, sort_by_suit, suit_rule};

mod card;
mod command;
mod render;

pub use card::{Card as PubCard, Suit as PubSuit};
pub use command::Command;

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

pub fn end_points(players: usize) -> u32 {
    (50 - players * 5) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub num_players: usize,
    pub finished: bool,
    pub current_player: usize,
    pub has_played: bool,
    pub deck: Vec<Card>,
    pub discard_pile: Vec<Card>,
    pub hands: Vec<Vec<Card>>,
    pub palettes: Vec<Vec<Card>>,
    pub scored_cards: Vec<Vec<Card>>,
    pub eliminated: Vec<bool>,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in the game (2-4).
    pub num_players: usize,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// Number of cards remaining in the draw deck.
    pub deck_len: usize,
    /// The discard pile. The suit of the top card determines the current winning rule.
    pub discard_pile: Vec<Card>,
    /// Number of cards in each player's hand, indexed by player.
    pub hand_sizes: Vec<usize>,
    /// Cards each player has played to their palette, indexed by player.
    pub palettes: Vec<Vec<Card>>,
    /// Cards each player has scored from won rounds, indexed by player.
    pub scored_cards: Vec<Vec<Card>>,
    /// Whether each player has been eliminated this round, indexed by player.
    pub eliminated: Vec<bool>,
    /// True when the game is over.
    pub finished: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// Cards in this player's hand.
    pub hand: Vec<Card>,
}

impl Game {
    fn start_round(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let l = self.num_players;

        for p in 0..l {
            self.deck.append(&mut self.hands[p]);
            self.deck.append(&mut self.palettes[p]);
        }
        self.deck.append(&mut self.discard_pile);
        self.hands = vec![vec![]; l];
        self.palettes = vec![vec![]; l];
        self.eliminated = vec![false; l];

        if self.deck.len() < l * 8 {
            self.end_game(&mut logs);
            return logs;
        }

        self.deck.shuffle(&mut self.rng);

        for p in 0..l {
            logs.extend(self.draw(p, 7));
            self.palettes[p] = vec![self.deck.remove(0)];
        }

        let leader_idx = self.leader().0;
        self.current_player = self.next_player(leader_idx);
        self.start_turn(&mut logs);
        logs
    }

    fn draw(&mut self, player: usize, n: usize) -> Vec<Log> {
        let mut logs = vec![];
        let deck_len = self.deck.len();
        if deck_len == 0 {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" didn't draw from the deck as there are no cards left"),
            ]));
            return logs;
        }
        let n = n.min(deck_len);
        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" drew "),
            N::Bold(vec![N::text(format!("{}", n))]),
            N::text(" cards from the deck"),
        ]));

        let drawn: Vec<Card> = self.deck.drain(..n).collect();
        let mut display = drawn.clone();
        sort_by_suit(&mut display);
        display.reverse();
        let card_nodes: Vec<N> = display
            .iter()
            .map(|c| {
                N::Fg(
                    c.suit.color().into(),
                    vec![N::Bold(vec![N::text(format!("{}", c))])],
                )
            })
            .collect();
        let mut content = vec![N::text("You drew ")];
        content.extend(card_nodes);
        logs.push(Log::private(content, vec![player]));

        self.hands[player].extend(drawn);
        logs
    }

    fn start_turn(&mut self, logs: &mut Vec<Log>) {
        self.has_played = false;
        if self.hands[self.current_player].is_empty() {
            self.eliminate(self.current_player, " for not having any cards left", logs);
            self.end_turn(logs);
        }
    }

    fn end_turn(&mut self, logs: &mut Vec<Log>) {
        if !self.eliminated[self.current_player] {
            let leader_idx = self.leader().0;
            if leader_idx != self.current_player {
                self.eliminate(
                    self.current_player,
                    " for not being the leader at the end of their turn",
                    logs,
                );
            }
        }

        if self.remaining_players().len() == 1 {
            self.end_round(logs);
            return;
        }

        self.current_player = self.next_player(self.current_player);
        self.start_turn(logs);
    }

    fn end_round(&mut self, logs: &mut Vec<Log>) {
        let (leader_idx, leader_palette) = self.leader();
        let pts = points(&leader_palette);
        self.scored_cards[leader_idx].extend(&leader_palette);
        self.palettes[leader_idx].retain(|c| !leader_palette.contains(c));

        let mut sorted_pal = leader_palette.clone();
        sort_by_suit(&mut sorted_pal);
        let card_nodes: Vec<N> = sorted_pal
            .iter()
            .map(|c| {
                N::Fg(
                    c.suit.color().into(),
                    vec![N::Bold(vec![N::text(format!("{}", c))])],
                )
            })
            .collect();

        let mut content = vec![N::Player(leader_idx), N::text(" won the round with ")];
        content.extend(card_nodes);
        content.push(N::text(" for "));
        content.push(N::Bold(vec![N::text(format!("{}", pts))]));
        content.push(N::text(" points, now on "));
        content.push(N::Bold(vec![N::text(format!(
            "{}",
            self.player_points(leader_idx)
        ))]));
        content.push(N::text(" points"));
        logs.push(Log::public(content));

        let ep = end_points(self.num_players);
        for p in 0..self.num_players {
            if self.player_points(p) >= ep {
                self.end_game(logs);
                return;
            }
        }

        let round_logs = self.start_round();
        logs.extend(round_logs);
    }

    fn end_game(&mut self, logs: &mut Vec<Log>) {
        logs.push(Log::public(vec![N::Bold(vec![N::text(
            "It is the end of the game",
        )])]));
        self.finished = true;
    }

    fn eliminate(&mut self, player: usize, message: &str, logs: &mut Vec<Log>) {
        self.eliminated[player] = true;
        logs.push(Log::public(vec![
            N::Player(player),
            N::text(format!(" has been eliminated{}", message)),
        ]));
    }

    pub fn leader(&self) -> (usize, Vec<Card>) {
        self.leader_with_suit(self.current_rule())
    }

    pub fn leader_with_suit(&self, suit: Suit) -> (usize, Vec<Card>) {
        let rule_fn = suit_rule(suit);
        let mut player_map: Vec<usize> = vec![];
        let mut palettes: Vec<Vec<Card>> = vec![];

        for p in 0..self.num_players {
            if self.eliminated[p] {
                continue;
            }
            player_map.push(p);
            palettes.push(rule_fn(&self.palettes[p]));
        }

        let (l_index, palette) = leader(&palettes);
        (player_map[l_index], palette)
    }

    pub fn current_rule(&self) -> Suit {
        self.discard_pile
            .last()
            .map(|c| c.suit)
            .unwrap_or(Suit::Red)
    }

    fn next_player(&self, from: usize) -> usize {
        let l = self.num_players;
        let mut n = (from + 1) % l;
        loop {
            if n == from || !self.eliminated[n] {
                break;
            }
            n = (n + 1) % l;
        }
        n
    }

    pub fn player_points(&self, player: usize) -> u32 {
        points(&self.scored_cards[player])
    }

    fn remaining_players(&self) -> Vec<usize> {
        (0..self.num_players)
            .filter(|&p| !self.eliminated[p])
            .collect()
    }

    fn can_play(&self, player: usize) -> bool {
        self.current_player == player && !self.has_played && !self.finished
    }

    fn can_discard(&self, player: usize) -> bool {
        self.current_player == player && !self.finished
    }

    fn can_done(&self, player: usize) -> bool {
        self.current_player == player && !self.finished
    }

    pub fn play(&mut self, player: usize, card: Card) -> Result<Vec<Log>, GameError> {
        if !self.can_play(player) {
            return Err(GameError::invalid_input("you can't play at the moment"));
        }
        let index = self.hands[player]
            .iter()
            .position(|&c| c == card)
            .ok_or_else(|| GameError::invalid_input("you don't have that card"))?;
        self.hands[player].remove(index);
        self.palettes[player].push(card);
        self.has_played = true;
        Ok(vec![Log::public(vec![
            N::Player(player),
            N::text(" played "),
            N::Fg(
                card.suit.color().into(),
                vec![N::Bold(vec![N::text(format!("{}", card))])],
            ),
        ])])
    }

    pub fn discard(&mut self, player: usize, card: Card) -> Result<Vec<Log>, GameError> {
        if !self.can_discard(player) {
            return Err(GameError::invalid_input("you can't discard at the moment"));
        }
        let index = self.hands[player]
            .iter()
            .position(|&c| c == card)
            .ok_or_else(|| GameError::invalid_input("you don't have that card"))?;

        let (leader_idx, _) = self.leader_with_suit(card.suit);
        if leader_idx != player {
            return Err(GameError::invalid_input(
                "you wouldn't be the leader after discarding that card",
            ));
        }

        self.hands[player].remove(index);
        self.discard_pile.push(card);

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" discarded "),
            N::Fg(
                card.suit.color().into(),
                vec![N::Bold(vec![N::text(format!("{}", card))])],
            ),
            N::text(", the new rule is "),
            N::Bold(vec![N::text(card.suit.rule_str().to_string())]),
        ])];

        if card.rank as usize > self.palettes[player].len() {
            logs.extend(self.draw(player, 1));
        }

        self.has_played = true;
        self.end_turn(&mut logs);
        Ok(logs)
    }

    pub fn done(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_done(player) {
            return Err(GameError::invalid_input("you can't done at the moment"));
        }
        let mut logs = vec![];
        if !self.has_played {
            self.eliminate(
                self.current_player,
                " for not playing or discarding",
                &mut logs,
            );
        }
        self.end_turn(&mut logs);
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
            num_players: players,
            finished: false,
            current_player: 0,
            has_played: false,
            deck: full_deck(),
            discard_pile: vec![],
            hands: vec![vec![]; players],
            palettes: vec![vec![]; players],
            scored_cards: vec![vec![]; players],
            eliminated: vec![false; players],
            rng: GameRng::seed_from_u64(seed),
        };
        let logs = g.start_round();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.finished {
            let metrics: Vec<Vec<i32>> = (0..self.num_players)
                .map(|p| vec![self.player_points(p) as i32])
                .collect();
            Status::Finished {
                placings: gen_placings(&metrics),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: (0..self.num_players)
                    .filter(|&p| self.eliminated[p])
                    .collect(),
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            num_players: self.num_players,
            current_player: self.current_player,
            deck_len: self.deck.len(),
            discard_pile: self.discard_pile.clone(),
            hand_sizes: self.hands.iter().map(|h| h.len()).collect(),
            palettes: self.palettes.clone(),
            scored_cards: self.scored_cards.clone(),
            eliminated: self.eliminated.clone(),
            finished: self.finished,
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
    ) -> Result<brdgme_game::CommandResponse, GameError> {
        use brdgme_game::CommandResponse;
        use brdgme_game::command::parser::Output as ParseOutput;

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
                value: Command::Play { card },
                ..
            }) => {
                let logs = self.play(player, card)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Discard { card },
                ..
            }) => {
                let logs = self.discard(player, card)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Done,
                ..
            }) => {
                let logs = self.done(player)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Err(e) => Err(GameError::invalid_input(e.to_string())),
        }
    }

    fn command_spec(&self, player: usize) -> Option<brdgme_game::command::Spec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.num_players)
            .map(|p| self.player_points(p) as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
    }

    fn player_count(&self) -> usize {
        self.num_players
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
    use crate::card::*;

    fn crd(input: &str) -> Card {
        Card::parse(input).unwrap_or_else(|| panic!("could not parse card {}", input))
    }

    fn crds(inputs: &[&str]) -> Vec<Card> {
        inputs.iter().map(|&s| crd(s)).collect()
    }

    #[test]
    fn test_game_start() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.hands[0].len(), 7);
        assert_eq!(g.hands[1].len(), 7);
        assert_eq!(g.palettes[0].len(), 1);
        assert_eq!(g.palettes[1].len(), 1);
    }

    #[test]
    fn test_game_current_rule() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        assert_eq!(Suit::Red, g.current_rule());
        g.discard_pile.push(crd("b5"));
        assert_eq!(Suit::Blue, g.current_rule());
        g.discard_pile.push(crd("y5"));
        assert_eq!(Suit::Yellow, g.current_rule());
    }

    #[test]
    fn test_game_decode() {
        let (g, _) = Game::start(2, 0).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let _g2: Game = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_game_leader() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.palettes = vec![crds(&["y3"]), crds(&["b4"])];
        let (leader_idx, palette) = g.leader();
        assert_eq!(1, leader_idx);
        assert_eq!(crds(&["b4"]), palette);
    }

    #[test]
    fn test_game_next_player() {
        let (mut g, _) = Game::start(4, 0).unwrap();
        assert_eq!(1, g.next_player(0));
        g.eliminated[0] = true;
        assert_eq!(3, g.next_player(2));
        g.eliminated[1] = true;
        assert_eq!(3, g.next_player(2));
        g.eliminated[3] = true;
        assert_eq!(2, g.next_player(0));
    }

    #[test]
    fn test_end_points() {
        assert_eq!(40, end_points(2));
        assert_eq!(35, end_points(3));
        assert_eq!(30, end_points(4));
    }

    #[test]
    fn test_game_end_round() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let initial_len = g.deck.len();
        let player = g.current_player;
        g.command(player, "done", &[]).unwrap();
        assert_eq!(g.deck.len(), initial_len - 1);
    }

    #[test]
    fn test_parse_card() {
        let cases: Vec<(&str, bool, Option<Card>)> = vec![
            ("b5", true, Some(Card::new(Suit::Blue, 5))),
            ("a5", false, None),
            ("r0", false, None),
            ("red5", false, None),
            ("r8", false, None),
            ("r10", false, None),
            ("o4", true, Some(Card::new(Suit::Orange, 4))),
        ];
        for (input, expected_success, expected_card) in cases {
            let result = Card::parse(input);
            assert_eq!(expected_success, result.is_some(), "input: {}", input);
            if expected_success {
                assert_eq!(expected_card.unwrap(), result.unwrap());
            }
        }
    }

    #[test]
    fn test_highest_card() {
        let input = crds(&["r6", "r7", "b7"]);
        assert_eq!(crds(&["r7"]), highest_card(&input));
    }

    #[test]
    fn test_cards_of_one_number() {
        assert_eq!(
            crds(&["r7", "b7"]),
            cards_of_one_number(&crds(&["r6", "r7", "b7"]))
        );
        assert_eq!(
            crds(&["r6", "b6"]),
            cards_of_one_number(&crds(&["r6", "r7", "b6"]))
        );
    }

    #[test]
    fn test_cards_of_one_color() {
        assert_eq!(
            crds(&["r7", "r6"]),
            cards_of_one_color(&crds(&["r6", "r7", "b7"]))
        );
        assert_eq!(
            crds(&["b7", "b6"]),
            cards_of_one_color(&crds(&["r6", "b7", "b6", "r5"]))
        );
    }

    #[test]
    fn test_most_even_cards() {
        assert_eq!(crds(&["r6"]), most_even_cards(&crds(&["r6", "r7", "b7"])));
        assert_eq!(
            crds(&["r6", "b6"]),
            most_even_cards(&crds(&["r6", "b7", "b6", "r5"]))
        );
    }

    #[test]
    fn test_cards_of_different_colors() {
        assert_eq!(
            crds(&["r7", "y7", "b7"]),
            cards_of_different_colors(&crds(&["r6", "r7", "b7", "y3", "y7"]))
        );
    }

    #[test]
    fn test_cards_that_form_a_run() {
        assert_eq!(
            crds(&["r7", "r6"]),
            cards_that_form_a_run(&crds(&["r6", "r7", "b7", "y3", "y7"]))
        );
        assert_eq!(
            crds(&["y3", "g2", "b1"]),
            cards_that_form_a_run(&crds(&["r6", "b1", "r7", "g2", "b7", "y3", "y7"]))
        );
    }

    #[test]
    fn test_most_cards_below_4() {
        assert_eq!(
            crds(&["y3", "g2", "b1"]),
            most_cards_below_4(&crds(&["b1", "b4", "r6", "g2", "r7", "b7", "y3", "y7"]))
        );
    }

    #[test]
    fn test_leader() {
        let palettes = vec![crds(&["y5", "b2"]), crds(&["r5", "b2"]), crds(&["g6"])];
        let (leader_idx, leader_pal) = leader(&palettes);
        assert_eq!(1, leader_idx);
        assert_eq!(crds(&["r5", "b2"]), leader_pal);
    }

    #[test]
    fn test_game_render() {
        let (g, _) = Game::start(2, 0).unwrap();
        use brdgme_game::Renderer;
        let rendered = g.pub_state().render();
        assert!(!rendered.is_empty());
    }

    #[test]
    fn pub_state_does_not_leak_hidden_info() {
        let (g, _) = Game::start(2, 42).unwrap();
        let ps = g.pub_state();
        let json = serde_json::to_value(&ps).unwrap();
        assert!(
            json.get("hand").is_none(),
            "PubState must not contain a hand field"
        );
        assert!(
            json.get("hands").is_none(),
            "PubState must not contain a hands field"
        );
        assert!(
            json.get("deck").is_none(),
            "PubState must not contain a deck field"
        );
        assert!(
            json.get("rng").is_none(),
            "PubState must not contain an rng field"
        );
    }
}
