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
use crate::websocket::{GameBroadcaster, WebSocketMessage};
use brdgme_cmd::api::{Request, Response};
use brdgme_game::Status;

#[derive(Deserialize)]
pub struct CreateGameRequest {
    pub game_version_id: Uuid,
    pub opponent_ids: Option<Vec<Uuid>>,
    pub opponent_emails: Option<Vec<String>>,
}

pub async fn create_game(
    session: Session,
    State(pool): State<PgPool>,
    State(broadcaster): State<GameBroadcaster>,
    Json(payload): Json<CreateGameRequest>,
) -> impl IntoResponse {
    let user = match get_user_from_session(&session).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Authentication required").into_response(),
    };

    let opponent_ids = payload.opponent_ids.unwrap_or_default();
    let opponent_emails = payload.opponent_emails.unwrap_or_default();
    let player_count = 1 + opponent_ids.len() + opponent_emails.len();

    let game_version = match db::find_game_version(&pool, payload.game_version_id).await {
        Ok(Some(gv)) => gv,
        Ok(None) => return (StatusCode::NOT_FOUND, "Game version not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    let resp = match client::request(&game_version.uri, &Request::New { players: player_count }).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Game service error: {}", e)).into_response(),
    };

    let (game_info, logs, _public_render, _player_renders) = match resp {
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

    broadcaster.broadcast(WebSocketMessage::GameUpdate { game_id: game.id });

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
    Path(id): Path<Uuid>,
    Json(payload): Json<CommandRequest>,
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

    let player = match game_extended.game_players.iter().find(|p| p.user.id == user.id) {
        Some(p) => p,
        None => return (StatusCode::FORBIDDEN, "You are not a player in this game").into_response(),
    };

    if !player.game_player.is_turn {
        return (StatusCode::FORBIDDEN, "Not your turn").into_response();
    }

    let names: Vec<String> = game_extended.game_players.iter().map(|p| p.user.name.clone()).collect();

    let resp = match client::request(
        &game_extended.game_version.uri,
        &Request::Play {
            player: player.game_player.position as usize,
            game: game_extended.game.game_state.clone(),
            command: payload.command,
            names,
        }
    ).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Game service error: {}", e)).into_response(),
    };

    let (game_response, logs, can_undo, remaining_input, _public_render, _player_renders) = match resp {
        Response::Play { game, logs, can_undo, remaining_input, public_render, player_renders } =>
            (game, logs, can_undo, remaining_input, public_render, player_renders),
        Response::UserError { message } => return (StatusCode::BAD_REQUEST, message).into_response(),
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected response from game service").into_response(),
    };

    if !remaining_input.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, format!("Unexpected input: {}", remaining_input)).into_response();
    }

    let prev_game_state = game_extended.game.game_state.clone();
    let (is_finished, whose_turn, eliminated, placings) = match game_response.status {
        Status::Active { whose_turn, eliminated } => (false, whose_turn, eliminated, vec![]),
        Status::Finished { placings, .. } => (true, vec![], vec![], placings),
    };

    if let Err(e) = db::update_game_command_success(
        &pool,
        id,
        player.game_player.id,
        &prev_game_state,
        &game_response.state,
        can_undo,
        is_finished,
        &whose_turn,
        &eliminated,
        &placings,
        &game_response.points,
    ).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update game: {}", e)).into_response();
    }

    if let Err(e) = db::create_game_logs(&pool, id, logs).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create game logs: {}", e)).into_response();
    }

    broadcaster.broadcast(WebSocketMessage::GameUpdate { game_id: id });

    StatusCode::OK.into_response()
}

pub fn api_routes() -> axum::Router<crate::state::AppState> {
    axum::Router::new()
        .route("/game/new", axum::routing::post(create_game))
        .route("/game/{id}", axum::routing::get(get_game))
        .route("/game/{id}/command", axum::routing::post(play_command))
}
