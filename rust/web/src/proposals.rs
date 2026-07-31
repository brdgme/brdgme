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

use crate::components::opponent_slot::{OpponentSlot, OpponentSlotEditor};

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
    pub viewer_user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteSummary {
    pub proposal_id: Uuid,
    pub game_type_name: String,
    pub owner_name: String,
    pub player_count: i64,
}

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
pub trait InviteMailer: Send + Sync {
    fn send_invite(&self, proposal_id: Uuid, invitee_user_id: Uuid, email_token: Option<String>);
    async fn send_invite_now(
        &self,
        proposal_id: Uuid,
        invitee_user_id: Uuid,
        email_token: Option<String>,
    ) -> bool;
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

/// Last-resort labels when a lookup fails inside a spawned mailer task. Blank
/// substitutions produced subjects like " invite from " (wd F34).
#[cfg(feature = "ssr")]
const UNKNOWN_GAME_TYPE_NAME: &str = "Game";
#[cfg(feature = "ssr")]
const UNKNOWN_PLAYER_NAME: &str = "Someone";

/// Loads a proposal for a mailer task, logging instead of returning silently:
/// inside a spawned task a DB error is otherwise indistinguishable from
/// "proposal deleted" and from "recipient opted out" (wd F34). `what` names the
/// mailer method.
#[cfg(feature = "ssr")]
async fn mailer_proposal(pool: &PgPool, proposal_id: Uuid, what: &str) -> Option<Proposal> {
    match find_proposal(pool, proposal_id).await {
        Ok(Some(p)) => Some(p),
        Ok(None) => {
            tracing::warn!("invite mailer ({what}): proposal {proposal_id} not found; no email");
            None
        }
        Err(e) => {
            tracing::error!("invite mailer ({what}): proposal {proposal_id} lookup failed: {e}");
            None
        }
    }
}

/// Same for a recipient row (wd F34).
#[cfg(feature = "ssr")]
async fn mailer_recipient(pool: &PgPool, user_id: Uuid, what: &str) -> Option<InviteRecipient> {
    match fetch_invite_recipient(pool, user_id).await {
        Ok(Some(r)) => Some(r),
        Ok(None) => {
            tracing::warn!("invite mailer ({what}): user {user_id} not found; no email");
            None
        }
        Err(e) => {
            tracing::error!("invite mailer ({what}): user {user_id} lookup failed: {e}");
            None
        }
    }
}

#[cfg(feature = "ssr")]
async fn proposal_game_type_name(pool: &PgPool, proposal: &Proposal) -> String {
    let game_version = match crate::db::find_game_version(pool, proposal.game_version_id).await {
        Ok(Some(gv)) => gv,
        Ok(None) => {
            tracing::warn!(
                "invite mailer: game version {} not found; using a generic label",
                proposal.game_version_id
            );
            return UNKNOWN_GAME_TYPE_NAME.to_string();
        }
        Err(e) => {
            tracing::error!(
                "invite mailer: game version {} lookup failed: {e}",
                proposal.game_version_id
            );
            return UNKNOWN_GAME_TYPE_NAME.to_string();
        }
    };
    match find_game_type_name(pool, game_version.game_type_id).await {
        Ok(Some(name)) => name,
        Ok(None) => {
            tracing::warn!(
                "invite mailer: game type {} not found; using a generic label",
                game_version.game_type_id
            );
            UNKNOWN_GAME_TYPE_NAME.to_string()
        }
        Err(e) => {
            tracing::error!(
                "invite mailer: game type {} lookup failed: {e}",
                game_version.game_type_id
            );
            UNKNOWN_GAME_TYPE_NAME.to_string()
        }
    }
}

#[cfg(feature = "ssr")]
impl RealInviteMailer {
    /// The real invite-send work, awaited and observable. Returns `true` when
    /// the invite was sent OR is permanently unsendable (no token, recipient
    /// missing/gated, no address, proposal gone, not open, or a rotated/stale
    /// token) - any outcome that should NOT be retried. Returns `false` only on
    /// a transient condition (web presence suppression, or a failed send) so the
    /// caller can leave the row unmarked and retry next tick. Mirrors the
    /// reminder sweep's at-least-once semantics (D-02).
    async fn send_invite_core(
        &self,
        proposal_id: Uuid,
        invitee_user_id: Uuid,
        email_token: Option<String>,
    ) -> bool {
        let pool = &self.pool;
        let resend = &self.resend;
        let Some(token) = email_token else {
            return true;
        };
        let Some(recip) = mailer_recipient(pool, invitee_user_id, "send_invite").await else {
            return true;
        };
        let suppressed =
            crate::email::outbound::suppress_for_web_presence(pool, Some(invitee_user_id)).await;
        if suppressed {
            return false;
        }
        if !invite_recipient_should_send(&recip, false) {
            return true;
        }
        let Some(email) = recip.email else {
            return true;
        };
        let Some(proposal) = mailer_proposal(pool, proposal_id, "send_invite").await else {
            return true;
        };
        if proposal.status != "open" {
            return true;
        }
        let Ok(Some(pp)) = find_proposal_player_by_email_token(pool, &token).await else {
            return true;
        };
        if pp.proposal_id != proposal_id
            || pp.user_id != Some(invitee_user_id)
            || pp.response != "pending"
        {
            return true;
        }
        let game_type_name = proposal_game_type_name(pool, &proposal).await;
        let owner_name = mailer_recipient(pool, proposal.owner_user_id, "send_invite")
            .await
            .map(|r| r.name)
            .unwrap_or_else(|| UNKNOWN_PLAYER_NAME.to_string());
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
        let unsub_token: Option<String> =
            match crate::email::outbound::ensure_unsubscribe_token(pool, invitee_user_id).await {
                Ok(tok) => Some(tok),
                Err(err) => {
                    tracing::warn!(
                        "invite: unsubscribe token fetch failed for {}: {}",
                        invitee_user_id,
                        err
                    );
                    None
                }
            };
        let unsubscribe = unsub_token
            .as_ref()
            .map(|tok| crate::email::render::Unsubscribe {
                kind: crate::email::render::EmailKind::Invite,
                token: tok,
            });
        let rendered = crate::email::render::render_game_email(
            &content,
            palette,
            &[],
            Some(&format!("proposal-{proposal_id}")),
            true,
            &format!("i-{token}@brdg.me"),
            unsubscribe,
        );
        crate::email::outbound::try_send_rendered_email(resend.as_ref(), rendered, &email).await
    }
}

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
impl InviteMailer for RealInviteMailer {
    fn send_invite(&self, proposal_id: Uuid, invitee_user_id: Uuid, email_token: Option<String>) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let mailer = RealInviteMailer { pool, resend };
            mailer
                .send_invite_core(proposal_id, invitee_user_id, email_token)
                .await;
        });
    }

    async fn send_invite_now(
        &self,
        proposal_id: Uuid,
        invitee_user_id: Uuid,
        email_token: Option<String>,
    ) -> bool {
        self.send_invite_core(proposal_id, invitee_user_id, email_token)
            .await
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
            let Some(recip) =
                mailer_recipient(&pool, invitee_user_id, "notify_changed_reinvite").await
            else {
                return;
            };
            let suppressed =
                crate::email::outbound::suppress_for_web_presence(&pool, Some(invitee_user_id))
                    .await;
            if !invite_recipient_should_send(&recip, suppressed) {
                return;
            }
            let Some(email) = recip.email else { return };
            let Some(proposal) =
                mailer_proposal(&pool, proposal_id, "notify_changed_reinvite").await
            else {
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
            let unsub_token: Option<String> =
                match crate::email::outbound::ensure_unsubscribe_token(&pool, invitee_user_id).await
                {
                    Ok(tok) => Some(tok),
                    Err(err) => {
                        tracing::warn!(
                            "invite: unsubscribe token fetch failed for {}: {}",
                            invitee_user_id,
                            err
                        );
                        None
                    }
                };
            let unsubscribe = unsub_token
                .as_ref()
                .map(|tok| crate::email::render::Unsubscribe {
                    kind: crate::email::render::EmailKind::Invite,
                    token: tok,
                });
            let rendered = crate::email::render::render_game_email(
                &content,
                palette,
                &[],
                Some(&format!("proposal-{proposal_id}")),
                false,
                &format!("i-{token}@brdg.me"),
                unsubscribe,
            );
            crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email).await;
        });
    }

    fn notify_owner_decline(&self, proposal_id: Uuid, invitee_user_id: Uuid) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Some(proposal) = mailer_proposal(&pool, proposal_id, "notify_owner_decline").await
            else {
                return;
            };
            let Some(owner_recip) =
                mailer_recipient(&pool, proposal.owner_user_id, "notify_owner_decline").await
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
            let invitee_name = mailer_recipient(&pool, invitee_user_id, "notify_owner_decline")
                .await
                .map(|r| r.name)
                .unwrap_or_else(|| UNKNOWN_PLAYER_NAME.to_string());
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let content = crate::email::render::EmailContent {
                subject: format!("{game_type_name} invite"),
                header: Some(format!("{invitee_name} declined your invite.")),
                digest: None,
                board: None,
                you_can: None,
                browser_url: Some(invite_browser_url(proposal_id)),
                rules_url: Some(crate::email::notify::rules_url(proposal.game_version_id)),
                // No reply channel: these are one-way notifications and the
                // proposal reply route needs a player email_token, which this
                // mail has none of (wd F33).
                footer: Some("Unsubscribe anytime.".into()),
            };
            let palette = crate::email::render::palette_for_slug(owner_recip.theme_slug.as_deref());
            let unsub_token: Option<String> =
                match crate::email::outbound::ensure_unsubscribe_token(
                    &pool,
                    proposal.owner_user_id,
                )
                .await
                {
                    Ok(tok) => Some(tok),
                    Err(err) => {
                        tracing::warn!(
                            "invite: unsubscribe token fetch failed for {}: {}",
                            proposal.owner_user_id,
                            err
                        );
                        None
                    }
                };
            let unsubscribe = unsub_token
                .as_ref()
                .map(|tok| crate::email::render::Unsubscribe {
                    kind: crate::email::render::EmailKind::Invite,
                    token: tok,
                });
            let rendered = crate::email::render::render_game_email(
                &content,
                palette,
                &[],
                Some(&format!("proposal-{proposal_id}")),
                false,
                &crate::email::notify::invite_reply_address("noreply"),
                unsubscribe,
            );
            crate::email::outbound::send_rendered_email(resend.as_ref(), rendered, &email).await;
        });
    }

    fn notify_cancelled(&self, proposal_id: Uuid, accepted_user_ids: Vec<Uuid>) {
        let pool = self.pool.clone();
        let resend = self.resend.clone();
        tokio::spawn(async move {
            let Some(proposal) = mailer_proposal(&pool, proposal_id, "notify_cancelled").await
            else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            for user_id in accepted_user_ids {
                let Some(recip) = mailer_recipient(&pool, user_id, "notify_cancelled").await else {
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
                    // No reply channel: these are one-way notifications and the
                    // proposal reply route needs a player email_token, which this
                    // mail has none of (wd F33).
                    footer: Some("Unsubscribe anytime.".into()),
                };
                let palette = crate::email::render::palette_for_slug(recip.theme_slug.as_deref());
                let unsub_token: Option<String> =
                    match crate::email::outbound::ensure_unsubscribe_token(&pool, user_id).await {
                        Ok(tok) => Some(tok),
                        Err(err) => {
                            tracing::warn!(
                                "invite: unsubscribe token fetch failed for {}: {}",
                                user_id,
                                err
                            );
                            None
                        }
                    };
                let unsubscribe =
                    unsub_token
                        .as_ref()
                        .map(|tok| crate::email::render::Unsubscribe {
                            kind: crate::email::render::EmailKind::Invite,
                            token: tok,
                        });
                let rendered = crate::email::render::render_game_email(
                    &content,
                    palette,
                    &[],
                    Some(&format!("proposal-{proposal_id}")),
                    false,
                    &crate::email::notify::invite_reply_address("noreply"),
                    unsubscribe,
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
            let Some(proposal) = mailer_proposal(&pool, proposal_id, "notify_started").await else {
                return;
            };
            let game_type_name = proposal_game_type_name(&pool, &proposal).await;
            let base = crate::config::public_base_url();
            let game_url = format!("{base}/games/{game_id}");
            for user_id in invitee_user_ids {
                let Some(recip) = mailer_recipient(&pool, user_id, "notify_started").await else {
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
                    // No reply channel: these are one-way notifications and the
                    // proposal reply route needs a player email_token, which this
                    // mail has none of (wd F33).
                    footer: Some("Unsubscribe anytime.".into()),
                };
                let palette = crate::email::render::palette_for_slug(recip.theme_slug.as_deref());
                let unsub_token: Option<String> =
                    match crate::email::outbound::ensure_unsubscribe_token(&pool, user_id).await {
                        Ok(tok) => Some(tok),
                        Err(err) => {
                            tracing::warn!(
                                "invite: unsubscribe token fetch failed for {}: {}",
                                user_id,
                                err
                            );
                            None
                        }
                    };
                let unsubscribe =
                    unsub_token
                        .as_ref()
                        .map(|tok| crate::email::render::Unsubscribe {
                            kind: crate::email::render::EmailKind::Invite,
                            token: tok,
                        });
                let rendered = crate::email::render::render_game_email(
                    &content,
                    palette,
                    &[],
                    Some(&format!("proposal-{proposal_id}")),
                    false,
                    &crate::email::notify::invite_reply_address("noreply"),
                    unsubscribe,
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
            let Some(proposal) = mailer_proposal(&pool, proposal_id, "notify_owner_ready").await
            else {
                return;
            };
            let Some(owner_recip) =
                mailer_recipient(&pool, proposal.owner_user_id, "notify_owner_ready").await
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
                // No reply channel: these are one-way notifications and the
                // proposal reply route needs a player email_token, which this
                // mail has none of (wd F33).
                footer: Some("Unsubscribe anytime.".into()),
            };
            let palette = crate::email::render::palette_for_slug(owner_recip.theme_slug.as_deref());
            let unsub_token: Option<String> =
                match crate::email::outbound::ensure_unsubscribe_token(
                    &pool,
                    proposal.owner_user_id,
                )
                .await
                {
                    Ok(tok) => Some(tok),
                    Err(err) => {
                        tracing::warn!(
                            "invite: unsubscribe token fetch failed for {}: {}",
                            proposal.owner_user_id,
                            err
                        );
                        None
                    }
                };
            let unsubscribe = unsub_token
                .as_ref()
                .map(|tok| crate::email::render::Unsubscribe {
                    kind: crate::email::render::EmailKind::Invite,
                    token: tok,
                });
            let rendered = crate::email::render::render_game_email(
                &content,
                palette,
                &[],
                Some(&format!("proposal-{proposal_id}")),
                false,
                &crate::email::notify::invite_reply_address("noreply"),
                unsubscribe,
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
         pp.responded_at, \
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
    sqlx::query_as(
        "UPDATE game_proposal_players \
         SET response = 'pending', responded_at = NULL, \
             email_token = replace(gen_random_uuid()::text, '-', ''), \
             updated_at = (now() AT TIME ZONE 'utc') \
         WHERE proposal_id = $1 AND response = 'accepted' \
           AND user_id IS NOT NULL AND user_id <> $2 \
         RETURNING user_id, email_token",
    )
    .bind(proposal_id)
    .bind(owner_user_id)
    .fetch_all(&mut *tx)
    .await
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
           AND gp.created_at < NOW() - ($1 * interval '1 second') \
           AND pp.response = 'pending' AND pp.user_id IS NOT NULL",
    )
    .bind(threshold_secs)
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
        "SELECT id FROM game_proposals WHERE status = 'open' AND created_at < NOW() - ($1 * interval '1 second')",
    )
    .bind(threshold_secs)
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
    let owner: Option<Uuid> = match sqlx::query_scalar(
        "SELECT owner_user_id FROM game_proposals WHERE id = $1 AND status = 'open'",
    )
    .bind(proposal_id)
    .fetch_optional(pool)
    .await
    {
        Ok(owner) => owner,
        Err(e) => {
            tracing::error!(
                "invite_expiry: owner read failed for {}: {}",
                proposal_id,
                e
            );
            return None;
        }
    };
    let owner = owner?;
    let accepted: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT user_id FROM game_proposal_players WHERE proposal_id = $1 AND response = 'accepted' AND user_id IS NOT NULL",
    )
    .bind(proposal_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(
                "invite_expiry: accepted-read failed for {}: {}",
                proposal_id,
                e
            );
            return None;
        }
    };
    let accepted_ids: Vec<Uuid> = accepted.into_iter().filter(|id| *id != owner).collect();
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
    Some((owner, accepted_ids))
}

#[cfg(feature = "ssr")]
pub async fn fetch_auto_decline_candidates(
    pool: &PgPool,
    threshold_secs: i64,
) -> Vec<(Uuid, Uuid, Uuid)> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
        "SELECT pp.id, pp.proposal_id, pp.user_id \
         FROM game_proposal_players pp \
         JOIN game_proposals gp ON gp.id = pp.proposal_id \
         WHERE gp.status = 'open' \
           AND pp.response = 'pending' \
           AND pp.user_id IS NOT NULL \
           AND pp.updated_at < NOW() - ($1 * interval '1 second')",
    )
    .bind(threshold_secs)
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
pub async fn auto_decline_proposal_player(pool: &PgPool, player_id: Uuid) -> bool {
    match sqlx::query(
        "UPDATE game_proposal_players SET response = 'declined', responded_at = NOW(), updated_at = NOW() WHERE id = $1 AND response = 'pending'",
    )
    .bind(player_id)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected() == 1,
        Err(e) => {
            tracing::error!("invite_auto_decline: decline failed for {}: {}", player_id, e);
            false
        }
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
pub(crate) async fn find_or_create_user_by_email_tx(
    tx: &mut sqlx::PgConnection,
    email: &crate::auth::email_addr::CanonicalEmail,
) -> Result<Uuid, ServerFnError> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN user_emails ue ON u.id = ue.user_id WHERE ue.email = $1",
    )
    .bind(email.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal("resolve invite email: lookup"))?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_user_id = Uuid::new_v4();
    let username = crate::db::generate_unique_username(&mut *tx)
        .await
        .map_err(internal("resolve invite email: gen username"))?;
    sqlx::query("INSERT INTO users (id, name, pref_colors) VALUES ($1,$2,$3)")
        .bind(new_user_id)
        .bind(&username)
        .bind(Vec::<String>::new())
        .execute(&mut *tx)
        .await
        .map_err(internal("resolve invite email: insert user"))?;
    sqlx::query(
        "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1,$2,true,NOW())",
    )
    .bind(new_user_id)
    .bind(email.as_str())
    .execute(&mut *tx)
    .await
    .map_err(internal("resolve invite email: insert email"))?;
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
    proposal: &Proposal,
    players: &[ProposalPlayer],
    game_version: &crate::models::game::GameVersion,
    fetched: crate::game::server_fns::FetchedGame,
) -> Result<Uuid, ServerFnError> {
    use crate::game::server_fns::{BotSlot, CreateGameSeed};

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
    let mut bot_slots: Vec<BotSlot> = accepted
        .iter()
        .filter(|p| p.user_id.is_none())
        .map(|p| BotSlot {
            name: p.bot_name.clone().unwrap_or_default(),
            bot_name: p.bot_difficulty.clone().unwrap_or_default(),
        })
        .collect();

    let canonical_names = crate::db::validate_bot_slots(&mut *tx, &bot_slots)
        .await
        .map_err(internal("start_proposal_tx: validate bot slots"))?
        .map_err(ServerFnError::new)?;
    for (slot, canonical) in bot_slots.iter_mut().zip(canonical_names) {
        slot.bot_name = canonical;
    }

    let game = crate::game::server_fns::insert_game_from_service(
        &mut *tx,
        game_version.id,
        CreateGameSeed {
            creator_id,
            opponent_ids: &opponent_ids,
            opponent_emails: &[],
            bot_slots: &bot_slots,
            all_accepted: true,
        },
        fetched,
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

/// Why a respond_proposal call must be rejected, or None if allowed.
/// The owner can never respond: declining would wedge the proposal
/// (declined is terminal and the owner slot cannot be removed).
#[cfg(feature = "ssr")]
fn respond_denied_reason(is_owner: bool, current: &str, target: &str) -> Option<&'static str> {
    if is_owner {
        return Some("The owner can't respond to their own proposal. Cancel the invite instead.");
    }
    match (current, target) {
        ("pending", "accepted") | ("pending", "declined") | ("accepted", "declined") => None,
        _ => Some(if current == "declined" {
            "You have already declined this invite."
        } else {
            "You have already accepted this invite."
        }),
    }
}

/// Ownership may only move to a human roster member who has accepted:
/// a pending or declined owner could never respond (owners can't respond)
/// and would wedge the proposal permanently.
#[cfg(feature = "ssr")]
fn transfer_target_error(players: &[ProposalPlayer], target_user_id: Uuid) -> Option<&'static str> {
    match players.iter().find(|p| p.user_id == Some(target_user_id)) {
        None => Some("That player isn't in this proposal."),
        Some(p) if p.response != "accepted" => {
            Some("Ownership can only be transferred to a player who has accepted.")
        }
        Some(_) => None,
    }
}

/// Accepted human roster members other than the owner - the notification
/// audience for cancel/start emails.
#[cfg(feature = "ssr")]
fn accepted_invitee_ids(players: &[ProposalPlayer], owner_user_id: Uuid) -> Vec<Uuid> {
    players
        .iter()
        .filter(|p| p.response == "accepted")
        .filter_map(|p| p.user_id)
        .filter(|id| *id != owner_user_id)
        .collect()
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
    use crate::game::server_fns::{
        CreateGameSeed, fetch_game_from_service, insert_game_from_service,
    };
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = crate::friends::require_user().await?;

    let opponent_ids = opponent_ids.unwrap_or_default();
    let opponent_emails = opponent_emails.unwrap_or_default();
    let opponent_emails: Vec<crate::auth::email_addr::CanonicalEmail> = opponent_emails
        .into_iter()
        .map(|e| crate::auth::email_addr::canonicalize_email(&e))
        .collect();
    if opponent_emails
        .iter()
        .any(|e| e.is_empty() || !e.contains('@'))
    {
        return Err(ServerFnError::new("Invalid email address"));
    }
    let mut bot_slots = bot_slots.unwrap_or_default();

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

    let canonical_names = crate::db::validate_bot_slots(&pool, &bot_slots)
        .await
        .map_err(internal("create_proposal: validate bot slots"))?
        .map_err(ServerFnError::new)?;
    for (slot, canonical) in bot_slots.iter_mut().zip(canonical_names) {
        slot.bot_name = canonical;
    }

    let fetched = if opponent_ids.is_empty() && opponent_emails.is_empty() {
        Some(fetch_game_from_service(&http_client, &game_version, player_count).await?)
    } else {
        None
    };

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
        let game = insert_game_from_service(
            &mut tx,
            game_version.id,
            CreateGameSeed {
                creator_id: user.id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &bot_slots,
                all_accepted: false,
            },
            fetched.expect("fetched when no human invitees"),
        )
        .await?;
        tx.commit()
            .await
            .map_err(internal("create_proposal: commit transaction"))?;
        crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game.id).await;
        crate::email::notify::notify_game_emails(
            resend.as_ref(),
            &pool,
            &http_client,
            game.id,
            None,
        )
        .await;
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
pub async fn respond_proposal(proposal_id: Uuid, accept: bool) -> Result<(), ServerFnError> {
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
    if let Some(msg) = respond_denied_reason(
        user.id == proposal.owner_user_id,
        me.response.as_str(),
        target,
    ) {
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
                .ok_or_else(|| ServerFnError::new("Game type not found"))?;
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

    Ok(())
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
    let resend = expect_context::<Option<resend_rs::Resend>>();
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
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
    let count = players.iter().filter(|p| p.response != "declined").count();
    if let Some(msg) = crate::game::server_fns::roster_error(&player_counts, count) {
        return Err(ServerFnError::new(msg));
    }

    let game_version = crate::db::find_game_version(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal: game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let accepted_count = players.iter().filter(|p| p.response == "accepted").count();
    let fetched = crate::game::server_fns::fetch_game_from_service(
        &http_client,
        &game_version,
        accepted_count,
    )
    .await?;

    let game_id = start_proposal_tx(&mut tx, &proposal, &players, &game_version, fetched).await?;

    tx.commit()
        .await
        .map_err(internal("start_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;
    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;
    crate::email::notify::notify_game_emails(resend.as_ref(), &pool, &http_client, game_id, None)
        .await;

    let invitee_ids = accepted_invitee_ids(&players, proposal.owner_user_id);
    mailer().notify_started(proposal_id, game_id, invitee_ids);

    Ok(game_id)
}

/// Canonicalize and validate a proposal invite email before any account is
/// touched. Rejects empty / `@`-less input so a junk address can never mint a
/// verified ghost account (R-07 / F-124 / F-126).
fn validate_proposal_email(
    raw: &str,
) -> Result<crate::auth::email_addr::CanonicalEmail, ServerFnError> {
    let canonical = crate::auth::email_addr::canonicalize_email(raw);
    if canonical.is_empty() || !canonical.contains('@') {
        return Err(ServerFnError::new("Invalid email address"));
    }
    Ok(canonical)
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

    let canonical_email: Option<crate::auth::email_addr::CanonicalEmail> =
        email.as_deref().map(validate_proposal_email).transpose()?;

    let human_id = if let Some(uid) = user_id {
        Some(uid)
    } else if let Some(canonical) = &canonical_email {
        Some(find_or_create_user_by_email_tx(&mut tx, canonical).await?)
    } else {
        None
    };

    if let Some(hid) = human_id {
        let policy_ids: Vec<Uuid> = user_id.into_iter().collect();
        let policy_emails: Vec<crate::auth::email_addr::CanonicalEmail> =
            canonical_email.clone().into_iter().collect();
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
    } else if let Some(mut bot) = bot {
        let canonical_names = crate::db::validate_bot_slots(&mut *tx, std::slice::from_ref(&bot))
            .await
            .map_err(internal("add_proposal_player: validate bot slots"))?
            .map_err(ServerFnError::new)?;
        bot.bot_name = canonical_names.into_iter().next().unwrap();
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
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn cancel_proposal(proposal_id: Uuid) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

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

    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("cancel_proposal: players"))?;

    update_proposal_status(&mut tx, proposal_id, "cancelled", None)
        .await
        .map_err(internal("cancel_proposal: status"))?;

    tx.commit()
        .await
        .map_err(internal("cancel_proposal: commit transaction"))?;

    broadcaster.broadcast_proposal_update(proposal_id).await;
    mailer().notify_cancelled(
        proposal_id,
        accepted_invitee_ids(&players, proposal.owner_user_id),
    );

    Ok(())
}

/// Owner-only: remove any slot (human or bot), allowing invalid player counts;
/// re-normalizes positions and resets accepted humans to pending.
#[server(RemoveProposalSlot, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
pub async fn remove_proposal_slot(proposal_id: Uuid, player_id: Uuid) -> Result<(), ServerFnError> {
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = crate::friends::require_user().await?;

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
    if let Some(msg) = transfer_target_error(&players, target_user_id) {
        return Err(ServerFnError::new(msg));
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
        viewer_user_id: user.id,
    })
}

/// Lists the caller's pending invites.
#[server(GetPendingInvites, "/api")]
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all))]
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
    let add_action = ServerAction::<AddProposalPlayer>::new();
    let transfer_action = ServerAction::<TransferProposalOwnership>::new();
    let start_action = ServerAction::<StartProposal>::new();

    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);
    let bot_names = LocalResource::new(crate::game::server_fns::get_available_bots);

    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(())) = respond_action.value().get() {
            crate::websocket_client::bump_proposal_update(
                proposal_update,
                proposal_id().unwrap_or_default(),
            );
        }
    });

    let nav3 = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(())) = cancel_action.value().get() {
            nav3("/", NavigateOptions::default());
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = remove_action.value().get()
            && let Some(pid) = proposal_id()
        {
            crate::websocket_client::bump_proposal_update(proposal_update, pid);
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = add_action.value().get()
            && let Some(pid) = proposal_id()
        {
            crate::websocket_client::bump_proposal_update(proposal_update, pid);
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = transfer_action.value().get()
            && let Some(pid) = proposal_id()
        {
            crate::websocket_client::bump_proposal_update(proposal_update, pid);
        }
    });

    let nav_start = navigate.clone();
    Effect::new(move |_| {
        if let Some(Ok(gid)) = start_action.value().get() {
            nav_start(&format!("/games/{}", gid), NavigateOptions::default());
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
                            add_action=add_action
                            transfer_action=transfer_action
                            start_action=start_action
                            suggestions=suggestions
                            bot_names=bot_names
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
    add_action: ServerAction<AddProposalPlayer>,
    transfer_action: ServerAction<TransferProposalOwnership>,
    start_action: ServerAction<StartProposal>,
    suggestions: LocalResource<Result<Vec<crate::friends::OpponentSuggestion>, ServerFnError>>,
    bot_names: LocalResource<Result<Vec<String>, ServerFnError>>,
) -> impl IntoView {
    let is_open = pv.proposal.status == "open";
    let viewer_role = pv.viewer_role.clone();
    let proposal_id = pv.proposal.id;
    let owner_user_id = pv.proposal.owner_user_id;
    let game_type_name = pv.game_type_name.clone();
    let version_name = pv.version_name.clone();
    let player_counts = pv.player_counts.clone();

    let is_owner = viewer_role == ViewerRole::Owner;
    let is_invitee = viewer_role == ViewerRole::Invitee;

    let my_response = if is_invitee {
        pv.players
            .iter()
            .find(|p| p.user_id == Some(pv.viewer_user_id))
            .map(|p| p.response.clone())
    } else {
        None
    };

    let prospective_count = pv
        .players
        .iter()
        .filter(|p| p.response != "declined")
        .count();
    let count_invalid = !player_counts.contains(&(prospective_count as i32));
    let count_warning = StoredValue::new(if count_invalid {
        let counts = player_counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "This game supports {counts} players, but the roster has {prospective_count}"
        ))
    } else {
        None
    });

    let (add_slot, set_add_slot) = signal(OpponentSlot::default());
    let roster_user_ids: Vec<Uuid> = pv.players.iter().filter_map(|p| p.user_id).collect();
    let taken = Signal::derive(move || roster_user_ids.clone());

    let players_for_rows = StoredValue::new(pv.players.clone());

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
            {players_for_rows.with_value(|players| {
                players.iter().map(|p| {
                    let name = p.name.clone();
                    let response = p.response.clone();
                    let is_bot = p.user_id.is_none();
                    let status_class = format!("invite-status invite-status-{}", response);
                    let pid = p.id;
                    let p_uid = p.user_id;
                    let is_owner_row = p_uid == Some(owner_user_id);
                    let show_remove = is_owner && !is_owner_row;
                    let show_make_owner =
                        is_owner && !is_bot && !is_owner_row && response == "accepted";
                    let remove_name = p.name.clone();
                    let transfer_name = p.name.clone();
                    view! {
                        <div class="friend-row">
                            <span>{name}</span>
                            {is_bot.then(|| view! { <span>" (bot)"</span> })}
                            " - "
                            <span class=status_class>{response.clone()}</span>
                            {show_remove.then(|| {
                                let rn = remove_name.clone();
                                view! {
                                    " "
                                    <a href="#" on:click=move |ev| {
                                        ev.prevent_default();
                                        if crate::components::confirm(&format!("Remove {rn} from the game?")) {
                                            remove_action.dispatch(RemoveProposalSlot {
                                                proposal_id,
                                                player_id: pid,
                                            });
                                        }
                                    }>"(X)"</a>
                                }
                            })}
                            {show_make_owner.then(|| {
                                let tn = transfer_name.clone();
                                let uid = p_uid.unwrap_or_default();
                                view! {
                                    " "
                                    <a href="#" on:click=move |ev| {
                                        ev.prevent_default();
                                        if crate::components::confirm(&format!("Transfer ownership to {tn}?")) {
                                            transfer_action.dispatch(TransferProposalOwnership {
                                                proposal_id,
                                                target_user_id: uid,
                                            });
                                        }
                                    }>"(make owner)"</a>
                                }
                            })}
                        </div>
                    }
                }).collect_view()
            })}
        </section>

        <Show when=move || is_invitee && is_open>
            <section>
                <h2>"Your invite"</h2>
                {match my_response.as_deref() {
                    Some("pending") => view! {
                        <div class="form-actions">
                            <a href="#" on:click=move |ev| {
                                ev.prevent_default();
                                respond_action.dispatch(RespondProposal { proposal_id, accept: true });
                            }>"Accept"</a>
                            " | "
                            <a href="#" on:click=move |ev| {
                                ev.prevent_default();
                                if crate::components::confirm("Decline this invite?") {
                                    respond_action.dispatch(RespondProposal { proposal_id, accept: false });
                                }
                            }>"Decline"</a>
                        </div>
                    }.into_any(),
                    Some("accepted") => view! {
                        <div class="form-actions">
                            <span>"You accepted this invite."</span>
                            " "
                            <a href="#" on:click=move |ev| {
                                ev.prevent_default();
                                if crate::components::confirm("Decline this invite? You will need to be re-invited to join.") {
                                    respond_action.dispatch(RespondProposal { proposal_id, accept: false });
                                }
                            }>"Decline"</a>
                        </div>
                    }.into_any(),
                    Some("declined") => view! {
                        <p>"You declined this invite."</p>
                    }.into_any(),
                    _ => ().into_any(),
                }}
            </section>
        </Show>

        <Show when=move || is_owner && is_open>
            <section>
                <h2>"Owner actions"</h2>

                <div>
                    <h3>"Add player"</h3>
                    <OpponentSlotEditor
                        label="New player".to_string()
                        radio_group="add-player-mode".to_string()
                        bot_default_name="Bot".to_string()
                        get=add_slot.into()
                        set=Callback::new(move |s: OpponentSlot| set_add_slot.set(s))
                        taken=taken
                        suggestions=suggestions
                        bot_names=bot_names
                    />
                    <div class="form-actions">
                        <button type="button" on:click=move |_| {
                            let slot = add_slot.get_untracked();
                            let (user_id, email, bot) = match slot {
                                OpponentSlot::Player { selected: Some((id, _)), .. } => {
                                    (Some(id), None, None)
                                }
                                OpponentSlot::Email(e) if !e.is_empty() => {
                                    (None, Some(e), None)
                                }
                                OpponentSlot::Bot { name, bot_name } => {
                                    (None, None, Some(crate::game::server_fns::BotSlot { name, bot_name }))
                                }
                                _ => return,
                            };
                            add_action.dispatch(AddProposalPlayer {
                                proposal_id,
                                user_id,
                                email,
                                bot,
                            });
                            set_add_slot.set(OpponentSlot::default());
                        }>"Add player"</button>
                    </div>
                </div>

                <div class="form-actions">
                    <button
                        type="button"
                        disabled=move || start_action.pending().get()
                        on:click=move |_| {
                            start_action.dispatch(StartProposal { proposal_id });
                        }
                    >"Start game"</button>
                    <Show when=move || start_action.pending().get()>
                        <crate::components::Spinner/>
                    </Show>
                    " "
                    <a href="#" on:click=move |ev| {
                        ev.prevent_default();
                        if crate::components::confirm("Cancel this game invite?") {
                            cancel_action.dispatch(CancelProposal { proposal_id });
                        }
                    }>"Cancel invite"</a>
                </div>
                {count_warning.with_value(|w| w.clone().map(|msg| view! { <div class="form-error">{msg}</div> }))}
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
        <Show when=move || {
            add_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || add_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || {
            transfer_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || transfer_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
            </div>
        </Show>
        <Show when=move || {
            start_action.value().get().is_some_and(|r| r.is_err())
        }>
            <div class="form-error">
                {move || start_action.value().get().and_then(|r| r.err()).map(|e| e.to_string()).unwrap_or_default()}
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
    async fn sweep_candidate_queries_match_backdated_proposals(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
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
            Some("tok-sweep".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        sqlx::query(
            "UPDATE game_proposals SET created_at = created_at - interval '1 hour' WHERE id = $1",
        )
        .bind(pid)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE game_proposal_players SET updated_at = updated_at - interval '1 hour' WHERE proposal_id = $1",
        )
        .bind(pid)
        .execute(&pool)
        .await
        .unwrap();

        // 60s threshold: the 1h-old proposal is a candidate everywhere.
        assert!(
            fetch_nudge_candidates(&pool, 60)
                .await
                .iter()
                .any(|c| c.proposal_id == pid && c.user_id == a),
            "nudge query must return the backdated pending invitee"
        );
        assert!(
            fetch_expiry_candidates(&pool, 60).await.contains(&pid),
            "expiry query must return the backdated proposal"
        );
        assert!(
            fetch_auto_decline_candidates(&pool, 60)
                .await
                .iter()
                .any(|(_, p, _)| *p == pid),
            "auto-decline query must return the backdated pending slot"
        );

        // 2h threshold: nothing qualifies.
        assert!(fetch_nudge_candidates(&pool, 7200).await.is_empty());
        assert!(fetch_expiry_candidates(&pool, 7200).await.is_empty());
        assert!(fetch_auto_decline_candidates(&pool, 7200).await.is_empty());
    }

    // wfe F34: auto-decline reports a real transition exactly once. The first
    // call flips pending -> declined (true); the second finds no pending row
    // (false), so the sweep only marks/notifies on the real transition (D-02).
    #[sqlx::test]
    async fn auto_decline_proposal_player_transitions_once(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let player_id = insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(a),
            None,
            None,
            "pending",
            Some("tok-f34".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert!(
            auto_decline_proposal_player(&pool, player_id).await,
            "first decline of a pending row must report a real transition"
        );
        assert!(
            !auto_decline_proposal_player(&pool, player_id).await,
            "second decline of an already-declined row must report no transition"
        );
    }

    // wd F28: the auto-decline window keys on pp.updated_at, NOT gp.created_at.
    // A proposal whose created_at is backdated past the threshold but with a
    // freshly-added (fresh pp.updated_at) pending player is NOT a candidate;
    // once pp.updated_at is backdated past the threshold it BECOMES one.
    #[sqlx::test]
    async fn auto_decline_keys_on_player_updated_at_not_proposal_created_at(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        let player_id = insert_proposal_player(
            &mut tx,
            pid,
            1,
            Some(a),
            None,
            None,
            "pending",
            Some("tok-f28".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        sqlx::query(
            "UPDATE game_proposals SET created_at = created_at - interval '1 hour' WHERE id = $1",
        )
        .bind(pid)
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            !fetch_auto_decline_candidates(&pool, 60)
                .await
                .iter()
                .any(|(pp_id, _, _)| *pp_id == player_id),
            "a fresh pp.updated_at must NOT be a candidate even when gp.created_at is old"
        );

        sqlx::query(
            "UPDATE game_proposal_players SET updated_at = updated_at - interval '1 hour' WHERE id = $1",
        )
        .bind(player_id)
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            fetch_auto_decline_candidates(&pool, 60)
                .await
                .iter()
                .any(|(pp_id, _, _)| *pp_id == player_id),
            "an old pp.updated_at must be a candidate"
        );
    }

    // wd F38: the invite-send core returns `true` (permanently unsendable, no
    // send) for a CANCELLED proposal and for a STALE/rotated token. With
    // `resend = None` there is no outbox, so the `true` return is the assertion.
    #[sqlx::test]
    async fn send_invite_core_permanent_skip_cancelled_and_stale_token(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
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
            Some("tok-f38".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mailer = mailer_from(pool.clone(), None);

        // (a) CANCELLED proposal => permanent skip.
        sqlx::query("UPDATE game_proposals SET status = 'cancelled' WHERE id = $1")
            .bind(pid)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            mailer.send_invite_now(pid, a, Some("tok-f38".into())).await,
            "a cancelled proposal must be a permanent skip (true)"
        );

        // Restore to open so only the token is stale.
        sqlx::query("UPDATE game_proposals SET status = 'open' WHERE id = $1")
            .bind(pid)
            .execute(&pool)
            .await
            .unwrap();

        // (b) STALE token (matches no pending row) => permanent skip.
        assert!(
            mailer
                .send_invite_now(pid, a, Some("stale-rotated-token".into()))
                .await,
            "a stale/rotated token must be a permanent skip (true)"
        );
    }

    #[sqlx::test]
    async fn roster_view_never_exposes_email_token(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let a = seed_invite_user(&pool, true).await;
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
            Some("secret-token-do-not-leak".into()),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let roster = find_proposal_roster(&pool, pid).await.unwrap();
        assert_eq!(roster.len(), 2);
        let json = serde_json::to_string(&roster).unwrap();
        assert!(
            !json.contains("email_token"),
            "email_token field must not be serialized: {json}"
        );
        assert!(
            !json.contains("secret-token-do-not-leak"),
            "token value must not appear in serialized roster: {json}"
        );
    }

    #[test]
    fn respond_denied_reason_blocks_owner_and_bad_transitions() {
        // Owner is always rejected, regardless of state.
        assert!(respond_denied_reason(true, "accepted", "declined").is_some());
        assert!(respond_denied_reason(true, "pending", "accepted").is_some());
        // Invitee transitions unchanged.
        assert!(respond_denied_reason(false, "pending", "accepted").is_none());
        assert!(respond_denied_reason(false, "pending", "declined").is_none());
        assert!(respond_denied_reason(false, "accepted", "declined").is_none());
        assert_eq!(
            respond_denied_reason(false, "declined", "accepted"),
            Some("You have already declined this invite.")
        );
        assert_eq!(
            respond_denied_reason(false, "accepted", "accepted"),
            Some("You have already accepted this invite.")
        );
    }

    #[test]
    fn transfer_target_must_be_accepted_human() {
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
        let accepted = Uuid::new_v4();
        let pending = Uuid::new_v4();
        let declined = Uuid::new_v4();
        let players = vec![
            mk(Some(accepted), "accepted"),
            mk(Some(pending), "pending"),
            mk(Some(declined), "declined"),
            mk(None, "accepted"), // bot
        ];
        assert!(transfer_target_error(&players, accepted).is_none());
        assert!(transfer_target_error(&players, pending).is_some());
        assert!(transfer_target_error(&players, declined).is_some());
        assert!(transfer_target_error(&players, Uuid::new_v4()).is_some());
    }

    #[test]
    fn accepted_invitee_ids_excludes_owner_bots_and_nonaccepted() {
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
        let a = Uuid::new_v4();
        let players = vec![
            mk(Some(owner), "accepted"),
            mk(Some(a), "accepted"),
            mk(Some(Uuid::new_v4()), "pending"),
            mk(Some(Uuid::new_v4()), "declined"),
            mk(None, "accepted"), // bot
        ];
        assert_eq!(accepted_invitee_ids(&players, owner), vec![a]);
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

    #[sqlx::test]
    async fn start_proposal_tx_rejects_disabled_bot(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;

        sqlx::query(
            "INSERT INTO game_proposal_players (proposal_id, position, user_id, response) VALUES ($1, 0, $2, 'accepted')",
        )
        .bind(pid)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO game_proposal_players (proposal_id, position, bot_name, bot_difficulty, response) VALUES ($1, 1, 'Bot 1', 'easy', 'accepted')",
        )
        .bind(pid)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("UPDATE bots SET enabled = false WHERE name = 'easy'")
            .execute(&pool)
            .await
            .unwrap();

        let proposal = find_proposal(&pool, pid).await.unwrap().unwrap();
        let players = find_proposal_players(&pool, pid).await.unwrap();
        let game_version = crate::db::find_game_version(&pool, gv)
            .await
            .unwrap()
            .unwrap();

        let fetched = crate::game::server_fns::FetchedGame {
            game_info: brdgme_cmd::api::GameResponse {
                state: String::new(),
                points: vec![0.0, 0.0],
                status: brdgme_game::Status::Active {
                    whose_turn: vec![0],
                    eliminated: vec![],
                },
            },
            logs: vec![],
        };

        let mut tx = pool.begin().await.unwrap();
        let result = start_proposal_tx(&mut tx, &proposal, &players, &game_version, fetched).await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("easy"),
            "error should mention the invalid bot: {err_msg}"
        );
    }

    async fn proposal_status(pool: &PgPool, proposal_id: Uuid) -> String {
        sqlx::query_scalar("SELECT status FROM game_proposals WHERE id = $1")
            .bind(proposal_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn cancel_proposal_for_expiry_reads_roster_then_cancels(pool: PgPool) {
        let gv = seed_game_version(&pool).await;
        let owner = seed_invite_user(&pool, true).await;
        let invitee = seed_invite_user(&pool, true).await;
        let pid = seed_proposal(&pool, gv, owner).await;
        let mut tx = pool.begin().await.unwrap();
        insert_proposal_player(&mut tx, pid, 0, Some(owner), None, None, "accepted", None)
            .await
            .unwrap();
        insert_proposal_player(&mut tx, pid, 1, Some(invitee), None, None, "accepted", None)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let outcome = cancel_proposal_for_expiry(&pool, pid).await;
        let (owner_id, accepted_ids) = outcome.expect("open proposal should cancel");
        assert_eq!(owner_id, owner);
        assert!(
            accepted_ids.contains(&invitee),
            "accepted ids should contain the non-owner invitee"
        );
        assert!(
            !accepted_ids.contains(&owner),
            "accepted ids should not contain the owner"
        );
        assert_eq!(proposal_status(&pool, pid).await, "cancelled");

        assert!(
            cancel_proposal_for_expiry(&pool, pid).await.is_none(),
            "an already-cancelled proposal returns None"
        );
        assert_eq!(
            proposal_status(&pool, pid).await,
            "cancelled",
            "status stays cancelled and nothing changes"
        );
    }

    // wd F34: a failed game-type lookup must not produce a blank substitution
    // (" invite from Alice"); the mail still goes out with a generic label.
    #[sqlx::test]
    async fn proposal_game_type_name_falls_back_to_a_label(pool: PgPool) {
        let midnight = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );
        let proposal = Proposal {
            id: Uuid::new_v4(),
            created_at: midnight,
            updated_at: midnight,
            // No such game_version row: find_game_version returns Ok(None).
            game_version_id: Uuid::new_v4(),
            owner_user_id: Uuid::new_v4(),
            restarted_game_id: None,
            status: "open".to_string(),
            started_game_id: None,
            nudged_at: None,
        };
        assert_eq!(
            proposal_game_type_name(&pool, &proposal).await,
            UNKNOWN_GAME_TYPE_NAME
        );
    }

    #[sqlx::test]
    async fn invite_canonical_email_resolves_to_existing_user(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
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
        .bind(user_id)
        .bind("foo@x.com")
        .execute(&pool)
        .await
        .unwrap();
        let users_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let resolved = find_or_create_user_by_email_tx(
            &mut tx,
            &crate::auth::email_addr::canonicalize_email("Foo@x.com "),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(
            resolved, user_id,
            "case/space variant resolves to the owner"
        );
        let users_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(users_after, users_before, "no second user created");
    }

    #[test]
    fn validate_proposal_email_rejects_empty_and_atless() {
        assert!(validate_proposal_email("").is_err());
        assert!(validate_proposal_email("   ").is_err());
        assert!(validate_proposal_email("no-at-sign").is_err());
        assert_eq!(
            validate_proposal_email(" Foo@x.com ").unwrap().as_str(),
            "foo@x.com"
        );
        assert_eq!(
            validate_proposal_email("BAR@x.com").unwrap().as_str(),
            "bar@x.com"
        );
    }

    #[sqlx::test]
    async fn add_proposal_player_email_creates_no_ghost_account(pool: PgPool) {
        let foo_owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("foo_owner")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(foo_owner_id)
        .bind("foo@x.com")
        .execute(&pool)
        .await
        .unwrap();
        let bar_owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("bar_owner")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(bar_owner_id)
        .bind("bar@x.com")
        .execute(&pool)
        .await
        .unwrap();

        let users_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        let noncanon_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email <> lower(btrim(email))",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let c = validate_proposal_email(" Foo@x.com ").unwrap();
        let mut tx = pool.begin().await.unwrap();
        let resolved = find_or_create_user_by_email_tx(&mut tx, &c).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(resolved, foo_owner_id, "raw variant resolves to the owner");

        let c = validate_proposal_email("BAR@x.com").unwrap();
        let mut tx = pool.begin().await.unwrap();
        let resolved = find_or_create_user_by_email_tx(&mut tx, &c).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            resolved, bar_owner_id,
            "non-canonical variant resolves to the owner"
        );

        assert!(validate_proposal_email("").is_err());

        let noncanon_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email <> lower(btrim(email))",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(noncanon_after, noncanon_before);
        let users_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(users_after, users_before);
    }

    /// Runs `f` with the full context the `#[server] add_proposal_player` fn
    /// pulls via `expect_context` / `require_user`: the `PgPool`, a NATS-backed
    /// `GameBroadcaster`, the `Option<Resend>` mailer context, and a
    /// tower-sessions `Session` (placed in the `axum` request `Parts` that
    /// `leptos_axum::extract` reads) logged in as `session_user`. This lets a
    /// `#[sqlx::test]` call the literal server fn rather than a helper.
    async fn with_logged_in_context<F, Fut, T>(
        pool: &PgPool,
        session_user: crate::auth::session::SessionUser,
        f: F,
    ) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        use crate::auth::session::SESSION_USER_KEY;
        use crate::websocket::GameBroadcaster;
        use leptos::reactive::owner::Owner;
        use std::sync::Arc;
        use tower_sessions::{MemoryStore, Session};

        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        session.insert(SESSION_USER_KEY, session_user).await.unwrap();
        let (mut parts, _) = axum::http::Request::new(()).into_parts();
        parts.extensions.insert(session);

        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let client = async_nats::connect(&nats_url).await.unwrap();
        let broadcaster = GameBroadcaster::new(client);

        let owner = Owner::new();
        owner.with(|| {
            provide_context(pool.clone());
            provide_context(broadcaster);
            provide_context(None::<resend_rs::Resend>);
            provide_context(parts);
        });
        owner
            .with(|| leptos::reactive::computed::ScopedFuture::new(f()))
            .await
    }

    /// AC3 (literal): drive the actual `#[server] add_proposal_player` with raw,
    /// non-canonical, and empty email inputs and prove no verified ghost account
    /// is minted for any of them.
    #[sqlx::test]
    async fn add_proposal_player_server_fn_creates_no_ghost_account(pool: PgPool) {
        // The logged-in proposal owner (the session user).
        let owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("proposal_owner")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(owner_id)
        .bind("owner@x.com")
        .execute(&pool)
        .await
        .unwrap();
        let auth_token_id = Uuid::new_v4();
        sqlx::query("INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)")
            .bind(auth_token_id)
            .bind(owner_id)
            .execute(&pool)
            .await
            .unwrap();

        // Pre-existing canonical accounts the invites must resolve to.
        let foo_owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("foo_owner")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(foo_owner_id)
        .bind("foo@x.com")
        .execute(&pool)
        .await
        .unwrap();
        let bar_owner_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind("bar_owner")
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
             VALUES ($1, $2, true, NOW())",
        )
        .bind(bar_owner_id)
        .bind("bar@x.com")
        .execute(&pool)
        .await
        .unwrap();

        let gv = seed_game_version(&pool).await;
        let proposal_id = seed_proposal(&pool, gv, owner_id).await;

        let users_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        let noncanon_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email <> lower(btrim(email))",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let session_user = crate::auth::session::SessionUser {
            id: owner_id,
            name: "proposal_owner".to_string(),
            email: "owner@x.com".to_string(),
            auth_token_id,
        };

        // Raw input (surrounding spaces + mixed case) resolves to the existing
        // owner instead of minting a ghost account.
        with_logged_in_context(&pool, session_user.clone(), || {
            add_proposal_player(proposal_id, None, Some(" Foo@x.com ".to_string()), None)
        })
        .await
        .unwrap();

        // A non-canonical case variant resolves to the existing owner too.
        with_logged_in_context(&pool, session_user.clone(), || {
            add_proposal_player(proposal_id, None, Some("BAR@x.com".to_string()), None)
        })
        .await
        .unwrap();

        // Empty and whitespace-only inputs are rejected before any account is
        // touched.
        assert!(
            with_logged_in_context(&pool, session_user.clone(), || {
                add_proposal_player(proposal_id, None, Some(String::new()), None)
            })
            .await
            .is_err(),
            "empty email must be rejected"
        );
        assert!(
            with_logged_in_context(&pool, session_user.clone(), || {
                add_proposal_player(proposal_id, None, Some("   ".to_string()), None)
            })
            .await
            .is_err(),
            "whitespace-only email must be rejected"
        );

        let users_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(users_after, users_before, "no ghost account may be minted");

        let noncanon_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email <> lower(btrim(email))",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            noncanon_after, noncanon_before,
            "no non-canonical email row may be stored"
        );

        // The invites resolved to the pre-existing owners (exactly one verified
        // row each), not to newly created verified accounts.
        let foo_verified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email = 'foo@x.com' AND verified_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(foo_verified, 1, "foo@x.com stays a single verified row");
        let bar_verified: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_emails WHERE email = 'bar@x.com' AND verified_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(bar_verified, 1, "bar@x.com stays a single verified row");
    }
}
