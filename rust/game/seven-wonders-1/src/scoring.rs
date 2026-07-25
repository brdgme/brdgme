use std::collections::HashMap;

use crate::Game;
use crate::card::{CardEffect, CardKind, DIR_LEFT, DIR_NEIGHBOURS, Field, all_fields};

impl Game {
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

    pub(crate) fn score_science(counts: &HashMap<Field, i32>) -> i32 {
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
                CardEffect::DrawDiscard { vp: stage_vp } => vp += stage_vp,
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
}
