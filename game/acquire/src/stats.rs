use serde_derive::{Serialize, Deserialize};

use brdgme_game::Stat;

use std::collections::HashMap;

use crate::Corp;

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub buy_sum: usize,
    pub buys: usize,
    pub sell_sum: usize,
    pub sells: usize,
    pub founds: Vec<Corp>,
    pub merges: usize,
    pub trades: usize,
    pub trade_loss_sum: usize,
    pub trade_gain_sum: usize,
    pub major_bonus_sum: usize,
    pub major_bonuses: usize,
    pub minor_bonus_sum: usize,
    pub minor_bonuses: usize,
}

impl Stats {
    pub fn to_brdgme_stats(&self) -> HashMap<String, Stat> {
        let mut s: HashMap<String, Stat> = HashMap::new();
        s.insert("Buys".to_string(), Stat::Int(self.buys as i32));
        s.insert("Buy total".to_string(), Stat::Int(self.buy_sum as i32));
        s.insert(
            "Buy average".to_string(),
            Stat::Fraction(self.buy_sum as i32, self.buys as i32),
        );
        s.insert("Sells".to_string(), Stat::Int(self.sells as i32));
        s.insert("Sell total".to_string(), Stat::Int(self.sell_sum as i32));
        s.insert(
            "Sell average".to_string(),
            Stat::Fraction(self.sell_sum as i32, self.sells as i32),
        );
        s.insert(
            "Corporations founded".to_string(),
            Stat::List(self.founds.iter().map(|c| c.name()).collect()),
        );
        s.insert("Merges".to_string(), Stat::Int(self.merges as i32));
        s.insert("Trades".to_string(), Stat::Int(self.merges as i32));
        s.insert(
            "Trade difference".to_string(),
            Stat::Int(self.trade_gain_sum as i32 - self.trade_loss_sum as i32),
        );
        let bonuses = self.major_bonuses as i32 + self.minor_bonuses as i32;
        let bonus_sum = self.major_bonus_sum as i32 + self.minor_bonus_sum as i32;
        s.insert("Bonuses".to_string(), Stat::Int(bonuses));
        s.insert("Bonus total".to_string(), Stat::Int(bonus_sum));
        s.insert(
            "Bonus average".to_string(),
            Stat::Fraction(bonus_sum, bonuses),
        );
        s.insert(
            "Major bonuses".to_string(),
            Stat::Int(self.major_bonuses as i32),
        );
        s.insert(
            "Major bonus total".to_string(),
            Stat::Int(self.major_bonus_sum as i32),
        );
        s.insert(
            "Major bonus average".to_string(),
            Stat::Fraction(self.major_bonus_sum as i32, self.major_bonuses as i32),
        );
        s.insert(
            "Minor bonuses".to_string(),
            Stat::Int(self.minor_bonuses as i32),
        );
        s.insert(
            "Minor bonus total".to_string(),
            Stat::Int(self.minor_bonus_sum as i32),
        );
        s.insert(
            "Minor bonus average".to_string(),
            Stat::Fraction(self.minor_bonus_sum as i32, self.minor_bonuses as i32),
        );
        s
    }
}
