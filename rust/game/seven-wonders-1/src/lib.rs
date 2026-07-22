pub mod card;
pub mod command;
pub mod render;

pub use card::*;
pub use command::Command;

use std::collections::HashMap;

use brdgme_cost::{Cost, can_afford_perm};
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status, placings_log};
use brdgme_markup::Node as N;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 3;
pub const MAX_PLAYERS: usize = 7;
const TAVERN_COINS: i32 = 5;
const DISCARD_COINS: i32 = 3;
const BASE_TRADE_COST: i32 = 2;
const DISCOUNTED_TRADE_COST: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Build {
        card: usize,
        free: bool,
        wonder: bool,
        deal: Option<usize>,
        chosen: bool,
    },
    Discard {
        card: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolver {
    DrawDiscard { player: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub round: u8,
    pub finished: bool,
    pub hands: Vec<Vec<Card>>,
    pub discard: Vec<Card>,
    pub actions: Vec<Option<Action>>,
    pub to_resolve: Vec<Resolver>,
    pub cards: Vec<Vec<Card>>,
    pub coins: Vec<i32>,
    pub victory_tokens: Vec<i32>,
    pub defeat_tokens: Vec<i32>,
    pub cities: Vec<City>,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubState {
    /// Number of players (3-7).
    pub players: usize,
    /// Current age (1, 2, or 3).
    pub round: u8,
    /// True when the game is over (all 3 ages complete).
    pub finished: bool,
    /// Number of cards in the shared discard pile.
    pub discard_count: usize,
    /// Cards each player has built, indexed by player.
    pub cards: Vec<Vec<Card>>,
    /// Coins each player holds, indexed by player.
    pub coins: Vec<i32>,
    /// Victory tokens (from military wins and VP cards) per player.
    pub victory_tokens: Vec<i32>,
    /// Defeat tokens (from military losses) per player. Each defeat token is -1 VP.
    pub defeat_tokens: Vec<i32>,
    /// The wonder city assigned to each player (determines wonder stages and starting resource).
    pub cities: Vec<City>,
    /// Number of cards in each player's current hand, indexed by player.
    pub hand_sizes: Vec<usize>,
    /// Whether each player has chosen their action for this hand, indexed by player.
    pub actions_chosen: Vec<bool>,
    /// If set, this player must resolve a DrawDiscard effect (take a card from the discard pile).
    pub to_resolve_player: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// The full public game state.
    pub public: PubState,
    /// Which player this private state belongs to.
    pub player: usize,
    /// Cards in this player's current hand.
    pub hand: Vec<Card>,
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

        let mut rng = GameRng::seed_from_u64(seed);

        let mut all_cities = cities();
        all_cities.shuffle(&mut rng);
        let assigned_cities: Vec<City> = all_cities[..players].to_vec();

        let mut logs = vec![];
        for (p, city) in assigned_cities.iter().enumerate() {
            logs.push(Log::public(vec![
                N::Player(p),
                N::text(format!(" was assigned {}", city.name)),
            ]));
        }

        let mut g = Game {
            players,
            round: 0,
            finished: false,
            hands: vec![],
            discard: vec![],
            actions: vec![None; players],
            to_resolve: vec![],
            cards: vec![vec![]; players],
            coins: vec![3; players],
            victory_tokens: vec![0; players],
            defeat_tokens: vec![0; players],
            cities: assigned_cities,
            rng,
        };

        let round_logs = g.start_round(1);
        logs.extend(round_logs);

        Ok((g, logs))
    }

    fn start_round(&mut self, round: u8) -> Vec<Log> {
        self.round = round;
        let mut deck = match round {
            1 => deck_age1(self.players),
            2 => deck_age2(self.players),
            _ => deck_age3(self.players, &mut self.rng),
        };
        deck.shuffle(&mut self.rng);

        let per_hand = deck.len() / self.players;
        self.hands = (0..self.players)
            .map(|i| deck[i * per_hand..(i + 1) * per_hand].to_vec())
            .collect();

        self.actions = vec![None; self.players];
        self.to_resolve = vec![];

        for p in 0..self.players {
            for c in &mut self.cards[p] {
                if let CardEffect::FreeBuild { has_built } = &mut c.effect {
                    *has_built = false;
                }
            }
        }

        vec![Log::public(vec![N::text(format!("Age {} begins", round))])]
    }

    fn start_hand(&mut self) -> Vec<Log> {
        self.actions = vec![None; self.players];
        vec![]
    }

    fn end_hand(&mut self) -> Vec<Log> {
        let max_hand = self.hands.iter().map(|h| h.len()).max().unwrap_or(0);

        if max_hand == 0 {
            return self.end_round();
        }

        if max_hand == 1 {
            let mut logs = vec![];
            for p in 0..self.players {
                if self.hands[p].len() == 1 && !self.has_play_final_card(p) {
                    let card = self.hands[p].pop().unwrap();
                    self.discard.push(card);
                    self.coins[p] += DISCARD_COINS;
                    logs.push(Log::public(vec![
                        N::Player(p),
                        N::text(" discarded their last card"),
                    ]));
                }
            }
            let any_cards = self.hands.iter().any(|h| !h.is_empty());
            if !any_cards {
                let er_logs = self.end_round();
                logs.extend(er_logs);
                return logs;
            }
            let sh_logs = self.start_hand();
            logs.extend(sh_logs);
            return logs;
        }

        self.pass_hands();
        self.start_hand()
    }

    fn end_round(&mut self) -> Vec<Log> {
        let mut logs = self.military_conflicts();

        if self.round < 3 {
            let rl = self.start_round(self.round + 1);
            logs.extend(rl);
        } else {
            self.finished = true;
            logs.push(Log::public(vec![N::text("The game is over")]));
        }

        logs
    }

    fn pass_hands(&mut self) {
        let n = self.players;
        let new_hands: Vec<Vec<Card>> = if self.round % 2 == 1 {
            (0..n).map(|i| self.hands[(i + 1) % n].clone()).collect()
        } else {
            (0..n)
                .map(|i| self.hands[(i + n - 1) % n].clone())
                .collect()
        };
        self.hands = new_hands;
    }

    fn check_hand_complete(&mut self) -> Vec<Log> {
        for p in 0..self.players {
            if self.hands[p].is_empty() {
                continue;
            }
            match &self.actions[p] {
                None => return vec![],
                Some(Action::Build { chosen: false, .. }) => return vec![],
                _ => {}
            }
        }

        let mut logs = self.execute_actions();

        if self.to_resolve.is_empty() {
            let eh_logs = self.end_hand();
            logs.extend(eh_logs);
        }

        logs
    }

    fn execute_actions(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let actions: Vec<Option<Action>> = self.actions.clone();

        for (p, action) in actions.iter().enumerate() {
            if let Some(action) = action {
                match action {
                    Action::Build {
                        card,
                        free,
                        wonder,
                        deal,
                        ..
                    } => {
                        let (build_logs, built) =
                            self.execute_build(p, *card, *free, *wonder, *deal);
                        logs.extend(build_logs);
                        if let Some(c) = built {
                            let hl = self.post_build_hook(p, &c);
                            logs.extend(hl);
                        }
                    }
                    Action::Discard { card } => {
                        let dl = self.execute_discard(p, *card);
                        logs.extend(dl);
                    }
                }
            }
        }

        self.actions = vec![None; self.players];
        logs
    }

    fn execute_build(
        &mut self,
        player: usize,
        card_idx: usize,
        free: bool,
        wonder: bool,
        deal: Option<usize>,
    ) -> (Vec<Log>, Option<Card>) {
        let mut logs = vec![];

        if wonder {
            let city = self.cities[player].clone();
            let stages_built = self.cards[player]
                .iter()
                .filter(|c| c.kind == CardKind::Wonder)
                .count();
            let db = card_db();
            let stage_name = city.wonder_stages[stages_built].clone();
            let stage_card = db[&stage_name].clone();

            if !free {
                let deal_map = self.resolve_deal(player, &stage_card.cost, deal);
                self.pay_cost(player, &stage_card.cost, &deal_map);
            }

            let hand_card = self.hands[player].remove(card_idx);
            self.discard.push(hand_card);
            self.cards[player].push(stage_card.clone());

            logs.push(Log::public(vec![
                N::Player(player),
                N::text(format!(" built wonder stage {}", stage_card.name)),
            ]));

            (logs, Some(stage_card))
        } else {
            let card = self.hands[player].remove(card_idx);

            if free {
                for c in &mut self.cards[player] {
                    if let CardEffect::FreeBuild { has_built } = &mut c.effect
                        && !*has_built
                    {
                        *has_built = true;
                        break;
                    }
                }
            } else {
                let deal_map = self.resolve_deal(player, &card.cost, deal);
                self.pay_cost(player, &card.cost, &deal_map);
            }

            self.cards[player].push(card.clone());

            logs.push(Log::public(vec![
                N::Player(player),
                N::text(format!(" built {}", card.name)),
            ]));

            (logs, Some(card))
        }
    }

    fn execute_discard(&mut self, player: usize, card_idx: usize) -> Vec<Log> {
        let card = self.hands[player].remove(card_idx);
        self.discard.push(card.clone());
        self.coins[player] += DISCARD_COINS;
        vec![Log::public(vec![
            N::Player(player),
            N::text(format!(
                " discarded {} for {} coins",
                card.name, DISCARD_COINS
            )),
        ])]
    }

    fn post_build_hook(&mut self, player: usize, card: &Card) -> Vec<Log> {
        let mut logs = vec![];
        match &card.effect {
            CardEffect::Tavern => {
                self.coins[player] += TAVERN_COINS;
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(format!(" gained {} coins from Tavern", TAVERN_COINS)),
                ]));
            }
            CardEffect::Multi { resources } => {
                if let Some(&coins) = resources.0.get(&MultiResource::Coin) {
                    self.coins[player] += coins;
                }
                if let Some(&vp) = resources.0.get(&MultiResource::VP) {
                    self.victory_tokens[player] += vp;
                }
            }
            CardEffect::Bonus {
                target_kinds,
                directions,
                coins,
                ..
            } => {
                if *coins > 0 {
                    let earned = self.bonus_count(player, target_kinds, directions) * coins;
                    if earned > 0 {
                        self.coins[player] += earned;
                        logs.push(Log::public(vec![
                            N::Player(player),
                            N::text(format!(" gained {} coins from {}", earned, card.name)),
                        ]));
                    }
                }
            }
            CardEffect::DrawDiscard { .. } if !self.discard.is_empty() => {
                self.to_resolve.push(Resolver::DrawDiscard { player });
            }
            _ => {}
        }
        logs
    }

    fn resolve_deal(
        &self,
        player: usize,
        cost: &Cost<Good>,
        deal: Option<usize>,
    ) -> HashMap<i32, i32> {
        match deal {
            Some(idx) => {
                let (_, deals) = self.can_afford_cost(player, cost);
                deals.get(idx).cloned().unwrap_or_default()
            }
            None => HashMap::new(),
        }
    }

    fn pay_cost(&mut self, player: usize, cost: &Cost<Good>, deal: &HashMap<i32, i32>) {
        let coin_cost = cost.0.get(&Good::Coin).copied().unwrap_or(0);
        self.coins[player] -= coin_cost;

        for (&dir, &coins) in deal {
            let neighbor = if dir == DIR_LEFT {
                (player + self.players - 1) % self.players
            } else {
                (player + 1) % self.players
            };
            self.coins[player] -= coins;
            self.coins[neighbor] += coins;
        }
    }

    pub fn can_afford_cost(
        &self,
        player: usize,
        cost: &Cost<Good>,
    ) -> (bool, Vec<HashMap<i32, i32>>) {
        let coin_cost = cost.0.get(&Good::Coin).copied().unwrap_or(0);
        if self.coins[player] < coin_cost {
            return (false, vec![]);
        }

        let goods_cost: Cost<Good> = Cost(
            cost.0
                .iter()
                .filter(|(g, _)| **g != Good::Coin)
                .map(|(g, v)| (*g, *v))
                .collect(),
        );

        if goods_cost.is_zero() {
            return (true, vec![HashMap::new()]);
        }

        let own = self.player_goods_options(player);
        let left_player = (player + self.players - 1) % self.players;
        let right_player = (player + 1) % self.players;
        let left = self.player_goods_options(left_player);
        let right = self.player_goods_options(right_player);

        let own_count = own.len();
        let left_count = left.len();

        let mut with = own;
        with.extend(left);
        with.extend(right);

        let (can, allocations) = can_afford_perm(&goods_cost, &with);
        if !can {
            return (false, vec![]);
        }

        let mut deals: Vec<HashMap<i32, i32>> = vec![];
        for alloc in &allocations {
            let mut deal: HashMap<i32, i32> = HashMap::new();
            for (i, c) in alloc.iter().enumerate() {
                let dir = if i < own_count {
                    continue;
                } else if i < own_count + left_count {
                    DIR_LEFT
                } else {
                    DIR_RIGHT
                };
                for (good, amount) in &c.0 {
                    if *amount > 0 {
                        let per_good = self.trade_cost_per_good(player, dir, *good);
                        *deal.entry(dir).or_insert(0) += amount * per_good;
                    }
                }
            }
            let total_deal_cost: i32 = deal.values().sum();
            if self.coins[player] - coin_cost >= total_deal_cost && !deals.contains(&deal) {
                deals.push(deal);
            }
        }

        if deals.is_empty() {
            (false, vec![])
        } else {
            (true, deals)
        }
    }

    pub fn can_build_card(&self, player: usize, card_idx: usize) -> (bool, Vec<HashMap<i32, i32>>) {
        let hand = &self.hands[player];
        if card_idx >= hand.len() {
            return (false, vec![]);
        }
        let card = &hand[card_idx];

        if self.cards[player].iter().any(|c| c.name == card.name) {
            return (false, vec![]);
        }

        for prereq in &card.free_with {
            if self.cards[player].iter().any(|c| &c.name == prereq) {
                return (true, vec![HashMap::new()]);
            }
        }

        self.can_afford_cost(player, &card.cost)
    }

    pub fn can_free_build(&self, player: usize, card_idx: usize) -> bool {
        if !self.has_free_build(player) {
            return false;
        }
        let hand = &self.hands[player];
        if card_idx >= hand.len() {
            return false;
        }
        let card = &hand[card_idx];
        !self.cards[player].iter().any(|c| c.name == card.name)
    }

    pub fn can_build_wonder(&self, player: usize) -> bool {
        let city = &self.cities[player];
        let stages_built = self.cards[player]
            .iter()
            .filter(|c| c.kind == CardKind::Wonder)
            .count();
        if stages_built >= city.wonder_stages.len() {
            return false;
        }
        let db = card_db();
        let stage_name = &city.wonder_stages[stages_built];
        let stage_card = &db[stage_name];
        let (can, _) = self.can_afford_cost(player, &stage_card.cost);
        can
    }

    pub fn has_free_build(&self, player: usize) -> bool {
        self.cards[player]
            .iter()
            .any(|c| matches!(c.effect, CardEffect::FreeBuild { has_built: false }))
    }

    pub fn has_play_final_card(&self, player: usize) -> bool {
        self.cards[player]
            .iter()
            .any(|c| matches!(c.effect, CardEffect::PlayFinalCard))
    }

    fn player_goods_options(&self, player: usize) -> Vec<Vec<Cost<Good>>> {
        let mut options = vec![];
        let city = &self.cities[player];
        let mut city_cost = HashMap::new();
        city_cost.insert(city.initial_resource, 1);
        options.push(vec![Cost(city_cost)]);
        for card in &self.cards[player] {
            if let CardEffect::Good { goods } = &card.effect {
                options.push(goods.clone());
            }
        }
        options
    }

    fn trade_cost_per_good(&self, player: usize, dir: i32, good: Good) -> i32 {
        for card in &self.cards[player] {
            if let CardEffect::Trade { directions, goods } = &card.effect
                && directions.contains(&dir)
                && goods.contains(&good)
            {
                return DISCOUNTED_TRADE_COST;
            }
        }
        BASE_TRADE_COST
    }

    fn bonus_count(&self, player: usize, target_kinds: &[BonusTarget], directions: &[i32]) -> i32 {
        let mut count = 0;
        for &dir in directions {
            let target_player = match dir {
                DIR_LEFT => (player + self.players - 1) % self.players,
                DIR_RIGHT => (player + 1) % self.players,
                DIR_DOWN => player,
                _ => continue,
            };
            for target in target_kinds {
                match target {
                    BonusTarget::Kind(kind) => {
                        count += self.cards[target_player]
                            .iter()
                            .filter(|c| c.kind == *kind)
                            .count() as i32;
                    }
                    BonusTarget::DefeatTokens => {
                        count += self.defeat_tokens[target_player];
                    }
                }
            }
        }
        count
    }

    pub fn attack_strength(&self, player: usize) -> i32 {
        let mut strength = 0;
        for card in &self.cards[player] {
            match &card.effect {
                CardEffect::Military { strength: s } => strength += s,
                CardEffect::Multi { resources } => {
                    strength += resources
                        .0
                        .get(&MultiResource::AttackStrength)
                        .copied()
                        .unwrap_or(0);
                }
                _ => {}
            }
        }
        strength
    }

    pub fn science_vp(&self, player: usize) -> i32 {
        let mut field_options: Vec<Vec<Field>> = vec![];
        for card in &self.cards[player] {
            if let CardEffect::Science { fields } = &card.effect {
                field_options.push(fields.clone());
            }
        }
        if field_options.is_empty() {
            return 0;
        }
        let mut best = 0;
        let mut counts: HashMap<Field, i32> = HashMap::new();
        Self::science_permute(&field_options, &mut counts, 0, &mut best);
        best
    }

    fn science_permute(
        options: &[Vec<Field>],
        counts: &mut HashMap<Field, i32>,
        idx: usize,
        best: &mut i32,
    ) {
        if idx == options.len() {
            let score = Self::score_science(counts);
            if score > *best {
                *best = score;
            }
            return;
        }
        for &field in &options[idx] {
            *counts.entry(field).or_insert(0) += 1;
            Self::science_permute(options, counts, idx + 1, best);
            *counts.get_mut(&field).unwrap() -= 1;
        }
    }

    fn score_science(counts: &HashMap<Field, i32>) -> i32 {
        let mut score = 0;
        let mut min_count = i32::MAX;
        for field in all_fields() {
            let count = counts.get(&field).copied().unwrap_or(0);
            score += count * count;
            if count < min_count {
                min_count = count;
            }
        }
        if min_count == i32::MAX {
            min_count = 0;
        }
        score + min_count * 7
    }

    pub fn player_vp(&self, player: usize) -> i32 {
        let mut vp = self.victory_tokens[player] - self.defeat_tokens[player];
        vp += self.coins[player] / 3;
        vp += self.science_vp(player);

        for card in &self.cards[player] {
            match &card.effect {
                CardEffect::VP { vp: card_vp } => vp += card_vp,
                CardEffect::Bonus {
                    target_kinds,
                    directions,
                    vp: bonus_vp,
                    ..
                } if *bonus_vp > 0 => {
                    vp += self.bonus_count(player, target_kinds, directions) * bonus_vp;
                }
                CardEffect::MimicGuild => {
                    vp += self.mimic_guild_vp(player);
                }
                _ => {}
            }
        }

        vp
    }

    fn mimic_guild_vp(&self, player: usize) -> i32 {
        let mut best = 0;
        for &dir in DIR_NEIGHBOURS {
            let neighbor = if dir == DIR_LEFT {
                (player + self.players - 1) % self.players
            } else {
                (player + 1) % self.players
            };
            for card in &self.cards[neighbor] {
                if card.kind != CardKind::Guild {
                    continue;
                }
                if let CardEffect::Bonus {
                    target_kinds,
                    directions,
                    vp: bonus_vp,
                    ..
                } = &card.effect
                {
                    let card_vp = self.bonus_count(player, target_kinds, directions) * bonus_vp;
                    if card_vp > best {
                        best = card_vp;
                    }
                }
            }
        }
        best
    }

    fn military_conflicts(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        let tokens = (self.round as i32) * 2 - 1;
        let n = self.players;

        let strengths: Vec<i32> = (0..n).map(|p| self.attack_strength(p)).collect();

        for p in 0..n {
            let right = (p + 1) % n;
            let my_str = strengths[p];
            let their_str = strengths[right];
            if my_str > their_str {
                self.victory_tokens[p] += tokens;
                self.defeat_tokens[right] += 1;
                logs.push(Log::public(vec![
                    N::Player(p),
                    N::text(format!(
                        " defeated player {} in military conflict (+{} victory, +1 defeat)",
                        right, tokens
                    )),
                ]));
            }
        }

        logs
    }

    fn choose_build(
        &mut self,
        player: usize,
        card_idx: usize,
        free: bool,
        wonder: bool,
    ) -> Result<Vec<Log>, GameError> {
        if card_idx >= self.hands[player].len() {
            return Err(GameError::invalid_input("card index out of range"));
        }

        if wonder {
            if !self.can_build_wonder(player) {
                return Err(GameError::invalid_input("cannot build wonder stage"));
            }
            let city = self.cities[player].clone();
            let stages_built = self.cards[player]
                .iter()
                .filter(|c| c.kind == CardKind::Wonder)
                .count();
            let db = card_db();
            let stage_name = &city.wonder_stages[stages_built];
            let stage_card = &db[stage_name];
            let (_, deals) = self.can_afford_cost(player, &stage_card.cost);

            let (deal, chosen) = if deals.len() <= 1 {
                (deals.first().map(|_| 0), true)
            } else {
                (None, false)
            };

            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free,
                wonder: true,
                deal,
                chosen,
            });
        } else if free {
            if !self.can_free_build(player, card_idx) {
                return Err(GameError::invalid_input("cannot free build this card"));
            }
            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free: true,
                wonder: false,
                deal: None,
                chosen: true,
            });
        } else {
            let (can, deals) = self.can_build_card(player, card_idx);
            if !can {
                return Err(GameError::invalid_input("cannot afford this card"));
            }

            let (deal, chosen) = if deals.len() <= 1 {
                (deals.first().map(|_| 0), true)
            } else {
                (None, false)
            };

            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free: false,
                wonder: false,
                deal,
                chosen,
            });
        }

        Ok(self.check_hand_complete())
    }

    fn choose_discard(&mut self, player: usize, card_idx: usize) -> Result<Vec<Log>, GameError> {
        if card_idx >= self.hands[player].len() {
            return Err(GameError::invalid_input("card index out of range"));
        }
        self.actions[player] = Some(Action::Discard { card: card_idx });
        Ok(self.check_hand_complete())
    }

    fn choose_deal(&mut self, player: usize, deal_idx: usize) -> Result<Vec<Log>, GameError> {
        let action = self.actions[player].clone();
        match action {
            Some(Action::Build {
                card,
                free,
                wonder,
                chosen: false,
                ..
            }) => {
                let cost = if wonder {
                    let city = self.cities[player].clone();
                    let stages_built = self.cards[player]
                        .iter()
                        .filter(|c| c.kind == CardKind::Wonder)
                        .count();
                    let db = card_db();
                    let stage_name = &city.wonder_stages[stages_built];
                    db[stage_name].cost.clone()
                } else {
                    self.hands[player][card].cost.clone()
                };

                let (_, deals) = self.can_afford_cost(player, &cost);
                if deal_idx >= deals.len() {
                    return Err(GameError::invalid_input("deal index out of range"));
                }

                self.actions[player] = Some(Action::Build {
                    card,
                    free,
                    wonder,
                    deal: Some(deal_idx),
                    chosen: true,
                });

                Ok(self.check_hand_complete())
            }
            _ => Err(GameError::invalid_input(
                "no pending deal selection for this player",
            )),
        }
    }

    fn take_from_discard(&mut self, player: usize, card_idx: usize) -> Result<Vec<Log>, GameError> {
        if self.to_resolve.is_empty() {
            return Err(GameError::invalid_input("nothing to resolve"));
        }
        let Resolver::DrawDiscard { player: rp } = &self.to_resolve[0];
        if *rp != player {
            return Err(GameError::invalid_input("not your resolver"));
        }
        if card_idx >= self.discard.len() {
            return Err(GameError::invalid_input("discard index out of range"));
        }

        let card = &self.discard[card_idx];
        if self.cards[player].iter().any(|c| c.name == card.name) {
            return Err(GameError::invalid_input("already own this card"));
        }
        let card = self.discard.remove(card_idx);
        self.cards[player].push(card.clone());

        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" took {} from the discard pile", card.name)),
        ])];

        self.to_resolve.remove(0);

        if self.to_resolve.is_empty() {
            let eh_logs = self.end_hand();
            logs.extend(eh_logs);
        }

        Ok(logs)
    }
}

impl Gamer for Game {
    type PubState = PubState;
    type PlayerState = PlayerState;

    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        Game::start_game(players, seed)
    }

    fn pub_state(&self) -> Self::PubState {
        let to_resolve_player = self.to_resolve.first().map(|r| match r {
            Resolver::DrawDiscard { player } => *player,
        });

        PubState {
            players: self.players,
            round: self.round,
            finished: self.finished,
            discard_count: self.discard.len(),
            cards: self.cards.clone(),
            coins: self.coins.clone(),
            victory_tokens: self.victory_tokens.clone(),
            defeat_tokens: self.defeat_tokens.clone(),
            cities: self.cities.clone(),
            hand_sizes: self.hands.iter().map(|h| h.len()).collect(),
            actions_chosen: self
                .actions
                .iter()
                .map(|a| match a {
                    Some(Action::Build { chosen, .. }) => *chosen,
                    Some(Action::Discard { .. }) => true,
                    None => false,
                })
                .collect(),
            to_resolve_player,
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
        self.assert_not_finished()?;
        self.assert_player_turn(player)?;

        let output = match self.command_parser(player) {
            Some(p) => p.parse(input, players),
            None => {
                return Err(GameError::invalid_input(
                    "not expecting any commands at the moment",
                ));
            }
        };
        match output {
            Ok(ParseOutput {
                remaining,
                value: Command::Build { card },
                ..
            }) => {
                let mut logs = self.choose_build(player, card, false, false)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Free { card },
                ..
            }) => {
                let mut logs = self.choose_build(player, card, true, false)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Wonder { card },
                ..
            }) => {
                let mut logs = self.choose_build(player, card, false, true)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
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
                let mut logs = self.choose_discard(player, card)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Deal { deal },
                ..
            }) => {
                let mut logs = self.choose_deal(player, deal)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Take { card },
                ..
            }) => {
                let mut logs = self.take_from_discard(player, card)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> =
                        (0..self.players).map(|p| (p, self.player_vp(p))).collect();
                    let placings = gen_placings(
                        &(0..self.players)
                            .map(|p| vec![self.player_vp(p), self.coins[p]])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
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
        if self.finished {
            let metrics: Vec<Vec<i32>> = (0..self.players)
                .map(|p| vec![self.player_vp(p), self.coins[p]])
                .collect();
            let placings = gen_placings(&metrics);
            Status::Finished {
                placings,
                stats: vec![],
            }
        } else if let Some(Resolver::DrawDiscard { player }) = self.to_resolve.first() {
            Status::Active {
                whose_turn: vec![*player],
                eliminated: vec![],
            }
        } else {
            let whose_turn: Vec<usize> = (0..self.players)
                .filter(|&p| {
                    if self.hands[p].is_empty() {
                        return false;
                    }
                    match &self.actions[p] {
                        None => true,
                        Some(Action::Build { chosen, .. }) => !chosen,
                        Some(Action::Discard { .. }) => false,
                    }
                })
                .collect();
            Status::Active {
                whose_turn,
                eliminated: vec![],
            }
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
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
    use brdgme_game::Gamer;

    const MICK: usize = 0;
    const STEVE: usize = 1;
    const GREG: usize = 2;

    fn players() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string(), "Greg".to_string()]
    }

    fn cmd(g: &mut Game, player: usize, input: &str) -> Result<CommandResponse, GameError> {
        let p = players();
        g.command(player, input, &p)
    }

    fn new_game() -> Game {
        let (g, _) = Game::start_game(3, 42).unwrap();
        g
    }

    fn rhodes_a() -> City {
        cities().into_iter().find(|c| c.name == "Rhodes A").unwrap()
    }

    fn db_card(name: &str) -> Card {
        card_db()[name].clone()
    }

    #[test]
    fn test_player_science_vp() {
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Babylon A Wonder Stage 2")];
        assert_eq!(g.science_vp(MICK), 1);
    }

    #[test]
    fn test_science_vp() {
        let counts: HashMap<Field, i32> = HashMap::new();
        assert_eq!(Game::score_science(&counts), 0);

        let mut counts: HashMap<Field, i32> = HashMap::new();
        *counts.entry(Field::Engineering).or_insert(0) += 1;
        *counts.entry(Field::Theology).or_insert(0) += 1;
        *counts.entry(Field::Mathematics).or_insert(0) += 1;
        assert_eq!(Game::score_science(&counts), 10);

        let mut counts: HashMap<Field, i32> = HashMap::new();
        *counts.entry(Field::Engineering).or_insert(0) += 2;
        *counts.entry(Field::Theology).or_insert(0) += 1;
        *counts.entry(Field::Mathematics).or_insert(0) += 1;
        assert_eq!(Game::score_science(&counts), 13);

        let mut counts: HashMap<Field, i32> = HashMap::new();
        *counts.entry(Field::Engineering).or_insert(0) += 4;
        assert_eq!(Game::score_science(&counts), 16);

        let mut counts: HashMap<Field, i32> = HashMap::new();
        *counts.entry(Field::Engineering).or_insert(0) += 2;
        *counts.entry(Field::Theology).or_insert(0) += 2;
        *counts.entry(Field::Mathematics).or_insert(0) += 2;
        assert_eq!(Game::score_science(&counts), 26);
    }

    #[test]
    fn test_can_build_card_free() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![vec![], vec![], vec![]];
        g.hands[MICK] = vec![db_card("Lumber Yard")];
        let (can, deals) = g.can_build_card(MICK, 0);
        assert!(can);
        assert!(deals.iter().all(|d| d.is_empty()));
    }

    #[test]
    fn test_can_build_card_prereq() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![vec![db_card("Training Ground")], vec![], vec![]];
        g.hands[MICK] = vec![db_card("Circus")];
        let (can, deals) = g.can_build_card(MICK, 0);
        assert!(can);
        assert!(deals.iter().all(|d| d.is_empty()));
    }

    #[test]
    fn test_can_build_card_owned() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![vec![db_card("Loom")], vec![], vec![]];
        g.hands[MICK] = vec![db_card("Loom")];
        let (can, _) = g.can_build_card(MICK, 0);
        assert!(!can);
    }

    #[test]
    fn test_can_build_card_self() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Tree Farm"), db_card("Clay Pit"), db_card("Loom")],
            vec![],
            vec![],
        ];
        g.hands[MICK] = vec![db_card("Haven")];
        let (can, deals) = g.can_build_card(MICK, 0);
        assert!(can);
        assert!(deals.iter().all(|d| d.is_empty()));
    }

    #[test]
    fn test_can_build_card_poor() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Tree Farm"), db_card("Clay Pit"), db_card("Loom")],
            vec![],
            vec![],
        ];
        g.hands[MICK] = vec![db_card("Arsenal")];
        let (can, _) = g.can_build_card(MICK, 0);
        assert!(!can);
    }

    #[test]
    fn test_can_build_card_trade() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Clay Pit"), db_card("Loom")],
            vec![db_card("Tree Farm")],
            vec![],
        ];
        g.hands[MICK] = vec![db_card("Haven")];
        let (can, deals) = g.can_build_card(MICK, 0);
        assert!(can);
        assert!(
            deals
                .iter()
                .any(|d| d.get(&DIR_RIGHT) == Some(&2) && d.len() == 1)
        );
    }

    #[test]
    fn test_can_build_card_trade_poor() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Clay Pit")],
            vec![db_card("Tree Farm")],
            vec![db_card("Loom")],
        ];
        g.hands[MICK] = vec![db_card("Haven")];
        let (can, _) = g.can_build_card(MICK, 0);
        assert!(!can);
    }

    #[test]
    fn test_can_build_card_trade_discount() {
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.cards = vec![
            vec![db_card("Clay Pit"), db_card("East Trading Post")],
            vec![db_card("Tree Farm")],
            vec![db_card("Loom")],
        ];
        g.hands[MICK] = vec![db_card("Haven")];
        let (can, deals) = g.can_build_card(MICK, 0);
        assert!(can);
        assert!(
            deals
                .iter()
                .any(|d| d.get(&DIR_LEFT) == Some(&2) && d.get(&DIR_RIGHT) == Some(&1))
        );
    }

    #[test]
    fn test_free_build() {
        let mut g = new_game();
        g.hands[MICK][0] = db_card("Palace");

        assert!(cmd(&mut g, MICK, "build 1").is_err());
        assert!(cmd(&mut g, MICK, "free 1").is_err());

        g.cards[MICK] = vec![db_card("Olympia A Wonder Stage 2")];
        assert!(cmd(&mut g, MICK, "free 1").is_ok());

        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert!(cmd(&mut g, MICK, "free 1").is_err());

        for _ in 0..5 {
            cmd(&mut g, MICK, "discard 1").unwrap();
            cmd(&mut g, STEVE, "discard 1").unwrap();
            cmd(&mut g, GREG, "discard 1").unwrap();
        }

        assert_eq!(g.round, 2);
        assert!(cmd(&mut g, MICK, "free 1").is_ok());
    }

    #[test]
    fn test_take_command() {
        let mut g = new_game();
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.whose_turn(), vec![MICK]);

        cmd(&mut g, MICK, "take 1").unwrap();

        assert_eq!(g.cards[MICK].len(), 4);
        assert_eq!(g.discard.len(), 2);
    }

    #[test]
    fn test_take_command_currently_discarded() {
        let mut g = new_game();
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert!(!g.to_resolve.is_empty());
        assert_eq!(g.whose_turn(), vec![MICK]);
    }

    #[test]
    fn test_take_command_empty() {
        let mut g = new_game();
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.discard = vec![];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert!(g.to_resolve.is_empty());
    }

    #[test]
    fn test_take_command_already_build() {
        let mut g = new_game();
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry"), db_card("Palace")];
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.whose_turn(), vec![MICK]);
        assert!(cmd(&mut g, MICK, "take 1").is_err());
    }

    #[test]
    fn test_card_commercial_tavern() {
        let mut g = new_game();
        g.hands[STEVE][0] = db_card("Tavern");
        let steve_coins = g.coins[STEVE];

        cmd(&mut g, MICK, "discard 1").unwrap();
        cmd(&mut g, STEVE, "build 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.coins[STEVE], steve_coins + TAVERN_COINS);
    }

    #[test]
    fn test_card_mimic_guild() {
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Olympia B Wonder Stage 3")];
        g.cards[STEVE] = vec![db_card("Builders Guild")];
        g.cards[GREG] = vec![db_card("Workers Guild")];

        assert_eq!(g.player_vp(MICK), 2);
    }

    #[test]
    fn test_card_play_final_card_with() {
        let mut g = new_game();
        for p in 0..3 {
            g.hands[p].truncate(2);
        }
        g.cards[STEVE] = vec![db_card("Babylon B Wonder Stage 2")];

        cmd(&mut g, MICK, "discard 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.round, 1);
        assert!(!g.hands[STEVE].is_empty());

        cmd(&mut g, STEVE, "discard 1").unwrap();

        assert_eq!(g.round, 2);
    }

    #[test]
    fn test_card_play_final_card_without() {
        let mut g = new_game();
        for p in 0..3 {
            g.hands[p].truncate(2);
        }

        cmd(&mut g, MICK, "discard 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.round, 2);
    }

    #[test]
    fn test_pub_state_does_not_leak_hidden_info() {
        let g = new_game();
        let ps = g.pub_state();
        let json = serde_json::to_string(&ps).unwrap();

        assert!(!json.contains("\"hand\""));

        for p in 1..3 {
            for card in &g.hands[p] {
                assert!(
                    !json.contains(&card.name),
                    "leaked card name: {}",
                    card.name
                );
            }
        }
    }
}
