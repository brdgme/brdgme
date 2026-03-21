#[cfg(feature = "ssr")]
pub mod client;
#[cfg(feature = "ssr")]
pub mod server;
pub mod server_fns;

#[cfg(feature = "ssr")]
pub async fn execute_command(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    broadcaster: &crate::websocket::GameBroadcaster,
    game_id: uuid::Uuid,
    user_id: uuid::Uuid,
    command: String,
) -> anyhow::Result<()> {
    use brdgme_cmd::api::{Request, Response};
    use brdgme_game::Status;
    use crate::websocket::WebSocketMessage;

    let ge = crate::db::find_game_extended(pool, game_id).await?
        .ok_or_else(|| anyhow::anyhow!("Game not found"))?;

    if ge.game.is_finished {
        return Err(anyhow::anyhow!("Game is already finished"));
    }

    let player = ge.game_players.iter().find(|p| p.user.id == user_id)
        .ok_or_else(|| anyhow::anyhow!("You are not a player in this game"))?;

    if !player.game_player.is_turn {
        return Err(anyhow::anyhow!("Not your turn"));
    }

    let names: Vec<String> = ge.game_players.iter().map(|p| p.user.name.clone()).collect();

    let resp = client::request(http_client, &ge.game_version.uri, &Request::Play {
        player: player.game_player.position as usize,
        game: ge.game.game_state.clone(),
        command,
        names,
    }).await?;

    let (game_response, logs, can_undo, remaining_input) = match resp {
        Response::Play { game, logs, can_undo, remaining_input, .. } =>
            (game, logs, can_undo, remaining_input),
        Response::UserError { message } => return Err(anyhow::anyhow!("{}", message)),
        _ => return Err(anyhow::anyhow!("Unexpected response from game service")),
    };

    if !remaining_input.trim().is_empty() {
        return Err(anyhow::anyhow!("Unexpected input: {}", remaining_input));
    }

    let prev_game_state = ge.game.game_state.clone();
    let (is_finished, whose_turn, eliminated, placings) = match game_response.status {
        Status::Active { whose_turn, eliminated } => (false, whose_turn, eliminated, vec![]),
        Status::Finished { placings, .. } => (true, vec![], vec![], placings),
    };

    crate::db::update_game_command_success(
        pool,
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
    ).await?;

    crate::db::create_game_logs(pool, game_id, logs).await?;

    broadcaster.broadcast(WebSocketMessage::GameUpdate { game_id });
    Ok(())
}
