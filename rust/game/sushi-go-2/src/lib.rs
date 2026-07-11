use std::collections::HashSet;

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
use rand::prelude::*;

use command::Command;

const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 5;
pub const DUMMY: usize = 2;
const TOTAL_ROUNDS: usize = 3;

#[derive(
    Default, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum Card {
    #[default]
    Played,
    Tempura,
    Sashimi,
    Dumpling,
    MakiRoll3,
    MakiRoll2,
    MakiRoll1,
    SalmonNigiri,
    SquidNigiri,
    EggNigiri,
    Pudding,
    Wasabi,
    Chopsticks,
}

impl Card {
    pub fn all() -> &'static [Card] {
        &[
            Card::Tempura,
            Card::Sashimi,
            Card::Dumpling,
            Card::MakiRoll3,
            Card::MakiRoll2,
            Card::MakiRoll1,
            Card::SalmonNigiri,
            Card::SquidNigiri,
            Card::EggNigiri,
            Card::Pudding,
            Card::Wasabi,
            Card::Chopsticks,
        ]
    }

    pub fn count(self) -> usize {
        match self {
            Card::Tempura => 14,
            Card::Sashimi => 14,
            Card::Dumpling => 14,
            Card::MakiRoll3 => 8,
            Card::MakiRoll2 => 12,
            Card::MakiRoll1 => 6,
            Card::SalmonNigiri => 10,
            Card::SquidNigiri => 5,
            Card::EggNigiri => 5,
            Card::Pudding => 10,
            Card::Wasabi => 6,
            Card::Chopsticks => 4,
            Card::Played => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Card::Played => "played",
            Card::Tempura => "tempura",
            Card::Sashimi => "sashimi",
            Card::Dumpling => "dumpling",
            Card::MakiRoll3 => "maki x3",
            Card::MakiRoll2 => "maki x2",
            Card::MakiRoll1 => "maki x1",
            Card::SalmonNigiri => "salmon nigiri",
            Card::SquidNigiri => "squid nigiri",
            Card::EggNigiri => "egg nigiri",
            Card::Pudding => "pudding",
            Card::Wasabi => "wasabi",
            Card::Chopsticks => "chopsticks",
        }
    }

    pub fn color(self) -> brdgme_color::Color {
        use brdgme_color::*;
        match self {
            Card::Played => GREY,
            Card::Tempura => PURPLE,
            Card::Sashimi => PURPLE,
            Card::Dumpling => YELLOW,
            Card::MakiRoll3 | Card::MakiRoll2 | Card::MakiRoll1 => RED,
            Card::SalmonNigiri | Card::SquidNigiri | Card::EggNigiri => CYAN,
            Card::Pudding => BLUE,
            Card::Wasabi => GREEN,
            Card::Chopsticks => BLACK,
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            Card::Tempura => "x2 = 5",
            Card::Sashimi => "x3 = 10",
            Card::Dumpling => "1 3 6 10 15",
            Card::MakiRoll3 | Card::MakiRoll2 | Card::MakiRoll1 => "most: 6/3",
            Card::SalmonNigiri => "2",
            Card::SquidNigiri => "3",
            Card::EggNigiri => "1",
            Card::Pudding => "end: most 6, least -6",
            Card::Wasabi => "next nigiri x3",
            Card::Chopsticks => "swap for 2",
            Card::Played => "",
        }
    }

    pub fn base_score(self) -> Option<i32> {
        match self {
            Card::SalmonNigiri => Some(2),
            Card::SquidNigiri => Some(3),
            Card::EggNigiri => Some(1),
            _ => None,
        }
    }
}

fn player_draw_counts() -> &'static [(usize, usize)] {
    &[(2, 9), (3, 9), (4, 8), (5, 7)]
}

fn draw_count(players: usize) -> usize {
    player_draw_counts()
        .iter()
        .find(|(p, _)| *p == players)
        .map(|(_, c)| *c)
        .unwrap_or(9)
}

fn deck() -> Vec<Card> {
    let mut d = vec![];
    for &c in Card::all() {
        for _ in 0..c.count() {
            d.push(c);
        }
    }
    d
}

fn sort_cards(mut cards: Vec<Card>) -> Vec<Card> {
    cards.sort();
    cards
}

fn trim_played(cards: &[Card]) -> Vec<Card> {
    cards
        .iter()
        .filter(|&&c| c != Card::Played)
        .copied()
        .collect()
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub all_players: usize,
    pub round: usize,
    pub deck: Vec<Card>,
    pub hands: Vec<Vec<Card>>,
    pub playing: Vec<Option<Vec<Card>>>,
    pub played: Vec<Vec<Card>>,
    pub player_points: Vec<i32>,
    pub controller: usize,
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PubState {
    pub players: usize,
    pub all_players: usize,
    pub round: usize,
    pub controller: usize,
    pub played: Vec<Vec<Card>>,
    pub player_points: Vec<i32>,
    pub finished: bool,
    pub final_scores: Vec<i32>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub hand: Vec<Card>,
    pub playing: Option<Vec<Card>>,
    pub dummy_playing: Option<Vec<Card>>,
}

impl Game {
    pub fn is_finished(&self) -> bool {
        self.round == TOTAL_ROUNDS
            && !self.hands.is_empty()
            && self.hands[0].is_empty()
            && self.playing[0].is_none()
    }

    pub fn whose_turn_inner(&self) -> Vec<usize> {
        if self.is_finished() {
            return vec![];
        }
        (0..self.players)
            .filter(|&p| self.can_play(p) || self.can_dummy(p))
            .collect()
    }

    pub fn can_play(&self, player: usize) -> bool {
        player < self.playing.len() && self.playing[player].is_none()
    }

    pub fn can_dummy(&self, player: usize) -> bool {
        self.players == 2 && self.controller == player && self.playing[DUMMY].is_none()
    }

    pub fn render_name(&self, player: usize) -> N {
        if player > self.players - 1 {
            N::Fg(
                brdgme_color::GREY.into(),
                vec![N::Bold(vec![N::text("<dummy>")])],
            )
        } else {
            N::Player(player)
        }
    }

    pub fn render_names(&self, players: &[usize]) -> Vec<N> {
        players.iter().map(|&p| self.render_name(p)).collect()
    }

    pub fn pudding_cards(&self, player: usize) -> i32 {
        self.played[player]
            .iter()
            .filter(|&&c| c == Card::Pudding)
            .count() as i32
    }

    pub fn placings(&self) -> Vec<usize> {
        let metrics: Vec<Vec<i32>> = (0..self.players)
            .map(|p| vec![self.player_points[p], self.pudding_cards(p)])
            .collect();
        gen_placings(&metrics)
    }

    pub fn start_round(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        self.round += 1;
        for p in 0..self.all_players {
            let new_played: Vec<Card> = self.played[p]
                .iter()
                .filter(|&&c| c == Card::Pudding)
                .copied()
                .collect();
            self.played[p] = new_played;
        }
        self.hands = vec![vec![]; self.all_players];
        let dc = draw_count(self.all_players);
        let pass_dir = if self.round == 2 { "right" } else { "left" };
        logs.push(Log::public(vec![
            N::text("Starting round "),
            N::Bold(vec![N::text(self.round.to_string())]),
            N::text(", hands will be passed to the "),
            N::Bold(vec![N::text(pass_dir)]),
            N::text(".  Dealing "),
            N::Bold(vec![N::text(dc.to_string())]),
            N::text(" cards to each player"),
        ]));
        for p in 0..self.all_players {
            let hand: Vec<Card> = self.deck.drain(0..dc).collect();
            self.hands[p] = sort_cards(hand);
        }
        logs.extend(self.start_hand());
        logs
    }

    pub fn start_hand(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        if self.players == 2 && !self.hands[DUMMY].is_empty() {
            let i = self.rng.random_range(0..self.hands[DUMMY].len());
            let drawn = self.hands[DUMMY][i];
            logs.push(Log::private(
                vec![
                    N::text("You drew "),
                    render::card(drawn),
                    N::text(" from "),
                    self.render_name(DUMMY),
                ],
                vec![self.controller],
            ));
            self.hands[self.controller].push(drawn);
            self.hands[self.controller] = sort_cards(self.hands[self.controller].clone());
            self.hands[DUMMY].remove(i);
        }
        logs
    }

    pub fn end_hand(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        for p in 0..self.all_players {
            self.hands[p] = trim_played(&self.hands[p]);
            let playing = self.playing[p].clone().unwrap_or_default();
            self.played[p].extend(&playing);
            logs.push(Log::public(vec![
                self.render_name(p),
                N::text(" played "),
                render::cards_list(&playing),
            ]));
            if playing.len() == 2
                && let Some(i) = self.played[p].iter().position(|&c| c == Card::Chopsticks)
            {
                self.hands[p].push(Card::Chopsticks);
                self.played[p].remove(i);
            }
            self.playing[p] = None;
        }
        if self.players == 2 {
            self.controller = (self.controller + 1) % self.players;
        }
        if self.hands[0].is_empty() {
            logs.extend(self.end_round());
            return logs;
        }
        if self.players == 2 {
            logs.push(Log::public(vec![N::text("Players are swapping hands")]));
            self.hands.swap(0, 1);
        } else if self.round % 2 == 1 {
            logs.push(Log::public(vec![
                N::text("Passing hands to the "),
                N::Bold(vec![N::text("left")]),
            ]));
            self.hands.rotate_left(1);
        } else {
            logs.push(Log::public(vec![
                N::text("Passing hands to the "),
                N::Bold(vec![N::text("right")]),
            ]));
            self.hands.rotate_right(1);
        }
        logs.extend(self.start_hand());
        logs
    }

    pub fn score(&self) -> (Vec<i32>, Vec<Vec<N>>) {
        let mut scores = vec![0i32; self.all_players];
        let mut output: Vec<Vec<N>> = vec![];

        // Score maki
        let mut maki = vec![0i32; self.all_players];
        for (p, maki_p) in maki.iter_mut().enumerate() {
            for &c in &self.played[p] {
                match c {
                    Card::MakiRoll1 => *maki_p += 1,
                    Card::MakiRoll2 => *maki_p += 2,
                    Card::MakiRoll3 => *maki_p += 3,
                    _ => {}
                }
            }
        }
        let mut first = 0i32;
        let mut first_players: Vec<usize> = vec![];
        let mut second = 0i32;
        let mut second_players: Vec<usize> = vec![];
        for (p, &m) in maki.iter().enumerate() {
            if m > first {
                second = first;
                second_players = first_players.clone();
                first = m;
                first_players = vec![];
            }
            if m == first {
                first_players.push(p);
            } else if m > second {
                second = m;
                second_players = vec![];
            }
            if m != first && m == second {
                second_players.push(p);
            }
        }
        let maki_str = N::Fg(
            brdgme_color::RED.into(),
            vec![N::Bold(vec![N::text("maki rolls")])],
        );
        if first == 0 {
            output.push(vec![
                N::text("Nobody had "),
                maki_str,
                N::text(", no points awarded"),
            ]);
        } else {
            let first_points = 6 / first_players.len() as i32;
            output.push(vec![
                render::comma_list_nodes(self.render_names(&first_players)),
                N::text(" had "),
                N::Bold(vec![N::text(first.to_string())]),
                N::text(" "),
                maki_str.clone(),
                N::text(", awarding "),
                N::Bold(vec![N::text(first_points.to_string())]),
                N::text(" points"),
            ]);
            for &p in &first_players {
                scores[p] += first_points;
            }
            if first_players.len() == 1 && second > 0 && second_players.len() <= 3 {
                let second_points = 3 / second_players.len() as i32;
                output.push(vec![
                    render::comma_list_nodes(self.render_names(&second_players)),
                    N::text(" had "),
                    N::Bold(vec![N::text(second.to_string())]),
                    N::text(" "),
                    maki_str,
                    N::text(", awarding "),
                    N::Bold(vec![N::text(second_points.to_string())]),
                    N::text(" points"),
                ]);
                for &p in &second_players {
                    scores[p] += second_points;
                }
            }
        }

        // Score puddings (round 3 only)
        if self.round == TOTAL_ROUNDS {
            let mut pudding = vec![0i32; self.all_players];
            for (p, pudding_p) in pudding.iter_mut().enumerate() {
                for &c in &self.played[p] {
                    if c == Card::Pudding {
                        *pudding_p += 1;
                    }
                }
            }
            let mut first = 0i32;
            let mut first_players: Vec<usize> = vec![];
            let mut last = 0i32;
            let mut last_players: Vec<usize> = vec![];
            for (p, &c) in pudding.iter().enumerate() {
                if c > first {
                    first = c;
                    first_players = vec![];
                }
                if c == first {
                    first_players.push(p);
                }
                if c < last || last_players.is_empty() {
                    last = c;
                    last_players = vec![];
                }
                if c == last {
                    last_players.push(p);
                }
            }
            let puddings_str = N::Fg(
                brdgme_color::BLUE.into(),
                vec![N::Bold(vec![N::text("puddings")])],
            );
            if first == last {
                output.push(vec![
                    N::text("Everybody had the same number of "),
                    puddings_str,
                    N::text(", no points awarded"),
                ]);
            } else {
                let first_points = 6 / first_players.len() as i32;
                output.push(vec![
                    render::comma_list_nodes(self.render_names(&first_players)),
                    N::text(" had "),
                    N::Bold(vec![N::text(first.to_string())]),
                    N::text(" "),
                    puddings_str.clone(),
                    N::text(", awarding "),
                    N::Bold(vec![N::text(first_points.to_string())]),
                    N::text(" points"),
                ]);
                for &p in &first_players {
                    scores[p] += first_points;
                }
                if self.players != 2 {
                    let last_points = -6 / last_players.len() as i32;
                    output.push(vec![
                        render::comma_list_nodes(self.render_names(&last_players)),
                        N::text(" had "),
                        N::Bold(vec![N::text(last.to_string())]),
                        N::text(" "),
                        puddings_str,
                        N::text(", awarding "),
                        N::Bold(vec![N::text(last_points.to_string())]),
                        N::text(" points"),
                    ]);
                    for &p in &last_players {
                        scores[p] += last_points;
                    }
                }
            }
        }

        // Score normal cards
        for (p, score_p) in scores.iter_mut().enumerate() {
            output.push(vec![N::Bold(vec![
                N::text("Scoring cards for "),
                self.render_name(p),
            ])]);
            let mut counts: Vec<(Card, i32)> = vec![];
            for &c in &self.played[p] {
                if let Some(s) = c.base_score() {
                    let mut s = s;
                    let wasabi_count = counts
                        .iter()
                        .find(|(card, _)| *card == Card::Wasabi)
                        .map(|(_, n)| *n)
                        .unwrap_or(0);
                    if wasabi_count > 0 {
                        s *= 3;
                        if let Some(entry) =
                            counts.iter_mut().find(|(card, _)| *card == Card::Wasabi)
                        {
                            entry.1 -= 1;
                        }
                        output.push(vec![
                            render::card(c),
                            N::text(" + "),
                            render::card(Card::Wasabi),
                            N::text(", "),
                            N::Bold(vec![N::text(s.to_string())]),
                            N::text(" points"),
                        ]);
                    } else {
                        output.push(vec![
                            render::card(c),
                            N::text(", "),
                            N::Bold(vec![N::text(s.to_string())]),
                            N::text(" points"),
                        ]);
                    }
                    *score_p += s;
                } else {
                    if let Some(entry) = counts.iter_mut().find(|(card, _)| *card == c) {
                        entry.1 += 1;
                    } else {
                        counts.push((c, 1));
                    }
                }
            }
            let get = |cc: &[(Card, i32)], c: Card| {
                cc.iter()
                    .find(|(card, _)| *card == c)
                    .map(|(_, n)| *n)
                    .unwrap_or(0)
            };
            let tempura_count = get(&counts, Card::Tempura);
            let s = tempura_count / 2 * 5;
            if s > 0 {
                output.push(vec![
                    N::text(format!("{} x ", tempura_count)),
                    render::card(Card::Tempura),
                    N::text(", "),
                    N::Bold(vec![N::text(s.to_string())]),
                    N::text(" points"),
                ]);
                *score_p += s;
            }
            let sashimi_count = get(&counts, Card::Sashimi);
            let s = sashimi_count / 3 * 10;
            if s > 0 {
                output.push(vec![
                    N::text(format!("{} x ", sashimi_count)),
                    render::card(Card::Sashimi),
                    N::text(", "),
                    N::Bold(vec![N::text(s.to_string())]),
                    N::text(" points"),
                ]);
                *score_p += s;
            }
            let dumpling_count = get(&counts, Card::Dumpling);
            if dumpling_count > 0 {
                let mut s = (dumpling_count * dumpling_count + dumpling_count) / 2;
                if s > 15 {
                    s = 15;
                }
                output.push(vec![
                    N::text(format!("{} x ", dumpling_count)),
                    render::card(Card::Dumpling),
                    N::text(", "),
                    N::Bold(vec![N::text(s.to_string())]),
                    N::text(" points"),
                ]);
                *score_p += s;
            }
        }
        (scores, output)
    }

    pub fn end_round(&mut self) -> Vec<Log> {
        let (scores, mut output) = self.score();
        output.push(vec![N::Bold(vec![N::text(
            "The scores after this round are:",
        )])]);
        for (p, &s) in scores.iter().enumerate() {
            self.player_points[p] += s;
            output.push(vec![
                self.render_name(p),
                N::text(": "),
                N::Bold(vec![N::text(self.player_points[p].to_string())]),
                N::text(" points"),
            ]);
        }
        let mut content: Vec<N> = vec![N::Bold(vec![N::text(format!(
            "It is the end of round {}, scoring\n",
            self.round
        ))])];
        for (i, line) in output.iter().enumerate() {
            if i > 0 {
                content.push(N::text("\n"));
            }
            content.extend(line.clone());
        }
        let mut logs = vec![Log::public(content)];
        if self.round < TOTAL_ROUNDS {
            logs.extend(self.start_round());
        }
        logs
    }

    pub fn play(&mut self, player: usize, cards: Vec<usize>) -> Result<Vec<Log>, GameError> {
        if !self.can_play(player) {
            return Err(GameError::invalid_input("you can't play at the moment"));
        }
        let l = cards.len();
        if l == 0 || l > 2 {
            return Err(GameError::invalid_input(
                "you must specify one or two cards to play",
            ));
        }
        if l == 2 {
            if !self.played[player].contains(&Card::Chopsticks) {
                return Err(GameError::invalid_input(
                    "you can only play a second card if you've previously played chopsticks",
                ));
            }
            if player == self.controller
                && self.playing[DUMMY].is_none()
                && self.players == 2
                && self.hands[player].len() == 2
            {
                return Err(GameError::invalid_input(
                    "you can't play two cards now, you have to save one for the dummy player",
                ));
            }
        }
        self.play_cards(player, player, &cards)
    }

    pub fn dummy(&mut self, player: usize, card: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_dummy(player) {
            return Err(GameError::invalid_input("you can't dummy at the moment"));
        }
        self.play_cards(DUMMY, player, &[card])
    }

    fn play_cards(
        &mut self,
        to_player: usize,
        from_player: usize,
        cards: &[usize],
    ) -> Result<Vec<Log>, GameError> {
        let mut card_map: HashSet<usize> = HashSet::new();
        for &c in cards {
            if c >= self.hands[from_player].len() {
                return Err(GameError::invalid_input("that card number is not valid"));
            }
            if card_map.contains(&c) {
                return Err(GameError::invalid_input("please specify different cards"));
            }
            if self.hands[from_player][c] == Card::Played {
                return Err(GameError::invalid_input(
                    "that card has already been played",
                ));
            }
            card_map.insert(c);
        }
        let played_cards: Vec<Card> = cards.iter().map(|&c| self.hands[from_player][c]).collect();
        for &c in cards {
            self.hands[from_player][c] = Card::Played;
        }
        self.playing[to_player] = Some(played_cards);
        for p in 0..self.all_players {
            if self.playing[p].is_none() {
                return Ok(vec![]);
            }
        }
        Ok(self.end_hand())
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
            all_players: if players == 2 { players + 1 } else { players },
            round: 0,
            deck: deck(),
            hands: vec![],
            playing: vec![],
            played: vec![],
            player_points: vec![],
            controller: 0,
            rng: GameRng::seed_from_u64(seed),
        };
        g.deck.shuffle(&mut g.rng);
        g.playing = vec![None; g.all_players];
        g.played = vec![vec![]; g.all_players];
        g.player_points = vec![0; g.all_players];
        let mut logs = vec![];
        if players == 2 {
            logs.push(Log::public(vec![
                N::text("Because there are only two players, you will be joined by "),
                g.render_name(DUMMY),
            ]));
        }
        logs.extend(g.start_round());
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.is_finished() {
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
        let finished = self.is_finished();
        PubState {
            players: self.players,
            all_players: self.all_players,
            round: self.round,
            controller: self.controller,
            played: self.played.clone(),
            player_points: self.player_points.clone(),
            finished,
            final_scores: if finished {
                (0..self.players).map(|p| self.player_points[p]).collect()
            } else {
                vec![]
            },
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: if player < self.hands.len() {
                self.hands[player].clone()
            } else {
                vec![]
            },
            playing: if player < self.playing.len() {
                self.playing[player].clone()
            } else {
                None
            },
            dummy_playing: if self.players == 2 && player == self.controller {
                self.playing[DUMMY].clone()
            } else {
                None
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
                value: Command::Play(cards),
                ..
            }) => {
                let logs = self.play(player, cards)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Dummy(card),
                ..
            }) => {
                let logs = self.dummy(player, card)?;
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
        (0..self.players)
            .map(|p| self.player_points[p] as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![2, 3, 4, 5]
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

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const BJ: usize = 2;

    fn names() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string(), "BJ".to_string()]
    }

    // --- 1:1 Go test ports (game_test.go) ---

    #[test]
    fn test_game_start() {
        let g = Game::start(2, 1);
        assert!(g.is_ok());
    }

    #[test]
    fn test_game_score_maki() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);

        g.played[MICK] = vec![Card::MakiRoll1];
        let (score, _) = g.score();
        assert_eq!(vec![6, 0, 0], score);

        g.played[STEVE] = vec![Card::MakiRoll1];
        let (score, _) = g.score();
        assert_eq!(vec![3, 3, 0], score);

        g.played[BJ] = vec![Card::MakiRoll1];
        let (score, _) = g.score();
        assert_eq!(vec![2, 2, 2], score);

        g.played[STEVE] = vec![Card::MakiRoll2];
        let (score, _) = g.score();
        assert_eq!(vec![1, 6, 1], score);
    }

    #[test]
    fn test_game_score_pudding() {
        let (mut g, _) = Game::start(3, 1).unwrap();

        g.played[MICK] = vec![Card::Pudding];
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);

        g.round = 3;
        let (score, _) = g.score();
        assert_eq!(vec![6, -3, -3], score);

        g.played[BJ] = vec![Card::Pudding, Card::Pudding];
        let (score, _) = g.score();
        assert_eq!(vec![0, -6, 6], score);

        g.played[MICK] = vec![Card::Pudding, Card::Pudding];
        g.played[STEVE] = vec![Card::Pudding, Card::Pudding];
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);
    }

    #[test]
    fn test_game_score_nigiri() {
        let (mut g, _) = Game::start(3, 1).unwrap();

        g.played[MICK] = vec![Card::EggNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![1, 0, 0], score);

        g.played[MICK] = vec![Card::EggNigiri, Card::Wasabi];
        let (score, _) = g.score();
        assert_eq!(vec![1, 0, 0], score);

        g.played[MICK] = vec![Card::Wasabi, Card::EggNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![3, 0, 0], score);

        g.played[STEVE] = vec![Card::SalmonNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![3, 2, 0], score);

        g.played[STEVE] = vec![Card::Wasabi, Card::SalmonNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![3, 6, 0], score);

        g.played[BJ] = vec![Card::SquidNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![3, 6, 3], score);

        g.played[BJ] = vec![Card::Wasabi, Card::SquidNigiri];
        let (score, _) = g.score();
        assert_eq!(vec![3, 6, 9], score);
    }

    #[test]
    fn test_game_score_tempura() {
        let (mut g, _) = Game::start(3, 1).unwrap();

        g.played[MICK] = vec![Card::Tempura];
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);

        g.played[MICK] = vec![Card::Tempura, Card::Tempura];
        let (score, _) = g.score();
        assert_eq!(vec![5, 0, 0], score);

        g.played[MICK] = vec![Card::Tempura, Card::Tempura, Card::Tempura];
        let (score, _) = g.score();
        assert_eq!(vec![5, 0, 0], score);

        g.played[MICK] = vec![Card::Tempura, Card::Tempura, Card::Tempura, Card::Tempura];
        let (score, _) = g.score();
        assert_eq!(vec![10, 0, 0], score);
    }

    #[test]
    fn test_game_score_sashimi() {
        let (mut g, _) = Game::start(3, 1).unwrap();

        g.played[MICK] = vec![Card::Sashimi];
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);

        g.played[MICK] = vec![Card::Sashimi, Card::Sashimi];
        let (score, _) = g.score();
        assert_eq!(vec![0, 0, 0], score);

        g.played[MICK] = vec![Card::Sashimi, Card::Sashimi, Card::Sashimi];
        let (score, _) = g.score();
        assert_eq!(vec![10, 0, 0], score);

        g.played[MICK] = vec![Card::Sashimi, Card::Sashimi, Card::Sashimi, Card::Sashimi];
        let (score, _) = g.score();
        assert_eq!(vec![10, 0, 0], score);
    }

    #[test]
    fn test_game_score_dumpling() {
        let (mut g, _) = Game::start(3, 1).unwrap();

        g.played[MICK] = vec![Card::Dumpling];
        let (score, _) = g.score();
        assert_eq!(vec![1, 0, 0], score);

        g.played[MICK] = vec![Card::Dumpling, Card::Dumpling];
        let (score, _) = g.score();
        assert_eq!(vec![3, 0, 0], score);

        g.played[MICK] = vec![Card::Dumpling, Card::Dumpling, Card::Dumpling];
        let (score, _) = g.score();
        assert_eq!(vec![6, 0, 0], score);

        g.played[MICK] = vec![
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
        ];
        let (score, _) = g.score();
        assert_eq!(vec![10, 0, 0], score);

        g.played[MICK] = vec![
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
        ];
        let (score, _) = g.score();
        assert_eq!(vec![15, 0, 0], score);

        g.played[MICK] = vec![
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
            Card::Dumpling,
        ];
        let (score, _) = g.score();
        assert_eq!(vec![15, 0, 0], score);
    }

    // --- 1:1 Go test ports (deck_test.go) ---

    #[test]
    fn test_deck() {
        let d = deck();
        assert_eq!(108, d.len());
    }

    #[test]
    fn test_sort() {
        let d = vec![Card::SquidNigiri, Card::Sashimi, Card::MakiRoll1];
        let sorted = sort_cards(d.clone());
        assert_eq!(
            vec![Card::Sashimi, Card::MakiRoll1, Card::SquidNigiri],
            sorted
        );
        assert_eq!(vec![Card::SquidNigiri, Card::Sashimi, Card::MakiRoll1], d);
    }

    #[test]
    fn test_shuffle() {
        let d = vec![Card::SquidNigiri, Card::Sashimi, Card::MakiRoll1];
        let mut shuffled = d.clone();
        let mut rng = GameRng::seed_from_u64(1);
        shuffled.shuffle(&mut rng);
        assert_eq!(d.len(), shuffled.len());
        for &c in &d {
            assert!(shuffled.contains(&c));
        }
    }

    // --- 1:1 Go test ports (play_command_test.go) ---

    #[test]
    fn test_play_command_call() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        assert_eq!(vec![MICK, STEVE, BJ], g.whose_turn());

        let mick_card = g.hands[MICK][0];
        g.command(MICK, "play 1", &n).unwrap();
        assert_eq!(Card::Played, g.hands[MICK][0]);
        assert_eq!(vec![mick_card], g.playing[MICK].clone().unwrap());
        assert_eq!(vec![STEVE, BJ], g.whose_turn());

        let bj_card = g.hands[BJ][1];
        g.command(BJ, "play 2", &n).unwrap();
        assert_eq!(Card::Played, g.hands[BJ][1]);
        assert_eq!(vec![bj_card], g.playing[BJ].clone().unwrap());
        assert_eq!(vec![STEVE], g.whose_turn());

        let steve_hand_len = g.hands[STEVE].len();
        let steve_card = g.hands[STEVE][8];
        g.command(STEVE, "play 9", &n).unwrap();
        assert_eq!(steve_hand_len - 1, g.hands[STEVE].len());
        assert_eq!(vec![mick_card], g.played[MICK]);
        assert_eq!(vec![bj_card], g.played[BJ]);
        assert_eq!(vec![steve_card], g.played[STEVE]);
        assert_eq!(vec![MICK, STEVE, BJ], g.whose_turn());
    }

    #[test]
    fn test_play_command_call_chopsticks() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();

        g.hands[MICK] = vec![
            Card::Dumpling,
            Card::MakiRoll3,
            Card::MakiRoll2,
            Card::MakiRoll1,
        ];
        g.hands[STEVE] = vec![
            Card::Dumpling,
            Card::SalmonNigiri,
            Card::SquidNigiri,
            Card::EggNigiri,
        ];
        g.played[MICK] = vec![Card::SquidNigiri, Card::EggNigiri, Card::Dumpling];
        g.played[STEVE] = vec![Card::Pudding, Card::Chopsticks, Card::Sashimi];

        assert!(g.command(MICK, "play 1 2", &n).is_err());
        g.played[MICK][1] = Card::Chopsticks;
        g.command(MICK, "play 1 2", &n).unwrap();

        g.command(STEVE, "play 3", &n).unwrap();
        g.command(BJ, "play 2", &n).unwrap();

        assert_eq!(
            vec![Card::Dumpling, Card::SalmonNigiri, Card::EggNigiri],
            g.hands[MICK]
        );
        assert_eq!(
            vec![Card::MakiRoll2, Card::MakiRoll1, Card::Chopsticks],
            g.hands[BJ]
        );
    }

    #[test]
    fn test_play_command_call_dummy_play_two() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = names();

        g.played[MICK] = vec![Card::Chopsticks];
        g.hands[MICK] = vec![Card::MakiRoll1, Card::MakiRoll2];
        assert!(g.command(MICK, "play 1 2", &n).is_err());

        g.playing[DUMMY] = Some(vec![Card::MakiRoll1]);
        g.command(MICK, "play 1 2", &n).unwrap();
    }

    // --- 1:1 Go test ports (dummy_command_test.go) ---

    #[test]
    fn test_dummy_command_call() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = names();
        assert_eq!(vec![MICK, STEVE], g.whose_turn());

        let mick_card = g.hands[MICK][0];
        g.command(MICK, "play 1", &n).unwrap();
        assert_eq!(vec![mick_card], g.playing[MICK].clone().unwrap());
        assert_eq!(vec![MICK, STEVE], g.whose_turn());

        let steve_card = g.hands[STEVE][8];
        g.command(STEVE, "play 9", &n).unwrap();
        assert_eq!(vec![MICK], g.whose_turn());

        let dummy_card = g.hands[MICK][4];
        g.command(MICK, "dummy 5", &n).unwrap();
        assert_eq!(vec![mick_card], g.played[MICK]);
        assert_eq!(vec![steve_card], g.played[STEVE]);
        assert_eq!(vec![dummy_card], g.played[DUMMY]);
        assert_eq!(vec![MICK, STEVE], g.whose_turn());
    }

    // --- baseline tests per step 8's thin-suite rule ---

    #[test]
    fn test_player_counts() {
        assert_eq!(vec![2, 3, 4, 5], Game::player_counts());
        assert!(Game::start(1, 1).is_err());
        assert!(Game::start(6, 1).is_err());
        assert!(Game::start(2, 1).is_ok());
        assert!(Game::start(5, 1).is_ok());
    }

    #[test]
    fn test_start_state() {
        let (g, logs) = Game::start(3, 1).unwrap();
        assert!(!logs.is_empty());
        assert_eq!(3, g.players);
        assert_eq!(3, g.all_players);
        assert_eq!(1, g.round);
        assert_eq!(3, g.hands.len());
        assert_eq!(9, g.hands[0].len());
        assert_eq!(9, g.hands[1].len());
        assert_eq!(9, g.hands[2].len());
        assert!(g.deck.len() == 108 - 27);
        assert_eq!(0, g.controller);
        assert!(!g.is_finished());
        assert_eq!(vec![MICK, STEVE, BJ], g.whose_turn());
    }

    #[test]
    fn test_start_state_2p() {
        let (g, logs) = Game::start(2, 1).unwrap();
        assert!(!logs.is_empty());
        assert_eq!(2, g.players);
        assert_eq!(3, g.all_players);
        assert_eq!(1, g.round);
        assert_eq!(3, g.hands.len());
        // Controller drew 1 from dummy: 10, other 9, dummy 8
        assert_eq!(10, g.hands[MICK].len());
        assert_eq!(9, g.hands[STEVE].len());
        assert_eq!(8, g.hands[DUMMY].len());
        assert_eq!(0, g.controller);
        assert_eq!(vec![MICK, STEVE], g.whose_turn());
    }

    #[test]
    fn test_deck_composition() {
        let d = deck();
        assert_eq!(108, d.len());
        assert_eq!(14, d.iter().filter(|&&c| c == Card::Tempura).count());
        assert_eq!(14, d.iter().filter(|&&c| c == Card::Sashimi).count());
        assert_eq!(14, d.iter().filter(|&&c| c == Card::Dumpling).count());
        assert_eq!(8, d.iter().filter(|&&c| c == Card::MakiRoll3).count());
        assert_eq!(12, d.iter().filter(|&&c| c == Card::MakiRoll2).count());
        assert_eq!(6, d.iter().filter(|&&c| c == Card::MakiRoll1).count());
        assert_eq!(10, d.iter().filter(|&&c| c == Card::SalmonNigiri).count());
        assert_eq!(5, d.iter().filter(|&&c| c == Card::SquidNigiri).count());
        assert_eq!(5, d.iter().filter(|&&c| c == Card::EggNigiri).count());
        assert_eq!(10, d.iter().filter(|&&c| c == Card::Pudding).count());
        assert_eq!(6, d.iter().filter(|&&c| c == Card::Wasabi).count());
        assert_eq!(4, d.iter().filter(|&&c| c == Card::Chopsticks).count());
    }

    #[test]
    fn test_draw_counts() {
        assert_eq!(9, draw_count(2));
        assert_eq!(9, draw_count(3));
        assert_eq!(8, draw_count(4));
        assert_eq!(7, draw_count(5));
    }

    #[test]
    fn test_can_play_can_dummy() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        // Both real players can play initially
        assert!(g.can_play(MICK));
        assert!(g.can_play(STEVE));
        // Mick is controller, dummy hasn't played -> can dummy
        assert!(g.can_dummy(MICK));
        assert!(!g.can_dummy(STEVE));
        // After Mick plays, can't play again but can still dummy
        g.playing[MICK] = Some(vec![Card::Tempura]);
        assert!(!g.can_play(MICK));
        assert!(g.can_dummy(MICK));
        // After dummy plays, can't dummy
        g.playing[DUMMY] = Some(vec![Card::Sashimi]);
        assert!(!g.can_dummy(MICK));
    }

    #[test]
    fn test_play_errors() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        // Out of range card number
        assert!(g.command(MICK, "play 99", &n).is_err());
        // Playing after already played
        g.command(MICK, "play 1", &n).unwrap();
        assert!(g.command(MICK, "play 1", &n).is_err());
        // Two cards without chopsticks
        assert!(g.command(STEVE, "play 1 2", &n).is_err());
    }

    #[test]
    fn test_play_same_card_twice() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        g.played[MICK] = vec![Card::Chopsticks];
        // Playing the same card index twice
        assert!(g.command(MICK, "play 1 1", &n).is_err());
    }

    #[test]
    fn test_command_after_finished() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        g.round = 3;
        g.hands = vec![vec![], vec![], vec![]];
        g.playing = vec![None, None, None];
        assert!(g.is_finished());
        assert!(g.command(0, "play 1", &n).is_err());
        assert!(g.command(0, "dummy 1", &n).is_err());
    }

    #[test]
    fn test_command_unknown() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        assert!(g.command(0, "frobnicate", &n).is_err());
    }

    #[test]
    fn test_dummy_wrong_player() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = names();
        // Steve is not the controller
        assert!(g.command(STEVE, "dummy 1", &n).is_err());
    }

    #[test]
    fn test_dummy_in_non_2p() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        assert!(g.command(MICK, "dummy 1", &n).is_err());
    }

    #[test]
    fn test_chopsticks_returned_to_hand() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        g.hands[MICK] = vec![Card::Dumpling, Card::MakiRoll3];
        g.hands[STEVE] = vec![Card::Dumpling, Card::SalmonNigiri];
        g.hands[BJ] = vec![Card::Dumpling, Card::SquidNigiri];
        g.played[MICK] = vec![Card::Chopsticks];
        g.command(MICK, "play 1 2", &n).unwrap();
        g.command(STEVE, "play 1", &n).unwrap();
        g.command(BJ, "play 1", &n).unwrap();
        // Chopsticks was returned to Mick's hand, then hands were passed left
        // (round 1). Mick's hand went to BJ (position 2 after rotate_left).
        assert!(!g.played[MICK].contains(&Card::Chopsticks));
        assert!(g.hands[BJ].contains(&Card::Chopsticks));
    }

    #[test]
    fn test_hand_passing_left() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        // Round 1 passes left (rotate_left)
        g.hands[MICK] = vec![Card::Tempura];
        g.hands[STEVE] = vec![Card::Sashimi];
        g.hands[BJ] = vec![Card::Dumpling];
        g.playing[MICK] = Some(vec![Card::Tempura]);
        g.playing[STEVE] = Some(vec![Card::Sashimi]);
        g.playing[BJ] = Some(vec![Card::Dumpling]);
        // Trigger end_hand by having all played (hands are 1 card, after trim -> empty -> end_round)
        g.command(MICK, "play 1", &n).unwrap_err();
        // Actually all already have playing set, so we need to call end_hand directly
        g.end_hand();
        // After end_hand, hands were 1 card each, trimmed to empty, end_round called
        // Round 2 starts and deals new hands. So we can't easily check passing here.
        // Instead test passing with 2-card hands.
    }

    #[test]
    fn test_passing_direction() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        // Round 1: pass left. Simulate each player having played 1 card (mark as Played).
        g.hands = vec![
            vec![Card::Played, Card::Sashimi],
            vec![Card::Played, Card::MakiRoll1],
            vec![Card::Played, Card::SquidNigiri],
        ];
        g.playing = vec![
            Some(vec![Card::Tempura]),
            Some(vec![Card::Dumpling]),
            Some(vec![Card::SalmonNigiri]),
        ];
        g.end_hand();
        // After trim: [Sashimi], [MakiRoll1], [SquidNigiri]
        // Pass left (rotate_left): [0]<-[1], [1]<-[2], [2]<-[0]
        assert_eq!(vec![Card::MakiRoll1], g.hands[MICK]);
        assert_eq!(vec![Card::SquidNigiri], g.hands[STEVE]);
        assert_eq!(vec![Card::Sashimi], g.hands[BJ]);
    }

    #[test]
    fn test_placings() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_points = vec![10, 5, 8];
        g.round = 3;
        g.hands = vec![vec![], vec![], vec![]];
        g.playing = vec![None, None, None];
        assert_eq!(vec![1, 3, 2], g.placings());
    }

    #[test]
    fn test_placings_tie_standard_competition() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_points = vec![10, 10, 5];
        g.round = 3;
        g.hands = vec![vec![], vec![], vec![]];
        g.playing = vec![None, None, None];
        assert_eq!(vec![1, 1, 3], g.placings());
    }

    #[test]
    fn test_placings_pudding_tiebreaker() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        // Mick and Steve tied on points, Mick has more puddings
        g.player_points = vec![10, 10, 0];
        g.played[MICK] = vec![Card::Pudding, Card::Pudding];
        g.played[STEVE] = vec![Card::Pudding];
        g.round = 3;
        g.hands = vec![vec![], vec![], vec![]];
        g.playing = vec![None, None, None];
        assert_eq!(vec![1, 2], g.placings());
    }

    #[test]
    fn test_points_current() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_points = vec![5, 3, 0];
        let pts = g.points();
        assert_eq!(3, pts.len());
        assert_eq!(5.0, pts[0]);
        assert_eq!(3.0, pts[1]);
        assert_eq!(0.0, pts[2]);
    }

    #[test]
    fn test_pub_state_redacts_hands() {
        let (g, _) = Game::start(3, 1).unwrap();
        let ps = g.pub_state();
        assert_eq!(g.played, ps.played);
        assert_eq!(g.player_points, ps.player_points);
        assert_eq!(g.round, ps.round);
        assert!(!ps.finished);
        assert!(ps.final_scores.is_empty());
    }

    #[test]
    fn test_player_state_has_own_hand() {
        let (g, _) = Game::start(3, 1).unwrap();
        let pls = g.player_state(0);
        assert_eq!(g.hands[0], pls.hand);
        assert_eq!(g.playing[0], pls.playing);
    }

    #[test]
    fn test_player_state_2p_dummy_playing() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.playing[DUMMY] = Some(vec![Card::Tempura]);
        let pls = g.player_state(g.controller);
        assert_eq!(Some(vec![Card::Tempura]), pls.dummy_playing);
        let pls_other = g.player_state(1 - g.controller);
        assert_eq!(None, pls_other.dummy_playing);
    }

    #[test]
    fn test_finished_pub_state_has_scores() {
        let (mut g, _) = Game::start(3, 1).unwrap();
        g.player_points = vec![10, 5, 8];
        g.round = 3;
        g.hands = vec![vec![], vec![], vec![]];
        g.playing = vec![None, None, None];
        let ps = g.pub_state();
        assert!(ps.finished);
        assert_eq!(vec![10, 5, 8], ps.final_scores);
    }

    #[test]
    fn test_full_game_3p_completes() {
        let n = names();
        let (mut g, _) = Game::start(3, 1).unwrap();
        for _ in 0..1000 {
            if g.is_finished() {
                break;
            }
            let wt = g.whose_turn();
            if wt.is_empty() {
                break;
            }
            let p = wt[0];
            let spec = g.command_spec(p);
            if spec.is_none() {
                break;
            }
            // Just play card 1 for each player
            g.command(p, "play 1", &n).unwrap();
        }
        assert!(g.is_finished());
        assert_eq!(3, g.round);
    }

    #[test]
    fn test_full_game_2p_completes() {
        let n = vec!["Mick".to_string(), "Steve".to_string()];
        let (mut g, _) = Game::start(2, 1).unwrap();
        for _ in 0..5000 {
            if g.is_finished() {
                break;
            }
            let wt = g.whose_turn();
            if wt.is_empty() {
                break;
            }
            for &p in &wt {
                if g.is_finished() {
                    break;
                }
                if g.can_play(p) {
                    let idx = g.hands[p]
                        .iter()
                        .position(|&c| c != Card::Played)
                        .unwrap_or(0);
                    g.command(p, &format!("play {}", idx + 1), &n).unwrap();
                }
                if g.is_finished() {
                    break;
                }
                if g.can_dummy(p) {
                    let idx = g.hands[p]
                        .iter()
                        .position(|&c| c != Card::Played)
                        .unwrap_or(0);
                    g.command(p, &format!("dummy {}", idx + 1), &n).unwrap();
                }
            }
        }
        assert!(g.is_finished());
        assert_eq!(3, g.round);
    }
}
