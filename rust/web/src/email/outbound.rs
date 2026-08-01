//! #22b email outbound plumbing: the single send choke point, per-player reply
//! tokens, and recipient resolution. Turn-transition call sites wire into these
//! helpers; nothing here sends on its own. SSR-only like the rest of `email`.

use sqlx::PgPool;
use uuid::Uuid;

/// Per-recipient web-presence suppression for AUTOMATED emails only: true iff
/// the recipient has a user who pinged the server within the presence window
/// (i.e. has a page open). Bots and addressless slots have no user (`None`) and
/// are never suppressed here. Direct responses to inbound email never call this.
/// Fails open (false => send) on a DB error, via `db::is_user_recently_active`.
pub async fn suppress_for_web_presence(pool: &PgPool, user_id: Option<Uuid>) -> bool {
    match user_id {
        Some(uid) => crate::db::is_user_recently_active(pool, uid).await,
        None => false,
    }
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
    // R-20 / I2,C2: test-only tap on the single send choke point. Every mail
    // any production path produces flows through here, so recording `(to,
    // subject)` with a global sequence number gives tests a direct, non-timing
    // observable of actual mails and their order relative to broadcasts.
    #[cfg(test)]
    test_events::record_mail(to, &email.subject);
    let Some(resend) = resend else {
        println!(
            "\n==> GAME EMAIL for {}\nSubject: {}\nReply-To: {}\n\n{}\n",
            to, email.subject, email.reply_to, email.text
        );
        return true;
    };
    let from_addr =
        std::env::var("EMAIL_FROM").unwrap_or_else(|_| "brdg.me <mail@brdg.me>".to_string());
    let mut opts = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to.to_string()],
        email.subject.clone(),
    )
    .with_text(&email.text)
    .with_html(&email.html)
    .with_reply(&email.reply_to);
    for (k, v) in email.headers {
        opts = opts.with_header(&k, &v);
    }
    match resend.emails.send(opts).await {
        Ok(_) => {
            axum_prometheus::metrics::counter!("game_emails_sent_total").increment(1);
            true
        }
        Err(e) => {
            axum_prometheus::metrics::counter!("game_emails_failed_total").increment(1);
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
pub(crate) fn generate_email_token() -> String {
    use rand::RngExt as _;
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Returns the player's `email_token`, generating and persisting one on first
/// use (lazy population, per migration 014). Atomic COALESCE prevents races.
pub async fn ensure_email_token(pool: &PgPool, game_player_id: Uuid) -> anyhow::Result<String> {
    let token = generate_email_token();
    let row: Option<(String,)> = sqlx::query_as(
        "UPDATE game_players SET email_token = COALESCE(email_token, $1), updated_at = NOW() WHERE id = $2 RETURNING email_token",
    )
    .bind(&token)
    .bind(game_player_id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some((tok,)) => Ok(tok),
        None => anyhow::bail!("game_player {} not found", game_player_id),
    }
}

/// Transaction-aware [`ensure_email_token`]: identical atomic COALESCE update,
/// but on the caller's connection so it can run inside a held transaction (the
/// reminder sweep's `FOR UPDATE` claim) instead of deadlocking against that
/// claim's row lock on the shared pool. Mirrors `sweep::mark_reminder_sent_tx`.
pub async fn ensure_email_token_tx(
    tx: &mut sqlx::PgConnection,
    game_player_id: Uuid,
) -> anyhow::Result<String> {
    let token = generate_email_token();
    let row: Option<(String,)> = sqlx::query_as(
        "UPDATE game_players SET email_token = COALESCE(email_token, $1), updated_at = NOW() WHERE id = $2 RETURNING email_token",
    )
    .bind(&token)
    .bind(game_player_id)
    .fetch_optional(&mut *tx)
    .await?;
    match row {
        Some((tok,)) => Ok(tok),
        None => anyhow::bail!("game_player {} not found", game_player_id),
    }
}

/// Returns the user's `settings_email_token`, generating and persisting one on
/// first use (lazy population, per the WP-56 migration). Plain query, matching
/// `ensure_email_token`.
pub async fn ensure_settings_email_token(pool: &PgPool, user_id: Uuid) -> anyhow::Result<String> {
    let row: Option<(
        Option<String>,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
    )> = sqlx::query_as(
        "SELECT settings_email_token, settings_token_expires_at, settings_token_used_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if let Some((Some(tok), Some(expires), used)) = row
        && expires > time::OffsetDateTime::now_utc()
        && used.is_none()
    {
        return Ok(tok);
    }
    let token = generate_email_token();
    sqlx::query("UPDATE users SET settings_email_token = $1, settings_token_expires_at = NOW() + interval '24 hours', settings_token_used_at = NULL, updated_at = NOW() WHERE id = $2")
        .bind(&token)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(token)
}

/// Returns the user's `unsubscribe_token`, generating and persisting one on
/// first use (lazy population, per the WP-58 migration). Plain query, matching
/// `ensure_settings_email_token`.
pub async fn ensure_unsubscribe_token(pool: &PgPool, user_id: Uuid) -> anyhow::Result<String> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT unsubscribe_token FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    if let Some((Some(tok),)) = row {
        return Ok(tok);
    }
    let token = generate_email_token();
    sqlx::query("UPDATE users SET unsubscribe_token = $1, updated_at = NOW() WHERE id = $2")
        .bind(&token)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(token)
}

/// Everything needed to decide whether (and how) to email one game-player slot:
/// the verified primary address, the recipient's theme, the account opt-out, the
/// owning user (for the per-recipient web-presence check; `None` for bots), and
/// whether the slot is a bot. Plain query, matching `db::get_user_theme`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailRecipient {
    pub email: Option<String>,
    pub theme_slug: Option<String>,
    pub turn_emails_enabled: bool,
    pub reminder_emails_enabled: bool,
    pub user_id: Option<Uuid>,
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
            COALESCE(u.reminder_emails_enabled, false) AS reminder_emails_enabled,
            gp.user_id AS user_id,
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

/// Whether an automated game email may go to this recipient: NOT a bot AND has a
/// verified primary email AND `turn_emails_enabled`. (`email.is_some()` is
/// implicit - you cannot mail an addressless slot.) The per-recipient web-
/// presence suppression is a separate check (`suppress_for_web_presence`) applied
/// at the automated send sites, never here and never for direct responses.
pub fn should_email_recipient(recipient: &EmailRecipient) -> bool {
    recipient.email.is_some() && !recipient.is_bot && recipient.turn_emails_enabled
}

/// R-20 / I2,C2: process-global, test-only event log tapped at the two choke
/// points that matter for the notification-ordering and duplicate-mail
/// properties - the mail send (`try_send_rendered_email`, above) and the
/// post-commit broadcast (`game::broadcast_and_trigger`). A single monotonic
/// sequence orders events across both, so a test can assert a mail was recorded
/// before a broadcast (I2: notify-before-broadcast, the guard against a fast bot
/// move double-mailing) or count actual mails to one recipient (C2: exactly one
/// mail per invitee). Tests filter by per-test-unique recipients/game ids, so
/// parallel `#[sqlx::test]` runs do not contaminate each other. Compiles to
/// nothing outside `cfg(test)`.
#[cfg(test)]
pub mod test_events {
    use std::sync::{LazyLock, Mutex};
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct MailEvent {
        pub seq: u64,
        pub to: String,
        pub subject: String,
    }

    #[derive(Debug, Clone)]
    pub struct BroadcastEvent {
        pub seq: u64,
        pub game_id: Uuid,
    }

    static SEQ: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));
    static MAILS: LazyLock<Mutex<Vec<MailEvent>>> = LazyLock::new(|| Mutex::new(Vec::new()));
    static BROADCASTS: LazyLock<Mutex<Vec<BroadcastEvent>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));

    fn next_seq() -> u64 {
        let mut seq = SEQ.lock().unwrap();
        *seq += 1;
        *seq
    }

    pub fn record_mail(to: &str, subject: &str) {
        MAILS.lock().unwrap().push(MailEvent {
            seq: next_seq(),
            to: to.to_string(),
            subject: subject.to_string(),
        });
    }

    pub fn record_broadcast(game_id: Uuid) {
        BROADCASTS.lock().unwrap().push(BroadcastEvent {
            seq: next_seq(),
            game_id,
        });
    }

    pub fn mails_to(to: &str) -> Vec<MailEvent> {
        MAILS
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.to == to)
            .cloned()
            .collect()
    }

    pub fn broadcasts_for(game_id: Uuid) -> Vec<BroadcastEvent> {
        BROADCASTS
            .lock()
            .unwrap()
            .iter()
            .filter(|b| b.game_id == game_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipient(email: Option<&str>, is_bot: bool, enabled: bool) -> EmailRecipient {
        EmailRecipient {
            email: email.map(String::from),
            theme_slug: None,
            turn_emails_enabled: enabled,
            reminder_emails_enabled: true,
            user_id: None,
            is_bot,
        }
    }

    #[test]
    fn should_email_recipient_truth_table() {
        // bot
        assert!(!should_email_recipient(&recipient(
            Some("a@b.c"),
            true,
            true
        )));
        // opted out
        assert!(!should_email_recipient(&recipient(
            Some("a@b.c"),
            false,
            false
        )));
        // normal human
        assert!(should_email_recipient(&recipient(
            Some("a@b.c"),
            false,
            true
        )));
        // addressless slot
        assert!(!should_email_recipient(&recipient(None, false, true)));
    }

    async fn seed_user(pool: &PgPool) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id")
            .bind(format!("u-{}", Uuid::new_v4()))
            .bind(Vec::<String>::new())
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40).
    #[sqlx::test]
    async fn suppress_for_web_presence_tracks_ping_recency(pool: PgPool) {
        let active = seed_user(&pool).await;
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(active)
            .execute(&pool)
            .await
            .unwrap();
        assert!(suppress_for_web_presence(&pool, Some(active)).await);

        let stale = seed_user(&pool).await;
        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(stale)
        .execute(&pool)
        .await
        .unwrap();
        assert!(!suppress_for_web_presence(&pool, Some(stale)).await);

        let never = seed_user(&pool).await;
        assert!(!suppress_for_web_presence(&pool, Some(never)).await);

        // Bots / addressless slots have no user and are never suppressed here.
        assert!(!suppress_for_web_presence(&pool, None).await);
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

    // Runs only where a Postgres is available (CI); expected to fail to connect
    // locally (backlog #40). Plain queries throughout to avoid `.sqlx` churn.
    #[sqlx::test]
    async fn ensure_settings_email_token_generates_and_reuses(pool: PgPool) {
        let user_id = seed_user(&pool).await;

        let first = ensure_settings_email_token(&pool, user_id).await.unwrap();
        let second = ensure_settings_email_token(&pool, user_id).await.unwrap();
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(first, second);

        let stored: Option<String> =
            sqlx::query_scalar("SELECT settings_email_token FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(first));
    }

    #[sqlx::test]
    async fn ensure_unsubscribe_token_generates_and_reuses(pool: PgPool) {
        let user_id = seed_user(&pool).await;

        let first = ensure_unsubscribe_token(&pool, user_id).await.unwrap();
        let second = ensure_unsubscribe_token(&pool, user_id).await.unwrap();
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(first, second);

        let stored: Option<String> =
            sqlx::query_scalar("SELECT unsubscribe_token FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(first));
    }
}
