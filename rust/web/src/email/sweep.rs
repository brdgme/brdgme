//! #22c turn-reminder sweep: a periodic tokio task that nudges players who have
//! held the turn past a threshold. One reminder per turn (reset on transition).
//! Structured for future periodic jobs (22d unverified-email cleanup, #24 invite
//! nudge/expiry) via `spawn_periodic_sweeps`.

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub const DEFAULT_REMINDER_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(86400);

pub const DEFAULT_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(900);

const REMINDER_SWEEP_LIMIT: i64 = 200;

pub fn reminder_threshold() -> std::time::Duration {
    std::env::var("TURN_REMINDER_AFTER")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_REMINDER_THRESHOLD)
}

pub fn sweep_interval() -> std::time::Duration {
    std::env::var("TURN_REMINDER_SWEEP_INTERVAL")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_SWEEP_INTERVAL)
}

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
        "UPDATE game_players SET turn_reminder_sent_at = NOW(), updated_at = NOW()
         WHERE id = $1 AND turn_reminder_sent_at IS NULL AND is_turn = true",
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
    game_id: Uuid,
    game_player_id: Uuid,
    token: String,
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
        Ok(None) => return ReminderOutcome::PermanentSkip,
        Err(e) => {
            tracing::error!(
                "turn_reminder: recipient lookup failed for {}: {}",
                game_player_id,
                e
            );
            return ReminderOutcome::Retry;
        }
    };

    if !(recipient.email.is_some() && !recipient.is_bot && recipient.reminder_emails_enabled) {
        return ReminderOutcome::PermanentSkip;
    }
    if crate::email::outbound::suppress_for_web_presence(pool, recipient.user_id).await {
        return ReminderOutcome::Retry;
    }

    let palette = crate::email::render::palette_for_slug(recipient.theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();

    let log_count = crate::email::notify::game_log_count(pool, game_id).await;
    let subject =
        crate::email::notify::turn_subject_or_fallback(&ge.game_type.name, game_id, log_count);
    let header = Some(crate::email::notify::reminder_header_text(
        recipient_player.name(),
    ));

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
        None,
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
        let mut claim_tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!(
                    "turn_reminder: failed to begin claim tx for {}: {}",
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
        .fetch_optional(&mut *claim_tx)
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
        let token = match crate::email::outbound::ensure_email_token_tx(
            &mut *claim_tx,
            c.game_player_id,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "turn_reminder: failed to ensure email token for {}: {}",
                    c.game_player_id,
                    e
                );
                continue;
            }
        };
        if let Err(e) = claim_tx.commit().await {
            tracing::error!(
                "turn_reminder: failed to commit claim tx for {}: {}",
                c.game_player_id,
                e
            );
            continue;
        }
        let outcome = send_reminder(
            resend,
            pool,
            http_client,
            c.game_id,
            c.game_player_id,
            token,
        )
        .await;
        match outcome {
            ReminderOutcome::Sent | ReminderOutcome::PermanentSkip => {
                let mut mark_tx = match pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        tracing::error!(
                            "turn_reminder: failed to begin mark tx for {}: {}",
                            c.game_player_id,
                            e
                        );
                        continue;
                    }
                };
                if let Err(e) = mark_reminder_sent_tx(&mut mark_tx, c.game_player_id).await {
                    tracing::error!(
                        "turn_reminder: failed to mark sent for {}: {}",
                        c.game_player_id,
                        e
                    );
                    continue;
                }
                if let Err(e) = mark_tx.commit().await {
                    tracing::error!(
                        "turn_reminder: failed to commit mark tx for {}: {}",
                        c.game_player_id,
                        e
                    );
                }
            }
            ReminderOutcome::Retry => {}
        }
    }
}

/// Spawns one periodic sweep: a `MissedTickBehavior::Skip` interval that runs
/// `run()` every tick until `shutdown` is cancelled (R-11 / F-109). Six sweeps
/// used to repeat this loop verbatim (wfe F38). Returns the `JoinHandle` so
/// `main` can boundedly drain the sweep on shutdown.
fn spawn_sweep<F, Fut>(
    name: &'static str,
    interval: std::time::Duration,
    shutdown: CancellationToken,
    mut run: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    tracing::info!("{name}: sweep every {interval:?}");
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("{name}: shutdown signalled; stopping sweep");
                    return;
                }
                _ = tick.tick() => {}
            }
            run().await;
        }
    })
}

pub fn spawn_turn_reminder_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    spawn_sweep("turn_reminder", sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        let resend = resend.clone();
        let http_client = http_client.clone();
        async move { sweep_once(resend.as_ref(), &pool, &http_client).await }
    })
}

pub const DEFAULT_BOT_TURN_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(900);

pub const DEFAULT_BOT_TURN_SWEEP_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(900);

pub fn bot_turn_threshold() -> std::time::Duration {
    std::env::var("BOT_TURN_THRESHOLD")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_BOT_TURN_THRESHOLD)
}

pub fn bot_turn_sweep_interval() -> std::time::Duration {
    std::env::var("BOT_TURN_SWEEP_INTERVAL")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_BOT_TURN_SWEEP_INTERVAL)
}

#[derive(Debug, sqlx::FromRow)]
struct BotTurnCandidate {
    game_id: Uuid,
    position: i32,
    bot_name: String,
    is_dangling: bool,
    updated_at: time::PrimitiveDateTime,
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
                (b.id IS NULL OR b.enabled = false) AS is_dangling,
                g.updated_at AS updated_at
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
            updated_at: c.updated_at,
        }];
        crate::game::publish_bot_turns(jetstream, c.game_id, &turns, 0).await;
        axum_prometheus::metrics::counter!("bot_turn_sweep_republished_total").increment(1);
    }
    axum_prometheus::metrics::gauge!("bot_turn_dangling_bot_names")
        .set(dangling_names.len() as f64);
}

pub fn spawn_bot_turn_sweep(
    pool: PgPool,
    jetstream: async_nats::jetstream::Context,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    spawn_sweep("bot_turn_sweep", bot_turn_sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        let jetstream = jetstream.clone();
        async move { sweep_bot_turns_once(&pool, &jetstream).await }
    })
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
pub fn spawn_unverified_email_sweep(pool: PgPool, shutdown: CancellationToken) -> tokio::task::JoinHandle<()> {
    spawn_sweep("unverified_email_expiry", sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        async move {
            sweep_unverified_emails_once(&pool).await;
            sweep_processed_webhook_events_once(&pool).await;
        }
    })
}

pub const DEFAULT_INVITE_REMINDER_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(86400);

pub const DEFAULT_INVITE_EXPIRY_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(1209600);

pub fn invite_reminder_threshold() -> std::time::Duration {
    std::env::var("INVITE_REMINDER_AFTER")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_INVITE_REMINDER_THRESHOLD)
}

pub fn invite_expiry_threshold() -> std::time::Duration {
    std::env::var("INVITE_EXPIRE_AFTER")
        .ok()
        .and_then(|v| parse_duration(&v))
        .unwrap_or(DEFAULT_INVITE_EXPIRY_THRESHOLD)
}

pub const DEFAULT_INVITE_AUTO_DECLINE_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(172800);

pub fn invite_auto_decline_threshold() -> std::time::Duration {
    std::env::var("INVITE_AUTO_DECLINE_AFTER")
        .ok()
        .and_then(|v| parse_duration(&v))
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
    for c in &candidates {
        use crate::proposals::InviteMailer;
        let ok = mailer
            .send_invite_now(c.proposal_id, c.user_id, c.email_token.clone())
            .await;
        if ok {
            crate::proposals::mark_proposal_player_nudged(pool, c.game_proposal_player_id).await;
        }
    }
}

pub fn spawn_invite_nudge_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    spawn_sweep("invite_nudge", sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        let resend = resend.clone();
        async move { sweep_invite_nudge_once(resend.as_ref(), &pool).await }
    })
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

pub fn spawn_invite_expiry_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    spawn_sweep("invite_expiry", sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        let resend = resend.clone();
        async move { sweep_invite_expiry_once(resend.as_ref(), &pool).await }
    })
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
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    spawn_sweep("invite_auto_decline", sweep_interval(), shutdown, move || {
        let pool = pool.clone();
        let resend = resend.clone();
        let broadcaster = broadcaster.clone();
        async move { sweep_invite_auto_decline_once(resend.as_ref(), &pool, &broadcaster).await }
    })
}

/// Spawns all periodic email/bot sweeps, each observing `shutdown` (R-11 /
/// F-109). Returns their `JoinHandle`s so `main` can boundedly drain them on
/// shutdown.
pub fn spawn_periodic_sweeps(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
    broadcaster: crate::websocket::GameBroadcaster,
    jetstream: async_nats::jetstream::Context,
    shutdown: CancellationToken,
) -> Vec<tokio::task::JoinHandle<()>> {
    vec![
        spawn_turn_reminder_sweep(pool.clone(), resend.clone(), http_client.clone(), shutdown.clone()),
        spawn_unverified_email_sweep(pool.clone(), shutdown.clone()),
        spawn_invite_nudge_sweep(pool.clone(), resend.clone(), shutdown.clone()),
        spawn_invite_expiry_sweep(pool.clone(), resend.clone(), shutdown.clone()),
        spawn_invite_auto_decline_sweep(pool.clone(), resend.clone(), broadcaster, shutdown.clone()),
        spawn_bot_turn_sweep(pool.clone(), jetstream, shutdown.clone()),
    ]
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

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

    // The turn reminder is skipped while the recipient is active on the web
    // (Retry) and sent once they are no longer active (Sent).
    #[sqlx::test]
    async fn turn_reminder_suppressed_by_recipient_presence(pool: PgPool) {
        let (game_id, user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        let mut tx = pool.begin().await.unwrap();
        let token = crate::email::outbound::ensure_email_token_tx(&mut *tx, gp_id)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .unwrap();
        let outcome = send_reminder(None, &pool, &http, game_id, gp_id, token.clone()).await;
        assert!(
            matches!(outcome, ReminderOutcome::Retry),
            "turn reminder should be suppressed while recipient is active on the web"
        );

        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
        let outcome = send_reminder(None, &pool, &http, game_id, gp_id, token).await;
        assert!(
            matches!(outcome, ReminderOutcome::Sent),
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

    // R-08 / F-136: a transient DB error in `fetch_email_recipient` must be
    // classified Retry (row left unmarked), not PermanentSkip (row marked sent
    // with nothing sent). Renaming `user_emails` makes the recipient lookup fail
    // while `fetch_candidates` and the claim query still succeed, driving the
    // full `sweep_once` path through the classifier at sweep.rs:134-138.
    #[sqlx::test]
    async fn turn_reminder_transient_recipient_lookup_error_leaves_row_unmarked(pool: PgPool) {
        let (_game_id, _user_id, gp_id) = seed_reminder_game(&pool).await;
        let http = reqwest::Client::new();

        let candidates = fetch_candidates(&pool, std::time::Duration::from_secs(86400)).await;
        assert!(
            candidates.iter().any(|c| c.game_player_id == gp_id),
            "the seeded player must be a candidate before the fault is injected"
        );

        sqlx::query("ALTER TABLE user_emails RENAME TO user_emails_r08_hidden")
            .execute(&pool)
            .await
            .unwrap();

        sweep_once(None, &pool, &http).await;

        sqlx::query("ALTER TABLE user_emails_r08_hidden RENAME TO user_emails")
            .execute(&pool)
            .await
            .unwrap();

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            sent_at.is_none(),
            "a transient DB error must classify as Retry: row must remain unmarked"
        );
    }

    // R-08 / F-145: a transient DB error in `mailer_recipient` (the invitee
    // lookup inside `send_invite_core`) must cause `send_invite_now` to return
    // `false` so `sweep_invite_nudge_once` does NOT call `mark_proposal_nudged`.
    // Renaming `user_emails` makes `fetch_invite_recipient` fail while
    // `fetch_nudge_candidates` (which only touches `game_proposals` and
    // `game_proposal_players`) still returns the candidate.
    #[sqlx::test]
    async fn invite_nudge_transient_lookup_error_leaves_proposal_unmarked(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Nudge {}", Uuid::new_v4()))
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

        let owner: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(owner)
        .bind(format!("owner-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        let invitee: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(invitee)
        .bind(format!("invitee-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        let pid: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, created_at)
             VALUES ($1, $2, 'open', NOW() - interval '48 hours') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner)
        .fetch_one(&pool)
        .await
        .unwrap();

        let mut tx = pool.begin().await.unwrap();
        crate::proposals::insert_proposal_player(
            &mut tx,
            pid,
            0,
            Some(owner),
            None,
            None,
            "accepted",
            None,
        )
        .await
        .unwrap();
        crate::proposals::insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(invitee),
            None,
            None,
            "pending",
            Some(format!("tok-{}", Uuid::new_v4())),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let candidates =
            crate::proposals::fetch_nudge_candidates(&pool, 86400).await;
        assert!(
            candidates.iter().any(|c| c.proposal_id == pid),
            "the seeded proposal must be a nudge candidate before the fault is injected"
        );

        sqlx::query("ALTER TABLE user_emails RENAME TO user_emails_r08_nudge_hidden")
            .execute(&pool)
            .await
            .unwrap();

        sweep_invite_nudge_once(None, &pool).await;

        sqlx::query("ALTER TABLE user_emails_r08_nudge_hidden RENAME TO user_emails")
            .execute(&pool)
            .await
            .unwrap();

        let nudged_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT nudged_at FROM game_proposals WHERE id = $1")
                .bind(pid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            nudged_at.is_none(),
            "a transient DB error must prevent marking the proposal as nudged"
        );
    }

    // R-19 / F-144: the nudge dedup must be keyed PER-INVITEE, not per-proposal.
    // Seed one open proposal with TWO pending, emailable invitees and make one
    // web-present so its `send_invite_now` returns `false` (a transient no-send,
    // the web-presence suppression at proposals.rs:280-284). Run the nudge sweep
    // twice. The sendable invitee must be recorded as nudged exactly once - its
    // own per-invitee marker set - while the retrying (web-present) invitee stays
    // unmarked so it is retried next tick. Asserts the per-invitee marker state
    // directly (`game_proposal_players.nudged_at`), not logs.
    //
    // RED against the current code: the dedup gate and the mark are per-proposal
    // (`game_proposals.nudged_at`), there is NO per-invitee marker, so the whole
    // roster is re-selected and re-nudged every tick for as long as one invitee
    // stays web-present. This test cannot compile against the current schema
    // because the per-invitee `game_proposal_players.nudged_at` column does not
    // exist yet - it is the marker the fix introduces.
    #[sqlx::test]
    async fn invite_nudge_dedup_is_per_invitee(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Nudge {}", Uuid::new_v4()))
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

        let owner: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(owner)
        .bind(format!("owner-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        // The sendable invitee: emailable and never active on the web, so its
        // send proceeds (returns true).
        let sendable: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(sendable)
        .bind(format!("sendable-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        // The retrying invitee: emailable but web-present, so its send is
        // transiently suppressed (returns false).
        let retrying: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, true) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(retrying)
        .bind(format!("retrying-{}@example.com", Uuid::new_v4()))
        .execute(&pool)
        .await
        .unwrap();

        let pid: Uuid = sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status, created_at)
             VALUES ($1, $2, 'open', NOW() - interval '48 hours') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner)
        .fetch_one(&pool)
        .await
        .unwrap();

        let mut tx = pool.begin().await.unwrap();
        crate::proposals::insert_proposal_player(
            &mut tx,
            pid,
            0,
            Some(owner),
            None,
            None,
            "accepted",
            None,
        )
        .await
        .unwrap();
        let sendable_pp: Uuid = crate::proposals::insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(sendable),
            None,
            None,
            "pending",
            Some(format!("tok-{}", Uuid::new_v4())),
        )
        .await
        .unwrap();
        let retrying_pp: Uuid = crate::proposals::insert_proposal_player(
            &mut tx,
            pid,
            2,
            Some(retrying),
            None,
            None,
            "pending",
            Some(format!("tok-{}", Uuid::new_v4())),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Make the retrying invitee web-present so its send is a transient no-send.
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(retrying)
            .execute(&pool)
            .await
            .unwrap();

        let candidates = crate::proposals::fetch_nudge_candidates(&pool, 86400).await;
        assert!(
            candidates.iter().any(|c| c.user_id == sendable)
                && candidates.iter().any(|c| c.user_id == retrying),
            "both pending invitees must be nudge candidates before the sweep runs"
        );

        sweep_invite_nudge_once(None, &pool).await;

        // Candidate-state evidence: after one sweep the sendable invitee is
        // marked and therefore no longer selectable, while the retrying
        // (web-present) invitee is still selectable for a later tick.
        let candidates_after_first = crate::proposals::fetch_nudge_candidates(&pool, 86400).await;
        assert!(
            !candidates_after_first.iter().any(|c| c.user_id == sendable),
            "the sendable invitee must not be selectable again once nudged"
        );
        assert!(
            candidates_after_first.iter().any(|c| c.user_id == retrying),
            "the retrying invitee must remain selectable until its send succeeds"
        );

        sweep_invite_nudge_once(None, &pool).await;

        // Per-invitee marker state: the sendable invitee is nudged exactly once
        // (its own marker set), the retrying invitee stays unmarked for retry.
        let sendable_nudged: Option<time::PrimitiveDateTime> = sqlx::query_scalar(
            "SELECT nudged_at FROM game_proposal_players WHERE id = $1",
        )
        .bind(sendable_pp)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            sendable_nudged.is_some(),
            "the sendable invitee must be recorded as nudged via its own per-invitee marker"
        );

        let retrying_nudged: Option<time::PrimitiveDateTime> = sqlx::query_scalar(
            "SELECT nudged_at FROM game_proposal_players WHERE id = $1",
        )
        .bind(retrying_pp)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            retrying_nudged.is_none(),
            "the retrying (web-present) invitee must remain unmarked"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sweep_stops_on_shutdown() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;
        use tokio_util::sync::CancellationToken;

        let shutdown = CancellationToken::new();
        let runs = Arc::new(AtomicUsize::new(0));
        let runs_clone = runs.clone();
        let handle = spawn_sweep(
            "shutdown_sweep",
            Duration::from_millis(10),
            shutdown.clone(),
            move || {
                let runs = runs_clone.clone();
                async move {
                    runs.fetch_add(1, Ordering::SeqCst);
                }
            },
        );
        tokio::time::timeout(Duration::from_secs(600), async {
            while runs.load(Ordering::SeqCst) < 2 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("sweep did not run on its interval");
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(600), handle)
            .await
            .expect("sweep task did not exit after shutdown")
            .unwrap();
    }

    fn mock_player_render_response() -> brdgme_cmd::api::Response {
        use brdgme_cmd::api::{PlayerRender, Response};
        Response::PlayerRender {
            render: PlayerRender {
                player_state: "p0".to_string(),
                render: "board".to_string(),
                command_spec: None,
            },
        }
    }

    /// In-process game-service mock whose `PlayerRender` handler parks until the
    /// test releases it: `arrived` fires when the render request lands (the sweep
    /// is now inside the send window) and `release` lets it complete. Gives a
    /// deterministic rendezvous inside the (hoisted) render/send for R-18.
    async fn spawn_blocking_render_service(
        arrived: std::sync::Arc<tokio::sync::Notify>,
        release: std::sync::Arc<tokio::sync::Notify>,
    ) -> String {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::{Request, Response};
        use tokio::net::TcpListener;

        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| {
                let arrived = arrived.clone();
                let release = release.clone();
                async move {
                    match payload {
                        Request::PlayerRender { .. } => {
                            arrived.notify_one();
                            release.notified().await;
                            Json(mock_player_render_response())
                        }
                        _ => Json(Response::SystemError {
                            message: "unsupported in sweep test mock".to_string(),
                        }),
                    }
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    /// In-process game-service mock that counts `PlayerRender` requests, adds one
    /// `entered` permit per render (a render runs only after its sweep's claim TX
    /// has committed, so a permit proves that sweep holds no row lock and the row
    /// is still unmarked), then parks each on a shared barrier so concurrent
    /// sweeps can be held inside the (hoisted) send window simultaneously and
    /// released together.
    async fn spawn_barrier_render_service(
        renders: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        entered: std::sync::Arc<tokio::sync::Semaphore>,
        barrier: std::sync::Arc<tokio::sync::Barrier>,
    ) -> String {
        use axum::{Json, Router, routing::post};
        use brdgme_cmd::api::{Request, Response};
        use std::sync::atomic::Ordering;
        use tokio::net::TcpListener;

        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| {
                let renders = renders.clone();
                let entered = entered.clone();
                let barrier = barrier.clone();
                async move {
                    match payload {
                        Request::PlayerRender { .. } => {
                            renders.fetch_add(1, Ordering::SeqCst);
                            entered.add_permits(1);
                            barrier.wait().await;
                            Json(mock_player_render_response())
                        }
                        _ => Json(Response::SystemError {
                            message: "unsupported in sweep test mock".to_string(),
                        }),
                    }
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    /// Like `seed_reminder_game`, but points the game version at `uri` under a
    /// DNS-label version name (the game client rejects dotted names like `1.0.0`)
    /// and marks the recipient emailable and off the web, so the reminder send
    /// proceeds all the way to the game-service render.
    async fn seed_reminder_game_at(pool: &PgPool, uri: &str) -> (Uuid, Uuid, Uuid) {
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
             VALUES ($1, $2, $3, true, false) RETURNING id",
        )
        .bind(game_type_id)
        .bind(format!("reminder-mock-{}", Uuid::new_v4().simple()))
        .bind(uri)
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
            "INSERT INTO users
                 (name, pref_colors, reminder_emails_enabled, turn_emails_enabled, last_active_at)
             VALUES ($1, $2, true, true, NOW() - interval '11 minutes') RETURNING id",
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

    /// Best-effort concurrent flip of the recipient's turn, scoped to a short
    /// `lock_timeout` so it can never deadlock against a sweep still holding the
    /// `FOR UPDATE` claim lock across the send (the R-18 defect). Pre-hoist this
    /// times out on the held lock and the flip does not apply; post-hoist the lock
    /// is released during the send and the flip lands.
    async fn try_flip_turn(pool: &PgPool, gp_id: Uuid) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout = '250ms'").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_players SET is_turn = false WHERE id = $1")
            .bind(gp_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // R-18 / F-143 (WP-46 vs WP-79 reconciliation): once the game-service render
    // and Resend send are hoisted OUT of the claim transaction, the sweep must
    // re-check eligibility before marking. A concurrent player loses the turn
    // during the (hoisted) send window; the post-call re-check
    // (`turn_reminder_sent_at IS NULL AND is_turn = true`) must refuse the mark.
    // RED against the current code, which holds `FOR UPDATE` across the send and
    // marks unconditionally: the flip times out on the held lock and the row ends
    // marked.
    #[sqlx::test]
    async fn turn_reminder_recheck_rejects_concurrent_loss_of_turn(pool: PgPool) {
        use std::sync::Arc;
        use tokio::sync::Notify;

        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let uri = spawn_blocking_render_service(arrived.clone(), release.clone()).await;
        let (_game_id, _user_id, gp_id) = seed_reminder_game_at(&pool, &uri).await;
        let http = reqwest::Client::new();

        let sweep_pool = pool.clone();
        let sweep = tokio::spawn(async move { sweep_once(None, &sweep_pool, &http).await });

        // Deterministic hook: park until the sweep is inside the render/send.
        arrived.notified().await;

        // Concurrent modification while the send is in flight. Pre-hoist this
        // times out on the claim lock (ignored); post-hoist it lands.
        let _ = try_flip_turn(&pool, gp_id).await;

        release.notify_one();
        sweep.await.unwrap();

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            sent_at.is_none(),
            "post-call re-check must not mark a recipient who lost the turn during the send"
        );
    }

    // R-18 / F-143: the hoist trades claim-lock serialization for an at-most-once
    // DB mark plus an ACCEPTED rare duplicate send (CODING.md: "Mark work done
    // only after it succeeded ... a rare duplicate is the cheaper failure mode").
    //
    // Determinism: the second sweep is NOT spawned until the first is observed
    // inside the render (an `entered` semaphore permit). A render runs only after
    // its claim TX commits, so by then the first sweep holds no row lock and the
    // row is still unmarked; the second sweep's `FOR UPDATE SKIP LOCKED` claim is
    // therefore guaranteed to succeed rather than racing the first claim's
    // commit. A barrier then parks BOTH sweeps inside the (hoisted) send window,
    // proving they can both render (the tolerated duplicate) yet the conditional
    // mark (`WHERE turn_reminder_sent_at IS NULL`) lands at most once and stays
    // marked. RED against the current code: the claim lock is held across the
    // send, so the second sweep's SKIP LOCKED claim finds a locked row, never
    // renders, and its `entered` acquire times out.
    #[sqlx::test]
    async fn turn_reminder_concurrent_sweeps_mark_at_most_once(pool: PgPool) {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;
        use tokio::sync::{Barrier, Semaphore};

        let renders = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Semaphore::new(0));
        let barrier = Arc::new(Barrier::new(3));
        let uri =
            spawn_barrier_render_service(renders.clone(), entered.clone(), barrier.clone()).await;
        let (_game_id, _user_id, gp_id) = seed_reminder_game_at(&pool, &uri).await;
        let http = reqwest::Client::new();

        // Start the first sweep and wait until it is parked inside the (hoisted)
        // send window. A render runs only after `claim_tx.commit()`, so the permit
        // proves sweep 1 has released the claim row lock while the row is still
        // unmarked.
        let p1 = pool.clone();
        let h1 = http.clone();
        let s1 = tokio::spawn(async move { sweep_once(None, &p1, &h1).await });
        let first = tokio::time::timeout(Duration::from_secs(5), entered.acquire()).await;
        assert!(
            first.is_ok(),
            "the first sweep must reach the hoisted send window; pre-hoist it never renders outside the claim lock (the R-18 gap)"
        );
        first.unwrap().unwrap().forget();

        // Only now start the second sweep. Its `FOR UPDATE SKIP LOCKED` claim is
        // therefore guaranteed to see an unlocked, unmarked row and succeed: the
        // rendezvous no longer depends on the second claim racing the first
        // claim's commit. Both sweeps are now inside the send window at once.
        let p2 = pool.clone();
        let h2 = http.clone();
        let s2 = tokio::spawn(async move { sweep_once(None, &p2, &h2).await });
        let second = tokio::time::timeout(Duration::from_secs(5), entered.acquire()).await;
        assert!(
            second.is_ok(),
            "the second sweep must also reach the hoisted send window; pre-hoist SKIP LOCKED serializes the claim so only one arrives"
        );
        second.unwrap().unwrap().forget();

        // Both renders are in flight; release them together so each proceeds to its
        // conditional mark.
        barrier.wait().await;
        assert_eq!(
            renders.load(Ordering::SeqCst),
            2,
            "both sweeps rendered before either marked: the accepted rare duplicate send"
        );

        s1.await.unwrap();
        s2.await.unwrap();

        let sent_at: Option<time::PrimitiveDateTime> =
            sqlx::query_scalar("SELECT turn_reminder_sent_at FROM game_players WHERE id = $1")
                .bind(gp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(sent_at.is_some(), "the due candidate must be marked");

        // At-most-once is durable: a later sweep sees the mark, never renders again.
        let before = renders.load(Ordering::SeqCst);
        sweep_once(None, &pool, &http).await;
        assert_eq!(
            renders.load(Ordering::SeqCst),
            before,
            "a marked recipient must never be re-rendered or re-marked"
        );
    }
}
