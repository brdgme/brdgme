//! #24 game invites: pre-game proposals.
//! Spec: docs/superpowers/specs/2026-07-04-24-game-invites-design.md
//!
//! Bot column mapping (critical for the create_game step):
//! `game_proposal_players.bot_name` = `BotSlot.name` (the bot's display name),
//! and `game_proposal_players.bot_difficulty` = `BotSlot.bot_name` (the bot
//! type, e.g. "easy"/"medium"/"hard"). This mirrors `game_bots { name,
//! bot_name }`.

use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;
#[cfg(feature = "ssr")]
use sqlx::FromRow;

#[cfg_attr(feature = "ssr", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_version_id: Uuid,
    pub owner_user_id: Uuid,
    pub restarted_game_id: Option<Uuid>,
    pub status: String,
    pub started_game_id: Option<Uuid>,
    pub nudged_at: Option<PrimitiveDateTime>,
}

#[cfg_attr(feature = "ssr", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalPlayer {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub proposal_id: Uuid,
    pub position: i32,
    pub user_id: Option<Uuid>,
    pub bot_name: Option<String>,
    pub bot_difficulty: Option<String>,
    pub response: String,
    pub responded_at: Option<PrimitiveDateTime>,
    pub email_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalOutcome {
    pub proposal_id: Option<Uuid>,
    pub game_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondOutcome {
    pub accepted: bool,
    pub started: bool,
    pub game_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlotAction {
    Drop,
    ReplaceWithBot { name: String, bot_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotPolicy {
    pub player_id: Uuid,
    pub action: SlotAction,
}

#[cfg_attr(feature = "ssr", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalPlayerView {
    pub id: Uuid,
    pub position: i32,
    pub user_id: Option<Uuid>,
    pub bot_name: Option<String>,
    pub bot_difficulty: Option<String>,
    pub response: String,
    pub responded_at: Option<PrimitiveDateTime>,
    pub email_token: Option<String>,
    /// Resolved display name: the human's username, or the bot display name.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViewerRole {
    Owner,
    Invitee,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalView {
    pub proposal: Proposal,
    pub game_type_name: String,
    pub version_name: String,
    pub player_counts: Vec<i32>,
    pub players: Vec<ProposalPlayerView>,
    pub viewer_role: ViewerRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteSummary {
    pub proposal_id: Uuid,
    pub game_type_name: String,
    pub owner_name: String,
    pub player_count: i64,
}

#[cfg(feature = "ssr")]
pub trait InviteMailer: Send + Sync {
    fn send_invite(&self, proposal_id: Uuid, invitee_user_id: Uuid, email_token: Option<String>);
    fn notify_owner_decline(&self, proposal_id: Uuid, invitee_user_id: Uuid);
    fn notify_cancelled(&self, proposal_id: Uuid, accepted_user_ids: Vec<Uuid>);
    fn notify_started(&self, proposal_id: Uuid, game_id: Uuid, invitee_user_ids: Vec<Uuid>);
}

#[cfg(feature = "ssr")]
pub struct RealInviteMailer {
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
}

#[cfg(feature = "ssr")]
fn invite_browser_url(proposal_id: Uuid) -> String {
    let base = crate::config::public_base_url();
    format!("{base}/invites/{proposal_id}")
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, sqlx::FromRow)]
struct InviteRecipient {
    email: Option<String>,
    theme_slug: Option<String>,
    invite_emails_enabled: bool,
    name: String,
}

#[cfg(feature = "ssr")]
async fn fetch_invite_recipient(
    pool: &PgPool,
    user_id: Uuid,
) -> sqlx::Result<Option<InviteRecipient>> {
    sqlx::query_as::<_, InviteRecipient>(
        "SELECT ue.email, u.theme AS theme_slug, COALESCE(u.invite_emails_enabled, false) AS invite_emails_enabled, u.name \
         FROM users u \
         LEFT JOIN user_emails ue ON ue.user_id = u.id AND ue.is_primary AND ue.verified_at IS NOT NULL \
         WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Whether an automated invite email may go to this recipient: has a verified
/// primary address, has invite emails enabled, and is NOT suppressed by web
/// presence (the caller resolves presence via `suppress_for_web_presence`).
#[cfg(feature = "ssr")]
fn invite_recipient_should_send(recip: &InviteRecipient, suppressed_by_presence: bool) -> bool {
    recip.email.is_some() && recip.invite_emails_enabled && !suppressed_by_presence
}

#[cfg(feature = "ssr")]
async fn proposal_game_type_name(pool: &PgPool, proposal: &Proposal) -> String {
    let Ok(Some(gv)) = crate::db::find_game_version(pool, proposal.game_version_id).await else {
        return String::new();
    };
    find_game_type_name(pool, gv.game_type_id)
        .await
        .unwrap_or(None)
        .unwrap_or_default()
}

#[cfg(feature = "ssr")]
impl InviteMailer for RealInviteMailer {
    fn send_invite(&self, proposal_id: Uuid, invitee_user_id: Uuid, email_token: Option<String>) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Some(token) = email_token else { return };
            let Ok(Some(recip)) = fetch_invite_recipient(&pool, invitee_user_id).await else {
                return;
            };
            let suppressed =
                crate::email::outbound::suppress_for_web_presence(&pool, Some(invitee_user_id))
                    .await;
            if !invite_recipient_should_send(&recip, suppressed) {
                return;
            }
            let Some(email) = recip.email else { return };
            let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let owner_name = fetch_invite_recipient(&pool, proposal.owner_user_id)
                .await
                .ok()
                .flatten()
                .map(|r| r.name)
                .unwrap_or_default();
            let content = crate::email::render::EmailContent {
                subject: format!("{game_type_name} invite from {owner_name}"),
                header: Some(format!(
                    "{owner_name} invited you to play {game_type_name}."
                )),
                digest: None,
                board: None,
                you_can: Some(vec![
                    "Reply \"accept\" to join, or \"decline\" to pass.".into(),
                ]),
                browser_url: Some(invite_browser_url(proposal_id)),
                rules_url: Some(crate::email::notify::rules_url(proposal.game_version_id)),
                footer: Some("Reply to this email to respond, or unsubscribe anytime.".into()),
            };
            let palette = crate::email::render::palette_for_slug(recip.theme_slug.as_deref());
            let rendered = crate::email::render::render_game_email(
                &content,
                palette,
                &[],
                Some(&format!("proposal-{proposal_id}")),
                true,
                &format!("i-{token}@brdg.me"),
            );
            crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email).await;
        });
    }

    fn notify_owner_decline(&self, proposal_id: Uuid, invitee_user_id: Uuid) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else {
                return;
            };
            let Ok(Some(owner_recip)) = fetch_invite_recipient(&pool, proposal.owner_user_id).await
            else {
                return;
            };
            let Some(email) = owner_recip.email else {
                return;
            };
            let invitee_name = fetch_invite_recipient(&pool, invitee_user_id)
                .await
                .ok()
                .flatten()
                .map(|r| r.name)
                .unwrap_or_default();
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let content = crate::email::render::EmailContent {
                subject: format!("{game_type_name} invite"),
                header: Some(format!("{invitee_name} declined your invite.")),
                digest: None,
                board: None,
                you_can: None,
                browser_url: Some(invite_browser_url(proposal_id)),
                rules_url: Some(crate::email::notify::rules_url(proposal.game_version_id)),
                footer: Some("Reply to this email to respond, or unsubscribe anytime.".into()),
            };
            let palette = crate::email::render::palette_for_slug(owner_recip.theme_slug.as_deref());
            let rendered = crate::email::render::render_game_email(
                &content,
                palette,
                &[],
                Some(&format!("proposal-{proposal_id}")),
                false,
                &format!("i-{proposal_id}@brdg.me"),
            );
            crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email).await;
        });
    }

    fn notify_cancelled(&self, proposal_id: Uuid, accepted_user_ids: Vec<Uuid>) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            for user_id in accepted_user_ids {
                let Ok(Some(recip)) = fetch_invite_recipient(&pool, user_id).await else {
                    continue;
                };
                let suppressed =
                    crate::email::outbound::suppress_for_web_presence(&pool, Some(user_id)).await;
                if !invite_recipient_should_send(&recip, suppressed) {
                    continue;
                }
                let Some(email) = recip.email else { continue };
                let content = crate::email::render::EmailContent {
                    subject: format!("{game_type_name} invite"),
                    header: Some("The game invite was cancelled.".into()),
                    digest: None,
                    board: None,
                    you_can: None,
                    browser_url: Some(invite_browser_url(proposal_id)),
                    rules_url: Some(crate::email::notify::rules_url(proposal.game_version_id)),
                    footer: Some("Reply to this email to respond, or unsubscribe anytime.".into()),
                };
                let palette = crate::email::render::palette_for_slug(recip.theme_slug.as_deref());
                let rendered = crate::email::render::render_game_email(
                    &content,
                    palette,
                    &[],
                    Some(&format!("proposal-{proposal_id}")),
                    false,
                    &format!("i-{proposal_id}@brdg.me"),
                );
                crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email)
                    .await;
            }
        });
    }

    fn notify_started(&self, proposal_id: Uuid, game_id: Uuid, invitee_user_ids: Vec<Uuid>) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let base = crate::config::public_base_url();
            let game_url = format!("{base}/games/{game_id}");
            for user_id in invitee_user_ids {
                let Ok(Some(recip)) = fetch_invite_recipient(&pool, user_id).await else {
                    continue;
                };
                let suppressed =
                    crate::email::outbound::suppress_for_web_presence(&pool, Some(user_id)).await;
                if !invite_recipient_should_send(&recip, suppressed) {
                    continue;
                }
                let Some(email) = recip.email else { continue };
                let content = crate::email::render::EmailContent {
                    subject: format!("{game_type_name} invite"),
                    header: Some("The game has started!".into()),
                    digest: None,
                    board: None,
                    you_can: None,
                    browser_url: Some(game_url.clone()),
                    rules_url: Some(crate::email::notify::rules_url(proposal.game_version_id)),
                    footer: Some("Reply to this email to respond, or unsubscribe anytime.".into()),
                };
                let palette = crate::email::render::palette_for_slug(recip.theme_slug.as_deref());
                let rendered = crate::email::render::render_game_email(
                    &content,
                    palette,
                    &[],
                    Some(&format!("proposal-{proposal_id}")),
                    false,
                    &format!("i-{proposal_id}@brdg.me"),
                );
                crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email)
                    .await;
            }
        });
    }
}

#[cfg(feature = "ssr")]
pub(crate) fn mailer() -> RealInviteMailer {
    RealInviteMailer {
        pool: expect_context::<PgPool>(),
        resend: expect_context::<Option<resend_rs::Resend>>(),
    }
}

#[cfg(feature = "ssr")]
pub(crate) fn mailer_from(pool: PgPool, resend: Option<resend_rs::Resend>) -> RealInviteMailer {
    RealInviteMailer { pool, resend }
}

#[cfg(feature = "ssr")]
use sqlx::PgPool;

#[cfg(feature = "ssr")]
pub async fn find_proposal(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Proposal>> {
    sqlx::query_as::<_, Proposal>(
        "SELECT id, created_at, updated_at, game_version_id, owner_user_id, restarted_game_id, status, started_game_id, nudged_at FROM game_proposals WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn find_proposal_players(
    pool: &PgPool,
    proposal_id: Uuid,
) -> sqlx::Result<Vec<ProposalPlayer>> {
    sqlx::query_as::<_, ProposalPlayer>(
        "SELECT id, created_at, updated_at, proposal_id, \"position\", user_id, bot_name, bot_difficulty, response, responded_at, email_token FROM game_proposal_players WHERE proposal_id = $1 ORDER BY \"position\"",
    )
    .bind(proposal_id)
    .fetch_all(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn find_proposal_roster(
    pool: &PgPool,
    proposal_id: Uuid,
) -> sqlx::Result<Vec<ProposalPlayerView>> {
    sqlx::query_as::<_, ProposalPlayerView>(
        "SELECT pp.id, pp.\"position\", pp.user_id, pp.bot_name, pp.bot_difficulty, pp.response, \
         pp.responded_at, pp.email_token, \
         COALESCE(u.name, pp.bot_name, 'Bot') AS name \
         FROM game_proposal_players pp \
         LEFT JOIN users u ON u.id = pp.user_id \
         WHERE pp.proposal_id = $1 \
         ORDER BY pp.\"position\"",
    )
    .bind(proposal_id)
    .fetch_all(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn find_game_type_name(
    pool: &PgPool,
    game_type_id: Uuid,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT name FROM game_types WHERE id = $1")
        .bind(game_type_id)
        .fetch_optional(pool)
        .await
}

#[cfg(feature = "ssr")]
pub async fn insert_proposal(
    tx: &mut sqlx::PgConnection,
    game_version_id: Uuid,
    owner_user_id: Uuid,
    restarted_game_id: Option<Uuid>,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO game_proposals (game_version_id, owner_user_id, restarted_game_id) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind(game_version_id)
    .bind(owner_user_id)
    .bind(restarted_game_id)
    .fetch_one(&mut *tx)
    .await
}

#[cfg(feature = "ssr")]
#[allow(clippy::too_many_arguments)]
pub async fn insert_proposal_player(
    tx: &mut sqlx::PgConnection,
    proposal_id: Uuid,
    position: i32,
    user_id: Option<Uuid>,
    bot_name: Option<String>,
    bot_difficulty: Option<String>,
    response: &str,
    email_token: Option<String>,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO game_proposal_players (proposal_id, \"position\", user_id, bot_name, bot_difficulty, response, email_token) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(proposal_id)
    .bind(position)
    .bind(user_id)
    .bind(bot_name)
    .bind(bot_difficulty)
    .bind(response)
    .bind(email_token)
    .fetch_one(&mut *tx)
    .await
}

#[cfg(feature = "ssr")]
pub async fn update_proposal_status(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    status: &str,
    started_game_id: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE game_proposals SET status = $1, started_game_id = $2, updated_at = (now() AT TIME ZONE 'utc') WHERE id = $3",
    )
    .bind(status)
    .bind(started_game_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn update_proposal_player_response(
    tx: &mut sqlx::PgConnection,
    player_id: Uuid,
    response: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE game_proposal_players SET response = $1, responded_at = (now() AT TIME ZONE 'utc'), updated_at = (now() AT TIME ZONE 'utc') WHERE id = $2",
    )
    .bind(response)
    .bind(player_id)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn delete_proposal_player(
    tx: &mut sqlx::PgConnection,
    player_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM game_proposal_players WHERE id = $1")
        .bind(player_id)
        .execute(&mut *tx)
        .await
        .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn convert_proposal_player_to_bot(
    tx: &mut sqlx::PgConnection,
    player_id: Uuid,
    bot_name: &str,
    bot_difficulty: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE game_proposal_players SET user_id = NULL, bot_name = $1, bot_difficulty = $2, response = 'accepted', responded_at = (now() AT TIME ZONE 'utc'), updated_at = (now() AT TIME ZONE 'utc') WHERE id = $3",
    )
    .bind(bot_name)
    .bind(bot_difficulty)
    .bind(player_id)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn find_proposal_player_by_email_token(
    pool: &PgPool,
    token: &str,
) -> sqlx::Result<Option<ProposalPlayer>> {
    sqlx::query_as::<_, ProposalPlayer>(
        "SELECT id, created_at, updated_at, proposal_id, \"position\", user_id, bot_name, bot_difficulty, response, responded_at, email_token FROM game_proposal_players WHERE email_token = $1",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn count_pending_human_invitees(pool: &PgPool, proposal_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_proposal_players WHERE proposal_id = $1 AND response = 'pending' AND user_id IS NOT NULL",
    )
    .bind(proposal_id)
    .fetch_one(pool)
    .await
}

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
pub struct NudgeCandidate {
    pub proposal_id: Uuid,
    pub user_id: Uuid,
    pub email_token: Option<String>,
}

#[cfg(feature = "ssr")]
pub async fn fetch_nudge_candidates(pool: &PgPool, threshold_secs: i64) -> Vec<NudgeCandidate> {
    let rows = sqlx::query_as::<_, NudgeCandidate>(
        "SELECT gp.id AS proposal_id, pp.user_id, pp.email_token \
         FROM game_proposals gp \
         JOIN game_proposal_players pp ON pp.proposal_id = gp.id \
         WHERE gp.status = 'open' AND gp.nudged_at IS NULL \
           AND gp.created_at < NOW() - ($1 || ' seconds')::interval \
           AND pp.response = 'pending' AND pp.user_id IS NOT NULL",
    )
    .bind(threshold_secs.to_string())
    .fetch_all(pool)
    .await;
    match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("invite_nudge: candidate query failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn mark_proposal_nudged(pool: &PgPool, proposal_id: Uuid) {
    if let Err(e) =
        sqlx::query("UPDATE game_proposals SET nudged_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(proposal_id)
            .execute(pool)
            .await
    {
        tracing::error!("invite_nudge: mark failed for {}: {}", proposal_id, e);
    }
}

#[cfg(feature = "ssr")]
pub async fn fetch_expiry_candidates(pool: &PgPool, threshold_secs: i64) -> Vec<Uuid> {
    let rows = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM game_proposals WHERE status = 'open' AND created_at < NOW() - ($1 || ' seconds')::interval",
    )
    .bind(threshold_secs.to_string())
    .fetch_all(pool)
    .await;
    match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("invite_expiry: candidate query failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn cancel_proposal_for_expiry(
    pool: &PgPool,
    proposal_id: Uuid,
) -> Option<(Uuid, Vec<Uuid>)> {
    let result = sqlx::query(
        "UPDATE game_proposals SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND status = 'open'",
    )
    .bind(proposal_id)
    .execute(pool)
    .await;
    match result {
        Ok(r) if r.rows_affected() == 0 => return None,
        Err(e) => {
            tracing::error!("invite_expiry: cancel failed for {}: {}", proposal_id, e);
            return None;
        }
        _ => {}
    }
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT owner_user_id FROM game_proposals WHERE id = $1")
            .bind(proposal_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    let owner = owner?;
    let accepted: Vec<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM game_proposal_players WHERE proposal_id = $1 AND response = 'accepted' AND user_id IS NOT NULL",
    )
    .bind(proposal_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let accepted_ids: Vec<Uuid> = accepted.into_iter().filter(|id| *id != owner).collect();
    Some((owner, accepted_ids))
}

#[cfg(feature = "ssr")]
pub async fn fetch_auto_decline_candidates(
    pool: &PgPool,
    threshold_secs: i64,
) -> Vec<(Uuid, Uuid)> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT pp.id, pp.proposal_id \
         FROM game_proposal_players pp \
         JOIN game_proposals gp ON gp.id = pp.proposal_id \
         WHERE gp.status = 'open' \
           AND pp.response = 'pending' \
           AND pp.user_id IS NOT NULL \
           AND gp.created_at < NOW() - ($1 || ' seconds')::interval",
    )
    .bind(threshold_secs.to_string())
    .fetch_all(pool)
    .await;
    match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("invite_auto_decline: candidate query failed: {}", e);
            Vec::new()
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn auto_decline_proposal_player(pool: &PgPool, player_id: Uuid) {
    if let Err(e) = sqlx::query(
        "UPDATE game_proposal_players SET response = 'declined', responded_at = NOW(), updated_at = NOW() WHERE id = $1 AND response = 'pending'",
    )
    .bind(player_id)
    .execute(pool)
    .await
    {
        tracing::error!("invite_auto_decline: decline failed for {}: {}", player_id, e);
    }
}

#[cfg(feature = "ssr")]
pub async fn lock_proposal_for_update(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
) -> sqlx::Result<Option<Proposal>> {
    sqlx::query_as::<_, Proposal>(
        "SELECT id, created_at, updated_at, game_version_id, owner_user_id, restarted_game_id, status, started_game_id, nudged_at FROM game_proposals WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
}

#[cfg(feature = "ssr")]
pub async fn find_proposal_players_tx(
    tx: &mut sqlx::PgConnection,
    proposal_id: Uuid,
) -> sqlx::Result<Vec<ProposalPlayer>> {
    sqlx::query_as::<_, ProposalPlayer>(
        "SELECT id, created_at, updated_at, proposal_id, \"position\", user_id, bot_name, bot_difficulty, response, responded_at, email_token FROM game_proposal_players WHERE proposal_id = $1 ORDER BY \"position\"",
    )
    .bind(proposal_id)
    .fetch_all(&mut *tx)
    .await
}

#[cfg(feature = "ssr")]
pub async fn count_pending_human_invitees_tx(
    tx: &mut sqlx::PgConnection,
    proposal_id: Uuid,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_proposal_players WHERE proposal_id = $1 AND response = 'pending' AND user_id IS NOT NULL",
    )
    .bind(proposal_id)
    .fetch_one(&mut *tx)
    .await
}

#[cfg(feature = "ssr")]
async fn find_or_create_user_by_email_tx(
    tx: &mut sqlx::PgConnection,
    email: &str,
) -> Result<Uuid, ServerFnError> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN user_emails ue ON u.id = ue.user_id WHERE ue.email = $1",
    )
    .bind(email)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("create_proposal: resolve email"))?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_user_id = Uuid::new_v4();
    let username = crate::db::generate_unique_username(&mut *tx)
        .await
        .map_err(internal("create_proposal: gen username"))?;
    sqlx::query("INSERT INTO users (id, name, pref_colors) VALUES ($1,$2,$3)")
        .bind(new_user_id)
        .bind(&username)
        .bind(Vec::<String>::new())
        .execute(&mut *tx)
        .await
        .map_err(internal("create_proposal: resolve email"))?;
    sqlx::query(
        "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1,$2,true,NOW())",
    )
    .bind(new_user_id)
    .bind(email)
    .execute(&mut *tx)
    .await
    .map_err(internal("create_proposal: resolve email"))?;
    Ok(new_user_id)
}

#[cfg(feature = "ssr")]
pub async fn find_pending_invites_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> sqlx::Result<Vec<InviteSummary>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, i64)>(
        "SELECT gp.id, gt.name, u.name, (SELECT COUNT(*) FROM game_proposal_players x WHERE x.proposal_id = gp.id) \
         FROM game_proposal_players pp \
         JOIN game_proposals gp ON gp.id = pp.proposal_id AND gp.status = 'open' \
         JOIN game_versions gv ON gv.id = gp.game_version_id \
         JOIN game_types gt ON gt.id = gv.game_type_id \
         JOIN users u ON u.id = gp.owner_user_id \
         WHERE pp.user_id = $1 AND pp.response = 'pending' \
         ORDER BY gp.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(proposal_id, game_type_name, owner_name, player_count)| InviteSummary {
                proposal_id,
                game_type_name,
                owner_name,
                player_count,
            },
        )
        .collect())
}

/// Creates the game for a proposal from its ACCEPTED roster, links any
/// restarted game, and flips the proposal to `started`. Runs inside the
/// caller's transaction; the caller commits and then broadcasts/notifies.
#[cfg(feature = "ssr")]
pub(crate) async fn start_proposal_tx(
    tx: &mut sqlx::PgConnection,
    http_client: &reqwest::Client,
    proposal: &Proposal,
    players: &[ProposalPlayer],
    game_version: &crate::models::game::GameVersion,
) -> Result<Uuid, ServerFnError> {
    use crate::game::server_fns::{BotSlot, CreateGameSeed, create_game_from_service};

    let accepted: Vec<&ProposalPlayer> = players
        .iter()
        .filter(|p| p.response == "accepted")
        .collect();
    let creator_id = proposal.owner_user_id;
    let opponent_ids: Vec<Uuid> = accepted
        .iter()
        .filter_map(|p| p.user_id)
        .filter(|id| *id != creator_id)
        .collect();
    let bot_slots: Vec<BotSlot> = accepted
        .iter()
        .filter(|p| p.user_id.is_none())
        .map(|p| BotSlot {
            name: p.bot_name.clone().unwrap_or_default(),
            bot_name: p.bot_difficulty.clone().unwrap_or_default(),
        })
        .collect();
    let player_count = accepted.len();

    let game = create_game_from_service(
        &mut *tx,
        http_client,
        game_version,
        CreateGameSeed {
            player_count,
            creator_id,
            opponent_ids: &opponent_ids,
            opponent_emails: &[],
            bot_slots: &bot_slots,
            all_accepted: true,
        },
    )
    .await?;

    if let Some(old) = proposal.restarted_game_id {
        sqlx::query("UPDATE games SET restarted_game_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(game.id)
            .bind(old)
            .execute(&mut *tx)
            .await
            .map_err(internal("start_proposal: link restarted game"))?;
    }

    update_proposal_status(&mut *tx, proposal.id, "started", Some(game.id))
        .await
        .map_err(internal("start_proposal: status"))?;

    Ok(game.id)
}

/// Creates an open game-invite proposal (owner and bots accepted, humans
/// pending). With no human invitees (solo-vs-bots) it skips the proposal and
/// creates the game directly.
#[server(CreateProposal, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all))]
pub async fn create_proposal(
    game_version_id: Uuid,
    opponent_ids: Option<Vec<Uuid>>,
    opponent_emails: Option<Vec<String>>,
    bot_slots: Option<Vec<crate::game::server_fns::BotSlot>>,
) -> Result<ProposalOutcome, ServerFnError> {
    use crate::game::server_fns::{CreateGameSeed, create_game_from_service};
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = crate::friends::require_user().await?;

    let opponent_ids = opponent_ids.unwrap_or_default();
    let opponent_emails = opponent_emails.unwrap_or_default();
    let bot_slots = bot_slots.unwrap_or_default();

    let player_count = 1 + opponent_ids.len() + opponent_emails.len() + bot_slots.len();

    let game_version = crate::db::find_game_version(&pool, game_version_id)
        .await
        .map_err(internal("create_proposal: find game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let player_counts = crate::db::find_game_type_player_counts(&pool, game_version_id)
        .await
        .map_err(internal("create_proposal: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
    if let Some(msg) = crate::game::server_fns::roster_error(&player_counts, player_count) {
        return Err(ServerFnError::new(msg));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("create_proposal: begin transaction"))?;

    let violations =
        crate::db::check_invite_policy_tx(&mut tx, user.id, &opponent_ids, &opponent_emails)
            .await
            .map_err(internal("create_proposal: check invite policy"))?;
    if let Some(msg) = violations.into_iter().next() {
        return Err(ServerFnError::new(msg));
    }

    let mut human_invitees: Vec<Uuid> = opponent_ids.clone();
    for email in &opponent_emails {
        human_invitees.push(find_or_create_user_by_email_tx(&mut tx, email).await?);
    }

    let mut all = vec![user.id];
    all.extend(&human_invitees);
    all.sort();
    let before = all.len();
    all.dedup();
    if all.len() != before {
        return Err(ServerFnError::new(
            "Please ensure each player in the game is unique",
        ));
    }

    if human_invitees.is_empty() {
        let game = create_game_from_service(
            &mut tx,
            &http_client,
            &game_version,
            CreateGameSeed {
                player_count,
                creator_id: user.id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &bot_slots,
                all_accepted: false,
            },
        )
        .await?;
        tx.commit()
            .await
            .map_err(internal("create_proposal: commit transaction"))?;
        crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game.id).await;
        return Ok(ProposalOutcome {
            proposal_id: None,
            game_id: Some(game.id),
        });
    }

    let proposal_id = insert_proposal(&mut tx, game_version_id, user.id, None)
        .await
        .map_err(internal("create_proposal: insert proposal"))?;

    let mut position = 0;
    insert_proposal_player(
        &mut tx,
        proposal_id,
        position,
        Some(user.id),
        None,
        None,
        "accepted",
        None,
    )
    .await
    .map_err(internal("create_proposal: insert owner"))?;
    position += 1;

    let mut invite_tokens: Vec<(Uuid, String)> = Vec::new();
    for uid in &human_invitees {
        let token = Uuid::new_v4().simple().to_string();
        insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            Some(*uid),
            None,
            None,
            "pending",
            Some(token.clone()),
        )
        .await
        .map_err(internal("create_proposal: insert invitee"))?;
        invite_tokens.push((*uid, token));
        position += 1;
    }

    for bot in &bot_slots {
        insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            None,
            Some(bot.name.clone()),
            Some(bot.bot_name.clone()),
            "accepted",
            None,
        )
        .await
        .map_err(internal("create_proposal: insert bot"))?;
        position += 1;
    }

    tx.commit()
        .await
        .map_err(internal("create_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;
    for (uid, token) in &invite_tokens {
        mailer().send_invite(proposal_id, *uid, Some(token.clone()));
    }

    Ok(ProposalOutcome {
        proposal_id: Some(proposal_id),
        game_id: None,
    })
}

/// Records an invitee's accept/decline on an open proposal. Decline is
/// terminal. On accept, when no human invitees remain pending the game starts
/// automatically.
#[server(RespondProposal, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn respond_proposal(
    proposal_id: Uuid,
    accept: bool,
) -> Result<RespondOutcome, ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = crate::friends::require_user().await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("respond_proposal: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("respond_proposal: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;

    if proposal.status != "open" {
        return Err(ServerFnError::new("This invite is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("respond_proposal: players"))?;

    let me = players
        .iter()
        .find(|p| p.user_id == Some(user.id))
        .ok_or_else(|| ServerFnError::new("You are not an invitee of this proposal."))?;

    if me.response != "pending" {
        return Err(ServerFnError::new(
            "That invite has already been responded to.",
        ));
    }

    let response = if accept { "accepted" } else { "declined" };
    update_proposal_player_response(&mut tx, me.id, response)
        .await
        .map_err(internal("respond_proposal: update"))?;

    let mut started_game_id: Option<Uuid> = None;
    let mut roster: Vec<ProposalPlayer> = Vec::new();
    if accept {
        let pending = count_pending_human_invitees_tx(&mut tx, proposal_id)
            .await
            .map_err(internal("respond_proposal: count"))?;
        if pending == 0 {
            let game_version = crate::db::find_game_version(&pool, proposal.game_version_id)
                .await
                .map_err(internal("respond_proposal: game version"))?
                .ok_or_else(|| ServerFnError::new("Game version not found"))?;
            roster = find_proposal_players_tx(&mut tx, proposal_id)
                .await
                .map_err(internal("respond_proposal: roster"))?;
            let gid =
                start_proposal_tx(&mut tx, &http_client, &proposal, &roster, &game_version).await?;
            started_game_id = Some(gid);
        }
    }

    tx.commit()
        .await
        .map_err(internal("respond_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    if let Some(gid) = started_game_id {
        crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, gid).await;
        let invitee_ids: Vec<Uuid> = roster
            .iter()
            .filter(|p| p.response == "accepted")
            .filter_map(|p| p.user_id)
            .filter(|id| *id != proposal.owner_user_id)
            .collect();
        mailer().notify_started(proposal_id, gid, invitee_ids);
    } else if !accept {
        mailer().notify_owner_decline(proposal_id, user.id);
    }

    Ok(RespondOutcome {
        accepted: accept,
        started: started_game_id.is_some(),
        game_id: started_game_id,
    })
}

/// Number of proposal players that could be in the started game (accepted +
/// pending; declined slots are excluded).
#[cfg(feature = "ssr")]
fn prospective_count(players: &[ProposalPlayer]) -> usize {
    players.iter().filter(|p| p.response != "declined").count()
}

/// Owner-only: resolve every pending slot per the supplied policies (drop or
/// replace-with-bot), validate the final roster, and start the game early.
#[server(StartProposalEarly, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn start_proposal_early(
    proposal_id: Uuid,
    policies: Vec<SlotPolicy>,
) -> Result<Uuid, ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("start_proposal_early: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new(
            "Only the owner can start this proposal.",
        ));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let game_version = crate::db::find_game_version(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal_early: game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;
    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal_early: player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("start_proposal_early: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("start_proposal_early: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new(
            "Only the owner can start this proposal.",
        ));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("start_proposal_early: players"))?;

    for policy in &policies {
        let target = players
            .iter()
            .find(|p| p.id == policy.player_id)
            .ok_or_else(|| ServerFnError::new("Invalid slot policy."))?;
        if !(target.user_id.is_some() && target.response == "pending") {
            return Err(ServerFnError::new("Invalid slot policy."));
        }
        match &policy.action {
            SlotAction::Drop => {
                delete_proposal_player(&mut tx, policy.player_id)
                    .await
                    .map_err(internal("start_proposal_early: drop"))?;
            }
            SlotAction::ReplaceWithBot { name, bot_name } => {
                convert_proposal_player_to_bot(&mut tx, policy.player_id, name, bot_name)
                    .await
                    .map_err(internal("start_proposal_early: replace"))?;
            }
        }
    }

    let roster = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("start_proposal_early: roster"))?;
    if roster.iter().any(|p| p.response == "pending") {
        return Err(ServerFnError::new(
            "Resolve every pending invite before starting early.",
        ));
    }
    if let Some(msg) =
        crate::game::server_fns::roster_error(&player_counts, prospective_count(&roster))
    {
        return Err(ServerFnError::new(msg));
    }

    let gid = start_proposal_tx(&mut tx, &http_client, &proposal, &roster, &game_version).await?;

    tx.commit()
        .await
        .map_err(internal("start_proposal_early: commit transaction"))?;

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, gid).await;
    broadcaster.broadcast_proposal_update(proposal_id).await;
    let invitee_ids: Vec<Uuid> = roster
        .iter()
        .filter(|p| p.response == "accepted")
        .filter_map(|p| p.user_id)
        .filter(|id| *id != proposal.owner_user_id)
        .collect();
    mailer().notify_started(proposal_id, gid, invitee_ids);

    Ok(gid)
}

/// Owner-only: cancel an open proposal and notify accepted invitees.
#[server(CancelProposal, "/api")]
pub async fn cancel_proposal(proposal_id: Uuid) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("cancel_proposal: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new(
            "Only the owner can cancel this proposal.",
        ));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players(&pool, proposal_id)
        .await
        .map_err(internal("cancel_proposal: players"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("cancel_proposal: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("cancel_proposal: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new(
            "Only the owner can cancel this proposal.",
        ));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    update_proposal_status(&mut tx, proposal_id, "cancelled", None)
        .await
        .map_err(internal("cancel_proposal: status"))?;

    tx.commit()
        .await
        .map_err(internal("cancel_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;
    let invitee_ids: Vec<Uuid> = players
        .iter()
        .filter(|p| p.response == "accepted")
        .filter_map(|p| p.user_id)
        .filter(|id| *id != proposal.owner_user_id)
        .collect();
    mailer().notify_cancelled(proposal_id, invitee_ids);

    Ok(())
}

/// Owner-only: replace a declined/pending human slot with a bot, validating the
/// resulting roster size.
#[server(ReplaceProposalSlot, "/api")]
pub async fn replace_proposal_slot(
    proposal_id: Uuid,
    player_id: Uuid,
    bot_name: String,
    bot_difficulty: String,
) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("replace_proposal_slot: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("replace_proposal_slot: player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("replace_proposal_slot: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("replace_proposal_slot: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("replace_proposal_slot: players"))?;
    let target = players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| ServerFnError::new("That slot can't be replaced."))?;
    if !(target.user_id.is_some()
        && (target.response == "declined" || target.response == "pending"))
    {
        return Err(ServerFnError::new("That slot can't be replaced."));
    }

    convert_proposal_player_to_bot(&mut tx, player_id, &bot_name, &bot_difficulty)
        .await
        .map_err(internal("replace_proposal_slot: convert"))?;

    let roster = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("replace_proposal_slot: roster"))?;
    if let Some(msg) =
        crate::game::server_fns::roster_error(&player_counts, prospective_count(&roster))
    {
        return Err(ServerFnError::new(msg));
    }

    tx.commit()
        .await
        .map_err(internal("replace_proposal_slot: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    Ok(())
}

/// Owner-only: remove a declined/pending slot, validating the resulting roster
/// size.
#[server(RemoveProposalSlot, "/api")]
pub async fn remove_proposal_slot(proposal_id: Uuid, player_id: Uuid) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("remove_proposal_slot: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("remove_proposal_slot: player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("remove_proposal_slot: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("remove_proposal_slot: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("remove_proposal_slot: players"))?;
    let target = players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| ServerFnError::new("That slot can't be removed."))?;
    if !(target.response == "declined" || target.response == "pending") {
        return Err(ServerFnError::new("That slot can't be removed."));
    }

    delete_proposal_player(&mut tx, player_id)
        .await
        .map_err(internal("remove_proposal_slot: delete"))?;

    let roster = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("remove_proposal_slot: roster"))?;
    if let Some(msg) =
        crate::game::server_fns::roster_error(&player_counts, prospective_count(&roster))
    {
        return Err(ServerFnError::new(msg));
    }

    tx.commit()
        .await
        .map_err(internal("remove_proposal_slot: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    Ok(())
}

/// Loads a proposal's full view: roster with resolved names, game-type/version
/// names, valid player counts, and the caller's role.
#[server(GetProposal, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn get_proposal(proposal_id: Uuid) -> Result<ProposalView, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("get_proposal: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;

    let game_version = crate::db::find_game_version(&pool, proposal.game_version_id)
        .await
        .map_err(internal("get_proposal: game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;
    let version_name = game_version.name.clone();

    let game_type_name = find_game_type_name(&pool, game_version.game_type_id)
        .await
        .map_err(internal("get_proposal: game type name"))?
        .unwrap_or_default();

    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("get_proposal: player counts"))?
        .unwrap_or_default();

    let players = find_proposal_roster(&pool, proposal_id)
        .await
        .map_err(internal("get_proposal: roster"))?;

    let viewer_role = if user.id == proposal.owner_user_id {
        ViewerRole::Owner
    } else if players.iter().any(|p| p.user_id == Some(user.id)) {
        ViewerRole::Invitee
    } else {
        ViewerRole::Other
    };

    Ok(ProposalView {
        proposal,
        game_type_name,
        version_name,
        player_counts,
        players,
        viewer_role,
    })
}

/// Lists the caller's pending invites for the dashboard section.
#[server(GetPendingInvites, "/api")]
pub async fn get_pending_invites() -> Result<Vec<InviteSummary>, ServerFnError> {
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let user = crate::friends::require_user().await?;

    find_pending_invites_for_user(&pool, user.id)
        .await
        .map_err(internal("get_pending_invites: find"))
}

fn track_proposal_seq(
    prev: Option<(Option<Uuid>, Option<u64>)>,
    current_id: Option<Uuid>,
    update: Option<(Uuid, u64)>,
) -> (Option<Uuid>, Option<u64>) {
    let prev_seq = match prev {
        Some((prev_id, seq)) if prev_id == current_id => seq,
        _ => None,
    };
    let seq = match update {
        Some((id, seq)) if Some(id) == current_id => Some(seq),
        _ => prev_seq,
    };
    (current_id, seq)
}

#[component]
pub fn InvitePage() -> impl IntoView {
    let params = use_params_map();
    let proposal_id = move || {
        params
            .get()
            .get("id")
            .as_deref()
            .and_then(|id| Uuid::from_str(id).ok())
    };

    let proposal_update = expect_context::<crate::websocket_client::ProposalUpdate>().0;

    let seq_for_this_proposal = Memo::new(move |prev: Option<&(Option<Uuid>, Option<u64>)>| {
        track_proposal_seq(prev.copied(), proposal_id(), proposal_update.get())
    });

    let proposal_data: LocalResource<Result<ProposalView, ServerFnError>> =
        LocalResource::new(move || async move {
            let _ = seq_for_this_proposal.get();
            match proposal_id() {
                Some(id) => get_proposal(id).await,
                None => Err(ServerFnError::new("Invalid invite ID")),
            }
        });

    let respond_action = ServerAction::<RespondProposal>::new();
    let cancel_action = ServerAction::<CancelProposal>::new();
    let start_early_action = ServerAction::<StartProposalEarly>::new();
    let replace_action = ServerAction::<ReplaceProposalSlot>::new();
    let remove_action = ServerAction::<RemoveProposalSlot>::new();

    let navigate = use_navigate();

    let nav1 = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(outcome)) = respond_action.value().get() {
            if let Some(gid) = outcome.game_id {
                nav1(&format!("/games/{}", gid), NavigateOptions::default());
            } else {
                crate::websocket_client::bump_proposal_update(
                    proposal_update,
                    proposal_id().unwrap_or_default(),
                );
            }
        }
    });

    let nav2 = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(gid)) = start_early_action.value().get() {
            nav2(&format!("/games/{}", gid), NavigateOptions::default());
        }
    });

    let nav3 = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(())) = cancel_action.value().get() {
            nav3("/dashboard", NavigateOptions::default());
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = replace_action.value().get()
            && let Some(pid) = proposal_id()
        {
            crate::websocket_client::bump_proposal_update(proposal_update, pid);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(())) = remove_action.value().get()
            && let Some(pid) = proposal_id()
        {
            crate::websocket_client::bump_proposal_update(proposal_update, pid);
        }
    });

    view! {
        <crate::components::MainLayout>
            <div class="content-page">
                {move || match proposal_data.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <div class="error">"Error: " {e.to_string()}</div> }.into_any(),
                    Some(Ok(pv)) => {
                        view! { <ProposalDetail
                            pv=pv
                            respond_action=respond_action
                            cancel_action=cancel_action
                            start_early_action=start_early_action
                            replace_action=replace_action
                            remove_action=remove_action
                        /> }.into_any()
                    }
                }}
            </div>
        </crate::components::MainLayout>
    }
}

#[component]
fn ProposalDetail(
    pv: ProposalView,
    respond_action: ServerAction<RespondProposal>,
    cancel_action: ServerAction<CancelProposal>,
    start_early_action: ServerAction<StartProposalEarly>,
    replace_action: ServerAction<ReplaceProposalSlot>,
    remove_action: ServerAction<RemoveProposalSlot>,
) -> impl IntoView {
    let is_open = pv.proposal.status == "open";
    let viewer_role = pv.viewer_role.clone();
    let proposal_id = pv.proposal.id;
    let game_type_name = pv.game_type_name.clone();
    let version_name = pv.version_name.clone();

    let is_owner = viewer_role == ViewerRole::Owner;
    let is_invitee = viewer_role == ViewerRole::Invitee;

    let my_pending = pv
        .players
        .iter()
        .any(|p| p.user_id.is_some() && p.response == "pending")
        && is_invitee;

    let has_pending = pv
        .players
        .iter()
        .any(|p| p.response == "pending" && p.user_id.is_some());
    let has_declined = pv.players.iter().any(|p| p.response == "declined");

    let pending_slots: Vec<ProposalPlayerView> = pv
        .players
        .iter()
        .filter(|p| p.response == "pending" && p.user_id.is_some())
        .cloned()
        .collect();

    let declined_slots: Vec<ProposalPlayerView> = pv
        .players
        .iter()
        .filter(|p| p.response == "declined")
        .cloned()
        .collect();

    let (slot_actions, set_slot_actions) = signal(
        pending_slots
            .iter()
            .map(|p| (p.id, "drop".to_string()))
            .collect::<Vec<_>>(),
    );

    let pending_rows = StoredValue::new(
        pending_slots
            .iter()
            .map(|p| {
                let pid = p.id;
                let pname = p.name.clone();
                view! {
                    <div class="friend-row">
                        <span>{pname.clone()}</span>
                        " "
                        <select
                            aria-label=format!("Action for {}", pname)
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_slot_actions.update(|v| {
                                    if let Some(entry) = v.iter_mut().find(|(id, _)| *id == pid) {
                                        entry.1 = val;
                                    }
                                });
                            }
                        >
                            <option value="drop">"Drop"</option>
                            <option value="bot-easy">"Replace with bot (easy)"</option>
                            <option value="bot-medium">"Replace with bot (medium)"</option>
                            <option value="bot-hard">"Replace with bot (hard)"</option>
                        </select>
                    </div>
                }
            })
            .collect_view(),
    );

    let declined_rows = StoredValue::new(
        declined_slots
            .iter()
            .map(|p| {
                let pid = p.id;
                let pname = p.name.clone();
                view! {
                    <div class="friend-row">
                        <span>{pname.clone()}</span>
                        " - "
                        <a href="#" on:click=move |ev| {
                            ev.prevent_default();
                            replace_action.dispatch(ReplaceProposalSlot {
                                proposal_id,
                                player_id: pid,
                                bot_name: format!("{} (bot: medium)", pname),
                                bot_difficulty: "medium".to_string(),
                            });
                        }>"Replace with bot"</a>
                        " | "
                        <a href="#" on:click=move |ev| {
                            ev.prevent_default();
                            remove_action.dispatch(RemoveProposalSlot {
                                proposal_id,
                                player_id: pid,
                            });
                        }>"Remove"</a>
                    </div>
                }
            })
            .collect_view(),
    );

    let pending_names = StoredValue::new(
        pending_slots
            .iter()
            .map(|p| (p.id, p.name.clone()))
            .collect::<Vec<(Uuid, String)>>(),
    );

    view! {
        <h1>{game_type_name.clone()}</h1>
        <p class="game-card-meta">{version_name.clone()} " | " {format!("{} players", pv.players.len())}</p>

        <Show when=move || !is_open>
            <div class="form-error">
                {if pv.proposal.status == "cancelled" { "This invite was cancelled." }
                 else if pv.proposal.status == "started" { "This game has started." }
                 else { "This invite is closed." }}
            </div>
        </Show>

        <section>
            <h2>"Players"</h2>
            {pv.players.iter().map(|p| {
                let name = p.name.clone();
                let response = p.response.clone();
                let is_bot = p.user_id.is_none();
                let status_class = format!("invite-status invite-status-{}", response);
                view! {
                    <div class="friend-row">
                        <span>{name}</span>
                        {is_bot.then(|| view! { <span>" (bot)"</span> })}
                        " - "
                        <span class=status_class>{response.clone()}</span>
                    </div>
                }
            }).collect_view()}
        </section>

        <Show when=move || is_invitee && my_pending && is_open>
            <section>
                <h2>"Your invite"</h2>
                <div class="form-actions">
                    <a href="#" on:click=move |ev| {
                        ev.prevent_default();
                        respond_action.dispatch(RespondProposal { proposal_id, accept: true });
                    }>"Accept"</a>
                    " | "
                    <a href="#" on:click=move |ev| {
                        ev.prevent_default();
                        respond_action.dispatch(RespondProposal { proposal_id, accept: false });
                    }>"Decline"</a>
                </div>
            </section>
        </Show>

        <Show when=move || is_owner && is_open>
            <section>
                <h2>"Owner actions"</h2>

                <Show when=move || has_pending>
                    <div>
                        <h3>"Start early - resolve pending slots"</h3>
                        {pending_rows.into_inner()}
                        <div class="form-actions">
                            <a href="#" on:click=move |ev| {
                                ev.prevent_default();
                                let policies: Vec<SlotPolicy> = slot_actions.get_untracked().into_iter().map(|(id, action)| {
                                    let orig_name = pending_names.with_value(|names| names.iter().find(|(pid, _)| *pid == id).map(|(_, n)| n.clone()).unwrap_or_else(|| "Bot".to_string()));
                                    let action = match action.as_str() {
                                        "bot-easy" => SlotAction::ReplaceWithBot { name: format!("{} (bot: easy)", orig_name), bot_name: "easy".to_string() },
                                        "bot-medium" => SlotAction::ReplaceWithBot { name: format!("{} (bot: medium)", orig_name), bot_name: "medium".to_string() },
                                        "bot-hard" => SlotAction::ReplaceWithBot { name: format!("{} (bot: hard)", orig_name), bot_name: "hard".to_string() },
                                        _ => SlotAction::Drop,
                                    };
                                    SlotPolicy { player_id: id, action }
                                }).collect();
                                start_early_action.dispatch(StartProposalEarly { proposal_id, policies });
                            }>"Start early"</a>
                        </div>
                    </div>
                </Show>

                <Show when=move || has_declined>
                    <div>
                        <h3>"Declined slots"</h3>
                        {declined_rows.into_inner()}
                    </div>
                </Show>

                <div class="form-actions">
                    <a href="#" on:click=move |ev| {
                        ev.prevent_default();
                        cancel_action.dispatch(CancelProposal { proposal_id });
                    }>"Cancel invite"</a>
                </div>
            </section>
        </Show>

        <Show when=move || {
            respond_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || respond_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || {
            start_early_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || start_early_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || {
            cancel_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || cancel_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    async fn seed_invite_user(pool: &PgPool, invite_emails_enabled: bool) -> Uuid {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors, invite_emails_enabled)
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .bind(invite_emails_enabled)
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
        user_id
    }

    /// The exact gate the invite mailers apply: recipient resolution + the
    /// per-recipient web-presence check.
    async fn invite_gate(pool: &PgPool, user_id: Uuid) -> bool {
        let recip = fetch_invite_recipient(pool, user_id)
            .await
            .unwrap()
            .unwrap();
        let suppressed =
            crate::email::outbound::suppress_for_web_presence(pool, Some(user_id)).await;
        invite_recipient_should_send(&recip, suppressed)
    }

    #[sqlx::test]
    async fn invite_notification_suppressed_by_recipient_presence(pool: PgPool) {
        let active = seed_invite_user(&pool, true).await;
        sqlx::query("UPDATE users SET last_active_at = NOW() WHERE id = $1")
            .bind(active)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            !invite_gate(&pool, active).await,
            "invite email should be suppressed while the recipient is active on the web"
        );

        let inactive = seed_invite_user(&pool, true).await;
        sqlx::query(
            "UPDATE users SET last_active_at = NOW() - interval '11 minutes' WHERE id = $1",
        )
        .bind(inactive)
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            invite_gate(&pool, inactive).await,
            "invite email should send when the recipient is not active on the web"
        );
    }

    #[test]
    fn invite_recipient_should_send_truth_table() {
        let enabled = InviteRecipient {
            email: Some("a@b.c".into()),
            theme_slug: None,
            invite_emails_enabled: true,
            name: "A".into(),
        };
        assert!(invite_recipient_should_send(&enabled, false));
        assert!(!invite_recipient_should_send(&enabled, true));
        let disabled = InviteRecipient {
            invite_emails_enabled: false,
            ..enabled.clone()
        };
        assert!(!invite_recipient_should_send(&disabled, false));
        let no_email = InviteRecipient {
            email: None,
            ..enabled.clone()
        };
        assert!(!invite_recipient_should_send(&no_email, false));
    }
}
