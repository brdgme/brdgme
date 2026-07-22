pub mod card;
pub mod command;
pub mod render;

pub use card::*;
pub use command::Command;

use std::collections::BTreeMap;

use brdgme_color::NamedColor;
use brdgme_game::command::Spec as CommandSpec;
use brdgme_game::command::parser::Output as ParseOutput;
use brdgme_game::errors::GameError;
use brdgme_game::game::gen_placings;
use brdgme_game::rng::GameRng;
use brdgme_game::{CommandResponse, Gamer, Log, Status, placings_log};
use brdgme_markup::Node as N;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::command::PutWhere;
use crate::render::{PlayerState, PubState};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    ChooseModule,
    Produce,
    ChooseSector,
    Flight,
    TradeAndBuild,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Transaction(pub BTreeMap<Resource, i32>);

impl Transaction {
    pub fn inverse(&self) -> Transaction {
        let mut inv = BTreeMap::new();
        for (k, v) in &self.0 {
            inv.insert(*k, -v);
        }
        Transaction(inv)
    }

    pub fn is_empty(&self) -> bool {
        self.0.values().all(|v| *v == 0)
    }

    pub fn trim_empty(&mut self) {
        self.0.retain(|_, v| *v != 0);
    }

    pub fn resources(&self) -> Vec<Resource> {
        self.0
            .iter()
            .filter(|(_, v)| **v != 0)
            .map(|(r, _)| *r)
            .collect()
    }

    pub fn gain(&self) -> Transaction {
        let mut g = BTreeMap::new();
        for (r, v) in &self.0 {
            if *v > 0 {
                g.insert(*r, *v);
            }
        }
        Transaction(g)
    }

    pub fn lose(&self) -> Transaction {
        let mut l = BTreeMap::new();
        for (r, v) in &self.0 {
            if *v < 0 {
                l.insert(*r, *v);
            }
        }
        Transaction(l)
    }

    pub fn transaction_from_resources(resources: &[Resource]) -> Transaction {
        let mut t = BTreeMap::new();
        for r in resources {
            t.insert(*r, 1);
        }
        Transaction(t)
    }

    fn amount_plain(resource: Resource, amount: i32) -> String {
        if resource == Resource::Astro {
            format!("${}", amount)
        } else {
            format!("{} {}", amount, resource.name())
        }
    }

    pub fn gain_string(&self) -> Vec<N> {
        let parts: Vec<Vec<N>> = self
            .0
            .iter()
            .filter(|(_, v)| **v > 0)
            .map(|(r, v)| render_resource_amount(*r, *v))
            .collect();
        join_comma(&parts)
    }

    pub fn lose_string(&self) -> Vec<N> {
        let parts: Vec<Vec<N>> = self
            .0
            .iter()
            .filter(|(_, v)| **v < 0)
            .map(|(r, v)| render_resource_amount(*r, -*v))
            .collect();
        join_comma(&parts)
    }

    pub fn string(&self) -> Vec<N> {
        let g = self.gain_string();
        let l = self.lose_string();
        match (g.is_empty(), l.is_empty()) {
            (false, false) => {
                let mut n = vec![N::text("got ")];
                n.extend(g);
                n.push(N::text(" for "));
                n.extend(l);
                n
            }
            (false, true) => {
                let mut n = vec![N::text("got ")];
                n.extend(g);
                n
            }
            (true, false) => {
                let mut n = vec![N::text("paid ")];
                n.extend(l);
                n
            }
            (true, true) => vec![],
        }
    }

    fn gain_plain(&self) -> String {
        self.0
            .iter()
            .filter(|(_, v)| **v > 0)
            .map(|(r, v)| Transaction::amount_plain(*r, *v))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn lose_plain(&self) -> String {
        self.0
            .iter()
            .filter(|(_, v)| **v < 0)
            .map(|(r, v)| Transaction::amount_plain(*r, -*v))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn cannot_afford_error(&self) -> GameError {
        GameError::invalid_input(format!("can't afford {}", self.lose_plain()))
    }

    pub fn cannot_fit_error(&self) -> GameError {
        GameError::invalid_input(format!("not enough room for {}", self.gain_plain()))
    }
}

fn join_comma(parts: &[Vec<N>]) -> Vec<N> {
    let mut n = vec![];
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            n.push(N::text(", "));
        }
        n.extend(p.clone());
    }
    n
}

impl Resource {
    pub fn colony_ship_transaction() -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Ore, -1);
        t.insert(Resource::Fuel, -1);
        t.insert(Resource::Food, -1);
        t.insert(Resource::ColonyShip, 1);
        Transaction(t)
    }

    pub fn trade_ship_transaction() -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Ore, -1);
        t.insert(Resource::Fuel, -1);
        t.insert(Resource::Trade, -1);
        t.insert(Resource::TradeShip, 1);
        Transaction(t)
    }
}

impl Module {
    pub fn transaction(level: i32) -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Ore, -1);
        t.insert(Resource::Carbon, -1);
        t.insert(Resource::Food, -level);
        Transaction(t)
    }
}

fn take_transaction(resource: Resource) -> Transaction {
    let mut t = BTreeMap::new();
    t.insert(resource, 1);
    t.insert(Resource::Astro, -2);
    Transaction(t)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TradingPrices {
    pub buy: i32,
    pub sell: i32,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerBoard {
    pub player: usize,
    pub resources: BTreeMap<Resource, i32>,
    pub modules: BTreeMap<Module, i32>,
    pub completed_adventures: Vec<AdventureCard>,
    pub colonies: Vec<SectorCard>,
    pub trading_posts: Vec<SectorCard>,
    pub defeated_pirates: Vec<SectorCard>,
    pub friend_of_the_people: bool,
    pub hero_of_the_people: bool,
    pub last_sectors: Vec<i32>,
}

impl PlayerBoard {
    pub fn new(player: usize) -> Self {
        let mut resources = BTreeMap::new();
        resources.insert(Resource::Trade, 2);
        resources.insert(Resource::Science, 1);
        resources.insert(Resource::Astro, 25);
        resources.insert(Resource::ColonyShip, 1);
        resources.insert(Resource::TradeShip, 1);
        resources.insert(Resource::Booster, 2);
        resources.insert(Resource::Cannon, 1);
        PlayerBoard {
            player,
            resources,
            modules: BTreeMap::new(),
            completed_adventures: vec![],
            colonies: vec![starting_cards()[player].clone()],
            trading_posts: vec![],
            defeated_pirates: vec![],
            friend_of_the_people: false,
            hero_of_the_people: false,
            last_sectors: vec![],
        }
    }

    pub fn res(&self, r: Resource) -> i32 {
        self.resources.get(&r).copied().unwrap_or(0)
    }

    pub fn res_mut(&mut self, r: Resource) -> &mut i32 {
        self.resources.entry(r).or_insert(0)
    }

    pub fn module(&self, m: Module) -> i32 {
        self.modules.get(&m).copied().unwrap_or(0)
    }

    pub fn actions(&self) -> i32 {
        2 + self.module(Module::Command)
    }

    pub fn ships(&self) -> i32 {
        self.res(Resource::TradeShip) + self.res(Resource::ColonyShip)
    }

    pub fn can_build_ship(&self) -> bool {
        self.ships() < 2
    }

    pub fn can_build_booster(&self) -> bool {
        self.res(Resource::Booster) < 6
    }

    pub fn can_build_cannon(&self) -> bool {
        self.res(Resource::Cannon) < 6
    }

    pub fn can_build(&self) -> bool {
        self.can_build_ship() || self.can_build_booster() || self.can_build_cannon()
    }

    pub fn booster_transaction(&self) -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Fuel, -2);
        t.insert(Resource::Booster, 1);
        if self.res(Resource::Booster) >= 3 {
            t.insert(Resource::Science, -1);
        }
        Transaction(t)
    }

    pub fn cannon_transaction(&self) -> Transaction {
        let mut t = BTreeMap::new();
        t.insert(Resource::Carbon, -2);
        t.insert(Resource::Cannon, 1);
        if self.res(Resource::Booster) >= 3 {
            t.insert(Resource::Science, -1);
        }
        Transaction(t)
    }

    pub fn goods_limit(&self) -> i32 {
        2 + self.module(Module::Logistics)
    }

    pub fn cannot_fit_buy_error(&self, resource: Resource, amount: i32) -> String {
        let spare = (self.goods_limit() - self.res(resource)).max(0);
        format!(
            "not enough room for {} {} - you have room for {} more {}",
            amount,
            resource.name(),
            spare,
            resource.name()
        )
    }

    pub fn can_afford(&self, t: &Transaction) -> bool {
        self.can_fit(&t.lose())
    }

    pub fn can_fit(&self, t: &Transaction) -> bool {
        let mut trimmed = t.clone();
        trimmed.trim_empty();
        trimmed == self.fit_transaction(t)
    }

    pub fn fit_transaction(&self, t: &Transaction) -> Transaction {
        let mut fit: BTreeMap<Resource, i32> = t.0.clone();
        let mut ship_total = self.ships()
            + fit.get(&Resource::ColonyShip).copied().unwrap_or(0)
            + fit.get(&Resource::TradeShip).copied().unwrap_or(0);
        while ship_total > 2 {
            if fit.get(&Resource::ColonyShip).copied().unwrap_or(0) > 0 {
                *fit.entry(Resource::ColonyShip).or_insert(0) -= 1;
            } else {
                *fit.entry(Resource::TradeShip).or_insert(0) -= 1;
            }
            ship_total -= 1;
        }
        let goods_limit = self.goods_limit();
        let keys: Vec<Resource> = fit.keys().copied().collect();
        for r in keys {
            let v = fit.get(&r).copied().unwrap_or(0);
            let cur = self.res(r);
            let new_v = match r {
                Resource::Science => {
                    if cur + v > 4 {
                        4 - cur
                    } else {
                        v
                    }
                }
                Resource::Food
                | Resource::Fuel
                | Resource::Carbon
                | Resource::Ore
                | Resource::Trade => {
                    if cur + v > goods_limit {
                        goods_limit - cur
                    } else {
                        v
                    }
                }
                Resource::Booster | Resource::Cannon => {
                    if cur + v > 6 {
                        6 - cur
                    } else {
                        v
                    }
                }
                _ => v,
            };
            fit.insert(r, new_v);
            if self.res(r) + fit.get(&r).copied().unwrap_or(0) < 0 {
                fit.insert(r, -self.res(r));
            }
        }
        let mut out = Transaction(fit);
        out.trim_empty();
        out
    }

    pub fn transact(&mut self, t: &Transaction) -> Transaction {
        let fitted = self.fit_transaction(t);
        for (r, v) in &fitted.0 {
            *self.res_mut(*r) += v;
        }
        fitted
    }

    pub fn trading_post_prices(&self) -> BTreeMap<Resource, TradingPrices> {
        let mut prices: BTreeMap<Resource, TradingPrices> = BTreeMap::new();
        for c in &self.trading_posts {
            if let SectorCard::Trade {
                resources,
                price,
                direction,
                ..
            } = c
            {
                for r in resources {
                    let entry = prices
                        .entry(*r)
                        .or_insert(TradingPrices { buy: 0, sell: 0 });
                    if *direction != TradeDir::Sell && (entry.buy == 0 || *price < entry.buy) {
                        entry.buy = *price;
                    }
                    if *direction != TradeDir::Buy && (entry.sell == 0 || *price > entry.sell) {
                        entry.sell = *price;
                    }
                }
            }
        }
        prices
    }

    pub fn victory_points(&self) -> i32 {
        let mut vp = 0;
        if self.hero_of_the_people {
            vp += 1;
        }
        if self.friend_of_the_people {
            vp += 1;
        }
        for l in self.modules.values() {
            if *l == 2 {
                vp += 1;
            }
        }
        for c in self
            .colonies
            .iter()
            .chain(self.trading_posts.iter())
            .chain(self.defeated_pirates.iter())
        {
            vp += c.victory_points();
        }
        for c in &self.completed_adventures {
            vp += c.victory_points();
        }
        vp
    }

    pub fn medals(&self) -> i32 {
        let mut medals = 0;
        for c in self
            .colonies
            .iter()
            .chain(self.trading_posts.iter())
            .chain(self.defeated_pirates.iter())
        {
            medals += c.medals();
        }
        for c in &self.completed_adventures {
            medals += c.medals();
        }
        medals
    }

    pub fn diplomat_points(&self) -> i32 {
        let mut dp = 0;
        for c in self
            .colonies
            .iter()
            .chain(self.trading_posts.iter())
            .chain(self.defeated_pirates.iter())
        {
            dp += c.diplomat_points();
        }
        dp
    }

    pub fn module_list(&self) -> Vec<Module> {
        self.modules
            .iter()
            .filter(|(_, l)| **l > 0)
            .map(|(m, _)| *m)
            .collect()
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: usize,
    pub player_boards: [PlayerBoard; 2],
    pub sector_cards: BTreeMap<i32, Vec<SectorCard>>,
    pub sector_draw_pile: Vec<SectorCard>,
    pub peeking: Vec<SectorCard>,
    pub flight_cards: Vec<SectorCard>,
    pub flight_actions: BTreeMap<usize, bool>,
    pub current_sector: i32,
    pub trade_amount: i32,
    pub player_trade_amount: i32,
    pub adventure_cards: Vec<AdventureCard>,
    pub remove_adventure_card: usize,
    pub phase: Phase,
    pub current_player: usize,
    pub gain_player: usize,
    pub gain_resources: Option<Vec<Resource>>,
    pub gain_queue: Vec<Vec<Resource>>,
    pub yellow_dice: i32,
    pub card_finished: bool,
    pub losing_module: bool,
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
}

fn pop_n(deck: &mut Vec<SectorCard>, n: usize) -> Vec<SectorCard> {
    let n = n.min(deck.len());
    let start = deck.len() - n;
    deck.split_off(start)
}

impl Game {
    pub fn start_game(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError> {
        if players != 2 {
            return Err(GameError::PlayerCount {
                min: 2,
                max: 2,
                given: players,
            });
        }
        let mut g = Game {
            players,
            player_boards: [PlayerBoard::new(0), PlayerBoard::new(1)],
            sector_cards: BTreeMap::new(),
            sector_draw_pile: vec![],
            peeking: vec![],
            flight_cards: vec![],
            flight_actions: BTreeMap::new(),
            current_sector: 0,
            trade_amount: 0,
            player_trade_amount: 0,
            adventure_cards: vec![],
            remove_adventure_card: 0,
            phase: Phase::ChooseModule,
            current_player: 0,
            gain_player: 0,
            gain_resources: None,
            gain_queue: vec![],
            yellow_dice: 0,
            card_finished: false,
            losing_module: false,
            rng: GameRng::seed_from_u64(seed),
        };
        let mut deck = shuffled_sector_cards(&mut g.rng);
        for s in 1..=4 {
            let top = pop_n(&mut deck, 10);
            g.sector_cards.insert(s, top);
        }
        g.sector_draw_pile = deck;
        g.adventure_cards = shuffled_adventure_cards(&mut g.rng);
        Ok((g, vec![]))
    }

    pub fn whose_turn(&self) -> Vec<usize> {
        if self.gain_resources.is_some() {
            return vec![self.gain_player];
        }
        match self.phase {
            Phase::ChooseModule => (0..self.players)
                .filter(|p| self.player_boards[*p].modules.is_empty())
                .collect(),
            _ => vec![self.current_player],
        }
    }

    pub fn is_finished(&self) -> bool {
        self.player_boards.iter().any(|b| b.victory_points() >= 10)
    }

    pub fn flight_distance(&self) -> i32 {
        self.yellow_dice + self.player_boards[self.current_player].res(Resource::Booster)
    }

    pub fn remaining_moves(&self) -> i32 {
        self.flight_distance() - self.flight_cards.len() as i32
    }

    pub fn remaining_actions(&self) -> i32 {
        let used = self.flight_actions.values().filter(|a| **a).count() as i32;
        self.player_boards[self.current_player].actions() - used
    }

    pub fn remaining_trades(&self) -> i32 {
        2 - self.trade_amount
    }

    pub fn remaining_player_trades(&self) -> i32 {
        self.player_boards[self.current_player].module(Module::Trade) - self.player_trade_amount
    }

    pub fn tradable_resources(&self) -> Vec<Resource> {
        match self.phase {
            Phase::Flight if !self.flight_cards.is_empty() => {
                if let Some(SectorCard::Trade { resources, .. }) = self.flight_cards.last() {
                    resources.clone()
                } else {
                    vec![]
                }
            }
            Phase::TradeAndBuild => self.player_boards[self.current_player]
                .trading_post_prices()
                .keys()
                .copied()
                .collect(),
            _ => vec![],
        }
    }

    fn log_transaction(&self, player: usize, t: &Transaction) -> Log {
        let mut content = vec![N::Player(player), N::text(" ")];
        content.extend(t.string());
        Log::public(content)
    }

    pub fn gain_resource(&mut self, player: usize, resource: Resource) -> Vec<Log> {
        let mut t = Transaction::default();
        t.0.insert(resource, 1);
        let gained = self.player_boards[player].transact(&t);
        if gained.is_empty() {
            vec![]
        } else {
            vec![self.log_transaction(player, &gained)]
        }
    }

    pub fn gain_one(&mut self, player: usize, resources: &[Resource]) -> Vec<Log> {
        if self.gain_resources.is_some() {
            self.gain_queue.push(resources.to_vec());
            return vec![];
        }
        if resources.is_empty() {
            return self.gained(player);
        }
        let can_produce = self.player_boards[player]
            .fit_transaction(&Transaction::transaction_from_resources(resources))
            .resources();
        match can_produce.len() {
            0 => {
                let mut logs = vec![Log::public(vec![
                    N::Player(player),
                    N::text(" did not gain a resource, all full"),
                ])];
                logs.extend(self.gained(player));
                logs
            }
            1 => {
                let r = can_produce[0];
                let mut logs = self.gain_resource(player, r);
                logs.extend(self.gained(player));
                logs
            }
            _ => {
                self.gain_player = player;
                self.gain_resources = Some(can_produce);
                vec![]
            }
        }
    }

    pub fn gained(&mut self, player: usize) -> Vec<Log> {
        self.gain_resources = None;
        if !self.gain_queue.is_empty() {
            let resources = self.gain_queue.remove(0);
            return self.gain_one(player, &resources);
        }
        match self.phase {
            Phase::Produce => {
                if player == self.current_player {
                    let opp = (self.current_player + 1) % 2;
                    self.produce(opp)
                } else {
                    self.phase = Phase::ChooseSector;
                    vec![]
                }
            }
            Phase::Flight => {
                if matches!(
                    self.flight_cards.last(),
                    Some(SectorCard::AdventurePlanet { .. })
                ) {
                    self.completed()
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    pub fn produce(&mut self, player: usize) -> Vec<Log> {
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" is producing resources"),
        ])];
        let trade_level = self.player_boards[player].module(Module::Trade);
        if Module::trade_module_dice(trade_level, player).contains(&self.yellow_dice) {
            logs.extend(self.gain_resource(player, Resource::Trade));
        }
        let science_level = self.player_boards[player].module(Module::Science);
        if Module::science_module_dice(science_level, player).contains(&self.yellow_dice) {
            logs.extend(self.gain_resource(player, Resource::Science));
        }
        let mut producing: Vec<Resource> = vec![];
        for c in &self.player_boards[player].colonies {
            if let SectorCard::Colony { resource, dice, .. } = c
                && *dice == self.yellow_dice
                && !producing.contains(resource)
            {
                producing.push(*resource);
            }
        }
        if producing.is_empty() {
            logs.push(Log::public(vec![
                N::Player(player),
                N::text(" doesn't produce anything with their colonies"),
            ]));
            logs.extend(self.gained(player));
        } else {
            logs.extend(self.gain_one(player, &producing));
        }
        logs
    }

    pub fn new_turn(&mut self) -> Vec<Log> {
        self.phase = Phase::Produce;
        self.yellow_dice = self.rng.random_range(1..=3i32);
        let logs = vec![Log::public(vec![
            N::Player(self.current_player),
            N::text(" rolled a "),
            N::Bold(vec![N::text(self.yellow_dice.to_string())]),
            N::text(", flight distance will be "),
            N::Bold(vec![N::text(self.flight_distance().to_string())]),
        ])];
        let mut out = logs;
        out.extend(self.produce(self.current_player));
        out
    }

    pub fn next_turn(&mut self) -> Vec<Log> {
        self.current_player = (self.current_player + 1) % 2;
        self.new_turn()
    }

    pub fn next_sector_card(&mut self) -> Vec<Log> {
        if self.phase != Phase::Flight {
            return vec![];
        }
        let pile_empty = self
            .sector_cards
            .get(&self.current_sector)
            .map(|v| v.is_empty())
            .unwrap_or(true);
        if pile_empty || self.remaining_moves() <= 0 || self.remaining_actions() <= 0 {
            return self.end_flight();
        }
        let next_card = self
            .sector_cards
            .get_mut(&self.current_sector)
            .unwrap()
            .pop()
            .unwrap();
        self.flight_cards.push(next_card.clone());
        self.trade_amount = 0;
        self.card_finished = false;
        let mut content = vec![N::Player(self.current_player), N::text(" arrived at ")];
        content.extend(next_card.string());
        vec![Log::public(content)]
    }

    pub fn end_flight(&mut self) -> Vec<Log> {
        let logs = vec![Log::public(vec![N::text("The flight has ended")])];
        let mut pile = self
            .sector_cards
            .get(&self.current_sector)
            .cloned()
            .unwrap_or_default();
        let fc = std::mem::take(&mut self.flight_cards);
        pile.extend(fc);
        pile.shuffle(&mut self.rng);
        self.sector_cards.insert(self.current_sector, pile);
        self.player_boards[self.current_player]
            .last_sectors
            .insert(0, self.current_sector);
        self.trade_amount = 0;
        self.player_trade_amount = 0;
        self.phase = Phase::TradeAndBuild;
        logs
    }

    pub fn replace_card(&mut self) -> Vec<Log> {
        if !self.sector_draw_pile.is_empty() {
            let c = self.sector_draw_pile.pop().unwrap();
            self.flight_cards.push(c.clone());
            self.card_finished = true;
            let mut content = vec![N::text("The replacement card is ")];
            content.extend(c.full_string());
            vec![Log::public(content)]
        } else {
            let mut logs = vec![Log::public(vec![N::text("No replacement cards remain")])];
            logs.extend(self.end_flight());
            logs
        }
    }

    pub fn mark_card_actioned(&mut self) {
        self.flight_actions.insert(self.flight_cards.len(), true);
    }

    pub fn completed(&mut self) -> Vec<Log> {
        if self.remove_adventure_card > 0 {
            let current_len = current_adventure_cards(&self.adventure_cards).len();
            let ac_index =
                self.adventure_cards.len() - (current_len - self.remove_adventure_card) - 1;
            let ac = self.adventure_cards.remove(ac_index);
            self.player_boards[self.current_player]
                .completed_adventures
                .push(ac);
            self.remove_adventure_card = 0;
            self.recalculate_people_cards();
        }
        self.card_finished = true;
        vec![]
    }

    pub fn recalculate_people_cards(&mut self) {
        let p1d = self.player_boards[0].diplomat_points();
        let p2d = self.player_boards[1].diplomat_points();
        let d = if p1d > 3 && p1d > p2d {
            0
        } else if p2d > 3 && p2d > p1d {
            1
        } else {
            -1
        };
        let p1m = self.player_boards[0].medals();
        let p2m = self.player_boards[1].medals();
        let m = if p1m > 3 && p1m > p2m {
            0
        } else if p2m > 3 && p2m > p1m {
            1
        } else {
            -1
        };
        for p in 0..2 {
            self.player_boards[p].friend_of_the_people = d == p as i32;
            self.player_boards[p].hero_of_the_people = m == p as i32;
        }
    }

    pub fn can_trade(&self, player: usize, resource: Resource, amount: i32) -> (bool, i32, String) {
        if self.current_player != player {
            return (false, 0, "it's not your turn".to_string());
        }
        let trade_dir = amount_trade_dir(amount);
        let tradable = self.tradable_resources();
        match self.phase {
            Phase::Flight => {
                if self.flight_cards.is_empty() {
                    return (false, 0, "there are no flight cards".to_string());
                }
                let (card_resources, price, maximum, direction) = match self.flight_cards.last() {
                    Some(SectorCard::Trade {
                        resources,
                        price,
                        maximum,
                        direction,
                        ..
                    }) => (resources.clone(), *price, *maximum, *direction),
                    _ => {
                        return (
                            false,
                            0,
                            "the current flight card is not a trade card".to_string(),
                        );
                    }
                };
                if resource != Resource::Any && !tradable.contains(&resource) {
                    let names = tradable
                        .iter()
                        .map(|r| r.name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return (
                        false,
                        0,
                        format!(
                            "you can only {} {} with this trade card",
                            trade_dir.string(),
                            names
                        ),
                    );
                }
                if trade_dir != TradeDir::Both
                    && direction != TradeDir::Both
                    && trade_dir != direction
                {
                    return (
                        false,
                        0,
                        format!("you can only {} with this trade card", trade_dir.string()),
                    );
                }
                let target_amount = amount * trade_dir.sign() + self.trade_amount;
                if amount != 0 && maximum != 0 && target_amount > maximum {
                    return (
                        false,
                        0,
                        format!(
                            "you can only trade up to {} {} with this trade card, you have already traded {}",
                            maximum,
                            card_resources
                                .iter()
                                .map(|r| r.name())
                                .collect::<Vec<_>>()
                                .join(", "),
                            self.trade_amount
                        ),
                    );
                }
                if trade_dir == TradeDir::Buy {
                    if amount * price > self.player_boards[player].res(Resource::Astro) {
                        return (
                            false,
                            0,
                            format!(
                                "you only have ${}",
                                self.player_boards[player].res(Resource::Astro)
                            ),
                        );
                    }
                    if resource != Resource::Any {
                        let mut t = Transaction::default();
                        t.0.insert(resource, amount);
                        if !self.player_boards[player].can_fit(&t) {
                            return (
                                false,
                                0,
                                self.player_boards[player].cannot_fit_buy_error(resource, amount),
                            );
                        }
                    }
                }
                if trade_dir == TradeDir::Sell
                    && resource != Resource::Any
                    && amount * trade_dir.sign() > self.player_boards[player].res(resource)
                {
                    return (
                        false,
                        0,
                        format!(
                            "you only have {} {}",
                            self.player_boards[player].res(resource),
                            resource.name()
                        ),
                    );
                }
                (true, price, String::new())
            }
            Phase::TradeAndBuild => {
                if self.remaining_trades() == 0 {
                    return (
                        false,
                        0,
                        "you have already done two trades this phase".to_string(),
                    );
                }
                if resource == Resource::Any && tradable.is_empty() {
                    return (false, 0, "you don't have any trading posts".to_string());
                }
                if resource != Resource::Any {
                    if !tradable.contains(&resource) {
                        return (
                            false,
                            0,
                            "you don't have any trading posts for that resource".to_string(),
                        );
                    }
                    let prices = self.player_boards[player].trading_post_prices();
                    if trade_dir == TradeDir::Buy {
                        let mut t = Transaction::default();
                        t.0.insert(resource, amount);
                        if !self.player_boards[player].can_fit(&t) {
                            return (
                                false,
                                0,
                                self.player_boards[player].cannot_fit_buy_error(resource, amount),
                            );
                        }
                        if let Some(p) = prices.get(&resource)
                            && p.buy > 0
                        {
                            return (true, p.buy, String::new());
                        }
                        return (false, 0, "you aren't able to buy that resource".to_string());
                    }
                    if trade_dir == TradeDir::Sell {
                        if amount * trade_dir.sign() > self.player_boards[player].res(resource) {
                            return (
                                false,
                                0,
                                format!(
                                    "you only have {} {}",
                                    self.player_boards[player].res(resource),
                                    resource.name()
                                ),
                            );
                        }
                        if let Some(p) = prices.get(&resource)
                            && p.sell > 0
                        {
                            return (true, p.sell, String::new());
                        }
                        return (
                            false,
                            0,
                            "you aren't able to sell that resource".to_string(),
                        );
                    }
                }
                (true, 0, String::new())
            }
            _ => (false, 0, "it is not the correct phase to trade".to_string()),
        }
    }

    pub fn trade(
        &mut self,
        player: usize,
        resource: Resource,
        amount: i32,
    ) -> Result<Vec<Log>, GameError> {
        let trade_dir = amount_trade_dir(amount);
        let (ok, price, reason) = self.can_trade(player, resource, amount);
        if !ok {
            return Err(GameError::invalid_input(reason));
        }
        if resource == Resource::Any {
            return Err(GameError::invalid_input(
                "you must specify which resource to trade",
            ));
        }
        if trade_dir == TradeDir::Both {
            return Err(GameError::invalid_input(
                "you must either buy or sell when trading",
            ));
        }
        let total = amount * price;
        *self.player_boards[player].res_mut(Resource::Astro) -= total;
        *self.player_boards[player].res_mut(resource) += amount;
        match self.phase {
            Phase::Flight => {
                self.mark_card_actioned();
                self.trade_amount += amount * trade_dir.sign();
            }
            Phase::TradeAndBuild => {
                self.trade_amount += 1;
            }
            _ => {}
        }
        let past = match trade_dir {
            TradeDir::Buy => "bought",
            TradeDir::Sell => "sold",
            TradeDir::Both => "traded",
        };
        let mut content = vec![
            N::Player(player),
            N::text(format!(" {} {} ", past, amount * trade_dir.sign())),
        ];
        content.extend(render_resource(resource));
        content.push(N::text(" for "));
        content.extend(render_money(total * trade_dir.sign()));
        Ok(vec![Log::public(content)])
    }

    pub fn handle_trade(
        &mut self,
        player: usize,
        amount: i32,
        resource: Option<Resource>,
        dir: TradeDir,
    ) -> Result<Vec<Log>, GameError> {
        if amount <= 0 {
            return Err(GameError::invalid_input(
                "the amount must be a positive whole number",
            ));
        }
        let tradable = self.tradable_resources();
        let resource = match resource {
            Some(r) => r,
            None => {
                if tradable.len() == 1 {
                    tradable[0]
                } else {
                    return Err(GameError::invalid_input("you must specify a resource"));
                }
            }
        };
        self.trade(player, resource, amount * dir.sign())
    }

    // Guards

    pub fn can_choose(&self, player: usize) -> bool {
        self.phase == Phase::ChooseModule && self.player_boards[player].modules.is_empty()
    }

    pub fn can_gain(&self, player: usize) -> bool {
        self.gain_player == player && self.gain_resources.is_some()
    }

    pub fn can_put(&self, player: usize) -> bool {
        self.current_player == player && !self.peeking.is_empty()
    }

    pub fn can_sector(&self, player: usize) -> bool {
        self.phase == Phase::ChooseSector && self.current_player == player
    }

    pub fn can_build(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::TradeAndBuild
            && self.player_boards[player].can_build()
    }

    pub fn can_upgrade_module(&self, player: usize, module: Module) -> bool {
        let opponent = (player + 1) % 2;
        self.player_boards[player].module(module) == 0
            || (self.player_boards[player].module(module) == 1
                && self.player_boards[opponent].module(module) < 2)
    }

    pub fn available_module_upgrades(&self, player: usize) -> Vec<Module> {
        Module::ALL
            .iter()
            .copied()
            .filter(|m| self.can_upgrade_module(player, *m))
            .collect()
    }

    pub fn can_upgrade(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::TradeAndBuild
            && !self.available_module_upgrades(player).is_empty()
    }

    pub fn can_buy(&self, player: usize) -> bool {
        self.can_trade(player, Resource::Any, 1).0
    }

    pub fn can_sell(&self, player: usize) -> bool {
        self.can_trade(player, Resource::Any, -1).0
    }

    pub fn can_take_resource(&self, player: usize, resource: Resource) -> bool {
        if !(self.current_player == player
            && self.phase == Phase::TradeAndBuild
            && self.remaining_player_trades() > 0)
        {
            return false;
        }
        if resource == Resource::Any {
            return true;
        }
        let opponent = (player + 1) % 2;
        let t = take_transaction(resource);
        self.player_boards[player].can_fit(&t) && self.player_boards[opponent].can_fit(&t.inverse())
    }

    pub fn can_take(&self, player: usize) -> bool {
        self.can_take_resource(player, Resource::Any)
    }

    pub fn can_done(&self, player: usize) -> bool {
        self.current_player == player && self.phase == Phase::TradeAndBuild
    }

    fn flight_terminal(&self) -> bool {
        let pile_empty = self
            .sector_cards
            .get(&self.current_sector)
            .map(|v| v.is_empty())
            .unwrap_or(true);
        pile_empty || self.remaining_moves() <= 0 || self.remaining_actions() <= 0
    }

    pub fn can_next(&self, player: usize) -> bool {
        self.can_end(player) && !self.flight_terminal()
    }

    pub fn can_end(&self, player: usize) -> bool {
        if self.current_player != player
            || self.phase != Phase::Flight
            || self.flight_cards.is_empty()
            || self.gain_resources.is_some()
        {
            return false;
        }
        if self.card_finished {
            return true;
        }
        !self.flight_cards.last().unwrap().requires_action()
    }

    pub fn can_found_colony(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Flight
            && !self.flight_cards.is_empty()
            && self.player_boards[player].res(Resource::ColonyShip) > 0
            && matches!(self.flight_cards.last(), Some(SectorCard::Colony { .. }))
    }

    pub fn can_found_trading_post(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Flight
            && !self.flight_cards.is_empty()
            && self.trade_amount == 0
            && self.player_boards[player].res(Resource::TradeShip) > 0
            && self
                .flight_cards
                .last()
                .map(|c| c.can_found_trading_post())
                .unwrap_or(false)
    }

    pub fn can_fight(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Flight
            && !self.flight_cards.is_empty()
            && !self.losing_module
            && matches!(self.flight_cards.last(), Some(SectorCard::Pirate { .. }))
    }

    pub fn can_pay_ransom(&self, player: usize) -> bool {
        if !(self.current_player == player
            && self.phase == Phase::Flight
            && !self.flight_cards.is_empty()
            && !self.losing_module)
        {
            return false;
        }
        match self.flight_cards.last() {
            Some(SectorCard::Pirate { ransom, .. }) => {
                *ransom <= self.player_boards[player].res(Resource::Astro)
            }
            _ => false,
        }
    }

    pub fn can_lose_module(&self, player: usize) -> bool {
        self.current_player == player || self.losing_module
    }

    pub fn can_complete(&self, player: usize) -> bool {
        self.current_player == player
            && self.phase == Phase::Flight
            && matches!(
                self.flight_cards.last(),
                Some(SectorCard::AdventurePlanet { .. })
            )
            && !current_adventure_cards(&self.adventure_cards).is_empty()
    }

    // Actions

    pub fn choose(&mut self, player: usize, module: Module) -> Result<Vec<Log>, GameError> {
        if !self.can_choose(player) {
            return Err(GameError::invalid_input(
                "you can't choose a module at the moment",
            ));
        }
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" chose the "),
            N::Bold(vec![N::text(format!("{} module", module.name()))]),
        ])];
        self.player_boards[player].modules.insert(module, 1);
        if self.whose_turn().is_empty() {
            logs.extend(self.new_turn());
        }
        Ok(logs)
    }

    pub fn gain(&mut self, player: usize, resource: Resource) -> Result<Vec<Log>, GameError> {
        if !self.can_gain(player) {
            return Err(GameError::invalid_input(
                "you can't gain a resource at the moment",
            ));
        }
        let valid = self
            .gain_resources
            .as_ref()
            .map(|gr| gr.contains(&resource))
            .unwrap_or(false);
        if !valid {
            return Err(GameError::invalid_input(format!(
                "You aren't able to gain {} at the moment",
                resource.name()
            )));
        }
        let mut logs = self.gain_resource(player, resource);
        logs.extend(self.gained(player));
        Ok(logs)
    }

    pub fn put(&mut self, player: usize, num: usize, on: PutWhere) -> Result<Vec<Log>, GameError> {
        if !self.can_put(player) {
            return Err(GameError::invalid_input(
                "you can't put cards at the moment",
            ));
        }
        if num < 1 || num > self.peeking.len() {
            return Err(GameError::invalid_input(
                "you must specify the number of one of the listed cards",
            ));
        }
        let c = self.peeking.remove(num - 1);
        let where_str = match on {
            PutWhere::Top => {
                self.sector_cards
                    .entry(self.current_sector)
                    .or_default()
                    .push(c);
                "top"
            }
            PutWhere::Bottom => {
                self.sector_cards
                    .entry(self.current_sector)
                    .or_default()
                    .insert(0, c);
                "bottom"
            }
        };
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(format!(" put a card on the {} of the pile", where_str)),
        ])];
        if self.peeking.is_empty() {
            logs.extend(self.next_sector_card());
        }
        Ok(logs)
    }

    pub fn sector(&mut self, player: usize, sector: i32) -> Result<Vec<Log>, GameError> {
        if !self.can_sector(player) {
            return Err(GameError::invalid_input(
                "you can't choose a sector at the moment",
            ));
        }
        if !(1..=4).contains(&sector) {
            return Err(GameError::invalid_input("sector must be between 1 and 4"));
        }
        self.phase = Phase::Flight;
        self.current_sector = sector;
        self.flight_actions.clear();
        let sensor = self.player_boards[self.current_player].module(Module::Sensor);
        match sensor {
            1 => {
                let pile = self.sector_cards.entry(sector).or_default();
                self.peeking = pop_n(pile, 2);
                Ok(vec![Log::public(vec![
                    N::Player(player),
                    N::text(" is using the sensor module to peek at 2 cards"),
                ])])
            }
            2 => {
                let pile = self.sector_cards.entry(sector).or_default();
                self.peeking = pop_n(pile, 3);
                Ok(vec![Log::public(vec![
                    N::Player(player),
                    N::text(" is using the sensor module to peek at 3 cards"),
                ])])
            }
            _ => Ok(self.next_sector_card()),
        }
    }

    pub fn build(&mut self, player: usize, resource: Resource) -> Result<Vec<Log>, GameError> {
        if !self.can_build(player) {
            return Err(GameError::invalid_input("you cannot build at the moment"));
        }
        let t = match resource {
            Resource::TradeShip => Resource::trade_ship_transaction(),
            Resource::ColonyShip => Resource::colony_ship_transaction(),
            Resource::Cannon => self.player_boards[player].cannon_transaction(),
            Resource::Booster => self.player_boards[player].booster_transaction(),
            _ => return Err(GameError::invalid_input("invalid resource")),
        };
        if !self.player_boards[player].can_afford(&t) {
            return Err(t.cannot_afford_error());
        }
        if !self.player_boards[player].can_fit(&t) {
            return Err(t.cannot_fit_error());
        }
        self.player_boards[player].transact(&t);
        Ok(vec![self.log_transaction(player, &t)])
    }

    pub fn upgrade(&mut self, player: usize, module: Module) -> Result<Vec<Log>, GameError> {
        if !self.can_upgrade(player) {
            return Err(GameError::invalid_input(
                "you can't upgrade modules at the moment",
            ));
        }
        if !self.can_upgrade_module(player, module) {
            return Err(GameError::invalid_input(format!(
                "you can't upgrade {}",
                module.name()
            )));
        }
        let new_level = self.player_boards[player].module(module) + 1;
        let t = Module::transaction(new_level);
        if !self.player_boards[player].can_afford(&t) {
            return Err(t.cannot_afford_error());
        }
        self.player_boards[player].transact(&t);
        self.player_boards[player].modules.insert(module, new_level);
        let mut logs = vec![self.log_transaction(player, &t)];
        logs.push(Log::public(vec![
            N::Player(player),
            N::text(" upgraded their "),
            N::Bold(vec![N::text(format!("{} module", module.name()))]),
            N::text(" to "),
            N::Bold(vec![N::text(format!("level {}", new_level))]),
        ]));
        Ok(logs)
    }

    pub fn buy(
        &mut self,
        player: usize,
        amount: i32,
        resource: Option<Resource>,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_buy(player) {
            return Err(GameError::invalid_input("you can't buy at the moment"));
        }
        self.handle_trade(player, amount, resource, TradeDir::Buy)
    }

    pub fn sell(
        &mut self,
        player: usize,
        amount: i32,
        resource: Option<Resource>,
    ) -> Result<Vec<Log>, GameError> {
        if !self.can_sell(player) {
            return Err(GameError::invalid_input("you can't sell at the moment"));
        }
        self.handle_trade(player, amount, resource, TradeDir::Sell)
    }

    pub fn take(&mut self, player: usize, resource: Resource) -> Result<Vec<Log>, GameError> {
        if !self.can_take_resource(player, resource) {
            return Err(GameError::invalid_input(
                "can't take that resource at the moment",
            ));
        }
        if !Resource::GOODS.contains(&resource) {
            return Err(GameError::invalid_input("you can only take goods"));
        }
        let opponent = (player + 1) % 2;
        let t = take_transaction(resource);
        self.player_boards[player].transact(&t);
        let mut logs = vec![self.log_transaction(player, &t)];
        let inv = t.inverse();
        self.player_boards[opponent].transact(&inv);
        logs.push(self.log_transaction(opponent, &inv));
        self.player_trade_amount += 1;
        Ok(logs)
    }

    pub fn done(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_done(player) {
            return Err(GameError::invalid_input(
                "cannot finish your turn at the moment",
            ));
        }
        self.current_player = (self.current_player + 1) % 2;
        Ok(self.new_turn())
    }

    pub fn next(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_next(player) {
            return Err(GameError::invalid_input(
                "you can't advance to the next card",
            ));
        }
        Ok(self.next_sector_card())
    }

    pub fn end(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_end(player) {
            return Err(GameError::invalid_input(
                "cannot end the flight at the moment",
            ));
        }
        Ok(self.end_flight())
    }

    pub fn found(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if self.can_found_colony(player) {
            return self.found_colony(player);
        }
        if self.can_found_trading_post(player) {
            return self.found_trading_post(player);
        }
        Err(GameError::invalid_input("you are not able to found here"))
    }

    fn found_colony(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_found_colony(player) {
            return Err(GameError::invalid_input(
                "you are not able to found a colony",
            ));
        }
        let c = self.flight_cards.pop().unwrap();
        let mut content = vec![N::Player(player), N::text(" founded a colony on ")];
        content.extend(c.string());
        let mut logs = vec![Log::public(content)];
        self.player_boards[player].colonies.push(c);
        *self.player_boards[player].res_mut(Resource::ColonyShip) -= 1;
        logs.extend(self.replace_card());
        self.mark_card_actioned();
        self.recalculate_people_cards();
        Ok(logs)
    }

    fn found_trading_post(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_found_trading_post(player) {
            return Err(GameError::invalid_input(
                "you are not able to found a trading post",
            ));
        }
        let c = self.flight_cards.pop().unwrap();
        let mut content = vec![N::Player(player), N::text(" founded a trading post on ")];
        content.extend(c.string());
        let mut logs = vec![Log::public(content)];
        self.player_boards[player].trading_posts.push(c);
        *self.player_boards[player].res_mut(Resource::TradeShip) -= 1;
        logs.extend(self.replace_card());
        self.mark_card_actioned();
        self.recalculate_people_cards();
        Ok(logs)
    }

    pub fn fight(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_fight(player) {
            return Err(GameError::invalid_input("you are unable to fight"));
        }
        let (strength, destroy_cannon, destroy_module) = match self.flight_cards.last() {
            Some(SectorCard::Pirate {
                strength,
                destroy_cannon,
                destroy_module,
                ..
            }) => (*strength, *destroy_cannon, *destroy_module),
            _ => return Err(GameError::invalid_input("card isn't a pirate card")),
        };
        let pirate_roll = self.rng.random_range(1..=3i32);
        let pirate_attack = pirate_roll + strength;
        let player_roll = self.rng.random_range(1..=3i32);
        let player_cannon = self.player_boards[player].res(Resource::Cannon);
        let player_attack = player_roll + player_cannon;
        let player_won = player_attack >= pirate_attack;

        use brdgme_markup::Align;
        let rows = vec![
            vec![
                (Align::Left, vec![N::text("")]),
                (Align::Left, vec![N::Bold(vec![N::text("Str.")])]),
                (Align::Left, vec![N::Bold(vec![N::text("Roll")])]),
                (Align::Left, vec![N::Bold(vec![N::text("Attack")])]),
            ],
            vec![
                (Align::Left, vec![N::Player(player)]),
                (Align::Left, vec![N::text(player_cannon.to_string())]),
                (Align::Left, vec![N::text(player_roll.to_string())]),
                (
                    Align::Left,
                    vec![N::Bold(vec![N::text(player_attack.to_string())])],
                ),
            ],
            vec![
                (
                    Align::Left,
                    vec![N::Fg(
                        NamedColor::Grey.into(),
                        vec![N::Bold(vec![N::text("pirate")])],
                    )],
                ),
                (Align::Left, vec![N::text(strength.to_string())]),
                (Align::Left, vec![N::text(pirate_roll.to_string())]),
                (
                    Align::Left,
                    vec![N::Bold(vec![N::text(pirate_attack.to_string())])],
                ),
            ],
        ];
        let table = brdgme_markup::table_with_gap(&rows, 2);
        let result: Vec<N> = if player_won {
            vec![N::Player(player), N::text(" has defeated the pirate")]
        } else {
            vec![N::text("The pirate has defeated "), N::Player(player)]
        };
        let mut fight_content = vec![N::Player(player), N::text(" is fighting the pirate")];
        fight_content.push(table);
        fight_content.extend(result);
        let mut logs = vec![Log::public(fight_content)];

        if player_won {
            let c = self.flight_cards.pop().unwrap();
            self.player_boards[player].defeated_pirates.push(c);
            self.recalculate_people_cards();
            logs.extend(self.replace_card());
        } else {
            if destroy_cannon && self.player_boards[player].res(Resource::Cannon) > 0 {
                *self.player_boards[player].res_mut(Resource::Cannon) -= 1;
                logs.push(Log::public(vec![
                    N::Player(player),
                    N::text(" had a cannon destroyed by the pirate"),
                ]));
            }
            if destroy_module && !self.player_boards[player].module_list().is_empty() {
                self.losing_module = true;
            } else {
                logs.extend(self.end_flight());
            }
        }
        Ok(logs)
    }

    pub fn pay(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_pay_ransom(player) {
            return Err(GameError::invalid_input(
                "you aren't able to pay the ransom",
            ));
        }
        let ransom = match self.flight_cards.last() {
            Some(SectorCard::Pirate { ransom, .. }) => *ransom,
            _ => return Err(GameError::invalid_input("card isn't a pirate card")),
        };
        *self.player_boards[player].res_mut(Resource::Astro) -= ransom;
        let mut content = vec![N::Player(player), N::text(" paid a ransom of ")];
        content.extend(render_money(ransom));
        self.card_finished = true;
        Ok(vec![Log::public(content)])
    }

    pub fn lose(&mut self, player: usize, module: Module) -> Result<Vec<Log>, GameError> {
        if !self.can_lose_module(player) {
            return Err(GameError::invalid_input(
                "you can't lose a module at the moment",
            ));
        }
        if self.player_boards[player].module(module) <= 0 {
            return Err(GameError::invalid_input("you don't have that module"));
        }
        let new_level = self.player_boards[player].module(module) - 1;
        if new_level == 0 {
            self.player_boards[player].modules.remove(&module);
        } else {
            self.player_boards[player].modules.insert(module, new_level);
        }
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" had their "),
            N::Bold(vec![N::text(format!("{} module", module.name()))]),
            N::text(" destroyed by the pirate"),
        ])];
        self.losing_module = false;
        logs.extend(self.end_flight());
        Ok(logs)
    }

    pub fn complete(&mut self, player: usize, adventure: usize) -> Result<Vec<Log>, GameError> {
        if !self.can_complete(player) {
            return Err(GameError::invalid_input(
                "you can't complete an adventure at the moment",
            ));
        }
        if adventure == 0 {
            return Err(GameError::invalid_input(
                "the adventure number must be above 0",
            ));
        }
        let current = current_adventure_cards(&self.adventure_cards);
        if adventure > current.len() {
            return Err(GameError::invalid_input(format!(
                "the adventure number can't be higher than {}",
                current.len()
            )));
        }
        let planet = match self.flight_cards.last() {
            Some(SectorCard::AdventurePlanet { planet }) => *planet,
            _ => {
                return Err(GameError::invalid_input(
                    "you can't complete an adventure at the moment",
                ));
            }
        };
        let ac = current[adventure - 1];
        if ac.planet() != planet {
            return Err(GameError::invalid_input(
                "it is not the correct planet to complete that card",
            ));
        }
        let mut logs = ac.complete(player, self)?;
        self.mark_card_actioned();
        let mut content = vec![N::Player(player), N::text(" completed a mission on ")];
        content.extend(adventure_planet_string(planet));
        content.push(N::text(" - "));
        content.push(N::Fg(NamedColor::Grey.into(), vec![N::text(ac.text())]));
        logs.push(Log::public(content));
        self.remove_adventure_card = adventure;
        if self.gain_resources.is_none() {
            logs.extend(self.completed());
        }
        Ok(logs)
    }
}

impl AdventureCard {
    pub fn complete(self, player: usize, game: &mut Game) -> Result<Vec<Log>, GameError> {
        fn donate(game: &mut Game, player: usize, t: &Transaction) -> Result<Vec<Log>, GameError> {
            if !game.player_boards[player].can_afford(t) {
                return Err(t.cannot_afford_error());
            }
            game.player_boards[player].transact(t);
            Ok(vec![game.log_transaction(player, t)])
        }
        match self {
            AdventureCard::EnvironmentalCrisis => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Science, -1);
                t.0.insert(Resource::Astro, 3);
                let mut logs = donate(game, player, &t)?;
                logs.extend(game.gain_one(player, &Resource::GOODS));
                Ok(logs)
            }
            AdventureCard::DiplomaticGift | AdventureCard::MerchantGift => {
                Ok(game.gain_one(player, &Resource::GOODS))
            }
            AdventureCard::Famine => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Food, -1);
                let mut logs = donate(game, player, &t)?;
                logs.extend(game.gain_one(player, &Resource::GOODS));
                Ok(logs)
            }
            AdventureCard::WholesaleOrder1 => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Trade, -1);
                let mut logs = donate(game, player, &t)?;
                logs.extend(game.gain_one(player, &Resource::GOODS));
                Ok(logs)
            }
            AdventureCard::PirateNest => {
                if game.player_boards[player].res(Resource::Booster) < 4 {
                    return Err(GameError::invalid_input("you don't have enough boosters"));
                }
                Ok(game.gain_one(player, &Resource::GOODS))
            }
            AdventureCard::CouncilMeeting => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Astro, -6);
                let mut logs = donate(game, player, &t)?;
                logs.extend(game.gain_one(player, &Resource::GOODS));
                logs.extend(game.gain_one(player, &Resource::GOODS));
                Ok(logs)
            }
            AdventureCard::Epidemic => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Science, -2);
                donate(game, player, &t)
            }
            AdventureCard::Emergency => {
                if game.player_boards[player].res(Resource::Booster) < 4 {
                    return Err(GameError::invalid_input("you don't have enough boosters"));
                }
                Ok(game.gain_one(player, &Resource::GOODS))
            }
            AdventureCard::Reconstruction => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Astro, -10);
                donate(game, player, &t)
            }
            AdventureCard::Monument => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Ore, -2);
                t.0.insert(Resource::Carbon, -1);
                donate(game, player, &t)
            }
            AdventureCard::WholesaleOrder2 => {
                let mut t = Transaction::default();
                t.0.insert(Resource::Trade, -2);
                let mut logs = donate(game, player, &t)?;
                logs.extend(game.gain_one(player, &Resource::GOODS));
                logs.extend(game.gain_one(player, &Resource::GOODS));
                Ok(logs)
            }
        }
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
            phase: self.phase,
            current_player: self.current_player,
            current_sector: self.current_sector,
            player_boards: self.player_boards.clone(),
            flight_cards: self.flight_cards.clone(),
            trade_amount: self.trade_amount,
            player_trade_amount: self.player_trade_amount,
            yellow_dice: self.yellow_dice,
            flight_actions_used: self.flight_actions.values().filter(|a| **a).count(),
            card_finished: self.card_finished,
            losing_module: self.losing_module,
            current_adventure_cards: current_adventure_cards(&self.adventure_cards),
            adventure_deck_len: self.adventure_cards.len(),
            sector_pile_lens: self
                .sector_cards
                .iter()
                .map(|(k, v)| (*k, v.len()))
                .collect(),
            sector_draw_pile_len: self.sector_draw_pile.len(),
        }
    }

    fn player_state(&self, player: usize) -> Self::PlayerState {
        PlayerState {
            public: self.pub_state(),
            player,
            peeking: self.peeking.clone(),
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
            None => return Err(GameError::invalid_input("it is not your turn")),
        }
        .parse(input, players);
        match output {
            Ok(ParseOutput {
                remaining,
                value: Command::Choose { module },
                ..
            }) => Ok(CommandResponse {
                logs: self.choose(player, module)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Gain { resource },
                ..
            }) => {
                let mut logs = self.gain(player, resource)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].victory_points()))
                        .collect();
                    let placings = gen_placings(
                        &(0..2)
                            .map(|p| vec![self.player_boards[p].victory_points()])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: true,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Put { num, on },
                ..
            }) => Ok(CommandResponse {
                logs: self.put(player, num, on)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Sector { sector },
                ..
            }) => Ok(CommandResponse {
                logs: self.sector(player, sector)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Build { resource },
                ..
            }) => Ok(CommandResponse {
                logs: self.build(player, resource)?,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Upgrade { module },
                ..
            }) => {
                let mut logs = self.upgrade(player, module)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].victory_points()))
                        .collect();
                    let placings = gen_placings(
                        &(0..2)
                            .map(|p| vec![self.player_boards[p].victory_points()])
                            .collect::<Vec<Vec<i32>>>(),
                    );
                    logs.push(placings_log(&placings, Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: true,
                    remaining_input: remaining.to_string(),
                })
            }
            Ok(ParseOutput {
                remaining,
                value: Command::Buy { amount, resource },
                ..
            }) => Ok(CommandResponse {
                logs: self.buy(player, amount, resource)?,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Sell { amount, resource },
                ..
            }) => Ok(CommandResponse {
                logs: self.sell(player, amount, resource)?,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Take { resource },
                ..
            }) => Ok(CommandResponse {
                logs: self.take(player, resource)?,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Done,
                ..
            }) => Ok(CommandResponse {
                logs: self.done(player)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Next,
                ..
            }) => Ok(CommandResponse {
                logs: self.next(player)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::End,
                ..
            }) => Ok(CommandResponse {
                logs: self.end(player)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Found,
                ..
            }) => {
                let mut logs = self.found(player)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].victory_points()))
                        .collect();
                    let placings = gen_placings(
                        &(0..2)
                            .map(|p| vec![self.player_boards[p].victory_points()])
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
                value: Command::Fight,
                ..
            }) => {
                let mut logs = self.fight(player)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].victory_points()))
                        .collect();
                    let placings = gen_placings(
                        &(0..2)
                            .map(|p| vec![self.player_boards[p].victory_points()])
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
                value: Command::Pay,
                ..
            }) => Ok(CommandResponse {
                logs: self.pay(player)?,
                can_undo: true,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Lose { module },
                ..
            }) => Ok(CommandResponse {
                logs: self.lose(player, module)?,
                can_undo: false,
                remaining_input: remaining.to_string(),
            }),
            Ok(ParseOutput {
                remaining,
                value: Command::Complete { adventure },
                ..
            }) => {
                let mut logs = self.complete(player, adventure)?;
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_boards[p].victory_points()))
                        .collect();
                    let placings = gen_placings(
                        &(0..2)
                            .map(|p| vec![self.player_boards[p].victory_points()])
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
        if self.is_finished() {
            let metrics: Vec<Vec<i32>> = (0..2)
                .map(|p| vec![self.player_boards[p].victory_points()])
                .collect();
            Status::Finished {
                placings: gen_placings(&metrics),
                stats: vec![],
            }
        } else {
            Status::Active {
                whose_turn: self.whose_turn(),
                eliminated: vec![],
            }
        }
    }

    fn command_spec(&self, player: usize) -> Option<CommandSpec> {
        self.command_parser(player).map(|cp| cp.to_spec())
    }

    fn points(&self) -> Vec<f32> {
        (0..2)
            .map(|p| self.player_boards[p].victory_points() as f32)
            .collect()
    }

    fn player_count(&self) -> usize {
        self.players
    }

    fn player_counts() -> Vec<usize> {
        vec![2]
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

    fn players() -> Vec<String> {
        vec!["Mick".to_string(), "Steve".to_string()]
    }

    fn colony_card() -> SectorCard {
        SectorCard::Colony {
            name: "Test Colony".to_string(),
            resource: Resource::Carbon,
            dice: 1,
            start_card: false,
        }
    }

    #[test]
    fn start() {
        assert!(Game::start(2, 1).is_ok());
    }

    #[test]
    fn choose_module() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert_eq!(g.whose_turn(), vec![0, 1]);

        g.command(1, "choose lo", &players).unwrap();
        let mut expected = BTreeMap::new();
        expected.insert(Module::Logistics, 1);
        assert_eq!(g.player_boards[1].modules, expected);
        assert_eq!(g.whose_turn(), vec![0]);

        g.command(0, "choose se", &players).unwrap();
        let mut expected = BTreeMap::new();
        expected.insert(Module::Sensor, 1);
        assert_eq!(g.player_boards[0].modules, expected);
        assert_ne!(g.phase, Phase::ChooseModule);
    }

    #[test]
    fn sector_base_cards() {
        assert_eq!(card::sector_base_cards().len(), 40);
    }

    #[test]
    fn sector1_cards() {
        assert_eq!(card::sector1_cards().len(), 7);
    }

    #[test]
    fn sector2_cards() {
        assert_eq!(card::sector2_cards().len(), 7);
    }

    #[test]
    fn sector3_cards() {
        assert_eq!(card::sector3_cards().len(), 7);
    }

    #[test]
    fn sector4_cards() {
        assert_eq!(card::sector4_cards().len(), 7);
    }

    #[test]
    fn shuffled_sector_cards() {
        let mut rng = GameRng::seed_from_u64(1);
        assert_eq!(card::shuffled_sector_cards(&mut rng).len(), 68);
    }

    #[test]
    fn adventure1_cards() {
        assert_eq!(card::adventure1_cards().len(), 3);
    }

    #[test]
    fn adventure2_cards() {
        assert_eq!(card::adventure2_cards().len(), 3);
    }

    #[test]
    fn adventure3_cards() {
        assert_eq!(card::adventure3_cards().len(), 3);
    }

    #[test]
    fn adventure4_cards() {
        assert_eq!(card::adventure4_cards().len(), 3);
    }

    #[test]
    fn shuffled_adventure_cards() {
        let mut rng = GameRng::seed_from_u64(1);
        assert_eq!(card::shuffled_adventure_cards(&mut rng).len(), 12);
    }

    #[test]
    fn parse_module() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        assert!(g.command(0, "choose FARTS!", &players).is_err());
        assert!(g.command(0, "choose s", &players).is_err());
        g.command(0, "choose se", &players).unwrap();
        assert_eq!(g.player_boards[0].modules.get(&Module::Sensor), Some(&1));
    }

    #[test]
    fn winner_when_player_ahead() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].colonies = vec![colony_card(); 10];
        assert_eq!(g.player_boards[0].victory_points(), 10);
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings, vec![1, 2]),
            _ => panic!("expected a finished game"),
        }
    }

    #[test]
    fn winner_when_player_b_ahead() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[1].colonies = vec![colony_card(); 10];
        assert_eq!(g.player_boards[1].victory_points(), 10);
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings, vec![2, 1]),
            _ => panic!("expected a finished game"),
        }
    }

    #[test]
    fn tie_game() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.player_boards[0].colonies = vec![colony_card(); 10];
        g.player_boards[1].colonies = vec![colony_card(); 10];
        match g.status() {
            Status::Finished { placings, .. } => assert_eq!(placings, vec![1, 1]),
            _ => panic!("expected a finished game"),
        }
    }

    fn collect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    keys.push(k.clone());
                    collect_keys(v, keys);
                }
            }
            serde_json::Value::Array(items) => {
                for v in items {
                    collect_keys(v, keys);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn pub_state_does_not_leak_hidden_info() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.peeking = vec![colony_card()];
        let json = serde_json::to_value(g.pub_state()).unwrap();

        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("adventure_deck_len"));
        assert!(obj.contains_key("sector_pile_lens"));
        assert!(obj.contains_key("sector_draw_pile_len"));
        assert!(obj["adventure_deck_len"].is_number());
        assert!(obj["sector_draw_pile_len"].is_number());
        assert!(obj["sector_pile_lens"].is_object());
        for (_, v) in obj["sector_pile_lens"].as_object().unwrap() {
            assert!(v.is_number());
        }

        let mut keys = vec![];
        collect_keys(&json, &mut keys);
        for forbidden in [
            "peeking",
            "sector_draw_pile",
            "sector_cards",
            "adventure_cards",
        ] {
            assert!(
                !keys.iter().any(|k| k == forbidden),
                "PubState leaked hidden field {}",
                forbidden
            );
        }
    }

    #[test]
    fn cannot_fit_buy_error_shows_spare_capacity() {
        let mut board = PlayerBoard::new(0);
        *board.resources.entry(Resource::Food).or_insert(0) += 1;
        let err = board.cannot_fit_buy_error(Resource::Food, 3);
        assert_eq!(
            err,
            "not enough room for 3 food - you have room for 1 more food"
        );
    }

    #[test]
    fn cannot_fit_buy_error_zero_spare() {
        let mut board = PlayerBoard::new(0);
        *board.resources.entry(Resource::Food).or_insert(0) += 2;
        let err = board.cannot_fit_buy_error(Resource::Food, 1);
        assert_eq!(
            err,
            "not enough room for 1 food - you have room for 0 more food"
        );
    }

    #[test]
    fn buy_over_capacity_trade_and_build() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::TradeAndBuild;
        g.current_player = 0;
        g.player_boards[0].trading_posts = vec![SectorCard::Trade {
            name: "Test Trade".to_string(),
            resources: vec![Resource::Food],
            price: 1,
            maximum: 0,
            direction: TradeDir::Both,
            trading_post: true,
        }];
        *g.player_boards[0]
            .resources
            .entry(Resource::Food)
            .or_insert(0) += 1;
        g.player_boards[0].resources.insert(Resource::Astro, 100);
        let result = g.command(0, "buy 3 food", &players);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("you have room for 1 more food"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn can_next_and_can_end_non_terminal() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.flight_cards = vec![colony_card()];
        g.current_sector = 1;
        g.sector_cards.insert(1, vec![colony_card()]);
        g.yellow_dice = 3;
        g.player_boards[0].resources.insert(Resource::Booster, 0);
        g.flight_actions = BTreeMap::new();
        assert!(g.can_next(0));
        assert!(g.can_end(0));
    }

    #[test]
    fn can_next_false_when_pile_empty() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.flight_cards = vec![colony_card()];
        g.current_sector = 1;
        g.sector_cards.insert(1, vec![]);
        g.yellow_dice = 3;
        g.player_boards[0].resources.insert(Resource::Booster, 0);
        g.flight_actions = BTreeMap::new();
        assert!(!g.can_next(0));
        assert!(g.can_end(0));
    }

    #[test]
    fn can_next_false_when_no_moves() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.flight_cards = vec![colony_card()];
        g.current_sector = 1;
        g.sector_cards.insert(1, vec![colony_card()]);
        g.yellow_dice = 0;
        g.player_boards[0].resources.insert(Resource::Booster, 0);
        g.flight_actions = BTreeMap::new();
        assert!(!g.can_next(0));
        assert!(g.can_end(0));
    }

    #[test]
    fn can_next_false_when_no_actions() {
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.flight_cards = vec![colony_card()];
        g.current_sector = 1;
        g.sector_cards.insert(1, vec![colony_card()]);
        g.yellow_dice = 3;
        g.player_boards[0].resources.insert(Resource::Booster, 0);
        g.flight_actions = BTreeMap::from([(0, true), (1, true)]);
        assert!(!g.can_next(0));
        assert!(g.can_end(0));
    }

    #[test]
    fn command_next_rejected_terminal_end_allowed() {
        let players = players();
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::Flight;
        g.current_player = 0;
        g.flight_cards = vec![colony_card()];
        g.current_sector = 1;
        g.sector_cards.insert(1, vec![]);
        g.yellow_dice = 3;
        g.player_boards[0].resources.insert(Resource::Booster, 0);
        g.flight_actions = BTreeMap::new();
        assert!(g.command(0, "next", &players).is_err());
        assert!(g.command(0, "end", &players).is_ok());
    }

    #[test]
    fn last_sectors_leftmost_bold() {
        use brdgme_game::Renderer;
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::ChooseSector;
        g.player_boards[0].last_sectors = vec![3, 1, 2];
        let nodes = g.pub_state().render();
        let rendered = brdgme_markup::to_string(&nodes);
        assert!(
            rendered.contains("{{b}}3{{/b}} 1 2"),
            "expected bold leftmost entry, got: {rendered}"
        );
    }

    #[test]
    fn last_sectors_hidden_when_empty() {
        use brdgme_game::Renderer;
        let (mut g, _) = Game::start(2, 1).unwrap();
        g.phase = Phase::ChooseSector;
        g.player_boards[0].last_sectors = vec![];
        let nodes = g.pub_state().render();
        let rendered = brdgme_markup::to_string(&nodes);
        assert!(
            !rendered.contains("Last sectors"),
            "expected no Last sectors row, got: {rendered}"
        );
    }
}
