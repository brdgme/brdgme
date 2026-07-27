//! #22c turn-reminder sweep: a periodic tokio task that nudges players who have
//! held the turn past a threshold. One reminder per turn (reset on transition).
//! Structured for future periodic jobs (22d unverified-email cleanup, #24 invite
//! nudge/expiry) via `spawn_periodic_sweeps`.

use sqlx::PgPool;
use uuid::Uuid;

pub const DEFAULT_REMINDER_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(86400);

pub const DEFAULT_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(900);

const REMINDER_SWEEP_LIMIT: i64 = 200;

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

fn reminder_header_text(player_name: &str) -> String {
    format!("Still your turn, {player_name}.")
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
          LIMIT $2",
    )
    .bind(threshold_secs.to_string())
    .bind(REMINDER_SWEEP_LIMIT)
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

async fn mark_reminder_sent_tx(
    tx: &mut sqlx::PgConnection,
    game_player_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE game_players SET turn_reminder_sent_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(game_player_id)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

enum ReminderOutcome {
    Sent,
    PermanentSkip,
    Retry,
}

async fn send_reminder(
    resend: Option<&resend_rs::Resend>,
    pool: &PgPool,
    http_client: &reqwest::Client,
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    game_player_id: Uuid,
) -> ReminderOutcome {
    let ge = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            tracing::warn!("turn_reminder: game {} not found", game_id);
            return ReminderOutcome::PermanentSkip;
        }
        Err(e) => {
            tracing::error!("turn_reminder: failed to load game {}: {}", game_id, e);
            return ReminderOutcome::Retry;
        }
    };

    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == game_player_id)
    {
        Some(p) => p,
        None => return ReminderOutcome::PermanentSkip,
    };

    let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
    {
        Ok(Some(r)) => r,
        _ => return ReminderOutcome::PermanentSkip,
    };

    if !(recipient.email.is_some() && !recipient.is_bot && recipient.reminder_emails_enabled) {
        return ReminderOutcome::PermanentSkip;
    }
    if crate::email::outbound::suppress_for_web_presence(pool, recipient.user_id).await {
        return ReminderOutcome::Retry;
    }

    let token = match crate::email::outbound::ensure_email_token_tx(&mut *tx, game_player_id).await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "turn_reminder: failed to ensure email token for {}: {}",
                game_player_id,
                e
            );
            return ReminderOutcome::Retry;
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

    let unsub_token: Option<String> = match recipient.user_id {
        Some(uid) => match crate::email::outbound::ensure_unsubscribe_token(pool, uid).await {
            Ok(tok) => Some(tok),
            Err(err) => {
                tracing::warn!(
                    "turn_reminder: unsubscribe token fetch failed for {}: {}",
                    uid,
                    err
                );
                None
            }
        },
        None => None,
    };
    let unsubscribe = unsub_token
        .as_ref()
        .map(|tok| crate::email::render::Unsubscribe {
            kind: crate::email::render::EmailKind::Reminder,
            token: tok,
        });

    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        Some(&format!("game-{game_id}")),
        false,
        &crate::email::notify::reply_address(&token),
        unsubscribe,
    );

    let to = match recipient.email {
        Some(e) => e,
        None => return ReminderOutcome::PermanentSkip,
    };
    if crate::email::outbound::try_send_rendered_email(resend, rendered, &to).await {
        ReminderOutcome::Sent
    } else {
        ReminderOutcome::Retry
    }
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
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!(
                    "turn_reminder: failed to begin tx for {}: {}",
                    c.game_player_id,
                    e
                );
                continue;
            }
        };
        let claimed: Option<Uuid> = match sqlx::query_scalar(
            "SELECT id FROM game_players
             WHERE id = $1 AND turn_reminder_sent_at IS NULL AND is_turn = true
             FOR UPDATE SKIP LOCKED",
        )
        .bind(c.game_player_id)
        .fetch_optional(&mut *tx)
        .await
        {
            Ok(opt) => opt,
            Err(e) => {
                tracing::error!(
                    "turn_reminder: claim query failed for {}: {}",
                    c.game_player_id,
                    e
                );
                continue;
            }
        };
        if claimed.is_none() {
            continue;
        }
        let outcome = send_reminder(
            resend,
            pool,
            http_client,
            &mut tx,
            c.game_id,
            c.game_player_id,
        )
        .await;
        match outcome {
            ReminderOutcome::Sent | ReminderOutcome::PermanentSkip => {
                if let Err(e) = mark_reminder_sent_tx(&mut tx, c.game_player_id).await {
                    tracing::error!(
                        "turn_reminder: failed to mark sent for {}: {}",
                        c.game_player_id,
                        e
                    );
                    continue;
                }
                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        "turn_reminder: failed to commit for {}: {}",
                        c.game_player_id,
                        e
                    );
                }
            }
            ReminderOutcome::Retry => {}
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

pub const DEFAULT_BOT_TURN_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(900);

pub const DEFAULT_BOT_TURN_SWEEP_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(900);

pub fn bot_turn_threshold() -> std::time::Duration {
    std::env::var("BOT_TURN_THRESHOLD")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_BOT_TURN_THRESHOLD)
}

pub fn bot_turn_sweep_interval() -> std::time::Duration {
    std::env::var("BOT_TURN_SWEEP_INTERVAL")
        .ok()
        .and_then(|v| crate::email::outbound::parse_duration(&v))
        .unwrap_or(DEFAULT_BOT_TURN_SWEEP_INTERVAL)
}

#[derive(Debug, sqlx::FromRow)]
struct BotTurnCandidate {
    game_id: Uuid,
    position: i32,
    bot_name: String,
    is_dangling: bool,
}

async fn fetch_bot_turn_candidates(
    pool: &PgPool,
    threshold: std::time::Duration,
) -> Vec<BotTurnCandidate> {
    let threshold_secs = threshold.as_secs() as i64;
    let rows = sqlx::query_as::<_, BotTurnCandidate>(
        "SELECT gp.game_id AS game_id,
                gp.position AS position,
                gb.bot_name AS bot_name,
                (b.id IS NULL OR b.enabled = false) AS is_dangling
         FROM game_players gp
         JOIN games g ON gp.game_id = g.id
         JOIN game_bots gb ON gp.game_bot_id = gb.id
         LEFT JOIN bots b ON gb.bot_name = b.name
         WHERE gp.is_turn = true
           AND gp.game_bot_id IS NOT NULL
           AND g.is_finished = false
           AND gp.is_turn_at < NOW() - ($1 || ' seconds')::interval",
    )
    .bind(threshold_secs.to_string())
    .fetch_all(pool)
    .await;
    match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("bot_turn_sweep: candidate query failed: {}", e);
            Vec::new()
        }
    }
}

async fn sweep_bot_turns_once(pool: &PgPool, jetstream: &async_nats::jetstream::Context) {
    let threshold = bot_turn_threshold();
    let candidates = fetch_bot_turn_candidates(pool, threshold).await;
    let mut dangling_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for c in &candidates {
        if c.is_dangling {
            dangling_names.insert(c.bot_name.clone());
            continue;
        }
        let turns = vec![crate::db::BotTurn {
            position: c.position,
            bot_name: c.bot_name.clone(),
        }];
        crate::game::publish_bot_turns(jetstream, c.game_id, &turns, 0).await;
        axum_prometheus::metrics::counter!("bot_turn_sweep_republished_total").increment(1);
    }
    axum_prometheus::metrics::gauge!("bot_turn_dangling_bot_names")
        .set(dangling_names.len() as f64);
}

pub fn spawn_bot_turn_sweep(pool: PgPool, jetstream: async_nats::jetstream::Context) {
    let interval = bot_turn_sweep_interval();
    tracing::info!("bot_turn_sweep: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_bot_turns_once(&pool, &jetstream).await;
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

/// wfe F11: processed webhook events older than this are pruned. 7 days is
/// comfortably past the svix retry window (~5 days).
pub const PROCESSED_WEBHOOK_EVENT_RETENTION: std::time::Duration =
    std::time::Duration::from_secs(7 * 86400);

async fn sweep_processed_webhook_events_once(pool: &PgPool) {
    match crate::db::delete_old_processed_webhook_events(pool, PROCESSED_WEBHOOK_EVENT_RETENTION)
        .await
    {
        Ok(0) => {}
        Ok(n) => tracing::info!("processed_webhook_event_prune: deleted {} row(s)", n),
        Err(e) => tracing::error!("processed_webhook_event_prune: delete failed: {}", e),
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
            sweep_processed_webhook_events_once(&pool).await;
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
    tracing::info!("invite_nudge: {} candidate(s)", candidates.len());
    let mailer = crate::proposals::mailer_from(pool.clone(), resend.cloned());
    let mut all_sent: std::collections::HashMap<Uuid, bool> = std::collections::HashMap::new();
    for c in &candidates {
        use crate::proposals::InviteMailer;
        let ok = mailer
            .send_invite_now(c.proposal_id, c.user_id, c.email_token.clone())
            .await;
        *all_sent.entry(c.proposal_id).or_insert(true) &= ok;
    }
    for (pid, sent) in &all_sent {
        if *sent {
            crate::proposals::mark_proposal_nudged(pool, *pid).await;
        }
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
    resend: Option<&resend_rs::Resend>,
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
    let mailer = crate::proposals::mailer_from(pool.clone(), resend.cloned());
    let mut proposal_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    for (player_id, proposal_id, user_id) in &candidates {
        let declined = crate::proposals::auto_decline_proposal_player(pool, *player_id).await;
        if declined {
            proposal_ids.insert(*proposal_id);
            use crate::proposals::InviteMailer;
            mailer.notify_owner_decline(*proposal_id, *user_id);
        }
    }
    for pid in &proposal_ids {
        broadcaster.broadcast_proposal_update(*pid).await;
    }
}

pub fn spawn_invite_auto_decline_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    broadcaster: crate::websocket::GameBroadcaster,
) {
    let interval = sweep_interval();
    tracing::info!("invite_auto_decline: sweep every {:?}", interval);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            sweep_invite_auto_decline_once(resend.as_ref(), &pool, &broadcaster).await;
        }
    });
}

pub fn spawn_periodic_sweeps(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
    broadcaster: crate::websocket::GameBroadcaster,
    jetstream: async_nats::jetstream::Context,
) {
    spawn_turn_reminder_sweep(pool.clone(), resend.clone(), http_client.clone());
    spawn_unverified_email_sweep(pool.clone());
    spawn_invite_nudge_sweep(pool.clone(), resend.clone());
    spawn_invite_expiry_sweep(pool.clone(), resend.clone());
    spawn_invite_auto_decline_sweep(pool.clone(), resend.clone(), broadcaster);
    spawn_bot_turn_sweep(pool.clone(), jetstream);
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
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

        let mut tx = pool.begin().await.unwrap();
        mark_reminder_sent_tx(&mut tx, gp_id).await.unwrap();
        tx.commit().await.unwrap();

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

    /// One game with a single emailable, turn-emails-enabled player; returns
    /// `(game_id, user_id, game_player_id)`.
    async fn seed_reminder_game(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Reminder {}", Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://127.0.0.1:1', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let game_id: Uuid = sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, false, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, turn_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(format!("u-{}@example.com", Uuid::new_v4()))
        .execute(pool)
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
        .fetch_one(pool)
        .await
        .unwrap();
        (game_id, user_id, gp_id)
    }

    // The turn reminder is skipped while the recipient is active on the web (no
    // reply token minted => the send returned before rendering) and sent once
    // they are no longer active.
    #[sqlx::test]
    async fn turn_reminder_suppressed_by_recipient_presence(pool: PgPool) {
        let (game_id, user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        send_reminder(None, &pool, &http, &mut tx, game_id, gp_id).await;
        tx.commit().await.unwrap();
        let token: Option<String> =
            sqlx::query_scalar("SELECT email_token FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            token.is_none(),
            "turn reminder should be suppressed while recipient is active on the web"
        );

        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        send_reminder(None, &pool, &http, &mut tx, game_id, gp_id).await;
        tx.commit().await.unwrap();
        let token: Option<String> =
            sqlx::query_scalar("SELECT email_token FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            token.is_some(),
            "turn reminder should send once the recipient is no longer active"
        );
    }

    // F30 (headline): a web-present recipient is NOT marked (presence => Retry =>
    // rollback), and is sent+marked once presence lapses. Drives `sweep_once`,
    // not `send_reminder`. With `resend = None` there is no send counter, so we
    // assert on the DB mark (`turn_reminder_sent_at`).
    #[sqlx::test]
    async fn turn_reminder_sweep_suppressed_by_presence_then_sends(pool: PgPool) {
        let (_game_id, user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        sweep_once(None, &pool, &http).await;
        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            sent_at.is_none(),
            "presence => Retry => row must NOT be marked"
        );

        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        sweep_once(None, &pool, &http).await;
        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            sent_at.is_some(),
            "lapsed presence => Sent => row must be marked"
        );
    }

    // F32 / D-11: `reminder_emails_enabled` ALONE governs reminders. A user with
    // reminders on but turn emails OFF is still selected and marked sent through
    // `sweep_once` (the reminder sweep never consults `turn_emails_enabled`).
    #[sqlx::test]
    async fn turn_reminder_sweep_ignores_turn_emails_disabled(pool: PgPool) {
        let (_game_id, user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        sqlx::query("UPDATE users SET turn_emails_enabled = false WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert!(
            candidates.iter().any(|c| c.game_player_id == gp_id),
            "reminder_emails_enabled alone selects the candidate"
        );

        sweep_once(None, &pool, &http).await;
        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            sent_at.is_some(),
            "turn_emails_enabled=false must NOT stop the reminder sweep"
        );
    }

    // F31: two concurrent sweeps over ONE due candidate mark it exactly once.
    // The claim re-checks `turn_reminder_sent_at IS NULL` under FOR UPDATE SKIP
    // LOCKED, so only one replica can mark. With `resend = None` we cannot count
    // sends directly; we assert the row ends marked (it can only be marked once).
    #[sqlx::test]
    async fn turn_reminder_sweep_concurrent_marks_once(pool: PgPool) {
        let (_game_id, _user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        let ((), ()) = tokio::join!(
            sweep_once(None, &pool, &http),
            sweep_once(None, &pool, &http),
        );

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(sent_at.is_some(), "the due candidate must be marked");
    }

    // F40: `fetch_candidates` caps at REMINDER_SWEEP_LIMIT (200). Seed 201 due
    // candidates (one player per game, sharing a version) and assert exactly 200
    // come back.
    #[sqlx::test]
    async fn fetch_candidates_caps_at_sweep_limit(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Limit {}", Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(&pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://127.0.0.1:1', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let want = REMINDER_SWEEP_LIMIT + 1;
        for _ in 0..want {
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
            .bind(format!("u-{}", Uuid::new_v4()))
            .bind(Vec::<String>::new())
            .fetch_one(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO game_players
                     (game_id, user_id, position, color, has_accepted, is_turn,
                      is_turn_at, last_turn_at, is_eliminated, is_read)
                 VALUES ($1, $2, 0, 'Green', true, true,
                         NOW() - interval '48 hours', NOW(), false, false)",
            )
            .bind(game_id)
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        }

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert_eq!(candidates.len(), REMINDER_SWEEP_LIMIT as usize);
    }

    async fn seed_bot_sweep_game(pool: &PgPool, is_finished: bool) -> Uuid {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("BotSweep {}", Uuid::new_v4()))
        .bind(vec![2i32])
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO games (game_version_id, is_finished, game_state)
             VALUES ($1, $2, 'state') RETURNING id",
        )
        .bind(game_version_id)
        .bind(is_finished)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_bot_type(pool: &PgPool, enabled: bool) -> String {
        let name = format!("bot-{}", Uuid::new_v4());
        sqlx::query("INSERT INTO bots (name, enabled) VALUES ($1, $2)")
            .bind(&name)
            .bind(enabled)
            .execute(pool)
            .await
            .unwrap();
        name
    }

    async fn seed_bot_player(
        pool: &PgPool,
        game_id: Uuid,
        position: i32,
        color: &str,
        bot_type: &str,
        age_secs: i64,
    ) -> Uuid {
        let gb_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(game_id)
        .bind(format!("Bot {}", Uuid::new_v4()))
        .bind(bot_type)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO game_players
                 (game_id, user_id, position, color, has_accepted, is_turn,
                  is_turn_at, last_turn_at, is_eliminated, is_read, game_bot_id)
             VALUES ($1, NULL, $2, $3, true, true,
                     NOW() - ($4 || ' seconds')::interval, NOW(), false, false, $5)
             RETURNING id",
        )
        .bind(game_id)
        .bind(position)
        .bind(color)
        .bind(age_secs.to_string())
        .bind(gb_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_human_player(
        pool: &PgPool,
        game_id: Uuid,
        position: i32,
        color: &str,
        age_secs: i64,
    ) -> Uuid {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO game_players
                 (game_id, user_id, position, color, has_accepted, is_turn,
                  is_turn_at, last_turn_at, is_eliminated, is_read)
             VALUES ($1, $2, $3, $4, true, true,
                     NOW() - ($5 || ' seconds')::interval, NOW(), false, false)
             RETURNING id",
        )
        .bind(game_id)
        .bind(user_id)
        .bind(position)
        .bind(color)
        .bind(age_secs.to_string())
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn bot_turn_candidates_exclude_human_players(pool: PgPool) {
        let game_id = seed_bot_sweep_game(&pool, false).await;
        seed_human_player(&pool, game_id, 0, "Green", 3600).await;

        let candidates =
            fetch_bot_turn_candidates(&pool, std::time::Duration::from_secs(1800)).await;
        assert!(
            !candidates.iter().any(|c| c.game_id == game_id),
            "a human player (game_bot_id NULL) must never be a bot-turn candidate"
        );
    }

    #[sqlx::test]
    async fn bot_turn_candidates_exclude_finished_games(pool: PgPool) {
        let game_id = seed_bot_sweep_game(&pool, true).await;
        let bot_type = seed_bot_type(&pool, true).await;
        seed_bot_player(&pool, game_id, 0, "Green", &bot_type, 3600).await;

        let candidates =
            fetch_bot_turn_candidates(&pool, std::time::Duration::from_secs(1800)).await;
        assert!(
            !candidates.iter().any(|c| c.game_id == game_id),
            "a finished game must be excluded"
        );
    }

    #[sqlx::test]
    async fn bot_turn_candidates_exclude_recent_turns(pool: PgPool) {
        let game_id = seed_bot_sweep_game(&pool, false).await;
        let bot_type = seed_bot_type(&pool, true).await;
        seed_bot_player(&pool, game_id, 0, "Green", &bot_type, 60).await;

        let candidates =
            fetch_bot_turn_candidates(&pool, std::time::Duration::from_secs(1800)).await;
        assert!(
            !candidates.iter().any(|c| c.game_id == game_id),
            "a bot whose is_turn_at is within the threshold must be excluded"
        );
    }

    #[sqlx::test]
    async fn bot_turn_candidates_partition_live_and_dangling(pool: PgPool) {
        let game_id = seed_bot_sweep_game(&pool, false).await;

        let live_type = seed_bot_type(&pool, true).await;
        seed_bot_player(&pool, game_id, 0, "Green", &live_type, 3600).await;

        let disabled_type = seed_bot_type(&pool, false).await;
        seed_bot_player(&pool, game_id, 1, "Blue", &disabled_type, 3600).await;

        let missing_type = format!("missing-{}", Uuid::new_v4());
        seed_bot_player(&pool, game_id, 2, "Red", &missing_type, 3600).await;

        let candidates =
            fetch_bot_turn_candidates(&pool, std::time::Duration::from_secs(1800)).await;
        let mine: Vec<&BotTurnCandidate> =
            candidates.iter().filter(|c| c.game_id == game_id).collect();
        assert_eq!(mine.len(), 3);

        let live = mine.iter().find(|c| c.bot_name == live_type).unwrap();
        assert!(!live.is_dangling, "an enabled bot is a LIVE candidate");
        assert_eq!(live.position, 0);

        let disabled = mine.iter().find(|c| c.bot_name == disabled_type).unwrap();
        assert!(disabled.is_dangling, "a disabled bot is DANGLING");

        let missing = mine.iter().find(|c| c.bot_name == missing_type).unwrap();
        assert!(missing.is_dangling, "a bot with no bots row is DANGLING");

        assert_eq!(mine.iter().filter(|c| !c.is_dangling).count(), 1);
        assert_eq!(mine.iter().filter(|c| c.is_dangling).count(), 2);
    }
}
