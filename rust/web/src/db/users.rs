use super::*;
#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool))]
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.theme, u.is_admin
        FROM users u
        JOIN user_emails ue ON u.id = ue.user_id
        WHERE ue.email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %id))]
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Whether `user_id` has admin privileges - `false` if the user row doesn't
/// exist. Written as a plain (non-macro) query, matching `get_user_theme`
/// below.
#[cfg(feature = "ssr")]
pub async fn is_user_admin(pool: &PgPool, user_id: Uuid) -> Result<bool> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(a,)| a).unwrap_or(false))
}

#[cfg(feature = "ssr")]
pub async fn find_user_id_by_name(pool: &PgPool, name: &str) -> Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM users WHERE LOWER(name) = LOWER($1)")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow::anyhow!("find_user_id_by_name: {e}"))
}

/// Generates a default username: a 2-word petname (e.g. "scary-walrus"),
/// regenerated until it satisfies D2 (length; the crate's charset is already
/// safe) and is case-insensitively unused. Long words make regeneration
/// expected and cheap. The uuid fallback is unreachable in practice but keeps
/// this total. Takes a connection so it can run inside callers' transactions.
/// **Race note (ws F46):** the availability SELECT and the caller's INSERT are
/// separate statements, so a concurrent transaction can claim the same
/// generated name in between. The `users_name_lower_key` unique index
/// (migrations/009_username_rules.sql:41) is the actual guarantee; the loser
/// gets a 23505 that surfaces as a failed account/game creation. Retrying here
/// is not possible: every caller runs this inside an open transaction
/// (auth/server.rs:439, game/import.rs:181, proposals.rs:902,
/// db.rs `create_game_with_users_tx`), where a 23505 aborts the whole
/// transaction, so a retry would need SAVEPOINT plumbing in four modules. The
/// petname space plus the 100-attempt loop makes a collision vanishingly rare.
#[cfg(feature = "ssr")]
pub async fn generate_unique_username(conn: &mut sqlx::PgConnection) -> Result<String> {
    for _ in 0..100 {
        let Some(candidate) = petname::petname(2, "-") else {
            continue;
        };
        if !validate_username(&candidate) {
            continue;
        }
        let taken: Option<(bool,)> =
            sqlx::query_as("SELECT true FROM users WHERE lower(name) = lower($1)")
                .bind(&candidate)
                .fetch_optional(&mut *conn)
                .await?;
        if taken.is_none() {
            return Ok(candidate);
        }
    }
    Ok(format!(
        "user-{}",
        &Uuid::new_v4().simple().to_string()[..11]
    ))
}

/// Written as a plain (non-macro) query rather than `query_scalar!`. See
/// `docs/DEV.md` for the `cargo sqlx prepare` workflow if this is ever
/// converted to a macro query.
#[cfg(feature = "ssr")]
pub async fn get_user_theme(pool: &PgPool, user_id: Uuid) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT theme FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|(theme,)| theme))
}

#[cfg(feature = "ssr")]
pub async fn set_user_theme(pool: &PgPool, user_id: Uuid, theme: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE users SET theme = $1 WHERE id = $2")
        .bind(theme)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Window within which a presence ping counts as "recently active on the web".
/// 2x the client ping interval (5 min) for slack.
#[cfg(feature = "ssr")]
pub const RECENTLY_ACTIVE_WINDOW: std::time::Duration = std::time::Duration::from_secs(600);

#[cfg(feature = "ssr")]
pub async fn set_user_last_active(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Pure predicate: was the user last active within `window` of `now`? `None`
/// (never pinged) => false.
#[cfg(feature = "ssr")]
pub fn active_within_window(
    last_active_at: Option<time::OffsetDateTime>,
    now: time::OffsetDateTime,
    window: std::time::Duration,
) -> bool {
    let Some(last_active_at) = last_active_at else {
        return false;
    };
    let window = time::Duration::try_from(window).unwrap_or(time::Duration::minutes(10));
    (now - last_active_at) < window
}

/// Whether the user pinged the server within `RECENTLY_ACTIVE_WINDOW`. Fails
/// open (false) on a DB error or a missing user row.
#[cfg(feature = "ssr")]
pub async fn is_user_recently_active(pool: &PgPool, user_id: Uuid) -> bool {
    let row = sqlx::query_as::<_, (Option<time::OffsetDateTime>,)>(
        "SELECT last_active_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await;
    let row = match row {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to read last_active_at for {}: {}", user_id, e);
            return false;
        }
    };
    active_within_window(
        row.and_then(|(l,)| l),
        time::OffsetDateTime::now_utc(),
        RECENTLY_ACTIVE_WINDOW,
    )
}

// --- #30 friends (spec docs/changes/archive/2026-07-08-30-friends/spec.md) ---
// Plain (non-macro) queries throughout, matching get_user_theme above.

/// Exact-name lookup, case-insensitive (users_name_lower_key, migration 009).
#[cfg(feature = "ssr")]
pub async fn get_user_by_name(pool: &PgPool, name: &str) -> Result<Option<(Uuid, String)>> {
    Ok(
        sqlx::query_as("SELECT id, name FROM users WHERE lower(name) = lower($1)")
            .bind(name)
            .fetch_optional(pool)
            .await?,
    )
}

/// Display-name substring search for the new game page typeahead (#44):
/// case-insensitive, excludes the searching user, capped at 10. Users who
/// block the searcher or are blocked by the searcher (either direction) are
/// also excluded. Queries under 2 trimmed characters return nothing without
/// touching the DB.
#[cfg(feature = "ssr")]
pub async fn search_users(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
) -> Result<Vec<(Uuid, String)>> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    // Escape LIKE wildcards so users named "a%b" are findable and "%"
    // queries cannot match everyone.
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM users u
         WHERE u.id <> $1 AND u.name ILIKE $2 ESCAPE '\\'
           AND NOT EXISTS (SELECT 1 FROM blocks b
                           WHERE (b.blocker_user_id = $1 AND b.blocked_user_id = u.id)
                              OR (b.blocker_user_id = u.id AND b.blocked_user_id = $1))
         ORDER BY lower(u.name)
         LIMIT 10",
    )
    .bind(user_id)
    .bind(format!("%{escaped}%"))
    .fetch_all(pool)
    .await?)
}

/// The user's current name straight from the `users` table - the session's
/// cached copy can be stale after a rename. Plain query, matching
/// `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn get_user_name(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Renames a user. Returns `Ok(false)` when the name is already taken
/// case-insensitively (unique violation on `users_name_lower_key`); the
/// caller turns that into a field error. Plain query for the same reason as
/// `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn set_user_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<bool> {
    let res = sqlx::query("UPDATE users SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(user_id)
        .execute(pool)
        .await;
    match res {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// The user's stored colour preferences, legacy names ("Amber", "BlueGrey")
/// normalized onto the current palette. May be empty (never set) - the
/// settings server fn applies the palette-order default.
#[cfg(feature = "ssr")]
pub async fn get_user_pref_colors(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let row: Option<(Vec<String>,)> = sqlx::query_as("SELECT pref_colors FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(|(colors,)| colors.iter().map(|c| normalize_pref_color(c)).collect())
        .unwrap_or_default())
}

#[cfg(feature = "ssr")]
pub async fn set_user_pref_colors(pool: &PgPool, user_id: Uuid, colors: &[String]) -> Result<()> {
    sqlx::query("UPDATE users SET pref_colors = $1 WHERE id = $2")
        .bind(colors)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_user_email_prefs(pool: &PgPool, user_id: Uuid) -> Result<(bool, bool, bool)> {
    let row: (bool, bool, bool) = sqlx::query_as(
        "SELECT turn_emails_enabled, invite_emails_enabled, reminder_emails_enabled FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

#[cfg(feature = "ssr")]
pub async fn set_user_turn_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET turn_emails_enabled = $1 WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn set_user_invite_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET invite_emails_enabled = $1 WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn set_user_reminder_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET reminder_emails_enabled = $1 WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// RFC 8058 one-click unsubscribe (WP-58): disables the email preference that
/// `kind` maps to for the single user holding `unsubscribe_token`. Only ever
/// writes `false`. Returns whether exactly one row was updated; callers treat
/// `false` (unknown/expired token) identically to `true` so a token's validity
/// is never leaked. `Turn` and `GameEvent` share `turn_emails_enabled`. No
/// manual `updated_at` - the `users` BEFORE UPDATE trigger maintains it (ws F36).
#[cfg(feature = "ssr")]
pub async fn disable_email_pref_by_unsubscribe_token(
    pool: &PgPool,
    token: &str,
    kind: crate::email::render::EmailKind,
) -> Result<bool> {
    let result = match kind {
        crate::email::render::EmailKind::Turn | crate::email::render::EmailKind::GameEvent => {
            sqlx::query(
                "UPDATE users SET turn_emails_enabled = false, updated_at = NOW() WHERE unsubscribe_token = $1",
            )
            .bind(token)
            .execute(pool)
            .await?
        }
        crate::email::render::EmailKind::Reminder => {
            sqlx::query(
                "UPDATE users SET reminder_emails_enabled = false, updated_at = NOW() WHERE unsubscribe_token = $1",
            )
            .bind(token)
            .execute(pool)
            .await?
        }
        crate::email::render::EmailKind::Invite => {
            sqlx::query(
                "UPDATE users SET invite_emails_enabled = false, updated_at = NOW() WHERE unsubscribe_token = $1",
            )
            .bind(token)
            .execute(pool)
            .await?
        }
    };
    Ok(result.rows_affected() == 1)
}

// --- #22d multiple emails per account (spec 2026-07-05-22, section 22d) ---
// Plain (non-macro) queries throughout, matching get_user_theme above.

#[cfg(feature = "ssr")]
pub async fn invalidate_all_auth_tokens(pool: &PgPool, user_id: Uuid) -> Result<u64> {
    let result = sqlx::query("DELETE FROM user_auth_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    #[sqlx::test]
    async fn last_active_at_column_exists_and_defaults_null(pool: PgPool) -> sqlx::Result<()> {
        let user = make_user(&pool, "presence").await;
        let last: Option<time::OffsetDateTime> =
            sqlx::query_scalar("SELECT last_active_at FROM users WHERE id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await?;
        assert!(last.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn set_user_last_active_stamps_column(pool: PgPool) {
        let user = make_user(&pool, "pinger").await;
        set_user_last_active(&pool, user.id).await.unwrap();
        let last: Option<time::OffsetDateTime> =
            sqlx::query_scalar("SELECT last_active_at FROM users WHERE id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(last.is_some());
        assert!(is_user_recently_active(&pool, user.id).await);
    }

    #[sqlx::test]
    async fn is_user_recently_active_false_when_null(pool: PgPool) {
        let user = make_user(&pool, "never-pinged").await;
        assert!(!is_user_recently_active(&pool, user.id).await);
    }

    #[sqlx::test]
    async fn is_user_recently_active_false_when_stale(pool: PgPool) {
        let user = make_user(&pool, "stale").await;
        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();
        assert!(!is_user_recently_active(&pool, user.id).await);
    }

    #[test]
    fn active_within_window_truth_table() {
        let now = time::OffsetDateTime::now_utc();
        let window = std::time::Duration::from_secs(600);
        let five_min_ago = now - time::Duration::minutes(5);
        let eleven_min_ago = now - time::Duration::minutes(11);
        assert!(active_within_window(Some(five_min_ago), now, window));
        assert!(!active_within_window(Some(eleven_min_ago), now, window));
        assert!(!active_within_window(None, now, window));
    }

    #[sqlx::test]
    async fn user_theme_defaults_none_and_round_trips(pool: PgPool) {
        let user = make_user(&pool, "themed").await;

        assert_eq!(get_user_theme(&pool, user.id).await.unwrap(), None);

        set_user_theme(&pool, user.id, Some("dracula"))
            .await
            .unwrap();
        assert_eq!(
            get_user_theme(&pool, user.id).await.unwrap(),
            Some("dracula".to_string())
        );

        set_user_theme(&pool, user.id, None).await.unwrap();
        assert_eq!(get_user_theme(&pool, user.id).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn search_users_min_length_cap_and_excludes_self(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        for i in 0..12 {
            make_user(&pool, &format!("player{i:02}")).await;
        }

        // Under 2 trimmed characters: no results, no query.
        assert!(search_users(&pool, me.id, "p").await.unwrap().is_empty());
        assert!(search_users(&pool, me.id, " a ").await.unwrap().is_empty());
        assert!(search_users(&pool, me.id, "").await.unwrap().is_empty());

        // Results are capped at 10 of the 12 matches.
        assert_eq!(
            search_users(&pool, me.id, "player").await.unwrap().len(),
            10
        );

        // The searching user is never in their own results.
        assert!(
            search_users(&pool, me.id, "search")
                .await
                .unwrap()
                .is_empty()
        );

        // Case-insensitive substring match.
        let hits = search_users(&pool, me.id, "PLAYER00").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "player00");
    }

    #[sqlx::test]
    async fn search_users_escapes_like_wildcards(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        make_user(&pool, "percent%name").await;
        make_user(&pool, "underscore_name").await;

        let hits = search_users(&pool, me.id, "percent%").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "percent%name");

        // A raw "%%" query must not match everything.
        assert!(search_users(&pool, me.id, "%%").await.unwrap().is_empty());

        // "_" is a literal underscore, not a single-char wildcard.
        let hits = search_users(&pool, me.id, "score_n").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "underscore_name");
    }

    #[sqlx::test]
    async fn search_users_excludes_blocked_in_either_direction(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        let i_block = make_user(&pool, "player_iblock").await;
        let blocks_me = make_user(&pool, "player_blocksme").await;
        make_user(&pool, "player_open").await;
        block_user(&pool, me.id, i_block.id).await.unwrap();
        block_user(&pool, blocks_me.id, me.id).await.unwrap();

        let hits = search_users(&pool, me.id, "player").await.unwrap();
        let names: Vec<String> = hits.into_iter().map(|(_, n)| n).collect();
        assert!(!names.contains(&"player_iblock".to_string()));
        assert!(!names.contains(&"player_blocksme".to_string()));
        assert!(names.contains(&"player_open".to_string()));
    }

    #[sqlx::test]
    async fn email_prefs_default_all_true(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let (turn, invite, reminder) = get_user_email_prefs(&pool, user_id).await.unwrap();
        assert!(turn);
        assert!(invite);
        assert!(reminder);
    }

    #[sqlx::test]
    async fn set_email_prefs_toggles(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        set_user_turn_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        set_user_invite_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        set_user_reminder_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        assert_eq!(
            get_user_email_prefs(&pool, user_id).await.unwrap(),
            (false, false, false)
        );

        set_user_turn_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        set_user_invite_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        set_user_reminder_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        assert_eq!(
            get_user_email_prefs(&pool, user_id).await.unwrap(),
            (true, true, true)
        );
    }

    /// ws F35 + ws F45: `is_user_admin` had no test at all, and now returns
    /// `anyhow::Result`. Covers all three outcomes including the fail-closed
    /// unknown-user case.
    #[sqlx::test]
    async fn is_user_admin_true_false_and_unknown_user(pool: PgPool) {
        let plain = make_user(&pool, "plain").await;
        let admin = make_user(&pool, "adminuser").await;
        sqlx::query("UPDATE users SET is_admin = true WHERE id = $1")
            .bind(admin.id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(is_user_admin(&pool, admin.id).await.unwrap());
        assert!(!is_user_admin(&pool, plain.id).await.unwrap());
        // Fail closed for a user id that does not exist.
        assert!(!is_user_admin(&pool, Uuid::new_v4()).await.unwrap());
    }

    /// ws F35: `generate_unique_username` had no test. Asserts the result
    /// satisfies the D2 username rules and is unused, and that generating +
    /// claiming twice yields two distinct names.
    ///
    /// The taken-branch retry (line "if taken.is_none()") cannot be forced
    /// deterministically - the candidate comes from `petname`, so a test cannot
    /// pre-claim the exact name the next call will draw. This covers the loop's
    /// success path and the uniqueness contract, which is the part callers
    /// depend on.
    #[sqlx::test]
    async fn generate_unique_username_is_valid_and_unclaimed(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let first = generate_unique_username(&mut conn).await.unwrap();
        assert!(
            validate_username(&first),
            "generated name must satisfy the D2 rules: {first}"
        );
        assert!(
            find_user_id_by_name(&pool, &first).await.unwrap().is_none(),
            "generated name must be unused"
        );

        // Claim it, then generate again: the second name must differ and must
        // itself be claimable (the unique index would reject a duplicate).
        sqlx::query("INSERT INTO users (name, pref_colors) VALUES ($1, $2)")
            .bind(&first)
            .bind(Vec::<String>::new())
            .execute(&pool)
            .await
            .unwrap();

        let second = generate_unique_username(&mut conn).await.unwrap();
        assert!(validate_username(&second));
        assert_ne!(
            second.to_lowercase(),
            first.to_lowercase(),
            "must not hand back a name that is already claimed"
        );
        sqlx::query("INSERT INTO users (name, pref_colors) VALUES ($1, $2)")
            .bind(&second)
            .bind(Vec::<String>::new())
            .execute(&pool)
            .await
            .expect("second generated name must be insertable");
    }

    /// ws F35: seven untested single-statement user getters/setters, batched
    /// into one round-trip test per the coverage cut rule.
    #[sqlx::test]
    async fn user_getters_and_setters_round_trip(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let email = format!("alice-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)")
            .bind(user.id)
            .bind(&email)
            .execute(&pool)
            .await
            .unwrap();

        // get_user / get_user_by_email
        assert_eq!(get_user(&pool, user.id).await.unwrap().unwrap().id, user.id);
        assert!(get_user(&pool, Uuid::new_v4()).await.unwrap().is_none());
        assert_eq!(
            get_user_by_email(&pool, &email).await.unwrap().unwrap().id,
            user.id
        );
        assert!(
            get_user_by_email(&pool, "nobody@example.com")
                .await
                .unwrap()
                .is_none()
        );

        // get_user_name / find_user_id_by_name (case-insensitive)
        assert_eq!(get_user_name(&pool, user.id).await.unwrap(), "alice");
        assert_eq!(
            find_user_id_by_name(&pool, "ALICE").await.unwrap(),
            Some(user.id)
        );
        assert_eq!(find_user_id_by_name(&pool, "nobody").await.unwrap(), None);

        // set_user_name: success, then a case-insensitive conflict -> Ok(false)
        assert!(set_user_name(&pool, user.id, "alice2").await.unwrap());
        assert_eq!(get_user_name(&pool, user.id).await.unwrap(), "alice2");
        let other = make_user(&pool, "bob").await;
        assert!(
            !set_user_name(&pool, other.id, "ALICE2").await.unwrap(),
            "a case-insensitive name clash must be Ok(false), not an error"
        );
        assert_eq!(get_user_name(&pool, other.id).await.unwrap(), "bob");

        // pref colors: empty by default, round-trip, legacy names normalized
        assert!(
            get_user_pref_colors(&pool, user.id)
                .await
                .unwrap()
                .is_empty()
        );
        set_user_pref_colors(&pool, user.id, &["Green".to_string(), "Amber".to_string()])
            .await
            .unwrap();
        assert_eq!(
            get_user_pref_colors(&pool, user.id).await.unwrap(),
            vec!["Green".to_string(), "Orange".to_string()],
            "stored legacy 'Amber' must read back normalized to 'Orange'"
        );
        // Unknown user -> empty, not an error.
        assert!(
            get_user_pref_colors(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_empty()
        );
    }
}
