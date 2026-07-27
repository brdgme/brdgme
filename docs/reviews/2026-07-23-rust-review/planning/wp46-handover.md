# WP-46 Handover - sweep delivery semantics (survey of live code)

Surveyed at HEAD `914aa0c` (WP-38). All line numbers below are measured from the
live tree at survey time; navigate by named symbol, not line number.

---

## 1. Git state summary

- `git status --short`:
  - ` M docs/reviews/2026-07-23-rust-review/planning/EXECUTION-STATE.md`
  - `?? docs/reviews/2026-07-23-rust-review/planning/wp38-handover.md`
- **`rust/` tree is CLEAN** (`git diff --stat -- rust/` produced no output). The
  only working-tree changes are docs under the review planning dir.
- **WP-38 IS present**: `914aa0c fix(web): bot-turn wedge recovery -
  reconciliation sweep, ack/term semantics, ack heartbeat (WP-38)` is HEAD.
- Other recent relevant commits: `4d31f6e refactor(web): split db.rs into a
  module (WP-82)` (so there is **no `rust/web/src/db.rs` file** - it is now
  `rust/web/src/db/` with submodules), `da1ea24` (WP-56 email From-auth),
  `f56ff37` (WP-59 inbound).
- **Highest migration: `023_settings_email_token.sql`** (matches expectation).
  Migrations dir runs 001..023, contiguous.

---

## 2. STOP-AND-REPORT triggers

### TRIGGER 1 (count mismatch) - the "five unit tests to delete" is actually SEVEN

The spec says "identify the five unit tests that exercise
`is_reminder_candidate`/`should_reset_reminder` (these are to be deleted)".
A whole-tree grep (`is_reminder_candidate|should_reset_reminder`) shows these
two fns are referenced ONLY inside `sweep.rs` (definitions + tests). The pure
`#[test]` unit tests exercising them number **seven**, not five:

Exercising `should_reset_reminder` (1):
1. `should_reset_reminder_on_transition` (sweep.rs:509)

Exercising `is_reminder_candidate` (6):
2. `candidate_predicate_accepts_due_player` (sweep.rs:517)
3. `candidate_predicate_rejects_already_reminded` (sweep.rs:531)
4. `candidate_predicate_rejects_not_turn` (sweep.rs:545)
5. `candidate_predicate_rejects_eliminated` (sweep.rs:559)
6. `candidate_predicate_rejects_below_threshold` (sweep.rs:573)
7. `candidate_predicate_boundary_exact_threshold` (sweep.rs:587)

All seven are pure `#[test]` (non-DB) unit tests. The DB-backed `#[sqlx::test]`
cases (`fetch_candidates_*`, `mark_reminder_sent_*`, `reset_reminder_*`, etc.)
do NOT call these fns and are separate. **Executor: confirm with the spec author
whether all seven go, or whether `should_reset_reminder_on_transition` and/or
the boundary test are meant to survive.** The spec's "five" undercounts either
way.

### Resolved ambiguities (NOT triggers - recorded so the executor doesn't re-litigate)

- **`fetch_auto_decline_candidates` lives in `proposals.rs`, not `sweep.rs`.**
  The sweep.rs section of the spec asked to verify it there, but spec section 3d
  already noted proposals.rs; the live call site is
  `crate::proposals::fetch_auto_decline_candidates` (sweep.rs:450). Confirmed in
  `rust/web/src/proposals.rs:781`.
- **`db.rs` was split into a module (WP-82).** `crate::db::foo` paths are
  unchanged via re-exports; `delete_expired_unverified_emails` lives in
  `rust/web/src/db/emails.rs`.

### "Differs from spec" items that are the WORK TO BE DONE (expected, not triggers)

These current-state mismatches are exactly what WP-46 implements; flagging only
so they aren't mistaken for survey errors:
- `EmailRecipient` has NO `reminder_emails_enabled` field yet (WP-46 adds it).
- `fetch_email_recipient` SQL has NO `COALESCE(u.reminder_emails_enabled,
  false)` yet (WP-46 adds it).
- `fetch_auto_decline_candidates` returns `(pp.id, pp.proposal_id)` only; it
  does NOT yet return `pp.user_id` (WP-46 adds it).
- `auto_decline_proposal_player` returns `()` (unit), NOT `bool` (WP-46 changes
  it to `rows_affected() == 1`).
- `sweep_invite_auto_decline_once` / `spawn_invite_auto_decline_sweep` do NOT
  take a `resend` param today (WP-46 adds mailing => needs `resend`).

Everything else matched the spec's description of the current state - see §3.

---

## 3. Verified symbol locations (verbatim)

### rust/web/src/email/sweep.rs

`fetch_candidates` (sweep.rs:56) - MATCHES F31/F37/F40 (FOR UPDATE SKIP LOCKED,
autocommit `.fetch_all(pool)`, no LIMIT, excludes bots, filters
`reminder_emails_enabled`):
```rust
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
```

`send_reminder` (sweep.rs:98) - MATCHES F30/F32 (returns `bool`; `true` on both
suppression paths; gates on `should_email_recipient`). Full body:
```rust
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

    if !crate::email::outbound::should_email_recipient(&recipient) {
        return true;
    }
    if crate::email::outbound::suppress_for_web_presence(pool, recipient.user_id).await {
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
```

`sweep_once` (sweep.rs:195) - MATCHES (treats `true` as sent, calls
`mark_reminder_sent`):
```rust
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
```

`mark_reminder_sent` (sweep.rs:82):
```rust
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
```

`is_reminder_candidate` (sweep.rs:31) and `should_reset_reminder` (sweep.rs:27)
- MATCH F37 (Rust copy lacks `game_bot_id IS NULL` and `reminder_emails_enabled`;
referenced ONLY by sweep.rs tests, confirmed by whole-tree grep):
```rust
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
```

`sweep_invite_nudge_once` (sweep.rs:379) - MATCHES F33 (fire-and-forget
`send_invite`, then unconditional `mark_proposal_nudged`):
```rust
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
```

`sweep_invite_auto_decline_once` (sweep.rs:444) - MATCHES F34 (flips rows +
broadcasts, never mails). **Does NOT take `resend` today** (signature is
`(pool, broadcaster)`):
```rust
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
```

`sweep_invite_expiry_once` (sweep.rs:412) - MATCHES (reaches
`cancel_proposal_for_expiry`):
```rust
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
```

`sweep_unverified_emails_once` (sweep.rs:326) - pattern to mirror for F11:
```rust
async fn sweep_unverified_emails_once(pool: &PgPool) {
    match crate::db::delete_expired_unverified_emails(pool, UNVERIFIED_EMAIL_EXPIRY).await {
        Ok(0) => {}
        Ok(n) => tracing::info!("unverified_email_expiry: deleted {} row(s)", n),
        Err(e) => tracing::error!("unverified_email_expiry: delete failed: {}", e),
    }
}
```

`spawn_periodic_sweeps` (sweep.rs:481) - **MATCHES post-WP-38 signature
`(pool, resend, http_client, broadcaster, jetstream)`**; spawns SIX sweeps:
```rust
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
    spawn_invite_auto_decline_sweep(pool.clone(), broadcaster);
    spawn_bot_turn_sweep(pool.clone(), jetstream);
}
```

Spawn fn signatures:
```rust
pub fn spawn_turn_reminder_sweep(pool: PgPool, resend: Option<resend_rs::Resend>, http_client: reqwest::Client)   // sweep.rs:214
pub fn spawn_unverified_email_sweep(pool: PgPool)                                                                // sweep.rs:336
pub fn spawn_invite_nudge_sweep(pool: PgPool, resend: Option<resend_rs::Resend>)                                 // sweep.rs:399
pub fn spawn_invite_expiry_sweep(pool: PgPool, resend: Option<resend_rs::Resend>)                                // sweep.rs:431
pub fn spawn_invite_auto_decline_sweep(pool: PgPool, broadcaster: crate::websocket::GameBroadcaster)             // sweep.rs:465  <- NO resend today
pub fn spawn_bot_turn_sweep(pool: PgPool, jetstream: async_nats::jetstream::Context)                             // sweep.rs:309
```

Consts (all present):
```rust
pub const DEFAULT_REMINDER_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(86400);   // sweep.rs:9
pub const DEFAULT_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(900);         // sweep.rs:11
pub const DEFAULT_BOT_TURN_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(900);     // sweep.rs:231  (WP-38)
pub const DEFAULT_BOT_TURN_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(900);// sweep.rs:233  (WP-38)
pub const UNVERIFIED_EMAIL_EXPIRY: std::time::Duration = std::time::Duration::from_secs(86400);      // sweep.rs:324
pub const DEFAULT_INVITE_REMINDER_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(86400);   // sweep.rs:349
pub const DEFAULT_INVITE_EXPIRY_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(1209600);   // sweep.rs:352
pub const DEFAULT_INVITE_AUTO_DECLINE_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(172800); // sweep.rs:369
```

WP-38 additions ALL confirmed present:
- `bot_turn_threshold()` (sweep.rs:236), `bot_turn_sweep_interval()` (sweep.rs:243)
- `fetch_bot_turn_candidates(pool, threshold) -> Vec<BotTurnCandidate>` (sweep.rs:258)
- `sweep_bot_turns_once(pool, jetstream)` (sweep.rs:289)
- `spawn_bot_turn_sweep(pool, jetstream)` (sweep.rs:309)
- one spawn line in `spawn_periodic_sweeps` (sweep.rs:493)

Tests: `#[cfg(all(test, feature = "ssr"))] mod tests` EXISTS (sweep.rs:496).
Confirmed helpers/tests present:
- helper `seed_reminder_game` (sweep.rs:1036) - EXISTS
- `turn_reminder_suppressed_by_recipient_presence` (sweep.rs:1099) - EXISTS
- `fetch_candidates_excludes_reminder_disabled` (sweep.rs:746) - EXISTS
- `sweep_unverified_emails_deletes_expired_only` (sweep.rs:938) - EXISTS

Full test fn list in sweep.rs mod tests (for reference):
`should_reset_reminder_on_transition`, `candidate_predicate_accepts_due_player`,
`candidate_predicate_rejects_already_reminded`, `candidate_predicate_rejects_not_turn`,
`candidate_predicate_rejects_eliminated`, `candidate_predicate_rejects_below_threshold`,
`candidate_predicate_boundary_exact_threshold`, `reminder_threshold_defaults_to_24h`,
`reminder_threshold_parses_custom`, `sweep_interval_defaults_to_15m`,
`sweep_interval_parses_custom`, `reminder_header_contains_name`,
`fetch_candidates_returns_due_players`, `fetch_candidates_excludes_reminded`,
`fetch_candidates_excludes_reminder_disabled`, `mark_reminder_sent_sets_timestamp`,
`reset_reminder_clears_timestamp`, `unverified_email_expiry_is_24h`,
`sweep_unverified_emails_deletes_expired_only`, `invite_reminder_threshold_defaults_to_24h`,
`invite_reminder_threshold_parses_custom`, `invite_expiry_threshold_defaults_to_14_days`,
`invite_expiry_threshold_parses_custom`, `invite_auto_decline_threshold_defaults_to_48h`,
`invite_auto_decline_threshold_parses_custom`, `turn_reminder_suppressed_by_recipient_presence`,
`bot_turn_candidates_exclude_human_players`, `bot_turn_candidates_exclude_finished_games`,
`bot_turn_candidates_exclude_recent_turns`, `bot_turn_candidates_partition_live_and_dangling`.
Helpers: `fixed_now`, `seed_reminder_game`, `seed_bot_sweep_game`, `seed_bot_type`,
`seed_bot_player`, `seed_human_player`.

### rust/web/src/proposals.rs  (top-level module `crate::proposals`, NOT under email/)

`cancel_proposal_for_expiry` (proposals.rs:743) - MATCHES F35 (commits
`status='cancelled'` THEN reads owner via `.ok().flatten()` + `owner?`, accepted
ids via `.unwrap_or_default()`). Reached ONLY from `sweep_invite_expiry_once`
(grep: sole call site sweep.rs:423):
```rust
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
```

`fetch_auto_decline_candidates` (proposals.rs:781) - keys window on
`gp.created_at` (YES, wd F28); returns `(pp.id, pp.proposal_id)`; does NOT yet
return `pp.user_id` (WP-46 adds it):
```rust
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
           AND gp.created_at < NOW() - ($1 * interval '1 second')",
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
```

`auto_decline_proposal_player` (proposals.rs:807) - **current return type is
`()` (unit)**; WP-46 wants `bool` from `rows_affected() == 1`:
```rust
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
```

`mailer_from` (proposals.rs:468):
```rust
#[cfg(feature = "ssr")]
pub(crate) fn mailer_from(pool: PgPool, resend: Option<resend_rs::Resend>) -> RealInviteMailer {
    RealInviteMailer { pool, resend }
}
```
(There is also `mailer()` at proposals.rs:460 using `expect_context`.)

`InviteMailer` trait (proposals.rs:102). Note `notify_owner_decline`'s second
param is named `invitee_user_id` (spec called it `declined_user_id`); shape
(two `Uuid`s) matches:
```rust
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
```

`RealInviteMailer::send_invite` (proposals.rs:174) - MATCHES wd F38 (re-fetches
proposal via `find_proposal` at :189 but never re-checks `status='open'` and
never verifies the token still matches the player row; it just consumes the
passed `email_token`):
```rust
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
```
(The other five `RealInviteMailer` methods - `notify_changed_reinvite` :226,
`notify_owner_decline` :278, `notify_cancelled` :322, `notify_started` :365,
`notify_owner_ready` :410 - follow the same `tokio::spawn` + re-fetch shape.)

`find_proposal_player_by_email_token` (proposals.rs:671) - EXISTS:
```rust
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
```

`respond_proposal` (proposals.rs:1236) - CONFIRMED calls
`mailer().notify_owner_decline(proposal_id, user.id)` at proposals.rs:1302
(F34 reference):
```rust
    if became_ready {
        mailer().notify_owner_ready(proposal_id);
    } else if !accept {
        mailer().notify_owner_decline(proposal_id, user.id);
    }
```

`reset_accepted_humans_for_roster_change` (proposals.rs:650) - CONFIRMED bumps
`updated_at` and resets `response='pending'` (wd F28 rationale):
```rust
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
```

`fetch_nudge_candidates` (proposals.rs:692):
```sql
SELECT gp.id AS proposal_id, pp.user_id, pp.email_token
FROM game_proposals gp
JOIN game_proposal_players pp ON pp.proposal_id = gp.id
WHERE gp.status = 'open' AND gp.nudged_at IS NULL
  AND gp.created_at < NOW() - ($1 * interval '1 second')
  AND pp.response = 'pending' AND pp.user_id IS NOT NULL
```

`fetch_expiry_candidates` (proposals.rs:726):
```sql
SELECT id FROM game_proposals WHERE status = 'open' AND created_at < NOW() - ($1 * interval '1 second')
```

Tests: `mod tests` at proposals.rs:2170. `sweep_candidate_queries_match_backdated_proposals`
EXISTS (proposals.rs:2289). Full test fn list:
`invite_notification_suppressed_by_recipient_presence`,
`invite_recipient_should_send_truth_table`,
`sweep_candidate_queries_match_backdated_proposals`,
`roster_view_never_exposes_email_token`,
`respond_denied_reason_blocks_owner_and_bad_transitions`,
`transfer_target_must_be_accepted_human`,
`accepted_invitee_ids_excludes_owner_bots_and_nonaccepted`,
`reset_flips_accepted_humans_preserves_others`,
`add_player_inserts_pending_human_and_accepted_bot`,
`remove_works_on_accepted_slot_and_allows_invalid_count`,
`transfer_rejects_bot_and_nonplayer_targets`,
`normalize_positions_after_remove_and_add`,
`ready_to_start_requires_all_humans_accepted_and_valid_count`,
`respond_accept_does_not_auto_start`,
`ready_check_fires_only_when_last_human_accepts`,
`start_guards_reject_pending_declined_invalid_count`,
`start_conditions_met_when_all_accepted_and_valid`,
`accepted_to_declined_transition_works`,
`declined_to_accepted_is_rejected`,
`pending_to_accepted_still_works`,
`start_proposal_tx_rejects_disabled_bot`.
Helpers: `seed_invite_user`, `invite_gate`, `seed_game_version`, `seed_proposal`.

### rust/web/src/email/outbound.rs

`should_email_recipient` (outbound.rs:193) - MATCHES spec exactly:
```rust
pub fn should_email_recipient(recipient: &EmailRecipient) -> bool {
    recipient.email.is_some() && !recipient.is_bot && recipient.turn_emails_enabled
}
```

`EmailRecipient` (outbound.rs:154) - **NO `reminder_emails_enabled` yet** (WP-46
adds it):
```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailRecipient {
    pub email: Option<String>,
    pub theme_slug: Option<String>,
    pub turn_emails_enabled: bool,
    pub user_id: Option<Uuid>,
    pub is_bot: bool,
}
```

`fetch_email_recipient` (outbound.rs:166) - **NO `reminder_emails_enabled`
COALESCE yet** (WP-46 adds it):
```rust
pub async fn fetch_email_recipient(
    pool: &PgPool,
    game_player_id: Uuid,
) -> anyhow::Result<Option<EmailRecipient>> {
    let row = sqlx::query_as::<_, EmailRecipient>(
        "SELECT
            ue.email AS email,
            u.theme AS theme_slug,
            COALESCE(u.turn_emails_enabled, false) AS turn_emails_enabled,
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
```

### rust/web/src/db/emails.rs  (re-exported as `crate::db::...`)

`delete_expired_unverified_emails` (db/emails.rs:210) - pattern to mirror
(`make_interval(secs => $1::double precision)`, `Ok(rows_affected)`):
```rust
#[cfg(feature = "ssr")]
pub async fn delete_expired_unverified_emails(
    pool: &PgPool,
    threshold: std::time::Duration,
) -> Result<u64> {
    let secs = threshold.as_secs() as i64;
    let res = sqlx::query(
        "DELETE FROM user_emails
         WHERE verified_at IS NULL
           AND created_at < NOW() - make_interval(secs => $1::double precision)",
    )
    .bind(secs)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
```
Its tests (db/emails.rs `mod tests`, :230):
- `expiry_cleanup_deletes_only_expired_unverified` (db/emails.rs:459)
- `delete_expired_unverified_emails_zero_threshold` (db/emails.rs:513)

Webhook cleanup: **NO existing `delete_old_processed_webhook_events`** (grep:
zero matches in rust/). The `processed_webhook_events` table EXISTS and
migration `014_email_play.sql` ships `idx_processed_webhook_events_processed_at`
on `processed_at` (014:33, 014:37-38).

### rust/web/src/main.rs

Call site (main.rs:89) - `resend` IS in scope (used at :91; also referenced at
:60-69):
```rust
    web::email::sweep::spawn_periodic_sweeps(
        pool.clone(),
        resend.clone(),
        http_client.clone(),
        broadcaster.clone(),
        jetstream.clone(),
    );
```

---

## 4. Exact names the executor needs

**Unit tests to delete (exercising `is_reminder_candidate`/`should_reset_reminder`)**
- spec said "five"; live tree has SEVEN - see TRIGGER 1. Full set:
  1. `should_reset_reminder_on_transition`
  2. `candidate_predicate_accepts_due_player`
  3. `candidate_predicate_rejects_already_reminded`
  4. `candidate_predicate_rejects_not_turn`
  5. `candidate_predicate_rejects_eliminated`
  6. `candidate_predicate_rejects_below_threshold`
  7. `candidate_predicate_boundary_exact_threshold`

**db submodule holding `delete_expired_unverified_emails`:**
`rust/web/src/db/emails.rs` (re-exported via `crate::db`).

**File holding `fetch_auto_decline_candidates`:**
`rust/web/src/proposals.rs` (top-level `crate::proposals`, NOT email/, NOT sweep.rs).

---

## 5. WP-51 shape note

WP-51 will later rewrite `send_reminder`'s body, the six `RealInviteMailer`
methods, and collapse the five `spawn_*` loops. WP-46 changes that WP-51 must
respect / rebase onto:

- **`spawn_periodic_sweeps` arity and the six-loop fan-out.** Post-WP-38 the
  signature is `(pool, resend, http_client, broadcaster, jetstream)` and it
  spawns six sweeps. WP-46 is expected to thread `resend` into
  `spawn_invite_auto_decline_sweep` / `sweep_invite_auto_decline_once` (today
  they take only `(pool, broadcaster)`). When WP-51 collapses the five loops, it
  must preserve the auto-decline sweep's new mailer access (resend) and must not
  drop the sixth (bot-turn) loop, which is WP-38's and out of WP-51's "five".
- **`send_reminder` return contract + tx-awareness (CORRECTED - earlier note
  said `bool`, which is WRONG).** WP-46 changed `send_reminder` to return a
  private `enum ReminderOutcome { Sent, PermanentSkip, Retry }` and to take a
  `tx: &mut sqlx::PgConnection` parameter:
  `async fn send_reminder(resend, pool, http_client, tx: &mut PgConnection, game_id, game_player_id) -> ReminderOutcome`.
  `sweep_once` opens `pool.begin()`, claims the row `SELECT ... FOR UPDATE SKIP
  LOCKED` on the tx, passes `&mut tx` into `send_reminder`, then on
  `Sent|PermanentSkip` calls `mark_reminder_sent_tx(&mut tx)` + `tx.commit()`
  and on `Retry` drops/rolls back. CRITICAL for WP-51: the `game_players` WRITE
  inside `send_reminder` (`ensure_email_token_tx`, see below) MUST stay on the
  claim tx connection - routing it back to the pool re-introduces the
  self-deadlock WP-46 fixed (the held `FOR UPDATE` lock blocks a pool-side
  `UPDATE game_players`). Pool READS (`find_game_extended`,
  `fetch_email_recipient`, `suppress_for_web_presence`, render,
  `try_send_rendered_email`) stay on the pool. WP-51's body rewrite must keep
  the `ReminderOutcome`/`sweep_once` mark-or-rollback contract AND the
  tx-vs-pool split, or rewrite both together.
- **Second outbound.rs edit (beyond the `EmailRecipient` field).** WP-46 added
  `pub async fn ensure_email_token_tx(tx: &mut sqlx::PgConnection,
  game_player_id: Uuid) -> anyhow::Result<String>` (a duplicated body of
  `ensure_email_token` running on `&mut *tx`). The original pool-based
  `ensure_email_token` is UNCHANGED (notify.rs still calls it). WP-60 owns the
  rest of outbound.rs; WP-51 must not drop `ensure_email_token_tx` if it reworks
  the reminder send path.
- **`EmailRecipient` gains `reminder_emails_enabled: bool` and
  `fetch_email_recipient` gains `COALESCE(u.reminder_emails_enabled, false)`.**
  WP-51 touches `send_reminder`'s body and the mailer methods; any recipient
  gating it rewrites must read the new field, and `should_email_recipient`
  (`email.is_some() && !is_bot && turn_emails_enabled`) is the shared predicate
  also used by notify.rs/inbound.rs - changing it has blast radius beyond sweep.
- **`auto_decline_proposal_player -> bool` and `fetch_auto_decline_candidates`
  returning `pp.user_id`.** WP-51's rewrite of the six mailer methods and the
  auto-decline path must consume the `bool` (rows_affected == 1) and the added
  `user_id` column rather than the old `(pp.id, pp.proposal_id)` tuple / unit
  return.
- **`cancel_proposal_for_expiry` ordering (F35).** Commit `status='cancelled'`
  first, then read owner/accepted. WP-51 must not reorder this if it touches the
  expiry path.
- **`RealInviteMailer::send_invite` token/status gap (wd F38).** WP-46 may add a
  status='open' / token-match re-check; WP-51's method rewrite should preserve
  whatever guard WP-46 lands rather than reverting to the current unguarded
  re-fetch.
