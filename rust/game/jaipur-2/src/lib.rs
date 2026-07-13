use std::collections::HashMap;
use std::fmt;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use brdgme_color::NamedColor;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{Gamer, Log, Status};
use brdgme_markup::Node as N;

use crate::render::render_goods_items;

mod command;
mod render;
pub use command::Command;

pub const NUM_PLAYERS: usize = 2;
pub const HAND_SIZE: usize = 7;
pub const CAMEL_BONUS_POINTS: u32 = 5;
pub const MIN_TRADE_BONUS: usize = 3;
pub const MAX_TRADE_BONUS: usize = 5;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Good {
    Diamond,
    Gold,
    Silver,
    Cloth,
    Spice,
    Leather,
    Camel,
}

impl Good {
    pub fn trade_goods() -> &'static [Good; 6] {
        &[
            Good::Diamond,
            Good::Gold,
            Good::Silver,
            Good::Cloth,
            Good::Spice,
            Good::Leather,
        ]
    }

    pub fn all_goods() -> &'static [Good; 7] {
        &[
            Good::Diamond,
            Good::Gold,
            Good::Silver,
            Good::Cloth,
            Good::Spice,
            Good::Leather,
            Good::Camel,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Good::Diamond => "diamond",
            Good::Gold => "gold",
            Good::Silver => "silver",
            Good::Cloth => "cloth",
            Good::Spice => "spice",
            Good::Leather => "leather",
            Good::Camel => "camel",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Good::Diamond => "diamonds",
            Good::Gold => "golds",
            Good::Silver => "silvers",
            Good::Cloth => "cloths",
            Good::Spice => "spices",
            Good::Leather => "leathers",
            Good::Camel => "camels",
        }
    }

    pub fn color(self) -> NamedColor {
        match self {
            Good::Diamond => NamedColor::Red,
            Good::Gold => NamedColor::Yellow,
            Good::Silver => NamedColor::Grey,
            Good::Cloth => NamedColor::Purple,
            Good::Spice => NamedColor::Green,
            Good::Leather => NamedColor::Blue,
            Good::Camel => NamedColor::Foreground,
        }
    }

    pub fn card_count(self) -> u32 {
        match self {
            Good::Diamond => 6,
            Good::Gold => 6,
            Good::Silver => 6,
            Good::Cloth => 8,
            Good::Spice => 8,
            Good::Leather => 10,
            Good::Camel => 8,
        }
    }

    pub fn min_sale(self) -> u32 {
        match self {
            Good::Diamond | Good::Gold | Good::Silver => 2,
            Good::Cloth | Good::Spice | Good::Leather => 1,
            Good::Camel => 0,
        }
    }

    pub fn token_values(self) -> &'static [u32] {
        match self {
            Good::Diamond => &[7, 7, 5, 5, 5],
            Good::Gold => &[6, 6, 5, 5, 5],
            Good::Silver => &[5, 5, 5, 5, 5],
            Good::Cloth => &[5, 3, 3, 2, 2, 1, 1],
            Good::Spice => &[5, 3, 3, 2, 2, 1, 1],
            Good::Leather => &[4, 3, 2, 1, 1, 1, 1, 1, 1],
            Good::Camel => &[],
        }
    }
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub fn bonus_values(sale_size: usize) -> &'static [u32] {
    match sale_size {
        3 => &[3, 3, 2, 2, 2, 1, 1],
        4 => &[6, 6, 5, 5, 4, 4],
        5 => &[10, 10, 9, 8, 8],
        _ => &[],
    }
}

pub fn bonus_sizes() -> std::ops::RangeInclusive<usize> {
    MIN_TRADE_BONUS..=MAX_TRADE_BONUS
}

pub fn initial_deck() -> Vec<Good> {
    let mut deck = Vec::new();
    for g in Good::all_goods() {
        for _ in 0..g.card_count() {
            deck.push(*g);
        }
    }
    deck
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub current_player: usize,
    pub round_wins: [u8; 2],
    pub deck: Vec<Good>,
    pub hands: [Vec<Good>; 2],
    pub tokens: [Vec<u32>; 2],
    pub camels: [u32; 2],
    pub bonus_tokens: [u32; 2],
    pub good_tokens: [u32; 2],
    pub bonuses: HashMap<usize, Vec<u32>>,
    pub goods: HashMap<Good, Vec<u32>>,
    pub market: Vec<Good>,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PubState {
    pub current_player: usize,
    pub round_wins: [u8; 2],
    pub market: Vec<Good>,
    pub deck_len: usize,
    pub camels: [u32; 2],
    pub hand_sizes: [usize; 2],
    pub token_counts: [usize; 2],
    pub goods: HashMap<Good, Vec<u32>>,
    pub bonuses: HashMap<usize, usize>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub hand: Vec<Good>,
}

pub fn opponent(player: usize) -> usize {
    (player + 1) % 2
}

impl Game {
    fn start_round(&mut self) -> Vec<Log> {
        let mut logs: Vec<Log> = vec![Log::public(vec![
            N::text("It is the start of the round, starting market with "),
            N::Bold(vec![N::Fg(
                Good::Camel.color().into(),
                vec![N::text("3 camels")],
            )]),
        ])];
        self.deck = initial_deck();
        self.deck.shuffle(&mut self.rng);
        self.market = vec![Good::Camel, Good::Camel, Good::Camel];
        self.replenish_market(&mut logs);

        self.camels = [0, 0];
        self.bonus_tokens = [0, 0];
        self.good_tokens = [0, 0];
        self.hands = [vec![], vec![]];
        self.tokens = [vec![], vec![]];
        for p in 0..NUM_PLAYERS {
            let hand: Vec<Good> = self.deck.drain(..5).collect();
            logs.extend(self.receive_cards(p, hand));
            self.tokens[p] = vec![];
        }

        self.goods = HashMap::new();
        for good in Good::trade_goods() {
            self.goods.insert(*good, good.token_values().to_vec());
        }

        self.bonuses = HashMap::new();
        for size in bonus_sizes() {
            let mut pile = bonus_values(size).to_vec();
            pile.shuffle(&mut self.rng);
            self.bonuses.insert(size, pile);
        }

        logs
    }

    fn replenish_market(&mut self, logs: &mut Vec<Log>) -> bool {
        let n = 5usize.saturating_sub(self.market.len());
        if n == 0 {
            return true;
        }
        if self.deck.len() < n {
            self.end_round(logs);
            return false;
        }
        let drawn: Vec<Good> = self.deck.drain(..n).collect();
        let mut display_drawn = drawn.clone();
        display_drawn.sort_by_key(|g| *g as u8);
        let mut drawn_content = vec![N::text("Drew ")];
        drawn_content.extend(brdgme_markup::comma_list_and(&render_goods_items(
            &display_drawn,
        )));
        drawn_content.push(N::text(" from the deck and added "));
        drawn_content.push(N::text(if n == 1 { "it" } else { "them" }));
        drawn_content.push(N::text(" to the market"));
        logs.push(Log::public(drawn_content));
        self.market.extend(drawn);
        true
    }

    fn receive_cards(&mut self, player: usize, mut cards: Vec<Good>) -> Vec<Log> {
        cards.sort_by_key(|g| *g as u8);

        let mut logs = vec![];
        let mut drawn_goods = 0u32;
        let mut drawn_camels = 0u32;
        for &c in &cards {
            match c {
                Good::Camel => {
                    self.camels[player] += 1;
                    drawn_camels += 1;
                }
                _ => {
                    self.hands[player].push(c);
                    drawn_goods += 1;
                }
            }
        }
        let mut drew_content = vec![N::text("You drew ")];
        drew_content.extend(brdgme_markup::comma_list_and(&render_goods_items(&cards)));
        logs.push(Log::private(drew_content, vec![player]));

        let other = opponent(player);
        logs.push(Log::private(
            vec![
                N::Player(player),
                N::text(" drew "),
                N::Bold(vec![N::text(format!("{}", drawn_goods))]),
                N::text(" goods and "),
                N::Bold(vec![N::text(format!("{}", drawn_camels))]),
                N::text(" camels"),
            ],
            vec![other],
        ));
        logs
    }

    fn can_take(&self, player: usize) -> bool {
        self.current_player == player && !self.is_finished()
    }

    pub fn take_camels(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_take(player) {
            return Err(GameError::invalid_input("can't take at the moment"));
        }
        let num_camels = self.market.iter().filter(|&&g| g == Good::Camel).count();
        if num_camels == 0 {
            return Err(GameError::invalid_input(
                "there are no camels in the market",
            ));
        }
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            N::Bold(vec![N::Fg(
                Good::Camel.color().into(),
                vec![N::text(format!(
                    "{} {}",
                    num_camels,
                    if num_camels == 1 { "camel" } else { "camels" }
                ))],
            )]),
            N::text(" from the market"),
        ])];
        self.camels[player] += num_camels as u32;
        self.market.retain(|&g| g != Good::Camel);
        let mut replenish_logs = vec![];
        if self.replenish_market(&mut replenish_logs) {
            self.next_player();
        }
        logs.extend(replenish_logs);
        Ok(logs)
    }

    pub fn take_goods(
        &mut self,
        player: usize,
        take_goods: Vec<Good>,
        give_goods: Vec<Good>,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_take(player) {
            return Err(GameError::invalid_input("can't take at the moment"));
        }
        let num_take = take_goods.len();
        let num_give = give_goods.len();
        if num_take == 0 {
            return Err(GameError::invalid_input("you must specify a good to take"));
        }
        if num_take == 1 && num_give != 0 {
            return Err(GameError::invalid_input(
                "if you are taking a single good you can't put any back",
            ));
        }
        if num_take > 1 && num_take != num_give {
            return Err(GameError::invalid_input(
                "if you are taking more than one good you must trade for the same number of goods in your hand, eg take gold dia for lea lea",
            ));
        }
        if take_goods.contains(&Good::Camel) {
            return Err(GameError::invalid_input(
                "the only way to take camels is using \"take camel\" which will take all the camels from the market and replace them with cards drawn from the deck",
            ));
        }

        let mut hand_size_after = self.hands[player].len() + num_take;
        for &g in &give_goods {
            if g != Good::Camel {
                hand_size_after = hand_size_after.saturating_sub(1);
            }
        }
        if hand_size_after > HAND_SIZE {
            return Err(GameError::invalid_input(
                "that would exceed your hand size of 7",
            ));
        }

        let mut take_map: HashMap<Good, usize> = HashMap::new();
        for &g in &take_goods {
            *take_map.entry(g).or_insert(0) += 1;
        }
        for &g in &give_goods {
            if let Some(&count) = take_map.get(&g)
                && count > 0
            {
                return Err(GameError::invalid_input(
                    "you can't trade the same type of good",
                ));
            }
        }

        let mut market_map: HashMap<Good, usize> = HashMap::new();
        for &g in &self.market {
            *market_map.entry(g).or_insert(0) += 1;
        }
        for (&good, &n) in &take_map {
            let available = market_map.get(&good).copied().unwrap_or(0);
            if available < n {
                return Err(GameError::invalid_input(format!(
                    "the market only has {} {}",
                    available,
                    good.name()
                )));
            }
        }

        let mut for_map: HashMap<Good, usize> = HashMap::new();
        for &g in &give_goods {
            *for_map.entry(g).or_insert(0) += 1;
        }
        let mut hand_map: HashMap<Good, usize> = HashMap::new();
        for &g in &self.hands[player] {
            *hand_map.entry(g).or_insert(0) += 1;
        }
        hand_map.insert(Good::Camel, self.camels[player] as usize);
        for (&good, &n) in &for_map {
            let available = hand_map.get(&good).copied().unwrap_or(0);
            if available < n {
                return Err(GameError::invalid_input(format!(
                    "you only have {} {}",
                    available,
                    good.name()
                )));
            }
        }

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            N::Bold(vec![N::text(format!("{} cards", num_take))]),
        ])];

        for (&good, &n) in &take_map {
            for _ in 0..n {
                if let Some(pos) = self.market.iter().position(|&g| g == good) {
                    self.market.remove(pos);
                }
            }
        }

        for (&good, &n) in &for_map {
            for _ in 0..n {
                match good {
                    Good::Camel => {
                        self.camels[player] = self.camels[player].saturating_sub(1);
                    }
                    _ => {
                        if let Some(pos) = self.hands[player].iter().position(|&g| g == good) {
                            self.hands[player].remove(pos);
                        }
                    }
                }
            }
        }

        self.market.extend(give_goods);
        self.hands[player].extend(take_goods);

        let mut replenish_logs = vec![];
        if self.replenish_market(&mut replenish_logs) {
            self.next_player();
        }
        logs.extend(replenish_logs);
        Ok(logs)
    }

    fn can_sell(&self, player: usize) -> bool {
        self.current_player == player && !self.is_finished()
    }

    pub fn sell(
        &mut self,
        player: usize,
        good: Good,
        quantity: usize,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_sell(player) {
            return Err(GameError::invalid_input("you can't sell at the moment"));
        }
        if good == Good::Camel {
            return Err(GameError::invalid_input("can't sell that good type"));
        }
        let min_sales = good.min_sale() as usize;
        if quantity < min_sales {
            return Err(GameError::invalid_input(format!(
                "the minimum amount you can sell of that good is {}",
                min_sales,
            )));
        }
        let in_hand = self.hands[player].iter().filter(|&&g| g == good).count();
        if quantity > in_hand {
            return Err(GameError::invalid_input(format!(
                "you only have {} of that good",
                in_hand
            )));
        }

        let goods_pile = self.goods.entry(good).or_default();
        let num_tokens = quantity.min(goods_pile.len());
        let points: u32 = goods_pile[..num_tokens].iter().sum();
        self.tokens[player].extend_from_slice(&goods_pile[..num_tokens]);
        goods_pile.drain(..num_tokens);
        self.good_tokens[player] += num_tokens as u32;

        let mut suffix = String::new();
        let mut bonus_taken: Option<u32> = None;
        if let Some(bonuses) = self.bonuses.get_mut(&quantity)
            && let Some(bonus) = bonuses.first().copied()
        {
            self.tokens[player].push(bonus);
            self.bonus_tokens[player] += 1;
            bonuses.remove(0);
            suffix = " and took a bonus token".to_string();
            bonus_taken = Some(bonus);
        }

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" sold "),
            N::Bold(vec![N::Fg(
                good.color().into(),
                vec![N::text(format!(
                    "{} {}",
                    quantity,
                    if quantity == 1 {
                        good.name()
                    } else {
                        good.plural()
                    }
                ))],
            )]),
            N::text(format!(
                " for {} {}{}",
                points,
                if points == 1 { "point" } else { "points" },
                suffix,
            )),
        ])];

        if let Some(bonus) = bonus_taken {
            logs.push(Log::private(
                vec![N::text(format!("The bonus token was {} points", bonus))],
                vec![player],
            ));
        }

        for _ in 0..quantity {
            if let Some(pos) = self.hands[player].iter().position(|&g| g == good) {
                self.hands[player].remove(pos);
            }
        }

        let emptied = Good::trade_goods()
            .iter()
            .filter(|g| self.goods.get(g).is_none_or(|v| v.is_empty()))
            .count();
        if emptied >= 3 {
            self.end_round(&mut logs);
        } else {
            self.next_player();
        }
        Ok(logs)
    }

    fn end_round(&mut self, logs: &mut Vec<Log>) {
        let camel_winner = if self.camels[0] > self.camels[1] {
            Some(0)
        } else if self.camels[1] > self.camels[0] {
            Some(1)
        } else {
            None
        };
        if let Some(cw) = camel_winner {
            logs.push(Log::public(vec![
                N::Player(cw),
                N::text(format!(
                    " won the 5 point camel bonus for having {} camels, ",
                    self.camels[cw]
                )),
                N::Player(opponent(cw)),
                N::text(format!(" had {}", self.camels[opponent(cw)])),
            ]));
            self.tokens[cw].push(CAMEL_BONUS_POINTS);
            self.bonus_tokens[cw] += 1;
        }

        let mut scores = [0u32; 2];
        for (p, score) in scores.iter_mut().enumerate() {
            *score = self.tokens[p].iter().sum();
            logs.push(Log::public(vec![
                N::Player(p),
                N::text(format!(
                    " had {} points from {} bonus tokens and {} good tokens",
                    *score, self.bonus_tokens[p], self.good_tokens[p],
                )),
            ]));
        }

        let winner = if scores[0] > scores[1] {
            Some(0)
        } else if scores[1] > scores[0] {
            Some(1)
        } else if self.bonus_tokens[0] > self.bonus_tokens[1] {
            Some(0)
        } else if self.bonus_tokens[1] > self.bonus_tokens[0] {
            Some(1)
        } else if self.good_tokens[0] > self.good_tokens[1] {
            Some(0)
        } else if self.good_tokens[1] > self.good_tokens[0] {
            Some(1)
        } else {
            None
        };

        if let Some(w) = winner {
            logs.push(Log::public(vec![N::Player(w), N::text(" won the round")]));
            self.round_wins[w] += 1;
        } else {
            logs.push(Log::public(vec![N::text(
                "Against all odds, the round was tied and will be replayed",
            )]));
        }

        if !self.is_finished() {
            let l = self.start_round();
            logs.extend(l);
        }
    }

    fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % 2;
    }

    pub fn is_finished(&self) -> bool {
        self.round_wins[0] == 2 || self.round_wins[1] == 2
    }

    fn winners(&self) -> Vec<usize> {
        if self.round_wins[0] == 2 {
            vec![0]
        } else if self.round_wins[1] == 2 {
            vec![1]
        } else {
            vec![]
        }
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != NUM_PLAYERS {
            return Err(GameError::PlayerCount {
                min: NUM_PLAYERS,
                max: NUM_PLAYERS,
                given: players,
            });
        }
        let mut g = Game {
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
        let logs = g.start_round();
        Ok((g, logs))
    }

    fn status(&self) -> Status {
        if self.is_finished() {
            let winner = self.winners();
            let metrics: Vec<Vec<i32>> = (0..NUM_PLAYERS)
                .map(|p| vec![i32::from(winner.contains(&p))])
                .collect();
            Status::Finished {
                placings: gen_placings(&metrics),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: vec![self.current_player],
                eliminated: vec![],
            }
        }
    }

    fn pub_state(&self) -> Self::PubState {
        let bonuses: HashMap<usize, usize> =
            self.bonuses.iter().map(|(&k, v)| (k, v.len())).collect();
        PubState {
            current_player: self.current_player,
            round_wins: self.round_wins,
            market: self.market.clone(),
            deck_len: self.deck.len(),
            camels: self.camels,
            hand_sizes: [self.hands[0].len(), self.hands[1].len()],
            token_counts: [self.tokens[0].len(), self.tokens[1].len()],
            goods: self.goods.clone(),
            bonuses,
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
                value: Command::Take { take, give },
                ..
            }) => {
                let logs = if take.len() == 1 && take[0] == Good::Camel && give.is_empty() {
                    self.take_camels(player)?
                } else {
                    self.take_goods(player, take, give)?
                };
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Sell { good, quantity },
                ..
            }) => {
                let logs = self.sell(player, good, quantity)?;
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
        (0..NUM_PLAYERS)
            .map(|p| self.tokens[p].iter().sum::<u32>() as f32)
            .collect()
    }

    fn player_counts() -> Vec<usize> {
        vec![NUM_PLAYERS]
    }

    fn player_count(&self) -> usize {
        NUM_PLAYERS
    }

    fn rules() -> String {
        include_str!("../RULES.md").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_has_52_cards() {
        assert_eq!(initial_deck().len(), 52);
    }

    #[test]
    fn deck_composition_matches_card_counts() {
        let deck = initial_deck();
        for g in Good::all_goods() {
            assert_eq!(
                deck.iter().filter(|&&c| c == *g).count(),
                g.card_count() as usize,
                "wrong count for {}",
                g.name()
            );
        }
    }

    #[test]
    fn trade_goods_are_six_without_camel() {
        let tg = Good::trade_goods();
        assert_eq!(tg.len(), 6);
        assert!(!tg.contains(&Good::Camel));
    }

    #[test]
    fn all_goods_includes_camel() {
        assert!(Good::all_goods().contains(&Good::Camel));
    }

    #[test]
    fn rare_goods_require_min_2_sale() {
        assert_eq!(Good::Diamond.min_sale(), 2);
        assert_eq!(Good::Gold.min_sale(), 2);
        assert_eq!(Good::Silver.min_sale(), 2);
    }

    #[test]
    fn common_goods_require_min_1_sale() {
        assert_eq!(Good::Cloth.min_sale(), 1);
        assert_eq!(Good::Spice.min_sale(), 1);
        assert_eq!(Good::Leather.min_sale(), 1);
    }

    #[test]
    fn token_piles_have_correct_lengths() {
        assert_eq!(Good::Diamond.token_values().len(), 5);
        assert_eq!(Good::Gold.token_values().len(), 5);
        assert_eq!(Good::Silver.token_values().len(), 5);
        assert_eq!(Good::Cloth.token_values().len(), 7);
        assert_eq!(Good::Spice.token_values().len(), 7);
        assert_eq!(Good::Leather.token_values().len(), 9);
        assert_eq!(Good::Camel.token_values().len(), 0);
    }

    #[test]
    fn bonus_piles_have_correct_lengths() {
        assert_eq!(bonus_values(3).len(), 7);
        assert_eq!(bonus_values(4).len(), 6);
        assert_eq!(bonus_values(5).len(), 5);
    }

    #[test]
    fn bonus_sizes_are_3_to_5() {
        assert_eq!(*bonus_sizes().start(), 3);
        assert_eq!(*bonus_sizes().end(), 5);
    }

    #[test]
    fn start_validates_two_players() {
        assert!(Game::start(1, 0).is_err());
        assert!(Game::start(3, 0).is_err());
        assert!(Game::start(2, 0).is_ok());
    }

    #[test]
    fn start_deck_is_40() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.deck.len(), 40);
    }

    #[test]
    fn start_each_player_has_5_cards_total() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.hands[0].len() + g.camels[0] as usize, 5);
        assert_eq!(g.hands[1].len() + g.camels[1] as usize, 5);
    }

    #[test]
    fn start_goods_map_has_6_entries() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.goods.len(), 6);
    }

    #[test]
    fn start_bonuses_has_3_size_tiers_with_correct_lengths() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.bonuses.len(), 3);
        assert_eq!(g.bonuses[&3].len(), 7);
        assert_eq!(g.bonuses[&4].len(), 6);
        assert_eq!(g.bonuses[&5].len(), 5);
    }

    #[test]
    fn start_market_has_5_cards() {
        let (g, _) = Game::start(2, 0).unwrap();
        assert_eq!(g.market.len(), 5);
    }

    #[test]
    fn state_round_trips_through_json() {
        let (g, _) = Game::start(2, 42).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let decoded: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(g.deck, decoded.deck);
        assert_eq!(g.market, decoded.market);
        assert_eq!(g.hands, decoded.hands);
        assert_eq!(g.camels, decoded.camels);
        assert_eq!(g.round_wins, decoded.round_wins);
        assert_eq!(g.current_player, decoded.current_player);
    }

    #[test]
    fn command_parser_returns_none_when_game_finished() {
        let mut g = Game::start(2, 0).unwrap().0;
        g.round_wins = [2, 0];
        assert!(g.command_parser(0).is_none());
        assert!(g.command_parser(1).is_none());
    }

    #[test]
    fn command_parser_returns_none_for_wrong_player() {
        let g = Game::start(2, 0).unwrap().0;
        let current = g.current_player;
        assert!(g.command_parser(current).is_some());
        assert!(g.command_parser(opponent(current)).is_none());
    }

    #[test]
    fn take_parser_parses_single_camel() {
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let output = parser.parse("take camel", &[]).unwrap();
        match output.value {
            Command::Take { ref take, ref give } => {
                assert_eq!(take, &[Good::Camel]);
                assert!(give.is_empty());
            }
            _ => panic!("expected Take command"),
        }
    }

    #[test]
    fn take_parser_parses_single_good() {
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let output = parser.parse("take dia", &[]).unwrap();
        match output.value {
            Command::Take { ref take, ref give } => {
                assert!(take.contains(&Good::Diamond));
                assert!(give.is_empty());
            }
            _ => panic!("expected Take command"),
        }
    }

    #[test]
    fn take_parser_parses_trade_with_for() {
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let output = parser.parse("take dia silv for lea lea", &[]).unwrap();
        match output.value {
            Command::Take { ref take, ref give } => {
                assert!(take.contains(&Good::Diamond));
                assert!(take.contains(&Good::Silver));
                assert_eq!(give.iter().filter(|&&g| g == Good::Leather).count(), 2);
            }
            _ => panic!("expected Take command"),
        }
    }

    #[test]
    fn sell_parser_parses_quantity_prefix() {
        let g = Game::start(2, 0).unwrap().0;
        let parser = g.command_parser(g.current_player).unwrap();
        let output = parser.parse("sell 2 gold", &[]).unwrap();
        match output.value {
            Command::Sell { good, quantity } => {
                assert_eq!(good, Good::Gold);
                assert_eq!(quantity, 2);
            }
            _ => panic!("expected Sell command"),
        }
    }

    #[test]
    fn take_camel_with_no_camels_in_market_errors() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.market.retain(|&g| g != Good::Camel);
        assert!(g.take_camels(player).is_err());
    }

    #[test]
    fn take_goods_rejects_single_camel_without_mutation() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        let market_before = g.market.clone();
        let hand_before = g.hands[player].clone();
        let camels_before = g.camels[player];
        let result = g.take_goods(player, vec![Good::Camel], vec![]);
        assert!(result.is_err());
        assert_eq!(g.market, market_before);
        assert_eq!(g.hands[player], hand_before);
        assert_eq!(g.camels[player], camels_before);
    }

    #[test]
    fn sell_below_minimum_errors() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Gold];
        assert!(g.sell(player, Good::Gold, 2).is_err());
        assert!(g.sell(player, Good::Gold, 1).is_err());
    }

    #[test]
    fn sell_succeeds_and_collects_tokens() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Gold, Good::Leather, Good::Gold];
        let logs = g.sell(player, Good::Gold, 2).unwrap();
        assert_eq!(g.tokens[player], vec![6, 6]);
        assert_eq!(g.goods.get(&Good::Gold).unwrap(), &vec![5, 5, 5]);
        assert_eq!(g.hands[player], vec![Good::Leather]);
        assert!(!logs.is_empty());
    }

    #[test]
    fn camel_bonus_awarded_to_player_with_more_camels() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.round_wins = [1, 0];
        g.camels = [3, 0];
        g.tokens[0] = vec![];
        g.tokens[1] = vec![];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert_eq!(g.round_wins[0], 2);
        assert_eq!(g.round_wins[1], 0);
    }

    #[test]
    fn camel_bonus_not_awarded_on_tie() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.round_wins = [1, 0];
        g.camels = [2, 2];
        g.tokens[0] = vec![10];
        g.tokens[1] = vec![];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert!(!g.tokens[0].contains(&CAMEL_BONUS_POINTS));
        assert!(!g.tokens[1].contains(&CAMEL_BONUS_POINTS));
    }

    #[test]
    fn round_win_increments_round_wins() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.camels = [0, 0];
        g.tokens[0] = vec![10, 10];
        g.tokens[1] = vec![5];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert_eq!(g.round_wins[0], 1);
        assert_eq!(g.round_wins[1], 0);
    }

    #[test]
    fn match_finishes_after_two_round_wins() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.round_wins = [2, 0];
        assert!(g.is_finished());
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings, vec![1, 2]),
            _ => panic!("expected finished"),
        }
    }

    #[test]
    fn finished_game_rejects_commands() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.round_wins = [2, 0];
        assert!(g.command_parser(0).is_none());
        assert!(g.command_parser(1).is_none());
    }

    #[test]
    fn sell_cannot_sell_camels() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        assert!(g.sell(player, Good::Camel, 1).is_err());
    }

    #[test]
    fn sell_more_than_held_errors() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Leather];
        assert!(g.sell(player, Good::Leather, 3).is_err());
    }

    #[test]
    fn tie_breaker_bonus_tokens_goes_to_higher_count() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.camels = [0, 0];
        g.tokens[0] = vec![10];
        g.tokens[1] = vec![10];
        g.bonus_tokens = [3, 1];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert_eq!(g.round_wins[0], 1);
    }

    #[test]
    fn tie_breaker_good_tokens_goes_to_higher_count() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.camels = [0, 0];
        g.tokens[0] = vec![10];
        g.tokens[1] = vec![10];
        g.bonus_tokens = [1, 1];
        g.good_tokens = [5, 3];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert_eq!(g.round_wins[0], 1);
    }

    #[test]
    fn full_tie_replays_round() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let current_round_wins = g.round_wins;
        g.camels = [0, 0];
        g.tokens[0] = vec![10];
        g.tokens[1] = vec![10];
        g.bonus_tokens = [1, 1];
        g.good_tokens = [3, 3];
        let mut logs = vec![];
        g.end_round(&mut logs);
        assert_eq!(g.round_wins, current_round_wins);
    }

    #[test]
    fn take_command_response_can_undo_is_false() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.market = vec![
            Good::Diamond,
            Good::Leather,
            Good::Leather,
            Good::Leather,
            Good::Leather,
        ];
        let response = g.command(player, "take dia", &[]).unwrap();
        assert!(!response.can_undo);
    }

    #[test]
    fn sell_command_response_can_undo_is_false() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Gold, Good::Gold];
        let response = g.command(player, "sell 2 gold", &[]).unwrap();
        assert!(!response.can_undo);
    }

    #[test]
    fn sell_with_bonus_includes_private_bonus_log() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Leather, Good::Leather, Good::Leather];
        let logs = g.sell(player, Good::Leather, 3).unwrap();
        let has_private_to_player = logs.iter().any(|l| !l.public && l.to == vec![player]);
        assert!(
            has_private_to_player,
            "expected a private log addressed to the selling player"
        );
    }

    #[test]
    fn pub_state_renders_without_panicking() {
        let (g, _) = Game::start(2, 0).unwrap();
        use brdgme_game::Renderer;
        let rendered = g.pub_state().render();
        assert!(!rendered.is_empty());
    }

    #[test]
    fn player_state_renders_without_panicking() {
        let (g, _) = Game::start(2, 0).unwrap();
        use brdgme_game::Renderer;
        let rendered = g.player_state(0).render();
        assert!(!rendered.is_empty());
    }

    #[test]
    fn player_state_renders_own_hand() {
        let (g, _) = Game::start(2, 0).unwrap();
        use brdgme_game::Renderer;
        let rendered = g.player_state(0).render();
        let markup = brdgme_markup::to_string(&rendered);
        assert!(markup.contains("You have"));
    }

    #[test]
    fn pub_state_does_not_leak_hand_contents() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Diamond, Good::Gold, Good::Silver];
        g.hands[opponent(player)] = vec![Good::Cloth, Good::Spice];

        let ps = g.pub_state();
        let json = serde_json::to_value(&ps).unwrap();
        assert!(
            json.get("hand").is_none(),
            "PubState must not contain a hand field"
        );
        assert_eq!(
            json["hand_sizes"][player].as_u64().unwrap() as usize,
            g.hands[player].len()
        );
        assert_eq!(
            json["hand_sizes"][opponent(player)].as_u64().unwrap() as usize,
            g.hands[opponent(player)].len()
        );
    }

    #[test]
    fn token_table_renders_bottom_up_source_order() {
        use brdgme_game::Renderer;

        let mut goods: HashMap<Good, Vec<u32>> = HashMap::new();
        goods.insert(Good::Diamond, vec![7, 5, 5]);
        for g in Good::trade_goods() {
            if *g != Good::Diamond {
                goods.insert(*g, vec![]);
            }
        }
        let ps = PubState {
            goods,
            ..PubState::default()
        };
        let rendered = ps.render();
        let markup = brdgme_markup::to_string(&rendered);
        let pos5 = markup.find("{{b}}5{{/b}}").unwrap();
        let pos7 = markup.find("{{b}}7{{/b}}").unwrap();
        assert!(
            pos5 < pos7,
            "5 should be above 7 in token table, got positions 5={pos5} 7={pos7}"
        );
    }

    #[test]
    fn goods_comma_list_follows_natural_language() {
        use brdgme_markup::{comma_list_and, plain, transform};

        let render_plain = |goods: &[Good]| -> String {
            plain(&transform(&comma_list_and(&render_goods_items(goods)), &[]))
        };

        assert_eq!(render_plain(&[Good::Diamond]), "diamond");
        assert_eq!(
            render_plain(&[Good::Diamond, Good::Gold]),
            "diamond and gold"
        );
        assert_eq!(
            render_plain(&[Good::Diamond, Good::Gold, Good::Silver]),
            "diamond, gold and silver"
        );
    }

    #[test]
    fn pub_state_camels_are_exact() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.camels = [5, 0];
        let ps = g.pub_state();
        assert_eq!(ps.camels, [5, 0]);
    }

    #[test]
    fn player_state_includes_own_hand_and_player_index() {
        let (g, _) = Game::start(2, 0).unwrap();
        let ps0 = g.player_state(0);
        assert_eq!(ps0.player, 0);
        assert_eq!(ps0.hand, g.hands[0]);
        let ps1 = g.player_state(1);
        assert_eq!(ps1.player, 1);
        assert_eq!(ps1.hand, g.hands[1]);
    }

    #[test]
    fn take_with_for_same_type_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Diamond, Good::Diamond, Good::Diamond];
        g.market = vec![
            Good::Diamond,
            Good::Diamond,
            Good::Silver,
            Good::Camel,
            Good::Camel,
        ];
        assert!(
            g.take_goods(
                player,
                vec![Good::Diamond, Good::Diamond],
                vec![Good::Diamond, Good::Diamond]
            )
            .is_err()
        );
    }

    #[test]
    fn take_exceeding_hand_size_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Gold; 5];
        g.market = vec![
            Good::Diamond,
            Good::Diamond,
            Good::Diamond,
            Good::Diamond,
            Good::Diamond,
        ];
        assert!(
            g.take_goods(
                player,
                vec![Good::Diamond, Good::Diamond, Good::Diamond],
                vec![]
            )
            .is_err()
        );
    }

    #[test]
    fn take_insufficient_market_stock_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.market = vec![
            Good::Diamond,
            Good::Camel,
            Good::Camel,
            Good::Camel,
            Good::Camel,
        ];
        assert!(
            g.take_goods(
                player,
                vec![Good::Diamond, Good::Diamond],
                vec![Good::Leather, Good::Leather]
            )
            .is_err()
        );
    }

    #[test]
    fn take_insufficient_hand_stock_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Leather];
        g.market = vec![
            Good::Diamond,
            Good::Diamond,
            Good::Silver,
            Good::Camel,
            Good::Camel,
        ];
        assert!(
            g.take_goods(
                player,
                vec![Good::Diamond, Good::Diamond],
                vec![Good::Leather, Good::Leather]
            )
            .is_err()
        );
    }

    #[test]
    fn take_single_good_with_for_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        assert!(
            g.take_goods(player, vec![Good::Diamond], vec![Good::Leather])
                .is_err()
        );
    }

    #[test]
    fn take_multi_good_count_mismatch_rejected() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.market = vec![
            Good::Diamond,
            Good::Diamond,
            Good::Silver,
            Good::Camel,
            Good::Camel,
        ];
        assert!(
            g.take_goods(
                player,
                vec![Good::Diamond, Good::Diamond],
                vec![Good::Leather]
            )
            .is_err()
        );
    }

    #[test]
    fn sell_3_goods_emptied_triggers_end_round() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        let player = g.current_player;
        let opp = opponent(player);
        g.round_wins = [0, 0];
        g.round_wins[player] = 1;
        g.tokens[player] = vec![10, 10];
        g.tokens[opp] = vec![];
        g.camels = [0, 0];
        g.bonus_tokens = [0, 0];
        g.good_tokens = [0, 0];
        g.goods.insert(Good::Diamond, vec![]);
        g.goods.insert(Good::Gold, vec![]);
        g.goods.insert(Good::Silver, vec![5]);
        g.goods.insert(Good::Cloth, vec![1]);
        g.hands[player] = vec![Good::Cloth];
        let _ = g.sell(player, Good::Cloth, 1).unwrap();
        assert!(g.is_finished());
        assert!(g.goods[&Good::Cloth].is_empty());
        assert_eq!(g.round_wins[player], 2);
    }

    #[test]
    fn take_no_goods_specified_errors() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        assert!(g.take_goods(player, vec![], vec![]).is_err());
    }

    #[test]
    fn points_reflects_running_token_total() {
        let (mut g, _) = Game::start(2, 0).unwrap();
        g.tokens[0] = vec![3, 5, 7];
        g.tokens[1] = vec![2, 2];
        let points = g.points();
        assert_eq!(points[0], 15.0);
        assert_eq!(points[1], 4.0);
    }

    #[test]
    fn command_spec_returns_none_for_wrong_player() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let current = g.current_player;
        assert!(g.command_spec(current).is_some());
        assert!(g.command_spec(opponent(current)).is_none());
        g.round_wins = [2, 0];
        assert!(g.command_spec(0).is_none());
    }

    #[test]
    fn replenish_log_includes_drawn_goods_identities() {
        let mut g = Game {
            rng: GameRng::seed_from_u64(0),
            ..Game::default()
        };
        g.market = vec![Good::Camel, Good::Camel];
        g.deck = vec![Good::Diamond, Good::Gold, Good::Leather];
        let mut logs = vec![];
        assert!(g.replenish_market(&mut logs));
        assert_eq!(g.market.len(), 5);
        assert!(g.market.contains(&Good::Diamond));
        assert!(g.market.contains(&Good::Gold));
        assert!(g.market.contains(&Good::Leather));

        let pub_logs: Vec<&Log> = logs.iter().filter(|l| l.public).collect();
        assert_eq!(pub_logs.len(), 1, "expected exactly one public log");
        let log_str = brdgme_markup::plain(&brdgme_markup::transform(&pub_logs[0].content, &[]));
        assert_eq!(
            log_str, "Drew diamond, gold and leather from the deck and added them to the market",
            "goods should be joined with a natural-language comma list, got: {log_str}"
        );
    }

    #[test]
    fn start_round_replenish_public_log_identifies_goods() {
        let (g, logs) = Game::start(2, 42).unwrap();
        let pub_logs: Vec<&Log> = logs.iter().filter(|l| l.public).collect();
        let start_log_str = brdgme_markup::to_string(&pub_logs[0].content);
        assert!(start_log_str.contains("It is the start of the round"),);
        let repl_log = pub_logs.get(1).expect("should have a replenish public log");
        let repl_str = brdgme_markup::to_string(&repl_log.content);
        let non_camel_in_market: Vec<_> = g.market.iter().filter(|&&g| g != Good::Camel).collect();
        for good in &non_camel_in_market {
            assert!(
                repl_str.contains(good.name()),
                "replenish log should contain {name}, got: {repl_str}",
                name = good.name()
            );
        }
    }

    #[test]
    fn command_preserves_remaining_input() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let player = g.current_player;
        g.hands[player] = vec![Good::Gold, Good::Leather, Good::Gold];
        let resp = g.command(player, "sell 2 gold and then", &[]).unwrap();
        assert_eq!(resp.remaining_input, " and then");
        assert!(!resp.can_undo);
    }
}
