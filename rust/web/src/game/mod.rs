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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::{self, CreateGameOpts};
    use crate::models::user::User;
    use axum::{routing::post, Json, Router};
    use brdgme_cmd::api::{CliLog, GameResponse, PlayerRender, PubRender, Request, Response};
    use sqlx::PgPool;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use uuid::Uuid;

    fn now() -> time::PrimitiveDateTime {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    }

    /// Starts an in-process mock game service that answers every request with
    /// whatever `handler` returns; mirrors the pattern in `game::client::tests`.
    async fn spawn_mock_game_service<F>(handler: F) -> String
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| {
                let handler = handler.clone();
                async move { Json(handler(payload)) }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    async fn make_user(pool: &PgPool, name: &str) -> User {
        sqlx::query_as!(
            User,
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING *",
            Uuid::new_v4(),
            name,
            &Vec::<String>::new()
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn make_game_version(pool: &PgPool, uri: &str) -> Uuid {
        let game_type_id = sqlx::query_scalar!(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
            format!("Test Game {}", Uuid::new_v4()),
            &vec![2, 3, 4]
        )
        .fetch_one(pool)
        .await
        .unwrap();

        sqlx::query_scalar!(
            r#"INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
               VALUES ($1, $2, $3, true, false) RETURNING id"#,
            game_type_id,
            "1.0.0",
            uri
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn make_broadcaster() -> crate::websocket::GameBroadcaster {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        let conn = client.get_multiplexed_async_connection().await.unwrap();
        crate::websocket::GameBroadcaster::new(conn, client)
    }

    /// Two human players (position 0, 1), player 0 on turn, pointed at `uri`.
    async fn make_two_player_game(pool: &PgPool, uri: &str) -> (Uuid, User, User) {
        let p0 = make_user(pool, "p0").await;
        let p1 = make_user(pool, "p1").await;
        let game_version_id = make_game_version(pool, uri).await;
        let game = db::create_game_with_users(
            pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: p0.id,
                opponent_ids: &[p1.id],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "initial_state",
            },
        )
        .await
        .unwrap();
        (game.id, p0, p1)
    }

    fn play_response(state: &str, whose_turn: Vec<usize>, can_undo: bool) -> Response {
        Response::Play {
            game: GameResponse {
                state: state.to_string(),
                points: vec![0.0, 0.0],
                status: brdgme_game::Status::Active {
                    whose_turn,
                    eliminated: vec![],
                },
            },
            logs: vec![CliLog {
                content: "did a thing".to_string(),
                at: now(),
                public: true,
                to: vec![],
            }],
            can_undo,
            remaining_input: String::new(),
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: vec![
                PlayerRender {
                    player_state: "p0".to_string(),
                    render: "p0render".to_string(),
                    command_spec: None,
                },
                PlayerRender {
                    player_state: "p1".to_string(),
                    render: "p1render".to_string(),
                    command_spec: None,
                },
            ],
        }
    }

    #[sqlx::test]
    async fn happy_path_saves_state_and_advances_turn(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "new_state");
        assert!(!ge.game.is_finished);

        let logs = db::get_all_game_logs(&pool, game_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].body, "did a thing");

        let player0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let player1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(!player0.game_player.is_turn);
        assert!(player1.game_player.is_turn);
        assert_eq!(
            player0.game_player.undo_game_state.as_deref(),
            Some("initial_state")
        );
        assert!(player1.game_player.undo_game_state.is_none());
    }

    #[sqlx::test]
    async fn not_players_turn_returns_err_and_leaves_game_unchanged(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            1, // not player 1's turn
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn finished_game_returns_err_and_leaves_game_unchanged(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        // Force the game to already be finished.
        sqlx::query!("UPDATE games SET is_finished = true WHERE id = $1", game_id)
            .execute(&pool)
            .await
            .unwrap();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn user_error_propagated_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::UserError {
            message: "invalid command".to_string(),
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid command"));
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn system_error_propagated_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::SystemError {
            message: "boom".to_string(),
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn remaining_input_returns_err_and_no_db_write(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| {
            let mut resp = play_response("new_state", vec![1], true);
            if let Response::Play {
                ref mut remaining_input,
                ..
            } = resp
            {
                *remaining_input = "extra".to_string();
            }
            resp
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        let result = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await;

        assert!(result.is_err());
        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ge.game.game_state, "initial_state");
    }

    #[sqlx::test]
    async fn finished_status_persists_placings(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| Response::Play {
            game: GameResponse {
                state: "final_state".to_string(),
                points: vec![1.0, 0.0],
                status: brdgme_game::Status::Finished {
                    placings: vec![0, 1],
                    stats: vec![],
                },
            },
            logs: vec![],
            can_undo: false,
            remaining_input: String::new(),
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: vec![
                PlayerRender {
                    player_state: "p0".to_string(),
                    render: "p0render".to_string(),
                    command_spec: None,
                },
                PlayerRender {
                    player_state: "p1".to_string(),
                    render: "p1render".to_string(),
                    command_spec: None,
                },
            ],
        })
        .await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();

        execute_command(
            &pool,
            &http_client,
            &broadcaster,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        let ge = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert!(ge.game.is_finished);
        assert!(ge.game.finished_at.is_some());

        let player0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let player1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(player0.game_player.place, Some(0));
        assert_eq!(player1.game_player.place, Some(1));
    }

    #[tokio::test]
    async fn trigger_bot_turns_noop_when_bot_service_url_unset() {
        assert!(std::env::var("BOT_SERVICE_URL").is_err());

        let http_client = reqwest::Client::new();
        let ge = crate::db::GameExtended {
            game: crate::models::game::Game {
                id: Uuid::new_v4(),
                created_at: now(),
                updated_at: now(),
                game_version_id: Uuid::new_v4(),
                is_finished: false,
                finished_at: None,
                game_state: "state".to_string(),
                chat_id: None,
                restarted_game_id: None,
            },
            game_type: crate::models::game::GameType {
                id: Uuid::new_v4(),
                created_at: now(),
                updated_at: now(),
                name: "Test".to_string(),
                player_counts: vec![2],
                weight: 1.0,
            },
            game_version: crate::models::game::GameVersion {
                id: Uuid::new_v4(),
                created_at: now(),
                updated_at: now(),
                game_type_id: Uuid::new_v4(),
                name: "1.0.0".to_string(),
                uri: "http://localhost:0".to_string(),
                is_public: true,
                is_deprecated: false,
            },
            game_players: vec![],
        };

        // No-op, no panic: nothing to assert beyond "returns".
        trigger_bot_turns(&http_client, &ge).await;
    }
}
