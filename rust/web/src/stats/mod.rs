//! #29 player stats: DTOs, queries and server fns for /players pages.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileUser {
    pub user_id: Uuid,
    pub name: String,
    pub pref_color: Option<String>,
    pub pref_colors: Vec<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverallTotals {
    pub finished_games: i64,
    pub wins: i64,
    pub win_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeStats {
    pub game_type_name: String,
    pub games: i64,
    pub wins: i64,
    pub win_percent: f64,
    pub avg_place_percentile: Option<f64>,
    pub rating: Option<i32>,
    pub peak_rating: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingPoint {
    pub finished_at: PrimitiveDateTime,
    pub rating: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opponent {
    pub user_id: Option<Uuid>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpponentWithPlace {
    pub user_id: Option<Uuid>,
    pub name: String,
    pub place: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchElo {
    pub min: i32,
    pub max: i32,
    pub avg: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRow {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub is_finished: bool,
    pub started_at: PrimitiveDateTime,
    pub finished_at: Option<PrimitiveDateTime>,
    pub my_place: Option<i32>,
    pub player_count: i64,
    pub my_rating_change: Option<i32>,
    pub opponents: Vec<OpponentWithPlace>,
    pub match_elo: Option<MatchElo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryFilters {
    pub status: Option<bool>,
    pub game_type: Option<String>,
    pub include_single_human: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerHistoryData {
    pub user: ProfileUser,
    pub rows: Vec<HistoryRow>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub filters: HistoryFilters,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinishedGameRow {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub finished_at: Option<PrimitiveDateTime>,
    pub place: Option<i32>,
    pub player_count: i64,
    pub rating_change: Option<i32>,
    pub opponents: Vec<Opponent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveGameRow {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub is_turn: bool,
    pub opponents: Vec<Opponent>,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadToHead {
    pub user_id: Option<Uuid>,
    pub name: String,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub ties: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormResult {
    pub game_id: Uuid,
    pub finished_at: Option<PrimitiveDateTime>,
    pub place: Option<i32>,
    pub player_count: i64,
    pub rating_change: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeForm {
    pub game_type_name: String,
    pub results: Vec<FormResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerProfileData {
    pub user: ProfileUser,
    pub totals: OverallTotals,
    pub game_types: Vec<GameTypeStats>,
    pub recent_form: Vec<GameTypeForm>,
    pub recent_finished: Vec<FinishedGameRow>,
    pub active_games: Vec<ActiveGameRow>,
    /// None when the viewer is anonymous (profiles are public).
    pub viewer_user_id: Option<Uuid>,
    /// False when already friends or viewer has an outgoing request.
    pub can_add_friend: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerGameTypeData {
    pub user: ProfileUser,
    /// Canonical game type name (URL segment is matched case-insensitively).
    pub game_type_name: String,
    /// Aggregate row for this game type (games/wins/win_percent/avg_place_percentile/rating/peak_rating).
    pub stats: GameTypeStats,
    pub rating_series: Vec<RatingPoint>,
    pub finished_games: Vec<FinishedGameRow>,
    pub head_to_head: Vec<HeadToHead>,
}

#[cfg(feature = "ssr")]
mod queries;

#[cfg(feature = "ssr")]
pub use queries::*;

pub mod viz;

#[server(GetPlayerProfile, "/api")]
pub async fn get_player_profile(
    name: String,
    include_single_human: bool,
) -> Result<Option<PlayerProfileData>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let viewer_user_id = get_current_user().await?.map(|u| u.id);

    let user = match get_profile_user(&pool, &name)
        .await
        .map_err(internal("get_player_profile: find user"))?
    {
        Some(user) => user,
        None => return Ok(None),
    };

    let totals = overall_totals(&pool, user.user_id, include_single_human)
        .await
        .map_err(internal("get_player_profile: totals"))?;
    let game_types = game_type_stats(&pool, user.user_id, include_single_human, None)
        .await
        .map_err(internal("get_player_profile: game_types"))?;
    let recent_form = recent_form(&pool, user.user_id, 10, include_single_human)
        .await
        .map_err(internal("get_player_profile: recent_form"))?;
    let recent_finished = finished_games(
        &pool,
        user.user_id,
        None,
        include_single_human,
        Some(20),
        viewer_user_id,
    )
    .await
    .map_err(internal("get_player_profile: recent_finished"))?;

    let active_games = active_games(&pool, user.user_id, viewer_user_id)
        .await
        .map_err(internal("get_player_profile: active_games"))?;

    let can_add_friend = match viewer_user_id {
        Some(vid) if vid != user.user_id => {
            !crate::db::should_hide_add_friend(&pool, vid, user.user_id)
                .await
                .map_err(internal("get_player_profile: friend status"))?
        }
        _ => false,
    };

    Ok(Some(PlayerProfileData {
        user,
        totals,
        game_types,
        recent_form,
        recent_finished,
        active_games,
        viewer_user_id,
        can_add_friend,
    }))
}

#[server(GetPlayerGameTypeStats, "/api")]
pub async fn get_player_game_type_stats(
    name: String,
    game_type: String,
    include_single_human: bool,
) -> Result<Option<PlayerGameTypeData>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let viewer_user_id = get_current_user().await?.map(|u| u.id);

    let user = match get_profile_user(&pool, &name)
        .await
        .map_err(internal("get_player_game_type_stats: find user"))?
    {
        Some(user) => user,
        None => return Ok(None),
    };

    let canonical = match find_game_type_name(&pool, &game_type)
        .await
        .map_err(internal("get_player_game_type_stats: find game type"))?
    {
        Some(name) => name,
        None => return Ok(None),
    };

    let stats = game_type_stats(&pool, user.user_id, include_single_human, Some(&canonical))
        .await
        .map_err(internal("get_player_game_type_stats: stats"))?
        .into_iter()
        .find(|s| s.game_type_name == canonical)
        .unwrap_or_else(|| GameTypeStats {
            game_type_name: canonical.clone(),
            games: 0,
            wins: 0,
            win_percent: 0.0,
            avg_place_percentile: None,
            rating: None,
            peak_rating: None,
        });

    let mut rating_series = rating_series(&pool, user.user_id, &canonical)
        .await
        .map_err(internal("get_player_game_type_stats: rating_series"))?;
    let finished_games = finished_games(
        &pool,
        user.user_id,
        Some(&canonical),
        include_single_human,
        Some(100),
        viewer_user_id,
    )
    .await
    .map_err(internal("get_player_game_type_stats: finished_games"))?;
    let mut head_to_head = head_to_head(
        &pool,
        user.user_id,
        &canonical,
        include_single_human,
        viewer_user_id,
    )
    .await
    .map_err(internal("get_player_game_type_stats: head_to_head"))?;
    head_to_head.truncate(50);

    let rating_series = if rating_series.len() > 200 {
        rating_series.split_off(rating_series.len() - 200)
    } else {
        rating_series
    };

    Ok(Some(PlayerGameTypeData {
        user,
        game_type_name: canonical,
        stats,
        rating_series,
        finished_games,
        head_to_head,
    }))
}

#[server(GetPlayerHistory, "/api")]
pub async fn get_player_history(
    name: String,
    page: i64,
    status: Option<bool>,
    game_type: Option<String>,
    include_single_human: bool,
) -> Result<Option<PlayerHistoryData>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let viewer_user_id = get_current_user().await?.map(|u| u.id);

    let user = match get_profile_user(&pool, &name)
        .await
        .map_err(internal("get_player_history: find user"))?
    {
        Some(user) => user,
        None => return Ok(None),
    };

    let game_type = match game_type {
        Some(ref gt) if !gt.is_empty() => match find_game_type_name(&pool, gt)
            .await
            .map_err(internal("get_player_history: find game type"))?
        {
            Some(name) => Some(name),
            None => return Ok(None),
        },
        _ => None,
    };

    let page_size: i64 = 50;
    let page = page.clamp(1, 1_000_000);
    let offset = (page - 1) * page_size;

    let total = game_history_count(
        &pool,
        user.user_id,
        status,
        game_type.as_deref(),
        include_single_human,
    )
    .await
    .map_err(internal("get_player_history: count"))?;
    let rows = game_history(
        &pool,
        user.user_id,
        status,
        game_type.as_deref(),
        include_single_human,
        page_size,
        offset,
        viewer_user_id,
    )
    .await
    .map_err(internal("get_player_history: rows"))?;

    Ok(Some(PlayerHistoryData {
        user,
        rows,
        page,
        page_size,
        total,
        filters: HistoryFilters {
            status,
            game_type,
            include_single_human,
        },
    }))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::queries::fixtures;
    use super::*;
    use sqlx::PgPool;
    use time::macros::datetime;

    /// F-154 / wd F52: an unknown, nonempty `game_type` must resolve to
    /// nothing - `Ok(None)`, matching the sibling `get_player_game_type_stats`
    /// (which 404s on an unresolvable type) - not the player's entire history.
    /// Pre-fix this fails: `find_game_type_name`'s `None` binds straight into
    /// the `($3 IS NULL OR gt.name = $3)` predicate as "no filter", so the full
    /// history comes back wrapped in `Ok(Some(_))`.
    #[sqlx::test]
    async fn get_player_history_unknown_game_type_returns_none(pool: PgPool) {
        let user = fixtures::make_user(&pool, "targetplayer").await;
        let opponent = fixtures::make_user(&pool, "opponent").await;
        let (_gt, gv) = fixtures::make_game_type(&pool, "Camel Up").await;
        fixtures::insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        // Sanity: the seeded history is genuinely non-empty, so the bug would
        // surface as a full-history `Ok(Some(_))` rather than an accidental
        // pass on empty data.
        let unfiltered = crate::test_support::anonymous(&pool, || async {
            get_player_history("targetplayer".to_string(), 1, None, None, false).await
        })
        .await;
        let unfiltered = unfiltered.expect("unfiltered ok").expect("user exists");
        assert_eq!(unfiltered.total, 1);
        assert!(!unfiltered.rows.is_empty());

        let result = crate::test_support::anonymous(&pool, || async {
            get_player_history(
                "targetplayer".to_string(),
                1,
                None,
                Some("NoSuchGameType".to_string()),
                false,
            )
            .await
        })
        .await;
        assert!(
            result.expect("query ok").is_none(),
            "unknown game type must return Ok(None), matching get_player_game_type_stats"
        );
    }

    /// wd F46: the server clamps `page` to `1..=1_000_000`. Regression coverage
    /// for underflow (0 and negative) and overflow page values; the clamp is
    /// already in place, so this passes today and guards it.
    #[sqlx::test]
    async fn get_player_history_clamps_page_bounds(pool: PgPool) {
        fixtures::make_user(&pool, "targetplayer").await;

        let underflow = crate::test_support::anonymous(&pool, || async {
            get_player_history("targetplayer".to_string(), 0, None, None, false).await
        })
        .await;
        assert_eq!(
            underflow.expect("ok").expect("user").page,
            1,
            "page 0 clamps up to 1"
        );

        let negative = crate::test_support::anonymous(&pool, || async {
            get_player_history("targetplayer".to_string(), -5, None, None, false).await
        })
        .await;
        assert_eq!(
            negative.expect("ok").expect("user").page,
            1,
            "negative page clamps up to 1"
        );

        let overflow = crate::test_support::anonymous(&pool, || async {
            get_player_history("targetplayer".to_string(), 1_000_001, None, None, false).await
        })
        .await;
        assert_eq!(
            overflow.expect("ok").expect("user").page,
            1_000_000,
            "page above the ceiling clamps down to 1_000_000"
        );
    }

    /// wd F48: exercise the `get_player_game_type_stats` ENTRY POINT, not just
    /// the `game_type_stats` query the F-151 regression test already covers. The
    /// user holds rating rows for two alphabetically-ordered types ("Acquire" <
    /// "Zebra Game"); requesting the later one must return that type's own
    /// aggregate and rating. Driving the real server fn keeps the caller's
    /// defence-in-depth `.find(|s| s.game_type_name == canonical)` (mod.rs:270)
    /// covered: with the F-151 SQL fix the query yields a single row and the
    /// `.find` selects it by canonical name rather than blindly taking the first
    /// row, so the returned `stats` carry the requested type's rating.
    #[sqlx::test]
    async fn get_player_game_type_stats_returns_requested_type_and_rating(pool: PgPool) {
        let user = fixtures::make_user(&pool, "targetplayer").await;
        let opponent = fixtures::make_user(&pool, "opponent").await;

        let (gt_acquire, _gv_acquire) = fixtures::make_game_type(&pool, "Acquire").await;
        let (gt_zebra, gv_zebra) = fixtures::make_game_type(&pool, "Zebra Game").await;

        fixtures::insert_finished_game(
            &pool,
            gv_zebra,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        fixtures::set_game_type_rating(&pool, gt_acquire, user, 1300, 1350).await;
        fixtures::set_game_type_rating(&pool, gt_zebra, user, 1400, 1450).await;

        let data = crate::test_support::anonymous(&pool, || async {
            get_player_game_type_stats("targetplayer".to_string(), "Zebra Game".to_string(), false)
                .await
        })
        .await
        .expect("query ok")
        .expect("user and game type resolve");

        assert_eq!(data.game_type_name, "Zebra Game");
        assert_eq!(
            data.stats.game_type_name, "Zebra Game",
            "the entry point must select the requested type, not the alphabetical first"
        );
        assert_eq!(
            data.stats.rating,
            Some(1400),
            "rating must be Zebra Game's, not Acquire's 1300"
        );
        assert_eq!(data.stats.peak_rating, Some(1450));
        assert_eq!(data.stats.games, 1);
        assert_eq!(data.stats.wins, 1);
    }
}
