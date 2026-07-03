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
    player_position: usize,
    command: String,
) -> anyhow::Result<()> {
    use brdgme_cmd::api::{Request, Response};
    use brdgme_game::Status;

    let ge = crate::db::find_game_extended(pool, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Game not found"))?;

    if ge.game.is_finished {
        return Err(anyhow::anyhow!("Game is already finished"));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.position as usize == player_position)
        .ok_or_else(|| anyhow::anyhow!("Invalid player position"))?;

    if !player.game_player.is_turn {
        return Err(anyhow::anyhow!("Not your turn"));
    }

    let names: Vec<String> = ge
        .game_players
        .iter()
        .map(|p| p.name().to_string())
        .collect();

    let resp = client::request(
        http_client,
        &ge.game_version.uri,
        &Request::Play {
            player: player.game_player.position as usize,
            game: ge.game.game_state.clone(),
            command,
            names,
        },
    )
    .await?;

    let (game_response, logs, can_undo, remaining_input, public_render, player_renders) = match resp
    {
        Response::Play {
            game,
            logs,
            can_undo,
            remaining_input,
            public_render,
            player_renders,
        } => (
            game,
            logs,
            can_undo,
            remaining_input,
            public_render,
            player_renders,
        ),
        Response::UserError { message } => return Err(anyhow::anyhow!("{}", message)),
        _ => return Err(anyhow::anyhow!("Unexpected response from game service")),
    };

    if !remaining_input.trim().is_empty() {
        return Err(anyhow::anyhow!("Unexpected input: {}", remaining_input));
    }

    let prev_game_state = ge.game.game_state.clone();
    let (is_finished, whose_turn, eliminated, placings) = match game_response.status {
        Status::Active {
            whose_turn,
            eliminated,
        } => (false, whose_turn, eliminated, vec![]),
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
    )
    .await?;

    crate::db::create_game_logs(pool, game_id, logs).await?;

    // Fetch updated state for broadcast and bot triggering
    match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(updated_ge)) => {
            let all_logs = crate::db::get_all_game_logs(pool, game_id)
                .await
                .unwrap_or_default();
            broadcaster
                .broadcast_game_update(
                    pool,
                    &updated_ge,
                    &all_logs,
                    &public_render,
                    &player_renders,
                )
                .await;
            trigger_bot_turns(http_client, &updated_ge).await;
        }
        Ok(None) => tracing::warn!(%game_id, "Game not found after command execution"),
        Err(e) => tracing::warn!(%game_id, "Failed to reload game after command execution: {}", e),
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn trigger_bot_turns(http_client: &reqwest::Client, ge: &crate::db::GameExtended) {
    let bot_service_url = match std::env::var("BOT_SERVICE_URL") {
        Ok(u) => u,
        Err(_) => {
            tracing::warn!(game_id = %ge.game.id, "BOT_SERVICE_URL not set, skipping bot triggers");
            return;
        }
    };

    for player in &ge.game_players {
        tracing::debug!(
            game_id = %ge.game.id,
            position = player.game_player.position,
            is_turn = player.game_player.is_turn,
            is_bot = player.game_bot.is_some(),
            "Checking player for bot trigger"
        );
        if !player.game_player.is_turn {
            continue;
        }
        let bot = match &player.game_bot {
            Some(b) => b,
            None => continue,
        };
        let url = format!("{}/trigger", bot_service_url);
        tracing::info!(
            game_id = %ge.game.id,
            position = player.game_player.position,
            difficulty = %bot.difficulty,
            %url,
            "Triggering bot turn"
        );
        let body = serde_json::json!({
            "game_id": ge.game.id,
            "player_position": player.game_player.position,
            "difficulty": bot.difficulty,
        });
        let client = http_client.clone();
        tokio::spawn(async move {
            match client.post(&url).json(&body).send().await {
                Err(e) => tracing::warn!(%url, "Failed to trigger bot turn: {}", e),
                Ok(r) if !r.status().is_success() => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    tracing::warn!(%url, %status, "Bot trigger returned error: {}", body);
                }
                Ok(_) => tracing::debug!(%url, "Bot turn triggered successfully"),
            }
        });
    }
}
