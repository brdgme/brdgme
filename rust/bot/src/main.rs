// config.rs and crypto.rs expose a fuller API (e.g. encrypt, BotConfig.name,
// ProviderConfig.priority) than the binary currently consumes at runtime; the
// unused items are exercised by their own unit tests.
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod crypto;
mod nats;
mod prompt;
mod routing;

use anyhow::{Context, Result, anyhow};
use axum::{Router, extract::State as AxumState, http::StatusCode, routing::get};
use brdgme_cmd::api::{Request, Response};
use brdgme_color::LIGHT;
use futures_util::StreamExt;
use nats::{BotCommandEvent, BotTurnEvent};
use prompt::{
    FailedCommand, PlayerInfo, SystemContext, UserContext, markup_resolve_players, render_system,
    render_user, spec_to_yaml,
};
use routing::ProviderRouter;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    http: reqwest::Client,
    /// Client for game service calls: shorter timeout than the LLM client,
    /// but generous enough for KEDA scale-from-zero cold starts.
    game_http: reqwest::Client,
    encryption_key: Option<[u8; 32]>,
    jetstream: async_nats::jetstream::Context,
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

/// Handles one `bot.turn` event: Status -> LLM -> game service `Play`
/// (stateless validate) -> retry LLM on invalid -> publish `bot.command` when
/// valid. The monolith owns the actual DB commit; this only validates.
async fn run_bot_turn(state: &AppState, req: BotTurnEvent, trace_id: Uuid) -> Result<()> {
    // 1. Fetch game data from DB.
    let row = sqlx::query(
        r#"
        SELECT g.game_state, gv.uri, gv.name as version_name, gv.rules, gv.interface_version, gt.name as game_name, gb.name as bot_name, gp.is_turn, gp.id as game_player_id
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
    let version_name: String = row.try_get("version_name").context("version_name")?;
    let interface_version: i32 = row.try_get("interface_version").unwrap_or(1);
    let game_player_id: Uuid = row.try_get("game_player_id").context("game_player_id")?;
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
        bot_name = %req.bot_name,
        attempt = req.attempt,
        players = ?names,
        "Bot turn triggered"
    );

    let table_empty = config::bots_table_empty(&state.pool)
        .await
        .context("Failed to check bots table")?;
    let bot_cfg = match config::load_bot_config(&state.pool, &req.bot_name)
        .await
        .context("Failed to load bot config")?
    {
        Some(c) => c,
        None => {
            if table_empty {
                config::BotConfig {
                    name: req.bot_name.clone(),
                    include_basic_strategy: true,
                    include_advanced_strategy: false,
                    temperature: 0.2,
                }
            } else {
                tracing::info!(
                    trace_id = %trace_id,
                    game_id = %req.game_id,
                    bot_name = %req.bot_name,
                    "Bot not found or disabled, skipping turn"
                );
                return Ok(());
            }
        }
    };

    let mut providers = match &state.encryption_key {
        Some(key) => config::load_providers(&state.pool, &req.bot_name, key)
            .await
            .context("Failed to load providers")?,
        None => Vec::new(),
    };
    if providers.is_empty()
        && table_empty
        && let Some(p) = config::env_fallback_provider()
    {
        providers = vec![p];
    }
    if providers.is_empty() {
        return Err(anyhow!(
            "No LLM providers configured for bot {}",
            req.bot_name
        ));
    }
    let mut router = ProviderRouter::new(providers);

    // 3. Load game context (state, structured game data, logs). Extracted into a helper
    //    so it can be refreshed mid-loop if the game state changes while the LLM is thinking.
    let mut bot_ctx = load_bot_context(
        state,
        &game_service_uri,
        &version_name,
        req.game_id,
        req.player_position,
        game_player_id,
        game_state,
        &names,
        interface_version,
    )
    .await
    .context("Failed to load initial bot context")?;

    const MAX_ATTEMPTS: usize = 20;
    let mut failed_commands: Vec<FailedCommand> = Vec::new();

    for attempt in 0..MAX_ATTEMPTS {
        let provider = router
            .next()
            .ok_or_else(|| anyhow!("All LLM providers exhausted"))?;
        let url = provider.url.clone();
        let model = provider.model.clone();
        let api_key = provider.api_key.clone();
        let reasoning_effort = provider.reasoning_effort.clone();
        let extra_body = provider.extra_body.clone();

        let messages = build_messages(
            &bot_cfg,
            &bot_ctx,
            &names,
            req.player_position as usize,
            &bot_name,
            failed_commands.clone(),
        )
        .with_context(|| format!("Failed to build messages on attempt {}", attempt + 1))?;

        tracing::trace!(
            trace_id = %trace_id,
            attempt,
            prompt = %messages.first().map(|m| m.content.as_str()).unwrap_or(""),
            "Rendered prompt"
        );

        let raw_response = match call_llm(
            &state.http,
            &url,
            &model,
            &messages,
            api_key.as_deref(),
            bot_cfg.temperature,
            reasoning_effort,
            extra_body.as_ref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    trace_id = %trace_id,
                    game_id = %req.game_id,
                    attempt,
                    model = %model,
                    error = %e,
                    "LLM API error, failing over to next provider"
                );
                router.mark_failed();
                continue;
            }
        };

        tracing::info!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            attempt,
            response = %raw_response,
            "LLM response received"
        );

        let command = raw_response.trim().to_string();

        // Re-check DB after the LLM responds: it may have taken a while and the game
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
                "Game state changed while the LLM was thinking, bot turn no longer active"
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
                "Game state changed while the LLM was thinking, refreshing context and retrying"
            );
            bot_ctx = load_bot_context(
                state,
                &game_service_uri,
                &version_name,
                req.game_id,
                req.player_position,
                game_player_id,
                current_game_state,
                &names,
                interface_version,
            )
            .await
            .context("Failed to refresh bot context")?;
            failed_commands.clear();
            continue;
        }

        // Validate the command against the game service directly. `Play` is
        // stateless (returns the new state but doesn't persist), so this
        // retry loop never round-trips through the monolith.
        let validate_result = brdgme_game_client::request(
            &state.game_http,
            &game_service_uri,
            &version_name,
            &Request::Play {
                player: req.player_position as usize,
                game: bot_ctx.game_state.clone(),
                command: command.clone(),
                names: names.clone(),
            },
        )
        .await;

        let error_body = match validate_result {
            Ok(Response::Play { .. }) => {
                tracing::info!(
                    trace_id = %trace_id,
                    game_id = %req.game_id,
                    player = req.player_position,
                    attempt,
                    command = %command,
                    "Command validated, publishing bot.command"
                );
                publish_bot_command(
                    state,
                    req.game_id,
                    req.player_position,
                    command,
                    req.attempt,
                )
                .await?;
                return Ok(());
            }
            Ok(Response::UserError { message }) => message,
            Ok(_) => "Unexpected response from game service".to_string(),
            Err(e) => e.to_string(),
        };

        if attempt + 1 == MAX_ATTEMPTS {
            return Err(anyhow!(
                "Command rejected after {} attempts. Last command: {:?}. Last error: {}",
                MAX_ATTEMPTS,
                command,
                error_body
            ));
        }

        tracing::warn!(
            trace_id = %trace_id,
            game_id = %req.game_id,
            player = req.player_position,
            attempt,
            command = %command,
            error = %error_body,
            "Bot command rejected by game service validation, retrying"
        );

        failed_commands.push(FailedCommand {
            command,
            error: error_body,
        });
    }

    unreachable!()
}

async fn publish_bot_command(
    state: &AppState,
    game_id: Uuid,
    player_position: i32,
    command: String,
    attempt: i32,
) -> Result<()> {
    let event = BotCommandEvent {
        game_id,
        player_position,
        command,
        attempt,
    };
    let payload = serde_json::to_vec(&event).context("Failed to serialize bot.command event")?;
    state
        .jetstream
        .publish(nats::SUBJECT_COMMAND, payload.into())
        .await
        .context("Failed to publish bot.command")?
        .await
        .context("Failed waiting for bot.command publish ack")?;
    Ok(())
}

struct BotContext {
    game_state: String,
    game_data: brdgme_game_client::GameData,
    recent_logs: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
async fn load_bot_context(
    state: &AppState,
    game_service_uri: &str,
    version_name: &str,
    game_id: uuid::Uuid,
    player_position: i32,
    game_player_id: uuid::Uuid,
    game_state: String,
    names: &[String],
    interface_version: i32,
) -> Result<BotContext> {
    let game_data = brdgme_game_client::fetch_game_data(
        &state.game_http,
        game_service_uri,
        version_name,
        game_state.clone(),
        player_position as usize,
        interface_version,
    )
    .await
    .context("Failed to fetch game data")?;

    let log_rows = sqlx::query(
        "SELECT body FROM game_logs \
         WHERE game_id = $1 \
           AND (is_public = true OR id IN ( \
               SELECT game_log_id FROM game_log_targets WHERE game_player_id = $2 \
           )) \
         ORDER BY logged_at DESC LIMIT 20",
    )
    .bind(game_id)
    .bind(game_player_id)
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
        game_data,
        recent_logs,
    })
}

fn build_messages(
    bot_cfg: &config::BotConfig,
    bot_ctx: &BotContext,
    names: &[String],
    player_position: usize,
    fallback_name: &str,
    failed_commands: Vec<FailedCommand>,
) -> Result<Vec<ChatMessage>> {
    let system_ctx = SystemContext {
        game_rules: bot_ctx.game_data.rules.clone(),
        include_basic_strategy: bot_cfg.include_basic_strategy,
        basic_strategy: bot_ctx.game_data.basic_strategy.clone(),
        include_advanced_strategy: bot_cfg.include_advanced_strategy,
        advanced_strategy: bot_ctx.game_data.advanced_strategy.clone(),
        data_docs: bot_ctx.game_data.data_docs.clone(),
    };
    let system_content = render_system(&system_ctx).context("Failed to render system prompt")?;

    let my_name = names
        .get(player_position)
        .map(|s| s.as_str())
        .unwrap_or(fallback_name)
        .to_string();
    let my_colour = format!("{}", LIGHT.player_color(player_position));
    let players = names
        .iter()
        .enumerate()
        .map(|(i, name)| PlayerInfo {
            name: name.clone(),
            colour: format!("{}", LIGHT.player_color(i)),
            score: bot_ctx.game_data.points.get(i).copied().unwrap_or(0.0),
        })
        .collect();
    let command_spec = bot_ctx
        .game_data
        .command_spec
        .as_ref()
        .map(spec_to_yaml)
        .unwrap_or_default();
    let user_ctx = UserContext {
        my_name,
        my_colour,
        players,
        pub_state_yaml: bot_ctx.game_data.pub_state_yaml.clone(),
        player_state_yaml: bot_ctx.game_data.player_state_yaml.clone(),
        command_spec,
        recent_logs: bot_ctx.recent_logs.clone(),
        failed_commands,
    };
    let user_content = render_user(&user_ctx).context("Failed to render user prompt")?;

    Ok(vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_content,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_content,
        },
    ])
}

/// Applies a JSON Merge Patch (RFC 7396) to `target` in place: keys set to
/// `null` in `patch` are removed from `target`, other keys are set/overwritten,
/// recursing when both sides hold an object at the same key.
fn merge_json_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    let Some(patch_map) = patch.as_object() else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = serde_json::Value::Object(serde_json::Map::new());
    }
    let target_map = target
        .as_object_mut()
        .expect("just ensured target is an object");
    for (key, patch_value) in patch_map {
        if patch_value.is_null() {
            target_map.remove(key);
            continue;
        }
        let target_entry = target_map
            .entry(key.clone())
            .or_insert(serde_json::Value::Null);
        if patch_value.is_object() && target_entry.is_object() {
            merge_json_patch(target_entry, patch_value);
        } else {
            *target_entry = patch_value.clone();
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn call_llm(
    http: &reqwest::Client,
    llm_url: &str,
    model: &str,
    messages: &[ChatMessage],
    api_key: Option<&str>,
    temperature: f32,
    reasoning_effort: Option<String>,
    extra_body: Option<&serde_json::Value>,
) -> Result<String> {
    let url = format!("{}/v1/chat/completions", llm_url);
    let body = ChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        stream: false,
        temperature,
        reasoning_effort,
    };

    let mut req = match extra_body {
        Some(patch) => {
            let mut value =
                serde_json::to_value(&body).context("Failed to serialize LLM request body")?;
            merge_json_patch(&mut value, patch);
            http.post(&url).json(&value)
        }
        None => http.post(&url).json(&body),
    };
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

async fn healthz(AxumState(state): AxumState<AppState>) -> StatusCode {
    match state.jetstream.client().connection_state() {
        async_nats::connection::State::Connected => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Serves `/healthz` on `LISTEN_ADDR`, reporting NATS connection health.
/// Spawned alongside the bot.turn consumer loop; the loop is not expected to
/// exit in normal operation, so this runs for the lifetime of the process.
async fn serve_health(state: AppState, listen_addr: String) -> Result<()> {
    let app = Router::new()
        .route("/healthz", get(healthz))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .with_context(|| format!("Failed to bind LISTEN_ADDR {}", listen_addr))?;
    tracing::info!(listen_addr = %listen_addr, "Bot health endpoint listening");
    axum::serve(listener, app)
        .await
        .context("Health server failed")
}

/// Waits for the monolith to have created the `BOT` stream and `bot-turn`
/// consumer (it does so idempotently on its own startup), retrying with a
/// short backoff since the bot and monolith can start in either order.
async fn wait_for_turn_consumer(
    jetstream: &async_nats::jetstream::Context,
) -> Result<async_nats::jetstream::consumer::PullConsumer> {
    loop {
        match jetstream
            .get_consumer_from_stream(nats::CONSUMER_TURN, nats::STREAM_NAME)
            .await
        {
            Ok(consumer) => return Ok(consumer),
            Err(e) => {
                tracing::warn!("bot-turn consumer not ready yet ({}), retrying in 2s", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let encryption_key = match crypto::load_key() {
        Ok(key) => Some(key),
        Err(e) => {
            tracing::warn!(
                "DATABASE_ENCRYPTION_KEY not loaded ({}); DB-stored provider API keys will be unavailable, env-var fallback only",
                e
            );
            None
        }
    };
    tracing::info!("Bot service starting");

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPool::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let nats_url = std::env::var("NATS_URL").context("NATS_URL must be set")?;
    let jetstream = nats::connect(&nats_url)
        .await
        .context("Failed to connect to NATS")?;

    // The LLM call can take minutes, so the client's overall timeout is generous;
    // shorter game-service calls override it with a per-request timeout.
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to build HTTP client")?;
    let game_http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to build game service HTTP client")?;
    let state = AppState {
        pool,
        http,
        game_http,
        encryption_key,
        jetstream: jetstream.clone(),
    };

    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());
    let health_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = serve_health(health_state, listen_addr).await {
            tracing::error!("Bot health endpoint failed: {}", e);
        }
    });

    let consumer = wait_for_turn_consumer(&jetstream).await?;
    let mut messages = consumer.messages().await?;

    tracing::info!("Bot subscribed to bot.turn, waiting for messages");

    while let Some(message) = messages.next().await {
        let message = match message {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to pull bot.turn message: {}", e);
                continue;
            }
        };
        let event: BotTurnEvent = match serde_json::from_slice(&message.payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to parse bot.turn payload: {}", e);
                if let Err(e) = message.ack().await {
                    tracing::warn!("Failed to ack unparseable bot.turn message: {}", e);
                }
                continue;
            }
        };

        let state = state.clone();
        tokio::spawn(async move {
            let trace_id = Uuid::new_v4();
            match run_bot_turn(&state, event, trace_id).await {
                Ok(()) => {
                    if let Err(e) = message.ack().await {
                        tracing::warn!(trace_id = %trace_id, "Failed to ack bot.turn message: {}", e);
                    }
                }
                Err(e) => {
                    // Leave unacked: JetStream redelivers after ack_wait, bounded by
                    // max_deliver as a poison-message backstop.
                    tracing::error!(trace_id = %trace_id, error = ?e, "Bot turn hard failed - game is stuck waiting for bot (will be redelivered)");
                }
            }
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_json_patch_empty_patch_is_noop() {
        let mut target = json!({"model": "gpt", "temperature": 0.2});
        let original = target.clone();
        merge_json_patch(&mut target, &json!({}));
        assert_eq!(target, original);
    }

    #[test]
    fn merge_json_patch_overwrites_key() {
        let mut target = json!({"model": "gpt", "temperature": 0.2});
        merge_json_patch(&mut target, &json!({"temperature": 0.5}));
        assert_eq!(target, json!({"model": "gpt", "temperature": 0.5}));
    }

    #[test]
    fn merge_json_patch_null_deletes_key() {
        let mut target = json!({"model": "gpt", "reasoning_effort": "low"});
        merge_json_patch(&mut target, &json!({"reasoning_effort": null}));
        assert_eq!(target, json!({"model": "gpt"}));
    }

    #[test]
    fn merge_json_patch_reasoning_effort_and_thinking() {
        let mut target = json!({
            "model": "deepseek-v4-flash",
            "reasoning_effort": "low",
        });
        merge_json_patch(
            &mut target,
            &json!({
                "reasoning_effort": null,
                "thinking": {"type": "disabled"},
            }),
        );
        assert_eq!(
            target,
            json!({
                "model": "deepseek-v4-flash",
                "thinking": {"type": "disabled"},
            })
        );
    }
}
