use std::collections::HashMap;

use axum::extract::State;

use crate::proposals::InviteMailer;
use crate::state::AppState;

pub fn parse_reply_commands(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            continue;
        }
        if trimmed.starts_with("On ") && trimmed.ends_with("wrote:") {
            break;
        }
        let t = line.trim();
        if t == "-- " || t == "--" {
            break;
        }
        if t.is_empty() {
            continue;
        }
        commands.push(t.to_string());
    }
    commands
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboundRoute {
    Game(String),
    Invite(String),
    Settings(String),
}

pub fn parse_reply_address(addr: &str) -> Option<InboundRoute> {
    let local = addr.split('@').next().unwrap_or(addr);
    let (tok, route) = if let Some(tok) = local.strip_prefix("g-") {
        (tok, InboundRoute::Game(tok.to_string()))
    } else if let Some(tok) = local.strip_prefix("i-") {
        (tok, InboundRoute::Invite(tok.to_string()))
    } else {
        let tok = local.strip_prefix("s-")?;
        (tok, InboundRoute::Settings(tok.to_string()))
    };
    if tok.is_empty() {
        return None;
    }
    Some(route)
}

pub fn extract_plain_text(raw: &str) -> Option<String> {
    let msg = mail_parser::MessageParser::default().parse(raw)?;
    msg.body_text(0).map(|s| s.to_string())
}

#[async_trait::async_trait]
pub trait InboundEmailSource: Send + Sync {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String>;
}

pub struct ResendInbound {
    pub api_key: String,
    pub http: reqwest::Client,
}

#[derive(serde::Deserialize)]
struct ResendEmailResponse {
    raw: ResendRaw,
}

#[derive(serde::Deserialize)]
struct ResendRaw {
    download_url: String,
}

#[async_trait::async_trait]
impl InboundEmailSource for ResendInbound {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String> {
        let url = format!("https://api.resend.com/emails/receiving/{email_id}");
        let resp: ResendEmailResponse = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let raw = self
            .http
            .get(&resp.raw.download_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(raw)
    }
}

pub struct StaticInbound(pub HashMap<String, String>);

#[async_trait::async_trait]
impl InboundEmailSource for StaticInbound {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String> {
        self.0
            .get(email_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("email not found: {email_id}"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("invalid secret")]
    InvalidSecret,
    #[error("missing header: {0}")]
    MissingHeader(&'static str),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("timestamp too old")]
    TimestampTooOld,
    #[error("timestamp in future")]
    FutureTimestamp,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("verification failed: {0}")]
    Other(String),
}

pub fn verify_webhook(
    secret: &str,
    msg_id: &str,
    signature: &str,
    timestamp: &str,
    raw_body: &[u8],
) -> Result<(), VerifyError> {
    use axum::http::HeaderValue;

    let webhook = svix::webhooks::Webhook::new(secret).map_err(|_| VerifyError::InvalidSecret)?;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("svix-id", HeaderValue::from_str(msg_id).unwrap());
    headers.insert("svix-timestamp", HeaderValue::from_str(timestamp).unwrap());
    headers.insert("svix-signature", HeaderValue::from_str(signature).unwrap());
    webhook.verify(raw_body, &headers).map_err(|e| match e {
        svix::webhooks::WebhookError::InvalidSecret(_)
        | svix::webhooks::WebhookError::EmptySecret => VerifyError::InvalidSecret,
        svix::webhooks::WebhookError::MissingHeader(_) => VerifyError::MissingHeader("svix"),
        svix::webhooks::WebhookError::InvalidSignature => VerifyError::InvalidSignature,
        svix::webhooks::WebhookError::TimestampTooOldError => VerifyError::TimestampTooOld,
        svix::webhooks::WebhookError::FutureTimestampError => VerifyError::FutureTimestamp,
        svix::webhooks::WebhookError::InvalidTimestamp => VerifyError::InvalidTimestamp,
        other => VerifyError::Other(other.to_string()),
    })
}

#[derive(serde::Deserialize)]
struct ResendEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: ResendInboundData,
}

#[derive(serde::Deserialize)]
struct ResendInboundData {
    email_id: String,
    from: String,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    received_for: Vec<String>,
}

/// First recipient address that parses to a routing token wins; `to` is checked
/// before `received_for`.
pub fn select_route(to: &[String], received_for: &[String]) -> Option<InboundRoute> {
    to.iter()
        .chain(received_for.iter())
        .find_map(|addr| parse_reply_address(addr))
}

pub enum CommandLoopOutcome<E> {
    AllExecuted(usize),
    Failed { index: usize, error: E },
}

/// Runs `commands` in order through `execute`, stopping at the first error.
pub async fn run_commands_in_order<F, Fut, E>(
    commands: &[String],
    mut execute: F,
) -> CommandLoopOutcome<E>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<(), E>>,
{
    for (index, cmd) in commands.iter().enumerate() {
        if let Err(error) = execute(cmd.clone()).await {
            return CommandLoopOutcome::Failed { index, error };
        }
    }
    CommandLoopOutcome::AllExecuted(commands.len())
}

pub fn confirmed_header_text(count: usize) -> String {
    match count {
        1 => "Move confirmed.".to_string(),
        n => format!("{n} moves confirmed."),
    }
}

pub fn no_command_header_text() -> String {
    "I could not find a command in your email.".to_string()
}

fn settings_response_header(error: Option<String>, last_status: Option<String>) -> String {
    if let Some(err) = error {
        err
    } else if let Some(status) = last_status {
        status
    } else {
        no_command_header_text()
    }
}

pub fn error_reply_text(err: &crate::game::ExecuteCommandError) -> String {
    use crate::game::ExecuteCommandError;
    match err {
        ExecuteCommandError::UserError(msg) => msg.clone(),
        ExecuteCommandError::Conflict => {
            "Your move could not be applied because the game changed; please try again.".to_string()
        }
        ExecuteCommandError::Other(_) => {
            "An unexpected error occurred while processing your move.".to_string()
        }
    }
}

struct EmailPlayer {
    game_player_id: uuid::Uuid,
    game_id: uuid::Uuid,
    user_id: uuid::Uuid,
    position: i32,
}

async fn find_game_player_by_email_token(
    pool: &sqlx::PgPool,
    token: &str,
) -> anyhow::Result<Option<EmailPlayer>> {
    let row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, i32)>(
        "SELECT id, game_id, user_id, position FROM game_players WHERE email_token = $1 AND user_id IS NOT NULL",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(game_player_id, game_id, user_id, position)| EmailPlayer {
            game_player_id,
            game_id,
            user_id,
            position,
        }),
    )
}

async fn from_matches_verified_email(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    from: &str,
) -> anyhow::Result<bool> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM user_emails WHERE user_id = $1 AND verified_at IS NOT NULL AND LOWER(email) = LOWER($2))",
    )
    .bind(user_id)
    .bind(from)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn resolve_user_by_verified_from(
    pool: &sqlx::PgPool,
    from: &str,
) -> anyhow::Result<Option<uuid::Uuid>> {
    let row = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM user_emails WHERE verified_at IS NOT NULL AND LOWER(email) = LOWER($1) LIMIT 1",
    )
    .bind(from)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Insert-or-skip idempotency marker. Returns true if THIS call inserted the row
/// (proceed); false if it already existed (a duplicate delivery -> skip).
async fn mark_event_processed(pool: &sqlx::PgPool, event_id: &str) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "INSERT INTO processed_webhook_events (event_id) VALUES ($1) ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(event_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

fn header_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// `POST /api/webhooks/resend` - Resend inbound-email webhook. Verifies the
/// svix signature, dedupes on `svix-id`, then routes the reply by its token.
pub async fn resend_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::http::StatusCode {
    use axum::http::StatusCode;

    let secret = match std::env::var("RESEND_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::error!("resend webhook: RESEND_WEBHOOK_SECRET is not configured");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let (Some(msg_id), Some(timestamp), Some(signature)) = (
        header_value(&headers, "svix-id"),
        header_value(&headers, "svix-timestamp"),
        header_value(&headers, "svix-signature"),
    ) else {
        tracing::warn!("resend webhook: missing svix headers");
        return StatusCode::UNAUTHORIZED;
    };

    if let Err(e) = verify_webhook(&secret, &msg_id, &signature, &timestamp, &body) {
        tracing::warn!("resend webhook: signature verification failed: {e}");
        return StatusCode::UNAUTHORIZED;
    }

    match mark_event_processed(&state.pool, &msg_id).await {
        Ok(true) => {}
        Ok(false) => return StatusCode::OK,
        Err(e) => {
            tracing::error!("resend webhook: idempotency check failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    let event: ResendEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("resend webhook: failed to parse event payload: {e}");
            return StatusCode::OK;
        }
    };
    if event.event_type != "email.received" {
        tracing::info!("resend webhook: ignoring event type {}", event.event_type);
        return StatusCode::OK;
    }

    match select_route(&event.data.to, &event.data.received_for) {
        Some(InboundRoute::Game(token)) => {
            handle_game_reply(&state, &token, &event.data.from, &event.data.email_id).await;
        }
        Some(InboundRoute::Invite(token)) => {
            handle_invite_reply(&state, &token, &event.data.from, &event.data.email_id).await;
        }
        Some(InboundRoute::Settings(_)) | None => {
            handle_settings_reply_route(&state, &event.data.from, &event.data.email_id).await;
        }
    }
    StatusCode::OK
}

async fn handle_game_reply(state: &AppState, token: &str, from: &str, email_id: &str) {
    let pool = &state.pool;

    let player = match find_game_player_by_email_token(pool, token).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::info!("resend webhook: unknown game token; no response");
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: token lookup failed: {e}");
            return;
        }
    };

    match from_matches_verified_email(pool, player.user_id, from).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!("resend webhook: From does not match a verified address; no response");
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: From verification failed: {e}");
            return;
        }
    }

    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::error!("resend webhook: RESEND_API_KEY not configured; cannot fetch body");
            return;
        }
    };
    let source = ResendInbound {
        api_key,
        http: state.http_client.clone(),
    };
    let raw = match source.fetch_raw_email(email_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("resend webhook: failed to fetch raw email {email_id}: {e}");
            return;
        }
    };
    let text = extract_plain_text(&raw).unwrap_or_default();
    let commands = parse_reply_commands(&text);

    if commands.is_empty() {
        send_game_reply_response(state, &player, token, from, no_command_header_text()).await;
        return;
    }

    let ctx = crate::email::commands::EmailCommandCtx {
        pool: &state.pool,
        http_client: &state.http_client,
        broadcaster: &state.broadcaster,
        jetstream: &state.jetstream,
        resend: state.resend.as_ref(),
        game_id: player.game_id,
        game_player_id: player.game_player_id,
        user_id: player.user_id,
        position: player.position as usize,
    };

    let mut move_count: usize = 0;
    let mut last_status: Option<String> = None;
    let mut full_content: Option<(String, String)> = None;
    let mut error_header: Option<String> = None;

    for line in &commands {
        match crate::email::commands::dispatch_email_command(&ctx, line).await {
            Ok(crate::email::commands::CommandReply::GameMove) => {
                move_count += 1;
            }
            Ok(crate::email::commands::CommandReply::Status(msg)) => {
                last_status = Some(msg);
            }
            Ok(crate::email::commands::CommandReply::FullContent { html, text }) => {
                full_content = Some((html, text));
                break;
            }
            Err(crate::email::commands::CommandError::User(msg)) => {
                error_header = Some(msg);
                break;
            }
            Err(crate::email::commands::CommandError::Internal(e)) => {
                tracing::error!("resend webhook: command error: {e}");
                error_header =
                    Some("An unexpected error occurred while processing your command.".to_string());
                break;
            }
        }
    }

    if let Some((html, text)) = full_content {
        send_rules_reply_response(state, &player, token, from, html, text).await;
        return;
    }

    let header = if let Some(err) = error_header {
        err
    } else if move_count > 0 {
        confirmed_header_text(move_count)
    } else if let Some(status) = last_status {
        status
    } else {
        no_command_header_text()
    };

    send_game_reply_response(state, &player, token, from, header).await;
}

async fn handle_invite_reply(state: &AppState, token: &str, from: &str, email_id: &str) {
    let pool = &state.pool;

    let player = match crate::proposals::find_proposal_player_by_email_token(pool, token).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::info!("resend webhook: unknown invite token; no response");
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite token lookup failed: {e}");
            return;
        }
    };

    let Some(user_id) = player.user_id else {
        tracing::info!("resend webhook: invite token belongs to a bot slot; no response");
        return;
    };

    match from_matches_verified_email(pool, user_id, from).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                "resend webhook: invite From does not match a verified address; no response"
            );
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite From verification failed: {e}");
            return;
        }
    }

    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::error!("resend webhook: RESEND_API_KEY not configured; cannot fetch body");
            return;
        }
    };
    let source = ResendInbound {
        api_key,
        http: state.http_client.clone(),
    };
    let raw = match source.fetch_raw_email(email_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("resend webhook: failed to fetch raw email {email_id}: {e}");
            return;
        }
    };
    let text = extract_plain_text(&raw).unwrap_or_default();
    let commands = parse_reply_commands(&text);

    let accept = commands.iter().any(|c| c.eq_ignore_ascii_case("accept"));
    let decline = commands.iter().any(|c| c.eq_ignore_ascii_case("decline"));

    if !accept && !decline {
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            no_command_header_text(),
            None,
        )
        .await;
        return;
    }

    let proposal_id = player.proposal_id;
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("resend webhook: invite begin tx failed: {e}");
            return;
        }
    };

    let proposal = match crate::proposals::lock_proposal_for_update(&mut tx, proposal_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!("resend webhook: proposal {proposal_id} not found");
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite lock proposal failed: {e}");
            return;
        }
    };

    if proposal.status != "open" {
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "This invite is no longer open.".to_string(),
            None,
        )
        .await;
        return;
    }

    let players = match crate::proposals::find_proposal_players_tx(&mut tx, proposal_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("resend webhook: invite players lookup failed: {e}");
            return;
        }
    };

    let me = match players.iter().find(|p| p.id == player.id) {
        Some(p) => p,
        None => return,
    };

    if me.response != "pending" {
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "That invite has already been responded to.".to_string(),
            None,
        )
        .await;
        return;
    }

    let response = if accept { "accepted" } else { "declined" };
    if let Err(e) =
        crate::proposals::update_proposal_player_response(&mut tx, player.id, response).await
    {
        tracing::error!("resend webhook: invite update response failed: {e}");
        return;
    }

    let mut started_game_id: Option<uuid::Uuid> = None;
    let mut roster: Vec<crate::proposals::ProposalPlayer> = Vec::new();

    if accept {
        let pending =
            match crate::proposals::count_pending_human_invitees_tx(&mut tx, proposal_id).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!("resend webhook: invite count pending failed: {e}");
                    return;
                }
            };
        if pending == 0 {
            let game_version =
                match crate::db::find_game_version(&state.pool, proposal.game_version_id).await {
                    Ok(Some(gv)) => gv,
                    Ok(None) => {
                        tracing::error!("resend webhook: game version not found for proposal");
                        return;
                    }
                    Err(e) => {
                        tracing::error!("resend webhook: invite game version lookup failed: {e}");
                        return;
                    }
                };
            roster = match crate::proposals::find_proposal_players_tx(&mut tx, proposal_id).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("resend webhook: invite roster lookup failed: {e}");
                    return;
                }
            };
            match crate::proposals::start_proposal_tx(
                &mut tx,
                &state.http_client,
                &proposal,
                &roster,
                &game_version,
            )
            .await
            {
                Ok(gid) => started_game_id = Some(gid),
                Err(e) => {
                    tracing::error!("resend webhook: invite start proposal failed: {e}");
                    return;
                }
            }
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("resend webhook: invite commit failed: {e}");
        return;
    }

    state
        .broadcaster
        .broadcast_proposal_update(proposal_id)
        .await;

    if let Some(gid) = started_game_id {
        crate::game::broadcast_and_trigger(&state.pool, &state.broadcaster, &state.jetstream, gid)
            .await;
        let invitee_ids: Vec<uuid::Uuid> = roster
            .iter()
            .filter(|p| p.response == "accepted")
            .filter_map(|p| p.user_id)
            .filter(|id| *id != proposal.owner_user_id)
            .collect();
        crate::proposals::mailer_from(state.pool.clone(), state.resend.clone()).notify_started(
            proposal_id,
            gid,
            invitee_ids,
        );
    } else if !accept {
        crate::proposals::mailer_from(state.pool.clone(), state.resend.clone())
            .notify_owner_decline(proposal_id, user_id);
    }

    let header = if accept {
        if started_game_id.is_some() {
            "Invite accepted. The game has started!".to_string()
        } else {
            "Invite accepted.".to_string()
        }
    } else {
        "Invite declined.".to_string()
    };

    send_invite_reply_response(state, &player, user_id, from, header, started_game_id).await;
}

async fn send_invite_reply_response(
    state: &AppState,
    player: &crate::proposals::ProposalPlayer,
    user_id: uuid::Uuid,
    from: &str,
    header: String,
    game_id: Option<uuid::Uuid>,
) {
    let pool = &state.pool;
    let proposal_id = player.proposal_id;

    let theme_slug = match crate::db::get_user_theme(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("resend webhook: invite theme lookup failed: {e}");
            None
        }
    };

    let (game_type_name, game_version_id) =
        match crate::proposals::find_proposal(pool, proposal_id).await {
            Ok(Some(proposal)) => {
                let gvid = proposal.game_version_id;
                match crate::db::find_game_version(pool, proposal.game_version_id).await {
                    Ok(Some(gv)) => (
                        crate::proposals::find_game_type_name(pool, gv.game_type_id)
                            .await
                            .unwrap_or(None)
                            .unwrap_or_default(),
                        Some(gvid),
                    ),
                    _ => (String::new(), Some(gvid)),
                }
            }
            _ => (String::new(), None),
        };

    let base = crate::config::public_base_url();
    let browser_url = match game_id {
        Some(gid) => format!("{base}/games/{gid}"),
        None => format!("{base}/invites/{proposal_id}"),
    };

    let palette = crate::email::render::palette_for_slug(theme_slug.as_deref());
    let content = crate::email::render::EmailContent {
        subject: format!("{game_type_name} invite"),
        header: Some(header),
        digest: None,
        board: None,
        you_can: None,
        browser_url: Some(browser_url),
        rules_url: game_version_id.map(crate::email::notify::rules_url),
        footer: Some("Reply to this email to respond, or unsubscribe anytime.".to_string()),
    };
    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &[],
        Some(&format!("proposal-{proposal_id}")),
        false,
        &format!("i-{}@brdg.me", player.email_token.as_deref().unwrap_or("")),
    );
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

async fn send_game_reply_response(
    state: &AppState,
    player: &EmailPlayer,
    token: &str,
    from: &str,
    header: String,
) {
    let pool = &state.pool;
    let ge = match crate::db::find_game_extended(pool, player.game_id).await {
        Ok(Some(ge)) => ge,
        Ok(None) => {
            tracing::warn!(
                "resend webhook: game {} not found for response",
                player.game_id
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                "resend webhook: failed to load game {} for response: {e}",
                player.game_id
            );
            return;
        }
    };
    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == player.game_player_id)
    {
        Some(p) => p,
        None => {
            tracing::warn!(
                "resend webhook: player {} not in game {}",
                player.game_player_id,
                player.game_id
            );
            return;
        }
    };
    let (board, you_can) = crate::email::notify::render_board_and_you_can(
        &state.http_client,
        &ge,
        player.position as usize,
    )
    .await;
    let content = crate::email::render::EmailContent {
        subject: crate::email::notify::game_subject(&ge, recipient_player),
        header: Some(header),
        digest: None,
        board,
        you_can,
        browser_url: Some(crate::email::notify::browser_url(ge.game.id)),
        rules_url: Some(crate::email::notify::rules_url(ge.game_version.id)),
        footer: Some("Reply to this email to play, or unsubscribe anytime.".to_string()),
    };
    let theme_slug =
        match crate::email::outbound::fetch_email_recipient(pool, player.game_player_id).await {
            Ok(Some(r)) => r.theme_slug,
            _ => None,
        };
    let palette = crate::email::render::palette_for_slug(theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();
    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        Some(&format!("game-{}", ge.game.id)),
        false,
        &crate::email::notify::reply_address(token),
    );
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

async fn send_rules_reply_response(
    state: &AppState,
    player: &EmailPlayer,
    token: &str,
    from: &str,
    html: String,
    text: String,
) {
    let pool = &state.pool;
    let theme_slug =
        match crate::email::outbound::fetch_email_recipient(pool, player.game_player_id).await {
            Ok(Some(r)) => r.theme_slug,
            _ => None,
        };
    let palette = crate::email::render::palette_for_slug(theme_slug.as_deref());
    let bg = palette.background.hex();
    let fg = palette.foreground.hex();

    let full_html = format!(
        "<html><body style=\"background-color:{bg};color:{fg};font-family:sans-serif;padding:16px;\">{html}</body></html>"
    );

    let mut headers = std::collections::BTreeMap::new();
    let msg_id = format!("<game-{}@brdg.me>", player.game_id);
    headers.insert("In-Reply-To".to_string(), msg_id.clone());
    headers.insert("References".to_string(), msg_id);
    headers.insert(
        "List-Unsubscribe".to_string(),
        "<mailto:unsubscribe@brdg.me?subject=unsubscribe>".to_string(),
    );
    headers.insert(
        "List-Unsubscribe-Post".to_string(),
        "List-Unsubscribe=One-Click".to_string(),
    );

    let rendered = crate::email::render::RenderedEmail {
        subject: "Rules".to_string(),
        text,
        html: full_html,
        headers,
        reply_to: crate::email::notify::reply_address(token),
    };
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

async fn handle_settings_reply_route(state: &AppState, from: &str, email_id: &str) {
    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::error!(
                "resend webhook: RESEND_API_KEY not configured; cannot fetch settings body"
            );
            return;
        }
    };
    let source = ResendInbound {
        api_key,
        http: state.http_client.clone(),
    };
    let raw = match source.fetch_raw_email(email_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("resend webhook: failed to fetch raw email {email_id}: {e}");
            return;
        }
    };
    handle_settings_reply(state, from, &raw).await;
}

async fn handle_settings_reply(state: &AppState, from: &str, raw_body: &str) {
    let user_id = match resolve_user_by_verified_from(&state.pool, from).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::info!(
                "resend webhook: settings reply from unverified/unknown address; no response"
            );
            return;
        }
        Err(e) => {
            tracing::error!("resend webhook: settings From resolution failed: {e}");
            return;
        }
    };

    let text = extract_plain_text(raw_body).unwrap_or_default();
    let commands = parse_reply_commands(&text);

    if commands.is_empty() {
        send_settings_response(
            &state.pool,
            state.resend.as_ref(),
            user_id,
            from,
            no_command_header_text(),
        )
        .await;
        return;
    }

    let sctx = crate::email::commands::StandaloneCommandCtx {
        pool: &state.pool,
        http_client: &state.http_client,
        broadcaster: &state.broadcaster,
        jetstream: &state.jetstream,
        resend: state.resend.as_ref(),
        user_id,
    };

    let mut last_status: Option<String> = None;
    let mut error_header: Option<String> = None;

    for line in &commands {
        match crate::email::commands::dispatch_standalone_server_command(&sctx, line).await {
            Ok(crate::email::commands::CommandReply::Status(msg)) => {
                last_status = Some(msg);
            }
            Ok(_) => {}
            Err(crate::email::commands::CommandError::User(msg)) => {
                error_header = Some(msg);
                break;
            }
            Err(crate::email::commands::CommandError::Internal(e)) => {
                tracing::error!("resend webhook: settings command error: {e}");
                error_header =
                    Some("An unexpected error occurred while processing your command.".to_string());
                break;
            }
        }
    }

    let header = settings_response_header(error_header, last_status);
    send_settings_response(&state.pool, state.resend.as_ref(), user_id, from, header).await;
}

async fn send_settings_response(
    pool: &sqlx::PgPool,
    resend: Option<&resend_rs::Resend>,
    user_id: uuid::Uuid,
    from: &str,
    header: String,
) {
    let theme_slug = match crate::db::get_user_theme(pool, user_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("resend webhook: settings theme lookup failed: {e}");
            None
        }
    };
    let palette = crate::email::render::palette_for_slug(theme_slug.as_deref());
    let reply_address = format!("s-{user_id}@brdg.me");
    let thread_id = format!("settings-{user_id}");
    let content = crate::email::render::EmailContent {
        subject: "Your brdg.me settings".to_string(),
        header: Some(header),
        digest: None,
        board: None,
        you_can: None,
        browser_url: None,
        rules_url: None,
        footer: Some(
            "Reply to this email to change your settings, or send 'help' for the command list."
                .to_string(),
        ),
    };
    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &[],
        Some(&thread_id),
        false,
        &reply_address,
    );
    crate::email::outbound::send_rendered_email(resend, rendered, from).await;
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn parse_reply_commands_clean_single() {
        assert_eq!(parse_reply_commands("play e4"), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_strips_quoted_lines() {
        let input = "play d4\n> previous move was e4\n> another quote";
        assert_eq!(parse_reply_commands(input), vec!["play d4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_on_wrote() {
        let input = "play e4\nOn Mon, Jul 20, 2026 at 10:00 AM Alice wrote:\n> play d4";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_signature() {
        let input = "play e4\n-- \nSent from my phone";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);

        let input2 = "play e4\n--\nSent from my phone";
        assert_eq!(parse_reply_commands(input2), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_multiple_in_order() {
        let input = "play e4\nplay d5\nresign";
        assert_eq!(
            parse_reply_commands(input),
            vec!["play e4", "play d5", "resign"]
        );
    }

    #[test]
    fn parse_reply_commands_drops_blank_lines() {
        let input = "play e4\n\n   \nplay d5";
        assert_eq!(parse_reply_commands(input), vec!["play e4", "play d5"]);
    }

    #[test]
    fn parse_reply_commands_keeps_arguments() {
        assert_eq!(parse_reply_commands("play e4 to e5"), vec!["play e4 to e5"]);
    }

    #[test]
    fn parse_reply_commands_empty_input() {
        assert_eq!(parse_reply_commands(""), Vec::<String>::new());
    }

    #[test]
    fn parse_reply_address_game() {
        assert_eq!(
            parse_reply_address("g-abc@brdg.me"),
            Some(InboundRoute::Game("abc".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_invite() {
        assert_eq!(
            parse_reply_address("i-xyz@example.com"),
            Some(InboundRoute::Invite("xyz".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_settings() {
        assert_eq!(
            parse_reply_address("s-tok@anything"),
            Some(InboundRoute::Settings("tok".to_string()))
        );
    }

    #[test]
    fn parse_reply_address_no_prefix() {
        assert_eq!(parse_reply_address("hello@brdg.me"), None);
    }

    #[test]
    fn parse_reply_address_bare_no_at() {
        assert_eq!(parse_reply_address("hello"), None);
    }

    #[test]
    fn parse_reply_address_empty_token() {
        assert_eq!(parse_reply_address("g-@x.com"), None);
    }

    #[test]
    fn extract_plain_text_multipart() {
        let raw = "MIME-Version: 1.0\r\n\
Content-Type: multipart/alternative; boundary=\"BOUNDARY\"\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hello plain world\r\n\
--BOUNDARY\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>Hello html world</p>\r\n\
--BOUNDARY--\r\n";
        assert_eq!(
            extract_plain_text(raw),
            Some("Hello plain world".to_string())
        );
    }

    #[test]
    fn extract_plain_text_single_part() {
        let raw = "MIME-Version: 1.0\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Just a plain body";
        assert_eq!(
            extract_plain_text(raw),
            Some("Just a plain body".to_string())
        );
    }

    #[test]
    fn verify_webhook_valid() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new(secret).unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        assert!(verify_webhook(secret, "msg_123", &sig, &ts.to_string(), body).is_ok());
    }

    #[test]
    fn verify_webhook_tampered_body() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new(secret).unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        let tampered = b"{\"test\": false}";
        assert!(verify_webhook(secret, "msg_123", &sig, &ts.to_string(), tampered).is_err());
    }

    #[test]
    fn verify_webhook_wrong_secret() {
        let body = b"{\"test\": true}";
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let wh = svix::webhooks::Webhook::new("whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw").unwrap();
        let sig = wh.sign("msg_123", ts, body).unwrap();
        assert!(
            verify_webhook(
                "whsec_C2FVsBQIhrscChlQIMV+b5sSYspob7oD",
                "msg_123",
                &sig,
                &ts.to_string(),
                body
            )
            .is_err()
        );
    }

    #[test]
    fn select_route_prefers_to_then_received_for() {
        let to = vec!["g-aaa@brdg.me".to_string()];
        let rf = vec!["g-bbb@brdg.me".to_string()];
        assert_eq!(
            select_route(&to, &rf),
            Some(InboundRoute::Game("aaa".to_string()))
        );
        let to2 = vec!["hello@brdg.me".to_string()];
        assert_eq!(
            select_route(&to2, &rf),
            Some(InboundRoute::Game("bbb".to_string()))
        );
    }

    #[test]
    fn select_route_none_when_unparseable() {
        let to = vec!["nope@brdg.me".to_string()];
        let rf = vec!["also-nope@example.com".to_string()];
        assert_eq!(select_route(&to, &rf), None);
        assert_eq!(select_route(&[], &[]), None);
    }

    #[test]
    fn select_route_routes_invite_and_settings() {
        assert_eq!(
            select_route(&["i-xyz@brdg.me".to_string()], &[]),
            Some(InboundRoute::Invite("xyz".to_string()))
        );
        assert_eq!(
            select_route(&[], &["s-tok@brdg.me".to_string()]),
            Some(InboundRoute::Settings("tok".to_string()))
        );
    }

    #[tokio::test]
    async fn run_commands_all_succeed() {
        let cmds: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        let outcome: CommandLoopOutcome<String> =
            run_commands_in_order(&cmds, |_cmd| async { Ok::<(), String>(()) }).await;
        match outcome {
            CommandLoopOutcome::AllExecuted(n) => assert_eq!(n, 3),
            CommandLoopOutcome::Failed { .. } => panic!("expected success"),
        }
    }

    #[tokio::test]
    async fn run_commands_stops_at_first_error() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let cmds: Vec<String> = vec!["ok".into(), "bad".into(), "never".into()];
        let seen = Arc::new(AtomicUsize::new(0));
        let seen2 = seen.clone();
        let outcome: CommandLoopOutcome<String> = run_commands_in_order(&cmds, move |cmd| {
            let seen2 = seen2.clone();
            async move {
                seen2.fetch_add(1, Ordering::SeqCst);
                if cmd == "bad" {
                    Err("boom".to_string())
                } else {
                    Ok(())
                }
            }
        })
        .await;
        match outcome {
            CommandLoopOutcome::Failed { index, error } => {
                assert_eq!(index, 1);
                assert_eq!(error, "boom");
            }
            CommandLoopOutcome::AllExecuted(_) => panic!("expected failure"),
        }
        assert_eq!(seen.load(Ordering::SeqCst), 2); // stopped before "never"
    }

    #[tokio::test]
    async fn run_commands_empty_list() {
        let cmds: Vec<String> = vec![];
        let outcome: CommandLoopOutcome<String> =
            run_commands_in_order(&cmds, |_cmd| async { Ok::<(), String>(()) }).await;
        match outcome {
            CommandLoopOutcome::AllExecuted(n) => assert_eq!(n, 0),
            CommandLoopOutcome::Failed { .. } => panic!("expected success"),
        }
    }

    #[test]
    fn confirmed_header_text_singular_and_plural() {
        assert_eq!(confirmed_header_text(1), "Move confirmed.");
        assert_eq!(confirmed_header_text(3), "3 moves confirmed.");
    }

    #[test]
    fn no_command_header_text_mentions_command() {
        assert!(no_command_header_text().contains("command"));
    }

    #[test]
    fn error_reply_text_maps_each_variant() {
        use crate::game::ExecuteCommandError;
        assert_eq!(
            error_reply_text(&ExecuteCommandError::UserError("nope".to_string())),
            "nope"
        );
        assert!(error_reply_text(&ExecuteCommandError::Conflict).contains("changed"));
        assert!(
            error_reply_text(&ExecuteCommandError::Other(anyhow::anyhow!("boom")))
                .contains("unexpected")
        );
    }

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40). Plain queries throughout to avoid `.sqlx` churn.
    async fn seed_game_with_player(
        pool: &sqlx::PgPool,
        token: &str,
    ) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Test Game {}", uuid::Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let game_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, false, 'initial') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let user_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("player")
        .bind(Vec::<String>::new())
        .fetch_one(pool)
        .await
        .unwrap();
        let game_player_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
             (game_id, user_id, position, color, has_accepted, is_turn,
              is_turn_at, last_turn_at, is_eliminated, is_read, email_token)
         VALUES ($1, $2, 0, 'Green', true, false, NOW(), NOW(), false, false, $3)
         RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .bind(token)
        .fetch_one(pool)
        .await
        .unwrap();
        (game_id, user_id, game_player_id)
    }

    #[sqlx::test]
    async fn find_game_player_by_email_token_lookup(pool: sqlx::PgPool) {
        let (game_id, user_id, game_player_id) = seed_game_with_player(&pool, "tok-found").await;
        let p = find_game_player_by_email_token(&pool, "tok-found")
            .await
            .unwrap()
            .expect("expected a player");
        assert_eq!(p.game_id, game_id);
        assert_eq!(p.user_id, user_id);
        assert_eq!(p.game_player_id, game_player_id);
        assert_eq!(p.position, 0);
        assert!(
            find_game_player_by_email_token(&pool, "tok-missing")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn mark_event_processed_dedups(pool: sqlx::PgPool) {
        assert!(mark_event_processed(&pool, "evt-1").await.unwrap());
        assert!(!mark_event_processed(&pool, "evt-1").await.unwrap());
        assert!(mark_event_processed(&pool, "evt-2").await.unwrap());
    }

    #[sqlx::test]
    async fn from_matches_verified_email_truth_table(pool: sqlx::PgPool) {
        let (_game_id, user_id, _gp) = seed_game_with_player(&pool, "tok-from").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind("verified@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NULL)",
        )
        .bind(user_id)
        .bind("unverified@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        let other_user: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("other")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(other_user)
        .bind("other@brdg.me")
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            from_matches_verified_email(&pool, user_id, "verified@brdg.me")
                .await
                .unwrap()
        );
        assert!(
            from_matches_verified_email(&pool, user_id, "VERIFIED@brdg.me")
                .await
                .unwrap()
        );
        assert!(
            !from_matches_verified_email(&pool, user_id, "unverified@brdg.me")
                .await
                .unwrap()
        );
        assert!(
            !from_matches_verified_email(&pool, user_id, "other@brdg.me")
                .await
                .unwrap()
        );
        assert!(
            !from_matches_verified_email(&pool, user_id, "nobody@brdg.me")
                .await
                .unwrap()
        );
    }

    #[test]
    fn settings_response_header_error_wins() {
        assert_eq!(
            settings_response_header(Some("err".to_string()), Some("status".to_string())),
            "err"
        );
    }

    #[test]
    fn settings_response_header_status_when_no_error() {
        assert_eq!(
            settings_response_header(None, Some("status".to_string())),
            "status"
        );
    }

    #[test]
    fn settings_response_header_fallback_when_both_none() {
        assert_eq!(
            settings_response_header(None, None),
            no_command_header_text()
        );
    }

    async fn seed_user(pool: &sqlx::PgPool, name: &str) -> uuid::Uuid {
        sqlx::query_scalar("INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn resolve_user_by_verified_from_truth_table(pool: sqlx::PgPool) {
        let user_a = seed_user(&pool, "user-a").await;
        let user_b = seed_user(&pool, "user-b").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_a)
        .bind("a@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NULL)",
        )
        .bind(user_a)
        .bind("unv@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_b)
        .bind("b@brdg.me")
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            resolve_user_by_verified_from(&pool, "a@brdg.me")
                .await
                .unwrap(),
            Some(user_a)
        );
        assert_eq!(
            resolve_user_by_verified_from(&pool, "A@brdg.me")
                .await
                .unwrap(),
            Some(user_a)
        );
        assert_eq!(
            resolve_user_by_verified_from(&pool, "unv@brdg.me")
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            resolve_user_by_verified_from(&pool, "nobody@brdg.me")
                .await
                .unwrap(),
            None
        );
    }

    #[sqlx::test]
    async fn settings_standalone_rejects_game_command(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "settings-user").await;
        match crate::email::commands::dispatch_settings_standalone(&pool, None, user_id, "concede")
            .await
        {
            Err(crate::email::commands::CommandError::User(msg)) => {
                assert!(msg.contains("not available"));
            }
            _ => panic!("expected User error for game command"),
        }
        match crate::email::commands::dispatch_settings_standalone(&pool, None, user_id, "settings")
            .await
        {
            Ok(crate::email::commands::CommandReply::Status(_)) => {}
            _ => panic!("expected Status reply for settings command"),
        }
    }

    #[sqlx::test]
    async fn resolve_user_by_verified_from_is_the_should_respond_gate(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "gate-user").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind("gate@brdg.me")
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            resolve_user_by_verified_from(&pool, "gate@brdg.me")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            resolve_user_by_verified_from(&pool, "unknown@brdg.me")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn find_user_id_by_name_resolves_case_insensitive(pool: sqlx::PgPool) {
        let user_a = seed_user(&pool, "user-a").await;
        let _user_b = seed_user(&pool, "user-b").await;

        assert_eq!(
            crate::db::find_user_id_by_name(&pool, "USER-A")
                .await
                .unwrap(),
            Some(user_a)
        );
        assert_eq!(
            crate::db::find_user_id_by_name(&pool, "user-a")
                .await
                .unwrap(),
            Some(user_a)
        );
        assert_eq!(
            crate::db::find_user_id_by_name(&pool, "nobody")
                .await
                .unwrap(),
            None
        );
    }
}
