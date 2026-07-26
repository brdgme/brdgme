# WP-44: proposals integrity and email_token leak

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Stop serializing invitee `email_token` secrets to browsers (wd F26), close the owner-decline and transfer-to-declined proposal wedges (wd F29, F30), fix the cancel-notify TOCTOU and authz duplication (wd F31, F35), and clean up the dead/garbled/mislabeled corners of `rust/web/src/proposals.rs` (wd F36, F40-F44).

**Architecture:** `rust/web/src/proposals.rs` is a single-file Leptos/Axum module: SQL helper fns at the top, `#[server]` fns in the middle (each opens a transaction, calls `lock_proposal_for_update`, mutates, commits, then broadcasts and fires `RealInviteMailer` tasks), and the `InvitePage`/`ProposalDetail` client components at the bottom. `get_proposal` returns a `ProposalView` (containing `Vec<ProposalPlayerView>`) over the server-fn wire to any authenticated user. Inline `#[cfg(all(test, feature = "ssr"))] mod tests` at the end of the file holds the module's tests.

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`; Leptos server fns; sqlx (runtime `query_as`, not compile-checked macros, for everything in this file); `#[sqlx::test]` for DB tests; Postgres 18.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Only per-crate commands: `cargo test -p web --features ssr <filter>`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. Never workspace-wide builds.
- `#[sqlx::test]` tests need the throwaway Postgres that `/home/beefsack/Development/brdgme/scripts/rust-test.sh` provides (`DATABASE_URL` on port 15432). In a plain shell without those containers, DB tests fail; that is pre-existing (backlog #40), not a regression. The full pre-commit gate is `scripts/rust-test.sh` and it MUST pass before the final commit of this package.
- No behavior changes outside the proposals scope. Serialized wire formats stay compatible except the two deliberate changes: removing `email_token` from `ProposalPlayerView` (Task 1) and removing `RespondOutcome` (Task 5). Both types are consumed only by this crate's own client code, which is updated in the same task.
- Every task ends with `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.

**Non-Goals:**

- The email From-address authentication redesign (D-1) is OUT - that is WP-56. This package removes the token from the wire; token rotation and inbound-auth hardening follow the redesign.
- Bot-slot validation (wd F27 / wfe F18) is OUT - WP-45 (blocked on D-8). Do not add bot-name validation to `create_proposal` / `add_proposal_player` here.
- Sweep delivery semantics (wd F28, F38, F39) are OUT - WP-46. Do not change what the sweep queries mean, only how the interval is bound (Task 6).
- Invite-mailer gating/Reply-To findings (wd F32, F33, F34) and email canonicalization (wd F37) are in other packages. Do not touch mailer gating or email normalization.

**Snapshot drift:** None. Live `rust/web/src/proposals.rs` at HEAD (`0243472`) is byte-identical to the review snapshot (`brdgme-review-snapshot`, commit f8763a5). All finding line numbers cited below were re-verified against the live file.

---

### Task 1: Stop serializing email_token to proposal viewers (wd F26)

**Problem (restated):** `ProposalPlayerView` (struct at `rust/web/src/proposals.rs:70-81`) has `pub email_token: Option<String>` at line 78. It is populated by `find_proposal_roster` (lines 507-523, `pp.email_token` in the SELECT at line 513) and returned by the `get_proposal` server fn (lines 1717-1765) inside `ProposalView.players` to ANY authenticated user - `viewer_role` is computed (lines 1748-1754) but never used to gate data. The email token is the credential the inbound email handler uses to accept/decline an invite on the invitee's behalf (`rust/web/src/email/inbound.rs:594`, `find_proposal_player_by_email_token`). Inbound additionally checks the From address, but From is forgeable (see WP-56); shipping every invitee's token to every viewer's browser turns a From forgery into a full invite takeover. Nothing legitimate consumes the field.

**Consumer audit (whole `rust/` tree, verified by grep):**

- `ProposalPlayerView` is referenced only in `proposals.rs` (struct def line 70, `ProposalView.players` line 96, `find_proposal_roster` lines 510-511). No other file names it.
- `find_proposal_roster` has exactly one caller: `get_proposal` (line 1744).
- The client component `ProposalDetail` (lines 1911-2185) reads only `p.id`, `p.position` (implicitly via ordering), `p.user_id`, `p.name`, `p.response` from `ProposalPlayerView`. It never touches `email_token`.
- The other `email_token` users in the tree (`email/sweep.rs:300`, `game/server_fns.rs:1245`, `email/inbound.rs`, `reset_accepted_humans_for_roster_change`) all use the **server-side `ProposalPlayer` model** (line 41) or `NudgeCandidate` (line 712), which never cross the wire. They are untouched.

**Fix:** Delete the field from the view struct and the roster SELECT. Minimal, no legitimate consumer breaks.

**Files:**
- Modify: `rust/web/src/proposals.rs` (line 78 struct field; line 513 SELECT column)
- Test: `rust/web/src/proposals.rs` inline `mod tests`

**Interfaces:**
- Consumes: existing test helpers `seed_game_version(pool)`, `seed_invite_user(pool, bool)`, `seed_proposal(pool, gv, owner)`, `insert_proposal_player(...)` already in `mod tests`.
- Produces: `ProposalPlayerView` without `email_token`; `find_proposal_roster(pool, proposal_id) -> sqlx::Result<Vec<ProposalPlayerView>>` signature unchanged.

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/web/src/proposals.rs`:

```rust
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
```

- [ ] Run it (requires DB containers): `cargo test -p web --features ssr roster_view_never_exposes_email_token`. Expected: FAILS on the `!json.contains("email_token")` assertion (field currently serialized).
- [ ] Implement. In `rust/web/src/proposals.rs`:

  1. Delete line 78 from the struct so it reads:

```rust
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
```

  2. In `find_proposal_roster` change the SELECT (lines 512-514) from:

```rust
        "SELECT pp.id, pp.\"position\", pp.user_id, pp.bot_name, pp.bot_difficulty, pp.response, \
         pp.responded_at, pp.email_token, \
         COALESCE(u.name, pp.bot_name, 'Bot') AS name \
```

     to:

```rust
        "SELECT pp.id, pp.\"position\", pp.user_id, pp.bot_name, pp.bot_difficulty, pp.response, \
         pp.responded_at, \
         COALESCE(u.name, pp.bot_name, 'Bot') AS name \
```

- [ ] Run: `cargo test -p web --features ssr roster_view_never_exposes_email_token` - expected PASS. Then `cargo test -p web --features ssr proposals::` - all module tests PASS (the existing tests use `ProposalPlayer`, not the view, so none reference the removed field).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): stop serializing invitee email_token to proposal viewers (wd F26, WP-44)`

---

### Task 2: Owner cannot decline their own proposal (wd F29)

**Problem:** `respond_proposal` (lines 1200-1282) finds the caller in the roster (lines 1229-1232) with no owner exclusion. The owner's row is `"accepted"` (inserted at line 1137), and the transition table (lines 1235-1238) allows `accepted -> declined`. Once the owner is declined the proposal is wedged: declined is terminal (no `declined -> *` transition), `remove_proposal_slot` refuses to remove the owner (lines 1619-1623), and `start_proposal` rejects any declined slot (lines 1331-1336). Only cancel or transfer-then-remove escapes, neither obvious.

**Fix:** Extract the transition decision into a pure helper that rejects any owner response, and use it in `respond_proposal`. Rejecting owner *accept* too is correct (today it already errors with "You have already accepted this invite"; the new message is clearer). The UI never shows respond buttons to the owner (`is_invitee` is false for owners, line 1748-1754 role computation + line 2038 `Show`), so this is API-level hardening only. Note the owner-exclusion key is `proposal.owner_user_id`, so after an ownership transfer the *previous* owner (now a regular accepted invitee) can still decline - intended.

**Files:**
- Modify: `rust/web/src/proposals.rs` (new helper near `proposal_ready_to_start` at line 1019; use it in `respond_proposal` lines 1234-1246)
- Test: inline `mod tests`

**Interfaces:**
- Produces: `#[cfg(feature = "ssr")] fn respond_denied_reason(is_owner: bool, current: &str, target: &str) -> Option<&'static str>`

**Steps:**

- [ ] Write the failing test (pure unit test, no DB):

```rust
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
```

- [ ] Run: `cargo test -p web --features ssr respond_denied_reason_blocks` - expected: compile FAILURE (`respond_denied_reason` not found).
- [ ] Implement. Add after `proposal_ready_to_start` (line 1029):

```rust
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
```

  Then in `respond_proposal`, replace lines 1234-1246:

```rust
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
```

  with:

```rust
    let target = if accept { "accepted" } else { "declined" };
    if let Some(msg) = respond_denied_reason(
        user.id == proposal.owner_user_id,
        me.response.as_str(),
        target,
    ) {
        return Err(ServerFnError::new(msg));
    }
```

- [ ] Run: `cargo test -p web --features ssr respond_denied_reason_blocks` - PASS. Then `cargo test -p web --features ssr proposals::` - all PASS (existing transition tests `accepted_to_declined_transition_works`, `declined_to_accepted_is_rejected`, `pending_to_accepted_still_works` assert on their own local copies of the matches! expression plus DB helpers, so they still pass; they cover non-owner rows).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): reject owner responding to own proposal to prevent wedge (wd F29, WP-44)`

---

### Task 3: Ownership transfer requires an accepted target (wd F30)

**Problem:** `transfer_proposal_ownership` (lines 1655-1711) checks only membership: line 1696 `players.iter().any(|p| p.user_id == Some(target_user_id))`. Transferring to a *declined* invitee creates a proposal whose owner has a terminal declined response - it can never start (line 1331-1336 declined guard), the owner slot cannot be removed (lines 1619-1623), and the response can never change. Transferring to a *pending* invitee was previously recoverable (they could accept), but **after Task 2 an owner can no longer respond at all**, so a pending owner also becomes a permanent wedge (proposal can never satisfy `pending_humans == 0`). The finding's "or at least not declined" fallback is therefore overturned: with F29 landed, the target MUST be `"accepted"`.

**Fix:** Require the target row to be a human with `response == "accepted"`, via a pure testable helper. Also gate the UI's "(make owner)" link on accepted rows so it doesn't offer an action that always errors.

**Files:**
- Modify: `rust/web/src/proposals.rs` (helper; `transfer_proposal_ownership` line 1696; `ProposalDetail` line 1992)
- Test: inline `mod tests`

**Interfaces:**
- Produces: `#[cfg(feature = "ssr")] fn transfer_target_error(players: &[ProposalPlayer], target_user_id: Uuid) -> Option<&'static str>`

**Steps:**

- [ ] Write the failing test:

```rust
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
```

- [ ] Run: `cargo test -p web --features ssr transfer_target_must_be_accepted` - expected: compile FAILURE (helper missing).
- [ ] Implement. Add next to `respond_denied_reason`:

```rust
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
```

  In `transfer_proposal_ownership`, replace lines 1696-1698:

```rust
    if !players.iter().any(|p| p.user_id == Some(target_user_id)) {
        return Err(ServerFnError::new("That player isn't in this proposal."));
    }
```

  with:

```rust
    if let Some(msg) = transfer_target_error(&players, target_user_id) {
        return Err(ServerFnError::new(msg));
    }
```

  In `ProposalDetail` change line 1992:

```rust
                    let show_make_owner = is_owner && !is_bot && !is_owner_row;
```

  to:

```rust
                    let show_make_owner =
                        is_owner && !is_bot && !is_owner_row && response == "accepted";
```

  (`response` is the `String` cloned at line 1985; comparing `String == &str` works. It is later moved into the view at line 2000 - the comparison happens before the move, so no borrow issue. If the compiler complains about `response` being moved into the earlier closure order, compute `show_make_owner` before the `view!` block, which is where line 1992 already sits.)

- [ ] Run: `cargo test -p web --features ssr transfer_target_must_be_accepted` - PASS. `cargo test -p web --features ssr proposals::` - PASS (the existing `transfer_rejects_bot_and_nonplayer_targets` test drives `update_proposal_owner` directly with an accepted target; unaffected).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): require accepted target for ownership transfer (wd F30, WP-44)`

---

### Task 4: Cancel notifies from the locked roster; drop duplicated pre-transaction authz (wd F31, wd F35)

**Problem (F31):** `cancel_proposal` (lines 1511-1572) fetches `players` from the pool at line 1532 BEFORE `begin()` (line 1537) and the `lock_proposal_for_update` (line 1541). The accepted-invitee list for `notify_cancelled` (lines 1563-1568) is derived from that stale snapshot. An invitee whose accept commits between the fetch and the lock gets no cancellation email. Every other mutating fn reads players via `find_proposal_players_tx` inside the lock.

**Problem (F35):** Four server fns run the identical find -> owner-check -> open-check sequence twice - once against the pool before `begin()`, then again after the lock: `add_proposal_player` (pre: 1396-1405, in-lock: 1412-1421), `cancel_proposal` (pre: 1519-1530, in-lock: 1541-1552), `remove_proposal_slot` (pre: 1585-1594, in-lock: 1601-1610), `transfer_proposal_ownership` (pre: 1666-1675, in-lock: 1682-1691). The in-lock check is the authoritative one; `respond_proposal` and `start_proposal` already get by with the in-lock check alone. ~60 lines of copy-paste.

**Fix:** Delete the four pre-transaction blocks (keep the in-lock ones). In `cancel_proposal`, move the players fetch inside the transaction after the lock (`find_proposal_players_tx`). Extract the accepted-invitee filter (duplicated between `start_proposal` lines 1362-1367 and `cancel_proposal` lines 1563-1568) into a pure helper so it is testable and stays in sync.

**Files:**
- Modify: `rust/web/src/proposals.rs`
- Test: inline `mod tests`

**Interfaces:**
- Produces: `#[cfg(feature = "ssr")] fn accepted_invitee_ids(players: &[ProposalPlayer], owner_user_id: Uuid) -> Vec<Uuid>`

**Steps:**

- [ ] Write the failing test:

```rust
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
```

- [ ] Run: `cargo test -p web --features ssr accepted_invitee_ids_excludes` - expected: compile FAILURE.
- [ ] Implement, four edits:

  1. Add the helper near the other pure helpers:

```rust
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
```

  2. `cancel_proposal`: delete the pre-transaction block (lines 1519-1534, i.e. the first `find_proposal` + owner check + open check AND the `find_proposal_players` pool fetch). After the surviving in-lock checks (lines 1541-1552), fetch players inside the lock, before the status update:

```rust
    let players = find_proposal_players_tx(&mut tx, proposal_id)
        .await
        .map_err(internal("cancel_proposal: players"))?;
```

  Then after commit, replace lines 1563-1569 with:

```rust
    broadcaster.broadcast_proposal_update(proposal_id).await;
    mailer().notify_cancelled(
        proposal_id,
        accepted_invitee_ids(&players, proposal.owner_user_id),
    );
```

  3. `start_proposal`: replace lines 1362-1368 with:

```rust
    let invitee_ids = accepted_invitee_ids(&players, proposal.owner_user_id);
    mailer().notify_started(proposal_id, game_id, invitee_ids);
```

  4. Delete the pre-transaction find/owner/open blocks in `add_proposal_player` (lines 1396-1405), `remove_proposal_slot` (lines 1585-1594), and `transfer_proposal_ownership` (lines 1666-1675). In each fn the flow becomes: (arg validation if any) -> `pool.begin()` -> `lock_proposal_for_update` -> the existing in-lock owner/open checks -> rest unchanged. Note in `add_proposal_player` the `provided != 1` argument check (lines 1390-1394) stays, before `begin()`.

- [ ] Run: `cargo test -p web --features ssr accepted_invitee_ids_excludes` - PASS. `cargo test -p web --features ssr proposals::` - all PASS.
- [ ] Behavior check (enumerated edge cases, no code): a caller hitting a nonexistent/foreign/closed proposal now pays one transaction before getting the same error message it got before; a racing accept committed before the lock is now included in cancel notifications; a racing accept committed after the lock waits on the row lock and then fails the `status != "open"` check.
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): read cancel roster under lock; drop duplicated pre-tx authz (wd F31, wd F35, WP-44)`

---

### Task 5: Delete dead RespondOutcome navigation (wd F36)

**Problem:** `respond_proposal` always returns `RespondOutcome { accepted, started: false, game_id: None }` (lines 1277-1281) - leftover from a removed auto-start design (the `respond_accept_does_not_auto_start` test documents the removal). The client effect (lines 1835-1846) still branches on `outcome.game_id` and navigates to `/games/{gid}` - dead code. `RespondOutcome` (lines 61-66) is referenced nowhere else in `rust/` (verified by grep; the email inbound accept path does not use the server fn).

**Fix:** Delete `RespondOutcome`; `respond_proposal` returns `Result<(), ServerFnError>`; the effect always bumps the proposal update.

**Files:**
- Modify: `rust/web/src/proposals.rs` (struct 61-66; return type line 1203; return value 1277-1281; effect 1834-1846; error-`Show` block at 2142-2148 unaffected - it only calls `.is_err()`/`.err()`)

**Steps:**

- [ ] No new test (pure dead-code removal; the compiler is the test - any surviving consumer of the struct or of `.game_id` fails the build). Existing DB test `respond_accept_does_not_auto_start` remains the behavioral guard.
- [ ] Implement:

  1. Delete lines 61-66 (`pub struct RespondOutcome {...}`).
  2. Change the signature (lines 1200-1203) to:

```rust
pub async fn respond_proposal(proposal_id: Uuid, accept: bool) -> Result<(), ServerFnError> {
```

  3. Replace lines 1277-1281 with:

```rust
    Ok(())
```

  4. Replace the effect (lines 1834-1846):

```rust
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
```

  with:

```rust
    Effect::new(move |_| {
        if let Some(Ok(())) = respond_action.value().get() {
            crate::websocket_client::bump_proposal_update(
                proposal_update,
                proposal_id().unwrap_or_default(),
            );
        }
    });
```

  (`nav1` disappears; `navigate` is still used by `nav3` and `nav_start`, so keep `let navigate = use_navigate();`.)

- [ ] Run: `cargo test -p web --features ssr proposals::` - PASS. `cargo clippy -p web --all-targets --features ssr -- -D warnings` - clean (this also compiles the hydrate/client cfg paths via --all-targets).
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `refactor(proposals): remove dead RespondOutcome auto-start plumbing (wd F36, WP-44)`

---

### Task 6: Typed interval binds in sweep candidate queries (wd F40)

**Problem:** Three sweep queries bind `threshold_secs.to_string()` and synthesize the interval with `($1 || ' seconds')::interval`: `fetch_nudge_candidates` (line 725), `fetch_expiry_candidates` (line 755), `fetch_auto_decline_candidates` (line 819). Parameterized (no injection) but roundabout: a text bind cast to interval at runtime. A numeric bind multiplied by `interval '1 second'` is typed end-to-end (Postgres implicitly widens `int8` to `float8` for the `float8 * interval` operator).

**Danger note:** these three fns swallow query errors into an empty Vec with only a `tracing::error!` (lines 731-737, 760-766, 824-830). A botched SQL rewrite would silently disable all three sweeps. The regression test below therefore asserts the *positive* path (candidates are returned), and MUST be written and seen PASSING against the old SQL before the rewrite, then kept passing after.

**Files:**
- Modify: `rust/web/src/proposals.rs` (lines 725/728, 755/757, 819/821)
- Test: inline `mod tests`

**Steps:**

- [ ] Write the regression test first and confirm it PASSES against the current code (guard, not red-green - this is a pure refactor):

```rust
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
        sqlx::query("UPDATE game_proposals SET created_at = created_at - interval '1 hour' WHERE id = $1")
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
                .any(|(_, p)| *p == pid),
            "auto-decline query must return the backdated pending slot"
        );

        // 2h threshold: nothing qualifies.
        assert!(fetch_nudge_candidates(&pool, 7200).await.is_empty());
        assert!(fetch_expiry_candidates(&pool, 7200).await.is_empty());
        assert!(fetch_auto_decline_candidates(&pool, 7200).await.is_empty());
    }
```

- [ ] Run: `cargo test -p web --features ssr sweep_candidate_queries_match_backdated` - expected PASS (guard established).
- [ ] Implement. In each of the three queries replace the interval predicate and the bind:

  - Line 725 (`fetch_nudge_candidates`): `AND gp.created_at < NOW() - ($1 || ' seconds')::interval` -> `AND gp.created_at < NOW() - ($1 * interval '1 second')`, and line 728 `.bind(threshold_secs.to_string())` -> `.bind(threshold_secs)`.
  - Line 755 (`fetch_expiry_candidates`): `AND created_at < NOW() - ($1 || ' seconds')::interval` -> `AND created_at < NOW() - ($1 * interval '1 second')`, and line 757 `.bind(threshold_secs.to_string())` -> `.bind(threshold_secs)`.
  - Line 819 (`fetch_auto_decline_candidates`): `AND gp.created_at < NOW() - ($1 || ' seconds')::interval` -> `AND gp.created_at < NOW() - ($1 * interval '1 second')`, and line 821 `.bind(threshold_secs.to_string())` -> `.bind(threshold_secs)`.

  (Do NOT change which timestamp column is compared - `gp.created_at` keying is wd F28, owned by WP-46.)

- [ ] Run: `cargo test -p web --features ssr sweep_candidate_queries_match_backdated` - PASS. Also run the sweep integration tests that exercise these fns: `cargo test -p web --features ssr sweep` - PASS.
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `refactor(proposals): bind sweep thresholds as numeric interval, not text concat (wd F40, WP-44)`

---

### Task 7: Single-statement roster reset (wd F41)

**Problem:** `reset_accepted_humans_for_roster_change` (lines 658-685) SELECTs matching rows then loops one UPDATE per player to assign fresh tokens. Harmless at roster scale, but a single `UPDATE ... RETURNING` does it in one round trip and removes the loop.

**Fix:** One UPDATE generating tokens in SQL. `replace(gen_random_uuid()::text, '-', '')` produces exactly the same 32-lowercase-hex shape as `Uuid::new_v4().simple().to_string()` (`gen_random_uuid()` is built into Postgres 13+; this repo runs Postgres 18). Row order of RETURNING is unspecified, which is fine - the result is only iterated to fire re-invite mailer tasks.

**Files:**
- Modify: `rust/web/src/proposals.rs` (lines 657-685)
- Test: existing `reset_flips_accepted_humans_preserves_others` (line 2307) is the guard - it asserts exactly which rows flip, that tokens are non-empty and distinct, and that owner/bot/declined/pending rows are untouched.

**Steps:**

- [ ] Confirm the guard passes before changing anything: `cargo test -p web --features ssr reset_flips_accepted_humans_preserves_others` - PASS.
- [ ] Implement. Replace the whole body of `reset_accepted_humans_for_roster_change`:

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

  (`query_as` into `(Uuid, String)`: both columns are nullable in the schema but the WHERE clause guarantees `user_id IS NOT NULL` and the SET guarantees `email_token IS NOT NULL`; runtime `query_as` decodes fine. Uniqueness across rows comes from `gen_random_uuid()` being evaluated per row.)

- [ ] Run: `cargo test -p web --features ssr reset_flips_accepted_humans_preserves_others` - PASS (asserts 2 rows reset, distinct non-empty tokens, others preserved - proves per-row token generation works). Then `cargo test -p web --features ssr proposals::` - PASS.
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `refactor(proposals): reset roster in one UPDATE ... RETURNING (wd F41, WP-44)`

---

### Task 8: Delete dead count_pending_human_invitees pool variant (wd F42)

**Problem:** Only the `_tx` variant (line 873) has a caller (`rust/web/src/email/inbound.rs:735`). The pool variant at lines 700-708 is unused everywhere in `rust/` (verified by grep). Being `pub`, rustc/clippy never flag it.

**Files:**
- Modify: `rust/web/src/proposals.rs` (delete lines 700-708 including the `#[cfg(feature = "ssr")]` attribute)

**Steps:**

- [ ] Implement: delete the fn:

```rust
#[cfg(feature = "ssr")]
pub async fn count_pending_human_invitees(pool: &PgPool, proposal_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM game_proposal_players WHERE proposal_id = $1 AND response = 'pending' AND user_id IS NOT NULL",
    )
    .bind(proposal_id)
    .fetch_one(pool)
    .await
}
```

- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` - clean compile proves no caller existed. `cargo test -p web --features ssr proposals::` - PASS.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `chore(proposals): delete dead count_pending_human_invitees pool variant (wd F42, WP-44)`

---

### Task 9: Missing player_counts row is an internal error, not a garbled message (wd F43)

**Problem:** When `find_game_type_player_counts` returns `None` (game-type row missing - an invariant violation), `start_proposal` (lines 1338-1341) and `respond_proposal` (lines 1257-1261) use `.unwrap_or_default()`, producing an empty counts list. In `start_proposal` that flows into `roster_error`, which renders the user-facing garbage "This game supports  players, but the roster has N". `create_proposal` (line 1066) already handles the same `None` with a clean "Game type not found" error - match it. In `respond_proposal` the counts only feed the `became_ready` owner-email check, but silently computing readiness against an empty list is the same masked invariant violation.

**Fix:** Replace both `.unwrap_or_default()` calls with `.ok_or_else(|| ServerFnError::new("Game type not found"))?`, exactly as `create_proposal` does. (The third `.unwrap_or_default()` at line 1742 in `get_proposal` is read-only display and was not cited by the finding; leave it - changing it would make an existing viewable proposal page hard-error, a behavior change beyond the finding.)

**Files:**
- Modify: `rust/web/src/proposals.rs` (lines 1257-1261, 1338-1341)

**Steps:**

- [ ] Implement. In `respond_proposal` replace:

```rust
        let player_counts =
            crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
                .await
                .map_err(internal("respond_proposal: player counts"))?
                .unwrap_or_default();
```

  with:

```rust
        let player_counts =
            crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
                .await
                .map_err(internal("respond_proposal: player counts"))?
                .ok_or_else(|| ServerFnError::new("Game type not found"))?;
```

  In `start_proposal` replace:

```rust
    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal: player counts"))?
        .unwrap_or_default();
```

  with:

```rust
    let player_counts = crate::db::find_game_type_player_counts(&pool, proposal.game_version_id)
        .await
        .map_err(internal("start_proposal: player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
```

- [ ] No isolated test: the branch is unreachable without deleting a `game_types` row out from under a live proposal, and the logic lives inline in server fns that need full request context. Guard is compile + existing suite: `cargo test -p web --features ssr proposals::` - PASS.
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): error cleanly when game type player counts missing (wd F43, WP-44)`

---

### Task 10: Neutral error labels and missing instrumentation (wd F44)

**Problem:** `find_or_create_user_by_email_tx` (lines 886-921) hardcodes `internal("create_proposal: ...")` context at lines 896, 904, 911, 919, but is also called from `add_proposal_player` (line 1430) - failures there log under the wrong fn name. Separately, `cancel_proposal` (line 1510), `remove_proposal_slot` (line 1576), and `get_pending_invites` (line 1768) are the only server fns in the file without a `tracing::instrument` attribute.

**Fix:** Neutral context strings naming the helper, plus the standard instrument attribute on the three fns (with `proposal_id` field where one exists).

**Files:**
- Modify: `rust/web/src/proposals.rs`

**Steps:**

- [ ] Implement:

  1. In `find_or_create_user_by_email_tx`, change the four labels:
     - line 896: `internal("create_proposal: resolve email")` -> `internal("resolve invite email: lookup")`
     - line 904: `internal("create_proposal: gen username")` -> `internal("resolve invite email: gen username")`
     - line 911: `internal("create_proposal: resolve email")` -> `internal("resolve invite email: insert user")`
     - line 919: `internal("create_proposal: resolve email")` -> `internal("resolve invite email: insert email")`

  2. Add instrumentation. Above `pub async fn cancel_proposal` (after its `#[server(CancelProposal, "/api")]` line):

```rust
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
```

  Above `pub async fn remove_proposal_slot` (after `#[server(RemoveProposalSlot, "/api")]`):

```rust
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all, fields(proposal_id = %proposal_id)))]
```

  Above `pub async fn get_pending_invites` (after `#[server(GetPendingInvites, "/api")]`):

```rust
#[cfg_attr(feature = "ssr", tracing::instrument(skip_all))]
```

- [ ] No isolated test (log-label/span change only). `cargo test -p web --features ssr proposals::` - PASS; `cargo clippy -p web --all-targets --features ssr -- -D warnings` - clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `chore(proposals): neutral resolve-email error labels, instrument remaining server fns (wd F44, WP-44)`

---

### Final verification

- [ ] Run the full pre-commit suite: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` - must pass end to end (fmt, clippy workspace + web-ssr, sqlx prepare check, workspace tests, web ssr tests). The sqlx prepare check is unaffected because every query touched in this package is a runtime `query_as`/`query_scalar`, not a compile-checked macro.

---

## Findings disposition

| Finding | Disposition |
|---|---|
| wd F26 | Task 1 - field + SELECT column removed; zero legitimate consumers found in whole `rust/` tree |
| wd F29 | Task 2 - pure `respond_denied_reason` helper; owner blocked from all responses, not just decline |
| wd F30 | Task 3 - **recommendation tightened:** the finding's "or at least not declined" fallback is unsound once F29 lands (a pending owner could never accept, so pending targets also wedge); target must be `accepted`. UI link gated too |
| wd F31 | Task 4 - roster read moved inside the lock via `find_proposal_players_tx` |
| wd F35 | Task 4 - pre-transaction blocks deleted in all four fns (finding's "drop the pre-checks" option chosen over the helper-extraction option; less code, identical authority) |
| wd F36 | Task 5 - `RespondOutcome` deleted entirely (finding offered shrink-or-unit; unit chosen since `accepted` had no consumer either) |
| wd F40 | Task 6 - `NOW() - ($1 * interval '1 second')` with numeric bind; positive-path regression test guards the silent-empty-on-error failure mode |
| wd F41 | Task 7 - single `UPDATE ... RETURNING` with SQL-generated tokens; existing test is the guard |
| wd F42 | Task 8 - dead pool variant deleted |
| wd F43 | Task 9 - both cited sites now match `create_proposal`'s "Game type not found"; `get_proposal`'s uncited read-only instance deliberately left alone |
| wd F44 | Task 10 - four labels neutralized; three server fns instrumented |
