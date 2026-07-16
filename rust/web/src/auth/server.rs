#[cfg(feature = "ssr")]
use crate::auth::session::{
    clear_user_session, get_user_from_session, invalidate_auth_token, set_user_session,
    validate_session_token,
};
#[cfg(feature = "ssr")]
use crate::error::internal;
#[cfg(feature = "ssr")]
use crate::models::user::{LoginConfirmation, User, UserEmail};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::extract;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use time::OffsetDateTime;
#[cfg(feature = "ssr")]
use tower_sessions::Session;
use uuid::Uuid;

/// Suppress re-sends for the same email inside this window (idempotent
/// resend shield against double-clicks and scripted hammering).
#[cfg(feature = "ssr")]
const LOGIN_RESEND_COOLDOWN_SECS: i64 = 60;

/// Max emails sent per address while its code is valid (1 hour window).
#[cfg(feature = "ssr")]
const LOGIN_MAX_SENDS_PER_EMAIL: i32 = 5;

/// Max login emails sent platform-wide per 24h, protecting the Resend
/// free-tier 100/day quota with headroom. DB-backed, so replica-safe.
#[cfg(feature = "ssr")]
const LOGIN_GLOBAL_MAX_SENDS_PER_DAY: i64 = 50;

/// Failed confirm attempts allowed against a single code before it is dead.
#[cfg(feature = "ssr")]
const CONFIRM_MAX_ATTEMPTS_PER_CODE: i32 = 10;

/// Postgres advisory-lock key serializing the send-cap check-and-bump in
/// `login()` across concurrent requests (any email). A global lock rather
/// than a per-email row lock because the 24h cap sums over every row, not
/// just the requesting email's; the endpoint is already IP-rate-limited and
/// capped at 50 sends/day, so serializing the whole (fast, DB-only) decision
/// section has no meaningful throughput cost. Arbitrary constant, just needs
/// to not collide with another advisory lock key in this codebase.
#[cfg(feature = "ssr")]
const LOGIN_CAP_LOCK_KEY: i64 = 0x6c6f_6769_6e63_6170; // "loginc" + "ap" bytes, no meaning beyond uniqueness

/// Builds the login-confirmation email's plain-text and HTML bodies. Pure
/// (no I/O), so it is unit-testable without a Resend account.
/// `code_valid` in `login()` allows a 1-hour validity window - this copy
/// must stay in sync with that if the window ever changes.
#[cfg(feature = "ssr")]
fn login_email_bodies(token: &str) -> (String, String) {
    let text = format!(
        "Your brdg.me confirmation is {token}\n\n\
         This confirmation will expire in 1 hour if not used."
    );
    let html = format!(
        r#"<link
    href="https://fonts.googleapis.com/css?family=Source+Code+Pro:400,700"
    rel="stylesheet"
>
<pre
    style="
        background-color: white;
        color: black;
        font-family: 'Source Code Pro', 'Lucida Console', monospace;
    "
>Your brdg.me confirmation is <b>{token}</b>

This confirmation will expire in 1 hour if not used.</pre>"#
    );
    (text, html)
}

#[cfg(feature = "ssr")]
async fn send_login_email(resend: Option<&resend_rs::Resend>, to_email: &str, token: &str) {
    let Some(resend) = resend else {
        // No RESEND_API_KEY configured (dev default): log instead of sending.
        println!("\n==> LOGIN CODE for {}: {}\n", to_email, token);
        return;
    };

    // Counts actual Resend API calls only (feeds the Resend quota alert), not
    // the dev-mode logging fallback above which never touches Resend at all.
    axum_prometheus::metrics::counter!("login_emails_sent_total").increment(1);

    let from_addr = std::env::var("EMAIL_FROM").unwrap_or_else(|_| "login@brdg.me".to_string());
    let (text_body, html_body) = login_email_bodies(token);
    let email = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to_email.to_string()],
        "brdg.me login confirmation",
    )
    .with_text(&text_body)
    .with_html(&html_body);

    if let Err(e) = resend.emails.send(email).await {
        tracing::error!("Failed to send login email to {}: {}", to_email, e);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[server(Login, "/api")]
pub async fn login(email: String) -> Result<LoginResponse, ServerFnError> {
    if email.is_empty() || !email.contains('@') {
        return Ok(LoginResponse {
            success: false,
            message: "Invalid email address".to_string(),
        });
    }

    let pool = expect_context::<PgPool>();

    // Everything below - GC, the cap checks, and the upsert that bumps the
    // counters they read - runs in one transaction guarded by a global
    // advisory lock. Without it, concurrent requests can each pass the cap
    // SELECTs before either upsert commits (TOCTOU), overshooting the
    // per-email and global caps by roughly the concurrency level.
    let mut tx = pool
        .begin()
        .await
        .map_err(internal("login: begin transaction"))?;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", LOGIN_CAP_LOCK_KEY)
        .execute(&mut *tx)
        .await
        .map_err(internal("login: acquire cap lock"))?;

    // Opportunistic GC: rows are useless for confirm after 1 hour, but they
    // still feed the 24h global send cap below, so only delete once they have
    // aged out of that accounting window too. No cron/job needed.
    sqlx::query!(
        "DELETE FROM login_confirmations WHERE last_sent_at < NOW() - INTERVAL '24 hours'"
    )
    .execute(&mut *tx)
    .await
    .map_err(internal("login: gc stale confirmations"))?;

    // The generic response returned whether we sent an email or a cooldown /
    // per-email cap suppressed it - it must be indistinguishable so the
    // endpoint is not an enumeration or behaviour oracle.
    let generic_success = LoginResponse {
        success: true,
        message: "Login email sent".to_string(),
    };

    let existing = sqlx::query_as!(
        LoginConfirmation,
        "SELECT * FROM login_confirmations WHERE email = $1",
        email
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("login: look up confirmation"))?;

    let now = OffsetDateTime::now_utc();
    if let Some(row) = &existing {
        let in_cooldown = row
            .last_sent_at
            .is_some_and(|at| at > now - time::Duration::seconds(LOGIN_RESEND_COOLDOWN_SECS));
        if in_cooldown {
            tx.commit()
                .await
                .map_err(internal("login: commit transaction"))?;
            return Ok(generic_success);
        }
        let code_valid = row.created_at > now - time::Duration::hours(1);
        if code_valid && row.sent_count >= LOGIN_MAX_SENDS_PER_EMAIL {
            tx.commit()
                .await
                .map_err(internal("login: commit transaction"))?;
            return Ok(generic_success);
        }
    }

    // Global cap protecting the Resend 100/day quota. Unlike edge/per-process
    // limits, these caps live in Postgres so they hold across replicas and
    // deploys (the per-IP edge limit is Cloudflare's, see the 2026-07-10 WP4
    // spec W6). This one affects legit users, so it is an honest refusal,
    // not a pretend-success.
    let sent_last_24h = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(sent_count), 0) AS "total!"
           FROM login_confirmations
           WHERE last_sent_at > NOW() - INTERVAL '24 hours'"#
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("login: sum sends for global cap"))?;
    if sent_last_24h >= LOGIN_GLOBAL_MAX_SENDS_PER_DAY {
        tx.commit()
            .await
            .map_err(internal("login: commit transaction"))?;
        axum_prometheus::metrics::counter!("login_email_cap_hit_total").increment(1);
        return Ok(LoginResponse {
            success: false,
            message: "Logins are temporarily limited, try again later.".to_string(),
        });
    }

    // Fresh code (restarting the validity window and per-code attempt count)
    // if the existing one expired, otherwise re-send the existing code.
    // sent_count keeps accumulating across windows so the caps above stay
    // honest; it only resets when the row is deleted (confirm or 24h GC).
    let fresh_code = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let code = sqlx::query_scalar!(
        r#"INSERT INTO login_confirmations (email, code, sent_count, last_sent_at)
           VALUES ($1, $2, 1, NOW())
           ON CONFLICT (email) DO UPDATE SET
               code = CASE WHEN login_confirmations.created_at <= NOW() - INTERVAL '1 hour'
                           THEN EXCLUDED.code ELSE login_confirmations.code END,
               created_at = CASE WHEN login_confirmations.created_at <= NOW() - INTERVAL '1 hour'
                                 THEN NOW() ELSE login_confirmations.created_at END,
               attempts = CASE WHEN login_confirmations.created_at <= NOW() - INTERVAL '1 hour'
                               THEN 0 ELSE login_confirmations.attempts END,
               sent_count = login_confirmations.sent_count + 1,
               last_sent_at = NOW()
           RETURNING code"#,
        email,
        fresh_code
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(internal("login: upsert confirmation"))?;

    tx.commit()
        .await
        .map_err(internal("login: commit transaction"))?;

    let resend = expect_context::<Option<resend_rs::Resend>>();
    send_login_email(resend.as_ref(), &email, &code).await;

    Ok(generic_success)
}

#[server(ConfirmLogin, "/api")]
pub async fn confirm_login(email: String, token: String) -> Result<AuthUser, ServerFnError> {
    // Extract the session before touching the database so a harness without
    // request parts fails here, not after user/token rows were written.
    let session: Session = extract()
        .await
        .map_err(internal("confirm_login: extract session"))?;

    let pool = expect_context::<PgPool>();
    let confirmed = confirm_login_inner(&pool, &email, &token).await?;

    set_user_session(
        &session,
        &confirmed.user,
        &confirmed.email,
        confirmed.auth_token_id,
    )
    .await
    .map_err(internal("confirm_login: set session"))?;

    Ok(AuthUser {
        id: confirmed.user.id,
        name: confirmed.user.name,
        email: confirmed.email,
    })
}

#[cfg(feature = "ssr")]
struct ConfirmedLogin {
    user: User,
    email: String,
    auth_token_id: Uuid,
}

/// Everything `confirm_login` does apart from the per-IP rate limit and the
/// session write, so tests can drive the confirm flow without HTTP request
/// parts. The user row is created here - not in `login()` - so unconfirmed
/// emails never touch the `users` table.
#[cfg(feature = "ssr")]
async fn confirm_login_inner(
    pool: &PgPool,
    email: &str,
    token: &str,
) -> Result<ConfirmedLogin, ServerFnError> {
    let invalid = || ServerFnError::new("Invalid or expired token".to_string());

    let confirmation = sqlx::query_as!(
        LoginConfirmation,
        "SELECT * FROM login_confirmations WHERE email = $1",
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(internal("confirm_login: look up confirmation"))?
    .ok_or_else(invalid)?;

    if confirmation.created_at <= OffsetDateTime::now_utc() - time::Duration::hours(1) {
        return Err(invalid());
    }
    // The real brute-force control: 10 attempts per code, independent of
    // source IP (per-IP limiting is a collective bucket on DOKS - see D6).
    if confirmation.attempts >= CONFIRM_MAX_ATTEMPTS_PER_CODE {
        axum_prometheus::metrics::counter!("login_confirm_attempt_cap_hit_total").increment(1);
        return Err(invalid());
    }
    if confirmation.code != token {
        sqlx::query!(
            "UPDATE login_confirmations SET attempts = attempts + 1 WHERE email = $1",
            email
        )
        .execute(pool)
        .await
        .map_err(internal("confirm_login: count failed attempt"))?;
        return Err(invalid());
    }

    // Accepted race: two concurrent confirms with the same valid code can
    // both pass the pre-checks above; the loser hits the user_emails unique
    // constraint below and surfaces a generic internal error. Self-recovers
    // on retry, so not worth locking around.
    let mut tx = pool
        .begin()
        .await
        .map_err(internal("confirm_login: begin transaction"))?;

    let user_email = sqlx::query_as!(
        UserEmail,
        "SELECT * FROM user_emails WHERE email = $1 AND is_primary = true",
        email
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("confirm_login: look up user email"))?;

    let user_id = if let Some(user_email) = user_email {
        user_email.user_id
    } else {
        let new_user_id = Uuid::new_v4();
        let username = crate::db::generate_unique_username(&mut *tx)
            .await
            .map_err(internal("confirm_login: generate username"))?;

        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            new_user_id,
            username,
            &Vec::<String>::new()
        )
        .execute(&mut *tx)
        .await
        .map_err(internal("confirm_login: create user"))?;

        sqlx::query!(
            "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
            new_user_id,
            email
        )
        .execute(&mut *tx)
        .await
        .map_err(internal("confirm_login: create user email"))?;

        new_user_id
    };

    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal("confirm_login: load user"))?;

    let auth_token_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
        auth_token_id,
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(internal("confirm_login: create auth token"))?;

    sqlx::query!("DELETE FROM login_confirmations WHERE email = $1", email)
        .execute(&mut *tx)
        .await
        .map_err(internal("confirm_login: delete confirmation"))?;

    tx.commit()
        .await
        .map_err(internal("confirm_login: commit transaction"))?;

    Ok(ConfirmedLogin {
        user,
        email: email.to_string(),
        auth_token_id,
    })
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session: Session = extract()
        .await
        .map_err(internal("get_current_user: extract session"))?;
    let session_user = get_user_from_session(&session).await;

    if let Some(user) = session_user {
        let pool = expect_context::<PgPool>();
        // Validate token matches database
        if validate_session_token(&pool, user.auth_token_id)
            .await
            .unwrap_or(false)
        {
            return Ok(Some(AuthUser {
                id: user.id,
                name: user.name,
                email: user.email,
            }));
        } else {
            // Token invalid, clear session
            let _ = clear_user_session(&session).await;
        }
    }

    Ok(None)
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<bool, ServerFnError> {
    let session: Session = extract()
        .await
        .map_err(internal("logout: extract session"))?;

    // Get user to check for auth token to invalidate
    if let Some(user) = get_user_from_session(&session).await {
        let pool = expect_context::<PgPool>();
        let _ = invalidate_auth_token(&pool, user.auth_token_id).await;
    }

    clear_user_session(&session)
        .await
        .map_err(internal("logout: clear session"))?;

    Ok(true)
}

/// Persists the caller's theme choice to their profile (client-side already
/// applied the cookie/`data-theme` before calling this - see `theme.rs` in
/// the components layer). `None` means "system".
#[server(SetTheme, "/api")]
pub async fn set_theme(theme: Option<String>) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    if let Some(t) = &theme
        && !crate::theme::is_known_slug(t)
    {
        return Err(ServerFnError::new("Unknown theme"));
    }

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::set_user_theme(&pool, user.id, theme.as_deref())
        .await
        .map_err(internal("set_theme: update"))
}

/// The signed-in user's stored theme preference, or `None` for anonymous
/// visitors / no preference set ("system"). Fetched client-side right after
/// login so the profile's theme wins over whatever was showing pre-login.
#[server(GetUserTheme, "/api")]
pub async fn get_user_theme() -> Result<Option<String>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    match get_current_user().await? {
        Some(user) => crate::db::get_user_theme(&pool, user.id)
            .await
            .map_err(internal("get_user_theme: load")),
        None => Ok(None),
    }
}

/// Everything the settings page needs in one round trip. `pref_colors` is
/// always exactly 3 entries: stored prefs normalized, or the palette-order
/// default (Green, Red, Blue) when unset - behaviour-neutral since identical
/// prefs resolve by rank with random tiebreak (see db.rs::choose_colors).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsData {
    pub name: String,
    pub email: String,
    pub pref_colors: Vec<String>,
}

#[server(GetSettings, "/api")]
pub async fn get_settings() -> Result<SettingsData, ServerFnError> {
    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let mut pref_colors = crate::db::get_user_pref_colors(&pool, user.id)
        .await
        .map_err(internal("get_settings: load pref colors"))?;
    if !validate_pref_colors(&pref_colors) {
        pref_colors = vec!["Green".to_string(), "Red".to_string(), "Blue".to_string()];
    }

    Ok(SettingsData {
        name: user.name,
        email: user.email,
        pref_colors,
    })
}

/// Renames the caller. `Ok(None)` on success; `Ok(Some(message))` is a field
/// error to render inline (validation or uniqueness) - not a ServerFnError,
/// so the form can distinguish expected rejections from transport failures.
#[server(SetUsername, "/api")]
pub async fn set_username(name: String) -> Result<Option<String>, ServerFnError> {
    if !crate::db::validate_username(&name) {
        return Ok(Some(
            "1-16 characters: letters, numbers, - and _. Must be unique.".to_string(),
        ));
    }

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    match crate::db::set_user_name(&pool, user.id, &name)
        .await
        .map_err(internal("set_username: update"))?
    {
        true => Ok(None),
        false => Ok(Some("That name is taken".to_string())),
    }
}

/// Exactly 3 distinct canonical palette colour names, in preference order.
/// Pure so it is unit-testable; `set_pref_colors` is the only caller.
pub fn validate_pref_colors(colors: &[String]) -> bool {
    colors.len() == 3
        && colors
            .iter()
            .all(|c| crate::theme::PLAYER_COLOR_NAMES.contains(&c.as_str()))
        && colors[0] != colors[1]
        && colors[0] != colors[2]
        && colors[1] != colors[2]
}

#[server(SetPrefColors, "/api")]
pub async fn set_pref_colors(colors: Vec<String>) -> Result<(), ServerFnError> {
    if !validate_pref_colors(&colors) {
        return Err(ServerFnError::new("Invalid colour preferences"));
    }

    let pool = expect_context::<PgPool>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::set_user_pref_colors(&pool, user.id, &colors)
        .await
        .map_err(internal("set_pref_colors: update"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    async fn with_pool_context<F, Fut, T>(pool: &PgPool, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let owner = Owner::new();
        owner.with(|| {
            provide_context(pool.clone());
            provide_context(None::<resend_rs::Resend>);
        });
        owner
            .with(|| leptos::reactive::computed::ScopedFuture::new(f()))
            .await
    }

    async fn get_confirmation(
        pool: &PgPool,
        email: &str,
    ) -> Option<crate::models::user::LoginConfirmation> {
        sqlx::query_as!(
            crate::models::user::LoginConfirmation,
            "SELECT * FROM login_confirmations WHERE email = $1",
            email
        )
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    async fn seed_confirmation(
        pool: &PgPool,
        email: &str,
        code: &str,
        code_age: time::Duration,
        attempts: i32,
        sent_count: i32,
        last_sent_age: time::Duration,
    ) {
        let now = OffsetDateTime::now_utc();
        sqlx::query!(
            "INSERT INTO login_confirmations
                 (email, code, created_at, attempts, sent_count, last_sent_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
            email,
            code,
            now - code_age,
            attempts,
            sent_count,
            now - last_sent_age
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn user_count(pool: &PgPool) -> i64 {
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM users"#)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn login_rejects_invalid_email(pool: PgPool) {
        let resp = with_pool_context(&pool, || login("not-an-email".to_string()))
            .await
            .unwrap();
        assert!(!resp.success);
    }

    #[sqlx::test]
    async fn login_creates_confirmation_but_no_user(pool: PgPool) {
        let email = "new-user@example.com";
        let resp = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert!(resp.success);

        assert_eq!(user_count(&pool).await, 0, "no user row until confirm");
        let row = get_confirmation(&pool, email).await.expect("row upserted");
        assert_eq!(row.code.len(), 6);
        assert_eq!(row.sent_count, 1);
        assert!(row.last_sent_at.is_some());
    }

    #[sqlx::test]
    async fn login_cooldown_suppresses_resend_with_identical_response(pool: PgPool) {
        let email = "cooldown@example.com";
        let first = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        let code = get_confirmation(&pool, email).await.unwrap().code;

        let second = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert_eq!(first.success, second.success);
        assert_eq!(first.message, second.message);

        let row = get_confirmation(&pool, email).await.unwrap();
        assert_eq!(row.sent_count, 1, "second send within 60s suppressed");
        assert_eq!(row.code, code, "code unchanged");
    }

    #[sqlx::test]
    async fn login_per_email_cap_suppresses_send(pool: PgPool) {
        let email = "capped@example.com";
        // 5 sends already this window; last one past the 60s cooldown so the
        // cap (not the cooldown) is what suppresses.
        seed_confirmation(
            &pool,
            email,
            "111111",
            time::Duration::minutes(10),
            0,
            5,
            time::Duration::minutes(5),
        )
        .await;

        let resp = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert!(resp.success, "cap must be indistinguishable from success");

        let row = get_confirmation(&pool, email).await.unwrap();
        assert_eq!(row.sent_count, 5, "no further send counted");
        assert_eq!(row.code, "111111", "code unchanged");
    }

    #[sqlx::test]
    async fn login_global_cap_refuses_honestly(pool: PgPool) {
        // 10 other emails with 5 sends each in the last 24h = 50 total.
        for i in 0..10 {
            seed_confirmation(
                &pool,
                &format!("burner-{i}@example.com"),
                "222222",
                time::Duration::minutes(30),
                0,
                5,
                time::Duration::minutes(30),
            )
            .await;
        }

        let email = "legit@example.com";
        let resp = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert!(!resp.success, "global cap is an honest refusal");
        assert!(resp.message.contains("temporarily limited"));
        assert!(
            get_confirmation(&pool, email).await.is_none(),
            "no row created while globally capped"
        );
    }

    #[sqlx::test]
    async fn login_expired_row_gets_fresh_code_and_attempts_reset(pool: PgPool) {
        let email = "expired-code@example.com";
        seed_confirmation(
            &pool,
            email,
            "333333",
            time::Duration::hours(2),
            7,
            3,
            time::Duration::hours(2),
        )
        .await;

        let resp = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert!(resp.success);

        let row = get_confirmation(&pool, email).await.unwrap();
        assert_eq!(row.attempts, 0, "attempts are per-code, reset with it");
        assert_eq!(row.sent_count, 4, "sent_count keeps accumulating");
        assert!(
            row.created_at > OffsetDateTime::now_utc() - time::Duration::minutes(1),
            "validity window restarted"
        );
    }

    #[sqlx::test]
    async fn login_deletes_rows_older_than_global_cap_window(pool: PgPool) {
        seed_confirmation(
            &pool,
            "stale@example.com",
            "444444",
            time::Duration::hours(30),
            0,
            5,
            time::Duration::hours(30),
        )
        .await;

        with_pool_context(&pool, || login("fresh@example.com".to_string()))
            .await
            .unwrap();

        assert!(
            get_confirmation(&pool, "stale@example.com").await.is_none(),
            "rows outside the 24h accounting window are GC'd on login"
        );
    }

    #[sqlx::test]
    async fn login_concurrent_requests_do_not_overshoot_per_email_cap(pool: PgPool) {
        let email = "hammered@example.com";
        // One send below the cap. Without serializing the check-and-bump,
        // concurrent requests can each read `sent_count = 3` before any of
        // their upserts commit, overshooting the cap of 5. With the fix,
        // requests are processed one at a time: the first legitimately
        // bumps to 4, and every request behind it in the burst then also
        // sees a `last_sent_at` from mere moments ago and is cooldown-
        // suppressed - so in practice a burst like this yields at most one
        // real send. Either way, the cap must never be exceeded.
        seed_confirmation(
            &pool,
            email,
            "999999",
            time::Duration::minutes(10),
            0,
            3,
            time::Duration::minutes(5),
        )
        .await;

        let calls = (0..5).map(|_| with_pool_context(&pool, || login(email.to_string())));
        let results = futures_util::future::join_all(calls).await;
        for r in results {
            assert!(
                r.unwrap().success,
                "cap must be indistinguishable from success"
            );
        }

        let row = get_confirmation(&pool, email).await.unwrap();
        assert!(
            row.sent_count <= LOGIN_MAX_SENDS_PER_EMAIL,
            "concurrent requests must not overshoot the per-email cap: got {}",
            row.sent_count
        );
    }

    #[sqlx::test]
    async fn login_concurrent_requests_do_not_overshoot_global_cap(pool: PgPool) {
        // 9 other emails * 5 sends = 45 already counted in the 24h window.
        for i in 0..9 {
            seed_confirmation(
                &pool,
                &format!("burner-{i}@example.com"),
                "222222",
                time::Duration::minutes(30),
                0,
                5,
                time::Duration::minutes(30),
            )
            .await;
        }

        // 6 distinct new emails logging in concurrently: only 5 more sends
        // fit under the 50 global cap (45 + 5 = 50), so exactly one of these
        // must be honestly refused. Without a lock spanning all rows (not
        // just one email's), concurrent requests across different emails can
        // each read the same pre-upsert SUM and all pass.
        let calls = (0..6)
            .map(|i| with_pool_context(&pool, move || login(format!("legit-{i}@example.com"))));
        let results = futures_util::future::join_all(calls).await;

        let refused = results
            .iter()
            .filter(|r| !r.as_ref().unwrap().success)
            .count();
        assert_eq!(
            refused, 1,
            "exactly one request must be refused at the boundary"
        );

        let total: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(sent_count), 0) AS "total!" FROM login_confirmations"#
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            total, LOGIN_GLOBAL_MAX_SENDS_PER_DAY,
            "concurrent requests must not overshoot the global cap"
        );
    }

    // `with_pool_context` always provides `None::<resend_rs::Resend>`, so
    // every successful `login()` test above already exercises the
    // RESEND_API_KEY-unset log-fallback path in `send_login_email` (it
    // would panic/hang trying to reach the real Resend API otherwise).
    // This test exercises `send_login_email` directly for that path.
    #[tokio::test]
    async fn send_login_email_logs_when_resend_unset() {
        // Must not panic or attempt any network I/O when `resend` is `None`.
        send_login_email(None, "someone@example.com", "123456").await;
    }

    #[test]
    fn login_email_bodies_use_brdg_me_branding_and_token() {
        let (text, html) = login_email_bodies("643856");

        assert!(text.contains("Your brdg.me confirmation is 643856"));
        assert!(text.contains("expire in 1 hour"));
        assert!(
            !text.contains("brdgme"),
            "must never render unbranded 'brdgme': {text}"
        );

        assert!(html.contains("<b>643856</b>"));
        assert!(html.contains("Source Code Pro"));
        assert!(html.contains("background-color: white"));
        assert!(html.contains("color: black"));
        assert!(
            !html.contains("brdgme"),
            "must never render unbranded 'brdgme': {html}"
        );
    }

    #[sqlx::test]
    async fn confirm_login_rejects_unknown_email(pool: PgPool) {
        let result = confirm_login_inner(&pool, "nobody@example.com", "000000").await;
        assert!(result.is_err());
        assert_eq!(user_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn confirm_login_rejects_wrong_code_and_counts_attempt(pool: PgPool) {
        let email = "wrong-code@example.com";
        seed_confirmation(
            &pool,
            email,
            "123456",
            time::Duration::minutes(1),
            0,
            1,
            time::Duration::minutes(1),
        )
        .await;

        let result = confirm_login_inner(&pool, email, "654321").await;
        assert!(result.is_err());
        let row = get_confirmation(&pool, email).await.unwrap();
        assert_eq!(row.attempts, 1);
        assert_eq!(user_count(&pool).await, 0, "failed confirm creates no user");
    }

    #[sqlx::test]
    async fn confirm_login_rejects_expired_code(pool: PgPool) {
        let email = "expired@example.com";
        seed_confirmation(
            &pool,
            email,
            "123456",
            time::Duration::hours(2),
            0,
            1,
            time::Duration::hours(2),
        )
        .await;

        let result = confirm_login_inner(&pool, email, "123456").await;
        assert!(result.is_err());
        assert_eq!(user_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn confirm_login_rejects_right_code_wrong_email(pool: PgPool) {
        seed_confirmation(
            &pool,
            "scoped@example.com",
            "654321",
            time::Duration::minutes(1),
            0,
            1,
            time::Duration::minutes(1),
        )
        .await;

        // Right code, but for a different email than the one the code was
        // issued to: must not succeed.
        let result = confirm_login_inner(&pool, "someone-else@example.com", "654321").await;
        assert!(result.is_err());
        assert_eq!(user_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn confirm_login_attempts_cap_invalidates_code(pool: PgPool) {
        let email = "brute@example.com";
        seed_confirmation(
            &pool,
            email,
            "123456",
            time::Duration::minutes(1),
            0,
            1,
            time::Duration::minutes(1),
        )
        .await;

        for _ in 0..10 {
            assert!(confirm_login_inner(&pool, email, "000000").await.is_err());
        }
        let row = get_confirmation(&pool, email).await.unwrap();
        assert_eq!(row.attempts, 10);

        // Even the correct code is dead once the attempt cap is reached.
        let result = confirm_login_inner(&pool, email, "123456").await;
        assert!(result.is_err());
        assert_eq!(user_count(&pool).await, 0);
    }

    #[sqlx::test]
    async fn confirm_login_creates_user_exactly_once(pool: PgPool) {
        let email = "brand-new@example.com";
        with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert_eq!(user_count(&pool).await, 0);
        let code = get_confirmation(&pool, email).await.unwrap().code;

        let confirmed = confirm_login_inner(&pool, email, &code).await.unwrap();
        assert!(
            crate::db::validate_username(&confirmed.user.name),
            "default username satisfies D2: {}",
            confirmed.user.name
        );
        assert_ne!(
            confirmed.user.name, "brand-new",
            "username no longer derived from email localpart"
        );
        assert_eq!(confirmed.email, email);
        assert_eq!(user_count(&pool).await, 1);
        assert!(
            get_confirmation(&pool, email).await.is_none(),
            "row deleted on successful confirm"
        );

        let token_exists = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM user_auth_tokens WHERE id = $1"#,
            confirmed.auth_token_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(token_exists, 1);

        // Repeat confirm with the same (now consumed) code must fail and must
        // not create a second user.
        let repeat = confirm_login_inner(&pool, email, &code).await;
        assert!(repeat.is_err());
        assert_eq!(user_count(&pool).await, 1);
    }

    #[sqlx::test]
    async fn confirm_login_reuses_existing_user(pool: PgPool) {
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            user_id,
            "existing",
            &Vec::<String>::new()
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
            user_id,
            "existing@example.com"
        )
        .execute(&pool)
        .await
        .unwrap();
        seed_confirmation(
            &pool,
            "existing@example.com",
            "123456",
            time::Duration::minutes(1),
            0,
            1,
            time::Duration::minutes(1),
        )
        .await;

        let confirmed = confirm_login_inner(&pool, "existing@example.com", "123456")
            .await
            .unwrap();
        assert_eq!(confirmed.user.id, user_id, "logs in the existing user");
        assert_eq!(user_count(&pool).await, 1, "no duplicate user created");
    }

    #[test]
    fn validate_pref_colors_rules() {
        let ok = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(validate_pref_colors(&ok(&["Green", "Red", "Blue"])));
        assert!(validate_pref_colors(&ok(&["Pink", "Cyan", "Brown"])));
        assert!(!validate_pref_colors(&ok(&["Green", "Red"])), "must be 3");
        assert!(
            !validate_pref_colors(&ok(&["Green", "Green", "Blue"])),
            "must be distinct"
        );
        assert!(
            !validate_pref_colors(&ok(&["Green", "Red", "Amber"])),
            "legacy names are normalized on read, not accepted on write"
        );
    }
}
