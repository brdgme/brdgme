use std::collections::HashMap;

use axum::extract::State;

use crate::proposals::InviteMailer;
use crate::state::AppState;

/// Splits an inbound reply body into command lines, dropping the quoted
/// original and everything after the attribution or signature.
///
/// Stop conditions (wfe F6):
/// 1. a single-line `On ... wrote:` attribution (the pre-existing rule) - stop;
/// 2. Outlook's unquoted `-----Original Message-----` separator - stop;
/// 3. an Outlook-style header line (`From:`, `Sent:`, `To:`, `Subject:`,
///    `Cc:`, `Date:`) - stop. Safe because no game grammar in the repo has a
///    colon-terminated top-level token;
/// 4. a `--` / `-- ` signature marker (the pre-existing rule) - stop;
/// 5. at the FIRST `>`-quoted line, retract the block of already-collected
///    lines since the last blank line if any of them looks like an attribution
///    (ends with `:`, or carries a `<...@...>` address). This is the
///    language-independent rule and it is what catches Gmail's two-line wrapped
///    attribution and localized clients that never write `wrote:`. Quoted lines
///    themselves are still skipped rather than terminating the scan, so a
///    command typed below a quote block still works, exactly as before.
///
/// Known limits: an attribution block that is NOT preceded by a blank line will
/// take the sender's last command with it (they are indistinguishable), and an
/// attribution that neither ends with `:` nor carries an address is not
/// detected.
pub fn parse_reply_commands(text: &str) -> Vec<String> {
    const HEADER_PREFIXES: [&str; 6] = ["from:", "sent:", "to:", "subject:", "cc:", "date:"];

    fn looks_like_attribution(line: &str) -> bool {
        if line.ends_with(':') {
            return true;
        }
        match (line.find('<'), line.rfind('>')) {
            (Some(open), Some(close)) if close > open => line[open..close].contains('@'),
            _ => false,
        }
    }

    let mut commands: Vec<String> = Vec::new();
    let mut block_start = 0usize;
    let mut retracted = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            if !retracted {
                retracted = true;
                if commands[block_start..]
                    .iter()
                    .any(|c| looks_like_attribution(c))
                {
                    commands.truncate(block_start);
                }
                block_start = commands.len();
            }
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
            block_start = commands.len();
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if lower.starts_with("-----original message") {
            break;
        }
        if HEADER_PREFIXES.iter().any(|p| lower.starts_with(p)) {
            break;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteIntent {
    Accept,
    Decline,
}

/// The sender's invite response: the FIRST command line that is exactly
/// `accept` or `decline` (ASCII-case-insensitive), so a body mentioning both
/// honours what was stated first instead of always accepting (wfe F15).
/// Matching is whole-line, as before: `decline politely` matches nothing.
pub fn parse_invite_intent(commands: &[String]) -> Option<InviteIntent> {
    commands.iter().find_map(|c| {
        if c.eq_ignore_ascii_case("accept") {
            Some(InviteIntent::Accept)
        } else if c.eq_ignore_ascii_case("decline") {
            Some(InviteIntent::Decline)
        } else {
            None
        }
    })
}

pub fn extract_plain_text(raw: &str) -> Option<String> {
    let msg = mail_parser::MessageParser::default().parse(raw)?;
    msg.body_text(0).map(|s| s.to_string())
}

pub fn extract_addr_spec(value: &str) -> Option<String> {
    // A fold (CR/LF followed by whitespace) is one logical header line: replace
    // it with a space so an address after the fold survives. A bare CR/LF
    // terminates the header, so stop there to keep injection rejection.
    let mut sanitized = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' || ch == '\n' {
            while matches!(chars.peek(), Some('\r' | '\n')) {
                chars.next();
            }
            if matches!(chars.peek(), Some(c) if c.is_whitespace()) {
                sanitized.push(' ');
            } else {
                break;
            }
        } else {
            sanitized.push(ch);
        }
    }
    let value = sanitized.trim();
    if value.is_empty() {
        return None;
    }

    if !value.contains(['<', '>', '"', '(', ')', ',', ':', ';'])
        && !value.contains(char::is_whitespace)
        && value.matches('@').count() == 1
    {
        return Some(value.to_string());
    }

    let raw = format!("From: {value}\r\n\r\n");
    let msg = mail_parser::MessageParser::default().parse(raw.as_bytes())?;
    let addr = msg.from()?.first()?.address()?.trim();
    if addr.is_empty() || !addr.contains('@') {
        return None;
    }
    Some(addr.to_string())
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthVerdict {
    Pass,
    Fail,
}

pub fn classify_inbound_auth(raw: &str) -> AuthVerdict {
    let msg = match mail_parser::MessageParser::default().parse(raw) {
        Some(msg) => msg,
        None => return AuthVerdict::Fail,
    };

    // Trust only the topmost Authentication-Results header; lower ones may be forged.
    let header = match msg
        .headers()
        .iter()
        .find(|h| h.name().eq_ignore_ascii_case("Authentication-Results"))
    {
        Some(h) => h,
        None => return AuthVerdict::Fail,
    };

    let value = match header.value().as_text() {
        Some(v) => v,
        None => return AuthVerdict::Fail,
    };

    let mut parts = value.split(';');
    let authserv_id = parts.next().unwrap_or("").trim();
    if !authserv_id.eq_ignore_ascii_case("amazonses.com") {
        return AuthVerdict::Fail;
    }

    let mut spf: Option<String> = None;
    let mut dkim: Option<String> = None;
    let mut dmarc: Option<String> = None;
    for part in parts {
        let (method, result) = match part.trim().split_once('=') {
            Some((m, r)) => (m.trim(), r.trim()),
            None => continue,
        };
        let result = result
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if method.eq_ignore_ascii_case("spf") && spf.is_none() {
            spf = Some(result);
        } else if method.eq_ignore_ascii_case("dkim") && dkim.is_none() {
            dkim = Some(result);
        } else if method.eq_ignore_ascii_case("dmarc") && dmarc.is_none() {
            dmarc = Some(result);
        }
    }

    let is = |r: &Option<String>, want: &str| r.as_deref() == Some(want);
    if is(&dmarc, "fail") {
        return AuthVerdict::Fail;
    }
    if is(&spf, "pass") || is(&dkim, "pass") {
        AuthVerdict::Pass
    } else {
        AuthVerdict::Fail
    }
}

#[async_trait::async_trait]
pub trait InboundEmailSource: Send + Sync {
    async fn fetch_raw_email(&self, email_id: &str) -> anyhow::Result<String>;
}

pub struct ResendInbound {
    /// `None` (or empty) when `RESEND_API_KEY` is unset; the fetch then fails at
    /// call time (see `fetch_raw_email`), preserving lookup-first behavior.
    pub api_key: Option<String>,
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
        let api_key = self
            .api_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or_else(|| anyhow::anyhow!("RESEND_API_KEY not configured"))?;
        let url = format!("https://api.resend.com/emails/receiving/{email_id}");
        let resp: ResendEmailResponse = self
            .http
            .get(&url)
            .bearer_auth(api_key)
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
    #[error("invalid header value")]
    InvalidHeaderValue,
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
    headers.insert(
        "svix-id",
        HeaderValue::from_str(msg_id).map_err(|_| VerifyError::InvalidHeaderValue)?,
    );
    headers.insert(
        "svix-timestamp",
        HeaderValue::from_str(timestamp).map_err(|_| VerifyError::InvalidHeaderValue)?,
    );
    headers.insert(
        "svix-signature",
        HeaderValue::from_str(signature).map_err(|_| VerifyError::InvalidHeaderValue)?,
    );
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
/// before `received_for`. Each candidate goes through `extract_addr_spec` first
/// so `"brdg.me <g-tok@brdg.me>"` routes the same as `"g-tok@brdg.me"`
/// (wfe F4); an unparseable value is still tried verbatim.
pub fn select_route(to: &[String], received_for: &[String]) -> Option<InboundRoute> {
    to.iter().chain(received_for.iter()).find_map(|addr| {
        let bare = extract_addr_spec(addr).unwrap_or_else(|| addr.to_string());
        parse_reply_address(&bare)
    })
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

/// Builds the header block for a command-failure report email. Layout (each a
/// line, rendered above the board):
///
/// ```text
/// Your command failed: <reason>
/// Commands applied:        <- whole section omitted when `applied` is empty
///   <command>...
/// Failed command: <failed>
/// Reason: <reason>
/// Commands not applied:    <- whole section omitted when `not_applied` is empty
///   <command>...
/// ```
pub fn failure_report_header(
    applied: &[String],
    failed: &str,
    reason: &str,
    not_applied: &[String],
) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("Your command failed: {reason}"));
    if !applied.is_empty() {
        lines.push("Commands applied:".to_string());
        for c in applied {
            lines.push(format!("  {c}"));
        }
    }
    lines.push(format!("Failed command: {failed}"));
    lines.push(format!("Reason: {reason}"));
    if !not_applied.is_empty() {
        lines.push("Commands not applied:".to_string());
        for c in not_applied {
            lines.push(format!("  {c}"));
        }
    }
    lines.join("\n")
}

/// Outcome of running a reply's commands through the game-command dispatch.
pub enum GameCommandLoopOutcome {
    /// Every command processed without error: `move_count` game moves applied
    /// plus the last non-move status message (if any).
    Done {
        move_count: usize,
        last_status: Option<String>,
    },
    /// A command produced full content (e.g. rules) that short-circuits the loop.
    FullContent { html: String, text: String },
    /// A command failed: the commands applied before it, the failing command, the
    /// user-facing error message, and the commands after it that were never
    /// attempted.
    Failed {
        applied: Vec<String>,
        failed: String,
        error: String,
        not_applied: Vec<String>,
    },
}

/// Runs `commands` in order through `dispatch`, applying each until one fails
/// (earlier commands stay applied), stopping at the first failure. A
/// `FullContent` reply (rules) short-circuits the loop.
pub async fn run_game_reply_commands<F, Fut>(
    commands: &[String],
    mut dispatch: F,
) -> GameCommandLoopOutcome
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<
            Output = Result<
                crate::email::commands::CommandReply,
                crate::email::commands::CommandError,
            >,
        >,
{
    use crate::email::commands::{CommandError, CommandReply};

    let mut move_count: usize = 0;
    let mut last_status: Option<String> = None;
    for (index, line) in commands.iter().enumerate() {
        match dispatch(line.clone()).await {
            Ok(CommandReply::GameMove) => move_count += 1,
            Ok(CommandReply::Status(msg)) => last_status = Some(msg),
            Ok(CommandReply::FullContent { html, text }) => {
                return GameCommandLoopOutcome::FullContent { html, text };
            }
            Err(CommandError::User(msg)) => {
                return GameCommandLoopOutcome::Failed {
                    applied: commands[..index].to_vec(),
                    failed: line.clone(),
                    error: msg,
                    not_applied: commands[index + 1..].to_vec(),
                };
            }
            Err(CommandError::Internal(e)) => {
                tracing::error!("resend webhook: command error: {e}");
                return GameCommandLoopOutcome::Failed {
                    applied: commands[..index].to_vec(),
                    failed: line.clone(),
                    error: "An unexpected error occurred while processing your command."
                        .to_string(),
                    not_applied: commands[index + 1..].to_vec(),
                };
            }
        }
    }
    GameCommandLoopOutcome::Done {
        move_count,
        last_status,
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

async fn find_user_by_settings_token(
    pool: &sqlx::PgPool,
    token: &str,
) -> anyhow::Result<Option<uuid::Uuid>> {
    let row = sqlx::query_scalar::<_, uuid::Uuid>(
        "UPDATE users SET settings_token_used_at = NOW() WHERE id = (SELECT id FROM users WHERE settings_email_token = $1 AND settings_token_expires_at > NOW() AND settings_token_used_at IS NULL) RETURNING id",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

async fn from_matches_verified_email(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    from: &str,
) -> anyhow::Result<bool> {
    let canonical = crate::auth::email_addr::canonicalize_email(from);
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM user_emails WHERE user_id = $1 AND verified_at IS NOT NULL AND email = $2)",
    )
    .bind(user_id)
    .bind(canonical.as_str())
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Insert-or-skip idempotency marker. Returns true if THIS call inserted the
/// row (proceed); false if it already existed (a duplicate delivery -> skip).
///
/// The check-then-mark window (see `event_already_processed`) lets two
/// simultaneous deliveries of one `svix-id` both process - the accepted
/// at-least-once cost under D-2.
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

async fn event_already_processed(pool: &sqlx::PgPool, event_id: &str) -> sqlx::Result<bool> {
    let (exists,): (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM processed_webhook_events WHERE event_id = $1)")
            .bind(event_id)
            .fetch_one(pool)
            .await?;
    Ok(exists)
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

    match event_already_processed(&state.pool, &msg_id).await {
        Ok(true) => return StatusCode::OK,
        Ok(false) => {}
        Err(e) => {
            tracing::error!("resend webhook: idempotency check failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    let event: ResendEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("resend webhook: failed to parse event payload: {e}");
            let _ = mark_event_processed(&state.pool, &msg_id).await;
            return StatusCode::OK;
        }
    };
    if event.event_type != "email.received" {
        tracing::info!("resend webhook: ignoring event type {}", event.event_type);
        let _ = mark_event_processed(&state.pool, &msg_id).await;
        return StatusCode::OK;
    }

    let Some(from) = extract_addr_spec(&event.data.from) else {
        tracing::warn!(
            "resend webhook: could not extract an address from the From value; no response"
        );
        let _ = mark_event_processed(&state.pool, &msg_id).await;
        return StatusCode::OK;
    };

    let start = std::time::Instant::now();
    // The inbound fetch seam is a local route dependency: production builds
    // `ResendInbound` here (tests pass `StaticInbound` to the handlers directly).
    // A missing/empty `RESEND_API_KEY` stays a fetch-time error (lookup-first).
    let source = ResendInbound {
        api_key: std::env::var("RESEND_API_KEY").ok(),
        http: state.http_client.clone(),
    };
    let outcome = match select_route(&event.data.to, &event.data.received_for) {
        Some(InboundRoute::Game(token)) => {
            handle_game_reply(&state, &source, &token, &from, &event.data.email_id).await
        }
        Some(InboundRoute::Invite(token)) => {
            handle_invite_reply(&state, &source, &token, &from, &event.data.email_id).await
        }
        Some(InboundRoute::Settings(token)) => {
            handle_settings_reply_route(&state, &source, &token, &from, &event.data.email_id).await
        }
        None => {
            tracing::info!("resend webhook: no route for recipient; ignoring");
            // Non-transient: the recipient routes to no handler; nothing to retry.
            RouteOutcome::Done
        }
    };
    let elapsed = start.elapsed();
    if elapsed > std::time::Duration::from_secs(10) {
        tracing::warn!("resend webhook: dispatch took {elapsed:?} (>10s); consider option C");
    }
    match outcome {
        // Done: finished (success or permanent failure); mark processed, return 200.
        RouteOutcome::Done => {
            if let Err(e) = mark_event_processed(&state.pool, &msg_id).await {
                tracing::error!("resend webhook: failed to mark event processed: {e}");
            }
            StatusCode::OK
        }
        RouteOutcome::Retry => {
            tracing::error!(
                "resend webhook: transient failure; not marking, returning 5xx for retry"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

enum InboundText {
    Text(String),
    NoBody,
    FetchFailed,
    AuthFailed,
}

/// Fetches an inbound email's raw MIME source via the injected
/// [`InboundEmailSource`]. The single place the inbound direction performs the
/// fetch; this block used to be duplicated verbatim in all three routes (wfe F9).
/// The source is built once at the `resend_webhook` dispatch boundary
/// (`ResendInbound` in production; tests pass `StaticInbound` to the handlers).
async fn fetch_inbound_raw(
    source: &dyn InboundEmailSource,
    email_id: &str,
) -> anyhow::Result<String> {
    source.fetch_raw_email(email_id).await
}

/// Lookup-first fetch+classify+extract step shared by all three route handlers.
/// Each handler does its token/From lookup first, then calls this: fetches the
/// raw MIME once (wfe F9 single fetch), runs SPF/DKIM auth classification, and
/// extracts the plain-text body. `FetchFailed` is transient (handler retries);
/// `AuthFailed` is a permanent drop (handler returns Done, no reply).
async fn fetch_inbound_text(
    source: &dyn InboundEmailSource,
    from: &str,
    email_id: &str,
) -> InboundText {
    let raw = match fetch_inbound_raw(source, email_id).await {
        Ok(raw) => raw,
        Err(e) => {
            tracing::error!("resend webhook: failed to fetch raw email {email_id}: {e}");
            return InboundText::FetchFailed;
        }
    };
    match classify_inbound_auth(&raw) {
        AuthVerdict::Fail => {
            tracing::warn!(
                "resend webhook: inbound auth failed; permanently rejecting from={from} email_id={email_id}"
            );
            return InboundText::AuthFailed;
        }
        AuthVerdict::Pass => {}
    }
    match extract_plain_text(&raw) {
        Some(text) => InboundText::Text(text),
        None => InboundText::NoBody,
    }
}

/// Releases the proposal `FOR UPDATE` lock on an invite early-exit path that
/// has written nothing, so the outbound response email is not sent while
/// holding it (wfe F7). Rollback, not commit: no path that calls this has
/// written anything.
async fn rollback_invite_tx(tx: sqlx::Transaction<'_, sqlx::Postgres>, context: &str) {
    if let Err(e) = tx.rollback().await {
        tracing::warn!("resend webhook: invite rollback failed ({context}): {e}");
    }
}

/// Outcome of routing an inbound webhook event.
///
/// `Done` = finished (successfully or failed unrecoverably); mark the event
/// and return 200. `Retry` = transient failure before any state mutation;
/// do not mark, return 5xx so svix retries.
enum RouteOutcome {
    Done,
    Retry,
}

/// Maps a transient (retryable) failure to `RouteOutcome::Retry`, logging it.
/// The single named contract shared by the invite (F-162) and settings (F-169)
/// routes for every error that occurs BEFORE any state mutation is committed:
/// the webhook then returns 5xx so svix retries (at-least-once, D-2). Errors
/// that are not transient (missing data, auth, commit) stay `RouteOutcome::Done`
/// and carry their own justifying comment at the site.
fn transient_failure(e: impl std::fmt::Display, ctx: &str) -> RouteOutcome {
    tracing::error!("resend webhook: {ctx}: {e}");
    RouteOutcome::Retry
}

async fn handle_game_reply(
    state: &AppState,
    source: &dyn InboundEmailSource,
    token: &str,
    from: &str,
    email_id: &str,
) -> RouteOutcome {
    let pool = &state.pool;

    let player = match find_game_player_by_email_token(pool, token).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::info!("resend webhook: unknown game token; no response");
            // Non-transient: unknown game token; retrying cannot resolve it.
            return RouteOutcome::Done;
        }
        Err(e) => {
            tracing::error!("resend webhook: token lookup failed: {e}");
            return RouteOutcome::Retry;
        }
    };

    match from_matches_verified_email(pool, player.user_id, from).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!("resend webhook: From does not match a verified address; no response");
            // Non-transient: From is not a verified address for this player.
            return RouteOutcome::Done;
        }
        Err(e) => {
            tracing::error!("resend webhook: From verification failed: {e}");
            return RouteOutcome::Retry;
        }
    }

    let text = match fetch_inbound_text(source, from, email_id).await {
        InboundText::FetchFailed => return RouteOutcome::Retry,
        // Non-transient: inbound SPF/DKIM auth failed; permanent drop, no reply.
        InboundText::AuthFailed => return RouteOutcome::Done,
        InboundText::NoBody => {
            send_game_reply_response(state, &player, token, from, no_command_header_text()).await;
            // Non-transient: no text body; no-command reply already sent.
            return RouteOutcome::Done;
        }
        InboundText::Text(text) => text,
    };
    let commands = parse_reply_commands(&text);

    if commands.is_empty() {
        send_game_reply_response(state, &player, token, from, no_command_header_text()).await;
        // Non-transient: no commands parsed; no-command reply already sent.
        return RouteOutcome::Done;
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

    let ctx_ref = &ctx;
    let outcome = run_game_reply_commands(&commands, |line| async move {
        crate::email::commands::dispatch_email_command(ctx_ref, &line).await
    })
    .await;

    match outcome {
        GameCommandLoopOutcome::FullContent { html, text } => {
            send_rules_reply_response(state, &player, token, from, html, text).await;
        }
        GameCommandLoopOutcome::Failed {
            applied,
            failed,
            error,
            not_applied,
        } => {
            let header = failure_report_header(&applied, &failed, &error, &not_applied);
            send_game_failure_report(state, &player, token, from, header).await;
        }
        GameCommandLoopOutcome::Done {
            move_count,
            last_status,
        } => {
            let header = if move_count > 0 {
                confirmed_header_text(move_count)
            } else if let Some(status) = last_status {
                status
            } else {
                no_command_header_text()
            };
            send_game_reply_response(state, &player, token, from, header).await;
        }
    }
    // Non-transient: commands processed and the response email was sent.
    RouteOutcome::Done
}

/// True when every roster member's identity and response are identical between
/// the pre-game-service-call snapshot and the post-call re-read. Used after the
/// hoisted external call to detect a concurrent roster mutation before starting
/// the game, so a game is never created on stale membership (R-18 / F-135).
fn invite_roster_unchanged(
    snapshot: &[crate::proposals::ProposalPlayer],
    current: &[crate::proposals::ProposalPlayer],
) -> bool {
    if snapshot.len() != current.len() {
        return false;
    }
    snapshot.iter().zip(current.iter()).all(|(a, b)| {
        a.id == b.id
            && a.user_id == b.user_id
            && a.bot_name == b.bot_name
            && a.bot_difficulty == b.bot_difficulty
            && a.response == b.response
    })
}

async fn handle_invite_reply(
    state: &AppState,
    source: &dyn InboundEmailSource,
    token: &str,
    from: &str,
    email_id: &str,
) -> RouteOutcome {
    let pool = &state.pool;

    let player = match crate::proposals::find_proposal_player_by_email_token(pool, token).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::info!("resend webhook: unknown invite token; no response");
            // Non-transient: unknown invite token; retrying cannot resolve it.
            return RouteOutcome::Done;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite token lookup failed: {e}");
            return RouteOutcome::Retry;
        }
    };

    let Some(user_id) = player.user_id else {
        tracing::info!("resend webhook: invite token belongs to a bot slot; no response");
        // Non-transient: token belongs to a bot slot; no human response possible.
        return RouteOutcome::Done;
    };

    match from_matches_verified_email(pool, user_id, from).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                "resend webhook: invite From does not match a verified address; no response"
            );
            // Non-transient: From is not a verified address for this user.
            return RouteOutcome::Done;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite From verification failed: {e}");
            return RouteOutcome::Retry;
        }
    }

    let text = match fetch_inbound_text(source, from, email_id).await {
        InboundText::FetchFailed => return RouteOutcome::Retry,
        // Non-transient: inbound SPF/DKIM auth failed; permanent drop, no reply.
        InboundText::AuthFailed => return RouteOutcome::Done,
        InboundText::NoBody => {
            send_invite_reply_response(
                state,
                &player,
                user_id,
                from,
                no_command_header_text(),
                None,
            )
            .await;
            // Non-transient: no text body; no-command reply already sent.
            return RouteOutcome::Done;
        }
        InboundText::Text(text) => text,
    };
    let commands = parse_reply_commands(&text);

    let intent = parse_invite_intent(&commands);

    let Some(intent) = intent else {
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            no_command_header_text(),
            None,
        )
        .await;
        // Non-transient: no accept/decline intent; no-command reply already sent.
        return RouteOutcome::Done;
    };
    let accept = intent == InviteIntent::Accept;

    let proposal_id = player.proposal_id;
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("resend webhook: invite begin tx failed: {e}");
            return RouteOutcome::Retry;
        }
    };

    let proposal = match crate::proposals::lock_proposal_for_update(&mut tx, proposal_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::warn!("resend webhook: proposal {proposal_id} not found");
            // Non-transient: the proposal no longer exists; nothing to retry.
            return RouteOutcome::Done;
        }
        Err(e) => {
            tracing::error!("resend webhook: invite lock proposal failed: {e}");
            return RouteOutcome::Retry;
        }
    };

    if proposal.status != "open" {
        rollback_invite_tx(tx, "invite no longer open").await;
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "This invite is no longer open.".to_string(),
            None,
        )
        .await;
        // Non-transient: the proposal is no longer open; reply already sent.
        return RouteOutcome::Done;
    }

    let players = match crate::proposals::find_proposal_players_tx(&mut tx, proposal_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("resend webhook: invite players lookup failed: {e}");
            return RouteOutcome::Retry;
        }
    };

    let me = match players.iter().find(|p| p.id == player.id) {
        Some(p) => p,
        None => {
            tracing::warn!(
                "resend webhook: invite token's player {} is not in proposal {proposal_id}'s roster; no response",
                player.id
            );
            rollback_invite_tx(tx, "invite player not in roster").await;
            // Non-transient: the token's player is not in the roster; no response.
            return RouteOutcome::Done;
        }
    };

    if me.response != "pending" {
        rollback_invite_tx(tx, "invite already responded").await;
        send_invite_reply_response(
            state,
            &player,
            user_id,
            from,
            "That invite has already been responded to.".to_string(),
            None,
        )
        .await;
        // Non-transient: the invite was already responded to; reply already sent.
        return RouteOutcome::Done;
    }

    let response = if accept { "accepted" } else { "declined" };
    if let Err(e) =
        crate::proposals::update_proposal_player_response(&mut tx, player.id, response).await
    {
        return transient_failure(e, "invite update response failed");
    }

    // R-18 / F-135: snapshot every input to the game-service call inside this
    // short transaction, then commit and release the `FOR UPDATE` lock BEFORE
    // the external HTTP round-trip. A second transaction re-locks, re-reads and
    // re-validates the exact roster/status after the call, so a concurrent
    // mutation that lands while the game service is in-flight can never start a
    // game on stale membership. No network I/O runs while a transaction or row
    // lock is held.
    //
    // `start_inputs` is Some only when this acceptance settles the proposal (no
    // pending humans remain) and a game must be started.
    let mut start_inputs: Option<(
        crate::models::game::GameVersion,
        Vec<crate::proposals::ProposalPlayer>,
        usize,
    )> = None;

    if accept {
        let pending =
            match crate::proposals::count_pending_human_invitees_tx(&mut tx, proposal_id).await {
                Ok(n) => n,
                Err(e) => return transient_failure(e, "invite count pending failed"),
            };
        if pending == 0 {
            let game_version =
                match crate::db::find_game_version(&state.pool, proposal.game_version_id).await {
                    Ok(Some(gv)) => gv,
                    Ok(None) => {
                        tracing::error!("resend webhook: game version not found for proposal");
                        // Non-transient: the game version is genuinely missing; retrying
                        // cannot create it, so finish without starting a game. The
                        // transaction is dropped, rolling back the response write above
                        // (unchanged from the pre-R-18 behavior).
                        return RouteOutcome::Done;
                    }
                    Err(e) => return transient_failure(e, "invite game version lookup failed"),
                };
            let roster =
                match crate::proposals::find_proposal_players_tx(&mut tx, proposal_id).await {
                    Ok(p) => p,
                    Err(e) => return transient_failure(e, "invite roster lookup failed"),
                };
            let accepted_count = roster.iter().filter(|p| p.response == "accepted").count();
            start_inputs = Some((game_version, roster, accepted_count));
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("resend webhook: invite commit failed: {e}");
        // Non-transient for retry: the commit may have persisted the response
        // mutation, so an at-least-once retry could double-apply it; finish Done.
        return RouteOutcome::Done;
    }

    // The response mutation is now durable. From here every failure is Done
    // (never Retry): a retry would see `me.response != "pending"` and bail out
    // as "already responded", so it could not re-attempt the start.
    let mut started_game_id: Option<uuid::Uuid> = None;

    if let Some((game_version, roster_snapshot, accepted_count)) = start_inputs {
        // External game-service call: NO transaction open, NO row lock held.
        let fetched = match crate::game::server_fns::fetch_game_from_service(
            &state.http_client,
            &game_version,
            accepted_count,
        )
        .await
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("resend webhook: invite fetch game failed: {e}");
                // Response already committed; the game simply did not start. The
                // proposal stays open and the owner can start it manually.
                send_invite_reply_response(
                    state,
                    &player,
                    user_id,
                    from,
                    "Invite accepted.".to_string(),
                    None,
                )
                .await;
                return RouteOutcome::Done;
            }
        };

        // Re-open a transaction, re-lock the proposal, re-read the roster, and
        // re-validate the exact snapshot taken before the external call.
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("resend webhook: invite re-start begin tx failed: {e}");
                return RouteOutcome::Done;
            }
        };
        let proposal =
            match crate::proposals::lock_proposal_for_update(&mut tx, proposal_id).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    tracing::warn!("resend webhook: proposal {proposal_id} not found on re-start");
                    return RouteOutcome::Done;
                }
                Err(e) => {
                    tracing::error!("resend webhook: invite re-start lock failed: {e}");
                    return RouteOutcome::Done;
                }
            };
        if proposal.status != "open" {
            rollback_invite_tx(tx, "invite no longer open on re-start").await;
            send_invite_reply_response(
                state,
                &player,
                user_id,
                from,
                "Invite accepted.".to_string(),
                None,
            )
            .await;
            return RouteOutcome::Done;
        }
        let roster_now =
            match crate::proposals::find_proposal_players_tx(&mut tx, proposal_id).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("resend webhook: invite re-start roster lookup failed: {e}");
                    return RouteOutcome::Done;
                }
            };
        let accepted_now = roster_now.iter().filter(|p| p.response == "accepted").count();
        if !invite_roster_unchanged(&roster_snapshot, &roster_now) || accepted_now != accepted_count
        {
            // A concurrent mutation changed the roster while the game service was
            // in-flight. Do not start on stale membership; the response is already
            // recorded and the proposal stays open.
            rollback_invite_tx(tx, "invite roster changed during game fetch").await;
            send_invite_reply_response(
                state,
                &player,
                user_id,
                from,
                "Invite accepted.".to_string(),
                None,
            )
            .await;
            return RouteOutcome::Done;
        }
        match crate::proposals::start_proposal_tx(
            &mut tx,
            &proposal,
            &roster_now,
            &game_version,
            fetched,
        )
        .await
        {
            Ok(gid) => {
                started_game_id = Some(gid);
            }
            Err(e) => {
                tracing::error!("resend webhook: invite start proposal failed: {e}");
                // Roll back so no partial game write persists; the response remains
                // committed and the proposal stays open. Finish Done.
                rollback_invite_tx(tx, "invite start proposal failed").await;
                send_invite_reply_response(
                    state,
                    &player,
                    user_id,
                    from,
                    "Invite accepted.".to_string(),
                    None,
                )
                .await;
                return RouteOutcome::Done;
            }
        }
        if let Err(e) = tx.commit().await {
            tracing::error!("resend webhook: invite re-start commit failed: {e}");
            // The commit may have persisted the game row; an at-least-once retry
            // could double-apply it; finish Done.
            return RouteOutcome::Done;
        }
    }

    state
        .broadcaster
        .broadcast_proposal_update(proposal_id)
        .await;

    if let Some(gid) = started_game_id {
        crate::proposals::mailer_from(state.pool.clone(), state.resend.clone())
            .notify_game_started(gid)
            .await;
        crate::game::broadcast_and_trigger(&state.pool, &state.broadcaster, &state.jetstream, gid)
            .await;
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
    // Non-transient: response recorded (and game started if applicable); reply sent.
    RouteOutcome::Done
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

    let mut game_type_name: Option<String> = None;
    let mut game_version_id: Option<uuid::Uuid> = None;
    match crate::proposals::find_proposal(pool, proposal_id).await {
        Ok(Some(proposal)) => {
            game_version_id = Some(proposal.game_version_id);
            match crate::db::find_game_version(pool, proposal.game_version_id).await {
                Ok(Some(gv)) => {
                    match crate::proposals::find_game_type_name(pool, gv.game_type_id).await {
                        Ok(Some(name)) => game_type_name = Some(name),
                        Ok(None) => tracing::warn!(
                            "resend webhook: game type {} not found for invite subject",
                            gv.game_type_id
                        ),
                        Err(e) => {
                            tracing::error!("resend webhook: invite game type lookup failed: {e}")
                        }
                    }
                }
                Ok(None) => tracing::warn!(
                    "resend webhook: game version {} not found for invite subject",
                    proposal.game_version_id
                ),
                Err(e) => {
                    tracing::error!("resend webhook: invite game version lookup failed: {e}")
                }
            }
        }
        Ok(None) => {
            tracing::warn!("resend webhook: proposal {proposal_id} not found for invite subject")
        }
        Err(e) => tracing::error!("resend webhook: invite proposal lookup failed: {e}"),
    }

    let base = crate::config::public_base_url();
    let browser_url = match game_id {
        Some(gid) => format!("{base}/games/{gid}"),
        None => format!("{base}/invites/{proposal_id}"),
    };

    let palette = crate::email::render::palette_for_slug(theme_slug.as_deref());
    let content = crate::email::render::EmailContent {
        subject: match &game_type_name {
            Some(name) => format!("{name} invite"),
            None => "Your brdg.me invite".to_string(),
        },
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
        &crate::email::notify::invite_reply_address(player.email_token.as_deref().unwrap_or("")),
        None,
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
        None,
    );
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

/// Emails the player a failure report after a multi-command reply stops at a
/// failing command. Reuses the turn-email rendering path (current render,
/// "Since last time" logs, command spec, footers) reflecting the game state
/// AFTER the successfully-applied commands, with the failure breakdown on top.
/// De-threaded like a turn email (unique `turn_subject`, no stable refs chain)
/// and `reply_to` set to the player's `g-{token}@brdg.me` so they can reply
/// again with corrected commands.
async fn send_game_failure_report(
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
                "resend webhook: game {} not found for failure report",
                player.game_id
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                "resend webhook: failed to load game {} for failure report: {e}",
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
    let content = crate::email::notify::failure_report_content(
        pool,
        &state.http_client,
        &ge,
        recipient_player,
        header,
    )
    .await;
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
        None,
        false,
        &crate::email::notify::reply_address(token),
        None,
    );
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

/// Builds the natural "rules" reply: threaded under the game's message id, and
/// deliberately carries NO `List-Unsubscribe` headers - it is a one-shot reply
/// to a sent game notification, not a notification itself.
fn rules_reply_message(
    game_id: uuid::Uuid,
    token: &str,
    text: String,
    html: String,
) -> crate::email::render::RenderedEmail {
    let mut headers = std::collections::BTreeMap::new();
    let msg_id = format!("<game-{game_id}@brdg.me>");
    headers.insert("In-Reply-To".to_string(), msg_id.clone());
    headers.insert("References".to_string(), msg_id);
    crate::email::render::RenderedEmail {
        subject: "Rules".to_string(),
        text,
        html,
        headers,
        reply_to: crate::email::notify::reply_address(token),
    }
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

    let rendered = rules_reply_message(player.game_id, token, text, full_html);
    crate::email::outbound::send_rendered_email(state.resend.as_ref(), rendered, from).await;
}

async fn handle_settings_reply_route(
    state: &AppState,
    source: &dyn InboundEmailSource,
    token: &str,
    from: &str,
    email_id: &str,
) -> RouteOutcome {
    let text = match fetch_inbound_text(source, from, email_id).await {
        InboundText::FetchFailed => return RouteOutcome::Retry,
        // Non-transient: inbound SPF/DKIM auth failed; permanent drop, no reply.
        InboundText::AuthFailed => return RouteOutcome::Done,
        // Non-transient: no text body to act on.
        InboundText::NoBody => return RouteOutcome::Done,
        InboundText::Text(text) => text,
    };
    handle_settings_reply(state, token, from, &text).await
}

/// Rate limiting on this path depends on R-37 (no rate-limiting middleware
/// exists in rust/web yet - F-94).
async fn handle_settings_reply(
    state: &AppState,
    token: &str,
    from: &str,
    text: &str,
) -> RouteOutcome {
    let user_id = match find_user_by_settings_token(&state.pool, token).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::info!("resend webhook: settings reply with unknown token; no response");
            // Non-transient: unknown, expired, or already-used settings token.
            return RouteOutcome::Done;
        }
        Err(e) => return transient_failure(e, "settings token lookup failed"),
    };

    match from_matches_verified_email(&state.pool, user_id, from).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                "resend webhook: settings From does not match a verified address; no response"
            );
            // Non-transient: From is not a verified address for this user.
            return RouteOutcome::Done;
        }
        Err(e) => return transient_failure(e, "settings From verification failed"),
    }

    let commands = parse_reply_commands(text);

    if commands.is_empty() {
        send_settings_response(
            &state.pool,
            state.resend.as_ref(),
            user_id,
            from,
            no_command_header_text(),
        )
        .await;
        // Non-transient: no commands; no-command response already sent.
        return RouteOutcome::Done;
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
    // Non-transient: settings commands processed; response already sent.
    RouteOutcome::Done
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
    let reply_address =
        match crate::email::outbound::ensure_settings_email_token(pool, user_id).await {
            Ok(token) => crate::email::notify::settings_reply_address(&token),
            Err(e) => {
                tracing::error!(
                    "resend webhook: settings email token unavailable; omitting reply address: {e}"
                );
                String::new()
            }
        };
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
        None,
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
    fn parse_reply_commands_realistic_reply_body_strips_quote_block() {
        let input = "play f10\n\
                     buy 1 sa\n\
                     buy 1 wo\n\
                     buy 1 to\n\
                     done\n\
                     \n\
                     On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me> wrote:\n\
                     > ...quoted original email...\n\
                     > more quoted text";
        assert_eq!(
            parse_reply_commands(input),
            vec!["play f10", "buy 1 sa", "buy 1 wo", "buy 1 to", "done"]
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
    fn parse_reply_commands_cuts_at_wrapped_gmail_attribution() {
        let input = "play e4\n\
                     \n\
                     On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me>\n\
                     wrote:\n\
                     > board\n\
                     > more board";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_localized_attribution() {
        let input = "play e4\n\
                     \n\
                     Le 22 juillet 2026 a 13:16, brdg.me a ecrit :\n\
                     > board";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_outlook_original_message_block() {
        let input = "play e4\n\
                     \n\
                     -----Original Message-----\n\
                     From: brdg.me <mail@brdg.me>\n\
                     Sent: Wednesday, 22 July 2026 13:16\n\
                     Subject: Your turn\n\
                     \n\
                     board text that is not quoted";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_cuts_at_bare_header_block() {
        let input = "play e4\n\
                     \n\
                     From: brdg.me <mail@brdg.me>\n\
                     Sent: Wednesday, 22 July 2026 13:16\n\
                     board text";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_keeps_last_command_before_blank_then_quote() {
        let input = "play e4\n\
                     done\n\
                     \n\
                     On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me> wrote:\n\
                     > board";
        assert_eq!(parse_reply_commands(input), vec!["play e4", "done"]);
    }

    #[test]
    fn parse_reply_commands_keeps_a_command_typed_below_a_quote_block() {
        let input = "> board\n\
                     > more board\n\
                     play e4";
        assert_eq!(parse_reply_commands(input), vec!["play e4"]);
    }

    #[test]
    fn parse_reply_commands_does_not_retract_a_command_directly_above_a_quote() {
        let input = "play d4\n> previous move was e4";
        assert_eq!(parse_reply_commands(input), vec!["play d4"]);
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
    fn classify_inbound_auth_pass() {
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=x.com; dkim=pass header.i=@x.com; dmarc=pass header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Pass);
    }

    #[test]
    fn classify_inbound_auth_fail_dmarc() {
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=x.com; dkim=pass header.i=@x.com; dmarc=fail header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_fail_spf_and_dkim() {
        let raw = "Authentication-Results: amazonses.com; spf=fail smtp.mailfrom=x.com; dkim=fail header.i=@x.com; dmarc=pass header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_absent_header() {
        let raw = "From: a@x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_wrong_authserv_id() {
        let raw = "Authentication-Results: mx.google.com; spf=fail smtp.mailfrom=x.com; dkim=fail header.i=@x.com; dmarc=fail header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_ignores_injected_lower_header() {
        let raw = "Authentication-Results: amazonses.com; spf=fail smtp.mailfrom=x.com; dkim=fail header.i=@x.com; dmarc=fail header.from=x.com\r\n\
Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=x.com; dkim=pass header.i=@x.com; dmarc=pass header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_softfail_is_not_fail() {
        let raw = "Authentication-Results: amazonses.com; spf=softfail smtp.mailfrom=x.com; dkim=none header.i=@x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_single_fail_is_not_fail() {
        let raw = "Authentication-Results: amazonses.com; spf=fail smtp.mailfrom=x.com; dkim=none header.i=@x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_dmarc_rule_spf_fail_dkim_none_is_not_pass() {
        let raw = "Authentication-Results: amazonses.com; spf=fail smtp.mailfrom=x.com; dkim=none header.i=@x.com\r\n\
\r\n\
body\r\n";
        assert_ne!(classify_inbound_auth(raw), AuthVerdict::Pass);
    }

    #[test]
    fn classify_inbound_auth_unknown_verdict_is_rejected() {
        let raw = "Authentication-Results: mx.google.com; spf=fail smtp.mailfrom=x.com; dkim=fail header.i=@x.com; dmarc=fail header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
    }

    #[test]
    fn classify_inbound_auth_attacker_supplied_sole_header_is_not_honoured() {
        let raw = "Authentication-Results: mail.attacker.example; spf=pass smtp.mailfrom=x.com; dkim=pass header.i=@x.com; dmarc=pass header.from=x.com\r\n\
\r\n\
body\r\n";
        assert_eq!(classify_inbound_auth(raw), AuthVerdict::Fail);
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
    fn verify_webhook_rejects_invalid_header_value() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let body = b"{}";
        let result = verify_webhook(secret, "msg\ninjection", "sig", "123", body);
        assert!(matches!(result, Err(VerifyError::InvalidHeaderValue)));
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

    #[test]
    fn extract_addr_spec_bare_address_is_unchanged() {
        assert_eq!(
            extract_addr_spec("alice@example.com").as_deref(),
            Some("alice@example.com")
        );
        assert_eq!(
            extract_addr_spec("  alice@example.com  ").as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn extract_addr_spec_display_name_forms() {
        for input in [
            "Alice <alice@example.com>",
            "\"Doe, Alice\" <alice@example.com>",
            "Alice (at home) <alice@example.com>",
            "=?utf-8?q?Alice?= <alice@example.com>",
            "<alice@example.com>",
        ] {
            assert_eq!(
                extract_addr_spec(input).as_deref(),
                Some("alice@example.com"),
                "input: {input}"
            );
        }
    }

    #[test]
    fn extract_addr_spec_first_of_several() {
        assert_eq!(
            extract_addr_spec("a@x.com, b@y.com").as_deref(),
            Some("a@x.com")
        );
    }

    #[test]
    fn extract_addr_spec_rejects_valueless_input() {
        assert_eq!(extract_addr_spec(""), None);
        assert_eq!(extract_addr_spec("   "), None);
        assert_eq!(extract_addr_spec("Alice"), None);
        assert_eq!(extract_addr_spec("<>"), None);
    }

    #[test]
    fn extract_addr_spec_strips_crlf_before_parsing() {
        assert_eq!(
            extract_addr_spec("Alice <alice@example.com>\r\nBcc: evil@x.com").as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn extract_addr_spec_keeps_address_after_fold() {
        assert_eq!(
            extract_addr_spec("Alice\r\n <alice@example.com>").as_deref(),
            Some("alice@example.com")
        );
        assert_eq!(
            extract_addr_spec("Alice\n\t<alice@example.com>").as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn rules_reply_message_keeps_threading_and_omits_list_unsubscribe() {
        let game_id = uuid::Uuid::new_v4();
        let rendered = rules_reply_message(
            game_id,
            "tok-rules",
            "rules text".to_string(),
            "<p>rules html</p>".to_string(),
        );
        let msg_id = format!("<game-{game_id}@brdg.me>");
        assert_eq!(rendered.subject, "Rules");
        assert_eq!(
            rendered.headers.get("In-Reply-To").map(String::as_str),
            Some(msg_id.as_str())
        );
        assert_eq!(
            rendered.headers.get("References").map(String::as_str),
            Some(msg_id.as_str())
        );
        assert_eq!(rendered.headers.get("List-Unsubscribe"), None);
        assert_eq!(rendered.headers.get("List-Unsubscribe-Post"), None);
        assert_eq!(rendered.reply_to, "g-tok-rules@brdg.me");
    }

    #[test]
    fn parse_invite_intent_first_verb_wins() {
        let decline_first: Vec<String> = vec!["decline".into(), "accept".into()];
        assert_eq!(
            parse_invite_intent(&decline_first),
            Some(InviteIntent::Decline)
        );
        let accept_first: Vec<String> = vec!["accept".into(), "decline".into()];
        assert_eq!(
            parse_invite_intent(&accept_first),
            Some(InviteIntent::Accept)
        );
    }

    #[test]
    fn parse_invite_intent_is_case_insensitive_and_whole_line() {
        assert_eq!(
            parse_invite_intent(&["ACCEPT".to_string()]),
            Some(InviteIntent::Accept)
        );
        assert_eq!(parse_invite_intent(&["decline politely".to_string()]), None);
        assert_eq!(parse_invite_intent(&[]), None);
    }

    #[test]
    fn select_route_handles_display_name_recipients() {
        assert_eq!(
            select_route(&["brdg.me <g-abc@brdg.me>".to_string()], &[]),
            Some(InboundRoute::Game("abc".to_string()))
        );
        assert_eq!(
            select_route(&[], &["Invites <i-xyz@brdg.me>".to_string()]),
            Some(InboundRoute::Invite("xyz".to_string()))
        );
    }

    #[tokio::test]
    async fn game_reply_loop_all_succeed_counts_moves() {
        use crate::email::commands::CommandReply;
        let cmds: Vec<String> = vec!["play f10".into(), "buy 1 sa".into(), "done".into()];
        let outcome =
            run_game_reply_commands(&cmds, |_line| async { Ok(CommandReply::GameMove) }).await;
        match outcome {
            GameCommandLoopOutcome::Done {
                move_count,
                last_status,
            } => {
                assert_eq!(move_count, 3);
                assert!(last_status.is_none());
            }
            _ => panic!("expected Done"),
        }
    }

    #[tokio::test]
    async fn game_reply_loop_stops_at_first_failure_with_earlier_applied() {
        use crate::email::commands::{CommandError, CommandReply};
        let cmds: Vec<String> = vec![
            "play f10".into(),
            "buy 1 sa".into(),
            "buy 1 wo".into(),
            "buy 1 to".into(),
            "done".into(),
        ];
        let outcome = run_game_reply_commands(&cmds, |line| async move {
            if line == "buy 1 wo" {
                Err(CommandError::User("not enough resources".to_string()))
            } else {
                Ok(CommandReply::GameMove)
            }
        })
        .await;
        match outcome {
            GameCommandLoopOutcome::Failed {
                applied,
                failed,
                error,
                not_applied,
            } => {
                assert_eq!(applied, vec!["play f10", "buy 1 sa"]);
                assert_eq!(failed, "buy 1 wo");
                assert_eq!(error, "not enough resources");
                assert_eq!(not_applied, vec!["buy 1 to", "done"]);
            }
            _ => panic!("expected Failed"),
        }
    }

    #[tokio::test]
    async fn game_reply_loop_first_command_failure_has_empty_applied() {
        use crate::email::commands::{CommandError, CommandReply};
        let cmds: Vec<String> = vec!["bad".into(), "done".into()];
        let outcome = run_game_reply_commands(&cmds, |line| async move {
            if line == "bad" {
                Err(CommandError::User("nope".to_string()))
            } else {
                Ok(CommandReply::GameMove)
            }
        })
        .await;
        match outcome {
            GameCommandLoopOutcome::Failed {
                applied,
                failed,
                not_applied,
                ..
            } => {
                assert!(applied.is_empty());
                assert_eq!(failed, "bad");
                assert_eq!(not_applied, vec!["done"]);
            }
            _ => panic!("expected Failed"),
        }
    }

    #[tokio::test]
    async fn game_reply_loop_full_content_short_circuits() {
        use crate::email::commands::CommandReply;
        let cmds: Vec<String> = vec!["rules".into(), "play f10".into()];
        let outcome = run_game_reply_commands(&cmds, |line| async move {
            if line == "rules" {
                Ok(CommandReply::FullContent {
                    html: "<p>rules</p>".to_string(),
                    text: "rules".to_string(),
                })
            } else {
                Ok(CommandReply::GameMove)
            }
        })
        .await;
        match outcome {
            GameCommandLoopOutcome::FullContent { html, text } => {
                assert_eq!(html, "<p>rules</p>");
                assert_eq!(text, "rules");
            }
            _ => panic!("expected FullContent"),
        }
    }

    #[test]
    fn failure_report_header_omits_empty_sections() {
        let h = failure_report_header(&[], "buy 1 wo", "not enough resources", &[]);
        assert!(!h.contains("Commands applied:"));
        assert!(h.contains("Failed command: buy 1 wo"));
        assert!(h.contains("Reason: not enough resources"));
        assert!(!h.contains("Commands not applied:"));
    }

    #[test]
    fn failure_report_header_includes_applied_and_not_applied() {
        let applied = vec!["play f10".to_string(), "buy 1 sa".to_string()];
        let not_applied = vec!["buy 1 to".to_string(), "done".to_string()];
        let h = failure_report_header(&applied, "buy 1 wo", "not enough resources", &not_applied);
        assert!(h.contains("Commands applied:"));
        assert!(h.contains("play f10"));
        assert!(h.contains("buy 1 sa"));
        assert!(h.contains("Failed command: buy 1 wo"));
        assert!(h.contains("Reason: not enough resources"));
        assert!(h.contains("Commands not applied:"));
        assert!(h.contains("buy 1 to"));
        assert!(h.contains("done"));
        // Applied section precedes the failed command; not-applied follows it.
        let applied_pos = h.find("Commands applied:").unwrap();
        let failed_pos = h.find("Failed command:").unwrap();
        let not_applied_pos = h.find("Commands not applied:").unwrap();
        assert!(applied_pos < failed_pos);
        assert!(failed_pos < not_applied_pos);
    }

    #[test]
    fn failure_report_header_first_line_states_failure_with_error() {
        let h = failure_report_header(&[], "done", "it is not your turn", &[]);
        let first = h.lines().next().expect("header has a first line");
        assert!(first.contains("it is not your turn"));
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

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40). The game-service render degrades to absent blocks
    // (no service running), which is fine: this asserts the subject scheme, the
    // reply_to field, and the de-threading (no threading headers).
    #[sqlx::test]
    async fn failure_report_is_dethreaded_and_sets_reply_to(pool: sqlx::PgPool) {
        let (game_id, _user_id, game_player_id) = seed_game_with_player(&pool, "tok-fail").await;
        let ge = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        let recipient_player = ge
            .game_players
            .iter()
            .find(|p| p.game_player.id == game_player_id)
            .expect("player in game");

        let content = crate::email::notify::failure_report_content(
            &pool,
            &reqwest::Client::new(),
            &ge,
            recipient_player,
            failure_report_header(&[], "buy 1 wo", "not enough resources", &[]),
        )
        .await;

        // De-threaded subject scheme, same as turn emails (fresh game => turn 0).
        assert_eq!(
            content.subject,
            crate::email::notify::turn_subject(&ge.game_type.name, game_id, 0)
        );
        assert!(
            content
                .header
                .as_deref()
                .unwrap()
                .contains("Failed command: buy 1 wo")
        );
        assert!(content.footer.is_some());

        let palette = crate::email::render::palette_for_slug(None);
        let rendered = crate::email::render::render_game_email(
            &content,
            palette,
            &[],
            None,
            false,
            &crate::email::notify::reply_address("tok-fail"),
            None,
        );
        assert_eq!(rendered.reply_to, "g-tok-fail@brdg.me");
        assert_eq!(rendered.headers.get("Message-Id"), None);
        assert_eq!(rendered.headers.get("In-Reply-To"), None);
        assert_eq!(rendered.headers.get("References"), None);
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

    #[sqlx::test]
    async fn from_matches_verified_email_unicode_canonical_agreement(pool: sqlx::PgPool) {
        let user_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", uuid::Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let canonical = crate::auth::email_addr::canonicalize_email("İ@example.com");
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(canonical.as_str())
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            from_matches_verified_email(&pool, user_id, "İ@example.com")
                .await
                .unwrap()
        );
        let stored: String = sqlx::query_scalar("SELECT email FROM user_emails WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, canonical.as_str());
        assert!(
            !from_matches_verified_email(&pool, user_id, "i@example.com")
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

    #[sqlx::test]
    async fn find_user_by_settings_token_lookup(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "token-lookup").await;
        let token = crate::email::outbound::ensure_settings_email_token(&pool, user_id)
            .await
            .unwrap();

        assert_eq!(
            find_user_by_settings_token(&pool, &token).await.unwrap(),
            Some(user_id)
        );
        assert!(
            find_user_by_settings_token(&pool, "tok-missing")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            find_user_by_settings_token(&pool, "")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn settings_token_is_not_the_user_id(pool: sqlx::PgPool) {
        let u = seed_user(&pool, "token-not-id").await;
        let token = crate::email::outbound::ensure_settings_email_token(&pool, u)
            .await
            .unwrap();

        assert_ne!(token, u.to_string());
        assert!(
            find_user_by_settings_token(&pool, &u.to_string())
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn settings_reply_requires_token_and_from(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "settings-auth").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind("verified@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        let token = crate::email::outbound::ensure_settings_email_token(&pool, user_id)
            .await
            .unwrap();

        let resolved = find_user_by_settings_token(&pool, &token).await.unwrap();
        assert_eq!(resolved, Some(user_id));
        assert!(
            from_matches_verified_email(&pool, user_id, "verified@brdg.me")
                .await
                .unwrap()
        );

        assert!(
            !from_matches_verified_email(&pool, user_id, "foreign@brdg.me")
                .await
                .unwrap()
        );

        assert!(
            find_user_by_settings_token(&pool, "tok-wrong")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn unrouted_recipient_is_ignored() {
        assert_eq!(select_route(&["hello@brdg.me".to_string()], &[]), None);
        assert_eq!(
            select_route(&["unsubscribe@brdg.me".to_string()], &[]),
            None
        );
    }

    #[sqlx::test]
    async fn settings_token_expired_is_rejected(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "expired-token").await;
        let token = crate::email::outbound::ensure_settings_email_token(&pool, user_id)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE users SET settings_token_expires_at = NOW() - interval '1 hour' WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            find_user_by_settings_token(&pool, &token)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    async fn settings_token_is_single_use(pool: sqlx::PgPool) {
        let user_id = seed_user(&pool, "single-use-token").await;
        let token = crate::email::outbound::ensure_settings_email_token(&pool, user_id)
            .await
            .unwrap();
        assert_eq!(
            find_user_by_settings_token(&pool, &token).await.unwrap(),
            Some(user_id)
        );
        assert!(
            find_user_by_settings_token(&pool, &token)
                .await
                .unwrap()
                .is_none()
        );
    }

    // ---- R-09 (F-162, F-169): the at-least-once `RouteOutcome::Retry` contract.
    //
    // These call the route handlers directly and inject a transient DB error,
    // asserting `Retry`. The inbound fetch is satisfied without a live Resend
    // API by passing a `StaticInbound` as the handlers' `&dyn InboundEmailSource`
    // argument (the production seam is `ResendInbound`, built at the
    // `resend_webhook` dispatch boundary; no `AppState` field is involved).
    //   * F-169: `handle_settings_reply` returns `RouteOutcome` (both `Err`
    //     lookup arms map to `Retry` via `transient_failure`), and
    //     `handle_settings_reply_route` propagates it instead of returning
    //     `Done` unconditionally.
    //   * F-162: the in-tx transient errors before `tx.commit()` in
    //     `handle_invite_reply` map to `Retry` via `transient_failure`.
    //
    // Like `tests/inbound_webhook.rs`, these build a full `AppState`, so they
    // need a running NATS (provided by scripts/rust-test.sh / CI) in addition
    // to Postgres.

    async fn make_inbound_test_state(pool: sqlx::PgPool) -> crate::state::AppState {
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let nats_client = async_nats::connect(&nats_url).await.expect("nats connect");
        let jetstream = async_nats::jetstream::new(nats_client.clone());
        let broadcaster = crate::websocket::GameBroadcaster::new(nats_client);
        crate::state::AppState {
            leptos_options: leptos::config::LeptosOptions::builder()
                .output_name("web")
                .build(),
            pool,
            broadcaster,
            http_client: reqwest::Client::new(),
            resend: None,
            jetstream,
        }
    }

    // A pool to the same (sqlx::test temp) database with a short lock_timeout,
    // so one contended statement fails fast with a transient lock-timeout error
    // while every uncontended statement still succeeds.
    async fn pool_with_lock_timeout(base: &sqlx::PgPool) -> sqlx::PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SET lock_timeout = '100ms'")
                        .execute(&mut *conn)
                        .await
                        .map(|_| ())
                })
            })
            .connect_with((*base.connect_options()).clone())
            .await
            .expect("connect lock-timeout pool")
    }

    #[sqlx::test]
    async fn settings_route_transient_db_error_is_retry(pool: sqlx::PgPool) {
        // F-169: a transient DB error on the settings route's token lookup must
        // be Retry, not Done. The StaticInbound seam satisfies the inbound
        // fetch; closing the pool then makes the first DB call
        // (find_user_by_settings_token) error, simulating transient DB
        // unavailability before any state mutation. The route entry
        // (handle_settings_reply_route) must propagate that as Retry instead of
        // returning Done unconditionally.
        let email_id = "em_settings_retry";
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=brdg.me\r\n\
                   MIME-Version: 1.0\r\n\
                   Content-Type: text/plain; charset=utf-8\r\n\
                   \r\n\
                   theme dracula\r\n";
        let mut emails = std::collections::HashMap::new();
        emails.insert(email_id.to_string(), raw.to_string());
        let source = StaticInbound(emails);
        let state = make_inbound_test_state(pool.clone()).await;
        state.pool.close().await;

        let outcome = handle_settings_reply_route(
            &state,
            &source,
            "settings-token",
            "user@brdg.me",
            email_id,
        )
        .await;

        assert!(
            matches!(outcome, RouteOutcome::Retry),
            "transient DB error on the settings token lookup must be Retry, not Done (F-169)"
        );
    }

    #[sqlx::test]
    async fn invite_route_transient_db_error_is_retry(pool: sqlx::PgPool) {
        // F-162: a transient DB error inside the invite transaction (before
        // commit) must be Retry, not Done. Drive handle_invite_reply to its
        // first in-tx write (update_proposal_player_response) and fail only
        // that statement with a lock timeout, so every earlier step succeeds.
        let user_id = seed_user(&pool, "invite-reply-user").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind("invitee@brdg.me")
        .execute(&pool)
        .await
        .unwrap();
        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Invite Game {}", uuid::Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(&pool)
        .await
        .unwrap();
        let game_version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) \
             VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let owner_id = seed_user(&pool, "invite-owner").await;
        let proposal_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status) \
             VALUES ($1, $2, 'open') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let token = "invite-tok-retry";
        let player_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposal_players \
              (proposal_id, \"position\", user_id, response, email_token) \
             VALUES ($1, 1, $2, 'pending', $3) RETURNING id",
        )
        .bind(proposal_id)
        .bind(user_id)
        .bind(token)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Inbound fetch satisfied by the StaticInbound seam: a raw MIME that
        // classifies SPF pass and extracts the body "decline".
        let email_id = "em_invite_retry";
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=brdg.me\r\n\
                   MIME-Version: 1.0\r\n\
                   Content-Type: text/plain; charset=utf-8\r\n\
                   \r\n\
                   decline\r\n";
        let mut emails = std::collections::HashMap::new();
        emails.insert(email_id.to_string(), raw.to_string());
        let source = StaticInbound(emails);
        let handler_pool = pool_with_lock_timeout(&pool).await;
        let state = make_inbound_test_state(handler_pool).await;

        // Hold a FOR UPDATE lock on the player row from a second connection so
        // the handler's in-tx UPDATE times out (a transient DB error) while the
        // preceding uncontended lookups succeed.
        let mut blocker = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM game_proposal_players WHERE id = $1 FOR UPDATE")
            .bind(player_id)
            .execute(&mut *blocker)
            .await
            .unwrap();

        let outcome =
            handle_invite_reply(&state, &source, token, "invitee@brdg.me", email_id).await;

        drop(blocker);
        assert!(
            matches!(outcome, RouteOutcome::Retry),
            "transient DB error inside the invite transaction must be Retry, not Done (F-162)"
        );
    }

    // ---- R-18 (F-135): concurrency regression for the external-call-to-
    // revalidation gap in handle_invite_reply.
    //
    // The game-service HTTP call (fetch_game_from_service) currently executes
    // inside the FOR UPDATE transaction. A concurrent roster mutation that
    // lands while the game service is in-flight is invisible to the in-memory
    // roster snapshot, so start_proposal_tx creates a game on stale membership.
    //
    // This test gates the mock game service with a tokio::sync::Notify pair so
    // the concurrent mutation is placed deterministically in the gap between the
    // external call and the transaction's use of the roster. It asserts the
    // state machine does NOT start a game on stale proposal membership/status.
    //
    // Expected RED on current code: the game IS started (stale roster used).
    // Expected GREEN after R-18 fix: the re-validation detects the stale roster
    // and aborts without starting a game.

    fn new_game_response(player_count: usize) -> brdgme_cmd::api::Response {
        use brdgme_cmd::api::{GameResponse, PlayerRender, PubRender, Response};
        Response::New {
            game: GameResponse {
                state: "test_game_state".to_string(),
                points: vec![0.0; player_count],
                status: brdgme_game::Status::Active {
                    whose_turn: vec![0],
                    eliminated: vec![],
                },
            },
            logs: vec![],
            public_render: PubRender {
                pub_state: "pub".to_string(),
                render: "render".to_string(),
            },
            player_renders: (0..player_count)
                .map(|i| PlayerRender {
                    player_state: format!("p{i}"),
                    render: format!("p{i}render"),
                    command_spec: None,
                })
                .collect(),
            seed: 42,
        }
    }

    #[sqlx::test]
    async fn invite_reply_does_not_start_game_on_stale_roster_after_game_fetch(
        pool: sqlx::PgPool,
    ) {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::Request;
        use std::sync::Arc;
        use tokio::net::TcpListener;

        let owner_id = seed_user(&pool, "r18-owner").await;
        let invitee_id = seed_user(&pool, "r18-invitee").await;
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(invitee_id)
        .bind("r18-invitee@brdg.me")
        .execute(&pool)
        .await
        .unwrap();

        let called = Arc::new(tokio::sync::Notify::new());
        let proceed = Arc::new(tokio::sync::Notify::new());
        let called_mock = called.clone();
        let proceed_mock = proceed.clone();

        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| {
                let called = called_mock.clone();
                let proceed = proceed_mock.clone();
                async move {
                    match payload {
                        Request::New { players, .. } => {
                            called.notify_one();
                            proceed.notified().await;
                            Json(new_game_response(players))
                        }
                        _ => Json(new_game_response(2)),
                    }
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mock_uri = format!("http://{addr}");

        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("R18 Game {}", uuid::Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(&pool)
        .await
        .unwrap();
        let game_version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) \
             VALUES ($1, $2, $3, true, false) RETURNING id",
        )
        .bind(game_type_id)
        .bind(format!("invite-mock-{}", uuid::Uuid::new_v4().simple()))
        .bind(&mock_uri)
        .fetch_one(&pool)
        .await
        .unwrap();
        let proposal_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status) \
             VALUES ($1, $2, 'open') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_proposal_players \
               (proposal_id, \"position\", user_id, response, email_token) \
             VALUES ($1, 0, $2, 'accepted', NULL)",
        )
        .bind(proposal_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
        let token = "r18-invite-tok";
        sqlx::query(
            "INSERT INTO game_proposal_players \
               (proposal_id, \"position\", user_id, response, email_token) \
             VALUES ($1, 1, $2, 'pending', $3)",
        )
        .bind(proposal_id)
        .bind(invitee_id)
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();

        let email_id = "em_r18_stale_roster";
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=brdg.me\r\n\
                   MIME-Version: 1.0\r\n\
                   Content-Type: text/plain; charset=utf-8\r\n\
                   \r\n\
                   accept\r\n";
        let mut emails = std::collections::HashMap::new();
        emails.insert(email_id.to_string(), raw.to_string());
        let state = std::sync::Arc::new(make_inbound_test_state(pool.clone()).await);
        let state_task = state.clone();
        let handle = tokio::spawn(async move {
            let source = StaticInbound(emails);
            handle_invite_reply(&state_task, &source, token, "r18-invitee@brdg.me", email_id)
                .await
        });

        called.notified().await;

        sqlx::query(
            "UPDATE game_proposal_players SET response = 'declined', \
               responded_at = (now() AT TIME ZONE 'utc'), \
               updated_at = (now() AT TIME ZONE 'utc') \
             WHERE proposal_id = $1 AND \"position\" = 0",
        )
        .bind(proposal_id)
        .execute(&pool)
        .await
        .unwrap();

        proceed.notify_one();
        let outcome = handle.await.unwrap();

        let (status, started_game_id): (String, Option<uuid::Uuid>) = sqlx::query_as(
            "SELECT status, started_game_id FROM game_proposals WHERE id = $1",
        )
        .bind(proposal_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let game_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM games g \
             JOIN game_versions gv ON gv.id = g.game_version_id \
             WHERE gv.id = $1",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            game_count, 0,
            "no game must be started on a stale roster: the owner declined \
             while the game-service call was in-flight (F-135)"
        );
        assert_ne!(
            status, "started",
            "proposal must not be flipped to 'started' when the roster changed \
             during the external call (F-135)"
        );
        assert!(
            started_game_id.is_none(),
            "started_game_id must remain NULL when the roster is stale (F-135)"
        );

        let owner_response: String = sqlx::query_scalar(
            "SELECT response FROM game_proposal_players WHERE proposal_id = $1 AND \"position\" = 0",
        )
        .bind(proposal_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            owner_response, "declined",
            "the concurrent owner decline must have persisted"
        );

        let invitee_response: String = sqlx::query_scalar(
            "SELECT response FROM game_proposal_players WHERE proposal_id = $1 AND \"position\" = 1",
        )
        .bind(proposal_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            invitee_response, "accepted",
            "the invitee's acceptance must have been recorded"
        );

        assert!(
            matches!(outcome, RouteOutcome::Done | RouteOutcome::Retry),
            "outcome must preserve RouteOutcome semantics (F-135)"
        );
    }

    // R-20 / C2 (F-179): handle_invite_reply auto-start produces exactly one
    // mail per on-turn invitee - no duplicate from a restored notify_game_emails
    // or notify_started. Token state cannot prove this (notify_started mints no
    // game-player token and ensure_email_token is idempotent), so this taps the
    // real send choke point via `email::outbound::test_events` and counts the
    // actual mails recorded to the on-turn player's address through the real
    // handle_invite_reply auto-start path. Restoring either removed duplicate
    // adds a second mail to that same address (both prefs are enabled above),
    // so the count assertion fails.
    #[sqlx::test]
    async fn invite_accept_auto_start_one_mail_per_on_turn_invitee(pool: sqlx::PgPool) {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::Request;
        use tokio::net::TcpListener;

        let owner_id = seed_user(&pool, "c2-owner").await;
        let invitee_id = seed_user(&pool, "c2-invitee").await;
        for uid in [owner_id, invitee_id] {
            sqlx::query(
                "INSERT INTO user_emails (user_id, email, is_primary, verified_at) \
                 VALUES ($1, $2, true, NOW())",
            )
            .bind(uid)
            .bind(format!("c2-{}@brdg.me", uuid::Uuid::new_v4().simple()))
            .execute(&pool)
            .await
            .unwrap();
            // Enable BOTH preference gates: the live game-start mail is gated by
            // `turn_emails_enabled`, while a restored `notify_started` duplicate
            // would be gated by `invite_emails_enabled`. Turning both on ensures
            // restoring either duplicate actually produces a mail the seam can
            // count (C2/F-179), rather than being silently suppressed by pref.
            sqlx::query(
                "UPDATE users SET turn_emails_enabled = true, invite_emails_enabled = true \
                 WHERE id = $1",
            )
            .bind(uid)
            .execute(&pool)
            .await
            .unwrap();
        }

        let app = Router::new().route(
            "/",
            post(|Json(payload): Json<Request>| async move {
                let players = match payload {
                    Request::New { players, .. } => players,
                    _ => 2,
                };
                Json(new_game_response(players))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let mock_uri = format!("http://{addr}");

        let game_type_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("C2 Game {}", uuid::Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(&pool)
        .await
        .unwrap();
        let game_version_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated) \
             VALUES ($1, $2, $3, true, false) RETURNING id",
        )
        .bind(game_type_id)
        .bind(format!("c2-mock-{}", uuid::Uuid::new_v4().simple()))
        .bind(&mock_uri)
        .fetch_one(&pool)
        .await
        .unwrap();
        let proposal_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status) \
             VALUES ($1, $2, 'open') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_proposal_players \
               (proposal_id, \"position\", user_id, response, email_token) \
             VALUES ($1, 0, $2, 'accepted', NULL)",
        )
        .bind(proposal_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
        let token = "c2-invite-tok";
        sqlx::query(
            "INSERT INTO game_proposal_players \
               (proposal_id, \"position\", user_id, response, email_token) \
             VALUES ($1, 1, $2, 'pending', $3)",
        )
        .bind(proposal_id)
        .bind(invitee_id)
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();

        let invitee_email: String = sqlx::query_scalar(
            "SELECT email FROM user_emails WHERE user_id = $1 AND is_primary = true",
        )
        .bind(invitee_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let email_id = "em_c2_auto_start";
        let raw = "Authentication-Results: amazonses.com; spf=pass smtp.mailfrom=brdg.me\r\n\
                   MIME-Version: 1.0\r\n\
                   Content-Type: text/plain; charset=utf-8\r\n\
                   \r\n\
                   accept\r\n";
        let mut emails = std::collections::HashMap::new();
        emails.insert(email_id.to_string(), raw.to_string());
        let source = StaticInbound(emails);
        let state = make_inbound_test_state(pool.clone()).await;

        let outcome =
            handle_invite_reply(&state, &source, token, &invitee_email, email_id).await;
        assert!(
            matches!(outcome, RouteOutcome::Done),
            "invite accept auto-start must complete as Done"
        );

        let started: Option<uuid::Uuid> = sqlx::query_scalar(
            "SELECT started_game_id FROM game_proposals WHERE id = $1",
        )
        .bind(proposal_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let _game_id = started.expect("proposal must have started a game");

        // The on-turn player is the owner (position 0); the accept reply mail
        // goes to the invitee who replied, so the owner receives ONLY the
        // game-start notification. Counting the actual mails recorded to the
        // owner's address through the send seam is the direct observable: it is
        // exactly one today, and a restored notify_game_emails or notify_started
        // duplicate would make it two.
        let owner_email: String = sqlx::query_scalar(
            "SELECT email FROM user_emails WHERE user_id = $1 AND is_primary = true",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let owner_mails = crate::email::outbound::test_events::mails_to(&owner_email);
        assert_eq!(
            owner_mails.len(),
            1,
            "the on-turn invitee must receive exactly one game-start mail; got {} \
             (subjects: {:?}) - a second indicates a restored notify_game_emails \
             or notify_started duplicate (C2/F-179)",
            owner_mails.len(),
            owner_mails.iter().map(|m| &m.subject).collect::<Vec<_>>()
        );
    }
}
