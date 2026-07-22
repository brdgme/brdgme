//! #22b email outbound plumbing: web-activity tracking (for active-web
//! suppression of turn emails), the single send choke point, per-player reply
//! tokens, and recipient resolution. Turn-transition call sites wire into these
//! helpers; nothing here sends on its own. SSR-only like the rest of `email`.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use sqlx::PgPool;
use uuid::Uuid;

/// v1 in-memory throttle, per-process: write `users.last_seen_at` at most once
/// per minute per user per process, so a busy authenticated user does not cost
/// a DB write per request. Per-process (not replica-safe) by design - presence
/// only needs to be approximate, and a restart or a second pod merely allows
/// one extra stamp per window.
pub const ACTIVITY_WRITE_THROTTLE: std::time::Duration = std::time::Duration::from_secs(60);

/// Per-user last-stamp map backing `throttle_allows`.
static ACTIVITY_RECORDED: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<Uuid, std::time::Instant>>,
> = std::sync::OnceLock::new();

/// Whether a `last_seen_at` write is allowed for `user_id` right now: true (and
/// records the stamp) only when the user has no entry or theirs is older than
/// `ACTIVITY_WRITE_THROTTLE`. Critical section is the map lookup/insert only.
fn throttle_allows(user_id: Uuid) -> bool {
    let map =
        ACTIVITY_RECORDED.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    match guard.get(&user_id) {
        Some(last) if now.duration_since(*last) < ACTIVITY_WRITE_THROTTLE => false,
        _ => {
            guard.insert(user_id, now);
            true
        }
    }
}

/// Stamps `users.last_seen_at = NOW()` for the user, throttled by
/// `ACTIVITY_WRITE_THROTTLE`. Never fails a request: a DB error is logged and
/// swallowed. No `updated_at` bump - this is a lightweight presence stamp.
pub async fn record_web_activity(pool: &PgPool, user_id: Uuid) {
    if !throttle_allows(user_id) {
        return;
    }
    if let Err(e) = sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
    {
        tracing::error!("Failed to record web activity for {}: {}", user_id, e);
    }
}

/// Axum middleware recording web activity for the authenticated user (throttled)
/// on every request. Must sit inside `session_layer` so the `Session` is already
/// in extensions when this runs; anonymous requests are a no-op.
pub async fn track_activity(
    State(pool): State<PgPool>,
    session: tower_sessions::Session,
    request: Request,
    next: Next,
) -> Response {
    if let Some(user) = crate::auth::session::get_user_from_session(&session).await {
        record_web_activity(&pool, user.id).await;
    }
    next.run(request).await
}

/// Parses a human duration like `"1 hour"`, `"30m"`, `"3600"`, `"2 days"` into
/// a `Duration`. A bare number is seconds; units (case-insensitive) are
/// `s`/`sec`/`second`/`seconds`, `m`/`min`/`minute`/`minutes`, `h`/`hour`/
/// `hours`, `d`/`day`/`days`. Returns `None` on empty input, no leading number,
/// an unknown unit, or trailing junk.
pub fn parse_duration(raw: &str) -> Option<std::time::Duration> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let num_end = raw
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(raw.len());
    if num_end == 0 {
        return None;
    }
    let n: u64 = raw[..num_end].parse().ok()?;
    let mult: u64 = match raw[num_end..].trim().to_ascii_lowercase().as_str() {
        "" | "s" | "sec" | "second" | "seconds" => 1,
        "m" | "min" | "minute" | "minutes" => 60,
        "h" | "hour" | "hours" => 3600,
        "d" | "day" | "days" => 86400,
        _ => return None,
    };
    Some(std::time::Duration::from_secs(n.saturating_mul(mult)))
}

/// Default active-web suppression window (1 hour) when the env var is unset or
/// unparseable.
pub const DEFAULT_SUPPRESS_WINDOW: std::time::Duration = std::time::Duration::from_secs(3600);

/// Resolves an optional raw duration string to a window, falling back to
/// `DEFAULT_SUPPRESS_WINDOW` on `None` or an unparseable value.
pub fn window_from(raw: Option<&str>) -> std::time::Duration {
    raw.and_then(parse_duration)
        .unwrap_or(DEFAULT_SUPPRESS_WINDOW)
}

/// The active-web suppression window, read from `EMAIL_SUPPRESS_IF_ACTIVE_WITHIN`
/// (a human duration, see `parse_duration`); defaults to 1 hour. A user active
/// on the web within this window is not emailed a turn notification.
pub fn suppress_window() -> std::time::Duration {
    window_from(
        std::env::var("EMAIL_SUPPRESS_IF_ACTIVE_WITHIN")
            .ok()
            .as_deref(),
    )
}

/// Pure predicate: was the user last seen within `window` of `now`? `None`
/// (never active) => `false`, so we send.
pub fn is_recently_active_at(
    last_seen_at: Option<time::PrimitiveDateTime>,
    now: time::PrimitiveDateTime,
    window: std::time::Duration,
) -> bool {
    let Some(last_seen_at) = last_seen_at else {
        return false;
    };
    let window = time::Duration::try_from(window).unwrap_or(time::Duration::hours(1));
    (now - last_seen_at) < window
}

fn now_utc() -> time::PrimitiveDateTime {
    let t = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(t.date(), t.time())
}

/// Whether the user was active on the web within `suppress_window()`. Fails open
/// (returns `false` => send) on a DB error or a missing user row.
pub async fn is_recently_active(pool: &PgPool, user_id: Uuid) -> bool {
    let row = sqlx::query_as::<_, (Option<time::PrimitiveDateTime>,)>(
        "SELECT last_seen_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await;
    let row = match row {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to read last_seen_at for {}: {}", user_id, e);
            return false;
        }
    };
    is_recently_active_at(row.and_then(|(l,)| l), now_utc(), suppress_window())
}

/// The single send choke point for rendered game emails. Mirrors
/// `auth::server::send_login_email`: with no `RESEND_API_KEY` (dev default) it
/// logs the email instead of sending; otherwise it counts the send and posts to
/// Resend, folding in the rendered threading/unsubscribe headers.
pub async fn try_send_rendered_email(
    resend: Option<&resend_rs::Resend>,
    email: crate::email::render::RenderedEmail,
    to: &str,
) -> bool {
    let Some(resend) = resend else {
        println!(
            "\n==> GAME EMAIL for {}\nSubject: {}\n\n{}\n",
            to, email.subject, email.text
        );
        return true;
    };
    axum_prometheus::metrics::counter!("game_emails_sent_total").increment(1);
    let from_addr =
        std::env::var("EMAIL_FROM").unwrap_or_else(|_| "brdg.me <mail@brdg.me>".to_string());
    let mut opts = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to.to_string()],
        email.subject.clone(),
    )
    .with_text(&email.text)
    .with_html(&email.html);
    for (k, v) in email.headers {
        opts = opts.with_header(&k, &v);
    }
    match resend.emails.send(opts).await {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("Failed to send email to {}: {}", to, e);
            false
        }
    }
}

pub async fn send_rendered_email(
    resend: Option<&resend_rs::Resend>,
    email: crate::email::render::RenderedEmail,
    to: &str,
) {
    try_send_rendered_email(resend, email, to).await;
}

/// 32-char `[a-zA-Z0-9]` (url-safe) reply token for the per-player Reply-To
/// address (`g-{token}@brdg.me`).
fn generate_email_token() -> String {
    use rand::RngExt as _;
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Returns the player's `email_token`, generating and persisting one on first
/// use (lazy population, per migration 014). Plain query, matching
/// `db::get_user_theme`.
pub async fn ensure_email_token(pool: &PgPool, game_player_id: Uuid) -> anyhow::Result<String> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT email_token FROM game_players WHERE id = $1")
            .bind(game_player_id)
            .fetch_optional(pool)
            .await?;
    if let Some((Some(tok),)) = row {
        return Ok(tok);
    }
    let token = generate_email_token();
    sqlx::query("UPDATE game_players SET email_token = $1, updated_at = NOW() WHERE id = $2")
        .bind(&token)
        .bind(game_player_id)
        .execute(pool)
        .await?;
    Ok(token)
}

/// Everything needed to decide whether (and how) to email one game-player slot:
/// the verified primary address, the recipient's theme, the account opt-out,
/// last web activity (for active-web suppression), and whether the slot is a
/// bot. Plain query, matching `db::get_user_theme`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailRecipient {
    pub email: Option<String>,
    pub theme_slug: Option<String>,
    pub turn_emails_enabled: bool,
    pub last_seen_at: Option<time::PrimitiveDateTime>,
    pub is_bot: bool,
}

/// Resolves the recipient data for a `game_players` slot, or `None` if the slot
/// does not exist. The address comes from the user's verified primary
/// `user_emails` row; bots (slots with a `game_bot_id`) have no user and thus no
/// address.
pub async fn fetch_email_recipient(
    pool: &PgPool,
    game_player_id: Uuid,
) -> anyhow::Result<Option<EmailRecipient>> {
    let row = sqlx::query_as::<_, EmailRecipient>(
        "SELECT
            ue.email AS email,
            u.theme AS theme_slug,
            COALESCE(u.turn_emails_enabled, false) AS turn_emails_enabled,
            u.last_seen_at AS last_seen_at,
            (gp.game_bot_id IS NOT NULL) AS is_bot
        FROM game_players gp
        LEFT JOIN users u ON gp.user_id = u.id
        LEFT JOIN user_emails ue ON ue.user_id = u.id AND ue.is_primary AND ue.verified_at IS NOT NULL
        WHERE gp.id = $1",
    )
    .bind(game_player_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Whether a turn email should go to this recipient: NOT a bot AND has a
/// verified primary email AND `turn_emails_enabled` AND NOT recently active on
/// the web. (`email.is_some()` is implicit - you cannot mail an addressless
/// slot.)
pub fn should_email_recipient(
    recipient: &EmailRecipient,
    now: time::PrimitiveDateTime,
    window: std::time::Duration,
) -> bool {
    recipient.email.is_some()
        && !recipient.is_bot
        && recipient.turn_emails_enabled
        && !is_recently_active_at(recipient.last_seen_at, now, window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, PrimitiveDateTime, Time};

    fn fixed_now() -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        )
    }

    fn recipient(
        email: Option<&str>,
        is_bot: bool,
        enabled: bool,
        last_seen_at: Option<PrimitiveDateTime>,
    ) -> EmailRecipient {
        EmailRecipient {
            email: email.map(String::from),
            theme_slug: None,
            turn_emails_enabled: enabled,
            last_seen_at,
            is_bot,
        }
    }

    #[test]
    fn parse_duration_parses_units() {
        assert_eq!(
            parse_duration("1 hour"),
            Some(std::time::Duration::from_secs(3600))
        );
        assert_eq!(
            parse_duration("30m"),
            Some(std::time::Duration::from_secs(1800))
        );
        assert_eq!(
            parse_duration("3600"),
            Some(std::time::Duration::from_secs(3600))
        );
        assert_eq!(
            parse_duration("2 days"),
            Some(std::time::Duration::from_secs(172800))
        );
        assert_eq!(
            parse_duration("90 seconds"),
            Some(std::time::Duration::from_secs(90))
        );
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("garbage"), None);
        assert_eq!(parse_duration("12 parsecs"), None);
    }

    #[test]
    fn window_from_falls_back_to_default() {
        assert_eq!(window_from(None), std::time::Duration::from_secs(3600));
        assert_eq!(
            window_from(Some("30m")),
            std::time::Duration::from_secs(1800)
        );
        assert_eq!(
            window_from(Some("nonsense")),
            std::time::Duration::from_secs(3600)
        );
    }

    #[test]
    fn is_recently_active_at_compares_window() {
        let now = fixed_now();
        let window = std::time::Duration::from_secs(3600);
        let half_hour_ago = now - time::Duration::minutes(30);
        let two_hours_ago = now - time::Duration::hours(2);
        assert!(is_recently_active_at(Some(half_hour_ago), now, window));
        assert!(!is_recently_active_at(Some(two_hours_ago), now, window));
        assert!(!is_recently_active_at(None, now, window));
    }

    #[test]
    fn should_email_recipient_truth_table() {
        let now = fixed_now();
        let window = std::time::Duration::from_secs(3600);
        let five_min_ago = now - time::Duration::minutes(5);

        // bot
        assert!(!should_email_recipient(
            &recipient(Some("a@b.c"), true, true, None),
            now,
            window
        ));
        // opted out
        assert!(!should_email_recipient(
            &recipient(Some("a@b.c"), false, false, None),
            now,
            window
        ));
        // recently active on the web
        assert!(!should_email_recipient(
            &recipient(Some("a@b.c"), false, true, Some(five_min_ago)),
            now,
            window
        ));
        // normal human
        assert!(should_email_recipient(
            &recipient(Some("a@b.c"), false, true, None),
            now,
            window
        ));
        // addressless slot
        assert!(!should_email_recipient(
            &recipient(None, false, true, None),
            now,
            window
        ));
    }

    #[test]
    fn generate_email_token_is_url_safe_and_unique() {
        let a = generate_email_token();
        let b = generate_email_token();
        assert!(!a.is_empty());
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, b);
    }

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40). Plain queries throughout to avoid `.sqlx` churn.
    #[sqlx::test]
    async fn ensure_email_token_generates_and_reuses(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Test Game {}", Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(&pool)
        .await
        .unwrap();

        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let game_id: Uuid = sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, false, 'initial') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("player")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let game_player_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read)
             VALUES ($1, $2, 0, 'Green', true, false, NOW(), NOW(), false, false)
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let first = ensure_email_token(&pool, game_player_id).await.unwrap();
        let second = ensure_email_token(&pool, game_player_id).await.unwrap();
        assert!(!first.is_empty());
        assert_eq!(first, second);

        let stored: Option<String> =
            sqlx::query_scalar("SELECT email_token FROM game_players WHERE id = $1")
                .bind(game_player_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(first));
    }
}
