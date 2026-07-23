use std::collections::HashMap;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub struct PlacingInput {
    pub game_player_id: Uuid,
    pub is_pure_bot: bool,
    pub left_at: Option<PrimitiveDateTime>,
    pub game_placing: Option<i32>,
}

pub fn compute_ranked_placings(players: &[PlacingInput]) -> HashMap<Uuid, i32> {
    let mut ranked: HashMap<Uuid, i32> = HashMap::new();

    let mut survivors: Vec<&PlacingInput> = players
        .iter()
        .filter(|p| !p.is_pure_bot && p.left_at.is_none())
        .collect();
    survivors.sort_by(|a, b| {
        a.game_placing
            .unwrap_or(i32::MAX)
            .cmp(&b.game_placing.unwrap_or(i32::MAX))
            .then(a.game_player_id.cmp(&b.game_player_id))
    });

    let mut leavers: Vec<&PlacingInput> = players
        .iter()
        .filter(|p| !p.is_pure_bot && p.left_at.is_some())
        .collect();
    leavers.sort_by(|a, b| {
        b.left_at
            .cmp(&a.left_at)
            .then(a.game_player_id.cmp(&b.game_player_id))
    });

    let mut place = 1;
    for p in survivors {
        ranked.insert(p.game_player_id, place);
        place += 1;
    }
    for p in leavers {
        ranked.insert(p.game_player_id, place);
        place += 1;
    }
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(minute: u8) -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(0, minute, 0).unwrap(),
        )
    }

    #[test]
    fn spec_worked_example() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        let bot_b = Uuid::new_v4();
        let players = vec![
            PlacingInput {
                game_player_id: a,
                is_pure_bot: false,
                left_at: Some(ts(1)),
                game_placing: Some(4),
            },
            PlacingInput {
                game_player_id: b,
                is_pure_bot: false,
                left_at: Some(ts(2)),
                game_placing: None,
            },
            PlacingInput {
                game_player_id: bot_b,
                is_pure_bot: true,
                left_at: None,
                game_placing: Some(1),
            },
            PlacingInput {
                game_player_id: c,
                is_pure_bot: false,
                left_at: Some(ts(3)),
                game_placing: Some(3),
            },
            PlacingInput {
                game_player_id: d,
                is_pure_bot: false,
                left_at: None,
                game_placing: Some(2),
            },
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
        let winner = Uuid::new_v4();
        let conceder = Uuid::new_v4();
        let players = vec![
            PlacingInput {
                game_player_id: winner,
                is_pure_bot: false,
                left_at: None,
                game_placing: Some(1),
            },
            PlacingInput {
                game_player_id: conceder,
                is_pure_bot: false,
                left_at: Some(ts(5)),
                game_placing: Some(2),
            },
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&winner), Some(&1));
        assert_eq!(ranked.get(&conceder), Some(&2));
    }

    #[test]
    fn survivors_ordered_by_game_placing() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let p3 = Uuid::new_v4();
        let players = vec![
            PlacingInput {
                game_player_id: p1,
                is_pure_bot: false,
                left_at: None,
                game_placing: Some(2),
            },
            PlacingInput {
                game_player_id: p2,
                is_pure_bot: false,
                left_at: None,
                game_placing: Some(1),
            },
            PlacingInput {
                game_player_id: p3,
                is_pure_bot: false,
                left_at: Some(ts(1)),
                game_placing: Some(3),
            },
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&p2), Some(&1));
        assert_eq!(ranked.get(&p1), Some(&2));
        assert_eq!(ranked.get(&p3), Some(&3));
    }
}
