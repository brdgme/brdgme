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

    let friends = crate::db::list_friends(&pool, user.id)
        .await
        .map_err(internal("get_logged_in_index: friends"))?;
    let mut friend_entries = Vec::new();
    for (friend_id, friend_name) in friends {
        let visible = crate::db::friend_recent_visible_game(&pool, friend_id, user.id, 10)
            .await
            .map_err(internal("get_logged_in_index: friend recent game"))?;
        friend_entries.push(FriendRecentGame {
            friend_user_id: friend_id,
            friend_name,
            game_id: visible.as_ref().map(|(id, _, _)| *id),
            game_type_name: visible.as_ref().map(|(_, name, _)| name.clone()),
            updated_at: visible.as_ref().map(|(_, _, ts)| *ts),
        });
    }

    let stats = crate::stats::game_type_stats(&pool, user.id, false)
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
