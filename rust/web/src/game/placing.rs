use std::collections::HashMap;
use uuid::Uuid;

pub struct PlacingInput {
    pub game_player_id: Uuid,
    pub is_pure_bot: bool,
    pub departure_sequence: Option<i32>,
    pub game_placing: Option<i32>,
}

/// Computes human-only competitive placings (`ranked_placing`).
///
/// A human participant is any row that is not a pure bot; replaced humans keep
/// their user identity and participate. Pure bots have no result. Fewer than
/// two humans yields no competitive result.
///
/// Active humans (no departure sequence) are ranked ahead of departed humans,
/// using authoritative game `place` after bot removal with standard-competition
/// ties. Departed humans follow in descending departure-event sequence, tied
/// within an event, with ranks advancing by member count across every tied
/// group. Only `departure_sequence` decides departed status and ordering.
///
/// A last-human stop has no game places and ranks either the single remaining
/// active human first, or all departed humans in reverse departure-event order.
/// No ranking is invented for incomplete data, and game points are never a
/// ranking source.
pub fn compute_ranked_placings(players: &[PlacingInput]) -> HashMap<Uuid, i32> {
    let humans: Vec<&PlacingInput> = players.iter().filter(|p| !p.is_pure_bot).collect();
    if humans.len() < 2 {
        return HashMap::new();
    }

    let active: Vec<&PlacingInput> = humans
        .iter()
        .copied()
        .filter(|p| p.departure_sequence.is_none())
        .collect();
    let departed: Vec<&PlacingInput> = humans
        .iter()
        .copied()
        .filter(|p| p.departure_sequence.is_some())
        .collect();

    let mut ranked: HashMap<Uuid, i32> = HashMap::new();

    let has_game_places = humans.iter().any(|p| p.game_placing.is_some());
    if has_game_places {
        // Normal service result. Active humans need authoritative places;
        // missing places on an active human are unsupported and yield no result.
        let mut by_place: Vec<(Uuid, i32)> = Vec::with_capacity(active.len());
        for p in &active {
            match p.game_placing {
                Some(place) => by_place.push((p.game_player_id, place)),
                None => return HashMap::new(),
            }
        }
        by_place.sort_by_key(|(_, place)| *place);
        let mut place = 1;
        let mut i = 0;
        while i < by_place.len() {
            let group_value = by_place[i].1;
            let mut j = i;
            while j < by_place.len() && by_place[j].1 == group_value {
                ranked.insert(by_place[j].0, place);
                j += 1;
            }
            place += (j - i) as i32;
            i = j;
        }
        let mut by_departure: Vec<(Uuid, i32)> = Vec::with_capacity(departed.len());
        for p in &departed {
            if let Some(seq) = p.departure_sequence {
                by_departure.push((p.game_player_id, seq));
            }
        }
        by_departure.sort_by_key(|(_, seq)| std::cmp::Reverse(*seq));
        let mut i = 0;
        while i < by_departure.len() {
            let group_value = by_departure[i].1;
            let mut j = i;
            while j < by_departure.len() && by_departure[j].1 == group_value {
                ranked.insert(by_departure[j].0, place);
                j += 1;
            }
            place += (j - i) as i32;
            i = j;
        }
        return ranked;
    }

    // Last-human stop: no game places at all.
    match active.len() {
        // Exactly one active human ranks first, then departed in reverse
        // departure-event order.
        1 => {
            for p in &active {
                ranked.insert(p.game_player_id, 1);
            }
            let mut by_departure: Vec<(Uuid, i32)> = Vec::with_capacity(departed.len());
            for p in &departed {
                if let Some(seq) = p.departure_sequence {
                    by_departure.push((p.game_player_id, seq));
                }
            }
            by_departure.sort_by_key(|(_, seq)| std::cmp::Reverse(*seq));
            let mut place = 2;
            let mut i = 0;
            while i < by_departure.len() {
                let group_value = by_departure[i].1;
                let mut j = i;
                while j < by_departure.len() && by_departure[j].1 == group_value {
                    ranked.insert(by_departure[j].0, place);
                    j += 1;
                }
                place += (j - i) as i32;
                i = j;
            }
        }
        // Zero active humans: every human participant ranks in reverse
        // departure-event order, latest tied event first.
        0 => {
            let mut by_departure: Vec<(Uuid, i32)> = Vec::with_capacity(departed.len());
            for p in &departed {
                if let Some(seq) = p.departure_sequence {
                    by_departure.push((p.game_player_id, seq));
                }
            }
            by_departure.sort_by_key(|(_, seq)| std::cmp::Reverse(*seq));
            let mut place = 1;
            let mut i = 0;
            while i < by_departure.len() {
                let group_value = by_departure[i].1;
                let mut j = i;
                while j < by_departure.len() && by_departure[j].1 == group_value {
                    ranked.insert(by_departure[j].0, place);
                    j += 1;
                }
                place += (j - i) as i32;
                i = j;
            }
        }
        // Multiple active humans with no game places: unsupported, no result.
        _ => {}
    }
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn player(
        id: Uuid,
        is_pure_bot: bool,
        departure_sequence: Option<i32>,
        game_placing: Option<i32>,
    ) -> PlacingInput {
        PlacingInput {
            game_player_id: id,
            is_pure_bot,
            departure_sequence,
            game_placing,
        }
    }

    fn uuid(n: u32) -> Uuid {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&n.to_be_bytes());
        Uuid::from_bytes(b)
    }

    #[test]
    fn active_before_departed_and_bot_removed() {
        let bot = uuid(1);
        let active_low = uuid(2);
        let active_high = uuid(3);
        let departed_latest = uuid(4);
        let departed_earlier = uuid(5);
        let players = vec![
            player(bot, true, None, Some(1)),
            player(active_low, false, None, Some(2)),
            player(active_high, false, None, Some(4)),
            player(departed_latest, false, Some(5), Some(3)),
            player(departed_earlier, false, Some(2), None),
        ];
        let ranked = compute_ranked_placings(&players);
        // The bot's game placing of 1 must not influence the competitive order.
        assert_eq!(ranked.get(&active_low), Some(&1));
        assert_eq!(ranked.get(&active_high), Some(&2));
        assert_eq!(ranked.get(&departed_latest), Some(&3));
        assert_eq!(ranked.get(&departed_earlier), Some(&4));
        assert!(!ranked.contains_key(&bot));
    }

    #[test]
    fn replaced_human_participates() {
        let replaced = uuid(1);
        let survivor = uuid(2);
        // A replaced human participates in the competitive ranking.
        let players = vec![
            player(replaced, false, Some(1), Some(2)),
            player(survivor, false, None, Some(1)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&survivor), Some(&1));
        assert_eq!(ranked.get(&replaced), Some(&2));
    }

    #[test]
    fn active_authoritative_ties_use_competition_ranks() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let d = uuid(4);
        let players = vec![
            player(a, false, None, Some(1)),
            player(b, false, None, Some(1)),
            player(c, false, None, Some(3)),
            player(d, false, Some(1), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&a), Some(&1));
        assert_eq!(ranked.get(&b), Some(&1));
        assert_eq!(ranked.get(&c), Some(&3));
        assert_eq!(ranked.get(&d), Some(&4));
    }

    #[test]
    fn departed_tie_shared_rank_advances_by_group_size() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let d = uuid(4);
        let e = uuid(5);
        let players = vec![
            player(a, false, None, Some(1)),
            player(b, false, Some(4), None),
            player(c, false, Some(4), None),
            player(d, false, Some(2), None),
            player(e, false, Some(2), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&a), Some(&1));
        assert_eq!(ranked.get(&b), Some(&2));
        assert_eq!(ranked.get(&c), Some(&2));
        assert_eq!(ranked.get(&d), Some(&4));
        assert_eq!(ranked.get(&e), Some(&4));
    }

    #[test]
    fn departed_ignore_left_at_and_non_sequence_inputs() {
        // Only departure_sequence may decide departed ordering. These rows have
        // conflicting signals (game places and raw order in the input), but the
        // sequence must win.
        let active = uuid(1);
        let later_event = uuid(2);
        let earlier_event = uuid(3);
        let players = vec![
            // Input order reverses the sequence order on purpose.
            player(earlier_event, false, Some(1), Some(3)),
            player(active, false, None, Some(1)),
            player(later_event, false, Some(2), Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&active), Some(&1));
        assert_eq!(ranked.get(&later_event), Some(&2));
        assert_eq!(ranked.get(&earlier_event), Some(&3));
    }

    #[test]
    fn last_human_stop_one_active_first_then_reverse_sequence() {
        let active = uuid(1);
        let latest_event = uuid(2);
        let earliest_event = uuid(3);
        let players = vec![
            player(active, false, None, None),
            player(earliest_event, false, Some(1), None),
            player(latest_event, false, Some(3), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&active), Some(&1));
        assert_eq!(ranked.get(&latest_event), Some(&2));
        assert_eq!(ranked.get(&earliest_event), Some(&3));
    }

    #[test]
    fn last_human_stop_zero_active_reverse_sequence_latest_tied_first() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let players = vec![
            player(a, false, Some(2), None),
            player(b, false, Some(1), None),
            player(c, false, Some(2), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&a), Some(&1));
        assert_eq!(ranked.get(&c), Some(&1));
        assert_eq!(ranked.get(&b), Some(&3));
    }

    #[test]
    fn zero_humans_yields_no_result() {
        let bot1 = uuid(1);
        let bot2 = uuid(2);
        let players = vec![
            player(bot1, true, None, Some(1)),
            player(bot2, true, Some(1), Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert!(ranked.is_empty());
    }

    #[test]
    fn one_human_yields_no_result() {
        let solo = uuid(1);
        let bot = uuid(2);
        let players = vec![
            player(solo, false, None, Some(1)),
            player(bot, true, None, Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert!(ranked.is_empty());
    }

    #[test]
    fn all_active_normal_finish_ranks_humans() {
        let a = uuid(1);
        let b = uuid(2);
        let players = vec![
            player(a, false, None, Some(1)),
            player(b, false, None, Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&a), Some(&1));
        assert_eq!(ranked.get(&b), Some(&2));
    }

    #[test]
    fn all_active_normal_finish_bot_filtered_with_ties() {
        let bot = uuid(1);
        let tied_a = uuid(2);
        let tied_b = uuid(3);
        let third = uuid(4);
        let players = vec![
            player(bot, true, None, Some(1)),
            player(tied_a, false, None, Some(2)),
            player(tied_b, false, None, Some(2)),
            player(third, false, None, Some(3)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert!(!ranked.contains_key(&bot));
        assert_eq!(ranked.get(&tied_a), Some(&1));
        assert_eq!(ranked.get(&tied_b), Some(&1));
        // Competition ranking: the next distinct place skips to 3.
        assert_eq!(ranked.get(&third), Some(&3));
    }

    #[test]
    fn multiple_active_humans_without_game_places_is_unsupported() {
        let a = uuid(1);
        let b = uuid(2);
        let departed = uuid(3);
        let players = vec![
            player(a, false, None, None),
            player(b, false, None, None),
            player(departed, false, Some(1), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert!(ranked.is_empty());
    }

    #[test]
    fn active_human_missing_game_place_is_incomplete_no_fallback() {
        let placed = uuid(1);
        let unplaced = uuid(2);
        let departed = uuid(3);
        let players = vec![
            player(placed, false, None, Some(1)),
            player(unplaced, false, None, None),
            player(departed, false, Some(1), None),
        ];
        // No invented ranking: a missing authoritative place on an active
        // human with other places present is unsupported data.
        let ranked = compute_ranked_placings(&players);
        assert!(ranked.is_empty());
    }

    #[test]
    fn departed_rows_do_not_require_game_places() {
        // A departed human may legitimately have no game placing; its rank
        // still comes from departure sequence only.
        let active = uuid(1);
        let departed = uuid(2);
        let players = vec![
            player(active, false, None, Some(1)),
            player(departed, false, Some(1), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&active), Some(&1));
        assert_eq!(ranked.get(&departed), Some(&2));
    }

    #[test]
    fn standard_competition_gap_after_active_tie() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let d = uuid(4);
        let players = vec![
            player(a, false, None, Some(1)),
            player(b, false, None, Some(1)),
            player(c, false, None, Some(2)),
            player(d, false, Some(1), None),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&a), Some(&1));
        assert_eq!(ranked.get(&b), Some(&1));
        // Competition ranking: the next distinct place skips to 3.
        assert_eq!(ranked.get(&c), Some(&3));
        assert_eq!(ranked.get(&d), Some(&4));
    }

    #[test]
    fn ranks_are_unique_when_no_ties() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let d = uuid(4);
        let players = vec![
            player(a, false, None, Some(2)),
            player(b, false, None, Some(1)),
            player(c, false, Some(1), Some(3)),
            player(d, false, Some(2), Some(4)),
        ];
        let ranked = compute_ranked_placings(&players);
        let values: HashSet<i32> = ranked.values().copied().collect();
        assert_eq!(values, [1, 2, 3, 4].into_iter().collect());
    }

    #[test]
    fn spec_worked_example() {
        let a = uuid(1);
        let b = uuid(2);
        let c = uuid(3);
        let d = uuid(4);
        let bot_b = uuid(5);
        let players = vec![
            player(a, false, Some(1), Some(4)),
            player(b, false, Some(2), None),
            player(bot_b, true, None, Some(1)),
            player(c, false, Some(3), Some(3)),
            player(d, false, None, Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&d), Some(&1)); // survivor
        assert_eq!(ranked.get(&c), Some(&2)); // latest leaver
        assert_eq!(ranked.get(&b), Some(&3));
        assert_eq!(ranked.get(&a), Some(&4)); // earliest leaver
        assert!(!ranked.contains_key(&bot_b)); // pure bot omitted
    }

    #[test]
    fn two_player_concede() {
        let winner = uuid(1);
        let conceder = uuid(2);
        let players = vec![
            player(winner, false, None, Some(1)),
            player(conceder, false, Some(1), Some(2)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&winner), Some(&1));
        assert_eq!(ranked.get(&conceder), Some(&2));
    }

    #[test]
    fn survivors_ordered_by_game_placing() {
        let p1 = uuid(1);
        let p2 = uuid(2);
        let p3 = uuid(3);
        let players = vec![
            player(p1, false, None, Some(2)),
            player(p2, false, None, Some(1)),
            player(p3, false, Some(1), Some(3)),
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&p2), Some(&1));
        assert_eq!(ranked.get(&p1), Some(&2));
        assert_eq!(ranked.get(&p3), Some(&3));
    }
}
