#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

/// A user's email address row for the settings list. `verified_at` is NULL
/// until the address is confirmed: added addresses start unverified, while
/// signup / invite / pre-feature rows are verified.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserEmailRow {
    pub id: Uuid,
    pub email: String,
    pub is_primary: bool,
    pub verified_at: Option<time::PrimitiveDateTime>,
}

/// The 22d switch-digest cap: at most this many turn notifications per switch
/// (Resend free-tier quota protection).
#[cfg(feature = "ssr")]
pub const SWITCH_DIGEST_CAP: usize = 20;

/// Pure predicate: an address may be removed only if it is NOT the primary.
#[cfg(feature = "ssr")]
pub fn can_remove_email(is_primary: bool) -> bool {
    !is_primary
}

/// Pure predicate: an address may be switched to (made active) only once
/// verified.
#[cfg(feature = "ssr")]
pub fn can_switch_to_email(verified_at: Option<time::PrimitiveDateTime>) -> bool {
    verified_at.is_some()
}

/// Pure predicate: an unverified address is expired once older than
/// `threshold` (the 22d ~24h cleanup window). Verified addresses never expire.
#[cfg(feature = "ssr")]
pub fn is_expired_unverified(
    verified_at: Option<time::PrimitiveDateTime>,
    created_at: time::PrimitiveDateTime,
    now: time::PrimitiveDateTime,
    threshold: std::time::Duration,
) -> bool {
    if verified_at.is_some() {
        return false;
    }
    let threshold = time::Duration::try_from(threshold).unwrap_or(time::Duration::hours(24));
    (now - created_at) >= threshold
}

/// All of a user's addresses, primary first then by creation order, with
/// verification status.
#[cfg(feature = "ssr")]
pub async fn list_user_emails(pool: &PgPool, user_id: Uuid) -> Result<Vec<UserEmailRow>> {
    Ok(sqlx::query_as::<_, UserEmailRow>(
        "SELECT id, email, is_primary, verified_at FROM user_emails
         WHERE user_id = $1
         ORDER BY is_primary DESC, created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// Which account (if any) already owns this address. Used to reject re-adding
/// an address already on the caller's account and to reject addresses owned by
/// another account (global UNIQUE(email)).
#[cfg(feature = "ssr")]
pub async fn find_email_owner(pool: &PgPool, email: &str) -> Result<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT user_id FROM user_emails WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?,
    )
}

/// Adds a new UNVERIFIED, non-primary address (the 22d "add address" first
/// step; confirmation later sets `verified_at`). `Ok(None)` on a global
/// UNIQUE(email) violation (address already taken).
#[cfg(feature = "ssr")]
pub async fn insert_unverified_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<Option<Uuid>> {
    let res = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO user_emails (user_id, email, is_primary)
         VALUES ($1, $2, false) RETURNING id",
    )
    .bind(user_id)
    .bind(email)
    .fetch_one(pool)
    .await;
    match res {
        Ok(id) => Ok(Some(id)),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Marks an address verified (the 22d "confirm address" step). Returns whether
/// a row was updated (false = no matching unverified address on this account).
#[cfg(feature = "ssr")]
pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid, email: &str) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE user_emails SET verified_at = NOW()
         WHERE user_id = $1 AND email = $2 AND verified_at IS NULL",
    )
    .bind(user_id)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Outcome of a make-active (set-primary) attempt.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPrimaryOutcome {
    Switched,
    NotFound,
    Unverified,
}

/// Sets `email` as the user's primary address in ONE transaction: rejects an
/// unknown or unverified address, otherwise clears the old primary and sets the
/// new one (the partial unique index enforces exactly one primary).
#[cfg(feature = "ssr")]
pub async fn set_primary_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<SetPrimaryOutcome> {
    let mut tx = pool.begin().await?;
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT (verified_at IS NOT NULL) FROM user_emails WHERE user_id = $1 AND email = $2",
    )
    .bind(user_id)
    .bind(email)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((verified,)) = row else {
        return Ok(SetPrimaryOutcome::NotFound);
    };
    if !verified {
        return Ok(SetPrimaryOutcome::Unverified);
    }
    sqlx::query(
        "UPDATE user_emails SET is_primary = false
         WHERE user_id = $1 AND is_primary = true",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE user_emails SET is_primary = true
         WHERE user_id = $1 AND email = $2",
    )
    .bind(user_id)
    .bind(email)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(SetPrimaryOutcome::Switched)
}

/// Outcome of a remove-address attempt.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveEmailOutcome {
    Removed,
    NotFound,
    IsPrimary,
}

/// Removes a non-primary address. The primary cannot be removed (switch first).
#[cfg(feature = "ssr")]
pub async fn remove_user_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<RemoveEmailOutcome> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT is_primary FROM user_emails WHERE user_id = $1 AND email = $2")
            .bind(user_id)
            .bind(email)
            .fetch_optional(pool)
            .await?;
    let Some((is_primary,)) = row else {
        return Ok(RemoveEmailOutcome::NotFound);
    };
    if !can_remove_email(is_primary) {
        return Ok(RemoveEmailOutcome::IsPrimary);
    }
    sqlx::query("DELETE FROM user_emails WHERE user_id = $1 AND email = $2 AND is_primary = false")
        .bind(user_id)
        .bind(email)
        .execute(pool)
        .await?;
    Ok(RemoveEmailOutcome::Removed)
}

/// Deletes unverified addresses older than `threshold` (the 22d expiry
/// cleanup). Verified rows are never touched. Returns the count deleted.
#[cfg(feature = "ssr")]
pub async fn delete_expired_unverified_emails(
    pool: &PgPool,
    threshold: std::time::Duration,
) -> Result<u64> {
    let secs = threshold.as_secs() as i64;
    let res = sqlx::query(
        // `make_interval(secs => ...)` instead of the older
        // `($1 || ' seconds')::interval` idiom: the parameter stays an integer
        // instead of being formatted to text and re-parsed as an interval
        // (ws F47). Not an injection fix - `$1` was always a bound parameter.
        "DELETE FROM user_emails
         WHERE verified_at IS NULL
           AND created_at < NOW() - make_interval(secs => $1::double precision)",
    )
    .bind(secs)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// Deletes processed webhook events older than `threshold` (the wfe F11
/// prune). Returns the count deleted.
#[cfg(feature = "ssr")]
pub async fn delete_old_processed_webhook_events(
    pool: &PgPool,
    threshold: std::time::Duration,
) -> Result<u64> {
    let secs = threshold.as_secs() as i64;
    let res = sqlx::query(
        "DELETE FROM processed_webhook_events
         WHERE processed_at < NOW() - make_interval(secs => $1::double precision)",
    )
    .bind(secs)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

#[cfg(feature = "ssr")]
pub async fn delete_login_confirmation(pool: &PgPool, email: &str) -> Result<()> {
    sqlx::query("DELETE FROM login_confirmations WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
        .map_err(Into::into)
        .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn delete_login_confirmation_tx(tx: &mut sqlx::PgConnection, email: &str) -> Result<()> {
    sqlx::query("DELETE FROM login_confirmations WHERE email = $1")
        .bind(email)
        .execute(&mut *tx)
        .await
        .map_err(Into::into)
        .map(|_| ())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

    #[test]
    fn can_remove_email_only_non_primary() {
        assert!(can_remove_email(false));
        assert!(!can_remove_email(true));
    }

    #[test]
    fn can_switch_to_email_requires_verified() {
        use time::{Date, Month, PrimitiveDateTime, Time};
        let now = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        );
        assert!(can_switch_to_email(Some(now)));
        assert!(!can_switch_to_email(None));
    }

    #[test]
    fn is_expired_unverified_truth_table() {
        use time::{Date, Month, PrimitiveDateTime, Time};
        let now = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        );
        let old = now - time::Duration::hours(25);
        let recent = now - time::Duration::hours(1);
        let day = std::time::Duration::from_secs(86400);
        assert!(is_expired_unverified(None, old, now, day));
        assert!(!is_expired_unverified(None, recent, now, day));
        assert!(
            !is_expired_unverified(Some(now), old, now, day),
            "verified never expires"
        );
        assert!(
            is_expired_unverified(None, now - time::Duration::hours(24), now, day),
            "boundary is inclusive"
        );
    }

    #[sqlx::test]
    async fn add_unverified_then_confirm_sets_verified(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let email = format!("add-{}@example.com", Uuid::new_v4());
        insert_unverified_email(&pool, user_id, &email)
            .await
            .unwrap()
            .unwrap();
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        let row = rows.iter().find(|r| r.email == email).unwrap();
        assert!(!row.is_primary);
        assert!(row.verified_at.is_none());

        assert!(mark_email_verified(&pool, user_id, &email).await.unwrap());
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        assert!(
            rows.iter()
                .find(|r| r.email == email)
                .unwrap()
                .verified_at
                .is_some()
        );
        assert!(
            !mark_email_verified(&pool, user_id, &email).await.unwrap(),
            "second mark is a no-op"
        );
    }

    #[sqlx::test]
    async fn set_primary_keeps_exactly_one_primary(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let primary = format!("p-{}@example.com", Uuid::new_v4());
        let secondary = format!("s-{}@example.com", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(&primary)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NOW())",
        )
        .bind(user_id)
        .bind(&secondary)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            set_primary_email(&pool, user_id, &secondary).await.unwrap(),
            SetPrimaryOutcome::Switched
        );
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        let primaries: Vec<_> = rows.iter().filter(|r| r.is_primary).collect();
        assert_eq!(primaries.len(), 1);
        assert_eq!(primaries[0].email, secondary);
    }

    #[sqlx::test]
    async fn set_primary_rejects_unverified_and_unknown(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let unverified = format!("uv-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind(&unverified)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            set_primary_email(&pool, user_id, &unverified)
                .await
                .unwrap(),
            SetPrimaryOutcome::Unverified
        );
        assert_eq!(
            set_primary_email(&pool, user_id, "missing@example.com")
                .await
                .unwrap(),
            SetPrimaryOutcome::NotFound
        );
    }

    #[sqlx::test]
    async fn remove_primary_rejected_non_primary_works(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let primary = format!("p-{}@example.com", Uuid::new_v4());
        let secondary = format!("s-{}@example.com", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(&primary)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NOW())",
        )
        .bind(user_id)
        .bind(&secondary)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            remove_user_email(&pool, user_id, &primary).await.unwrap(),
            RemoveEmailOutcome::IsPrimary
        );
        assert_eq!(
            remove_user_email(&pool, user_id, &secondary).await.unwrap(),
            RemoveEmailOutcome::Removed
        );
        assert_eq!(
            remove_user_email(&pool, user_id, &secondary).await.unwrap(),
            RemoveEmailOutcome::NotFound
        );
    }

    #[sqlx::test]
    async fn insert_unverified_rejects_globally_taken_email(pool: PgPool) {
        let u1: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let u2: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let email = format!("shared-{}@example.com", Uuid::new_v4());
        insert_unverified_email(&pool, u1, &email)
            .await
            .unwrap()
            .unwrap();
        assert!(
            insert_unverified_email(&pool, u2, &email)
                .await
                .unwrap()
                .is_none(),
            "global UNIQUE(email)"
        );
        assert_eq!(find_email_owner(&pool, &email).await.unwrap(), Some(u1));
    }

    #[sqlx::test]
    async fn expiry_cleanup_deletes_only_expired_unverified(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let expired = format!("exp-{}@example.com", Uuid::new_v4());
        let fresh = format!("fresh-{}@example.com", Uuid::new_v4());
        let verified_old = format!("vold-{}@example.com", Uuid::new_v4());
        // expired: unverified, created 48h ago
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, created_at) VALUES ($1, $2, false, NOW() - interval '48 hours')",
        )
        .bind(user_id)
        .bind(&expired)
        .execute(&pool)
        .await
        .unwrap();
        // fresh: unverified, created now
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind(&fresh)
            .execute(&pool)
            .await
            .unwrap();
        // verified but old: must survive
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at, created_at) VALUES ($1, $2, true, NOW(), NOW() - interval '48 hours')",
        )
        .bind(user_id)
        .bind(&verified_old)
        .execute(&pool)
        .await
        .unwrap();

        let deleted =
            delete_expired_unverified_emails(&pool, std::time::Duration::from_secs(86400))
                .await
                .unwrap();
        assert_eq!(deleted, 1);
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        assert!(rows.iter().any(|r| r.email == fresh));
        assert!(rows.iter().any(|r| r.email == verified_old));
        assert!(!rows.iter().any(|r| r.email == expired));
    }

    #[sqlx::test]
    async fn delete_old_processed_webhook_events_prunes_only_old(pool: PgPool) {
        use crate::email::sweep::PROCESSED_WEBHOOK_EVENT_RETENTION;

        let old_id = format!("evt-old-{}", Uuid::new_v4());
        let fresh_id = format!("evt-fresh-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO processed_webhook_events (event_id, processed_at)
             VALUES ($1, NOW() - interval '10 days')",
        )
        .bind(&old_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO processed_webhook_events (event_id) VALUES ($1)")
            .bind(&fresh_id)
            .execute(&pool)
            .await
            .unwrap();

        let deleted = delete_old_processed_webhook_events(&pool, PROCESSED_WEBHOOK_EVENT_RETENTION)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        let remaining: Vec<(String,)> =
            sqlx::query_as("SELECT event_id FROM processed_webhook_events")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(remaining.iter().any(|(id,)| id == &fresh_id));
        assert!(!remaining.iter().any(|(id,)| id == &old_id));
    }

    /// ws F47: `make_interval(secs => 0)` must behave like the old
    /// `('0' || ' seconds')::interval` - i.e. a zero threshold deletes every
    /// unverified row (created_at is strictly in the past) and still spares
    /// verified ones.
    #[sqlx::test]
    async fn delete_expired_unverified_emails_zero_threshold(pool: PgPool) {
        let user = make_user(&pool, "sweeper").await;
        let unverified = format!("u-{}@example.com", Uuid::new_v4());
        let verified = format!("v-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user.id)
            .bind(&unverified)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user.id)
        .bind(&verified)
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_expired_unverified_emails(&pool, std::time::Duration::from_secs(0))
            .await
            .unwrap();
        assert_eq!(deleted, 1, "zero threshold must delete the unverified row");
        let remaining = list_user_emails(&pool, user.id).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].email, verified);
    }
}
