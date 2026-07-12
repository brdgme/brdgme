//! Port of `brdgme-go/roll_through_the_ages_1/player_board.go`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::development::DevelopmentId;
use crate::good::{GOODS, Good, good_maximum, good_value};
use crate::monument::{MONUMENTS, MonumentId};

pub const BASE_CITY_SIZE: i32 = 3;
pub const GOODS_LIMIT: i32 = 6;
pub const CITY_LEVELS: [i32; 4] = [3, 7, 12, 18];
pub const MAX_CITY_PROGRESS: i32 = 18; // CITY_LEVELS[CITY_LEVELS.len() - 1]

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerBoard {
    pub city_progress: i32,
    pub developments: HashSet<DevelopmentId>,
    /// Progress towards each monument (workers spent building it so far).
    pub monuments: HashMap<MonumentId, i32>,
    pub monument_built_first: HashSet<MonumentId>,
    pub food: i32,
    pub goods: HashMap<Good, i32>,
    pub disasters: i32,
    pub ships: i32,
}

impl Default for PlayerBoard {
    /// Port of `NewPlayerBoard`.
    fn default() -> Self {
        PlayerBoard {
            city_progress: 0,
            developments: HashSet::new(),
            monuments: HashMap::new(),
            monument_built_first: HashSet::new(),
            food: 3,
            goods: HashMap::new(),
            disasters: 0,
            ships: 0,
        }
    }
}

impl PlayerBoard {
    /// Port of `Cities`.
    pub fn cities(&self) -> i32 {
        let mut size = BASE_CITY_SIZE;
        for &l in CITY_LEVELS.iter() {
            if self.city_progress < l {
                break;
            }
            size += 1;
        }
        size
    }

    /// Port of `Score`.
    pub fn score(&self) -> i32 {
        let mut score = 0;
        // Developments
        for &d in self.developments.iter() {
            score += d.value().points;
        }
        // Monuments
        let mut built_monuments = 0;
        for &m in MONUMENTS.iter() {
            let num = self.monuments.get(&m).copied().unwrap_or(0);
            let mv = m.value();
            if num >= mv.size {
                built_monuments += 1;
                if self.monument_built_first.contains(&m) {
                    score += mv.points;
                } else {
                    score += mv.subsequent_points;
                }
            }
        }
        // Bonus points
        if self.developments.contains(&DevelopmentId::Commerce) {
            score += self.goods_num();
        }
        if self.developments.contains(&DevelopmentId::Architecture) {
            score += built_monuments * 2;
        }
        if self.developments.contains(&DevelopmentId::Empire) {
            score += self.cities();
        }
        score - self.disasters
    }

    /// Port of `CoinsDieValue`.
    pub fn coins_die_value(&self) -> i32 {
        if self.developments.contains(&DevelopmentId::Coinage) {
            12
        } else {
            7
        }
    }

    /// Port of `FoodModifier`.
    pub fn food_modifier(&self) -> i32 {
        if self.developments.contains(&DevelopmentId::Agriculture) {
            1
        } else {
            0
        }
    }

    /// Port of `WorkerModifier`.
    pub fn worker_modifier(&self) -> i32 {
        if self.developments.contains(&DevelopmentId::Masonry) {
            1
        } else {
            0
        }
    }

    /// Port of `GainGoods`. Round-robin distribution always starts at Wood,
    /// every call, regardless of where a previous call left off (Go quirk
    /// #3 - preserved verbatim).
    pub fn gain_goods(&mut self, n: i32) {
        let mut quarrying_used = false;
        let mut good_idx = 0usize;
        for _ in 0..n {
            let good = GOODS[good_idx];
            self.gain_good(good);
            // Extra stone if player has quarrying, once per call.
            if good == Good::Stone
                && self.developments.contains(&DevelopmentId::Quarrying)
                && !quarrying_used
            {
                *self.goods.entry(good).or_insert(0) += 1;
                quarrying_used = true;
            }
            good_idx = (good_idx + 1) % GOODS.len();
        }
    }

    /// Port of `GainGood`.
    pub fn gain_good(&mut self, good: Good) {
        let max = good_maximum(good);
        let cur = self.goods.get(&good).copied().unwrap_or(0);
        if cur < max {
            self.goods.insert(good, cur + 1);
        }
    }

    /// Port of `GoodsNum`.
    pub fn goods_num(&self) -> i32 {
        self.goods.values().sum()
    }

    /// Port of `GoodsValue`.
    pub fn goods_value(&self) -> i32 {
        self.goods.iter().map(|(&g, &n)| good_value(g, n)).sum()
    }

    /// Port of `HasBuilt`.
    pub fn has_built(&self, monument: MonumentId) -> bool {
        self.monuments.get(&monument).copied().unwrap_or(0) >= monument.value().size
    }

    /// Port of `GoodsOverLimit`.
    pub fn goods_over_limit(&self) -> i32 {
        if self.developments.contains(&DevelopmentId::Caravans) {
            return 0;
        }
        let over_limit = self.goods_num() - GOODS_LIMIT;
        if over_limit < 0 { 0 } else { over_limit }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::good::Good;
    use crate::monument::MonumentId;

    #[test]
    fn cities_at_each_city_level_boundary() {
        let mut b = PlayerBoard::default();
        for (progress, expected) in [
            (0, 3),
            (2, 3),
            (3, 4),
            (6, 4),
            (7, 5),
            (11, 5),
            (12, 6),
            (17, 6),
            (18, 7),
        ] {
            b.city_progress = progress;
            assert_eq!(expected, b.cities(), "progress={}", progress);
        }
    }

    #[test]
    fn score_development_points() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Leadership); // 2 pts
        b.developments.insert(DevelopmentId::Empire); // 10 pts + city bonus
        assert_eq!(2 + 10 + b.cities(), b.score());
    }

    #[test]
    fn score_monument_first_vs_subsequent_points() {
        let mut first = PlayerBoard::default();
        first.monuments.insert(MonumentId::Temple, 7);
        first.monument_built_first.insert(MonumentId::Temple);
        assert_eq!(4, first.score());

        let mut later = PlayerBoard::default();
        later.monuments.insert(MonumentId::Temple, 7);
        assert_eq!(3, later.score());
    }

    #[test]
    fn score_partially_built_monument_scores_nothing() {
        let mut b = PlayerBoard::default();
        b.monuments.insert(MonumentId::Temple, 3);
        assert_eq!(0, b.score());
    }

    #[test]
    fn score_commerce_bonus() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Commerce); // 8 pts
        b.goods.insert(Good::Wood, 3);
        b.goods.insert(Good::Stone, 2);
        assert_eq!(8 + 5, b.score());
    }

    #[test]
    fn score_architecture_bonus() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Architecture); // 8 pts
        b.monuments.insert(MonumentId::StepPyramid, 3);
        b.monument_built_first.insert(MonumentId::StepPyramid);
        b.monuments.insert(MonumentId::StoneCircle, 5);
        b.monument_built_first.insert(MonumentId::StoneCircle);
        // 8 (dev) + 1 (step pyramid) + 2 (stone circle) + 2*2 (2 monuments)
        assert_eq!(8 + 1 + 2 + 4, b.score());
    }

    #[test]
    fn score_empire_bonus() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Empire); // 10 pts
        b.city_progress = 7; // cities() == 5
        assert_eq!(10 + 5, b.score());
    }

    #[test]
    fn score_combined_bonuses_and_disaster_subtraction() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Commerce);
        b.developments.insert(DevelopmentId::Architecture);
        b.developments.insert(DevelopmentId::Empire);
        b.goods.insert(Good::Wood, 2);
        b.monuments.insert(MonumentId::StepPyramid, 3);
        b.monument_built_first.insert(MonumentId::StepPyramid);
        b.city_progress = 3; // cities() == 4
        b.disasters = 3;
        let expected = 8 + 8 + 10 + 2 /* commerce goods */ + 1 /* step pyramid */ + 2 /* architecture */ + 4 /* empire cities */ - 3;
        assert_eq!(expected, b.score());
    }

    #[test]
    fn coins_die_value_with_and_without_coinage() {
        let mut b = PlayerBoard::default();
        assert_eq!(7, b.coins_die_value());
        b.developments.insert(DevelopmentId::Coinage);
        assert_eq!(12, b.coins_die_value());
    }

    #[test]
    fn food_modifier_with_and_without_agriculture() {
        let mut b = PlayerBoard::default();
        assert_eq!(0, b.food_modifier());
        b.developments.insert(DevelopmentId::Agriculture);
        assert_eq!(1, b.food_modifier());
    }

    #[test]
    fn worker_modifier_with_and_without_masonry() {
        let mut b = PlayerBoard::default();
        assert_eq!(0, b.worker_modifier());
        b.developments.insert(DevelopmentId::Masonry);
        assert_eq!(1, b.worker_modifier());
    }

    #[test]
    fn gain_goods_round_robin_starts_at_wood_every_call() {
        let mut b = PlayerBoard::default();
        b.gain_goods(2);
        assert_eq!(Some(&1), b.goods.get(&Good::Wood));
        assert_eq!(Some(&1), b.goods.get(&Good::Stone));
        assert_eq!(None, b.goods.get(&Good::Pottery));

        // Second call restarts at Wood rather than continuing at Pottery
        // (Go quirk #3, preserved verbatim).
        b.gain_goods(1);
        assert_eq!(Some(&2), b.goods.get(&Good::Wood));
        assert_eq!(Some(&1), b.goods.get(&Good::Stone));
    }

    #[test]
    fn gain_goods_quarrying_bonus_once_per_call() {
        let mut b = PlayerBoard::default();
        b.developments.insert(DevelopmentId::Quarrying);
        // 7 dice: wood,stone,pottery,cloth,spearhead,wood,stone -> stone hit
        // twice, but bonus should only apply once.
        b.gain_goods(7);
        assert_eq!(Some(&3), b.goods.get(&Good::Stone)); // 2 from round-robin + 1 bonus
    }

    #[test]
    fn gain_goods_respects_good_maximum_cap() {
        let mut b = PlayerBoard::default();
        // Spearhead cap is 4; feed 5*5=25 dice through, landing on spearhead
        // 5 times (index 4 of each 5-cycle).
        b.gain_goods(25);
        assert_eq!(Some(&4), b.goods.get(&Good::Spearhead));
    }

    #[test]
    fn goods_over_limit_with_and_without_caravans() {
        let mut b = PlayerBoard::default();
        b.goods.insert(Good::Wood, 8);
        assert_eq!(2, b.goods_over_limit());
        b.developments.insert(DevelopmentId::Caravans);
        assert_eq!(0, b.goods_over_limit());
    }

    #[test]
    fn goods_over_limit_at_or_below_limit_is_zero() {
        let mut b = PlayerBoard::default();
        b.goods.insert(Good::Wood, 6);
        assert_eq!(0, b.goods_over_limit());
    }

    #[test]
    fn has_built_at_below_above_size() {
        let mut b = PlayerBoard::default();
        b.monuments.insert(MonumentId::StepPyramid, 2);
        assert!(!b.has_built(MonumentId::StepPyramid));
        b.monuments.insert(MonumentId::StepPyramid, 3);
        assert!(b.has_built(MonumentId::StepPyramid));
        b.monuments.insert(MonumentId::StepPyramid, 4);
        assert!(b.has_built(MonumentId::StepPyramid));
    }
}
