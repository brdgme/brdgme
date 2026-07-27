pub mod card;
pub mod command;
pub mod render;
mod scoring;
mod trade;

pub use card::*;
pub use command::Command;

use std::collections::{BTreeMap, HashMap};

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
        /// Legacy: pre-upgrade saved states stored the chosen deal as an
        /// index into a deal list recomputed at execute time. Read only as a
        /// fallback when `deal_coins` is `None`; new states always write
        /// `None` here. Kept so old mid-hand states keep deserializing.
        deal: Option<usize>,
        /// The chosen trade payment (direction -> coins), captured at choose
        /// time so mid-turn state changes cannot reorder or shrink a
        /// recomputed deal list (b F9).
        #[serde(default)]
        deal_coins: Option<HashMap<i32, i32>>,
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

        let mut by_board: BTreeMap<String, Vec<City>> = BTreeMap::new();
        for c in cities() {
            let board = c
                .name
                .strip_suffix(" A")
                .or_else(|| c.name.strip_suffix(" B"))
                .unwrap_or(&c.name)
                .to_string();
            by_board.entry(board).or_default().push(c);
        }
        let mut boards: Vec<Vec<City>> = by_board.into_values().collect();
        boards.shuffle(&mut rng);
        let assigned_cities: Vec<City> = boards[..players]
            .iter()
            .map(|sides| sides[rng.random_range(0..sides.len())].clone())
            .collect();

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
            return logs;
        }

        self.pass_hands();
        vec![]
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
        logs.extend(self.prune_resolvers());

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
                        deal_coins,
                        ..
                    } => {
                        let (build_logs, built) = self.execute_build(
                            p,
                            *card,
                            *free,
                            *wonder,
                            *deal,
                            deal_coins.as_ref(),
                        );
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
        deal_coins: Option<&HashMap<i32, i32>>,
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
                let deal_map = self.resolve_deal(player, &stage_card.cost, deal, deal_coins);
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
                let deal_map = self.resolve_deal(player, &card.cost, deal, deal_coins);
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
            // Takeability can change between here and when the resolver fires
            // (same-hand discards grow the pile; an earlier resolver's take
            // shrinks it), so it is enforced by prune_resolvers() at hand
            // completion and after each take, not at queue time (b F2).
            CardEffect::DrawDiscard { .. } if !self.discard.is_empty() => {
                self.to_resolve.push(Resolver::DrawDiscard { player });
            }
            _ => {}
        }
        logs
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
                    N::text(" defeated "),
                    N::Player(right),
                    N::text(format!(
                        " in military conflict (+{} victory, +1 defeat)",
                        tokens
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

            let (deal_coins, chosen) = if deals.len() <= 1 {
                (deals.into_iter().next(), true)
            } else {
                (None, false)
            };

            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free,
                wonder: true,
                deal: None,
                deal_coins,
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
                deal_coins: None,
                chosen: true,
            });
        } else {
            let (can, deals) = self.can_build_card(player, card_idx);
            if !can {
                return Err(GameError::invalid_input("cannot afford this card"));
            }

            let (deal_coins, chosen) = if deals.len() <= 1 {
                (deals.into_iter().next(), true)
            } else {
                (None, false)
            };

            self.actions[player] = Some(Action::Build {
                card: card_idx,
                free: false,
                wonder: false,
                deal: None,
                deal_coins,
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
                    deal: None,
                    deal_coins: Some(deals[deal_idx].clone()),
                    chosen: true,
                });

                Ok(self.check_hand_complete())
            }
            _ => Err(GameError::invalid_input(
                "no pending deal selection for this player",
            )),
        }
    }

    fn has_takeable_discard(&self, player: usize) -> bool {
        self.discard
            .iter()
            .any(|c| !self.cards[player].iter().any(|o| o.name == c.name))
    }

    fn prune_resolvers(&mut self) -> Vec<Log> {
        let mut logs = vec![];
        while let Some(Resolver::DrawDiscard { player }) = self.to_resolve.first() {
            let player = *player;
            if self.has_takeable_discard(player) {
                break;
            }
            self.to_resolve.remove(0);
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" has no cards they can take from the discard pile"),
            ]));
        }
        logs
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
        logs.extend(self.prune_resolvers());

        if self.to_resolve.is_empty() {
            let eh_logs = self.end_hand();
            logs.extend(eh_logs);
        }

        Ok(logs)
    }

    fn finish_epilogue(&self, logs: &mut Vec<Log>) {
        let scores: Vec<(usize, i32)> = (0..self.players).map(|p| (p, self.player_vp(p))).collect();
        let placings = gen_placings(
            &(0..self.players)
                .map(|p| vec![self.player_vp(p), self.coins[p]])
                .collect::<Vec<Vec<i32>>>(),
        );
        logs.push(placings_log(&placings, Some(&scores)));
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
            hand: self.hands.get(player).cloned().unwrap_or_default(),
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
        let was_finished = self.is_finished();
        let (mut logs, can_undo, remaining) = match output {
            Ok(ParseOutput {
                remaining,
                value: Command::Build { card },
                ..
            }) => (
                self.choose_build(player, card, false, false)?,
                false,
                remaining,
            ),
            Ok(ParseOutput {
                remaining,
                value: Command::Free { card },
                ..
            }) => (
                self.choose_build(player, card, true, false)?,
                false,
                remaining,
            ),
            Ok(ParseOutput {
                remaining,
                value: Command::Wonder { card },
                ..
            }) => (
                self.choose_build(player, card, false, true)?,
                false,
                remaining,
            ),
            Ok(ParseOutput {
                remaining,
                value: Command::Discard { card },
                ..
            }) => (self.choose_discard(player, card)?, false, remaining),
            Ok(ParseOutput {
                remaining,
                value: Command::Deal { deal },
                ..
            }) => (self.choose_deal(player, deal)?, false, remaining),
            Ok(ParseOutput {
                remaining,
                value: Command::Take { card },
                ..
            }) => (self.take_from_discard(player, card)?, false, remaining),
            Err(e) => return Err(GameError::invalid_input(e.to_string())),
        };
        if !was_finished && self.is_finished() {
            self.finish_epilogue(&mut logs);
        }
        Ok(CommandResponse {
            logs,
            can_undo,
            remaining_input: remaining.to_string(),
        })
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

    fn giza_a() -> City {
        cities().into_iter().find(|c| c.name == "Giza A").unwrap()
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
    fn halicarnassus_b_stage_vp_is_scored() {
        // b F1: DrawDiscard stages carry printed VP (2/1/0 for the B side)
        // that player_vp dropped via the catch-all arm.
        let mut g = new_game();
        g.cards[MICK] = vec![
            db_card("Halicarnassus B Wonder Stage 1"),
            db_card("Halicarnassus B Wonder Stage 2"),
            db_card("Halicarnassus B Wonder Stage 3"),
        ];
        // 3 starting coins = 1 VP; stages = 2 + 1 + 0 = 3 VP.
        assert_eq!(g.player_vp(MICK), 4);
    }

    #[test]
    fn halicarnassus_a_stage_vp_unchanged() {
        // Lock in that the A side (vp: 0 on its DrawDiscard stage) is a no-op.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Halicarnassus A Wonder Stage 2")];
        assert_eq!(g.player_vp(MICK), 1); // coins only
    }

    #[test]
    fn auto_discarded_last_card_pays_no_coins() {
        // b F3: only the player-chosen discard action pays DISCARD_COINS;
        // the end-of-age auto-discard is free per the official rules.
        let mut g = new_game();
        for p in 0..3 {
            g.hands[p].truncate(2);
        }
        cmd(&mut g, MICK, "discard 1").unwrap();
        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap();

        assert_eq!(g.round, 2, "the age must have ended via the auto-discard");
        assert_eq!(g.discard.len(), 6, "3 chosen + 3 auto-discarded cards");
        // 3 starting coins + 3 for the chosen discard + 0 for the auto-discard.
        assert_eq!(g.coins, vec![6, 6, 6]);
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
    fn drawdiscard_pruned_when_pile_all_owned() {
        // b F2: with every pile card already owned by the resolver's player,
        // the resolver must be dropped instead of soft-locking the game.
        let mut g = new_game();
        // Giza's initial resource is Stone, so MICK's 3 Ore come only from his
        // own cards and the build resolves as a single, empty trade deal.
        for p in 0..3 {
            g.cities[p] = giza_a();
        }
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry"), db_card("Palace")];
        g.hands[STEVE][0] = db_card("Lumber Yard");
        g.hands[GREG][0] = db_card("Clay Pool");
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "build 1").unwrap();
        // STEVE and GREG BUILD (zero-cost cards) so the pile stays [Palace].
        cmd(&mut g, GREG, "build 1").unwrap();

        assert!(
            g.to_resolve.is_empty(),
            "a resolver with nothing takeable must be pruned"
        );
        assert_eq!(
            g.whose_turn(),
            vec![MICK, STEVE, GREG],
            "the hand must have ended and passed for everyone"
        );
    }

    #[test]
    fn second_resolver_pruned_when_pile_emptied() {
        // b F2 (multi-resolver): the first take empties the pile; the second
        // resolver must be pruned at that moment, not stranded.
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = giza_a();
        }
        g.hands[MICK][0] = db_card("Halicarnassus A Wonder Stage 2");
        g.cards[MICK] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.hands[STEVE][0] = db_card("Halicarnassus B Wonder Stage 1");
        g.cards[STEVE] = vec![db_card("Ore Vein"), db_card("Foundry")];
        g.hands[GREG][0] = db_card("Lumber Yard");
        g.discard = vec![db_card("Palace")];

        cmd(&mut g, MICK, "build 1").unwrap();
        cmd(&mut g, STEVE, "build 1").unwrap();
        cmd(&mut g, GREG, "build 1").unwrap();

        assert_eq!(g.to_resolve.len(), 2, "both DrawDiscard builds must queue");
        assert_eq!(g.whose_turn(), vec![MICK]);

        cmd(&mut g, MICK, "take 1").unwrap();

        assert!(g.cards[MICK].iter().any(|c| c.name == "Palace"));
        assert!(
            g.to_resolve.is_empty(),
            "the second resolver must be pruned once the pile is empty"
        );
    }

    #[test]
    fn legacy_action_json_still_deserializes() {
        // b F9: mid-hand saved states from before the deal_coins field carry
        // only the legacy index; they must keep deserializing.
        let old = r#"{"Build":{"card":2,"free":false,"wonder":false,"deal":0,"chosen":true}}"#;
        let a: Action = serde_json::from_str(old).unwrap();
        assert_eq!(
            a,
            Action::Build {
                card: 2,
                free: false,
                wonder: false,
                deal: Some(0),
                deal_coins: None,
                chosen: true,
            }
        );
    }

    #[test]
    fn stored_deal_paid_despite_mid_turn_divergence() {
        // b F9: the deal chosen at choose time must be the deal paid at
        // execute time, even when a recompute would find a different (or no)
        // deal list. Pre-fix, the recompute here finds NO deals and
        // unwrap_or_default() builds Haven for free.
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        // Haven costs Wood+Ore+Textile. MICK: Ore (city), Loom (textile),
        // Clay Pit; the Wood must come from STEVE's Tree Farm => one deal,
        // 2 coins to the right.
        g.cards = vec![
            vec![db_card("Clay Pit"), db_card("Loom")],
            vec![db_card("Tree Farm")],
            vec![],
        ];
        g.hands[MICK] = vec![db_card("Haven")];

        cmd(&mut g, MICK, "build 1").unwrap();
        let stored = match &g.actions[MICK] {
            Some(Action::Build {
                deal_coins: Some(m),
                chosen: true,
                ..
            }) => m.clone(),
            other => panic!("deal must be captured at choose time, got {:?}", other),
        };
        assert_eq!(stored.get(&DIR_RIGHT), Some(&2));

        // Sabotage: remove the traded-from neighbor's goods so a recompute
        // at execute time cannot reproduce the deal list.
        g.cards[STEVE].clear();

        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap(); // triggers execution

        assert!(g.cards[MICK].iter().any(|c| c.name == "Haven"));
        // 3 starting - 2 deal + 1 from Haven's own Raw-card Bonus (Clay Pit).
        // Pre-fix the sabotaged recompute built Haven for free, giving 4.
        assert_eq!(g.coins[MICK], 2);
        assert_eq!(
            g.coins[STEVE], 8,
            "3 starting + 2 trade payment + 3 discard coins"
        );
    }

    #[test]
    fn military_log_uses_player_node() {
        // b F12: the defeated player must be an N::Player node, not a raw
        // zero-based index in the text.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Stockade")];
        let logs = g.military_conflicts();
        assert_eq!(logs.len(), 1);
        let rendered = brdgme_markup::to_string(&logs[0].content);
        assert_eq!(
            rendered,
            "{{player 0}} defeated {{player 1}} in military conflict (+1 victory, +1 defeat)"
        );
    }

    #[test]
    fn out_of_range_player_is_guarded() {
        // b F10: a framework-passed bad index must degrade gracefully, not
        // panic (sibling-crate pattern: category-5-2, sushi-go-2).
        let g = new_game();
        assert!(g.player_state(99).hand.is_empty());
        assert!(g.command_parser(99).is_none());
        assert!(g.command_spec(99).is_none());
    }

    #[test]
    fn military_conflict_awards_tokens_per_age() {
        // b F14: victory tokens are 2*age - 1; each loss is one defeat token.
        let mut g = new_game();
        g.cards[MICK] = vec![db_card("Stockade")]; // strength 1 vs 0 vs 0
        g.round = 1;
        g.military_conflicts();
        // Only MICK (1) beats his right neighbor STEVE (0); STEVE vs GREG and
        // GREG vs MICK are not victories for the attacker.
        // NOTE: the live loop battles each player against their RIGHT
        // neighbor only (official rules battle both neighbors). No finding
        // covers that deviation; this test locks CURRENT behavior. If WP-16's
        // adjudication ever extends to it, update this test there.
        assert_eq!(g.victory_tokens, vec![1, 0, 0]);
        assert_eq!(g.defeat_tokens, vec![0, 1, 0]);

        g.round = 3;
        g.military_conflicts();
        assert_eq!(g.victory_tokens, vec![1 + 5, 0, 0]);
        assert_eq!(g.defeat_tokens, vec![0, 2, 0]);
    }

    #[test]
    fn hands_pass_toward_lower_index_in_odd_ages() {
        // b F14: odd ages take the hand from the next-higher index, even ages
        // from the next-lower (pass_hands, lib.rs).
        let mut g = new_game();
        let originals = g.hands.clone();
        g.round = 1;
        g.pass_hands();
        assert_eq!(g.hands[0], originals[1]);
        assert_eq!(g.hands[1], originals[2]);
        assert_eq!(g.hands[2], originals[0]);

        g.round = 2;
        g.pass_hands();
        assert_eq!(g.hands, originals, "one pass each way must round-trip");
    }

    #[test]
    fn haven_scores_own_raw_cards() {
        // Haven: 1 VP per own Raw card (DIR_SELF). Coins zeroed to isolate.
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[MICK] = vec![db_card("Haven"), db_card("Lumber Yard")];
        assert_eq!(g.player_vp(MICK), 1);
    }

    #[test]
    fn strategists_guild_scores_neighbor_defeats() {
        // Strategists Guild: 1 VP per neighbor defeat token (DIR_NEIGHBOURS).
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[STEVE] = vec![db_card("Strategists Guild")];
        g.defeat_tokens = vec![2, 0, 1];
        // Neighbors of STEVE hold 2 + 1 tokens; own tokens are 0.
        assert_eq!(g.player_vp(STEVE), 3);
    }

    #[test]
    fn builders_guild_scores_wonder_stages_all_directions() {
        // Builders Guild: 1 VP per wonder stage self + both neighbors.
        let mut g = new_game();
        g.coins = vec![0, 0, 0];
        g.cards[MICK] = vec![db_card("Rhodes A Wonder Stage 1")];
        g.cards[STEVE] = vec![db_card("Builders Guild")];
        g.cards[GREG] = vec![db_card("Rhodes A Wonder Stage 1")];
        // STEVE: 0 own + 1 (MICK) + 1 (GREG) = 2 VP; the guild card itself
        // is CardKind::Guild, not Wonder.
        assert_eq!(g.player_vp(STEVE), 2);
    }

    #[test]
    fn deal_command_selects_between_multiple_deals() {
        // b F14: Gardens costs Clay:2 + Wood:1. MICK's Clay Pool covers one
        // Clay, leaving {Clay:1, Wood:1} to trade. Each neighbor's Tree Farm
        // (Wood OR Clay) plus Clay Pool yields two distinct deals: buy both
        // from one side, or split. The player must be able to pick the second
        // deal via `deal 2` and pay exactly that deal.
        //
        // (The spec's original Haven setup produced a single deal: the
        // can_afford_perm early return at lib/cost collapses "same good from
        // either neighbor" before the second neighbor is explored. This setup
        // uses a multi-option source so two deals genuinely survive.)
        let mut g = new_game();
        for p in 0..3 {
            g.cities[p] = rhodes_a();
        }
        g.coins = vec![10, 3, 3];
        g.cards = vec![
            vec![db_card("Clay Pool")],
            vec![db_card("Tree Farm"), db_card("Clay Pool")],
            vec![db_card("Tree Farm"), db_card("Clay Pool")],
        ];
        g.hands[MICK] = vec![db_card("Gardens")];

        let (_, deals) = g.can_afford_cost(MICK, &db_card("Gardens").cost);
        assert_eq!(deals.len(), 2, "both-from-one-side vs split = two deals");
        assert_ne!(deals[0], deals[1]);

        cmd(&mut g, MICK, "build 1").unwrap();
        assert!(matches!(
            g.actions[MICK],
            Some(Action::Build { chosen: false, .. })
        ));
        // MICK still to act: the deal choice is pending.
        assert!(g.whose_turn().contains(&MICK));

        cmd(&mut g, MICK, "deal 2").unwrap();
        let expected = deals[1].clone();
        let coins_before = g.coins.clone();

        cmd(&mut g, STEVE, "discard 1").unwrap();
        cmd(&mut g, GREG, "discard 1").unwrap(); // triggers execution

        assert!(g.cards[MICK].iter().any(|c| c.name == "Gardens"));
        let paid: i32 = expected.values().sum();
        assert_eq!(g.coins[MICK], coins_before[MICK] - paid);
        for (&dir, &coins) in &expected {
            let neighbor = if dir == DIR_LEFT { GREG } else { STEVE };
            // +3 is that neighbor's own discard payment.
            assert_eq!(g.coins[neighbor], coins_before[neighbor] + 3 + coins);
        }
    }

    #[test]
    fn same_seed_same_game() {
        // b F14: starts are deterministic per seed. Compare typed values, not
        // JSON - Cost's HashMap serializes in instance-dependent order.
        let (a, _) = Game::start_game(5, 123).unwrap();
        let (b, _) = Game::start_game(5, 123).unwrap();
        assert_eq!(
            a.cities.iter().map(|c| &c.name).collect::<Vec<_>>(),
            b.cities.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        assert_eq!(a.hands, b.hands);
    }

    #[test]
    fn full_game_discard_replay_finishes() {
        // b F14: a deterministic all-discard game must run 3 ages to a
        // finished status with placings for everyone, never stalling.
        let p = players();
        let (mut g, _) = Game::start_game(3, 7).unwrap();
        let mut guard = 0;
        while !g.is_finished() {
            guard += 1;
            assert!(guard < 100, "game did not finish - state machine stalled");
            let turn = g.whose_turn();
            assert!(!turn.is_empty(), "active game with nobody to act");
            for pl in turn {
                g.command(pl, "discard 1", &p).unwrap();
            }
        }
        assert_eq!(g.round, 3);
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings.len(), 3),
            s => panic!("expected finished status, got {:?}", s),
        }
    }

    #[test]
    fn finish_epilogue_appended_once_on_transition() {
        let is_placings =
            |log: &Log| brdgme_markup::to_string(&log.content).contains("Final scores:");

        // Discard arm: an all-discard game finishes via the Discard command.
        let p = players();
        let (mut g, _) = Game::start_game(3, 7).unwrap();
        let mut guard = 0;
        let finishing = 'outer: loop {
            guard += 1;
            assert!(guard < 1000, "game did not finish - state machine stalled");
            for pl in g.whose_turn() {
                let before = g.is_finished();
                let resp = g.command(pl, "discard 1", &p).unwrap();
                if !before && g.is_finished() {
                    break 'outer resp;
                }
                assert!(
                    !resp.logs.iter().any(&is_placings),
                    "non-finishing command must not carry a placings log"
                );
            }
        };
        assert_eq!(
            finishing.logs.iter().filter(|l| is_placings(l)).count(),
            1,
            "exactly one placings log on finish"
        );
        assert!(
            is_placings(finishing.logs.last().unwrap()),
            "placings log last"
        );
        assert!(!finishing.can_undo, "finishing can_undo unchanged (false)");
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings.len(), 3),
            s => panic!("expected finished status, got {:?}", s),
        }

        // Build arm: a controlled age-3 hand finishes via the Build command.
        let mut g = new_game();
        g.round = 3;
        g.hands = vec![
            vec![db_card("Lumber Yard")],
            vec![db_card("Clay Pool")],
            vec![db_card("Ore Vein")],
        ];
        g.cards = vec![vec![], vec![], vec![]];
        g.actions = vec![None; 3];
        g.to_resolve = vec![];

        let r0 = cmd(&mut g, MICK, "build 1").unwrap();
        assert!(!r0.logs.iter().any(&is_placings));
        let r1 = cmd(&mut g, STEVE, "build 1").unwrap();
        assert!(!r1.logs.iter().any(&is_placings));
        let r2 = cmd(&mut g, GREG, "build 1").unwrap();
        assert_eq!(
            r2.logs.iter().filter(|l| is_placings(l)).count(),
            1,
            "exactly one placings log on build finish"
        );
        assert!(is_placings(r2.logs.last().unwrap()), "placings log last");
        assert!(!r2.can_undo, "finishing can_undo unchanged (false)");
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings.len(), 3),
            s => panic!("expected finished status, got {:?}", s),
        }
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

    #[test]
    fn test_start_game_no_duplicate_boards() {
        for players in 3..=4 {
            for seed in 0..200u64 {
                let (g, _) = Game::start_game(players, seed).unwrap();
                let mut board_names: Vec<String> = g
                    .cities
                    .iter()
                    .map(|c| {
                        c.name
                            .strip_suffix(" A")
                            .or_else(|| c.name.strip_suffix(" B"))
                            .unwrap_or(&c.name)
                            .to_string()
                    })
                    .collect();
                board_names.sort();
                board_names.dedup();
                assert_eq!(
                    players,
                    board_names.len(),
                    "seed {seed}: duplicate board dealt"
                );
            }
        }
    }
}
