use std::fmt;

use serde::{Deserialize, Serialize};

mod command;
mod render;

use brdgme_color as color;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;
use rand::prelude::*;

use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 10;
pub const END_SCORE: i32 = 66;
const ROWS: usize = 4;
const ROW_MAX: usize = 5;
const HAND_SIZE: usize = 10;
const DECK_SIZE: u8 = 104;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Card(pub u8);

impl Card {
    pub fn heads(self) -> i32 {
        let c = self.0 as i32;
        if c == 55 {
            7
        } else if c % 11 == 0 {
            5
        } else if c % 10 == 0 {
            3
        } else if c % 5 == 0 {
            2
        } else {
            1
        }
    }

    pub fn color(self) -> color::NamedColor {
        match self.heads() {
            7 => color::NamedColor::Purple,
            5 => color::NamedColor::Red,
            3 => color::NamedColor::Yellow,
            2 => color::NamedColor::Cyan,
            _ => color::NamedColor::Grey,
        }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn deck() -> Vec<Card> {
    (1..=DECK_SIZE).map(Card).collect()
}

pub fn shuffle(mut cards: Vec<Card>, rng: &mut GameRng) -> Vec<Card> {
    cards.shuffle(rng);
    cards
}

pub fn sort_cards(cards: &mut [Card]) {
    cards.sort();
}

pub fn cards_heads(cards: &[Card]) -> i32 {
    cards.iter().map(|c| c.heads()).sum()
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub deck: Vec<Card>,
    pub discard: Vec<Card>,
    pub player_points: Vec<i32>,
    pub hands: Vec<Vec<Card>>,
    pub player_cards: Vec<Vec<Card>>,
    pub plays: Vec<Option<Card>>,
    pub board: [Vec<Card>; ROWS],
    pub resolving: bool,
    pub choose_player: usize,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players in the game.
    pub players: usize,
    /// The four rows of cards on the table, each row is ascending cards.
    pub board: [Vec<Card>; ROWS],
    /// Number of cards in each player's taken pile.
    pub player_cards_counts: Vec<usize>,
    /// Accumulated bullhead points per player (lower is better).
    pub player_points: Vec<i32>,
    /// Whether the game has ended.
    pub finished: bool,
    /// Final standings once finished (1 = winner).
    pub placings: Vec<usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// This player's seat index.
    pub player: usize,
    /// Cards in this player's hand (private until played).
    pub hand: Vec<Card>,
}

impl Game {
    pub fn can_play(&self, player: usize) -> bool {
        !self.resolving && self.plays.get(player) == Some(&None) && !self.is_finished()
    }

    pub fn can_choose(&self, player: usize) -> bool {
        self.resolving && self.choose_player == player
    }

    pub fn start_round(&mut self) -> Vec<Log> {
        for i in 0..ROWS {
            self.discard.append(&mut self.board[i]);
        }
        for p in 0..self.players {
            self.discard.append(&mut self.player_cards[p]);
        }
        for i in 0..ROWS {
            self.board[i] = self.draw_cards(1);
        }
        for p in 0..self.players {
            let mut hand = self.draw_cards(HAND_SIZE);
            hand.sort();
            self.hands[p] = hand;
        }
        vec![Log::public(vec![N::text(
            "Starting a new round, dealing 10 cards to each player",
        )])]
    }

    pub fn resolve_plays(&mut self) -> Vec<Log> {
        self.resolving = true;
        let mut logs: Vec<Log> = vec![];
        loop {
            let mut lowest_card: Option<Card> = None;
            let mut lowest_player: usize = 0;
            for p in 0..self.players {
                match self.plays[p] {
                    Some(c) => {
                        if lowest_card.is_none() || c < lowest_card.unwrap() {
                            lowest_card = Some(c);
                            lowest_player = p;
                        }
                    }
                    None => continue,
                }
            }
            let lowest_card = match lowest_card {
                Some(c) => c,
                None => break,
            };
            let mut closest_card: Option<Card> = None;
            let mut closest_row: usize = 0;
            for i in 0..ROWS {
                let last_card = *self.board[i].last().expect("row is never empty");
                if last_card < lowest_card
                    && (closest_card.is_none() || last_card > closest_card.unwrap())
                {
                    closest_card = Some(last_card);
                    closest_row = i;
                }
            }
            match closest_card {
                None => {
                    self.choose_player = lowest_player;
                    return logs;
                }
                Some(_) => {
                    if self.board[closest_row].len() == ROW_MAX {
                        logs.push(Log::public(vec![
                            N::Player(lowest_player),
                            N::text(format!(" played {} as card ", lowest_card)),
                            N::Bold(vec![N::text(format!(
                                "{}",
                                self.board[closest_row].len() + 1
                            ))]),
                            N::text(" of row "),
                            N::Bold(vec![N::text(format!("{}", closest_row + 1))]),
                            N::text(" and "),
                            N::Bold(vec![N::text(format!(
                                "took the row for {} points",
                                cards_heads(&self.board[closest_row])
                            ))]),
                        ]));
                        self.player_cards[lowest_player].append(&mut self.board[closest_row]);
                        self.board[closest_row] = vec![lowest_card];
                    } else {
                        logs.push(Log::public(vec![
                            N::Player(lowest_player),
                            N::text(format!(" played {} as card ", lowest_card)),
                            N::Bold(vec![N::text(format!(
                                "{}",
                                self.board[closest_row].len() + 1
                            ))]),
                            N::text(" of row "),
                            N::Bold(vec![N::text(format!("{}", closest_row + 1))]),
                        ]));
                        self.board[closest_row].push(lowest_card);
                    }
                }
            }
            self.plays[lowest_player] = None;
        }
        self.resolving = false;
        match self.hands[0].len() {
            0 => logs.extend(self.end_round()),
            1 => {
                for p in 0..self.players {
                    let card = self.hands[p][0];
                    let play_logs = self
                        .play(p, card)
                        .expect("auto-play should only play valid cards");
                    logs.extend(play_logs);
                }
            }
            _ => {}
        }
        logs
    }

    pub fn end_round(&mut self) -> Vec<Log> {
        let mut lines: Vec<N> = vec![N::Bold(vec![N::text("End of the round, counting points")])];
        for p in 0..self.players {
            let total: i32 = self.player_cards[p].iter().map(|c| c.heads()).sum();
            self.player_points[p] += total;
            lines.push(N::Group(vec![
                N::text("\n  "),
                N::Player(p),
                N::text(" had "),
                N::Bold(vec![N::text(self.player_cards[p].len().to_string())]),
                N::text(" cards worth "),
                N::Bold(vec![N::text(total.to_string())]),
                N::text(" points, total now "),
                N::Bold(vec![N::text(self.player_points[p].to_string())]),
            ]));
        }
        let logs = vec![Log::public(lines)];
        if !self.is_finished() {
            let mut all_logs = logs;
            all_logs.extend(self.start_round());
            all_logs
        } else {
            logs
        }
    }

    pub fn draw_cards(&mut self, n: usize) -> Vec<Card> {
        if self.deck.len() >= n {
            self.deck.drain(..n).collect()
        } else {
            let mut cards: Vec<Card> = self.deck.drain(..).collect();
            let remaining = n - cards.len();
            self.deck = shuffle(std::mem::take(&mut self.discard), &mut self.rng);
            cards.extend(self.draw_cards(remaining));
            cards
        }
    }

    pub fn play(&mut self, player: usize, card: Card) -> Result<Vec<Log>, GameError> {
        if !self.can_play(player) {
            return Err(GameError::invalid_input("you can't play at the moment"));
        }
        let hand = &mut self.hands[player];
        let idx = hand
            .iter()
            .position(|&c| c == card)
            .ok_or_else(|| GameError::invalid_input("you don't have that card"))?;
        hand.remove(idx);
        self.plays[player] = Some(card);
        for p in 0..self.players {
            if self.plays[p].is_none() {
                return Ok(vec![]);
            }
        }
        Ok(self.resolve_plays())
    }

    pub fn choose(&mut self, player: usize, row: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_choose(player) {
            return Err(GameError::invalid_input("you can't choose at the moment"));
        }
        if !(1..=ROWS).contains(&row) {
            return Err(GameError::invalid_input("the row must be between 1 and 4"));
        }
        let row = row - 1;
        let played = self.plays[player].expect("choosing player has a played card");
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" played {} and chose to take row ", played)),
            N::Bold(vec![N::text(format!("{}", row + 1))]),
            N::text(" for "),
            N::Bold(vec![N::text(format!(
                "{} points",
                cards_heads(&self.board[row])
            ))]),
        ])];
        self.player_cards[player].append(&mut self.board[row]);
        self.board[row] = vec![played];
        self.plays[player] = None;
        logs.extend(self.resolve_plays());
        Ok(logs)
    }

    pub fn is_finished(&self) -> bool {
        for p in 0..self.players {
            if !self.hands[p].is_empty() {
                return false;
            }
        }
        let highest = self.player_points.iter().copied().max().unwrap_or(0);
        highest >= END_SCORE
    }

    fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![-self.player_points[p]])
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
        let mut g = Game {
            players,
            deck: shuffle(deck(), &mut rng),
            player_points: vec![0; players],
            hands: vec![vec![]; players],
            player_cards: vec![vec![]; players],
            plays: vec![None; players],
            rng,
            ..Game::default()
        };
        let logs = g.start_round();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            Status::Finished {
                placings: self.placings(),
                stats: vec![],
            }
        } else if self.resolving {
            Status::Active {
                whose_turn: vec![self.choose_player],
                eliminated: vec![],
            }
        } else {
            let whose: Vec<usize> = (0..self.players)
                .filter(|&p| self.plays[p].is_none())
                .collect();
            Status::Active {
                whose_turn: whose,
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            players: self.players,
            board: self.board.clone(),
            player_cards_counts: self.player_cards.iter().map(|c| c.len()).collect(),
            player_points: self.player_points.clone(),
            finished: self.is_finished(),
            placings: if self.is_finished() {
                self.placings()
            } else {
                vec![]
            },
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
                remaining,
                value: Command::Play(card),
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
                value: Command::Choose(row),
                ..
            }) => {
                let logs = self.choose(player, row)?;
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
        self.player_points.iter().map(|&p| p as f32).collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4, 5, 6, 7, 8, 9, 10]
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

    fn players(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("player{}", i)).collect()
    }

    #[test]
    fn test_game_draw_cards() {
        let mut g = Game::start(2, 1).unwrap().0;
        g.discard = g.draw_cards(75);
        assert_eq!(75, g.discard.len());
        assert_eq!(5, g.deck.len());
        let drawn = g.draw_cards(10);
        assert_eq!(10, drawn.len());
        assert_eq!(0, g.discard.len());
        assert_eq!(70, g.deck.len());
    }

    #[test]
    fn test_auto_play_last_card() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [vec![Card(1)], vec![Card(2)], vec![Card(3)], vec![Card(4)]];
        g.hands[MICK] = vec![Card(5), Card(6)];
        g.hands[STEVE] = vec![Card(7), Card(8)];
        let p = players(2);
        g.command(MICK, "play 5", &p).unwrap();
        g.command(STEVE, "play 7", &p).unwrap();
        assert_eq!(10, g.hands[MICK].len());
    }

    #[test]
    fn test_sort_cards() {
        let mut cards = vec![Card(3), Card(2), Card(1)];
        sort_cards(&mut cards);
        assert_eq!(vec![Card(1), Card(2), Card(3)], cards);
    }

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2, 3, 4, 5, 6, 7, 8, 9, 10], Game::player_counts());
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(11, 1).is_err());
        assert!(Game::start(2, 1).is_ok());
        assert!(Game::start(10, 1).is_ok());
    }

    #[test]
    fn test_start_initial_state() {
        let (g, logs) = Game::start(3, 1).unwrap();
        assert_eq!(3, g.players);
        for p in 0..3 {
            assert_eq!(HAND_SIZE, g.hands[p].len());
            assert!(g.hands[p].windows(2).all(|w| w[0] <= w[1]));
            assert_eq!(0, g.player_cards[p].len());
            assert_eq!(0, g.player_points[p]);
            assert!(g.plays[p].is_none());
        }
        for i in 0..ROWS {
            assert_eq!(1, g.board[i].len());
        }
        assert_eq!(DECK_SIZE as usize - ROWS - HAND_SIZE * 3, g.deck.len());
        assert!(!g.resolving);
        assert!(!logs.is_empty());
    }

    #[test]
    fn test_card_heads() {
        assert_eq!(7, Card(55).heads());
        assert_eq!(5, Card(11).heads());
        assert_eq!(5, Card(22).heads());
        assert_eq!(5, Card(99).heads());
        assert_eq!(3, Card(10).heads());
        assert_eq!(3, Card(50).heads());
        assert_eq!(3, Card(100).heads());
        assert_eq!(2, Card(5).heads());
        assert_eq!(2, Card(15).heads());
        assert_eq!(2, Card(25).heads());
        assert_eq!(1, Card(1).heads());
        assert_eq!(1, Card(7).heads());
        assert_eq!(1, Card(104).heads());
        // 55 is a multiple of 11 and 5; 7 wins. 50 is a multiple of 10 and 5;
        // 10 wins (3 bulls). 11 is a multiple of 1 only; 5 wins.
    }

    #[test]
    fn test_card_colors() {
        assert_eq!(color::NamedColor::Purple, Card(55).color());
        assert_eq!(color::NamedColor::Red, Card(11).color());
        assert_eq!(color::NamedColor::Yellow, Card(10).color());
        assert_eq!(color::NamedColor::Cyan, Card(5).color());
        assert_eq!(color::NamedColor::Grey, Card(7).color());
    }

    #[test]
    fn test_can_play_and_can_choose() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.can_play(0));
        assert!(g.can_play(1));
        assert!(!g.can_choose(0));
        assert!(!g.can_choose(1));
        g.resolving = true;
        g.choose_player = 0;
        assert!(!g.can_play(0));
        assert!(g.can_choose(0));
        assert!(!g.can_choose(1));
    }

    #[test]
    fn test_play_command_resolves_when_all_played() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [vec![Card(1)], vec![Card(2)], vec![Card(3)], vec![Card(4)]];
        // 3 cards each so the resolve doesn't cascade into auto-play/end_round.
        g.hands[MICK] = vec![Card(5), Card(6), Card(100)];
        g.hands[STEVE] = vec![Card(7), Card(8), Card(101)];
        let p = players(2);
        let resp = g.command(MICK, "play 6", &p).unwrap();
        // Only Mick played, no resolve yet.
        assert!(resp.logs.is_empty());
        assert!(g.plays[MICK] == Some(Card(6)));
        assert!(g.plays[STEVE].is_none());
        let resp = g.command(STEVE, "play 8", &p).unwrap();
        // Both played, resolved: 6 -> row 4 (last=4 is highest below 6),
        // 8 -> row 4 (last=6 is highest below 8).
        assert!(!resp.logs.is_empty());
        assert!(g.plays[MICK].is_none());
        assert!(g.plays[STEVE].is_none());
        assert_eq!(vec![Card(4), Card(6), Card(8)], g.board[3]);
    }

    #[test]
    fn test_play_wrong_player_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.hands[MICK] = vec![Card(5), Card(6)];
        g.hands[STEVE] = vec![Card(7), Card(8)];
        let p = players(2);
        g.command(MICK, "play 5", &p).unwrap();
        // Mick has already played, playing again errors.
        assert!(g.command(MICK, "play 6", &p).is_err());
    }

    #[test]
    fn test_play_card_not_in_hand_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.hands[MICK] = vec![Card(5), Card(6)];
        let p = players(2);
        // Card 99 is not in Mick's hand but parses fine; play() rejects it.
        assert!(g.command(MICK, "play 99", &p).is_err());
    }

    #[test]
    fn test_choose_command_when_card_below_all_rows() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [
            vec![Card(20)],
            vec![Card(30)],
            vec![Card(40)],
            vec![Card(50)],
        ];
        // Give 3 cards each so the resolve after choose doesn't trigger
        // auto-play (len 1) or end_round (len 0) and clear player_cards.
        g.hands[MICK] = vec![Card(5), Card(100), Card(101)];
        g.hands[STEVE] = vec![Card(25), Card(102), Card(103)];
        let p = players(2);
        g.command(MICK, "play 5", &p).unwrap();
        g.command(STEVE, "play 25", &p).unwrap();
        // Resolve: 5 is below all rows -> Mick chooses. 25 still pending.
        assert!(g.resolving);
        assert_eq!(MICK, g.choose_player);
        // Mick can choose, Steve can't.
        assert!(g.command(STEVE, "choose 2", &p).is_err());
        let resp = g.command(MICK, "choose 2", &p).unwrap();
        // Mick took row 2 ([30]), placed 5 as new start. Then 25 resolves:
        // row ends are 20, 5, 40, 50; 20 is the highest below 25, so 25 goes
        // into row 1 (index 0).
        assert!(!resp.logs.is_empty());
        assert!(!g.resolving);
        assert_eq!(vec![Card(30)], g.player_cards[MICK]);
        assert_eq!(vec![Card(20), Card(25)], g.board[0]);
        assert_eq!(vec![Card(5)], g.board[1]);
    }

    #[test]
    fn test_choose_row_validation() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [
            vec![Card(20)],
            vec![Card(30)],
            vec![Card(40)],
            vec![Card(50)],
        ];
        g.hands[MICK] = vec![Card(5)];
        g.hands[STEVE] = vec![Card(25)];
        let p = players(2);
        g.command(MICK, "play 5", &p).unwrap();
        g.command(STEVE, "play 25", &p).unwrap();
        assert!(g.command(MICK, "choose 0", &p).is_err());
        assert!(g.command(MICK, "choose 5", &p).is_err());
    }

    #[test]
    fn test_row_full_takes_row() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [
            vec![Card(1), Card(2), Card(3), Card(4), Card(5)],
            vec![Card(10)],
            vec![Card(20)],
            vec![Card(30)],
        ];
        // 3 cards each so the resolve doesn't cascade into end_round.
        g.hands[MICK] = vec![Card(6), Card(100), Card(101)];
        g.hands[STEVE] = vec![Card(7), Card(102), Card(103)];
        let p = players(2);
        g.command(MICK, "play 6", &p).unwrap();
        let resp = g.command(STEVE, "play 7", &p).unwrap();
        // 6 goes into row 1 (5<6, row full at 5) -> Mick takes 5 cards,
        // row 1 becomes [6]. Then 7 -> row 1 (6<7).
        assert!(!resp.logs.is_empty());
        assert_eq!(5, g.player_cards[MICK].len());
        assert_eq!(vec![Card(6), Card(7)], g.board[0]);
    }

    #[test]
    fn test_end_round_adds_points_and_starts_new_round() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.board = [vec![Card(1)], vec![Card(2)], vec![Card(3)], vec![Card(4)]];
        g.hands[MICK] = vec![Card(5)];
        g.hands[STEVE] = vec![Card(7)];
        g.player_cards[MICK] = vec![Card(10), Card(11)];
        g.player_points = vec![10, 20];
        let p = players(2);
        // Both play -> resolve -> hands empty -> end_round -> start_round.
        g.command(MICK, "play 5", &p).unwrap();
        g.command(STEVE, "play 7", &p).unwrap();
        // end_round added heads: Mick had 10(3) + 11(5) = 8, plus the card from
        // this resolve if any. Steve had 0 taken before. Just check points grew
        // and a new round was dealt.
        assert!(g.player_points[MICK] > 10);
        assert_eq!(HAND_SIZE, g.hands[MICK].len());
        assert_eq!(HAND_SIZE, g.hands[STEVE].len());
    }

    #[test]
    fn test_finished_at_threshold() {
        let mut g = Game::start(2, 1).unwrap().0;
        g.player_points = vec![END_SCORE, 10];
        // Hands empty and highest >= END_SCORE -> finished.
        g.hands = vec![vec![], vec![]];
        assert!(g.is_finished());
        g.player_points = vec![END_SCORE - 1, 10];
        assert!(!g.is_finished());
        // Hands not empty -> not finished even if score high.
        g.player_points = vec![END_SCORE, 10];
        g.hands = vec![vec![Card(1)], vec![]];
        assert!(!g.is_finished());
    }

    #[test]
    fn test_placings_standard_competition_ties() {
        // category_5 is lowest-score-wins (fewest bullheads), so lower
        // player_points ranks higher (1st).
        let mut g = Game::start(3, 1).unwrap().0;
        g.player_points = vec![10, 10, 5];
        assert_eq!(vec![2, 2, 1], g.placings());
        g.player_points = vec![10, 5, 10];
        assert_eq!(vec![2, 1, 2], g.placings());
        g.player_points = vec![5, 10, 10];
        assert_eq!(vec![1, 2, 2], g.placings());
        g.player_points = vec![10, 10, 10];
        assert_eq!(vec![1, 1, 1], g.placings());
        g.player_points = vec![5, 10, 7];
        assert_eq!(vec![1, 3, 2], g.placings());
    }

    #[test]
    fn test_command_unknown_input_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let p = players(2);
        assert!(g.command(0, "fly", &p).is_err());
    }

    #[test]
    fn test_command_after_finished_errors() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.hands = vec![vec![], vec![]];
        g.player_points = vec![END_SCORE, 10];
        let p = players(2);
        assert!(g.command(0, "play 5", &p).is_err());
        assert!(g.command(0, "choose 1", &p).is_err());
    }

    #[test]
    fn test_pub_state_captures_rendered_fields() {
        let g = Game::start(3, 1).unwrap().0;
        let ps = g.pub_state();
        assert_eq!(g.players, ps.players);
        assert_eq!(g.board, ps.board);
        assert_eq!(
            g.player_cards.iter().map(|c| c.len()).collect::<Vec<_>>(),
            ps.player_cards_counts
        );
        assert_eq!(g.player_points, ps.player_points);
        assert!(!ps.finished);
        assert!(ps.placings.is_empty());
    }

    #[test]
    fn test_player_state_includes_hand() {
        let g = Game::start(2, 1).unwrap().0;
        let ps = g.player_state(0);
        assert_eq!(g.hands[0], ps.hand);
        assert_eq!(0, ps.player);
    }
}
