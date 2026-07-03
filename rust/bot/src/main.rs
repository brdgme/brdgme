mod prompt;

use anyhow::{Context, Result, anyhow};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse};
use brdgme_cmd::api::{Request, Response};
use brdgme_color::player_color;
use prompt::{
    FailedCommand, PlayerInfo, PromptContext, markup_resolve_players, render_prompt, spec_to_yaml,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    http: reqwest::Client,
    llm_url: String,
    llm_api_key: Option<String>,
    bot_model: String,
    reasoning_effort: Option<String>,
    monolith_url: String,
    internal_api_key: String,
}

#[derive(Debug, Deserialize)]
struct TriggerRequest {
    game_id: Uuid,
    player_position: i32,
    difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
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
            tracing::error!(trace_id = %trace_id, error = ?e, "Bot turn hard failed - game is stuck waiting for bot");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

async fn run_bot_turn(state: &AppState, req: TriggerRequest, trace_id: Uuid) -> Result<()> {
    // 1. Fetch game data from DB.
    let row = sqlx::query(
        r#"
        SELECT g.game_state, gv.uri, gv.rules, gt.name as game_name, gb.name as bot_name, gp.is_turn
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
    let rules: String = row.try_get("rules").unwrap_or_default();
    let game_name: String = row
        .try_get("game_name")
        .unwrap_or_else(|_| "unknown".to_string());
    let bot_name: String = row
        .try_get::<Option<String>, _>("bot_name")
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
            user_name
                .or(bot_name)
                .unwrap_or_else(|| format!("Player {}", position + 1))
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
    let mut bot_ctx = load_bot_context(
        &state,
        &game_service_uri,
        req.game_id,
        req.player_position,
        game_state,
        &names,
    )
    .await
    .context("Failed to load initial bot context")?;

    let internal_url = format!(
        "{}/api/internal/game/{}/command",
        state.monolith_url, req.game_id
    );
    const MAX_ATTEMPTS: usize = 20;
    let mut failed_commands: Vec<FailedCommand> = Vec::new();

    for attempt in 0..MAX_ATTEMPTS {
        let prompt_ctx = build_prompt_context(
            &rules,
            &bot_name,
            &req.difficulty,
            &names,
            req.player_position as usize,
            &bot_ctx,
            std::mem::take(&mut failed_commands),
        );
        let messages = build_messages(&prompt_ctx)
            .with_context(|| format!("Failed to build messages on attempt {}", attempt + 1))?;
        // Restore failed_commands from the context (build_prompt_context consumed it).
        failed_commands = prompt_ctx.failed_commands;

        tracing::trace!(
            trace_id = %trace_id,
            attempt,
            prompt = %messages.first().map(|m| m.content.as_str()).unwrap_or(""),
            "Rendered prompt"
        );

        let raw_response = call_llm(
            &state.http,
            &state.llm_url,
            &state.bot_model,
            &messages,
            state.llm_api_key.as_deref(),
            state.reasoning_effort.clone(),
        )
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

        let current_game_state: String = recheck
            .try_get("game_state")
            .context("game_state recheck")?;
        if current_game_state != bot_ctx.game_state {
            tracing::warn!(
                trace_id = %trace_id,
                game_id = %req.game_id,
                player = req.player_position,
                attempt,
                "Game state changed while Ollama was thinking, refreshing context and retrying"
            );
            bot_ctx = load_bot_context(
                &state,
                &game_service_uri,
                req.game_id,
                req.player_position,
                current_game_state,
                &names,
            )
            .await
            .context("Failed to refresh bot context")?;
            failed_commands.clear();
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

        let result = state
            .http
            .post(&internal_url)
            .header("X-Internal-Key", &state.internal_api_key)
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
                MAX_ATTEMPTS,
                command,
                status,
                error_body
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

        failed_commands.push(FailedCommand {
            command,
            error: error_body,
        });
    }

    unreachable!()
}

struct BotContext {
    game_state: String,
    render: String,
    command_spec_yaml: String,
    recent_logs: Vec<String>,
    points: Vec<f32>,
}

async fn load_bot_context(
    state: &AppState,
    game_service_uri: &str,
    game_id: uuid::Uuid,
    player_position: i32,
    game_state: String,
    names: &[String],
) -> Result<BotContext> {
    let status_resp = call_game_service(
        &state.http,
        game_service_uri,
        &Request::Status {
            game: game_state.clone(),
        },
    )
    .await
    .context("Game service Status call failed")?;

    let (render_markup, command_spec, points) = match status_resp {
        Response::Status {
            game,
            player_renders,
            ..
        } => {
            let pr = player_renders
                .into_iter()
                .nth(player_position as usize)
                .ok_or_else(|| anyhow!("Player position out of range"))?;
            (pr.render, pr.command_spec, game.points)
        }
        _ => return Err(anyhow!("Unexpected response from game service")),
    };

    let render = markup_resolve_players(&render_markup, names);
    let command_spec_yaml = command_spec.as_ref().map(spec_to_yaml).unwrap_or_default();

    let log_rows = sqlx::query(
        "SELECT body FROM game_logs WHERE game_id = $1 ORDER BY logged_at DESC LIMIT 20",
    )
    .bind(game_id)
    .fetch_all(&state.pool)
    .await
    .context("Failed to fetch game logs")?;

    let recent_logs: Vec<String> = log_rows
        .into_iter()
        .rev()
        .map(|r| {
            let body = r.try_get::<String, _>("body").unwrap_or_default();
            markup_resolve_players(&body, names)
        })
        .collect();

    Ok(BotContext {
        game_state,
        render,
        command_spec_yaml,
        recent_logs,
        points,
    })
}

fn build_messages(ctx: &PromptContext) -> Result<Vec<ChatMessage>> {
    let content = render_prompt(ctx).context("Failed to render system prompt template")?;
    Ok(vec![
        ChatMessage {
            role: "system".to_string(),
            content,
        },
        ChatMessage {
            role: "user".to_string(),
            content: "Please provide your command now.".to_string(),
        },
    ])
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

    resp.json::<Response>()
        .await
        .context("Failed to parse game service response")
}

async fn call_llm(
    http: &reqwest::Client,
    llm_url: &str,
    model: &str,
    messages: &[ChatMessage],
    api_key: Option<&str>,
    reasoning_effort: Option<String>,
) -> Result<String> {
    let url = format!("{}/v1/chat/completions", llm_url);
    let body = ChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        stream: false,
        temperature: 0.2,
        reasoning_effort,
    };

    let mut req = http.post(&url).json(&body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().await.context("HTTP request to LLM failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("LLM returned {}: {}", status, body));
    }

    let chat_resp: ChatResponse = resp.json().await.context("Failed to parse LLM response")?;
    chat_resp
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("LLM returned no choices"))?
        .message
        .content
        .ok_or_else(|| anyhow!("LLM returned null content (reasoning budget exhausted?)"))
}

fn build_prompt_context(
    rules: &str,
    bot_name: &str,
    difficulty: &str,
    names: &[String],
    player_position: usize,
    bot_ctx: &BotContext,
    failed_commands: Vec<FailedCommand>,
) -> PromptContext {
    let my_name = names
        .get(player_position)
        .map(|s| s.as_str())
        .unwrap_or(bot_name)
        .to_string();
    let my_colour = format!("{}", player_color(player_position));

    let players = names
        .iter()
        .enumerate()
        .map(|(i, name)| PlayerInfo {
            name: name.clone(),
            colour: format!("{}", player_color(i)),
            score: bot_ctx.points.get(i).copied().unwrap_or(0.0),
        })
        .collect();

    PromptContext {
        game_rules: rules.to_string(),
        difficulty: difficulty.to_string(),
        my_name,
        my_colour,
        players,
        game_render: bot_ctx.render.clone(),
        recent_logs: bot_ctx.recent_logs.clone(),
        command_spec: bot_ctx.command_spec_yaml.clone(),
        failed_commands,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let llm_url = std::env::var("LLM_URL").context("LLM_URL must be set")?;
    let llm_api_key = std::env::var("LLM_API_KEY").ok();
    let bot_model = std::env::var("BOT_MODEL").context("BOT_MODEL must be set")?;
    let reasoning_effort =
        Some(std::env::var("REASONING_EFFORT").unwrap_or_else(|_| "low".to_string()));
    let monolith_url = std::env::var("MONOLITH_URL").context("MONOLITH_URL must be set")?;
    let internal_api_key =
        std::env::var("INTERNAL_API_KEY").context("INTERNAL_API_KEY must be set")?;
    tracing::info!(llm_url = %llm_url, bot_model = %bot_model, reasoning_effort = ?reasoning_effort, "Bot service starting");

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPool::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let http = reqwest::Client::new();
    let state = Arc::new(AppState {
        pool,
        http,
        llm_url,
        llm_api_key,
        bot_model,
        reasoning_effort,
        monolith_url,
        internal_api_key,
    });

    let app = Router::new()
        .route("/trigger", axum::routing::post(trigger))
        .with_state(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
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
