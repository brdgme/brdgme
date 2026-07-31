use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendRecentGame {
    pub friend_user_id: Uuid,
    pub friend_name: String,
    pub game_id: Option<Uuid>,
    pub game_type_name: Option<String>,
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeRating {
    pub game_type_name: String,
    pub rating: Option<i32>,
    pub trend: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameHistoryEntry {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub is_finished: bool,
    pub is_turn: bool,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedInIndexData {
    pub friends: Vec<FriendRecentGame>,
    pub game_types: Vec<GameTypeRating>,
    pub history: Vec<GameHistoryEntry>,
}

#[server(GetLoggedInIndex, "/api")]
pub async fn get_logged_in_index() -> Result<LoggedInIndexData, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = crate::friends::require_user().await?;

    use futures_util::stream::{StreamExt, TryStreamExt};
    let friends = crate::db::list_friends(&pool, user.id)
        .await
        .map_err(internal("get_logged_in_index: friends"))?;
    let viewer_id = user.id;
    let friend_entries: Vec<FriendRecentGame> = futures_util::stream::iter(friends)
        .map(|(friend_id, friend_name)| {
            let pool = pool.clone();
            async move {
                let visible =
                    crate::db::friend_recent_visible_game(&pool, friend_id, viewer_id, 10).await?;
                Ok::<_, anyhow::Error>(FriendRecentGame {
                    friend_user_id: friend_id,
                    friend_name,
                    game_id: visible.as_ref().map(|(id, _, _)| *id),
                    game_type_name: visible.as_ref().map(|(_, name, _)| name.clone()),
                    updated_at: visible.as_ref().map(|(_, _, ts)| *ts),
                })
            }
        })
        .buffered(10)
        .try_collect::<Vec<_>>()
        .await
        .map_err(internal("get_logged_in_index: friend recent game"))?;

    let stats = crate::stats::game_type_stats(&pool, user.id, false, None)
        .await
        .map_err(internal("get_logged_in_index: game_type_stats"))?;
    let form = crate::stats::recent_form(&pool, user.id, 10, false)
        .await
        .map_err(internal("get_logged_in_index: recent_form"))?;
    let game_types = stats
        .iter()
        .map(|s| {
            let results = form
                .iter()
                .find(|f| f.game_type_name == s.game_type_name)
                .map(|f| f.results.as_slice())
                .unwrap_or(&[]);
            GameTypeRating {
                game_type_name: s.game_type_name.clone(),
                rating: s.rating,
                trend: crate::players::rating_trend(s.rating, results),
            }
        })
        .collect();

    let rows = crate::db::recent_games_for_index(&pool, user.id, 10)
        .await
        .map_err(internal("get_logged_in_index: history"))?;
    let history = rows
        .into_iter()
        .map(
            |(game_id, game_type_name, is_finished, is_turn, updated_at)| GameHistoryEntry {
                game_id,
                game_type_name,
                is_finished,
                is_turn,
                updated_at,
            },
        )
        .collect();

    Ok(LoggedInIndexData {
        friends: friend_entries,
        game_types,
        history,
    })
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::PgPool;

    /// F-156 / wd F74: the logged-in index "friends' recent games" feed must
    /// not drop a friend who sorts alphabetically after the 20th position when
    /// that friend has a recent visible game. Pre-fix this fails: `list_friends`
    /// orders by `lower(u.name)` and `.take(20)` truncates on that name axis, so
    /// `friend_21` (alphabetically last, holder of the most recent game) is
    /// permanently invisible. The assertion is presence-only so it holds for
    /// either fix shape (drop the bound, or order by recency before bounding).
    #[sqlx::test]
    async fn get_logged_in_index_includes_late_alphabetical_friend_with_recent_game(pool: PgPool) {
        let (_, gv) = make_game_type_and_version(&pool).await;
        let co = make_user(&pool, "coplayer").await;

        let mut friend_ids: Vec<Uuid> = Vec::new();
        for i in 1..=21 {
            let f = make_user(&pool, &format!("friend_{i:02}")).await;
            friend_ids.push(f.id);
        }
        let last_id = friend_ids[20];
        // The alphabetically-last friend has the most recent, publicly visible
        // game (both seats default to 'public'); every other friend has none.
        let game = make_game_with_players(&pool, gv, last_id, &[co.id], 0, &[0]).await;

        let setup = pool.clone();
        let data = crate::test_support::non_admin(&pool, move || async move {
            let viewer = crate::friends::require_user().await.expect("authenticated");
            for fid in &friend_ids {
                accept_friends(&setup, viewer.id, *fid).await;
            }
            get_logged_in_index().await
        })
        .await;

        let data = data.expect("index ok");
        let entry = data
            .friends
            .iter()
            .find(|f| f.friend_user_id == last_id)
            .expect(
                "friend_21 (alphabetically after position 20) with a recent visible game \
                 was truncated from the feed",
            );
        assert_eq!(
            entry.game_id,
            Some(game.id),
            "the late friend's recent visible game must be carried in the feed"
        );
    }
}
