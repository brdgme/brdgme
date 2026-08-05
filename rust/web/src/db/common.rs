//! Shared row builders and pure helpers. `validate_username` is deliberately
//! ungated (no `#[cfg(feature = "ssr")]`) so the client settings form and the
//! server fns share one definition; the other items stay individually gated.
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use uuid::Uuid;

/// Starting rating for a player in a game type. Matches the
/// `game_type_users.rating` column default and the synthetic-row value in
/// `build_game_type_user`; rating reconstruction (`stats::queries::rating_series`)
/// seeds its running total from the same value.
pub const INITIAL_RATING: i32 = 1200;

#[cfg(feature = "ssr")]
pub(crate) fn build_user_from_row(
    id: Option<Uuid>,
    created_at: Option<time::PrimitiveDateTime>,
    updated_at: Option<time::PrimitiveDateTime>,
    name: Option<String>,
    pref_colors: Option<Vec<String>>,
) -> Result<Option<crate::models::user::User>> {
    let Some(id) = id else { return Ok(None) };
    Ok(Some(crate::models::user::User {
        id,
        created_at: created_at
            .ok_or_else(|| anyhow::anyhow!("user {id}: created_at missing from LEFT JOIN row"))?,
        updated_at: updated_at
            .ok_or_else(|| anyhow::anyhow!("user {id}: updated_at missing from LEFT JOIN row"))?,
        name: name.ok_or_else(|| anyhow::anyhow!("user {id}: name missing from LEFT JOIN row"))?,
        pref_colors: pref_colors
            .ok_or_else(|| anyhow::anyhow!("user {id}: pref_colors missing from LEFT JOIN row"))?,
        theme: None,
        is_admin: false,
    }))
}

#[cfg(feature = "ssr")]
pub(crate) fn build_game_bot_from_row(
    id: Option<Uuid>,
    game_id: Option<Uuid>,
    name: Option<String>,
    bot_name: Option<String>,
) -> Result<Option<crate::models::game::GameBot>> {
    let Some(id) = id else { return Ok(None) };
    Ok(Some(crate::models::game::GameBot {
        id,
        game_id: game_id
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: game_id missing from LEFT JOIN row"))?,
        name: name
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: name missing from LEFT JOIN row"))?,
        bot_name: bot_name
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: bot_name missing from LEFT JOIN row"))?,
    }))
}

/// Builds a `GameTypeUser` from LEFT-JOINed columns, synthesizing a default row
/// when the join produced NULLs (a player who has not been rated in this game
/// type yet).
///
/// **The synthetic row is marked by `id == Uuid::nil()`** and carries
/// `rating = peak_rating = 1200`, matching the `game_type_users.rating` column
/// default, with `last_game_finished_at = None`, `created_at`/`updated_at` set
/// to the caller's `default_ts`, and `user_id = default_user_id` (also
/// `Uuid::nil()` when the caller had no user id, i.e. a bot slot). That is
/// deliberate - new
/// players start at 1200 and the render path wants a value, not an `Option` -
/// but it means callers cannot tell "no rating row yet" from "a real row
/// sitting at 1200" except via the nil id. No caller reads `id` today (the
/// only field consumed off this struct outside db.rs is `rating`, at
/// `game/server_fns.rs:369`); if one ever needs the distinction, change the
/// return type to `Option<GameTypeUser>` rather than adding nil-id checks at
/// call sites (ws F43).
#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_game_type_user(
    id: Option<Uuid>,
    created_at: Option<time::PrimitiveDateTime>,
    updated_at: Option<time::PrimitiveDateTime>,
    game_type_id: Option<Uuid>,
    user_id: Option<Uuid>,
    last_game_finished_at: Option<time::PrimitiveDateTime>,
    rating: Option<i32>,
    peak_rating: Option<i32>,
    default_user_id: Option<Uuid>,
    default_game_type_id: Uuid,
    default_ts: time::PrimitiveDateTime,
) -> crate::models::game::GameTypeUser {
    match (
        id,
        created_at,
        updated_at,
        game_type_id,
        user_id,
        rating,
        peak_rating,
    ) {
        (
            Some(id),
            Some(created_at),
            Some(updated_at),
            Some(game_type_id),
            Some(user_id),
            Some(rating),
            Some(peak_rating),
        ) => crate::models::game::GameTypeUser {
            id,
            created_at,
            updated_at,
            game_type_id,
            user_id,
            last_game_finished_at,
            rating,
            peak_rating,
        },
        _ => crate::models::game::GameTypeUser {
            id: Uuid::nil(),
            created_at: default_ts,
            updated_at: default_ts,
            game_type_id: default_game_type_id,
            user_id: default_user_id.unwrap_or(Uuid::nil()),
            last_game_finished_at: None,
            rating: INITIAL_RATING,
            peak_rating: INITIAL_RATING,
        },
    }
}

#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_game_player_from_row(
    id: Uuid,
    created_at: time::PrimitiveDateTime,
    updated_at: time::PrimitiveDateTime,
    game_id: Uuid,
    user_id: Option<Uuid>,
    position: i32,
    color: String,
    has_accepted: bool,
    is_turn: bool,
    is_turn_at: time::PrimitiveDateTime,
    place: Option<i32>,
    last_turn_at: time::PrimitiveDateTime,
    is_eliminated: bool,
    is_read: bool,
    points: Option<f32>,
    undo_game_state: Option<String>,
    rating_change: Option<i32>,
    ranked_placing: Option<i32>,
    left_at: Option<time::PrimitiveDateTime>,
    departure_reason: Option<String>,
    departure_sequence: Option<i32>,
) -> crate::models::game::GamePlayer {
    crate::models::game::GamePlayer {
        id,
        created_at,
        updated_at,
        game_id,
        user_id,
        position,
        color,
        has_accepted,
        is_turn,
        is_turn_at,
        place,
        last_turn_at,
        is_eliminated,
        is_read,
        points,
        undo_game_state,
        rating_change,
        ranked_placing,
        left_at,
        departure_reason,
        departure_sequence,
    }
}

/// D2 username rules (docs/changes/archive/2026-07-11-35-user-settings-spec/spec.md):
/// `^[a-zA-Z0-9_-]{1,16}$`. Uniqueness is enforced separately by the
/// `users_name_lower_key` index (migration 009). Pure and ungated so the
/// client-side form and server fns share one definition.
pub fn validate_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 16
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Normalizes legacy stored preference names onto the current palette, so
/// prefs saved before the 2026-07 palette change still match. See
/// `theme::slot_from_color_name` for the same mapping applied to stored
/// `game_players.color`/`users.pref_colors` values.
#[cfg(feature = "ssr")]
pub(crate) fn normalize_pref_color(name: &str) -> String {
    if name.eq_ignore_ascii_case("Amber") {
        return "Orange".to_string();
    }
    if name.eq_ignore_ascii_case("BlueGrey") {
        return "Cyan".to_string();
    }
    crate::theme::PLAYER_COLOR_NAMES
        .iter()
        .find(|c| c.eq_ignore_ascii_case(name))
        .map(|c| c.to_string())
        .unwrap_or_else(|| name.to_string())
}

#[cfg(feature = "ssr")]
type LocPref = (usize, Vec<String>);

/// Drops each remaining pref's highest-ranked entry, returning `None` once no
/// pref has anything left (signals the caller to stop looping).
#[cfg(feature = "ssr")]
fn remove_highest_prefs(prefs: &[LocPref]) -> Option<Vec<LocPref>> {
    let mut some_remain = false;
    let new_prefs = prefs
        .iter()
        .map(|(pos, pref)| {
            let new_pref = if pref.is_empty() {
                vec![]
            } else {
                let p = pref[1..].to_owned();
                if !some_remain && !p.is_empty() {
                    some_remain = true;
                }
                p
            };
            (*pos, new_pref)
        })
        .collect::<Vec<LocPref>>();
    if some_remain { Some(new_prefs) } else { None }
}

/// Chooses colors for players based on preferences. Ported from the old
/// `api::db::color::choose` (see `git show ba975b5^:rust/api/src/db/color.rs`),
/// but operating on plain strings against a caller-supplied palette rather
/// than a fixed `Color` enum.
///
/// First tries to assign everyone's highest still-available preference, then
/// everyone's next, and so on, until all players have a color or the palette
/// runs out. When multiple players want the same color at the same rank, the
/// preference order is shuffled up front so the winner is randomly tiebroken.
/// Players with no remaining matching prefs get whatever's left of the
/// palette, in palette order. Legacy pref names ("Amber", "BlueGrey") are
/// normalized onto their current equivalents before matching. If there are
/// more players than the palette holds, players beyond the palette length
/// repeat the same assignment recursively (mirroring the old algorithm), and
/// exhausting the palette entirely falls back to "Pink".
#[cfg(feature = "ssr")]
pub(crate) fn choose_colors(prefs: &[Vec<String>], palette: &[&str]) -> Vec<String> {
    if palette.is_empty() || prefs.is_empty() {
        return prefs.iter().map(|_| "Pink".to_string()).collect();
    }

    use rand::seq::SliceRandom;
    use std::collections::HashMap;

    let sub_len = prefs.len().min(palette.len());
    let (sub_prefs, tail_prefs) = prefs.split_at(sub_len);

    let mut rng = rand::rng();
    let mut remaining: Vec<String> = palette.iter().map(|s| s.to_string()).collect();
    let mut assigned: HashMap<usize, String> = HashMap::new();

    let mut rem_prefs: Vec<LocPref> = sub_prefs
        .iter()
        .enumerate()
        .map(|(pos, pref)| {
            let normalized = pref
                .iter()
                .map(|s| normalize_pref_color(s))
                .filter(|s| palette.contains(&s.as_str()))
                .collect::<Vec<String>>();
            (pos, normalized)
        })
        .collect();
    rem_prefs.shuffle(&mut rng);

    'outer: loop {
        // Iterate by reference: the body mutates only `assigned` and
        // `remaining`, never `rem_prefs`, so the old per-pass clone of the
        // whole vec bought nothing (ws F49).
        for (pos, pref) in &rem_prefs {
            if assigned.contains_key(pos) || pref.is_empty() {
                continue;
            }
            let want_color = &pref[0];
            if let Some(idx) = remaining.iter().position(|c| c == want_color) {
                assigned.insert(*pos, remaining.remove(idx));
            }
            if remaining.is_empty() {
                break 'outer;
            }
        }
        if let Some(new_prefs) = remove_highest_prefs(&rem_prefs) {
            rem_prefs = new_prefs;
        } else {
            break 'outer;
        }
    }

    let mut left = remaining.into_iter();
    let mut res = Vec::with_capacity(sub_prefs.len());
    for pos in 0..sub_prefs.len() {
        res.push(
            assigned
                .remove(&pos)
                .unwrap_or_else(|| left.next().unwrap_or_else(|| "Pink".to_string())),
        );
    }

    if !tail_prefs.is_empty() {
        res.extend(choose_colors(tail_prefs, palette));
    }

    res
}

/// Pure: cap a switch-digest at the first `cap` items (quota protection).
#[cfg(feature = "ssr")]
pub fn cap_digest<T>(mut items: Vec<T>, cap: usize) -> Vec<T> {
    items.truncate(cap);
    items
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::*;

    #[test]
    fn validate_username_accepts_valid_names() {
        for name in ["Sam", "big-scary-walrus", "a", "user_1", "ABCDEFGHIJKLMNOP"] {
            assert!(validate_username(name), "{name} should be valid");
        }
    }

    #[test]
    fn validate_username_rejects_invalid_names() {
        for name in [
            "",
            "seventeen-letters!",
            "with space",
            "émile",
            "toolongtoolongtoo",
            "a.b",
        ] {
            assert!(!validate_username(name), "{name} should be invalid");
        }
    }

    #[test]
    fn petname_output_charset_is_username_safe() {
        // Length can exceed 16 (generate_unique_username retries those away);
        // the charset itself must always pass.
        for _ in 0..20 {
            let name = petname::petname(2, "-").expect("petname generates");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "unexpected char in {name}"
            );
        }
    }

    const PALETTE: [&str; 8] = [
        "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
    ];

    #[test]
    fn choose_colors_honours_preference() {
        let prefs = vec![vec!["Blue".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Blue".to_string()]);
    }

    #[test]
    fn choose_colors_same_rank_conflict_resolves_distinctly() {
        // Both players want Blue as their first pref; only one can have it,
        // the other falls back to a leftover palette color. All distinct.
        let prefs = vec![vec!["Blue".to_string()], vec!["Blue".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result.len(), 2);
        assert_ne!(result[0], result[1]);
        assert!(result.contains(&"Blue".to_string()));
        for c in &result {
            assert!(PALETTE.contains(&c.as_str()));
        }
    }

    #[test]
    fn choose_colors_normalizes_legacy_amber_to_orange() {
        let prefs = vec![vec!["Amber".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Orange".to_string()]);
    }

    #[test]
    fn choose_colors_normalizes_legacy_bluegrey_to_cyan() {
        let prefs = vec![vec!["BlueGrey".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Cyan".to_string()]);
    }

    #[test]
    fn choose_colors_no_prefs_fills_from_palette_order() {
        let prefs = vec![vec![], vec![], vec![]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(
            result,
            vec!["Green".to_string(), "Red".to_string(), "Blue".to_string()]
        );
    }

    #[test]
    fn cap_digest_truncates_to_cap() {
        let items: Vec<i32> = (0..25).collect();
        let capped = cap_digest(items, SWITCH_DIGEST_CAP);
        assert_eq!(capped.len(), SWITCH_DIGEST_CAP);
        assert_eq!(capped[0], 0);
        assert_eq!(
            capped[SWITCH_DIGEST_CAP - 1],
            (SWITCH_DIGEST_CAP - 1) as i32
        );
        let small: Vec<i32> = vec![1, 2, 3];
        assert_eq!(cap_digest(small, SWITCH_DIGEST_CAP).len(), 3);
    }
}
