use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use sqlx::PgPool;
use tower_sessions::Session;
use crate::auth::session::get_user_from_session;
use crate::db::{self, CreateGameOpts};
use crate::game::client;
use crate::websocket::GameBroadcaster;
use brdgme_cmd::api::{Request, Response};
use brdgme_game::Status;

#[derive(Deserialize)]
pub struct CreateGameRequest {
    pub game_version_id: Uuid,
    pub opponent_ids: Option<Vec<Uuid>>,
    pub opponent_emails: Option<Vec<String>>,
    pub bot_slots: Option<Vec<db::BotSlot>>,
}

pub async fn create_game(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Json(payload): Json<CreateGameRequest>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let opponent_ids = payload.opponent_ids.unwrap_or_default();
    let opponent_emails = payload.opponent_emails.unwrap_or_default();
    let bot_slots = payload.bot_slots.unwrap_or_default();
    let player_count = 1 + opponent_ids.len() + opponent_emails.len() + bot_slots.len();

    let game_version = match db::find_game_version(&pool, payload.game_version_id).await {
        Ok(Some(gv)) => gv,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game version not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let resp = match client::request(&http_client, &game_version.uri, &Request::New { players: player_count }).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Game service error: {}", e)).into_response(),
    };

    let (game_info, logs, public_render, player_renders) = match resp {
        Response::New { game, logs, public_render, player_renders } => (game, logs, public_render, player_renders),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected response from game service").into_response(),
    };

    let (_is_finished, whose_turn, eliminated, placings) = match game_info.status {
        Status::Active { whose_turn, eliminated } => (false, whose_turn, eliminated, vec![]),
        Status::Finished { placings, .. } => (true, vec![], vec![], placings),
    };

    let game = match db::create_game_with_users(
        &pool,
        CreateGameOpts {
            game_version_id: payload.game_version_id,
            whose_turn: &whose_turn,
            eliminated: &eliminated,
            placings: &placings,
            points: &game_info.points,
            creator_id: user.id,
            opponent_ids: &opponent_ids,
            opponent_emails: &opponent_emails,
            bot_slots: &bot_slots,
            chat_id: None,
            game_state: &game_info.state,
        },
    ).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create game: {}", e)).into_response(),
    };

    if let Err(e) = db::create_game_logs(&pool, game.id, logs).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create game logs: {}", e)).into_response();
    }

    if let Ok(Some(ge)) = db::find_game_extended(&pool, game.id).await {
        let all_logs = db::get_all_game_logs(&pool, game.id).await.unwrap_or_default();
        broadcaster.broadcast_game_update(&pool, &ge, &all_logs, &public_render, &player_renders).await;
        super::trigger_bot_turns(&http_client, &ge).await;
    }

    (StatusCode::CREATED, Json(game)).into_response()
}

pub async fn get_game(
    session: Session,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if get_user_from_session(&session).await.is_none() {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    match db::find_game_extended(&pool, id).await {
        Ok(Some(game)) => Json(game).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Game not found").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    }
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

pub async fn play_command(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CommandRequest>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let position: i32 = match sqlx::query_scalar!(
        "SELECT position FROM game_players WHERE game_id = $1 AND user_id = $2",
        id,
        user.id
    )
    .fetch_optional(&pool)
    .await {
        Ok(Some(pos)) => pos,
        Ok(None) => return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    match super::execute_command(&pool, &http_client, &broadcaster, id, position as usize, payload.command).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct InternalCommandRequest {
    pub player_position: usize,
    pub command: String,
}

pub async fn internal_play_command(
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<InternalCommandRequest>,
) -> impl IntoResponse {
    let expected_key = match std::env::var("INTERNAL_API_KEY") {
        Ok(k) => k,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_API_KEY not configured").into_response(),
    };
    let provided_key = match headers.get("X-Internal-Key").and_then(|v| v.to_str().ok()) {
        Some(k) => k.to_string(),
        None => return (StatusCode::UNAUTHORIZED, "Missing X-Internal-Key header").into_response(),
    };
    if provided_key != expected_key {
        return (StatusCode::UNAUTHORIZED, "Invalid internal key").into_response();
    }

    match super::execute_command(&pool, &http_client, &broadcaster, id, payload.player_position, payload.command).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn undo_game(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let game_extended = match db::find_game_extended(&pool, id).await {
        Ok(Some(ge)) => ge,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let player = match game_extended.game_players.iter().find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id)) {
        Some(p) => p,
        None => return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response(),
    };

    let undo_state = match &player.game_player.undo_game_state {
        Some(s) => s.clone(),
        None => return (StatusCode::BAD_REQUEST, "No undo state available").into_response(),
    };

    let resp = match client::request(&http_client, &game_extended.game_version.uri, &brdgme_cmd::api::Request::Status { game: undo_state.clone() }).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Game service error: {}", e)).into_response(),
    };

    let (game_response, public_render, player_renders) = match resp {
        Response::Status { game, public_render, player_renders } => (game, public_render, player_renders),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected response from game service").into_response(),
    };

    let (whose_turn, eliminated, placings) = match game_response.status {
        Status::Active { whose_turn, eliminated } => (whose_turn, eliminated, vec![]),
        Status::Finished { placings, .. } => (vec![], vec![], placings),
    };

    if let Err(e) = db::undo_game(&pool, id, &undo_state, player.game_player.position as usize, &whose_turn, &eliminated, &placings).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to undo game: {}", e)).into_response();
    }

    if let Ok(Some(updated_ge)) = db::find_game_extended(&pool, id).await {
        let all_logs = db::get_all_game_logs(&pool, id).await.unwrap_or_default();
        broadcaster.broadcast_game_update(&pool, &updated_ge, &all_logs, &public_render, &player_renders).await;
        super::trigger_bot_turns(&http_client, &updated_ge).await;
    }

    StatusCode::OK.into_response()
}

pub async fn restart_game(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let game_extended = match db::find_game_extended(&pool, id).await {
        Ok(Some(ge)) => ge,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    if !game_extended.game.is_finished {
        return (StatusCode::BAD_REQUEST, "Game is not finished").into_response();
    }

    if game_extended.game.restarted_game_id.is_some() {
        return (StatusCode::BAD_REQUEST, "Game has already been restarted").into_response();
    }

    if !game_extended.game_players.iter().any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id)) {
        return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response();
    }

    let player_count = game_extended.game_players.len();
    let resp = match client::request(&http_client, &game_extended.game_version.uri, &Request::New { players: player_count }).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Game service error: {}", e)).into_response(),
    };

    let (game_info, logs, public_render, player_renders) = match resp {
        Response::New { game, logs, public_render, player_renders } => (game, logs, public_render, player_renders),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected response from game service").into_response(),
    };

    let (whose_turn, eliminated, placings) = match game_info.status {
        Status::Active { whose_turn, eliminated } => (whose_turn, eliminated, vec![]),
        Status::Finished { placings, .. } => (vec![], vec![], placings),
    };

    let opponent_ids: Vec<Uuid> = game_extended.game_players.iter()
        .filter_map(|p| p.user.as_ref().filter(|u| u.id != user.id).map(|u| u.id))
        .collect();

    let new_game = match db::create_game_with_users(
        &pool,
        db::CreateGameOpts {
            game_version_id: game_extended.game.game_version_id,
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
    ).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create game: {}", e)).into_response(),
    };

    if let Err(e) = db::create_game_logs(&pool, new_game.id, logs).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create game logs: {}", e)).into_response();
    }

    if let Err(e) = sqlx::query!(
        "UPDATE games SET restarted_game_id = $1, updated_at = NOW() WHERE id = $2",
        new_game.id,
        id
    )
    .execute(&pool)
    .await {
        tracing::error!("Failed to set restarted_game_id on game {}: {}", id, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
    }

    if let Ok(Some(new_ge)) = db::find_game_extended(&pool, new_game.id).await {
        let all_logs = db::get_all_game_logs(&pool, new_game.id).await.unwrap_or_default();
        broadcaster.broadcast_game_update(&pool, &new_ge, &all_logs, &public_render, &player_renders).await;
        super::trigger_bot_turns(&http_client, &new_ge).await;
    }

    if let Ok(Some(old_ge)) = db::find_game_extended(&pool, id).await {
        if let Ok(Response::Status { public_render: old_pub, player_renders: old_pr, .. }) =
            client::request(&http_client, &old_ge.game_version.uri, &Request::Status { game: old_ge.game.game_state.clone() }).await
        {
            let old_logs = db::get_all_game_logs(&pool, id).await.unwrap_or_default();
            broadcaster.broadcast_game_update(&pool, &old_ge, &old_logs, &old_pub, &old_pr).await;
        }
    }

    (StatusCode::CREATED, Json(new_game)).into_response()
}

pub async fn concede_game(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    State(http_client): State<reqwest::Client>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let game_extended = match db::find_game_extended(&pool, id).await {
        Ok(Some(ge)) => ge,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    if game_extended.game.is_finished {
        return (StatusCode::BAD_REQUEST, "Game is already finished").into_response();
    }

    if game_extended.game_players.len() != 2 {
        return (StatusCode::BAD_REQUEST, "Concede is only available in 2-player games").into_response();
    }

    let player = match game_extended.game_players.iter().find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id)) {
        Some(p) => p,
        None => return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response(),
    };

    if let Err(e) = db::concede_game(&pool, id, player.game_player.id, player.name()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to concede game: {}", e)).into_response();
    }

    if let Ok(Some(updated_ge)) = db::find_game_extended(&pool, id).await {
        let all_logs = db::get_all_game_logs(&pool, id).await.unwrap_or_default();
        match client::request(&http_client, &updated_ge.game_version.uri, &Request::Status { game: updated_ge.game.game_state.clone() }).await {
            Ok(Response::Status { public_render, player_renders, .. }) => {
                broadcaster.broadcast_game_update(&pool, &updated_ge, &all_logs, &public_render, &player_renders).await;
                super::trigger_bot_turns(&http_client, &updated_ge).await;
            }
            _ => {
                tracing::error!("Unexpected response from game service on concede status call for game {}", id);
            }
        }
    }

    StatusCode::OK.into_response()
}

pub async fn mark_read(
    session: Session,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let is_player = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM game_players WHERE game_id = $1 AND user_id = $2)",
        id,
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(Some(false))
    .unwrap_or(false);

    if !is_player {
        return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response();
    }

    match db::mark_game_read(&pool, id, user.id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!("Failed to mark game {} read: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

pub fn api_routes() -> axum::Router<crate::state::AppState> {
    axum::Router::new()
        .route("/game/new", axum::routing::post(create_game))
        .route("/game/{id}", axum::routing::get(get_game))
        .route("/game/{id}/command", axum::routing::post(play_command))
        .route("/game/{id}/undo", axum::routing::post(undo_game))
        .route("/game/{id}/mark_read", axum::routing::post(mark_read))
        .route("/game/{id}/concede", axum::routing::post(concede_game))
        .route("/game/{id}/restart", axum::routing::post(restart_game))
        .route("/internal/game/{id}/command", axum::routing::post(internal_play_command))
}
