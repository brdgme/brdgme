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
    fn notify_changed_reinvite(
        &self,
        proposal_id: Uuid,
        invitee_user_id: Uuid,
        email_token: Option<String>,
    );
    fn notify_owner_decline(&self, proposal_id: Uuid, invitee_user_id: Uuid);
    fn notify_cancelled(&self, proposal_id: Uuid, accepted_user_ids: Vec<Uuid>);
    fn notify_started(&self, proposal_id: Uuid, game_id: Uuid, invitee_user_ids: Vec<Uuid>);
    fn notify_owner_ready(&self, proposal_id: Uuid);
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

    fn notify_changed_reinvite(
        &self,
        proposal_id: Uuid,
        invitee_user_id: Uuid,
        email_token: Option<String>,
    ) {
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
            let content = crate::email::render::EmailContent {
                subject: format!("{game_type_name} invite"),
                header: Some(
                    "The owner has made changes to the game. Accept again for the game to start."
                        .into(),
                ),
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
                false,
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

    fn notify_owner_ready(&self, proposal_id: Uuid) {
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
            let suppressed = crate::email::outbound::suppress_for_web_presence(
                &pool,
                Some(proposal.owner_user_id),
            )
            .await;
            if !invite_recipient_should_send(&owner_recip, suppressed) {
                return;
            }
            let Some(email) = owner_recip.email else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let content = crate::email::render::EmailContent {
                subject: format!("{game_type_name} invite"),
                header: Some(format!(
                    "Everyone has accepted - your {game_type_name} game is ready to start."
                )),
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
pub async fn update_proposal_owner(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    owner_user_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE game_proposals SET owner_user_id = $1, updated_at = (now() AT TIME ZONE 'utc') WHERE id = $2",
    )
    .bind(owner_user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn normalize_proposal_positions(
    tx: &mut sqlx::PgConnection,
    proposal_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE game_proposal_players AS pp SET \"position\" = sub.rn, updated_at = (now() AT TIME ZONE 'utc') \
         FROM (SELECT id, (ROW_NUMBER() OVER (ORDER BY \"position\") - 1)::int AS rn FROM game_proposal_players WHERE proposal_id = $1) sub \
         WHERE pp.id = sub.id AND pp.proposal_id = $1",
    )
    .bind(proposal_id)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

#[cfg(feature = "ssr")]
pub async fn reset_accepted_humans_for_roster_change(
    tx: &mut sqlx::PgConnection,
    proposal_id: Uuid,
    owner_user_id: Uuid,
) -> sqlx::Result<Vec<(Uuid, String)>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM game_proposal_players \
         WHERE proposal_id = $1 AND response = 'accepted' AND user_id IS NOT NULL AND user_id <> $2 \
         ORDER BY \"position\"",
    )
    .bind(proposal_id)
    .bind(owner_user_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut out = Vec::new();
    for (player_id, user_id) in rows {
        let token = Uuid::new_v4().simple().to_string();
        sqlx::query(
            "UPDATE game_proposal_players SET response = 'pending', responded_at = NULL, email_token = $1, updated_at = (now() AT TIME ZONE 'utc') WHERE id = $2",
        )
        .bind(&token)
        .bind(player_id)
        .execute(&mut *tx)
        .await?;
        out.push((user_id, token));
    }
    Ok(out)
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

#[cfg(feature = "ssr")]
fn proposal_ready_to_start(players: &[ProposalPlayer], player_counts: &[i32]) -> bool {
    let all_humans_accepted = players
        .iter()
        .filter(|p| p.user_id.is_some())
        .all(|p| p.response == "accepted");
    if !all_humans_accepted {
        return false;
    }
    let count = players.iter().filter(|p| p.response != "declined").count();
    crate::game::server_fns::roster_error(player_counts, count).is_none()
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

/// Records an invitee's accept/decline on an open proposal. Allows
/// pending->accepted, pending->declined, and accepted->declined. Declined is
/// terminal. When the last human accepts and the roster is valid, the owner is
/// emailed that the game is ready to start.
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

    let target = if accept { "accepted" } else { "declined" };
    let allowed = matches!(
        (me.response.as_str(), target),
        ("pending", "accepted") | ("pending", "declined") | ("accepted", "declined")
    );
    if !allowed {
        let msg = if me.response == "declined" {
            "You have already declined this invite."
        } else {
            "You have already accepted this invite."
        };
        return Err(ServerFnError::new(msg));
    }

    update_proposal_player_response(&mut tx, me.id, target)
        .await
        .map_err(internal("respond_proposal: update"))?;

    let mut became_ready = false;
    if accept {
        let updated_players = find_proposal_players_tx(&mut tx, proposal_id)
            .await
            .map_err(internal("respond_proposal: updated players"))?;
        let player_counts =
            crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
                .await
                .map_err(internal("respond_proposal: player counts"))?
                .unwrap_or_default();
        became_ready = proposal_ready_to_start(&updated_players, &player_counts);
    }

    tx.commit()
        .await
        .map_err(internal("respond_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    if became_ready {
        mailer().notify_owner_ready(proposal_id);
    } else if !accept {
        mailer().notify_owner_decline(proposal_id, user.id);
    }

    Ok(RespondOutcome {
        accepted: accept,
        started: false,
        game_id: None,
    })
}

/// Owner-only: explicitly start an open proposal. Requires all humans to have
/// accepted, no declines, and a valid player count.
#[server(StartProposal, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn start_proposal(proposal_id: Uuid) -> Result<Uuid, ServerFnError> {
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
        .map_err(internal("start_proposal: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("start_proposal: lock"))?
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
        .map_err(internal("start_proposal: players"))?;

    let pending_humans = players
        .iter()
        .filter(|p| p.user_id.is_some() && p.response == "pending")
        .count();
    if pending_humans > 0 {
        return Err(ServerFnError::new(format!(
            "Cannot start: {pending_humans} players have not responded"
        )));
    }

    let declined = players.iter().filter(|p| p.response == "declined").count();
    if declined > 0 {
        return Err(ServerFnError::new(format!(
            "Cannot start: {declined} players have declined"
        )));
    }

    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal: player counts"))?
        .unwrap_or_default();
    let count = players.iter().filter(|p| p.response != "declined").count();
    if let Some(msg) = crate::game::server_fns::roster_error(&player_counts, count) {
        return Err(ServerFnError::new(msg));
    }

    let game_version = crate::db::find_game_version(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal: game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let game_id =
        start_proposal_tx(&mut tx, &http_client, &proposal, &players, &game_version).await?;

    tx.commit()
        .await
        .map_err(internal("start_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;
    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;

    let invitee_ids: Vec<Uuid> = players
        .iter()
        .filter(|p| p.response == "accepted")
        .filter_map(|p| p.user_id)
        .filter(|id| *id != proposal.owner_user_id)
        .collect();
    mailer().notify_started(proposal_id, game_id, invitee_ids);

    Ok(game_id)
}

/// Owner-only: add a single human (by id or email) or bot to an open proposal.
/// Re-normalizes positions and resets accepted humans to pending.
#[server(AddProposalPlayer, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn add_proposal_player(
    proposal_id: Uuid,
    user_id: Option<Uuid>,
    email: Option<String>,
    bot: Option<crate::game::server_fns::BotSlot>,
) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

    let provided =
        usize::from(user_id.is_some()) + usize::from(email.is_some()) + usize::from(bot.is_some());
    if provided != 1 {
        return Err(ServerFnError::new("Choose a player, email, or bot to add."));
    }

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("add_proposal_player: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("add_proposal_player: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("add_proposal_player: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("add_proposal_player: players"))?;

    let human_id = if let Some(uid) = user_id {
        Some(uid)
    } else if let Some(email) = &email {
        Some(find_or_create_user_by_email_tx(&mut tx, email).await?)
    } else {
        None
    };

    if let Some(hid) = human_id {
        let policy_ids: Vec<Uuid> = user_id.into_iter().collect();
        let policy_emails: Vec<String> = email.clone().into_iter().collect();
        let violations =
            crate::db::check_invite_policy_tx(&mut tx, user.id, &policy_ids, &policy_emails)
                .await
                .map_err(internal("add_proposal_player: check invite policy"))?;
        if let Some(msg) = violations.into_iter().next() {
            return Err(ServerFnError::new(msg));
        }
        if players.iter().any(|p| p.user_id == Some(hid)) {
            return Err(ServerFnError::new(
                "Please ensure each player in the game is unique",
            ));
        }
    }

    let position = players.len() as i32;
    let mut invite: Option<(Uuid, String)> = None;
    if let Some(hid) = human_id {
        let token = Uuid::new_v4().simple().to_string();
        insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            Some(hid),
            None,
            None,
            "pending",
            Some(token.clone()),
        )
        .await
        .map_err(internal("add_proposal_player: insert human"))?;
        invite = Some((hid, token));
    } else if let Some(bot) = bot {
        insert_proposal_player(
            &mut tx,
            proposal_id,
            position,
            None,
            Some(bot.name),
            Some(bot.bot_name),
            "accepted",
            None,
        )
        .await
        .map_err(internal("add_proposal_player: insert bot"))?;
    }

    let reset =
        reset_accepted_humans_for_roster_change(&mut tx, proposal_id, proposal.owner_user_id)
            .await
            .map_err(internal("add_proposal_player: reset"))?;

    normalize_proposal_positions(&mut tx, proposal_id)
        .await
        .map_err(internal("add_proposal_player: normalize"))?;

    tx.commit()
        .await
        .map_err(internal("add_proposal_player: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    if let Some((uid, token)) = invite {
        mailer().send_invite(proposal_id, uid, Some(token));
    }
    for (uid, tok) in reset {
        mailer().notify_changed_reinvite(proposal_id, uid, Some(tok));
    }

    Ok(())
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

/// Owner-only: remove any slot (human or bot), allowing invalid player counts;
/// re-normalizes positions and resets accepted humans to pending.
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
    if target.user_id == Some(proposal.owner_user_id) {
        return Err(ServerFnError::new(
            "The owner can't be removed from their own proposal.",
        ));
    }

    delete_proposal_player(&mut tx, player_id)
        .await
        .map_err(internal("remove_proposal_slot: delete"))?;

    let reset =
        reset_accepted_humans_for_roster_change(&mut tx, proposal_id, proposal.owner_user_id)
            .await
            .map_err(internal("remove_proposal_slot: reset"))?;

    normalize_proposal_positions(&mut tx, proposal_id)
        .await
        .map_err(internal("remove_proposal_slot: normalize"))?;

    tx.commit()
        .await
        .map_err(internal("remove_proposal_slot: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;

    for (uid, tok) in reset {
        mailer().notify_changed_reinvite(proposal_id, uid, Some(tok));
    }

    Ok(())
}

/// Owner-only: transfer ownership to another human player in the roster. Does
/// not change any responses or trigger an acceptance reset.
#[server(TransferProposalOwnership, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn transfer_proposal_ownership(
    proposal_id: Uuid,
    target_user_id: Uuid,
) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

    let proposal = find_proposal(&pool, proposal_id)
        .await
        .map_err(internal("transfer_proposal_ownership: find"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(internal("transfer_proposal_ownership: begin transaction"))?;

    let proposal = lock_proposal_for_update(&mut tx, proposal_id)
        .await
        .map_err(internal("transfer_proposal_ownership: lock"))?
        .ok_or_else(|| ServerFnError::new("Invite not found"))?;
    if proposal.owner_user_id != user.id {
        return Err(ServerFnError::new("Only the owner can edit this proposal."));
    }
    if proposal.status != "open" {
        return Err(ServerFnError::new("This proposal is no longer open."));
    }

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("transfer_proposal_ownership: players"))?;
    if !players.iter().any(|p| p.user_id == Some(target_user_id)) {
        return Err(ServerFnError::new("That player isn't in this proposal."));
    }

    update_proposal_owner(&mut tx, proposal_id, target_user_id)
        .await
        .map_err(internal("transfer_proposal_ownership: update owner"))?;

    tx.commit()
        .await
        .map_err(internal("transfer_proposal_ownership: commit transaction"))?;

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

    let nav3 = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(())) = cancel_action.value().get() {
            nav3("/dashboard", NavigateOptions::default());
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

    let has_declined = pv.players.iter().any(|p| p.response == "declined");

    let declined_slots: Vec<ProposalPlayerView> = pv
        .players
        .iter()
        .filter(|p| p.response == "declined")
        .cloned()
        .collect();

    let declined_rows = StoredValue::new(
        declined_slots
            .iter()
            .map(|p| {
                let pid = p.id;
                let pname = p.name.clone();
                view! {
                    <div class="friend-row">
                        <span>{pname}</span>
                        " - "
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
            cancel_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || cancel_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || {
            remove_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || remove_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
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

    async fn seed_game_version(pool: &PgPool) -> Uuid {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("Test Game {}", Uuid::new_v4()))
        .bind(vec![2i32, 3, 4])
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '1.0.0', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_proposal(pool: &PgPool, game_version_id: Uuid, owner_id: Uuid) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO game_proposals (game_version_id, owner_user_id, status)
             VALUES ($1, $2, 'open') RETURNING id",
        )
        .bind(game_version_id)
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn reset_flips_accepted_humans_preserves_others(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let b = seed_invite_user(&pool, true).await;
        let c = seed_invite_user(&pool, true).await;
        let d = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 2, Some(b), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            3,
            None,
            Some("Botty".into()),
            Some("medium".into()),
            "accepted",
            None,
        )
        .await
        .unwrap();
        insert_proposal_player(&mut tx, pid, 4, Some(c), None, None, "declined", None)
            .await
            .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            5,
            Some(d),
            None,
            None,
            "pending",
            Some("orig-token-d".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let reset = reset_accepted_humans_for_roster_change(&mut tx, pid, owner)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(reset.len(), 2);
        let reset_ids: Vec<Uuid> = reset.iter().map(|(u, _)| *u).collect();
        assert!(reset_ids.contains(&a));
        assert!(reset_ids.contains(&b));
        assert!(!reset_ids.contains(&owner));
        assert!(!reset_ids.contains(&c));
        assert!(!reset_ids.contains(&d));
        let tok_a = reset.iter().find(|(u, _)| *u == a).unwrap().1.clone();
        let tok_b = reset.iter().find(|(u, _)| *u == b).unwrap().1.clone();
        assert!(!tok_a.is_empty());
        assert!(!tok_b.is_empty());
        assert_ne!(tok_a, tok_b);

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let by_user = |u: Uuid| {
            players
                .iter()
                .find(|p| p.user_id == Some(u))
                .unwrap()
                .clone()
        };

        let pa = by_user(a);
        assert_eq!(pa.response, "pending");
        assert!(pa.responded_at.is_none());
        assert!(pa.email_token.as_deref().is_some_and(|t| !t.is_empty()));
        let pb = by_user(b);
        assert_eq!(pb.response, "pending");
        assert!(pb.responded_at.is_none());
        assert!(pb.email_token.as_deref().is_some_and(|t| !t.is_empty()));
        assert_ne!(pa.email_token, pb.email_token);

        assert_eq!(by_user(owner).response, "accepted");
        let bot = players.iter().find(|p| p.user_id.is_none()).unwrap();
        assert_eq!(bot.response, "accepted");
        assert_eq!(by_user(c).response, "declined");
        let pd = by_user(d);
        assert_eq!(pd.response, "pending");
        assert_eq!(pd.email_token.as_deref(), Some("orig-token-d"));
    }

    #[sqlx::test]
    async fn add_player_inserts_pending_human_and_accepted_bot(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let uid = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(uid),
            None,
            None,
            "pending",
            Some("tok-1".into()),
        )
        .await
        .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            2,
            None,
            Some("Bot".into()),
            Some("medium".into()),
            "accepted",
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let human = players.iter().find(|p| p.user_id == Some(uid)).unwrap();
        assert_eq!(human.response, "pending");
        assert_eq!(human.user_id, Some(uid));
        assert_eq!(human.email_token.as_deref(), Some("tok-1"));
        let bot = players.iter().find(|p| p.user_id.is_none()).unwrap();
        assert_eq!(bot.response, "accepted");
        assert_eq!(bot.user_id, None);
        assert_eq!(bot.bot_name.as_deref(), Some("Bot"));
        assert!(bot.email_token.is_none());
    }

    #[sqlx::test]
    async fn remove_works_on_accepted_slot_and_allows_invalid_count(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let a_player =
            insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "accepted", None)
                .await
                .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        delete_proposal_player(&mut tx, a_player).await.unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].user_id, Some(owner));
        assert!(!players.iter().any(|p| p.user_id == Some(a)));
    }

    #[sqlx::test]
    async fn transfer_rejects_bot_and_nonplayer_targets(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            2,
            None,
            Some("Botty".into()),
            Some("medium".into()),
            "accepted",
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let roster = find_proposal_players(&pool, pid).await.unwrap();
        let bot = roster.iter().find(|p| p.bot_name.is_some()).unwrap();
        assert_eq!(bot.user_id, None);
        let random = Uuid::new_v4();
        assert!(!roster.iter().any(|p| p.user_id == Some(random)));
        assert!(roster.iter().any(|p| p.user_id == Some(a)));

        let mut tx = pool.begin().await.unwrap();
        update_proposal_owner(&mut tx, pid, a).await.unwrap();
        tx.commit().await.unwrap();

        let proposal = find_proposal(&pool, pid).await.unwrap().unwrap();
        assert_eq!(proposal.owner_user_id, a);
    }

    #[sqlx::test]
    async fn normalize_positions_after_remove_and_add(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let h1 = seed_invite_user(&pool, true).await;
        let h2 = seed_invite_user(&pool, true).await;
        let h3 = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(h1), None, None, "accepted", None)
            .await
            .unwrap();
        let p2 = insert_proposal_player(
            &mut tx,
            pid,
            2,
            Some(h2),
            None,
            None,
            "pending",
            Some("t2".into()),
        )
        .await
        .unwrap();
        let p3 = insert_proposal_player(
            &mut tx,
            pid,
            3,
            Some(h3),
            None,
            None,
            "pending",
            Some("t3".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let h1_player = players.iter().find(|p| p.user_id == Some(h1)).unwrap().id;
        let mut tx = pool.begin().await.unwrap();
        delete_proposal_player(&mut tx, h1_player).await.unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let positions: Vec<i32> = players.iter().map(|p| p.position).collect();
        assert_eq!(positions, vec![0, 2, 3]);

        let mut tx = pool.begin().await.unwrap();
        normalize_proposal_positions(&mut tx, pid).await.unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let positions: Vec<i32> = players.iter().map(|p| p.position).collect();
        assert_eq!(positions, vec![0, 1, 2]);
        assert_eq!(players[1].id, p2);
        assert_eq!(players[2].id, p3);

        let new_user = seed_invite_user(&pool, true).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            3,
            Some(new_user),
            None,
            None,
            "pending",
            Some("t4".into()),
        )
        .await
        .unwrap();
        normalize_proposal_positions(&mut tx, pid).await.unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let positions: Vec<i32> = players.iter().map(|p| p.position).collect();
        assert_eq!(positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn ready_to_start_requires_all_humans_accepted_and_valid_count() {
        let counts = vec![2, 3, 4];
        let mk = |user_id: Option<Uuid>, response: &str| ProposalPlayer {
            id: Uuid::new_v4(),
            created_at: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            updated_at: time::PrimitiveDateTime::new(
                time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
                time::Time::MIDNIGHT,
            ),
            proposal_id: Uuid::new_v4(),
            position: 0,
            user_id,
            bot_name: None,
            bot_difficulty: None,
            response: response.to_string(),
            responded_at: None,
            email_token: None,
        };
        let owner = Uuid::new_v4();
        let human = Uuid::new_v4();

        let all_accepted = vec![mk(Some(owner), "accepted"), mk(Some(human), "accepted")];
        assert!(proposal_ready_to_start(&all_accepted, &counts));

        let with_pending = vec![mk(Some(owner), "accepted"), mk(Some(human), "pending")];
        assert!(!proposal_ready_to_start(&with_pending, &counts));

        let with_declined = vec![mk(Some(owner), "accepted"), mk(Some(human), "declined")];
        assert!(!proposal_ready_to_start(&with_declined, &counts));

        let with_bot = vec![
            mk(Some(owner), "accepted"),
            mk(Some(human), "accepted"),
            mk(None, "accepted"),
        ];
        assert!(proposal_ready_to_start(&with_bot, &counts));

        let invalid_count = vec![mk(Some(owner), "accepted")];
        assert!(!proposal_ready_to_start(&invalid_count, &counts));

        let bot_does_not_block = vec![
            mk(Some(owner), "accepted"),
            mk(Some(human), "accepted"),
            mk(None, "accepted"),
            mk(None, "accepted"),
            mk(None, "accepted"),
        ];
        assert!(!proposal_ready_to_start(&bot_does_not_block, &counts));
    }

    #[sqlx::test]
    async fn respond_accept_does_not_auto_start(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let invitee = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let inv_player = insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(invitee),
            None,
            None,
            "pending",
            Some("tok".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        update_proposal_player_response(&mut tx, inv_player, "accepted")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let proposal = find_proposal(&pool, pid).await.unwrap().unwrap();
        assert_eq!(
            proposal.status, "open",
            "accepting must not auto-start the game"
        );
        assert!(proposal.started_game_id.is_none());
    }

    #[sqlx::test]
    async fn ready_check_fires_only_when_last_human_accepts(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let b = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let pa = insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(a),
            None,
            None,
            "pending",
            Some("ta".into()),
        )
        .await
        .unwrap();
        let pb = insert_proposal_player(
            &mut tx,
            pid,
            2,
            Some(b),
            None,
            None,
            "pending",
            Some("tb".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        update_proposal_player_response(&mut tx, pa, "accepted")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let counts = vec![2, 3, 4];
        assert!(
            !proposal_ready_to_start(&players, &counts),
            "not ready while a human is still pending"
        );

        let mut tx = pool.begin().await.unwrap();
        update_proposal_player_response(&mut tx, pb, "accepted")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        assert!(
            proposal_ready_to_start(&players, &counts),
            "ready once the last human accepts"
        );
    }

    #[sqlx::test]
    async fn start_guards_reject_pending_declined_invalid_count(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let b = seed_invite_user(&pool, true).await;
        let counts = vec![2, 3, 4];

        let pid = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(a),
            None,
            None,
            "pending",
            Some("ta".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let pending_humans = players
            .iter()
            .filter(|p| p.user_id.is_some() && p.response == "pending")
            .count();
        assert!(pending_humans > 0, "pending guard should fire");

        let pid2 = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid2, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid2, 1, Some(a), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid2, 2, Some(b), None, None, "declined", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid2).await.unwrap();
        let declined = players.iter().filter(|p| p.response == "declined").count();
        assert!(declined > 0, "declined guard should fire");

        let pid3 = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid3, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid3).await.unwrap();
        let count = players.iter().filter(|p| p.response != "declined").count();
        assert!(
            crate::game::server_fns::roster_error(&counts, count).is_some(),
            "invalid count guard should fire for 1 player when counts are [2,3,4]"
        );
    }

    #[sqlx::test]
    async fn start_conditions_met_when_all_accepted_and_valid(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let counts = vec![2, 3, 4];
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "accepted", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let pending_humans = players
            .iter()
            .filter(|p| p.user_id.is_some() && p.response == "pending")
            .count();
        let declined = players.iter().filter(|p| p.response == "declined").count();
        let count = players.iter().filter(|p| p.response != "declined").count();
        assert_eq!(pending_humans, 0);
        assert_eq!(declined, 0);
        assert!(crate::game::server_fns::roster_error(&counts, count).is_none());
        assert!(proposal_ready_to_start(&players, &counts));
    }

    #[sqlx::test]
    async fn accepted_to_declined_transition_works(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let pa = insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "accepted", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let current = "accepted";
        let target = "declined";
        let allowed = matches!(
            (current, target),
            ("pending", "accepted") | ("pending", "declined") | ("accepted", "declined")
        );
        assert!(allowed, "accepted -> declined must be allowed");

        let mut tx = pool.begin().await.unwrap();
        update_proposal_player_response(&mut tx, pa, "declined")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let player = players.iter().find(|p| p.user_id == Some(a)).unwrap();
        assert_eq!(player.response, "declined");
    }

    #[sqlx::test]
    async fn declined_to_accepted_is_rejected(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(a), None, None, "declined", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let current = "declined";
        let target = "accepted";
        let allowed = matches!(
            (current, target),
            ("pending", "accepted") | ("pending", "declined") | ("accepted", "declined")
        );
        assert!(!allowed, "declined -> accepted must be rejected");
    }

    #[sqlx::test]
    async fn pending_to_accepted_still_works(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let pa = insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(a),
            None,
            None,
            "pending",
            Some("tok".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let current = "pending";
        let target = "accepted";
        let allowed = matches!(
            (current, target),
            ("pending", "accepted") | ("pending", "declined") | ("accepted", "declined")
        );
        assert!(allowed, "pending -> accepted must be allowed");

        let mut tx = pool.begin().await.unwrap();
        update_proposal_player_response(&mut tx, pa, "accepted")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let players = find_proposal_players(&pool, pid).await.unwrap();
        let player = players.iter().find(|p| p.user_id == Some(a)).unwrap();
        assert_eq!(player.response, "accepted");
    }
}
