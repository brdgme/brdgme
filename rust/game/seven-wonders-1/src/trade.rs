use std::collections::HashMap;

use brdgme_cost::{Cost, can_afford_perm};

use crate::card::{CardEffect, DIR_LEFT, DIR_RIGHT, Good};
use crate::{BASE_TRADE_COST, DISCOUNTED_TRADE_COST, Game};

impl Game {
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

    pub(crate) fn resolve_deal(
        &self,
        player: usize,
        cost: &Cost<Good>,
        deal: Option<usize>,
        deal_coins: Option<&HashMap<i32, i32>>,
    ) -> HashMap<i32, i32> {
        if let Some(coins) = deal_coins {
            return coins.clone();
        }
        // Legacy fallback for pre-upgrade pending actions only (b F9).
        match deal {
            Some(idx) => {
                let (_, deals) = self.can_afford_cost(player, cost);
                deals.get(idx).cloned().unwrap_or_default()
            }
            None => HashMap::new(),
        }
    }

    pub(crate) fn pay_cost(&mut self, player: usize, cost: &Cost<Good>, deal: &HashMap<i32, i32>) {
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
}
