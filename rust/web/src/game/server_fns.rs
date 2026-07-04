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

#[server(GetActiveGames, "/api")]
pub async fn get_active_games() -> Result<Vec<GameSummary>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let user = get_current_user()
            .await?
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        let games = crate::db::find_active_games_for_user(&user.id, &pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let mut games = games;
        games.sort_by(|a, b| {
            let a_turn = a
                .game_players
                .iter()
                .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id) && p.game_player.is_turn);
            let b_turn = b
                .game_players
                .iter()
                .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id) && p.game_player.is_turn);
            b_turn
                .cmp(&a_turn)
                .then(b.game.updated_at.cmp(&a.game.updated_at))
        });
        let summaries: Vec<GameSummary> = games
            .into_iter()
            .map(|ge| {
                let opponents = ge
                    .game_players
                    .iter()
                    .filter(|p| p.user.as_ref().is_none_or(|u| u.id != user.id))
                    .map(|p| {
                        use std::str::FromStr;
                        let color = brdgme_color::Color::from_str(&p.game_player.color)
                            .unwrap_or(brdgme_color::WHITE)
                            .hex();
                        OpponentSummary {
                            name: p.name().to_string(),
                            color,
                        }
                    })
                    .collect();
                let is_turn = ge
                    .game_players
                    .iter()
                    .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
                    .map(|p| p.game_player.is_turn)
                    .unwrap_or(false);

                GameSummary {
                    id: ge.game.id,
                    name: ge.game_version.name,
                    type_name: ge.game_type.name,
                    opponents,
                    is_turn,
                }
            })
            .collect();
        Ok(summaries)
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetGameDetails, "/api")]
pub async fn get_game_details(game_id: Uuid) -> Result<GameViewData, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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

        // Setup markup players for transformation
        let mut markup_players = Vec::new();
        for p in &ge.game_players {
            use std::str::FromStr;
            markup_players.push(brdgme_markup::Player {
                name: p.name().to_string(),
                color: brdgme_color::Color::from_str(&p.game_player.color)
                    .unwrap_or(brdgme_color::WHITE),
            });
        }

        let html = brdgme_markup::html(&brdgme_markup::transform(&nodes, &markup_players));

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
                .map(|p| {
                    use std::str::FromStr;
                    let color = brdgme_color::Color::from_str(&p.game_player.color)
                        .unwrap_or(brdgme_color::WHITE)
                        .hex();
                    PlayerViewData {
                        name: p.name().to_string(),
                        color,
                        rating: p.game_type_user.rating,
                        points: p.game_player.points.unwrap_or(0.0),
                        is_turn: p.game_player.is_turn,
                        is_bot: p.game_bot.is_some(),
                    }
                })
                .collect(),
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
        use crate::auth::server::get_current_user;
        use crate::websocket::GameBroadcaster;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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
            game_id,
            position as usize,
            command,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetAvailableGameTypes, "/api")]
pub async fn get_available_game_types() -> Result<Vec<GameTypeInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
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
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(CreateNewGame, "/api")]
pub async fn create_new_game(
    game_version_id: Uuid,
    opponent_emails: Option<Vec<String>>,
    bot_slots: Option<Vec<BotSlot>>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use crate::db::CreateGameOpts;
        use crate::game::client;
        use crate::websocket::GameBroadcaster;
        use brdgme_cmd::api::{Request, Response};
        use brdgme_game::Status;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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

        let (game_info, logs, public_render, player_renders) = match resp {
            Response::New {
                game,
                logs,
                public_render,
                player_renders,
            } => (game, logs, public_render, player_renders),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };

        let (whose_turn, eliminated, placings) = match game_info.status {
            Status::Active {
                whose_turn,
                eliminated,
            } => (whose_turn, eliminated, vec![]),
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

        if let Ok(Some(ge)) = crate::db::find_game_extended(&pool, game.id).await {
            let all_logs = crate::db::get_all_game_logs(&pool, game.id)
                .await
                .unwrap_or_default();
            broadcaster
                .broadcast_game_update(&pool, &ge, &all_logs, &public_render, &player_renders)
                .await;
            crate::game::trigger_bot_turns(&http_client, &ge).await;
        }

        Ok(game.id)
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(GetGameLogs, "/api")]
pub async fn get_game_logs(game_id: Uuid) -> Result<Vec<GameLogEntry>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
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

        let markup_players: Vec<brdgme_markup::Player> = ge
            .game_players
            .iter()
            .map(|p| {
                use std::str::FromStr;
                brdgme_markup::Player {
                    name: p.name().to_string(),
                    color: brdgme_color::Color::from_str(&p.game_player.color)
                        .unwrap_or(brdgme_color::WHITE),
                }
            })
            .collect();

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
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(MarkRead, "/api")]
pub async fn mark_read(game_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let user = get_current_user()
            .await?
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        crate::db::mark_game_read(&pool, game_id, user.id)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(UndoGame, "/api")]
pub async fn undo_game(game_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use crate::websocket::GameBroadcaster;
        use brdgme_cmd::api::{Request, Response};
        use brdgme_game::Status;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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

        let (game_response, public_render, player_renders) = match resp {
            Response::Status {
                game,
                public_render,
                player_renders,
            } => (game, public_render, player_renders),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };

        let (whose_turn, eliminated, placings) = match game_response.status {
            Status::Active {
                whose_turn,
                eliminated,
            } => (whose_turn, eliminated, vec![]),
            Status::Finished { placings, .. } => (vec![], vec![], placings),
        };

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

        if let Ok(Some(updated_ge)) = crate::db::find_game_extended(&pool, game_id).await {
            let all_logs = crate::db::get_all_game_logs(&pool, game_id)
                .await
                .unwrap_or_default();
            broadcaster
                .broadcast_game_update(
                    &pool,
                    &updated_ge,
                    &all_logs,
                    &public_render,
                    &player_renders,
                )
                .await;
            crate::game::trigger_bot_turns(&http_client, &updated_ge).await;
        }
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(ConcedeGame, "/api")]
pub async fn concede_game(game_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use crate::game::client;
        use crate::websocket::GameBroadcaster;
        use brdgme_cmd::api::{Request, Response};
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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

        if let Ok(Some(updated_ge)) = crate::db::find_game_extended(&pool, game_id).await {
            let all_logs = crate::db::get_all_game_logs(&pool, game_id)
                .await
                .unwrap_or_default();
            match client::request(
                &http_client,
                &updated_ge.game_version.uri,
                &Request::Status {
                    game: updated_ge.game.game_state.clone(),
                },
            )
            .await
            {
                Ok(Response::Status {
                    public_render,
                    player_renders,
                    ..
                }) => {
                    broadcaster
                        .broadcast_game_update(
                            &pool,
                            &updated_ge,
                            &all_logs,
                            &public_render,
                            &player_renders,
                        )
                        .await;
                    crate::game::trigger_bot_turns(&http_client, &updated_ge).await;
                }
                _ => {
                    tracing::error!(
                        "Unexpected response from game service on concede status call for game {}",
                        game_id
                    );
                }
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(RestartGame, "/api")]
pub async fn restart_game(game_id: Uuid) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use crate::db::CreateGameOpts;
        use crate::game::client;
        use crate::websocket::GameBroadcaster;
        use brdgme_cmd::api::{Request, Response};
        use brdgme_game::Status;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let broadcaster = use_context::<GameBroadcaster>()
            .ok_or_else(|| ServerFnError::new("Broadcaster not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
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

        let player_count = ge.game_players.len();
        let resp = client::request(
            &http_client,
            &ge.game_version.uri,
            &Request::New {
                players: player_count,
            },
        )
        .await
        .map_err(|e| ServerFnError::new(format!("Game service error: {}", e)))?;

        let (game_info, logs, public_render, player_renders) = match resp {
            Response::New {
                game,
                logs,
                public_render,
                player_renders,
            } => (game, logs, public_render, player_renders),
            _ => return Err(ServerFnError::new("Unexpected response from game service")),
        };

        let (whose_turn, eliminated, placings) = match game_info.status {
            Status::Active {
                whose_turn,
                eliminated,
            } => (whose_turn, eliminated, vec![]),
            Status::Finished { placings, .. } => (vec![], vec![], placings),
        };

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
                game_version_id: ge.game.game_version_id,
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
        if let Ok(Some(new_ge)) = crate::db::find_game_extended(&pool, new_game.id).await {
            let all_logs = crate::db::get_all_game_logs(&pool, new_game.id)
                .await
                .unwrap_or_default();
            broadcaster
                .broadcast_game_update(&pool, &new_ge, &all_logs, &public_render, &player_renders)
                .await;
            crate::game::trigger_bot_turns(&http_client, &new_ge).await;
        }

        // Broadcast update for the old game with restarted_game_id now set, so
        // the other player's game view updates to show the "Go to new game" link.
        if let Ok(Some(old_ge)) = crate::db::find_game_extended(&pool, game_id).await
            && let Ok(Response::Status {
                public_render: old_pub,
                player_renders: old_pr,
                ..
            }) = client::request(
                &http_client,
                &old_ge.game_version.uri,
                &Request::Status {
                    game: old_ge.game.game_state.clone(),
                },
            )
            .await
        {
            let old_logs = crate::db::get_all_game_logs(&pool, game_id)
                .await
                .unwrap_or_default();
            broadcaster
                .broadcast_game_update(&pool, &old_ge, &old_logs, &old_pub, &old_pr)
                .await;
        }

        Ok(new_game.id)
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}

#[server(BumpBotTurns, "/api")]
pub async fn bump_bot_turns(game_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::auth::server::get_current_user;
        use leptos::prelude::*;
        use sqlx::PgPool;

        let pool =
            use_context::<PgPool>().ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        let http_client = use_context::<reqwest::Client>()
            .ok_or_else(|| ServerFnError::new("HTTP client not found"))?;
        let user = get_current_user()
            .await?
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Game not found"))?;

        // Only players in the game can bump bots.
        let is_player = ge
            .game_players
            .iter()
            .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id));
        if !is_player {
            return Err(ServerFnError::new("You are not a player in this game"));
        }

        crate::game::trigger_bot_turns(&http_client, &ge).await;
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    unreachable!()
}
