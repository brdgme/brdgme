use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: Uuid,
    pub name: String,
    pub type_name: String,
    pub opponents: Vec<String>,
    pub is_turn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameViewData {
    pub id: Uuid,
    pub type_name: String,
    pub version_name: String,
    pub html: String,
    pub is_my_turn: bool,
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

#[server(GetActiveGames, "/api")]
pub async fn get_active_games() -> Result<Vec<GameSummary>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let user = get_current_user().await?
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
            
        let games = crate::db::find_active_games_for_user(&user.id, &pool).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
            
        Ok(games.into_iter().map(|ge| {
            let opponents = ge.game_players.iter()
                .filter(|p| p.user.id != user.id)
                .map(|p| p.user.name.clone())
                .collect();
            let is_turn = ge.game_players.iter()
                .find(|p| p.user.id == user.id)
                .map(|p| p.game_player.is_turn)
                .unwrap_or(false);
                
            GameSummary {
                id: ge.game.id,
                name: ge.game_version.name,
                type_name: ge.game_type.name,
                opponents,
                is_turn,
            }
        }).collect())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetGameDetails, "/api")]
pub async fn get_game_details(game_id: Uuid) -> Result<GameViewData, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;
        
        let ge = crate::db::find_game_extended(&pool, game_id).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Game not found"))?;
            
        let player = ge.game_players.iter().find(|p| p.user.id == user.id);
        
        let render_resp = client::render(
            &ge.game_version.uri,
            ge.game.game_state.clone(),
            player.map(|p| p.game_player.position as usize)
        ).await.map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;
        
        // Convert markup to HTML
        let (nodes, _) = brdgme_markup::from_string(&render_resp.render)
            .map_err(|e| ServerFnError::new(format!("Markup error: {}", e)))?;
            
        // Setup markup players for transformation
        let mut markup_players = Vec::new();
        for p in &ge.game_players {
            use std::str::FromStr;
            markup_players.push(brdgme_markup::Player {
                name: p.user.name.clone(),
                color: brdgme_color::Color::from_str(&p.game_player.color.to_lowercase()).unwrap_or(brdgme_color::WHITE),
            });
        }
        
        let html = brdgme_markup::html(&brdgme_markup::transform(&nodes, &markup_players));
        
        Ok(GameViewData {
            id: ge.game.id,
            type_name: ge.game_type.name,
            version_name: ge.game_version.name,
            html,
            is_my_turn: player.map(|p| p.game_player.is_turn).unwrap_or(false),
            players: ge.game_players.iter().map(|p| PlayerViewData {
                name: p.user.name.clone(),
                color: p.game_player.color.clone(),
                rating: p.game_type_user.rating,
                points: 0.0, // TODO: add points to db
                is_turn: p.game_player.is_turn,
            }).collect(),
            command_spec: render_resp.command_spec,
        })
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(SubmitCommand, "/api")]
pub async fn submit_command(game_id: Uuid, command: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use crate::websocket::{GameBroadcaster, WebSocketMessage};
        use brdgme_cmd::api::{Request, Response};
        use brdgme_game::Status;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;
        
        let ge = crate::db::find_game_extended(&pool, game_id).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Game not found"))?;
            
        if ge.game.is_finished {
            return Err(ServerFnError::new("Game is already finished"));
        }
        
        let player = ge.game_players.iter().find(|p| p.user.id == user.id)
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;
            
        let names: Vec<String> = ge.game_players.iter().map(|p| p.user.name.clone()).collect();
        
        let resp = client::request(
            &ge.game_version.uri,
            &Request::Play {
                player: player.game_player.position as usize,
                game: ge.game.game_state.clone(),
                command,
                names,
            }
        ).await.map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;
        
        let (game_response, logs, can_undo, remaining_input, _public_render, _player_renders) = match resp {
            Response::Play { game, logs, can_undo, remaining_input, public_render, player_renders } =>
                (game, logs, can_undo, remaining_input, public_render, player_renders),
            Response::UserError { message } => return Err(ServerFnError::new(message)),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };

        if !remaining_input.trim().is_empty() {
            return Err(ServerFnError::new(format!("Unexpected input: {}", remaining_input)));
        }

        let prev_game_state = ge.game.game_state.clone();
        let (is_finished, whose_turn, eliminated, placings) = match game_response.status {
            Status::Active { whose_turn, eliminated } => (false, whose_turn, eliminated, vec![]),
            Status::Finished { placings, .. } => (true, vec![], vec![], placings),
        };

        crate::db::update_game_command_success(
            &pool,
            game_id,
            player.game_player.id,
            &prev_game_state,
            &game_response.state,
            can_undo,
            is_finished,
            &whose_turn,
            &eliminated,
            &placings,
            &game_response.points,
        ).await.map_err(|e| ServerFnError::new(format!("Failed to update game: {}", e)))?;
        
        crate::db::create_game_logs(&pool, game_id, logs).await
            .map_err(|e| ServerFnError::new(format!("Failed to create game logs: {}", e)))?;
            
        // Broadcast update
        broadcaster.broadcast(WebSocketMessage::GameUpdate {
            game_id,
        });

        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetAvailableGameTypes, "/api")]
pub async fn get_available_game_types() -> Result<Vec<GameTypeInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let _ = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        let game_types = crate::db::find_available_game_types(&pool).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        Ok(game_types.into_iter().map(|(gt, versions)| GameTypeInfo {
            id: gt.id,
            name: gt.name,
            player_counts: gt.player_counts,
            versions: versions.into_iter().map(|gv| GameVersionInfo {
                id: gv.id,
                name: gv.name,
            }).collect(),
        }).collect())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(CreateNewGame, "/api")]
pub async fn create_new_game(game_version_id: Uuid, opponent_emails: Vec<String>) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use crate::websocket::{GameBroadcaster, WebSocketMessage};
        use crate::db::CreateGameOpts;
        use brdgme_cmd::api::{Request, Response};
        use brdgme_game::Status;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        let player_count = 1 + opponent_emails.len();

        let game_version = crate::db::find_game_version(&pool, game_version_id).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Game version not found"))?;

        let resp = client::request(&game_version.uri, &Request::New { players: player_count }).await
            .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

        let (game_info, logs, _public_render, _player_renders) = match resp {
            Response::New { game, logs, public_render, player_renders } => (game, logs, public_render, player_renders),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };

        let (whose_turn, eliminated, placings) = match game_info.status {
            Status::Active { whose_turn, eliminated } => (whose_turn, eliminated, vec![]),
            Status::Finished { placings, .. } => (vec![], vec![], placings),
        };

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
                chat_id: None,
                game_state: &game_info.state,
            },
        ).await.map_err(|e| ServerFnError::new(format!("Failed to create game: {}", e)))?;

        crate::db::create_game_logs(&pool, game.id, logs).await
            .map_err(|e| ServerFnError::new(format!("Failed to create logs: {}", e)))?;

        broadcaster.broadcast(WebSocketMessage::GameUpdate { game_id: game.id });

        Ok(game.id)
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetGameLogs, "/api")]
pub async fn get_game_logs(game_id: Uuid) -> Result<Vec<GameLogEntry>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::PgPool;
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;

        let pool = use_context::<PgPool>()
            .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let user = get_current_user().await?.ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        let ge = crate::db::find_game_extended(&pool, game_id).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Game not found"))?;

        let player = ge.game_players.iter().find(|p| p.user.id == user.id)
            .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

        let last_turn_at = player.game_player.last_turn_at;
        let game_player_id = player.game_player.id;

        let logs = crate::db::get_game_logs(&pool, game_id, game_player_id).await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let markup_players: Vec<brdgme_markup::Player> = ge.game_players.iter().map(|p| {
            use std::str::FromStr;
            brdgme_markup::Player {
                name: p.user.name.clone(),
                color: brdgme_color::Color::from_str(&p.game_player.color.to_lowercase()).unwrap_or(brdgme_color::WHITE),
            }
        }).collect();

        let entries = logs.into_iter().map(|log| {
            let (nodes, _) = brdgme_markup::from_string(&log.body).unwrap_or_else(|_| (vec![], ""));
            let body_html = brdgme_markup::html(&brdgme_markup::transform(&nodes, &markup_players));
            let is_new = last_turn_at.map(|lta| log.logged_at > lta).unwrap_or(false);
            GameLogEntry { body_html, logged_at: log.logged_at, is_new }
        }).collect();

        Ok(entries)
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}
