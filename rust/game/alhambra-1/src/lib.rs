mod card;
mod command;
mod render;

pub use card::*;
pub use command::Command;

use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status};
use brdgme_markup::Node as N;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 6;
pub const DIRK: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Action,
    Place,
    FinalPlace,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub human_players: usize,
    pub all_players: usize,
    pub current_player: usize,
    pub phase: Phase,
    pub round: i32,
    pub boards: Vec<PlayerBoard>,
    pub cards: Vec<Card>,
    pub card_pile: Vec<DeckCard>,
    pub discard_pile: Vec<Card>,
    pub tiles: Vec<Tile>,
    pub tile_bag: Vec<Tile>,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubState {
    /// Number of human players (2-6).
    pub human_players: usize,
    /// Total players including Dirk (the AI opponent in 2-player games).
    pub all_players: usize,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// Current game phase (Action, Place, FinalPlace, or End).
    pub phase: Phase,
    /// Current scoring round (1-3).
    pub round: i32,
    /// Public board state for each player.
    pub boards: Vec<PubBoard>,
    /// Money cards available in the market (up to 4).
    pub cards: Vec<Card>,
    /// Building tiles available in the market (up to 4, one per currency).
    pub tiles: Vec<Tile>,
    /// Number of tiles remaining in the tile bag.
    pub tile_bag_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubBoard {
    /// The player's placed tiles on their grid.
    #[serde(with = "grid_serde")]
    pub grid: Grid,
    /// Tiles in the player's reserve (can be placed or swapped during the Action phase).
    pub reserve: Vec<Tile>,
    /// Number of money cards in the player's hand (actual cards are private).
    pub card_count: usize,
    /// Tiles the player has bought but not yet placed on their grid.
    pub place: Vec<Tile>,
    /// The player's current score.
    pub points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// Money cards in this player's hand.
    pub hand: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoundTypeScore {
    pub players: Vec<usize>,
    pub tile_count: i32,
    pub points: i32,
}

impl Game {
    pub fn start_game(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players) {
            return Err(GameError::PlayerCount {
                min: MIN_PLAYERS,
                max: MAX_PLAYERS,
                given: players,
            });
        }
        let all_players = if players == 2 { 3 } else { players };
        let mut rng = GameRng::seed_from_u64(seed);

        let mut card_pile = build_deck(players);
        card_pile.shuffle(&mut rng);

        let mut cards = vec![];
        for _ in 0..4 {
            if let Some(DeckCard::Money(c)) = card_pile.pop() {
                cards.push(c);
            }
        }

        let mut tile_bag = all_tiles();
        tile_bag.shuffle(&mut rng);
        let mut tiles = vec![];
        for _ in 0..4 {
            tiles.push(tile_bag.pop().unwrap_or_else(Tile::empty));
        }

        let boards: Vec<PlayerBoard> = (0..all_players).map(|_| PlayerBoard::new()).collect();

        let mut g = Game {
            human_players: players,
            all_players,
            current_player: 0,
            phase: Phase::Action,
            round: 1,
            boards,
            cards,
            card_pile,
            discard_pile: vec![],
            tiles,
            tile_bag,
            rng,
        };

        let mut logs = vec![];

        for p in 0..players {
            let mut total = 0;
            let mut drew = vec![];
            while total < 20 {
                let (draw_logs, drawn) = g.draw_cards(1);
                logs.extend(draw_logs);
                if drawn.is_empty() {
                    break;
                }
                let c = drawn[0];
                total += c.value;
                drew.push(c);
                g.boards[p].cards.push(c);
            }
            if !drew.is_empty() {
                let card_strs: Vec<String> = drew.iter().map(|c| c.to_string()).collect();
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(format!(" drew {}", card_strs.join(", "))),
                ]));
            }
        }

        let mut start_player = 0;
        let mut min_cards = usize::MAX;
        let mut min_value = i32::MAX;
        for p in 0..players {
            let count = g.boards[p].cards.len();
            let value: i32 = g.boards[p].cards.iter().map(|c| c.value).sum();
            if count < min_cards || (count == min_cards && value < min_value) {
                min_cards = count;
                min_value = value;
                start_player = p;
            }
        }
        g.current_player = start_player;

        if players == 2 {
            let dirk_logs = g.dirk_draw_tiles(6);
            logs.extend(dirk_logs);
        }

        g.inject_scoring_cards();

        Ok((g, logs))
    }

    fn inject_scoring_cards(&mut self) {
        let len = self.card_pile.len();
        if len < 10 {
            return;
        }
        let fifth = len / 5;
        if fifth == 0 {
            return;
        }
        let pos1 = fifth + self.rng.random_range(0..fifth);
        let pos2 = 3 * fifth + self.rng.random_range(0..fifth) + 1;
        self.card_pile.insert(pos2, DeckCard::Scoring);
        self.card_pile.insert(pos1, DeckCard::Scoring);
    }

    pub fn draw_cards(&mut self, n: i32) -> (Vec<Log>, Vec<Card>) {
        if n <= 0 {
            return (vec![], vec![]);
        }
        if self.card_pile.is_empty() {
            if self.discard_pile.is_empty() {
                return (vec![], vec![]);
            }
            self.card_pile = self.discard_pile.drain(..).map(DeckCard::Money).collect();
            self.card_pile.shuffle(&mut self.rng);
        }
        match self.card_pile.pop() {
            Some(DeckCard::Money(c)) => {
                let (logs, mut drawn) = self.draw_cards(n - 1);
                drawn.push(c);
                (logs, drawn)
            }
            Some(DeckCard::Scoring) => {
                let score_logs = self.score_round();
                let (mut logs, drawn) = self.draw_cards(n);
                let mut all = score_logs;
                all.append(&mut logs);
                (all, drawn)
            }
            None => (vec![], vec![]),
        }
    }

    pub fn score_round(&mut self) -> Vec<Log> {
        let mut logs = vec![Log::public(vec![N::text(format!(
            "It is now scoring round {}",
            self.round
        ))])];

        for tt in SCORING_TILE_TYPES {
            let scores = self.score_type(tt, self.round);
            for s in &scores {
                for &p in &s.players {
                    self.boards[p].points += s.points;
                }
                let player_nodes: Vec<N> = s
                    .players
                    .iter()
                    .flat_map(|&p| vec![N::Player(p), N::text(", ")])
                    .collect();
                let mut content = vec![N::text(format!("{:?} - ", tt))];
                if player_nodes.is_empty() {
                    content.push(N::text("nobody scored"));
                } else {
                    content.extend(player_nodes);
                    content.push(N::text(format!(
                        "scored {} points ({} tiles)",
                        s.points, s.tile_count
                    )));
                }
                logs.push(Log::public(content));
            }
        }

        logs.push(Log::public(vec![N::text("Scoring walls")]));
        for p in 0..self.human_players {
            let wall = grid_longest_ext_wall(&self.boards[p].grid);
            if wall > 0 {
                self.boards[p].points += wall;
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(format!(" scored {} points for walls", wall)),
                ]));
            }
        }

        if self.human_players == 2 {
            if self.round == 1 {
                let dirk_logs = self.dirk_draw_tiles(6);
                logs.extend(dirk_logs);
            } else if self.round == 2 {
                let n = self.tile_bag.len() / 3;
                let dirk_logs = self.dirk_draw_tiles(n as i32);
                logs.extend(dirk_logs);
            }
        }

        if self.round < 3 {
            self.round += 1;
        }

        logs
    }

    pub fn score_type(&self, tile_type: TileType, round: i32) -> Vec<RoundTypeScore> {
        let mut counts: Vec<(usize, i32)> = vec![];
        for p in 0..self.all_players {
            let tc = self.boards[p]
                .tile_counts()
                .get(&tile_type)
                .copied()
                .unwrap_or(0);
            if tc > 0 {
                counts.push((p, tc));
            }
        }
        counts.sort_by_key(|b| std::cmp::Reverse(b.1));

        let rewards = round_scores(tile_type);
        let mut reward_slice: Vec<i32> = rewards[..(round as usize).min(3)].to_vec();

        let mut results = vec![];
        let mut i = 0;
        while i < counts.len() && !reward_slice.is_empty() {
            let current_count = counts[i].1;
            let mut group_players = vec![];
            while i < counts.len() && counts[i].1 == current_count {
                group_players.push(counts[i].0);
                i += 1;
            }
            let n = group_players.len().min(reward_slice.len());
            let points: i32 = reward_slice[reward_slice.len() - n..].iter().sum::<i32>()
                / group_players.len() as i32;
            results.push(RoundTypeScore {
                players: group_players,
                tile_count: current_count,
                points,
            });
            let new_len = reward_slice.len() - n;
            reward_slice.truncate(new_len);
        }
        results
    }

    pub fn next_phase(&mut self) -> Vec<Log> {
        match self.phase {
            Phase::Action => self.place_phase(),
            Phase::Place => self.next_player(),
            Phase::FinalPlace => {
                let mut logs = self.reserve_tiles();
                let mut next_player = self.current_player;
                loop {
                    next_player = (next_player + 1) % self.human_players;
                    if next_player == self.current_player {
                        self.phase = Phase::End;
                        let score_logs = self.score_round();
                        logs.extend(score_logs);
                        break;
                    }
                    if !not_empty(&self.boards[next_player].place).is_empty() {
                        self.current_player = next_player;
                        break;
                    }
                }
                logs
            }
            Phase::End => vec![],
        }
    }

    fn place_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::Place;
        if not_empty(&self.boards[self.current_player].place).is_empty() {
            return self.next_phase();
        }
        vec![]
    }

    pub fn next_player(&mut self) -> Vec<Log> {
        let mut logs = self.reserve_tiles();

        for i in 0..4 {
            if self.tiles[i].tile_type == TileType::Empty {
                if let Some(t) = self.tile_bag.pop() {
                    self.tiles[i] = t;
                } else {
                    let fp_logs = self.final_place_phase();
                    logs.extend(fp_logs);
                    return logs;
                }
            }
        }

        while self.cards.len() < 4 {
            let (draw_logs, drawn) = self.draw_cards(1);
            logs.extend(draw_logs);
            if drawn.is_empty() {
                break;
            }
            self.cards.extend(drawn);
        }

        self.current_player = (self.current_player + 1) % self.human_players;
        self.phase = Phase::Action;
        logs
    }

    fn final_place_phase(&mut self) -> Vec<Log> {
        self.phase = Phase::FinalPlace;
        let mut logs = vec![];

        for currency in Currency::ALL {
            let ci = Currency::ALL.iter().position(|&c| c == currency).unwrap();
            if self.tiles[ci].tile_type == TileType::Empty {
                continue;
            }
            let mut best_player = None;
            let mut best_value = -1;
            let mut tied = false;
            for p in 0..self.human_players {
                let val = self.boards[p].currency_value(currency);
                if val > best_value {
                    best_value = val;
                    best_player = Some(p);
                    tied = false;
                } else if val == best_value {
                    tied = true;
                }
            }
            if tied {
                logs.push(Log::public(vec![N::text(format!(
                    "Nobody got the {:?} tile (tie)",
                    self.tiles[ci].tile_type
                ))]));
            } else if let Some(p) = best_player {
                let tile = self.tiles[ci].clone();
                self.boards[p].place.push(tile.clone());
                self.tiles[ci] = Tile::empty();
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(format!(" got the {:?} tile", tile.tile_type)),
                ]));
            }
        }

        if not_empty(&self.boards[self.current_player].place).is_empty() {
            let np_logs = self.next_phase();
            logs.extend(np_logs);
        }
        logs
    }

    fn reserve_tiles(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let place = std::mem::take(&mut self.boards[self.current_player].place);
        let non_empty: Vec<Tile> = place
            .into_iter()
            .filter(|t| t.tile_type != TileType::Empty)
            .collect();
        if !non_empty.is_empty() {
            let count = non_empty.len();
            self.boards[self.current_player].reserve.extend(non_empty);
            logs.push(Log::public(vec![
                N::Player(self.current_player),
                N::text(format!(" moved {} tile(s) to reserve", count)),
            ]));
        }
        logs
    }

    fn dirk_draw_tiles(&mut self, n: i32) -> Vec<Log> {
        if n <= 0 {
            return vec![];
        }
        let mut count = 0;
        for _ in 0..n {
            if let Some(t) = self.tile_bag.pop() {
                let x = self.boards[DIRK].grid.len() as i32;
                self.boards[DIRK].grid.insert(Vect { x, y: 0 }, t);
                count += 1;
            }
        }
        if count > 0 {
            vec![Log::public(vec![
                N::Player(DIRK),
                N::text(format!(" took {} tiles", count)),
            ])]
        } else {
            vec![]
        }
    }

    fn can_take(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Action
    }

    fn can_spend(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Action
    }

    fn can_place(&self, player: usize) -> bool {
        self.current_player == player
            && ((self.phase == Phase::Action && !self.boards[player].reserve.is_empty())
                || ((self.phase == Phase::Place || self.phase == Phase::FinalPlace)
                    && !not_empty(&self.boards[player].place).is_empty()))
    }

    fn can_swap(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Action
            && !self.boards[player].reserve.is_empty()
    }

    fn can_remove(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::Action
    }

    fn can_done(&self, player: usize) -> bool {
        self.current_player == player
            && (self.phase == Phase::Place || self.phase == Phase::FinalPlace)
    }

    pub fn take(&mut self, player: usize, cards: &[Card]) -> Result<Vec<Log>, GameError> {
        if !self.can_take(player) {
            return Err(GameError::invalid_input("can't take at the moment"));
        }
        if cards.is_empty() {
            return Err(GameError::invalid_input("must take at least one card"));
        }
        for c in cards {
            if !self.cards.contains(c) {
                return Err(GameError::invalid_input(format!("{} is not available", c)));
            }
        }
        if cards.len() > 1 {
            let total: i32 = cards.iter().map(|c| c.value).sum();
            if total > 5 {
                return Err(GameError::invalid_input(
                    "can't take more than one card with a total value over 5",
                ));
            }
        }
        for c in cards {
            if let Some(pos) = self.cards.iter().position(|mc| mc == c) {
                self.cards.remove(pos);
            }
            self.boards[player].cards.push(*c);
        }
        let card_strs: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" took {}", card_strs.join(", "))),
        ])];
        let np_logs = self.next_phase();
        logs.extend(np_logs);
        Ok(logs)
    }

    pub fn spend(&mut self, player: usize, cards: &[Card]) -> Result<Vec<Log>, GameError> {
        if !self.can_spend(player) {
            return Err(GameError::invalid_input("can't spend at the moment"));
        }
        if cards.is_empty() {
            return Err(GameError::invalid_input("must spend at least one card"));
        }
        let currency = cards[0].currency;
        for c in cards {
            if c.currency != currency {
                return Err(GameError::invalid_input(
                    "all cards must be the same currency",
                ));
            }
        }
        let ci = Currency::ALL.iter().position(|&c| c == currency).unwrap();
        if self.tiles[ci].tile_type == TileType::Empty {
            return Err(GameError::invalid_input(format!(
                "no tile available for {:?}",
                currency
            )));
        }
        let tile = &self.tiles[ci];
        let total: i32 = cards.iter().map(|c| c.value).sum();
        if total < tile.cost {
            return Err(GameError::invalid_input(format!(
                "not enough money, need {} but have {}",
                tile.cost, total
            )));
        }
        let mut hand = self.boards[player].cards.clone();
        for c in cards {
            match hand.iter().position(|hc| hc == c) {
                Some(pos) => {
                    hand.remove(pos);
                }
                None => {
                    return Err(GameError::invalid_input(format!("don't have {}", c)));
                }
            }
        }
        self.boards[player].cards = hand;
        let tile = self.tiles[ci].clone();
        self.discard_pile.extend(cards.iter().copied());
        self.boards[player].place.push(tile.clone());
        self.tiles[ci] = Tile::empty();

        let card_strs: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(
                " spent {} on {:?} tile",
                card_strs.join(", "),
                tile.tile_type
            )),
        ])];

        if total != tile.cost {
            let np_logs = self.next_phase();
            logs.extend(np_logs);
        }
        Ok(logs)
    }

    pub fn place(&mut self, player: usize, n: usize, coord: Vect) -> Result<Vec<Log>, GameError> {
        if !self.can_place(player) {
            return Err(GameError::invalid_input("can't place at the moment"));
        }
        if self.boards[player].grid.contains_key(&coord) {
            return Err(GameError::invalid_input("coordinate is not empty"));
        }
        let tile = match self.phase {
            Phase::Action => {
                if n >= self.boards[player].reserve.len() {
                    return Err(GameError::invalid_input("invalid reserve tile index"));
                }
                self.boards[player].reserve[n].clone()
            }
            _ => {
                if n >= self.boards[player].place.len() {
                    return Err(GameError::invalid_input("invalid place tile index"));
                }
                self.boards[player].place[n].clone()
            }
        };
        let mut test_grid = self.boards[player].grid.clone();
        test_grid.insert(coord, tile.clone());
        let (valid, msg) = grid_is_valid(&test_grid);
        if !valid {
            return Err(GameError::invalid_input(format!(
                "invalid placement: {}",
                msg
            )));
        }
        self.boards[player].grid.insert(coord, tile.clone());

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" placed {:?} tile", tile.tile_type)),
        ])];

        match self.phase {
            Phase::Action => {
                self.boards[player].reserve.remove(n);
                let np_logs = self.next_phase();
                logs.extend(np_logs);
            }
            _ => {
                self.boards[player].place[n] = Tile::empty();
                if not_empty(&self.boards[player].place).is_empty() {
                    let np_logs = self.next_phase();
                    logs.extend(np_logs);
                }
            }
        }
        Ok(logs)
    }

    pub fn swap(&mut self, player: usize, n: usize, coord: Vect) -> Result<Vec<Log>, GameError> {
        if !self.can_swap(player) {
            return Err(GameError::invalid_input("can't swap at the moment"));
        }
        if n >= self.boards[player].reserve.len() {
            return Err(GameError::invalid_input("invalid reserve tile index"));
        }
        if !self.boards[player].grid.contains_key(&coord) {
            return Err(GameError::invalid_input("no tile at coordinate"));
        }
        let reserve_tile = self.boards[player].reserve[n].clone();
        let grid_tile = self.boards[player].grid[&coord].clone();

        let mut test_grid = self.boards[player].grid.clone();
        test_grid.insert(coord, reserve_tile.clone());
        let (valid, msg) = grid_is_valid(&test_grid);
        if !valid {
            return Err(GameError::invalid_input(format!("invalid swap: {}", msg)));
        }

        self.boards[player].grid.insert(coord, reserve_tile.clone());
        self.boards[player].reserve[n] = grid_tile.clone();

        let logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(
                " swapped {:?} with {:?} tile",
                reserve_tile.tile_type, grid_tile.tile_type
            )),
        ])];
        let mut all_logs = logs;
        let np_logs = self.next_phase();
        all_logs.extend(np_logs);
        Ok(all_logs)
    }

    pub fn remove(&mut self, player: usize, coord: Vect) -> Result<Vec<Log>, GameError> {
        if !self.can_remove(player) {
            return Err(GameError::invalid_input("can't remove at the moment"));
        }
        if !self.boards[player].grid.contains_key(&coord) {
            return Err(GameError::invalid_input("no tile at coordinate"));
        }
        let tile = self.boards[player].grid[&coord].clone();
        let mut test_grid = self.boards[player].grid.clone();
        test_grid.remove(&coord);
        let (valid, msg) = grid_is_valid(&test_grid);
        if !valid {
            return Err(GameError::invalid_input(format!(
                "invalid removal: {}",
                msg
            )));
        }
        self.boards[player].grid.remove(&coord);
        self.boards[player].reserve.push(tile.clone());

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" removed {:?} tile to reserve", tile.tile_type)),
        ])];
        let np_logs = self.next_phase();
        logs.extend(np_logs);
        Ok(logs)
    }

    pub fn done(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_done(player) {
            return Err(GameError::invalid_input("can't finish at the moment"));
        }
        Ok(self.next_phase())
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        Game::start_game(players, seed)
    }

    fn pub_state(&self) -> Self::PubState {
        PubState {
            human_players: self.human_players,
            all_players: self.all_players,
            current_player: self.current_player,
            phase: self.phase,
            round: self.round,
            boards: self
                .boards
                .iter()
                .map(|b| PubBoard {
                    grid: b.grid.clone(),
                    reserve: b.reserve.clone(),
                    card_count: b.cards.len(),
                    place: b.place.clone(),
                    points: b.points,
                })
                .collect(),
            cards: self.cards.clone(),
            tiles: self.tiles.clone(),
            tile_bag_len: self.tile_bag.len(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            hand: self.boards[player].cards.clone(),
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
                value: Command::Take { cards },
                ..
            }) => {
                let logs = self.take(player, &cards)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Spend { cards },
                ..
            }) => {
                let logs = self.spend(player, &cards)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Place { tile, coord },
                ..
            }) => {
                let logs = self.place(player, tile, coord)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Swap { tile, coord },
                ..
            }) => {
                let logs = self.swap(player, tile, coord)?;
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Remove { coord },
                ..
            }) => {
                let logs = self.remove(player, coord)?;
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

    fn status(&self) -> Status {
        if self.phase == Phase::End {
            let metrics: Vec<Vec<i32>> = (0..self.human_players)
                .map(|p| vec![self.boards[p].points])
                .collect();
            let placings = gen_placings(&metrics);
            Status::Finished {
                placings,
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..self.human_players)
            .map(|p| self.boards[p].points as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
    }

    fn player_count(&self) -> usize {
        self.human_players
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
    use std::collections::HashMap;

    fn is_wall_char(c: char) -> bool {
        c == '|' || c == '-' || c == '+'
    }

    fn char_at(x: usize, y: usize, lines: &[&str]) -> char {
        if y >= lines.len() {
            return ' ';
        }
        let chars: Vec<char> = lines[y].chars().collect();
        if x >= chars.len() {
            return ' ';
        }
        chars[x]
    }

    fn parse_grid(input: &str) -> Grid {
        let mut g: Grid = HashMap::new();
        let input = input.trim_matches('\n');
        let lines: Vec<&str> = input.lines().collect();

        for y in (1..lines.len()).step_by(2) {
            let chars: Vec<char> = lines[y].chars().collect();
            for x in (1..chars.len()).step_by(2) {
                if chars[x] == ' ' {
                    continue;
                }
                let tile_type = match chars[x] {
                    'F' => TileType::Fountain,
                    'P' => TileType::Pavillion,
                    'S' => TileType::Seraglio,
                    'A' => TileType::Arcades,
                    'C' => TileType::Chambers,
                    'G' => TileType::Garden,
                    'T' => TileType::Tower,
                    _ => continue,
                };
                let mut walls: Vec<Dir> = vec![];
                if is_wall_char(char_at(x, y.saturating_sub(1), &lines)) {
                    walls.push(Dir::Up);
                }
                if is_wall_char(char_at(x, y + 1, &lines)) {
                    walls.push(Dir::Down);
                }
                if is_wall_char(char_at(x.saturating_sub(1), y, &lines)) {
                    walls.push(Dir::Left);
                }
                if is_wall_char(char_at(x + 1, y, &lines)) {
                    walls.push(Dir::Right);
                }
                g.insert(
                    Vect {
                        x: ((x - 1) / 2) as i32,
                        y: ((y - 1) / 2) as i32,
                    },
                    Tile::new(tile_type, 0, &walls),
                );
            }
        }
        g
    }

    #[test]
    fn test_parse_card() {
        let c = Card::parse("R10").unwrap();
        assert_eq!(c, Card::new(Currency::Red, 10));
    }

    #[test]
    fn test_game_score_type() {
        let (mut g, _) = Game::start(3, 0).unwrap();

        g.boards[0].grid = {
            let mut gr: Grid = HashMap::new();
            gr.insert(Vect { x: 0, y: 1 }, Tile::new(TileType::Pavillion, 0, &[]));
            gr.insert(Vect { x: 0, y: 2 }, Tile::new(TileType::Seraglio, 0, &[]));
            gr.insert(Vect { x: 0, y: 3 }, Tile::new(TileType::Seraglio, 0, &[]));
            gr.insert(Vect { x: 0, y: 4 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 5 }, Tile::new(TileType::Chambers, 0, &[]));
            gr.insert(Vect { x: 0, y: 6 }, Tile::new(TileType::Chambers, 0, &[]));
            gr
        };

        g.boards[1].grid = {
            let mut gr: Grid = HashMap::new();
            gr.insert(Vect { x: 0, y: 1 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 2 }, Tile::new(TileType::Chambers, 0, &[]));
            gr.insert(Vect { x: 0, y: 3 }, Tile::new(TileType::Seraglio, 0, &[]));
            gr.insert(Vect { x: 0, y: 4 }, Tile::new(TileType::Tower, 0, &[]));
            gr.insert(Vect { x: 0, y: 5 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 6 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 7 }, Tile::new(TileType::Chambers, 0, &[]));
            gr
        };

        g.boards[2].grid = {
            let mut gr: Grid = HashMap::new();
            gr.insert(Vect { x: 0, y: 1 }, Tile::new(TileType::Garden, 0, &[]));
            gr.insert(Vect { x: 0, y: 2 }, Tile::new(TileType::Tower, 0, &[]));
            gr.insert(Vect { x: 0, y: 3 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 4 }, Tile::new(TileType::Arcades, 0, &[]));
            gr.insert(Vect { x: 0, y: 5 }, Tile::new(TileType::Chambers, 0, &[]));
            gr
        };

        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0],
                tile_count: 1,
                points: 1
            }],
            g.score_type(TileType::Pavillion, 1)
        );
        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0],
                tile_count: 1,
                points: 8
            }],
            g.score_type(TileType::Pavillion, 2)
        );
        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0],
                tile_count: 1,
                points: 16
            }],
            g.score_type(TileType::Pavillion, 3)
        );

        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0],
                tile_count: 2,
                points: 2
            }],
            g.score_type(TileType::Seraglio, 1)
        );
        assert_eq!(
            vec![
                RoundTypeScore {
                    players: vec![0],
                    tile_count: 2,
                    points: 9
                },
                RoundTypeScore {
                    players: vec![1],
                    tile_count: 1,
                    points: 2
                },
            ],
            g.score_type(TileType::Seraglio, 2)
        );
        assert_eq!(
            vec![
                RoundTypeScore {
                    players: vec![0],
                    tile_count: 2,
                    points: 17
                },
                RoundTypeScore {
                    players: vec![1],
                    tile_count: 1,
                    points: 9
                },
            ],
            g.score_type(TileType::Seraglio, 3)
        );

        assert_eq!(
            vec![RoundTypeScore {
                players: vec![1],
                tile_count: 3,
                points: 3
            }],
            g.score_type(TileType::Arcades, 1)
        );
        assert_eq!(
            vec![
                RoundTypeScore {
                    players: vec![1],
                    tile_count: 3,
                    points: 10
                },
                RoundTypeScore {
                    players: vec![2],
                    tile_count: 2,
                    points: 3
                },
            ],
            g.score_type(TileType::Arcades, 2)
        );
        assert_eq!(
            vec![
                RoundTypeScore {
                    players: vec![1],
                    tile_count: 3,
                    points: 18
                },
                RoundTypeScore {
                    players: vec![2],
                    tile_count: 2,
                    points: 10
                },
                RoundTypeScore {
                    players: vec![0],
                    tile_count: 1,
                    points: 3
                },
            ],
            g.score_type(TileType::Arcades, 3)
        );

        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0, 1],
                tile_count: 2,
                points: 2
            }],
            g.score_type(TileType::Chambers, 1)
        );
        assert_eq!(
            vec![RoundTypeScore {
                players: vec![0, 1],
                tile_count: 2,
                points: 7
            }],
            g.score_type(TileType::Chambers, 2)
        );
        assert_eq!(
            vec![
                RoundTypeScore {
                    players: vec![0, 1],
                    tile_count: 2,
                    points: 15
                },
                RoundTypeScore {
                    players: vec![2],
                    tile_count: 1,
                    points: 4
                },
            ],
            g.score_type(TileType::Chambers, 3)
        );
    }

    #[test]
    fn test_grid_is_valid_valid() {
        let g = new_grid();
        let (valid, _) = grid_is_valid(&g);
        assert!(valid);
    }

    #[test]
    fn test_grid_is_valid_invalid_no_fountain() {
        let mut g = HashMap::new();
        g.insert(Vect { x: 0, y: 0 }, Tile::new(TileType::Pavillion, 3, &[]));
        let (valid, msg) = grid_is_valid(&g);
        assert!(!valid);
        assert_eq!(msg, GRID_INVALID_NO_FOUNTAIN);
    }

    #[test]
    fn test_grid_is_valid_invalid_wall() {
        let mut g = new_grid();
        g.insert(
            Vect { x: 1, y: 0 },
            Tile::new(TileType::Pavillion, 3, &[Dir::Left]),
        );
        let (valid, msg) = grid_is_valid(&g);
        assert!(!valid);
        assert_eq!(msg, GRID_INVALID_WALL);
    }

    #[test]
    fn test_grid_is_valid_invalid_cannot_walk() {
        let mut g = new_grid();
        g.insert(Vect { x: 2, y: 0 }, Tile::new(TileType::Pavillion, 3, &[]));
        let (valid, msg) = grid_is_valid(&g);
        assert!(!valid);
        assert_eq!(msg, GRID_INVALID_CANNOT_WALK);
    }

    #[test]
    fn test_grid_is_valid_invalid_gap() {
        let mut g: Grid = HashMap::new();
        g.insert(Vect { x: 0, y: 0 }, Tile::new(TileType::Arcades, 0, &[]));
        g.insert(Vect { x: 1, y: 0 }, Tile::new(TileType::Seraglio, 0, &[]));
        g.insert(Vect { x: 2, y: 0 }, Tile::new(TileType::Fountain, 0, &[]));
        g.insert(Vect { x: 0, y: 1 }, Tile::new(TileType::Arcades, 0, &[]));
        g.insert(Vect { x: 2, y: 1 }, Tile::new(TileType::Arcades, 0, &[]));
        g.insert(Vect { x: 1, y: 2 }, Tile::new(TileType::Arcades, 0, &[]));
        g.insert(Vect { x: 2, y: 2 }, Tile::new(TileType::Arcades, 0, &[]));
        let (valid, msg) = grid_is_valid(&g);
        assert!(!valid);
        assert_eq!(msg, GRID_INVALID_GAP);
    }

    #[test]
    fn test_grid_longest_ext_wall() {
        assert_eq!(
            5,
            grid_longest_ext_wall(&parse_grid(
                "
+-
|A A A
     -+
 A A A|
     -+
 A A A|
 -----+-+
       A|
       -+
"
            ))
        );
        assert_eq!(
            12,
            grid_longest_ext_wall(&parse_grid(
                "
+-----+
|A A A|
|    -+
|A A A|
|  ---+
|A A A|
+-----+
"
            ))
        );
    }

    #[test]
    fn test_grid_parse_coord() {
        let mut g: Grid = HashMap::new();
        g.insert(
            Vect { x: -2, y: -3 },
            Tile::new(TileType::Pavillion, 5, &[]),
        );

        let v = grid_parse_coord(&g, "a1").unwrap();
        assert_eq!(v, Vect { x: -3, y: -4 });
        let v = grid_parse_coord(&g, "1a").unwrap();
        assert_eq!(v, Vect { x: -3, y: -4 });
        let v = grid_parse_coord(&g, "2a").unwrap();
        assert_eq!(v, Vect { x: -3, y: -3 });
        let v = grid_parse_coord(&g, "2b").unwrap();
        assert_eq!(v, Vect { x: -2, y: -3 });
        let v = grid_parse_coord(&g, "b2").unwrap();
        assert_eq!(v, Vect { x: -2, y: -3 });
    }

    #[test]
    fn test_spend_command_multiple_same_card() {
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.boards[0].cards = vec![
            Card::new(Currency::Blue, 1),
            Card::new(Currency::Blue, 1),
            Card::new(Currency::Blue, 1),
        ];
        g.tiles[0] = Tile::new(TileType::Tower, 3, &[]);

        let result = g.command(0, "spend b1 b1 b1 b1", &[]);
        assert!(result.is_err());

        let result = g.command(0, "spend b1 b1 b1", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn pub_state_does_not_leak_hidden_info() {
        let (g, _) = Game::start(3, 0).unwrap();
        let ps = g.pub_state();
        let json = serde_json::to_string(&ps).unwrap();
        assert!(!json.contains("\"hand\""));
        assert!(!json.contains("\"card_pile\""));
        assert!(!json.contains("\"discard_pile\""));
        assert!(!json.contains("\"rng\""));
    }
}
