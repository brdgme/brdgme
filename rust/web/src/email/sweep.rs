//! #22c turn-reminder sweep: a periodic tokio task that nudges players who have
//! held the turn past a threshold. One reminder per turn (reset on transition).
//! Structured for future periodic jobs (22d unverified-email cleanup, #24 invite
//! nudge/expiry) via `spawn_periodic_sweeps`.

use sqlx::PgPool;
use uuid::Uuid;

pub const DEFAULT_REMINDER_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(86400);

pub const DEFAULT_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(900);

pub fn reminder_threshold() -> std::time::Duration {
    std::env::var("TURN_REMINDER_AFTER")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_REMINDER_THRESHOLD)
}

pub fn sweep_interval() -> std::time::Duration {
    std::env::var("TURN_REMINDER_SWEEP_INTERVAL")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_SWEEP_INTERVAL)
}

pub fn should_reset_reminder(was_turn: bool, is_turn: bool) -> bool {
    was_turn != is_turn
}

pub fn is_reminder_candidate(
    is_turn: bool,
    is_eliminated: bool,
    turn_reminder_sent_at: Option<time::PrimitiveDateTime>,
    is_turn_at: time::PrimitiveDateTime,
    now: time::PrimitiveDateTime,
    threshold: std::time::Duration,
) -> bool {
    if !is_turn || is_eliminated || turn_reminder_sent_at.is_some() {
        return false;
    }
    let threshold = time::Duration::try_from(threshold).unwrap_or(time::Duration::hours(24));
    (now - is_turn_at) >= threshold
}

fn reminder_header_text(player_name: &str) -> String {
    format!("Still your turn, {player_name}.")
}

fn now_utc() -> time::PrimitiveDateTime {
    let t = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(t.date(), t.time())
}

#[derive(Debug, sqlx::FromRow)]
struct ReminderCandidate {
    game_player_id: Uuid,
    game_id: Uuid,
}

async fn fetch_candidates(pool: &PgPool, threshold: std::time::Duration) -> Vec<ReminderCandidate> {
    let threshold_secs = threshold.as_secs() as i64;
    let rows = sqlx::query_as::<_, ReminderCandidate>(
        "SELECT gp.id AS game_player_id, gp.game_id AS game_id
         FROM game_players gp
         JOIN users u ON gp.user_id = u.id
         WHERE gp.is_turn = true
           AND gp.is_eliminated = false
           AND gp.turn_reminder_sent_at IS NULL
           AND gp.is_turn_at < NOW() - ($1 || ' seconds')::interval
            AND gp.game_bot_id IS NULL
            AND u.reminder_emails_enabled = true
         FOR UPDATE SKIP LOCKED",
    )
    .bind(threshold_secs.to_string())
    .fetch_all(pool)
    .await;
    match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("turn_reminder: candidate query failed: {}", e);
            Vec::new()
        }
    }
}

async fn mark_reminder_sent(pool: &PgPool, game_player_id: Uuid) {
    if let Err(e) = sqlx::query(
        "UPDATE game_players SET turn_reminder_sent_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(game_player_id)
    .execute(pool)
    .await
    {
        tracing::error!(
            "turn_reminder: failed to mark sent for {}: {}",
            game_player_id,
            e
        );
    }
}

async fn send_reminder(
    resend: Option<&resend_rs::Resend>,
    pool: &PgPool,
    http_client: &reqwest::Client,
    game_id: Uuid,
    game_player_id: Uuid,
) -> bool {
    let ge = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            tracing::warn!("turn_reminder: game {} not found", game_id);
            return false;
        }
        Err(e) => {
            tracing::error!("turn_reminder: failed to load game {}: {}", game_id, e);
            return false;
        }
    };

    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == game_player_id)
    {
        Some(p) => p,
        None => return false,
    };

    let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
    {
        Ok(Some(r)) => r,
        _ => return false,
    };

    if !crate::email::outbound::should_email_recipient(
        &recipient,
        now_utc(),
        crate::email::outbound::suppress_window(),
    ) {
        return true;
    }

    let token = match crate::email::outbound::ensure_email_token(pool, game_player_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "turn_reminder: failed to ensure email token for {}: {}",
                game_player_id,
                e
            );
            return false;
        }
    };

    let palette = crate::email::render::palette_for_slug(recipient.theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();

    let subject = crate::email::notify::game_subject(&ge, recipient_player);
    let header = Some(reminder_header_text(recipient_player.name()));

    let (board, you_can) = crate::email::notify::render_board_and_you_can(
        http_client,
        &ge,
        recipient_player.game_player.position as usize,
    )
    .await;

    let content = crate::email::render::EmailContent {
        subject,
        header,
        digest: None,
        board,
        you_can,
        browser_url: Some(crate::email::notify::browser_url(ge.game.id)),
        rules_url: Some(crate::email::notify::rules_url(ge.game_version.id)),
        footer: Some("Reply to this email to play, or unsubscribe anytime.".to_string()),
    };

    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        Some(&format!("game-{game_id}")),
        false,
        &crate::email::notify::reply_address(&token),
    );

    let to = match recipient.email {
        Some(e) => e,
        None => return false,
    };
    crate::email::outbound::try_send_rendered_email(resend, rendered, &to).await
}

async fn sweep_once(
    resend: Option<&resend_rs::Resend>,
    pool: &PgPool,
    http_client: &reqwest::Client,
) {
    let threshold = reminder_threshold();
    let candidates = fetch_candidates(pool, threshold).await;
    if candidates.is_empty() {
        return;
    }
    tracing::info!("turn_reminder: {} candidate(s)", candidates.len());
    for c in candidates {
        let ok = send_reminder(resend, pool, http_client, c.game_id, c.game_player_id).await;
        if ok {
            mark_reminder_sent(pool, c.game_player_id).await;
        }
    }
}

pub fn spawn_turn_reminder_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
) {
    let interval = sweep_interval();
    tracing::info!("turn_reminder: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_once(resend.as_ref(), &pool, &http_client).await;
        }
    });
}

/// The 22d unverified-address expiry window: unverified `user_emails` older
/// than this are deleted by `spawn_unverified_email_sweep`.
pub const UNVERIFIED_EMAIL_EXPIRY: std::time::Duration = std::time::Duration::from_secs(86400);

async fn sweep_unverified_emails_once(pool: &PgPool) {
    match crate::db::delete_expired_unverified_emails(pool, UNVERIFIED_EMAIL_EXPIRY).await {
        Ok(0) => {}
        Ok(n) => tracing::info!("unverified_email_expiry: deleted {} row(s)", n),
        Err(e) => tracing::error!("unverified_email_expiry: delete failed: {}", e),
    }
}

/// Periodic job deleting unverified addresses that were never confirmed
/// (the 22d expiry cleanup). Reuses the shared `sweep_interval()` cadence.
pub fn spawn_unverified_email_sweep(pool: PgPool) {
    let interval = sweep_interval();
    tracing::info!("unverified_email_expiry: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_unverified_emails_once(&pool).await;
        }
    });
}

pub const DEFAULT_INVITE_REMINDER_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(86400);

pub const DEFAULT_INVITE_EXPIRY_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(1209600);

pub fn invite_reminder_threshold() -> std::time::Duration {
    std::env::var("INVITE_REMINDER_AFTER")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_INVITE_REMINDER_THRESHOLD)
}

pub fn invite_expiry_threshold() -> std::time::Duration {
    std::env::var("INVITE_EXPIRE_AFTER")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_INVITE_EXPIRY_THRESHOLD)
}

pub const DEFAULT_INVITE_AUTO_DECLINE_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(172800);

pub fn invite_auto_decline_threshold() -> std::time::Duration {
    std::env::var("INVITE_AUTO_DECLINE_AFTER")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_INVITE_AUTO_DECLINE_THRESHOLD)
}

async fn sweep_invite_nudge_once(resend: Option<&resend_rs::Resend>, pool: &PgPool) {
    let threshold = invite_reminder_threshold();
    let threshold_secs = threshold.as_secs() as i64;
    let candidates = crate::proposals::fetch_nudge_candidates(pool, threshold_secs).await;
    if candidates.is_empty() {
        return;
    }
    let mut proposal_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    tracing::info!("invite_nudge: {} candidate(s)", candidates.len());
    let mailer = crate::proposals::mailer_from(pool.clone(), resend.cloned());
    for c in &candidates {
        use crate::proposals::InviteMailer;
        mailer.send_invite(c.proposal_id, c.user_id, c.email_token.clone());
        proposal_ids.insert(c.proposal_id);
    }
    for pid in &proposal_ids {
        crate::proposals::mark_proposal_nudged(pool, *pid).await;
    }
}

pub fn spawn_invite_nudge_sweep(pool: PgPool, resend: Option<resend_rs::Resend>) {
    let interval = sweep_interval();
    tracing::info!("invite_nudge: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_invite_nudge_once(resend.as_ref(), &pool).await;
        }
    });
}

async fn sweep_invite_expiry_once(resend: Option<&resend_rs::Resend>, pool: &PgPool) {
    let threshold = invite_expiry_threshold();
    let threshold_secs = threshold.as_secs() as i64;
    let candidates = crate::proposals::fetch_expiry_candidates(pool, threshold_secs).await;
    if candidates.is_empty() {
        return;
    }
    tracing::info!("invite_expiry: {} candidate(s)", candidates.len());
    let mailer = crate::proposals::mailer_from(pool.clone(), resend.cloned());
    for proposal_id in candidates {
        if let Some((_owner_id, accepted_ids)) =
            crate::proposals::cancel_proposal_for_expiry(pool, proposal_id).await
        {
            use crate::proposals::InviteMailer;
            mailer.notify_cancelled(proposal_id, accepted_ids);
        }
    }
}

pub fn spawn_invite_expiry_sweep(pool: PgPool, resend: Option<resend_rs::Resend>) {
    let interval = sweep_interval();
    tracing::info!("invite_expiry: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_invite_expiry_once(resend.as_ref(), &pool).await;
        }
    });
}

async fn sweep_invite_auto_decline_once(
    pool: &PgPool,
    broadcaster: &crate::websocket::GameBroadcaster,
) {
    let threshold = invite_auto_decline_threshold();
    let threshold_secs = threshold.as_secs() as i64;
    let candidates = crate::proposals::fetch_auto_decline_candidates(pool, threshold_secs).await;
    if candidates.is_empty() {
        return;
    }
    tracing::info!("invite_auto_decline: {} candidate(s)", candidates.len());
    let mut proposal_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    for (player_id, proposal_id) in &candidates {
        crate::proposals::auto_decline_proposal_player(pool, *player_id).await;
        proposal_ids.insert(*proposal_id);
    }
    for pid in &proposal_ids {
        broadcaster.broadcast_proposal_update(*pid).await;
    }
}

pub fn spawn_invite_auto_decline_sweep(
    pool: PgPool,
    broadcaster: crate::websocket::GameBroadcaster,
) {
    let interval = sweep_interval();
    tracing::info!("invite_auto_decline: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_invite_auto_decline_once(&pool, &broadcaster).await;
        }
    });
}

pub fn spawn_periodic_sweeps(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
    broadcaster: crate::websocket::GameBroadcaster,
) {
    spawn_turn_reminder_sweep(pool.clone(), resend.clone(), http_client.clone());
    spawn_unverified_email_sweep(pool.clone());
    spawn_invite_nudge_sweep(pool.clone(), resend.clone());
    spawn_invite_expiry_sweep(pool.clone(), resend.clone());
    spawn_invite_auto_decline_sweep(pool.clone(), broadcaster);
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use time::{Date, Month, PrimitiveDateTime, Time};

    fn fixed_now() -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        )
    }

    #[test]
    fn should_reset_reminder_on_transition() {
        assert!(should_reset_reminder(false, true));
        assert!(should_reset_reminder(true, false));
        assert!(!should_reset_reminder(true, true));
        assert!(!should_reset_reminder(false, false));
    }

    #[test]
    fn candidate_predicate_accepts_due_player() {
        let now = fixed_now();
        let old_turn = now - time::Duration::hours(25);
        assert!(is_reminder_candidate(
            true,
            false,
            None,
            old_turn,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn candidate_predicate_rejects_already_reminded() {
        let now = fixed_now();
        let old_turn = now - time::Duration::hours(25);
        assert!(!is_reminder_candidate(
            true,
            false,
            Some(now - time::Duration::hours(1)),
            old_turn,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn candidate_predicate_rejects_not_turn() {
        let now = fixed_now();
        let old_turn = now - time::Duration::hours(25);
        assert!(!is_reminder_candidate(
            false,
            false,
            None,
            old_turn,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn candidate_predicate_rejects_eliminated() {
        let now = fixed_now();
        let old_turn = now - time::Duration::hours(25);
        assert!(!is_reminder_candidate(
            true,
            true,
            None,
            old_turn,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn candidate_predicate_rejects_below_threshold() {
        let now = fixed_now();
        let recent_turn = now - time::Duration::hours(23);
        assert!(!is_reminder_candidate(
            true,
            false,
            None,
            recent_turn,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn candidate_predicate_boundary_exact_threshold() {
        let now = fixed_now();
        let exact = now - time::Duration::hours(24);
        assert!(is_reminder_candidate(
            true,
            false,
            None,
            exact,
            now,
            std::time::Duration::from_secs(86400),
        ));
    }

    #[test]
    fn reminder_threshold_defaults_to_24h() {
        unsafe { std::env::remove_var("TURN_REMINDER_AFTER") };
        assert_eq!(reminder_threshold(), std::time::Duration::from_secs(86400));
    }

    #[test]
    fn reminder_threshold_parses_custom() {
        unsafe { std::env::set_var("TURN_REMINDER_AFTER", "2h") };
        assert_eq!(reminder_threshold(), std::time::Duration::from_secs(7200));
        unsafe { std::env::remove_var("TURN_REMINDER_AFTER") };
    }

    #[test]
    fn sweep_interval_defaults_to_15m() {
        unsafe { std::env::remove_var("TURN_REMINDER_SWEEP_INTERVAL") };
        assert_eq!(sweep_interval(), std::time::Duration::from_secs(900));
    }

    #[test]
    fn sweep_interval_parses_custom() {
        unsafe { std::env::set_var("TURN_REMINDER_SWEEP_INTERVAL", "5m") };
        assert_eq!(sweep_interval(), std::time::Duration::from_secs(300));
        unsafe { std::env::remove_var("TURN_REMINDER_SWEEP_INTERVAL") };
    }

    #[test]
    fn reminder_header_contains_name() {
        let h = reminder_header_text("Alice");
        assert!(h.contains("Alice"));
        assert!(h.contains("Still your turn"));
    }

    #[sqlx::test]
    async fn fetch_candidates_returns_due_players(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Sweep Test {}", Uuid::new_v4()))
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
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, reminder_emails_enabled) VALUES ($1, $2, true) RETURNING id",
        )
        .bind("sweep_player")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let gp_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read)
             VALUES ($1, $2, 0, 'Green', true, true,
                     NOW() - interval '48 hours', NOW(), false, false)
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert!(candidates.iter().any(|c| c.game_player_id == gp_id));
    }

    #[sqlx::test]
    async fn fetch_candidates_excludes_reminded(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Sweep Test2 {}", Uuid::new_v4()))
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
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, reminder_emails_enabled) VALUES ($1, $2, true) RETURNING id",
        )
        .bind("sweep_player2")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let gp_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read, turn_reminder_sent_at)
             VALUES ($1, $2, 0, 'Green', true, true,
                     NOW() - interval '48 hours', NOW(), false, false, NOW())
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert!(!candidates.iter().any(|c| c.game_player_id == gp_id));
    }

    #[sqlx::test]
    async fn fetch_candidates_excludes_reminder_disabled(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Sweep Test3 {}", Uuid::new_v4()))
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
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, reminder_emails_enabled) VALUES ($1, $2, false) RETURNING id",
        )
        .bind("sweep_player3")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let gp_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read)
             VALUES ($1, $2, 0, 'Green', true, true,
                     NOW() - interval '48 hours', NOW(), false, false)
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert!(!candidates.iter().any(|c| c.game_player_id == gp_id));
    }

    #[sqlx::test]
    async fn mark_reminder_sent_sets_timestamp(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Sweep Test3 {}", Uuid::new_v4()))
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
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("sweep_player3")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let gp_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read)
             VALUES ($1, $2, 0, 'Green', true, true, NOW(), NOW(), false, false)
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        mark_reminder_sent(&pool, gp_id).await;

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(sent_at.is_some());
    }

    #[sqlx::test]
    async fn reset_reminder_clears_timestamp(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Sweep Test4 {}", Uuid::new_v4()))
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
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("sweep_player4")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let gp_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_players
                (game_id, user_id, position, color, has_accepted, is_turn,
                 is_turn_at, last_turn_at, is_eliminated, is_read, turn_reminder_sent_at)
             VALUES ($1, $2, 0, 'Green', true, true, NOW(), NOW(), false, false, NOW())
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("UPDATE game_players SET turn_reminder_sent_at = NULL WHERE id = $1")
            .bind(gp_id)
            .execute(&pool)
            .await
            .unwrap();

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(sent_at.is_none());
    }

    #[test]
    fn unverified_email_expiry_is_24h() {
        assert_eq!(
            UNVERIFIED_EMAIL_EXPIRY,
            std::time::Duration::from_secs(86400)
        );
    }

    #[sqlx::test]
    async fn sweep_unverified_emails_deletes_expired_only(pool: PgPool) {
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
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, created_at)
             VALUES ($1, $2, false, NOW() - interval '48 hours')",
        )
        .bind(user_id)
        .bind(&expired)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind(&fresh)
            .execute(&pool)
            .await
            .unwrap();

        sweep_unverified_emails_once(&pool).await;

        let remaining: Vec<(String,)> =
            sqlx::query_as("SELECT email FROM user_emails WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(remaining.iter().any(|(e,)| e == &fresh));
        assert!(!remaining.iter().any(|(e,)| e == &expired));
    }

    #[test]
    fn invite_reminder_threshold_defaults_to_24h() {
        unsafe { std::env::remove_var("INVITE_REMINDER_AFTER") };
        assert_eq!(
            invite_reminder_threshold(),
            std::time::Duration::from_secs(86400)
        );
    }

    #[test]
    fn invite_reminder_threshold_parses_custom() {
        unsafe { std::env::set_var("INVITE_REMINDER_AFTER", "1d") };
        assert_eq!(
            invite_reminder_threshold(),
            std::time::Duration::from_secs(86400)
        );
        unsafe { std::env::remove_var("INVITE_REMINDER_AFTER") };
    }

    #[test]
    fn invite_expiry_threshold_defaults_to_14_days() {
        unsafe { std::env::remove_var("INVITE_EXPIRE_AFTER") };
        assert_eq!(
            invite_expiry_threshold(),
            std::time::Duration::from_secs(1209600)
        );
    }

    #[test]
    fn invite_expiry_threshold_parses_custom() {
        unsafe { std::env::set_var("INVITE_EXPIRE_AFTER", "7d") };
        assert_eq!(
            invite_expiry_threshold(),
            std::time::Duration::from_secs(604800)
        );
        unsafe { std::env::remove_var("INVITE_EXPIRE_AFTER") };
    }

    #[test]
    fn invite_auto_decline_threshold_defaults_to_48h() {
        unsafe { std::env::remove_var("INVITE_AUTO_DECLINE_AFTER") };
        assert_eq!(
            invite_auto_decline_threshold(),
            std::time::Duration::from_secs(172800)
        );
    }

    #[test]
    fn invite_auto_decline_threshold_parses_custom() {
        unsafe { std::env::set_var("INVITE_AUTO_DECLINE_AFTER", "2d") };
        assert_eq!(
            invite_auto_decline_threshold(),
            std::time::Duration::from_secs(172800)
        );
        unsafe { std::env::remove_var("INVITE_AUTO_DECLINE_AFTER") };
    }
}
