use std::cmp::Ordering;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_derive::{Deserialize, Serialize};

use brdgme_markup::Node;

use crate::command;
use crate::errors::GameError;
use crate::game_log::Log;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Stat {
    Int(i32),
    Float(f32),
    List(Vec<String>),
    Fraction(i32, i32),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Status {
    Active {
        whose_turn: Vec<usize>,
        eliminated: Vec<usize>,
    },
    Finished {
        placings: Vec<usize>,
        stats: Vec<HashMap<String, Stat>>,
    },
}

impl Status {
    pub fn is_finished(&self) -> bool {
        match *self {
            Status::Active { .. } => false,
            Status::Finished { .. } => true,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct CommandResponse {
    pub logs: Vec<Log>,
    pub can_undo: bool,
    pub remaining_input: String,
}

pub trait Gamer: Sized {
    type PubState: Serialize + DeserializeOwned + Renderer;
    type PlayerState: Serialize + DeserializeOwned + Renderer;

    fn new(players: usize) -> Result<(Self, Vec<Log>), GameError>;
    fn pub_state(&self) -> Self::PubState;
    fn player_state(&self, player: usize) -> Self::PlayerState;
    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError>;
    fn status(&self) -> Status;
    fn command_spec(&self, player: usize) -> Option<command::Spec>;
    fn player_count(&self) -> usize;
    fn player_counts() -> Vec<usize>;

    fn is_finished(&self) -> bool {
        match self.status() {
            Status::Finished { .. } => true,
            _ => false,
        }
    }

    fn whose_turn(&self) -> Vec<usize> {
        match self.status() {
            Status::Active { whose_turn: wt, .. } => wt,
            _ => vec![],
        }
    }

    fn eliminated(&self) -> Vec<usize> {
        match self.status() {
            Status::Active { eliminated: e, .. } => e,
            _ => vec![],
        }
    }

    fn placings(&self) -> Vec<usize> {
        match self.status() {
            Status::Finished { placings, .. } => placings,
            _ => vec![],
        }
    }

    fn stats(&self) -> Vec<HashMap<String, Stat>> {
        match self.status() {
            Status::Finished { stats, .. } => stats,
            _ => vec![],
        }
    }

    fn assert_not_finished(&self) -> Result<(), GameError> {
        if self.is_finished() {
            Err(GameError::Finished)
        } else {
            Ok(())
        }
    }

    fn assert_player_turn(&self, player: usize) -> Result<(), GameError> {
        match self.whose_turn().iter().position(|&p| p == player) {
            Some(_) => Ok(()),
            None => Err(GameError::NotYourTurn),
        }
    }

    fn points(&self) -> Vec<f32> {
        vec![]
    }
}

pub trait Renderer {
    fn render(&self) -> Vec<Node>;
}

fn cmp_fallback(a: &[i32], b: &[i32]) -> Ordering {
    if a.is_empty() && b.is_empty() {
        return Ordering::Equal;
    }
    if a.is_empty() {
        return Ordering::Less;
    }
    if b.is_empty() {
        return Ordering::Greater;
    }
    match a[0].partial_cmp(&b[0]) {
        Some(Ordering::Equal) | None => cmp_fallback(&a[1..], &b[1..]),
        Some(ord) => ord,
    }
}

pub fn gen_placings(metrics: &[Vec<i32>]) -> Vec<usize> {
    let mut grouped: HashMap<&Vec<i32>, Vec<usize>> = HashMap::new();
    for (player, m) in metrics.iter().enumerate() {
        let entry = grouped.entry(m).or_insert_with(|| vec![]);
        entry.push(player);
    }

    let mut keys: Vec<&Vec<i32>> = grouped.keys().cloned().collect();
    keys.sort_by(|a, b| cmp_fallback(a.as_ref(), b.as_ref()));

    let mut placings: HashMap<usize, usize> = HashMap::new();
    let mut cur_place = 1;
    for key in keys.iter().rev() {
        let players = &grouped[key];
        for player in players {
            placings.insert(*player, cur_place);
        }
        cur_place += players.len();
    }

    metrics
        .iter()
        .enumerate()
        .map(|(player, _)| placings[&player])
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn gen_placings_works() {
        assert_eq!(
            vec![2, 1],
            super::gen_placings(&[vec![12i32, 34i32], vec![13i32, 33i32]])
        );
        assert_eq!(
            vec![1, 1],
            super::gen_placings(&[vec![12i32, 34i32], vec![12i32, 34i32]])
        );
        assert_eq!(
            vec![1, 2],
            super::gen_placings(&[vec![12i32, 36i32], vec![12i32, 35i32]])
        );
        assert_eq!(
            vec![1, 2],
            super::gen_placings(&[vec![12i32, 35i32, 0i32], vec![12i32, 35i32]])
        );
    }
}
