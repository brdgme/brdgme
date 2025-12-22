use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        
        let (game_response, logs, _can_undo, remaining_input, _public_render, _player_renders) = match resp {
            Response::Play { game, logs, can_undo, remaining_input, public_render, player_renders } => 
                (game, logs, can_undo, remaining_input, public_render, player_renders),
            Response::UserError { message } => return Err(ServerFnError::new(message)),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };
        
        if !remaining_input.trim().is_empty() {
            return Err(ServerFnError::new(format!("Unexpected input: {}", remaining_input)));
        }
        
        let (is_finished, whose_turn, _eliminated, placings) = match game_response.status {
            Status::Active { whose_turn, eliminated } => (false, whose_turn, eliminated, vec![]),
            Status::Finished { placings, .. } => (true, vec![], vec![], placings),
        };
        
        crate::db::update_game_command_success(
            &pool,
            game_id,
            player.game_player.id,
            &game_response.state,
            is_finished,
            &whose_turn,
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
