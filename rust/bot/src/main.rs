use anyhow::{anyhow, Context, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json, Router};
use brdgme_cmd::api::{Request, Response};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct TriggerRequest {
    game_id: Uuid,
    player_position: i32,
    difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    think: bool,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Serialize)]
struct InternalCommandRequest {
    player_position: i32,
    command: String,
}

async fn trigger(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TriggerRequest>,
) -> impl IntoResponse {
    let trace_id = Uuid::new_v4();
    match run_bot_turn(&state, payload, trace_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!(trace_id = %trace_id, error = %e, "Bot turn hard failed - game is stuck waiting for bot");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

async fn run_bot_turn(state: &AppState, req: TriggerRequest, trace_id: Uuid) -> Result<()> {
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let bot_model = std::env::var("BOT_MODEL")
        .unwrap_or_else(|_| "qwen3.5:4b".to_string());
    let monolith_url = std::env::var("MONOLITH_URL")
        .context("MONOLITH_URL must be set")?;
    let internal_key = std::env::var("INTERNAL_API_KEY")
        .context("INTERNAL_API_KEY must be set")?;
    let ollama_api_key = std::env::var("OLLAMA_API_KEY").ok();

    // 1. Fetch game data from DB.
    let row = sqlx::query(
        r#"
        SELECT g.game_state, gv.uri, gt.name as game_name, gb.name as bot_name, gp.is_turn
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        LEFT JOIN game_players gp ON gp.game_id = g.id AND gp.position = $2
        LEFT JOIN game_bots gb ON gb.id = gp.game_bot_id
        WHERE g.id = $1
        "#,
    )
    .bind(req.game_id)
    .bind(req.player_position)
    .fetch_one(&state.pool)
    .await
    .context("Failed to fetch game from database")?;

    let is_turn: bool = row.try_get("is_turn").unwrap_or(false);
    if !is_turn {
        tracing::info!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            player = req.player_position,
            "Bot turn no longer active (game state changed), skipping"
        );
        return Ok(());
    }

    let game_state: String = row.try_get("game_state").context("game_state")?;
    let game_service_uri: String = row.try_get("uri").context("uri")?;
    let game_name: String = row.try_get("game_name").unwrap_or_else(|_| "unknown".to_string());
    let bot_name: String = row.try_get::<Option<String>, _>("bot_name")
        .unwrap_or(None)
        .unwrap_or_else(|| format!("Bot {}", req.player_position + 1));

    // 2. Fetch all player names for the game (needed for Status call).
    let player_rows = sqlx::query(
        r#"
        SELECT gp.position, u.name as user_name, gb.name as bot_name
        FROM game_players gp
        LEFT JOIN users u ON u.id = gp.user_id
        LEFT JOIN game_bots gb ON gb.id = gp.game_bot_id
        WHERE gp.game_id = $1
        ORDER BY gp.position
        "#,
    )
    .bind(req.game_id)
    .fetch_all(&state.pool)
    .await
    .context("Failed to fetch game players")?;

    let names: Vec<String> = player_rows
        .iter()
        .map(|p| {
            let user_name: Option<String> = p.try_get("user_name").unwrap_or(None);
            let bot_name: Option<String> = p.try_get("bot_name").unwrap_or(None);
            let position: i32 = p.try_get("position").unwrap_or(0);
            user_name.or(bot_name).unwrap_or_else(|| format!("Player {}", position + 1))
        })
        .collect();

    tracing::info!(
        trace_id = %trace_id,
        game_id = %req.game_id,
        game = %game_name,
        player = req.player_position,
        player_name = %bot_name,
        difficulty = %req.difficulty,
        players = ?names,
        "Bot turn triggered"
    );

    // 3. Load game context (state, render, logs). Extracted into a helper so it can be
    //    refreshed mid-loop if the game state changes while Ollama is thinking.
    let rules = match call_game_service(&state.http, &game_service_uri, &Request::Rules).await {
        Ok(Response::Rules { rules }) => rules,
        _ => String::new(),
    };

    let mut ctx = load_bot_context(
        &state,
        &game_service_uri,
        req.game_id,
        req.player_position,
        game_state,
    ).await.context("Failed to load initial bot context")?;

    let system_prompt = build_system_prompt(&rules, &bot_name, &req.difficulty, &names, req.player_position as usize);
    let mut messages = build_initial_messages(&system_prompt, &ctx);

    let internal_url = format!("{}/api/internal/game/{}/command", monolith_url, req.game_id);
    const MAX_ATTEMPTS: usize = 20;

    for attempt in 0..MAX_ATTEMPTS {
        let raw_response = call_ollama(&state.http, &ollama_url, &bot_model, &messages, ollama_api_key.as_deref())
            .await
            .with_context(|| format!("Ollama call failed on attempt {}", attempt + 1))?;

        tracing::info!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            attempt,
            response = %raw_response,
            "Ollama response received"
        );

        let command = raw_response.trim().to_string();

        // Re-check DB after Ollama responds: it may have taken a while and the game
        // state could have changed (e.g., the player hit undo).
        let recheck = sqlx::query(
            "SELECT gp.is_turn, g.game_state FROM games g \
             LEFT JOIN game_players gp ON gp.game_id = g.id AND gp.position = $2 \
             WHERE g.id = $1",
        )
        .bind(req.game_id)
        .bind(req.player_position)
        .fetch_one(&state.pool)
        .await
        .context("Failed to re-check game state")?;

        let is_still_turn: bool = recheck.try_get("is_turn").unwrap_or(false);
        if !is_still_turn {
            tracing::info!(
                trace_id = %trace_id,
                game_id = %req.game_id,
                player = req.player_position,
                attempt,
                "Game state changed while Ollama was thinking, bot turn no longer active"
            );
            return Ok(());
        }

        let current_game_state: String = recheck.try_get("game_state").context("game_state recheck")?;
        if current_game_state != ctx.game_state {
            tracing::warn!(
                trace_id = %trace_id,
                game_id = %req.game_id,
                player = req.player_position,
                attempt,
                "Game state changed while Ollama was thinking, refreshing context and retrying"
            );
            ctx = load_bot_context(
                &state,
                &game_service_uri,
                req.game_id,
                req.player_position,
                current_game_state,
            ).await.context("Failed to refresh bot context")?;
            messages = build_initial_messages(&system_prompt, &ctx);
            continue;
        }

        tracing::info!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            player = req.player_position,
            attempt,
            command = %command,
            "Submitting command to monolith"
        );

        let result = state.http
            .post(&internal_url)
            .header("X-Internal-Key", &internal_key)
            .json(&InternalCommandRequest {
                player_position: req.player_position,
                command: command.clone(),
            })
            .send()
            .await
            .context("Failed to POST command to monolith")?;

        let status = result.status();

        if status.is_success() {
            tracing::info!(
                trace_id = %trace_id,
                game_id = %req.game_id,
                player = req.player_position,
                attempt,
                command = %command,
                "Bot command accepted"
            );
            return Ok(());
        }

        let error_body = result.text().await.unwrap_or_default();

        if attempt + 1 == MAX_ATTEMPTS {
            return Err(anyhow!(
                "Command rejected after {} attempts. Last command: {:?}. Last error (HTTP {}): {}",
                MAX_ATTEMPTS, command, status, error_body
            ));
        }

        tracing::warn!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            player = req.player_position,
            attempt,
            command = %command,
            http_status = %status,
            error = %error_body,
            "Bot command rejected, retrying"
        );

        messages.push(OllamaMessage { role: "assistant".to_string(), content: command.clone() });
        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: format!("That command was invalid: {}. Please try again with a different command.", error_body),
        });
    }

    unreachable!()
}

struct BotContext {
    game_state: String,
    render: String,
    command_spec: Option<brdgme_game::command::Spec>,
    recent_logs: Vec<String>,
}

async fn load_bot_context(
    state: &AppState,
    game_service_uri: &str,
    game_id: uuid::Uuid,
    player_position: i32,
    game_state: String,
) -> Result<BotContext> {
    let status_resp = call_game_service(
        &state.http,
        game_service_uri,
        &Request::Status { game: game_state.clone() },
    )
    .await
    .context("Game service Status call failed")?;

    let (render, command_spec) = match status_resp {
        Response::Status { player_renders, .. } => {
            let pr = player_renders
                .into_iter()
                .nth(player_position as usize)
                .ok_or_else(|| anyhow!("Player position out of range"))?;
            (pr.render, pr.command_spec)
        }
        _ => return Err(anyhow!("Unexpected response from game service")),
    };

    let log_rows = sqlx::query(
        "SELECT body FROM game_logs WHERE game_id = $1 ORDER BY logged_at DESC LIMIT 30",
    )
    .bind(game_id)
    .fetch_all(&state.pool)
    .await
    .context("Failed to fetch game logs")?;

    let recent_logs: Vec<String> = log_rows
        .into_iter()
        .rev()
        .map(|r| r.try_get::<String, _>("body").unwrap_or_default())
        .collect();

    Ok(BotContext { game_state, render, command_spec, recent_logs })
}

fn build_initial_messages(system_prompt: &str, ctx: &BotContext) -> Vec<OllamaMessage> {
    vec![
        OllamaMessage { role: "system".to_string(), content: system_prompt.to_string() },
        OllamaMessage { role: "user".to_string(), content: build_user_prompt(&ctx.render, &ctx.command_spec, &ctx.recent_logs) },
    ]
}

async fn call_game_service(
    http: &reqwest::Client,
    uri: &str,
    request: &Request,
) -> Result<Response> {
    let resp = http
        .post(uri)
        .json(request)
        .send()
        .await
        .context("HTTP request to game service failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Game service returned {}: {}", status, body));
    }

    resp.json::<Response>().await.context("Failed to parse game service response")
}

async fn call_ollama(
    http: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    messages: &[OllamaMessage],
    api_key: Option<&str>,
) -> Result<String> {
    let url = format!("{}/api/chat", ollama_url);
    let body = OllamaChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        think: false,
        stream: false,
    };

    let mut req = http.post(&url).json(&body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().await.context("HTTP request to Ollama failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama returned {}: {}", status, body));
    }

    let chat_resp: OllamaChatResponse = resp.json().await.context("Failed to parse Ollama response")?;
    Ok(chat_resp.message.content)
}

fn build_system_prompt(rules: &str, bot_name: &str, difficulty: &str, names: &[String], player_position: usize) -> String {
    let my_name = names.get(player_position).map(|s| s.as_str()).unwrap_or(bot_name);
    let difficulty_note = match difficulty {
        "easy" => "Play casually and make suboptimal moves occasionally.",
        "hard" => "Play at the highest level possible, analysing the position carefully.",
        _ => "Play at a reasonable level.",
    };
    let players_list = names.iter().enumerate()
        .map(|(i, n)| if i == player_position { format!("{} (you)", n) } else { n.clone() })
        .collect::<Vec<_>>()
        .join(", ");

    let mut prompt = format!(
        "You are {}, a bot playing a board game. You are player {} (0-indexed) in a game with players: {}.\n\
         {}\n\
         Commands are plain text. In the available commands syntax: bold/plain keywords must be typed exactly; \
         [name] is a placeholder - replace it with one of the listed values; 1-3 means a number in that range; \
         ? means optional; * means zero or more; + means one or more.\n\
         Respond with a single plain text command only. Do not include any explanation or additional text.",
        my_name, player_position, players_list, difficulty_note
    );
    if !rules.is_empty() {
        prompt.push_str("\n\n## Game Rules\n\n");
        prompt.push_str(rules);
    }
    prompt
}

fn build_user_prompt(
    render: &str,
    command_spec: &Option<brdgme_game::command::Spec>,
    recent_logs: &[String],
) -> String {
    let mut prompt = String::new();

    if !recent_logs.is_empty() {
        prompt.push_str("## Recent game log\n\n");
        for log in recent_logs {
            prompt.push_str(log);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str("## Current game state\n\n");
    prompt.push_str(render);
    prompt.push('\n');

    if let Some(spec) = command_spec {
        prompt.push_str("\n## Available commands\n\n");
        prompt.push_str(&serde_json::to_string_pretty(spec).unwrap_or_default());
        prompt.push('\n');
    }

    prompt.push_str("\nWhat is your next move? Respond with the command only.");
    prompt
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPool::connect(&database_url).await.context("Failed to connect to database")?;

    let http = reqwest::Client::new();
    let state = Arc::new(AppState { pool, http });

    let app = Router::new()
        .route("/trigger", axum::routing::post(trigger))
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    tracing::info!("Bot service listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await
        .context("Server error")?;

    Ok(())
}
