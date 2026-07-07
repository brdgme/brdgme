use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSlot {
    pub name: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentSummary {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: Uuid,
    pub name: String,
    pub type_name: String,
    pub opponents: Vec<OpponentSummary>,
    pub is_turn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameViewData {
    pub id: Uuid,
    pub type_name: String,
    pub version_name: String,
    pub html: String,
    pub is_my_turn: bool,
    pub is_finished: bool,
    pub can_undo: bool,
    pub restarted_game_id: Option<Uuid>,
    pub is_2player: bool,
    pub players: Vec<PlayerViewData>,
    pub command_spec: Option<brdgme_game::command::Spec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerViewData {
    pub name: String,
    pub color: String,
    pub rating: i32,
    pub points: f32,
    pub is_turn: bool,
    pub is_bot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersionInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTypeInfo {
    pub id: Uuid,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub versions: Vec<GameVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLogEntry {
    pub body_html: String,
    pub logged_at: PrimitiveDateTime,
    pub is_new: bool,
}

/// Builds the active-game summaries for `user`, or an empty list if there is
/// no logged-in user - anonymous visitors hit pages that render
/// `SidebarMenu` (e.g. the homepage), and "not logged in" is a normal state
/// there, not an error.
#[cfg(feature = "ssr")]
async fn active_games_summary(
    user: Option<crate::auth::AuthUser>,
    pool: &sqlx::PgPool,
) -> Result<Vec<GameSummary>, ServerFnError> {
    let Some(user) = user else {
        return Ok(Vec::new());
    };

    crate::db::find_active_game_summaries(pool, user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))
}

#[server(GetActiveGames, "/api")]
pub async fn get_active_games() -> Result<Vec<GameSummary>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user().await?;

    active_games_summary(user, &pool).await
}

#[server(GetGameDetails, "/api")]
pub async fn get_game_details(game_id: Uuid) -> Result<GameViewData, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::game::client;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id));

    let render_resp = client::render(
        &http_client,
        &ge.game_version.uri,
        ge.game.game_state.clone(),
        player.map(|p| p.game_player.position as usize),
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

    // Convert markup to HTML
    let (nodes, _) = brdgme_markup::from_string(&render_resp.render)
        .map_err(|e| ServerFnError::new(format!("Markup error: {}", e)))?;

    let html = brdgme_markup::html(&brdgme_markup::transform(&nodes, &ge.markup_players()));

    Ok(GameViewData {
        id: ge.game.id,
        type_name: ge.game_type.name,
        version_name: ge.game_version.name,
        html,
        is_my_turn: player.map(|p| p.game_player.is_turn).unwrap_or(false),
        is_finished: ge.game.is_finished,
        can_undo: player
            .and_then(|p| p.game_player.undo_game_state.as_ref())
            .is_some(),
        restarted_game_id: ge.game.restarted_game_id,
        is_2player: ge.game_players.len() == 2,
        players: ge
            .game_players
            .iter()
            .map(|p| PlayerViewData {
                name: p.name().to_string(),
                color: p.color().hex(),
                rating: p.game_type_user.rating,
                points: p.game_player.points.unwrap_or(0.0),
                is_turn: p.game_player.is_turn,
                is_bot: p.game_bot.is_some(),
            })
            .collect(),
        command_spec: render_resp.command_spec,
    })
}

#[server(SubmitCommand, "/api")]
pub async fn submit_command(game_id: Uuid, command: String) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let position: i32 = sqlx::query_scalar!(
        "SELECT position FROM game_players WHERE game_id = $1 AND user_id = $2",
        game_id,
        user.id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    super::execute_command(
        &pool,
        &http_client,
        &broadcaster,
        &jetstream,
        game_id,
        position as usize,
        command,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetAvailableGameTypes, "/api")]
pub async fn get_available_game_types() -> Result<Vec<GameTypeInfo>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let _ = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let game_types = crate::db::find_available_game_types(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(game_types
        .into_iter()
        .map(|(gt, versions)| GameTypeInfo {
            id: gt.id,
            name: gt.name,
            player_counts: gt.player_counts,
            versions: versions
                .into_iter()
                .map(|gv| GameVersionInfo {
                    id: gv.id,
                    name: gv.name,
                })
                .collect(),
        })
        .collect())
}

#[server(CreateNewGame, "/api")]
pub async fn create_new_game(
    game_version_id: Uuid,
    opponent_emails: Option<Vec<String>>,
    bot_slots: Option<Vec<BotSlot>>,
) -> Result<Uuid, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::db::CreateGameOpts;
    use crate::game::client;
    use crate::websocket::GameBroadcaster;
    use brdgme_cmd::api::{Request, Response};
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let opponent_emails = opponent_emails.unwrap_or_default();
    let bot_slots = bot_slots.unwrap_or_default();
    let player_count = 1 + opponent_emails.len() + bot_slots.len();

    let game_version = crate::db::find_game_version(&pool, game_version_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let resp = client::request(
        &http_client,
        &game_version.uri,
        &Request::New {
            players: player_count,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

    let (game_info, logs) = match resp {
        Response::New { game, logs, .. } => (game, logs),
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let (_, whose_turn, eliminated, placings) = crate::game::status_fields(game_info.status);

    let game = crate::db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id,
            whose_turn: &whose_turn,
            eliminated: &eliminated,
            placings: &placings,
            points: &game_info.points,
            creator_id: user.id,
            opponent_ids: &[],
            opponent_emails: &opponent_emails,
            bot_slots: &bot_slots,
            chat_id: None,
            game_state: &game_info.state,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to create game: {}", e)))?;

    crate::db::create_game_logs(&pool, game.id, logs)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create logs: {}", e)))?;

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game.id).await;

    Ok(game.id)
}

#[server(GetGameLogs, "/api")]
pub async fn get_game_logs(game_id: Uuid) -> Result<Vec<GameLogEntry>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    let last_turn_at = player.game_player.last_turn_at;
    let game_player_id = player.game_player.id;

    let logs = crate::db::get_game_logs(&pool, game_id, game_player_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let markup_players = ge.markup_players();

    let entries = logs
        .into_iter()
        .map(|log| {
            let (nodes, _) =
                brdgme_markup::from_string(&log.body).unwrap_or_else(|_| (vec![], ""));
            let body_html =
                brdgme_markup::html(&brdgme_markup::transform(&nodes, &markup_players));
            let is_new = log.created_at >= last_turn_at;
            GameLogEntry {
                body_html,
                logged_at: log.logged_at,
                is_new,
            }
        })
        .collect();

    Ok(entries)
}

#[server(MarkRead, "/api")]
pub async fn mark_read(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::mark_game_read(&pool, game_id, user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))
}

#[server(UndoGame, "/api")]
pub async fn undo_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::game::client;
    use crate::websocket::GameBroadcaster;
    use brdgme_cmd::api::{Request, Response};
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    let undo_state = player
        .game_player
        .undo_game_state
        .clone()
        .ok_or_else(|| ServerFnError::new("No undo state available"))?;

    let resp = client::request(
        &http_client,
        &ge.game_version.uri,
        &Request::Status {
            game: undo_state.clone(),
        },
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

    let game_response = match resp {
        Response::Status { game, .. } => game,
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let (_, whose_turn, eliminated, placings) = crate::game::status_fields(game_response.status);

    crate::db::undo_game(
        &pool,
        game_id,
        &undo_state,
        player.game_player.position as usize,
        &whose_turn,
        &eliminated,
        &placings,
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to undo game: {}", e)))?;

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;
    Ok(())
}

#[server(ConcedeGame, "/api")]
pub async fn concede_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }
    if ge.game_players.len() != 2 {
        return Err(ServerFnError::new(
            "Concede is only available in 2-player games",
        ));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    crate::db::concede_game(&pool, game_id, player.game_player.id, player.name())
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to concede game: {}", e)))?;

    broadcaster.broadcast_game_update(game_id).await;
    Ok(())
}

#[server(RestartGame, "/api")]
pub async fn restart_game(game_id: Uuid) -> Result<Uuid, ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::db::CreateGameOpts;
    use crate::game::client;
    use crate::websocket::GameBroadcaster;
    use brdgme_cmd::api::{Request, Response};
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;

    if !ge.game.is_finished {
        return Err(ServerFnError::new("Game is not finished"));
    }
    if ge.game.restarted_game_id.is_some() {
        return Err(ServerFnError::new("Game has already been restarted"));
    }
    if !ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
    {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    // If a newer, non-deprecated version of this game type exists, restart
    // onto that version rather than the (possibly deprecated) version the
    // finished game was played on. Falls back to the original version if
    // none is found.
    let restart_game_version =
        crate::db::find_latest_non_deprecated_game_version(&pool, ge.game_version.game_type_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .unwrap_or_else(|| ge.game_version.clone());

    let player_count = ge.game_players.len();
    let resp = client::request(
        &http_client,
        &restart_game_version.uri,
        &Request::New {
            players: player_count,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

    let (game_info, logs) = match resp {
        Response::New { game, logs, .. } => (game, logs),
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let (_, whose_turn, eliminated, placings) = crate::game::status_fields(game_info.status);

    let opponent_ids: Vec<Uuid> = ge
        .game_players
        .iter()
        .filter_map(|p| p.user.as_ref().filter(|u| u.id != user.id).map(|u| u.id))
        .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let new_game = crate::db::create_game_with_users_tx(
        &pool,
        &mut tx,
        CreateGameOpts {
            game_version_id: restart_game_version.id,
            whose_turn: &whose_turn,
            eliminated: &eliminated,
            placings: &placings,
            points: &game_info.points,
            creator_id: user.id,
            opponent_ids: &opponent_ids,
            opponent_emails: &[],
            bot_slots: &[],
            chat_id: None,
            game_state: &game_info.state,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to create game: {}", e)))?;

    crate::db::insert_game_logs_tx(&mut tx, new_game.id, logs)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create game logs: {}", e)))?;

    sqlx::query!(
        "UPDATE games SET restarted_game_id = $1, updated_at = NOW() WHERE id = $2",
        new_game.id,
        game_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    // Broadcast update for the new game.
    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, new_game.id).await;

    // Broadcast update for the old game with restarted_game_id now set, so
    // the other player's game view updates to show the "Go to new game" link.
    broadcaster.broadcast_game_update(game_id).await;

    Ok(new_game.id)
}

#[server(BumpBotTurns, "/api")]
pub async fn bump_bot_turns(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // Only players in the game can bump bots.
    let is_player = crate::db::is_player_in_game(&pool, game_id, user.id)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    if !is_player {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    crate::game::trigger_bot_turns(&pool, &jetstream, game_id).await;
    Ok(())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn make_user(pool: &PgPool, name: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            id,
            name,
            &Vec::<String>::new()
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn make_game_version(pool: &PgPool) -> Uuid {
        let game_type_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO game_types (id, name, player_counts) VALUES ($1, $2, $3)",
            game_type_id,
            "Test Game",
            &vec![2i32]
        )
        .execute(pool)
        .await
        .unwrap();
        let game_version_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO game_versions (id, game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, $2, $3, $4, true, false)",
            game_version_id,
            game_type_id,
            "v1",
            "http://127.0.0.1:8100"
        )
        .execute(pool)
        .await
        .unwrap();
        game_version_id
    }

    // Anonymous visitors hit pages that render SidebarMenu (e.g. the
    // homepage) before logging in; that must not surface as a 500.
    #[sqlx::test]
    async fn active_games_summary_returns_empty_for_anonymous_user(pool: PgPool) {
        let summaries = active_games_summary(None, &pool).await.unwrap();
        assert!(summaries.is_empty());
    }

    // Regression test for a hard-load of a bot game's page: the LEFT JOINed
    // bot player (NULL user_id) must not trip the summary query/mapping.
    #[sqlx::test]
    async fn active_games_summary_includes_bot_opponent(pool: PgPool) {
        let user_id = make_user(&pool, "human").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    difficulty: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "state",
            },
        )
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "human".to_string(),
            email: "human@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, game.id);
        assert_eq!(summaries[0].opponents.len(), 1);
        assert_eq!(summaries[0].opponents[0].name, "Botty");
    }

    // Pins the sidebar sort order: my-turn games first, then most recently
    // updated. Single-player games so whose_turn position 0 is always the
    // creator (player order is shuffled in multi-slot games).
    #[sqlx::test]
    async fn active_games_summary_sorts_my_turn_first_then_updated_at_desc(pool: PgPool) {
        let user_id = make_user(&pool, "human").await;
        let game_version_id = make_game_version(&pool).await;

        let make_game = |whose_turn: &'static [usize]| {
            crate::db::create_game_with_users(
                &pool,
                crate::db::CreateGameOpts {
                    game_version_id,
                    whose_turn,
                    eliminated: &[],
                    placings: &[],
                    points: &[],
                    creator_id: user_id,
                    opponent_ids: &[],
                    opponent_emails: &[],
                    bot_slots: &[],
                    chat_id: None,
                    game_state: "state",
                },
            )
        };

        // (a) not their turn, updated recently
        let game_a = make_game(&[]).await.unwrap();
        // (b) their turn, updated long ago
        let game_b = make_game(&[0]).await.unwrap();
        // (c) their turn, updated recently (creation timestamp left as-is)
        let game_c = make_game(&[0]).await.unwrap();

        // The update_games_updated_at trigger overwrites updated_at on every
        // UPDATE; disable it so the backdated values stick.
        sqlx::raw_sql("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE games SET updated_at = timezone('utc', now()) - interval '1 hour' WHERE id = $1",
            game_a.id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE games SET updated_at = timezone('utc', now()) - interval '10 days' WHERE id = $1",
            game_b.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "human".to_string(),
            email: "human@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        let ids: Vec<Uuid> = summaries.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![game_c.id, game_b.id, game_a.id]);
        assert!(summaries[0].is_turn);
        assert!(summaries[1].is_turn);
        assert!(!summaries[2].is_turn);
    }

    // The requesting user must never be listed among their own opponents;
    // every other human and bot must be, with the bot named from
    // game_bots.name.
    #[sqlx::test]
    async fn active_games_summary_excludes_self_from_opponents(pool: PgPool) {
        let user_id = make_user(&pool, "alice").await;
        let opponent_id = make_user(&pool, "bob").await;
        let game_version_id = make_game_version(&pool).await;
        crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[opponent_id],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    difficulty: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "state",
            },
        )
        .await
        .unwrap();

        let user = crate::auth::AuthUser {
            id: user_id,
            name: "alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        let summaries = active_games_summary(Some(user), &pool).await.unwrap();

        assert_eq!(summaries.len(), 1);
        let mut opponent_names: Vec<&str> = summaries[0]
            .opponents
            .iter()
            .map(|o| o.name.as_str())
            .collect();
        opponent_names.sort();
        assert_eq!(opponent_names, vec!["Botty", "bob"]);
    }
}
