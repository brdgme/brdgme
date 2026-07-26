# WP-51: invite-mailer and notify dedup

> **CITATION WARNING - line numbers in this spec are approximate and unverified.**
> Corpus-wide they measured **33-46% wrong**, and two "delete lines A-B" ranges
> would have destroyed live code. **Navigate by the named function, type or
> symbol** - never by line number alone. If the code at a cited location does not
> match this spec's description, **STOP and report**; do not improvise a fix or
> guess at the intended target.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Remove the two silent `find_game_extended(...).ok().flatten()` before-snapshot reads by having `game::execute_command` return the snapshot it already loaded, which simultaneously kills the only real `before = None` re-notification path (wd F8, wfe F42); stop `game_log_count`'s `unwrap_or(0)` from collapsing every affected turn email onto one subject (wfe F41); load the game and the log count once per `notify_game_emails` call instead of once per recipient (wfe F43); delete `sweep::send_reminder`'s ~90-line copy of `notify::send_one` by adding a `NotifyKind::Reminder` plus an outcome-returning send (wfe F36), which also gives the reminder the same de-threaded per-turn subject its turn email uses (wfe F39); collapse the five copy-pasted `tokio::spawn`/interval loops into one helper (wfe F38); and in `proposals.rs`: apply the invite gates every other mailer applies to `notify_owner_decline` (wd F32), stop promising a reply on the four pure-notification mails whose reply address can never resolve (wd F33), and log instead of silently swallowing the mailer tasks' DB errors while eliminating blank-substitution subjects (wd F34).

**Architecture — how the notification path works (read this before editing):**

`rust/web` is one Axum + Leptos crate; everything below is `#[cfg(feature = "ssr")]` server code. Four modules matter:

- **`src/game/mod.rs` (1102 lines).** `broadcast_and_trigger` (:51-59) = `broadcast_game_update` + `trigger_bot_turns`. `execute_command` (:79-171) is the single command-application path: loads `GameExtended` (:90-92), rejects finished games (:94-96) and non-turn players (:104-106), POSTs `Request::Play` to the game service (:114-125), writes via `db::update_game_command_success` (:148-168, returns `StaleStateConflict` -> `ExecuteCommandError::Conflict`), then `broadcast_and_trigger` (:169) and `Ok(())`. `handle_bot_command_event` (:336-422) wraps it for the `bot.command` consumer. Inline `#[cfg(all(test, feature = "ssr"))] mod tests` at :424-425 onward with the mock-game-service harness (`spawn_mock_game_service` :443, `make_broadcaster` :498, `make_jetstream` :508, `make_two_player_game` :517, `play_response` :573; first test `happy_path_saves_state_and_advances_turn` :610-658).
- **`src/game/server_fns.rs` (2597 lines).** `submit_command` (:502-560) is the web move path; `undo_game` (:834), `concede_game` (:909), `end_game` (:966) and the restart path (:1226) are the other `notify_game_emails` call sites.
- **`src/email/notify.rs` (679 lines).** Subject/URL/header primitives (:10-73), `render_board_and_you_can` (:78-108), private `enum NotifyKind { Turn, Eliminated, Finished }` (:110-114) and `enum SendMode { Normal, BypassSuppression, Forced }` (:116-120), `digest_since_last_turn` (:126-145), `build_content` (:149-190), `failure_report_content` (:197-222), `game_log_count` (:227-233), `send_one` (:235-336), the five public wrappers (:338-440), `notify_game_emails` (:445-494), and `mod tests` (:496-679).
- **`src/email/sweep.rs` (1046 lines).** Five periodic jobs: `sweep_once` (turn reminders, :195-212) with `fetch_candidates` (:56-80), `mark_reminder_sent` (:82-96) and `send_reminder` (:98-193); `sweep_unverified_emails_once` (:235-241); `sweep_invite_nudge_once` (:288-306); `sweep_invite_expiry_once` (:321-338); `sweep_invite_auto_decline_once` (:353-372). Each has a `spawn_*` fn with an identical 8-line tokio interval loop (:214-229, :245-256, :308-319, :340-351, :374-388), all started by `spawn_periodic_sweeps` (:390-401).
- **`src/email/outbound.rs` (355 lines).** `suppress_for_web_presence` (:42-47), `try_send_rendered_email` (:53-86) returning `bool` (and the `println!` dev path at :58-64 when `resend` is `None`), `send_rendered_email` (:88-94) discarding it, `ensure_email_token` (:110-126), `fetch_email_recipient` (:145-165), `should_email_recipient` (:172-174) = `email.is_some() && !is_bot && turn_emails_enabled`.
- **`src/email/render.rs` (553 lines).** `render_game_email` (:91-251): threading headers are emitted **only** when `thread_id` is `Some` (:226-234) — `is_first_message` then selects `Message-Id` vs `In-Reply-To`+`References`. With `thread_id = None` no threading header is emitted at all.
- **`src/proposals.rs` (2961 lines).** `trait InviteMailer` (:110-122) with six methods, `RealInviteMailer` (:125-128), `fetch_invite_recipient` (:146-159), `invite_recipient_should_send` (:165-167), `proposal_game_type_name` (:170-178), and the six `impl` methods, each a `tokio::spawn`: `send_invite` (:182-232), `notify_changed_reinvite` (:234-284), `notify_owner_decline` (:286-328), `notify_cancelled` (:330-371), `notify_started` (:373-416), `notify_owner_ready` (:418-464).

**Inbound routing (needed to judge wd F33).** `parse_reply_address` (`src/email/inbound.rs:37-51`) takes the local part before `@` and requires a `g-` / `i-` / `s-` prefix; anything else is `None`. `resend_webhook`'s route match (`inbound.rs:477-487`) sends `Some(Game)` to `handle_game_reply`, `Some(Invite)` to `handle_invite_reply`, and **`Some(Settings(_)) | None` both to `handle_settings_reply_route`**. So an unprefixed address is not inert — it lands in the From-authenticated settings command handler.

**Tech Stack:** Rust 1.97.0, edition 2024, workspace at `/home/beefsack/Development/brdgme/rust` (`rust-toolchain.toml` pins channel + rustfmt + clippy). One crate touched: `web`, feature `ssr`. Postgres 18 via sqlx (all queries in scope are **runtime** `query_scalar`/`query_as`, never the compile-checked macros). `time 0.3` with `std` (so `time::OffsetDateTime::now_utc()` is available — already used in prod code at `src/auth/server.rs:165`). `tokio` interval/`MissedTickBehavior`. `resend_rs`, `mrml`, `reqwest`, `tracing`, `anyhow`.

**Global Constraints:**

- Run all commands from `/home/beefsack/Development/brdgme/rust`. **Per-crate, ssr-gated only:** `cargo test -p web --features ssr [filter]`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`. NEVER a workspace-wide `cargo build`/`check`/`test` (AGENTS.md "Resource constraints": ~30 binaries link and RAM/disk spike).
- Each task ends with `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- `#[sqlx::test]` tests need the throwaway Postgres (port 15432) **and** the tests in `src/game/mod.rs` additionally need NATS (14222); both come from `/home/beefsack/Development/brdgme/scripts/rust-test.sh`. In a plain shell without those containers, DB/NATS tests fail to connect — pre-existing (AGENTS.md, backlog #40), **not** a regression. The full pre-commit gate `scripts/rust-test.sh` MUST pass before the **final** commit of this package.
- **No compile-checked `sqlx::query!` macro is added or changed anywhere in this package**, so `cargo sqlx prepare --check` cannot break. The one SQL statement touched (`game_log_count`) is and stays a runtime `query_scalar`.
- `docs/CODING.md` "Testing Conventions": changes to `rust/web/src/game/mod.rs` MUST land with tests. Task 1 does.
- `docs/CODING.md` "Comment discipline": comments only where the *why* is non-obvious. Every comment prescribed below encodes a hazard or a cross-package handoff; do not add others.
- Line numbers below are **live-file** numbers verified 2026-07-25 at HEAD `0243472`. Tasks that shift numbering say so, and later tasks **locate by symbol name, never by number**.
- No serialized wire format changes. `GameViewData`, `ProposalView`, `EmailContent`, `RenderedEmail` field sets are untouched. The only signature changes are internal to the crate: `game::execute_command`'s `Ok` type (Task 1) and `notify`'s send fns (Tasks 2-3).

**Non-Goals (owned elsewhere — do NOT absorb):**

- **Sweep delivery semantics — owned by WP-46 (BLOCKED-ON-DECISION D-2, D-11).** WP-46 owns wfe F30 (a suppressed reminder is permanently marked sent), wfe F31 (`FOR UPDATE SKIP LOCKED` is a no-op under autocommit, `sweep.rs:68`), **wfe F32 (which preference governs reminders — D-11)**, wfe F33, wfe F34, wfe F35, wfe F37 (the prod-dead `is_reminder_candidate`), wfe F40 (no `LIMIT`), wfe F11, wd F28, wd F38, wd F39. Concretely: **do not change `fetch_candidates` (`sweep.rs:56-80`), `mark_reminder_sent` (:82-96), `sweep_once`'s mark decision (:206-211), `is_reminder_candidate` (:31-44), `should_reset_reminder` (:27-29), or `sweep_invite_nudge_once` / `sweep_invite_expiry_once` / `sweep_invite_auto_decline_once` bodies.** Task 4 rewrites only the five `spawn_*` wrappers, and Task 3 rewrites only `send_reminder`'s body while preserving its `bool` contract byte-for-byte in effect. **The reminder's recipient gate stays exactly `should_email_recipient` (turn_emails_enabled) + `suppress_for_web_presence`**; D-11 changes it afterwards in the one place Task 3 leaves it.
- **`notify_game_emails` for concede/undo/end and their TOCTOU — owned by WP-40 (D-3).** WP-40 owns `undo_game`/`concede_game`/`end_game` in `server_fns.rs` and `run_concede`/`run_end`/`run_undo` in `email/commands.rs`, and will extract `concede_core`/`undo_core`. Task 1 must **not** touch those six functions: they already pass `Some(before)` from a snapshot they hold for their own guards (`server_fns.rs:834`, `:909`, `:966`; `commands.rs:933`, `:981`, `:1040`).
- **Inbound parsing, plumbing and error classification — owned by WP-59** (`specs/WP-59-inbound-processing-quality.md`, already finalized). WP-59 Task 8 rewrites **`notify.rs:8-12`** (the `reply_address` doc + fn) and extends the test at **`notify.rs:500-503`**, and adds `REPLY_DOMAIN`/`invite_reply_address`/`settings_reply_address` there. **This package must not touch `notify.rs:8-12` or that test.** Task 6 below writes a literal `i-noreply@brdg.me` in `proposals.rs`; if WP-59 has already landed, use `crate::email::notify::invite_reply_address("noreply")` instead — the task says so inline. WP-59 also declares "**do not edit `sweep.rs` at all**", so there is no sweep collision from its side.
- **Proposal integrity — owned by WP-44** (`specs/WP-44-proposals-integrity-email-token-leak.md`, already finalized). WP-44 explicitly fences wd F32/F33/F34 **to this package** ("Do not touch mailer gating or email normalization") and does not edit any of the six `RealInviteMailer` methods. WP-44 **does** edit `ProposalPlayerView` (:68-81), `find_proposal_roster`'s SELECT (:511-520), `count_pending_human_invitees` (:701-707) and four server fns, so **proposals.rs line numbers shift if WP-44 lands first**: Tasks 5-7 locate every edit by symbol name.
- **Outbound tokens, metrics, render — owned by WP-60:** wfe F44/F45 (`ensure_email_token` races), wfe F46 (metric before send, `outbound.rs:65`), wfe F47/F48 (mrml + `render_block` silent fallbacks), wfe F49, wfe F50 (`parse_duration` lives in `outbound.rs`), wfe F51. **Do not modify `outbound.rs` or `render.rs` in this package.** Task 2/3 call `try_send_rendered_email` instead of `send_rendered_email` — that is a call-site change in `notify.rs`, not a change to either fn.
- **Unsubscribe / RFC 8058 — owned by WP-58 (D-10).** Task 6 rewrites four footer *strings* in `proposals.rs`; it must keep the word "unsubscribe" in them and must not touch `render.rs:235-242`'s `List-Unsubscribe`/`List-Unsubscribe-Post` headers or add any unsubscribe endpoint.
- **From-header authentication — owned by WP-56 (D-1).** wfe F1's "`None` route falls through to the settings handler" (`inbound.rs:484-486`) is cited below as the *reason* Task 6 rejects a bare no-reply address. **Do not change the route match, `from_matches_verified_email`, or `resolve_user_by_verified_from`.**
- **The two newly-discovered `notify_game_emails` wiring gaps** (email-originated moves and every web game-start path never notify) — see "Cross-package / newly discovered". **Do not fix them here**, even though Task 1 makes the first one a five-line change.
- **Making the notify send loop concurrent.** wfe F43's "consider spawning the send loop" is explicitly **not** done — see Task 2's rejected alternatives.

**Snapshot drift (verified 2026-07-25 against snapshot commit `f8763a5`):**

| File | `diff -ru` snapshot vs live | Effect on this package |
|---|---|---|
| `rust/web/src/proposals.rs` | **NO LONGER EMPTY — WP-44 (`f4e7640`) has landed and rewrote this file**, shifting every citation below | live is now ~3094 lines (spec says 2961); `trait InviteMailer` ~:102 (spec says :110-122); `impl` `notify_owner_decline` ~:278 (spec says :286-328) — all *approximate, verify*. Tasks 5-7 must locate every edit by symbol name, which is what they already instruct |
| `rust/web/src/email/sweep.rs` | **empty, exit 0** | none |
| `rust/web/src/email/notify.rs` | **empty, exit 0** | none |
| `rust/web/src/email/outbound.rs` | **empty, exit 0** | none (read-only here) |
| `rust/web/src/email/render.rs` | **empty, exit 0** | none (read-only here) |
| `rust/web/src/game/mod.rs` | **+1 line**: `pub mod placing;` inserted at live :7 | every citation below live-file; wd F8's `web/src/game/mod.rs:344-347` is live **:345-348** |
| `rust/web/src/game/server_fns.rs` | **large** (#47 concede/end-game: `can_concede`/`can_end_game`/`is_replaced` fields, `count_active_humans`, `end_game`, recent-form 10 -> 5) | wd F8's `server_fns.rs:492-495` is live **:530-533**; the `end_game` server fn (:966) did not exist at snapshot time and is WP-40's, not touched here |
| `rust/web/src/email/commands.rs` | **large** (#47: `end` verb, `run_end`, concede replacement flow) | wfe F42's `commands.rs:426` is live **:427** and is `run_new`'s game-creation site (see the disposition table); the one-token edit in Task 1 is at live **:1275** |

Command used, for re-verification: `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/web/src/<f> /home/beefsack/Development/brdgme/rust/web/src/<f>`. The 2026-07-25 snapshot comparison is stale for `proposals.rs` (WP-44 landed after it), so re-run the diff for that file before trusting any of its numbers.

---

## Findings disposition — every finding re-derived from live source

| Finding | Sev | Finding's own evidence + recommendation (quoted) | Verdict | Re-derivation |
|---|---|---|---|---|
| **wd F8** — before-snapshot errors swallowed | nit | "`find_game_extended(...).await.ok().flatten()` discards a DB error without logging it. If the read fails, email notifications go out with `before = None` … and the failure is invisible. Same pattern in submit_command (server_fns.rs:492-495)". Rec: "Log a warn on the Err branch before falling back to `None`." | **CONFIRMED (defect exactly as described) — recommendation SUPERSEDED by a strictly better fix** | Both sites verified live: `game/mod.rs:345-348` and `server_fns.rs:530-533`, byte-for-byte `.await\n.ok()\n.flatten()`. But the read is **redundant**: `execute_command` (`game/mod.rs:90-92`) already loads the identical `find_game_extended(pool, game_id)` for its own finished/turn guards, immediately before applying the command. Returning it (Task 1) removes the second query on the hottest write path, removes the swallow **by construction** rather than by logging it, and removes the `before = None` failure mode that wfe F42 is about. A warn log would have left all three. |
| **wd F32** — `notify_owner_decline` bypasses invite gates | minor | "`notify_owner_decline` (lines 286-328) checks only that the owner has an email. Every other recipient-facing mailer method applies `invite_recipient_should_send` … An owner who disabled invite emails, or who is actively on the site watching the proposal page, still gets the decline email." Rec: "Apply the same `suppress_for_web_presence` + `invite_recipient_should_send` gate in `notify_owner_decline`." | **CONFIRMED — the finding is right and the recommendation is adopted verbatim** | Verified: `invite_recipient_should_send` appears at proposals.rs :193 (`send_invite`), :250 (`notify_changed_reinvite`), :344 (`notify_cancelled`), :389 (`notify_started`), :434 (`notify_owner_ready`) — **five** of six methods — and is absent from `notify_owner_decline` (:286-328), which goes straight from `owner_recip.email` (:297-299) to render+send. `notify_owner_ready` (:418-464) is the exact structural twin (same recipient: the owner) and is the shape to copy. Do not revert this on "but the owner wants to know": the same argument applies to `notify_owner_ready`, which is gated. |
| **wd F33** — dead Reply-To + reply-inviting footer | minor | "`notify_owner_decline`, `notify_cancelled`, `notify_started`, and `notify_owner_ready` set the reply address to `i-{proposal_id}@brdg.me` (lines 324, 365, 410, 460). `proposal_id` renders as a hyphenated UUID, which is not a player `email_token` (tokens are `Uuid::new_v4().simple()`), so `handle_invite_reply`'s token lookup (inbound.rs:594) always misses and the reply is silently dropped … every one of these emails ends with the footer 'Reply to this email to respond, or unsubscribe anytime.' (lines 315, 356, 401, 451)". Rec: "Use a no-reply address (or the recipient's own token where one exists) and drop the 'Reply to this email to respond' footer from pure notification emails." | **CONFIRMED (defect) / the "no-reply address" half is REJECTED as harmful; the "recipient's own token" half is REJECTED as wrong** | Defect verified end to end: the four `format!("i-{proposal_id}@brdg.me")` at :324/:365/:410/:460 and the four footers at :315/:356/:401/:451 are exactly as cited; proposal-player tokens are minted `Uuid::new_v4().simple().to_string()` (:674, :1146, and :1455 in tests), i.e. 32 hex chars with no hyphens, while `Uuid`'s `Display` is hyphenated — so `find_proposal_player_by_email_token` (:688-698, `WHERE email_token = $1`) can never match, and `handle_invite_reply` logs "unknown invite token; no response" (`inbound.rs:596-599`). **A bare `no-reply@brdg.me` would be worse than the bug:** `parse_reply_address` (`inbound.rs:37-51`) returns `None` for any local part without a `g-`/`i-`/`s-` prefix, and `resend_webhook` routes **`None` into `handle_settings_reply_route`** (`inbound.rs:484-486`), so a reply to a "no-reply" address would be parsed as a *settings command* authenticated by the From header (wfe F1 / WP-56 territory). The `i-` prefix is what keeps a stray reply inert. **The "recipient's own token" option is also wrong**: three of the four mails go to the proposal *owner* or to already-accepted players, and WP-44 Task 2 (wd F29) makes the owner unable to respond at all, so a working owner token would produce a rejection reply instead of a dead one. Fix taken: keep the `i-` route with an explicit, greppable `i-noreply@brdg.me`, and drop the reply promise from exactly those four footers (Task 6). |
| **wd F34** — mailer tasks swallow DB errors; blank-name subjects | minor | "All six `RealInviteMailer` methods use `let Ok(Some(..)) = ... else { return }` on `find_proposal` / `fetch_invite_recipient` inside spawned tasks, so a DB error at send time is indistinguishable from 'recipient opted out' and leaves no trace in logs. `proposal_game_type_name` (lines 170-178) likewise collapses errors into `String::new()`, and owner/invitee name lookups fall back to `unwrap_or_default()` (lines 201-206, 300-305), yielding subjects like ' invite from ' when a lookup fails." Rec: "Log (`tracing::warn!`) on the error arms before returning; consider skipping the send rather than sending an email with blank substitutions." | **CONFIRMED — logging adopted; "skip the send" REJECTED in favour of non-blank fallbacks** | Every cited site verified: the `let Ok(Some(..)) … else { return }` / `else { continue }` forms at :187, :197, :244, :254, :290, :293, :334, :339, :377, :384, :422, :425; `proposal_game_type_name` returning `String::new()` at :172 and `.unwrap_or(None).unwrap_or_default()` at :176-177; the two name fallbacks at :201-206 and :300-305. Subject damage confirmed by reading the `format!`s: `"{game_type_name} invite from {owner_name}"` (:208) becomes `" invite from "`, and the header `"{owner_name} invited you to play {game_type_name}."` (:209-211) becomes `" invited you to play ."`. **"Skip the send" is rejected for the invite path**: `send_invite` is the only thing that tells an invitee they were invited, the nudge sweep marks `nudged_at` unconditionally (wfe F33, WP-46) so there is no retry, and a mail reading "Game invite from Someone" is strictly better than no mail. Fix: log every arm through two small helpers, and make the two fallbacks non-blank (Task 7). |
| **wfe F36** — `send_reminder` duplicates `send_one` | minor | "`send_reminder` re-implements the load-game / find-player / fetch-recipient / suppression / token / palette / render / send pipeline of `notify::send_one` (notify.rs:235-336) nearly line-for-line, differing only in header text, thread parameters, and returning a bool. The two copies of the recipient-gating logic have already drifted (see the turn_emails_enabled finding)." Rec: "Add a `NotifyKind::Reminder` (and a result-returning variant) to `send_one` and delete `send_reminder`'s duplicated body." | **CONFIRMED, recommendation adopted verbatim in shape — but one clause of the evidence is WRONG and must not be repeated** | Duplication verified line by line: `sweep.rs:105-115` == `notify.rs:244-254` (game load), `:117-124` == `:256-266` (player find), `:126-130` == `:268-272` (recipient fetch), `:132-137` ≡ `:279-283` (gate), `:139-149` == `:288-298` (token), `:151-156` == `:300-305` (palette + players), `:161-166` == `:173-178` (board render), `:168-177` ≈ `build_content`'s `EmailContent` (`:180-189`), `:179-186` == `:322-329` (render), `:188-192` ≈ `:331-335` (send). Real differences: header text, `digest: None` vs `digest_since_last_turn`, `game_subject` vs `turn_subject`, `thread_id Some("game-{id}")` vs `None`, hardcoded `is_first_message = false` vs `log_count == 0`, `try_send_rendered_email` vs `send_rendered_email`, and the `bool` return. **The "two copies of the recipient-gating logic have already drifted" clause is false:** `sweep.rs:132-137` (`should_email_recipient` then `suppress_for_web_presence`) and `notify.rs:279-283` (`should_email_recipient && !suppress_for_web_presence`) are logically identical. The real drift wfe F32 describes is between the **SQL** filter (`reminder_emails_enabled`, `sweep.rs:67`) and the Rust gate (`turn_emails_enabled`, `outbound.rs:173`) — a *different* pair, and it is D-11's, not this package's. Task 3 therefore preserves the current gate exactly and leaves D-11 one place to change. |
| **wfe F38** — five copy-pasted spawn/interval loops | nit | "`spawn_turn_reminder_sweep`, `spawn_unverified_email_sweep`, `spawn_invite_nudge_sweep`, `spawn_invite_expiry_sweep`, and `spawn_invite_auto_decline_sweep` each repeat identical tokio interval/`MissedTickBehavior::Skip`/loop boilerplate." Rec: "One `spawn_sweep(name, interval, closure)` helper." | **CONFIRMED, recommendation adopted** | All five verified identical modulo the log label and the call: `sweep.rs:214-229`, `:245-256`, `:308-319`, `:340-351`, `:374-388`. Each is `tracing::info!("<label>: sweep every {:?}", interval); tokio::spawn(async move { let mut tick = tokio::time::interval(interval); tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip); loop { tick.tick().await; <call>.await; } });`. All five keep their public names and signatures (`spawn_periodic_sweeps` at :390-401 and `main.rs` call it). |
| **wfe F39** — reminder threads into the wrong thread | nit | "Turn notifications deliberately de-thread via unique per-turn subjects with `thread_id = None` (notify.rs:309-313), but the reminder for the same turn uses `game_subject` plus `thread_id = Some("game-{id}")` - the thread reserved for eliminated/finished mails. The reminder will not thread with the turn email it nudges and will thread with game-over mails instead." Rec: "Use the same per-turn subject scheme for reminders, or document why the game thread is preferred." | **CONFIRMED, first option taken** | Verified: `notify.rs:309-318` gives `Turn` -> `(turn_subject(...), None)` and `Eliminated | Finished` -> `(game_subject(...), Some("game-{id}"))`; `sweep.rs:158` uses `game_subject` and `:183` passes `Some(&format!("game-{game_id}"))` with `is_first_message = false` (:184), which `render.rs:226-233` turns into `In-Reply-To`+`References: <game-{id}@brdg.me>` — the same refs the game-over mail uses. Putting `Reminder` in the `Turn` arm is not just consistency: while the recipient still holds the turn, no log has been appended (`db::update_game_command_success` is the only writer of `game_logs` on the move path, `db.rs:1957`), so `turn_subject` yields the **same** string the turn email used and clients group the reminder with the mail it nudges — exactly the finding's stated goal. Falls out of Task 3 for free. |
| **wfe F41** — `game_log_count` collapses to 0 | minor | "`game_log_count` returns `unwrap_or(0)` on any DB error. In `send_one` (notify.rs:307-311) that makes `is_first_message = true` … and gives every affected turn email the identical subject `"{type} {game_id}-0"` - the unique-subject-per-turn de-threading lever (documented at notify.rs:68-73) collapses … The same count feeds `failure_report_content` (notify.rs:211)." Rec: "Propagate the error and skip/degrade explicitly (e.g. a timestamp suffix), or at least log and use a sentinel that keeps subjects unique." | **CONFIRMED — the finding is right; the "skip" option is REJECTED, the "timestamp suffix" option is adopted** | Verified: `notify.rs:227-233` is `…fetch_one(pool).await.unwrap_or(0)`, `:307-308` is `let log_count = game_log_count(...).await; let is_first_message = log_count == 0;`, `:311` builds the subject from it, `:211-213` in `failure_report_content` does the same. **Skipping the send is rejected**: the turn email is the mechanism that tells a player it is their turn (and mints their reply token, `:288`), so dropping it on a transient count failure trades a cosmetic threading bug for a stalled game. Fix: `Option<i64>` + a `-t{unix_secs}` fallback subject that preserves uniqueness, and `is_first_message = log_count == Some(0)` so an unknown count degrades to "not first" — which emits `In-Reply-To`/`References` instead of a `Message-Id` (`render.rs:228-233`), and a duplicated `Message-Id` is the one outcome that makes clients *hide* mail. |
| **wfe F42** — `before = None` re-notifies | minor | "With `before = None`, `was_turn` defaults to false for every player, so all players currently on turn are treated as newly-on-turn and emailed. `email/commands.rs:426` calls `notify_game_emails(..., None)` after inbound command handling; in simultaneous-turn games, players already on turn (and already notified) get a duplicate turn email on every such call." Rec: "Make `before` non-optional where call sites can capture it, or treat `None` as 'unknown' and skip transition-based sends rather than defaulting to false." | **CONFIRMED-with-different-fix. The mechanism is real; the cited call site is misidentified and BOTH offered options are unsound as written.** | The mechanism is verified: `notify.rs:480-482` and `:486-488` default `was_turn`/`was_elim` to `false` via `before_player.map(...).unwrap_or(false)`, and `:457` does the same for `was_finished`. **But the cited site is not "after inbound command handling":** snapshot `commands.rs:426` == live `:427` is inside `run_new`, immediately after `create_game_from_service` + `tx.commit()` (live :407-425) — a **brand-new game**, where every on-turn player is genuinely newly on turn and has been notified zero times. All four `None` sites are game *creations* (`commands.rs:427` `run_new`, `commands.rs:1120` and `server_fns.rs:1226` the two restart paths). **Option "make `before` non-optional" cannot apply to them** (there is no before-state to capture). **Option "treat `None` as unknown and skip transition sends" would break them** — a freshly created game would notify nobody, which is a functional regression, not a fix. The genuine duplicate-send path is the *accidental* `None` from wd F8's two `.ok().flatten()` reads, on an existing mid-play game. Task 1 removes it at the source and documents `None` as "brand-new game only", which is the sound reading of "make `before` non-optional where call sites can capture it". A DB-signal alternative was also traced and rejected: `is_turn_at` cannot substitute for the diff, because `db.rs:1921` (`let is_turn_at = if is_turn { now } else { p_is_turn_at };`) bumps it for **every** player on turn after every command, not only newly-on-turn ones. |
| **wfe F43** — per-recipient reload + serial sends (N+1) | minor | "`notify_game_emails` loads `after` via `find_game_extended`, then each `send_one` reloads the same `GameExtended` (notify.rs:244) plus a separate `game_log_count` per recipient - N+1 loads of an already-held snapshot - and sends serially … A 4-human game does 4 game loads, 4 log counts, 4 render calls, and 4 sequential mail API calls." Rec: "Pass the loaded `GameExtended` (and log count) into `send_one`, and consider spawning the send loop (the module contract is already best-effort/log-only)." | **CONFIRMED — pass-the-snapshot adopted; "spawn the send loop" REJECTED** | Verified: `notify_game_emails` loads `after` at `:452`, then every branch calls a wrapper that calls `send_one`, which reloads at `:244` and counts at `:307`. For a 4-human game that is 1 + 4 `find_game_extended` (each of which is itself multiple queries, `db.rs:404-...`) and 4 `COUNT(*)`. Task 2 splits `send_one_loaded` out and the loop reuses `after` + one count: 1 game load and 1 count total. **Spawning is rejected:** the sends would escape the request's tracing span and the `#[sqlx::test]` observation points every existing test uses (`turn_notification_suppressed_per_recipient_by_presence` asserts on the token minted *by the time the call returns*, `notify.rs:670-677`), it would fan out concurrent Resend calls with no rate limiting, and `futures-util` is a non-optional dep issue owned by WP-43 (ws F66). The 4 render calls stay 4 — they are per-recipient by nature (each renders that player's view, `:176`). |

**Counts:** 10 findings — 4 CONFIRMED-as-written (wd F32, wfe F38, wfe F39, wfe F41-mechanism), 4 CONFIRMED-with-a-different-or-narrowed-fix (wd F8, wd F33, wd F34, wfe F43), 1 CONFIRMED-with-corrected-evidence (wfe F36), 1 CONFIRMED-with-different-fix and a misidentified call site (wfe F42). **Zero REJECTED, zero NEEDS-DECISION** — no task below is decision-gated. Overturned recommendations: wd F8 (log -> delete the read), wd F33 (no-reply address -> `i-noreply`; recipient token -> never), wd F34 (skip the send -> non-blank fallback), wfe F41 (skip -> timestamped subject), wfe F42 (both options -> remove the accidental `None`), wfe F43 (spawn -> keep serial).

---

## Landing order and coordination

1. **Task 1** (wd F8 + wfe F42) — `game/mod.rs`, `game/server_fns.rs`, `email/commands.rs` (one token), `email/notify.rs` (doc comment only). Independent of every other package. Do it first: Tasks 2-3 quote `notify_game_emails`' body and Task 1 changes only its doc comment, so no conflict.
2. **Task 2** (wfe F41 + wfe F43) — `email/notify.rs`. Both are surgery on `send_one`; splitting them into separate tasks would mean rewriting the same 100 lines twice, so they land together with each part labelled.
3. **Task 3** (wfe F36 + wfe F39) — `email/notify.rs` + `email/sweep.rs`. Depends on Task 2's `send_one_loaded` / `SendResult`.
4. **Task 4** (wfe F38) — `email/sweep.rs` only, `spawn_*` wrappers. Independent of Task 3 (different functions in the same file); do it after so the file is only rewritten once per region.
5. **Task 5** (wd F32), **Task 6** (wd F33), **Task 7** (wd F34) — `proposals.rs` only. Order is deliberate: Task 5 and Task 6 are line-count-neutral or additive in one function; Task 7 rewrites the `else` arms in all six methods and therefore goes last, locating by symbol name.

**Against other packages:**
- **WP-59** must not have `notify.rs:8-12` (`reply_address`) or its test at `:500-503` touched by this package — none of the tasks below do. If WP-59 lands first, Task 2/3's line numbers in `notify.rs` shift by roughly +15; every edit below is located by symbol name for exactly this reason.
- **WP-46** inherits `sweep.rs`'s `send_reminder` as a 10-line wrapper and `sweep_once` unchanged. Task 3's comment tells WP-46 where wfe F30's outcome distinction and D-11's gate choice now live. If WP-46 lands first, Task 3 must adapt: `sweep_once`'s mark decision may no longer be a `bool`, in which case map `SendResult` onto whatever enum WP-46 introduced instead of onto `bool`, and **keep** the `Suppressed` variant distinct.
- **WP-40** owns the three lifecycle server fns and their email twins; Task 1 leaves all six alone.
- **WP-44** shifts `proposals.rs` numbering; Tasks 5-7 are symbol-located.
- **WP-60** owns `outbound.rs`/`render.rs`; this package only calls into them.

**Declared-path note:** WP-51's package definition lists `rust/web/src/proposals.rs`, `rust/web/src/email/{sweep.rs,notify.rs}`, `rust/web/src/game/{mod.rs,server_fns.rs}`. Task 1 additionally edits **one token** in `rust/web/src/email/commands.rs` (`Ok(())` -> `Ok(_)` at :1275), forced by `execute_command`'s return-type change. That file is WP-59's and WP-40's; the edited line is in neither package's task list (WP-59 Task 9 edits `:1113`; WP-40 owns `:887-1050`). Flagged here rather than silently absorbed.

---

## Task 1: `execute_command` returns the snapshot it already loaded (wd F8 nit, wfe F42 minor)

**Problem (restated):** two call sites read a second copy of the pre-command `GameExtended` purely to hand it to `notify_game_emails`, and both discard read errors: `game/mod.rs:345-348` and `server_fns.rs:530-533` are each

```rust
    let before = crate::db::find_game_extended(&pool, game_id)
        .await
        .ok()
        .flatten();
```

`execute_command` (`game/mod.rs:79-171`) has **already** loaded exactly that value at `:90-92` for its own finished/turn guards. So the second read is a redundant multi-query load on the hottest write path, and when it fails the failure is invisible (wd F8) **and** `notify_game_emails` receives `None`, which `:457`/`:480-482`/`:486-488` interpret as "nothing was true before" — re-notifying every player currently on turn and every already-eliminated player of an existing mid-play game (wfe F42's real mechanism).

**Fix (re-derived):** `execute_command` returns the pre-command snapshot on success. The two callers pass `Some(before)`. `None` then means one thing only — brand-new game — which is documented on `notify_game_emails`. Nothing needs a log because nothing is swallowed any more: if the load fails, `execute_command` fails with `?` at `:91` and no notification is attempted, which is already the correct outcome.

**Why not the finding's `tracing::warn!`:** it would make the failure visible but keep the redundant query, keep the duplicate-send path, and keep `None` ambiguous. This fix removes all three. The cost is one internal signature change with three non-test call sites.

**Edge cases:**
- Ownership: `ge` is borrowed by `player` (`:98-102`) and read at `:120`, `:146`, `:152`, `:158`. All those borrows end before the new `Ok(ge)` at the end, so no clone is needed. Do **not** add `.clone()`.
- Error paths return no snapshot, which is correct: today's callers do not notify on error either (`game/mod.rs:373-419`, `server_fns.rs:557-558`, `commands.rs:1276-1280`).
- `handle_bot_command_event` keeps `-> Result<(), ExecuteCommandError>`; only its internal `match` arm binds the value. `tests/nats_bot_eventing.rs` calls it as `let _ = handle_bot_command_event(...)` (`:319`, `:387`) and is unaffected.
- Nine test call sites in `game/mod.rs` use `.await.unwrap();` as a statement (`:618`, `:676`, `:934`, `:1029`) or `result.is_err()` / `result.unwrap_err()` (`:746`, `:779`, `:809`, `:840`, `:878`). `GameExtended` is not `#[must_use]` (`db.rs:371-378`), so a dropped `Ok` value compiles warning-free and **no existing test needs editing**.
- The snapshot is now read microseconds closer to the write, which strictly improves the diff's accuracy; it can never be *staler* than today's separate read.
- `notify_game_emails` keeps `before: Option<crate::db::GameExtended>`. Do **not** change it to a three-variant enum: the four legitimate `None` sites are all creations and the doc comment plus the removal of the accidental `None` makes the type unambiguous.

**Files:**
- Modify: `rust/web/src/game/mod.rs` (`execute_command` signature + tail; `handle_bot_command_event`; one new test)
- Modify: `rust/web/src/game/server_fns.rs` (`submit_command`)
- Modify: `rust/web/src/email/commands.rs` (one token)
- Modify: `rust/web/src/email/notify.rs` (`notify_game_emails` doc comment only)

**Steps:**

- [ ] Write the failing test. Append inside `mod tests` in `rust/web/src/game/mod.rs` (put it directly after `happy_path_saves_state_and_advances_turn`, i.e. after its closing `    }` at **:658** and before the `#[sqlx::test]` at :660 — locate by name, not by number):

```rust
    // wd F8 / wfe F42: the notification diff baseline must come from the load
    // execute_command already does, not from a second best-effort read whose
    // failure silently becomes "brand-new game".
    #[sqlx::test]
    async fn execute_command_returns_the_pre_command_snapshot(pool: PgPool) {
        let uri = spawn_mock_game_service(|_req| play_response("new_state", vec![1], true)).await;
        let (game_id, _p0, _p1) = make_two_player_game(&pool, &uri).await;
        let broadcaster = make_broadcaster().await;
        let http_client = reqwest::Client::new();
        let jetstream = make_jetstream().await;

        let before = execute_command(
            &pool,
            &http_client,
            &broadcaster,
            &jetstream,
            game_id,
            0,
            "abc".to_string(),
        )
        .await
        .unwrap();

        // The returned snapshot is the state BEFORE the command...
        assert_eq!(before.game.game_state, "initial_state");
        let before_p0 = before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert!(before_p0.game_player.is_turn, "p0 held the turn before");
        let before_p1 = before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(!before_p1.game_player.is_turn, "p1 did not hold it before");

        // ...while the DB now holds the state after it.
        let after = db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.game.game_state, "new_state");
        assert!(
            after
                .game_players
                .iter()
                .find(|p| p.game_player.position == 1)
                .unwrap()
                .game_player
                .is_turn
        );
    }
```

- [ ] Run: `cargo test -p web --features ssr execute_command_returns_the_pre_command_snapshot` — expected **FAIL to compile**: `` expected `GameExtended`, found `()` `` on `before.game.game_state` (or, with the containers absent, a connection failure — in that case run it again after the implementation and rely on `scripts/rust-test.sh` for the real signal).
- [ ] Implement, in `rust/web/src/game/mod.rs`:
  1. Change `execute_command`'s return type at :87 from `) -> Result<(), ExecuteCommandError> {` to:

```rust
) -> Result<crate::db::GameExtended, ExecuteCommandError> {
```

  2. Extend its doc comment (the block immediately above `pub async fn execute_command`) with, as the last paragraph:

```rust
/// On success returns the pre-command `GameExtended` it loaded for its own
/// guards, so the caller can diff it in `email::notify::notify_game_emails`
/// without a second read that could silently fail (wd F8, wfe F42).
```

  3. Replace the tail (`:169-170`, the `broadcast_and_trigger(...)` line followed by `Ok(())`) with:

```rust
    broadcast_and_trigger(pool, broadcaster, jetstream, game_id).await;
    Ok(ge)
```

  4. In `handle_bot_command_event`, **delete** the four lines `:345-348` (`let before = crate::db::find_game_extended(pool, event.game_id)` / `.await` / `.ok()` / `.flatten();`). Keep `let attempt = event.attempt;` (:344) and keep `let result = execute_command(` (:349) exactly as they are.
  5. In the same function, change the success arm `:361` from `Ok(()) => {` to `Ok(before) => {`, and the `notify_game_emails` argument `:368` from `before,` to `Some(before),`.
- [ ] Implement, in `rust/web/src/game/server_fns.rs` (`submit_command`):
  1. **Delete** lines `:530-533` (`let before = crate::db::find_game_extended(&pool, game_id)` / `.await` / `.ok()` / `.flatten();`) and the blank line `:534` they leave behind. The line before the range is `:528`'s `.ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;`; the line after is `:535`'s `match super::execute_command(`.
  2. Change `:546` from `        Ok(()) => {` to `        Ok(before) => {` and `:552` from `                before,` to `                Some(before),`.
- [ ] Implement, in `rust/web/src/email/commands.rs`: change line `:1275` from `        Ok(()) => Ok(CommandReply::GameMove),` to `        Ok(_) => Ok(CommandReply::GameMove),`. **Change nothing else in this file** — in particular do not add a `notify_game_emails` call here (see "Cross-package / newly discovered" item 1).
- [ ] Implement, in `rust/web/src/email/notify.rs`: extend `notify_game_emails`' doc comment (`:442-444`) to read exactly:

```rust
/// Diffs `before`/`after` game state and fires the appropriate notification for
/// each human player. Mail failures are isolated: every send logs and returns;
/// this never fails the game operation.
///
/// `before` is the pre-command snapshot to diff against; `game::execute_command`
/// returns it. `None` means **brand-new game** - nobody has been notified yet,
/// so every player currently on turn counts as newly on turn. NEVER pass `None`
/// to mean "I could not read the snapshot": that re-notifies every player
/// already on turn (wfe F42).
```

- [ ] Run: `cargo test -p web --features ssr game::` — the new test PASSES and all pre-existing `game::tests` PASS unmodified (`happy_path_saves_state_and_advances_turn`, `stale_state_conflict_*`, `user_error_propagated_and_no_db_write`, `system_error_propagated_and_no_db_write`, `remaining_input_returns_err_and_no_db_write`, `finished_status_persists_placings`, and the bot-event tests).
- [ ] Run: `cargo test -p web --features ssr email::` and `cargo test -p web --features ssr --test nats_bot_eventing` — PASS.
- [ ] Verify no swallow remains: `rg -n 'ok\(\)\s*$' rust/web/src/game/mod.rs rust/web/src/game/server_fns.rs` — expected **no** hits in a `find_game_extended` chain (before this task there were exactly two: `game/mod.rs:347`, `server_fns.rs:532`).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/game/mod.rs rust/web/src/game/server_fns.rs rust/web/src/email/commands.rs rust/web/src/email/notify.rs` ; message: `fix(game): return the pre-command snapshot instead of re-reading it best-effort (wd F8 wfe F42, WP-51)`

NOTE: this task deletes 4 lines around `game/mod.rs:345` and 5 around `server_fns.rs:530`, and adds ~40 test lines to `game/mod.rs`. No later task cites either file.

---

## Task 2: one game load and one honest log count per notification batch (wfe F41 minor, wfe F43 minor)

**Problem (restated):** two defects in the same function.

*wfe F43:* `notify_game_emails` loads `after` at `notify.rs:452`, then each per-player wrapper calls `send_one`, which loads the **same** game again at `:244` and runs its own `COUNT(*)` at `:307`. A 4-human game performs 5 `find_game_extended` calls (each several queries, `db.rs:404+`) and 4 counts where 1 and 1 suffice.

*wfe F41:* `game_log_count` (`:227-233`) ends in `.unwrap_or(0)`, so any transient DB error makes `is_first_message` true (`:308`) and gives the mail the subject `"{type} {game_id}-0"` (`:311`) — collapsing the deliberate unique-subject-per-turn de-threading lever documented at `:68-73`, so clients thread unrelated turns together. The same value feeds `failure_report_content` (`:211-213`).

**Fix (re-derived):**
1. `game_log_count` returns `Option<i64>` and logs on error. No `?`/`anyhow` propagation: both callers must still send, so they need a value, not an error.
2. A new `turn_subject_or_fallback` keeps the *one* property that matters — uniqueness per turn — by falling back to a `-t{unix_secs}` suffix. `is_first_message` becomes `log_count == Some(0)`, so an unknown count degrades to "not first" (`In-Reply-To`/`References`, `render.rs:230-233`) rather than emitting a `Message-Id` that could duplicate another mail's.
3. `send_one` splits into `send_one_loaded` (takes an already-loaded `&GameExtended` + the count) and a thin `send_one` (loads both, then delegates). `notify_game_emails` loads once and calls `send_one_loaded` per recipient.
4. `send_one_loaded` returns a `SendResult` so Task 3 can delete `sweep::send_reminder`'s body. Every existing caller ignores it, which is exactly today's behaviour.
5. `send_turn_notification`, `send_elimination_notification` and `send_game_finished_notification` are deleted: each existed only to name a `NotifyKind` for `notify_game_emails`, which now names it directly. `send_turn_digest` and `send_turn_digest_forced` **stay** — they have external callers (`auth/server.rs:888`, `email/commands.rs:461`).

**Edge cases:**
- **Query-order change:** `send_one` now fetches the log count *before* the recipient gate, where the old code fetched it after (`:307`). One extra `COUNT(*)` runs for a recipient that turns out to be suppressed on the three single-send paths (digest, forced digest, and Task 3's reminder). That query is index-backed (`idx_game_logs_game_id`, `web/migrations/001_initial_schema.sql:387`) and those paths run at most once per game per user action, while the batch path saves 3 full `GameExtended` loads in a 4-human game. Accepted deliberately; do not "optimise" it back with a lazily-fetched closure.
- `turn_subject`'s existing signature and behaviour are untouched, so `inbound.rs:1761`'s assertion `content.subject == turn_subject(&ge.game_type.name, game_id, 0)` keeps passing: that test seeds a game with zero logs (`inbound.rs:1690-1710`), so the count is a genuine `Some(0)`.
- The fallback subject is only unique per *second*. Two turns of one game inside one second with a failing count query would collide; that is acceptable and strictly better than every turn colliding forever.
- `game_subject` and `Some(format!("game-{game_id}"))` for `Eliminated`/`Finished` are unchanged. Do not de-thread game-over mail.
- `send_one_loaded` takes `&crate::db::GameExtended`; `game_subject` (`:54-66`) and `build_content` (`:149`) already take references, so only the `&ge` -> `ge` call-site sigils change.
- The finished branch of `notify_game_emails` (`:458-468`) keeps its early `return` — a game that just finished must not also emit turn mail.
- Two new `tracing` lines cover the previously silent `_ => return` on the recipient fetch (`:271`). That is the same "no silent drops on the notify path" rule wd F8 applies; it is not an invitation to add logs elsewhere.
- `SendResult` must be `pub` (Task 3's `sweep.rs` matches on it) and must **not** derive anything or grow variants beyond the three.

**Files:**
- Modify: `rust/web/src/email/notify.rs` (`game_log_count`, new `turn_subject_or_fallback`, new `SendResult`, `failure_report_content`, `send_one` -> `send_one_loaded` + `send_one`, delete three wrappers, `notify_game_emails`, tests)

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/web/src/email/notify.rs`, after `turn_subject_is_name_id_turn_and_unique_per_turn` (`:506-513`):

```rust
    #[test]
    fn turn_subject_fallback_stays_unique_when_the_count_is_unknown() {
        // wfe F41: a failed count must not collapse every turn onto
        // "{type} {id}-0" - that is the de-threading lever at :68-73.
        let id = uuid::Uuid::new_v4();
        assert_eq!(
            turn_subject_or_fallback("Acquire", id, Some(7)),
            turn_subject("Acquire", id, 7)
        );
        let fallback = turn_subject_or_fallback("Acquire", id, None);
        assert_ne!(fallback, turn_subject("Acquire", id, 0));
        assert!(fallback.starts_with(&format!("Acquire {id}-t")));
    }
```

  and, at the end of the same module (after `turn_notification_suppressed_per_recipient_by_presence`):

```rust
    // wfe F43 lock-in: one loaded snapshot drives the whole roster, and the
    // diff still only mails the player who is NEWLY on turn.
    #[sqlx::test]
    async fn notify_game_emails_only_mails_the_newly_on_turn_player(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        let before = crate::db::find_game_extended(&pool, game_id)
            .await
            .unwrap()
            .expect("game exists");
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp1)
            .execute(&pool)
            .await
            .unwrap();

        notify_game_emails(
            None,
            &pool,
            &reqwest::Client::new(),
            game_id,
            Some(before),
        )
        .await;

        // A reply token is minted only for a recipient we actually mail.
        assert!(
            email_token(&pool, gp1).await.is_some(),
            "the newly-on-turn player must be mailed"
        );
        assert!(
            email_token(&pool, gp0).await.is_none(),
            "a player whose turn state did not change must not be mailed"
        );
    }

    // wfe F42 companion: `None` means brand-new game, so everyone currently on
    // turn is newly on turn. This is the ONLY intended meaning of `None`.
    #[sqlx::test]
    async fn notify_game_emails_treats_none_as_a_brand_new_game(pool: sqlx::PgPool) {
        let (game_id, players) = seed_game_with_emailable_players(&pool, 2).await;
        let (_u0, gp0) = players[0];
        let (_u1, gp1) = players[1];
        sqlx::query("UPDATE game_players SET is_turn = true WHERE id = $1")
            .bind(gp1)
            .execute(&pool)
            .await
            .unwrap();

        notify_game_emails(None, &pool, &reqwest::Client::new(), game_id, None).await;

        assert!(email_token(&pool, gp1).await.is_some());
        assert!(email_token(&pool, gp0).await.is_none());
    }
```

- [ ] Run: `cargo test -p web --features ssr email::notify::tests::turn_subject_fallback` — expected FAIL to compile: `cannot find function turn_subject_or_fallback in this scope`.
- [ ] Implement (wfe F41), replacing `notify.rs:224-233` (the three-line doc comment starting `/// How many logs the game has (plain query; defaults to 0 on error). Every` through the closing `}` of `game_log_count`) with:

```rust
/// How many logs the game has, or `None` when the count could not be read.
/// Every command appends >=1 log, so this is a monotonic turn counter used both
/// to detect the opening turn and to build the per-turn de-threaded subject. A
/// failed read must NOT collapse to `0`: that gave every affected turn email the
/// identical subject and made it claim to be the game's first message, breaking
/// the de-threading lever documented above (wfe F41).
async fn game_log_count(pool: &sqlx::PgPool, game_id: uuid::Uuid) -> Option<i64> {
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_logs WHERE game_id = $1")
        .bind(game_id)
        .fetch_one(pool)
        .await
    {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::error!("notify: log count failed for game {}: {}", game_id, e);
            None
        }
    }
}

/// The per-turn de-threaded subject, with a timestamped fallback when the turn
/// counter is unavailable. Uniqueness per turn is the one property this subject
/// may never lose (wfe F41).
pub fn turn_subject_or_fallback(
    game_type_name: &str,
    game_id: uuid::Uuid,
    turn: Option<i64>,
) -> String {
    match turn {
        Some(t) => turn_subject(game_type_name, game_id, t),
        None => format!(
            "{game_type_name} {game_id}-t{}",
            time::OffsetDateTime::now_utc().unix_timestamp()
        ),
    }
}
```

- [ ] Implement (wfe F41), in `failure_report_content`: replace `:213`'s `        subject: turn_subject(&ge.game_type.name, ge.game.id, log_count),` with:

```rust
        subject: turn_subject_or_fallback(&ge.game_type.name, ge.game.id, log_count),
```

  (`:211`'s `let log_count = game_log_count(pool, ge.game.id).await;` is unchanged — its type simply becomes `Option<i64>`.)
- [ ] Run: `cargo test -p web --features ssr email::notify::tests::turn_subject_fallback` — PASSES.
- [ ] Implement (wfe F43), replacing `notify.rs:235-336` in full (`async fn send_one(` through its closing `}`; the line before the range is `:233`'s `}` closing `game_log_count`, now moved, and the line after is `:337`'s blank line followed by `pub async fn send_turn_notification(`) with:

```rust
/// What one notification attempt did. `Suppressed` covers every deliberate
/// non-send: opted out, bot slot, no verified address, or active on the web.
/// Only the turn-reminder sweep currently reads this (wfe F36); every other
/// caller is best-effort and drops it.
pub enum SendResult {
    Sent,
    Suppressed,
    Failed,
}

/// Sends one notification for an ALREADY-LOADED game. `log_count` is that
/// game's log count (`None` = it could not be read). Split out of `send_one` so
/// `notify_game_emails` can load the game and the count once for the whole
/// roster instead of once per recipient (wfe F43).
async fn send_one_loaded(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    ge: &crate::db::GameExtended,
    log_count: Option<i64>,
    game_player_id: uuid::Uuid,
    kind: NotifyKind,
    mode: SendMode,
) -> SendResult {
    let game_id = ge.game.id;

    let recipient_player = match ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == game_player_id)
    {
        Some(p) => p,
        None => {
            tracing::warn!("notify: player {} not in game {}", game_player_id, game_id);
            return SendResult::Failed;
        }
    };

    let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            tracing::warn!("notify: no recipient row for player {}", game_player_id);
            return SendResult::Failed;
        }
        Err(e) => {
            tracing::error!(
                "notify: failed to load recipient {}: {}",
                game_player_id,
                e
            );
            return SendResult::Failed;
        }
    };

    let should_send = match mode {
        SendMode::Forced => recipient.email.is_some() && !recipient.is_bot,
        SendMode::BypassSuppression => {
            recipient.email.is_some() && !recipient.is_bot && recipient.turn_emails_enabled
        }
        SendMode::Normal => {
            crate::email::outbound::should_email_recipient(&recipient)
                && !crate::email::outbound::suppress_for_web_presence(pool, recipient.user_id).await
        }
    };
    if !should_send {
        return SendResult::Suppressed;
    }

    let token = match crate::email::outbound::ensure_email_token(pool, game_player_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "notify: failed to ensure email token for {}: {}",
                game_player_id,
                e
            );
            return SendResult::Failed;
        }
    };

    let palette = crate::email::render::palette_for_slug(recipient.theme_slug.as_deref());
    let players: Vec<brdgme_markup::Player> = ge
        .game_players
        .iter()
        .map(|p| crate::email::render::player_for_slot(p.name(), &p.game_player.color, palette))
        .collect();

    let is_first_message = log_count == Some(0);
    let (subject, thread_id) = match &kind {
        NotifyKind::Turn => (
            turn_subject_or_fallback(&ge.game_type.name, game_id, log_count),
            None,
        ),
        NotifyKind::Eliminated | NotifyKind::Finished => (
            game_subject(ge, recipient_player),
            Some(format!("game-{game_id}")),
        ),
    };

    let content = build_content(pool, http_client, ge, recipient_player, kind, subject).await;

    let rendered = crate::email::render::render_game_email(
        &content,
        palette,
        &players,
        thread_id.as_deref(),
        is_first_message,
        &reply_address(&token),
    );

    // Unreachable: every `should_send` arm above requires `email.is_some()`.
    let to = match recipient.email.clone() {
        Some(e) => e,
        None => return SendResult::Suppressed,
    };
    if crate::email::outbound::try_send_rendered_email(resend, rendered, &to).await {
        SendResult::Sent
    } else {
        SendResult::Failed
    }
}

/// Loads the game and its log count, then sends one notification. Use
/// `send_one_loaded` directly when a caller already holds both (wfe F43).
async fn send_one(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
    kind: NotifyKind,
    mode: SendMode,
) -> SendResult {
    let ge = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            tracing::warn!("notify: game {} not found", game_id);
            return SendResult::Failed;
        }
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return SendResult::Failed;
        }
    };
    let log_count = game_log_count(pool, game_id).await;
    send_one_loaded(
        resend,
        pool,
        http_client,
        &ge,
        log_count,
        game_player_id,
        kind,
        mode,
    )
    .await
}
```

- [ ] Implement (wfe F43): **delete** `send_turn_notification` (`:338-355`), `send_elimination_notification` (`:404-421`) and `send_game_finished_notification` (`:423-440`) in full, including each one's doc comment where present (`send_turn_notification` and the two `send_game_*` have none; `send_turn_digest` `:357-361` and `send_turn_digest_forced` `:381-384` do — **keep those two functions and their doc comments**). After the edit the public send surface is exactly `send_turn_digest`, `send_turn_digest_forced`, `notify_game_emails`, plus the primitives.
- [ ] Implement (wfe F43): change the two `send_turn_digest*` bodies' trailing `.await;` calls from `send_one(...)` statements to `let _ = send_one(...)` — i.e. in each of the two functions replace `    send_one(` with `    let _ = send_one(` (their `SendResult` is deliberately dropped: both are fire-and-forget user pulls). Verify with `rg -n 'let _ = send_one' rust/web/src/email/notify.rs` -> 2 hits.
- [ ] Implement (wfe F43), replacing `notify_game_emails`' body (`:451-494`, i.e. everything between the `) {` on `:451` and the function's closing `}` on `:494`; keep the Task 1 doc comment and the signature) with:

```rust
    let after = match crate::db::find_game_extended(pool, game_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("notify: failed to load game {}: {}", game_id, e);
            return;
        }
    };
    // One load and one count for the whole roster (wfe F43).
    let log_count = game_log_count(pool, game_id).await;

    let was_finished = before.as_ref().map(|b| b.game.is_finished).unwrap_or(false);
    if after.game.is_finished && !was_finished {
        for p in after
            .game_players
            .iter()
            .filter(|p| p.user.is_some() && p.game_bot.is_none())
        {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Finished,
                SendMode::Normal,
            )
            .await;
        }
        return;
    }

    for p in after
        .game_players
        .iter()
        .filter(|p| p.user.is_some() && p.game_bot.is_none())
    {
        let before_player = before.as_ref().and_then(|b| {
            b.game_players
                .iter()
                .find(|bp| bp.game_player.position == p.game_player.position)
        });
        let was_turn = before_player
            .map(|b| b.game_player.is_turn)
            .unwrap_or(false);
        if p.game_player.is_turn && !was_turn {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Turn,
                SendMode::Normal,
            )
            .await;
        }
        let was_elim = before_player
            .map(|b| b.game_player.is_eliminated)
            .unwrap_or(false);
        if p.game_player.is_eliminated && !was_elim {
            send_one_loaded(
                resend,
                pool,
                http_client,
                &after,
                log_count,
                p.game_player.id,
                NotifyKind::Eliminated,
                SendMode::Normal,
            )
            .await;
        }
    }
```

  (The `Ok(None) => return` / `Err` split replaces the old `_ => return` at `:454`, so a missing game and a failed load are no longer indistinguishable — same rule as wd F8.)
- [ ] Implement: update the existing presence test. In `turn_notification_suppressed_per_recipient_by_presence` (`:655-678`) replace the two calls at `:667-668`

```rust
        send_turn_notification(None, &pool, &http, game_id, active_gp).await;
        send_turn_notification(None, &pool, &http, game_id, inactive_gp).await;
```

  with

```rust
        // `send_turn_notification` was a one-line wrapper around this; the
        // wrapper is gone (wfe F43) and the tests exercise the real path.
        send_one(
            None,
            &pool,
            &http,
            game_id,
            active_gp,
            NotifyKind::Turn,
            SendMode::Normal,
        )
        .await;
        send_one(
            None,
            &pool,
            &http,
            game_id,
            inactive_gp,
            NotifyKind::Turn,
            SendMode::Normal,
        )
        .await;
```

  Its two assertions (`:670-677`) are unchanged. This is the **only** permitted edit to an existing test in this task.
- [ ] Run: `cargo test -p web --features ssr email::notify` — all tests PASS, including the two new `#[sqlx::test]`s, the updated presence test, and `notify_game_emails_noop_for_missing_game` (`:561-571`).
- [ ] Run: `cargo test -p web --features ssr email::inbound::tests::failure_report_is_dethreaded_and_sets_reply_to` — PASSES unmodified (the game it seeds has zero logs, so the count is a real `Some(0)` and the subject is still `turn_subject(name, id, 0)`).
- [ ] Verify no caller of the deleted wrappers survives: `rg -n 'send_turn_notification|send_elimination_notification|send_game_finished_notification' rust/web/src rust/web/tests` — expected **zero** hits.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/email/notify.rs` ; message: `perf(email): one game load and one honest log count per notification batch (wfe F41 wfe F43, WP-51)`

NOTE: this task rewrites most of `notify.rs` between `game_log_count` and `notify_game_emails`. Task 3 locates every edit **by symbol name**.

---

## Task 3: delete `send_reminder`'s copy of `send_one` (wfe F36 minor, wfe F39 nit)

**Problem (restated):** `sweep::send_reminder` (`sweep.rs:98-193`) is a ~90-line re-implementation of `notify::send_one` — see the disposition table for the line-by-line correspondence. Because it is a copy, it also carries its own threading choice: `game_subject` (`:158`) plus `thread_id = Some("game-{game_id}")` and `is_first_message = false` (`:183-184`), which `render.rs:226-233` renders as `In-Reply-To`/`References: <game-{id}@brdg.me>` — the thread the *game-over* mail owns — while the turn email it nudges deliberately de-threads with a unique per-turn subject and no threading headers (`notify.rs:309-313`). So the reminder never threads with the mail it is reminding about (wfe F39).

**Fix (re-derived):** add `NotifyKind::Reminder`, put it in the `Turn` subject arm, skip the digest for it, and reduce `sweep::send_reminder` to a mapping from `SendResult` to today's `bool`.

**Behaviour deltas this deliberately does and does not introduce:**
- **Changed (wfe F39, intended):** the reminder's `Subject` becomes `"{Game type} {game_id}-{n}"` and it carries no `In-Reply-To`/`References`. Because no log is appended while the recipient still holds the turn (`db::update_game_command_success` is the only `game_logs` writer on the move path, `db.rs:1957`), `n` equals the value the turn email used, so the reminder groups with it by subject.
- **Unchanged (required):** the recipient gate stays `should_email_recipient` + `suppress_for_web_presence` (`SendMode::Normal`), i.e. `turn_emails_enabled` — **not** `reminder_emails_enabled`. That is wfe F32 / **D-11** and belongs to WP-46. Task 3 leaves exactly one place to change it.
- **Unchanged (required):** `send_reminder`'s `bool` contract, including its bug. Mapping proof against the live code, arm by arm — old `false`: game load `Err`/`Ok(None)` (`:107-114`), player not in game (`:123`), recipient fetch `Err`/`Ok(None)` (`:129`), token error (`:147`), `recipient.email == None` after the gate (`:190`, unreachable), send failure (`:192`); old `true`: `!should_email_recipient` (`:133`), presence-suppressed (`:137`), send success (`:192`). New: `Failed` -> `false` covers every one of the old `false` arms except the unreachable `email == None`, which becomes `Suppressed` -> `true`; `Suppressed` -> `true` covers both deliberate skips; `Sent` -> `true`. **Identical for every reachable input.**
- **Unchanged (deliberate):** the reminder carries no "Since last time" digest. `build_content` would otherwise add one (`notify.rs:171`), costing a query and repeating the lines the turn email already showed to a player who has not moved.
- `is_first_message` is irrelevant for the reminder now: with `thread_id = None`, `render.rs:226` emits no threading header at all.

**Edge cases:**
- `reminder_header_text` moves from `sweep.rs:46-48` to `notify.rs` (it is now `build_content`'s input). Its unit test moves with it, verbatim in substance.
- `sweep.rs`'s `use sqlx::PgPool;` and `use uuid::Uuid;` (`:6-7`) both stay used (`fetch_candidates`, `mark_reminder_sent`, the `spawn_*` fns).
- The existing `#[sqlx::test] turn_reminder_suppressed_by_recipient_presence` (`sweep.rs:1005-1045`) calls `send_reminder(...)` and discards the `bool` (`:1015`, `:1034`) — it stays **unmodified** and still passes: `send_one_loaded` returns before `ensure_email_token` when suppressed, so the "no token minted" assertion holds, and mints one when not suppressed.
- Do **not** make `send_reminder` public, and do not move it into `notify.rs`: `sweep_once`'s mark decision is WP-46's and must keep a local seam.

**Files:**
- Modify: `rust/web/src/email/notify.rs` (`NotifyKind`, `build_content`, `send_one_loaded`'s subject arm, new `reminder_header_text`, new `send_turn_reminder`, tests)
- Modify: `rust/web/src/email/sweep.rs` (delete `reminder_header_text` + its test, replace `send_reminder`'s body)

**Steps:**

- [ ] Implement, in `rust/web/src/email/notify.rs`:
  1. Replace the `NotifyKind` enum (`:110-114`) with:

```rust
enum NotifyKind {
    Turn,
    /// The turn-reminder sweep's nudge for a turn already notified. Shares
    /// `Turn`'s de-threaded per-turn subject so it groups with the mail it
    /// nudges (wfe F39).
    Reminder,
    Eliminated,
    Finished,
}
```

  2. Add, immediately after `turn_header_text` (`:14-16`):

```rust
pub fn reminder_header_text(player_name: &str) -> String {
    format!("Still your turn, {player_name}.")
}
```

  3. In `build_content`, change `let header = Some(match kind {` to `let header = Some(match &kind {` and add a `Reminder` arm so the match reads (arms in this order):

```rust
    let header = Some(match &kind {
        NotifyKind::Turn => turn_header_text(recipient_player.name()),
        NotifyKind::Reminder => reminder_header_text(recipient_player.name()),
        NotifyKind::Eliminated => eliminated_header_text(recipient_player.name()),
        NotifyKind::Finished => {
```

  (the `Finished` arm's body is unchanged.)
  4. In `build_content`, replace the digest line (`let digest = digest_since_last_turn(pool, ge, recipient_player).await;`) with:

```rust
    // A reminder carries no digest: the turn email it nudges already showed
    // those lines and the recipient has not moved since (wfe F36).
    let digest = match &kind {
        NotifyKind::Reminder => None,
        _ => digest_since_last_turn(pool, ge, recipient_player).await,
    };
```

  5. In `send_one_loaded`, change the subject match's first arm from `        NotifyKind::Turn => (` to `        NotifyKind::Turn | NotifyKind::Reminder => (`.
  6. Add, immediately after `send_turn_digest_forced`:

```rust
/// Sends the turn REMINDER for one player, reporting what happened so the sweep
/// can decide whether to mark the reminder as sent. Replaces the ~90-line copy
/// of this pipeline that used to live in `email::sweep` (wfe F36).
pub async fn send_turn_reminder(
    resend: Option<&resend_rs::Resend>,
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_id: uuid::Uuid,
    game_player_id: uuid::Uuid,
) -> SendResult {
    send_one(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
        NotifyKind::Reminder,
        SendMode::Normal,
    )
    .await
}
```

  7. Add the moved unit test to `mod tests`, next to `turn_and_eliminated_headers_contain_name`:

```rust
    #[test]
    fn reminder_header_contains_name() {
        let h = reminder_header_text("Alice");
        assert!(h.contains("Alice"));
        assert!(h.contains("Still your turn"));
    }
```

- [ ] Implement, in `rust/web/src/email/sweep.rs`:
  1. **Delete** `:46-48` (`fn reminder_header_text(player_name: &str) -> String { … }`) and the blank line after it. The line before the range is `:44`'s `}` closing `is_reminder_candidate`; the line after is `:50`'s `#[derive(Debug, sqlx::FromRow)]`.
  2. **Delete** the test `reminder_header_contains_name` at `:533-538` (from `    #[test]` through its closing `    }`) and the blank line after it. The line before the range is `:531`'s `    }` closing `sweep_interval_parses_custom`; the line after is `:540`'s `    #[sqlx::test]` for `fetch_candidates_returns_due_players`.
  3. Replace `:98-193` in full (`async fn send_reminder(` through its closing `}`; the line before the range is `:96`'s `}` closing `mark_reminder_sent`, the line after is `:194`'s blank line followed by `async fn sweep_once(`) with:

```rust
/// Sends the turn reminder for one candidate, returning whether `sweep_once`
/// should mark the reminder as sent.
///
/// The body used to be a ~90-line copy of `notify::send_one` (wfe F36). The
/// `bool` contract is preserved EXACTLY, bug included: a deliberate skip
/// (opted out, or active on the web) still counts as "handled", so the sweep
/// marks the reminder sent and the player is never reminded for this turn.
/// That is wfe F30 and it is WP-46's to fix under D-2 - do not change the
/// mapping here. Likewise the gate behind `SendResult::Suppressed` is still
/// `turn_emails_enabled`, not `reminder_emails_enabled`: that is D-11, and it
/// changes inside `send_one_loaded`, not by reintroducing a gate here.
async fn send_reminder(
    resend: Option<&resend_rs::Resend>,
    pool: &PgPool,
    http_client: &reqwest::Client,
    game_id: Uuid,
    game_player_id: Uuid,
) -> bool {
    use crate::email::notify::SendResult;
    match crate::email::notify::send_turn_reminder(
        resend,
        pool,
        http_client,
        game_id,
        game_player_id,
    )
    .await
    {
        SendResult::Sent | SendResult::Suppressed => true,
        SendResult::Failed => false,
    }
}
```

- [ ] Run: `cargo test -p web --features ssr email::sweep` — every test PASSES **unmodified**, including `turn_reminder_suppressed_by_recipient_presence` (`:1005-1045`), the three `fetch_candidates_*` tests and both `mark_reminder_sent`/`reset_reminder` tests.
- [ ] Run: `cargo test -p web --features ssr email::notify` — PASS, including the moved `reminder_header_contains_name`.
- [ ] Verify the duplication is gone: `wc -l rust/web/src/email/sweep.rs` — expect a net reduction of roughly **75-85** lines from 1046 (-96 `send_reminder` body, +~29 wrapper, -4 header fn + blank, -7 test + blank), i.e. somewhere near **965**. The exact number is not the assertion; the next check is. `rg -n 'palette_for_slug|render_game_email|ensure_email_token|EmailContent' rust/web/src/email/sweep.rs` — expected **zero** hits (before this task: 4, at `:151`, `:179`, `:139`, `:168`).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/email/notify.rs rust/web/src/email/sweep.rs` ; message: `refactor(email): reminders go through notify::send_one with the turn subject scheme (wfe F36 wfe F39, WP-51)`

**Manual verification (wfe F39 — the only way to observe the header change; both "before" claims are proved from source, not assumed):**

Run the web binary in dev with **no** `RESEND_API_KEY`, so `try_send_rendered_email` prints the mail instead of sending it (`outbound.rs:58-64`, which prints `==> GAME EMAIL for {to}` / `Subject: …` / `Reply-To: …`). Set `TURN_REMINDER_AFTER=1s` and `TURN_REMINDER_SWEEP_INTERVAL=10s`, put a human player on turn in an unfinished game with `turn_emails_enabled = true`, a verified primary address, and `last_active_at` older than 10 minutes.

| | Before this task | After this task |
|---|---|---|
| Reminder `Subject:` | `"{Game type} with {opponent names}"` (`sweep.rs:158` -> `notify::game_subject`) | `"{Game type} {game_id}-{n}"`, identical to the turn email's subject for that turn |
| Threading headers | `In-Reply-To` and `References` = `<game-{game_id}@brdg.me>` (`sweep.rs:183-184` + `render.rs:230-233`) | none emitted (`thread_id = None` -> `render.rs:226` skips the block) |
| Body | header "Still your turn, {name}.", no digest, board, "You can", links, reply-to footer | **identical** |
| `Reply-To:` | `g-{token}@brdg.me` | `g-{token}@brdg.me` (unchanged) |

The dev printout shows `Subject` and `Reply-To` directly; the threading headers are only visible in a real send (or by asserting on `render_game_email`'s output in a future test), so for those rely on the source trace above plus the absence of `Some(&format!("game-{game_id}"))` in the new code path.

---

## Task 4: one sweep-spawning helper (wfe F38 nit)

**Problem (restated):** five functions repeat the same 8 lines. Verified identical modulo label and call at `sweep.rs:214-229`, `:245-256`, `:308-319`, `:340-351`, `:374-388`.

**Fix (re-derived):** one generic `spawn_sweep(name, interval, run)` where `run: FnMut() -> Fut`. Each caller clones its captured handles inside the closure, so the returned future owns everything and is `Send + 'static` as `tokio::spawn` requires. All five public fns keep their names and signatures, so `spawn_periodic_sweeps` (`:390-401`) and `main.rs` are untouched.

**Why `FnMut() -> Fut` and not an `async` closure:** `async` closures are stable in this toolchain, but the plain `FnMut() -> impl Future` form needs no borrow gymnastics and is the boring option `docs/CODING.md` asks for. Do not use `AsyncFnMut`.

**Edge cases:**
- The per-tick clones are `PgPool` (an `Arc` internally), `reqwest::Client` (`Arc`), `Option<resend_rs::Resend>` and `GameBroadcaster` — all already cloned once per spawn today (`:396-400`). One extra cheap clone per 15-minute tick.
- The log text must stay byte-identical in shape (`"{name}: sweep every {interval:?}"`), so existing log-scraping/dashboards keep matching: `turn_reminder`, `unverified_email_expiry`, `invite_nudge`, `invite_expiry`, `invite_auto_decline`.
- `MissedTickBehavior::Skip` must be preserved: it is what stops a backlog of ticks after a slow sweep.
- Each closure must clone **inside** the closure body, before the `async move` block. Cloning outside would move the value into the first future and fail to compile on the second tick.
- No test exists for the spawn loops and none is added: a test would have to wait for real time. Verification is compile + clippy + the startup-log checklist below.

**Files:**
- Modify: `rust/web/src/email/sweep.rs` (add `spawn_sweep`, rewrite the five `spawn_*` bodies)

**Steps:**

- [ ] Implement: add the helper immediately above `spawn_turn_reminder_sweep` (locate by name — Task 3 shifted this file's numbering):

```rust
/// Spawns one periodic sweep: a `MissedTickBehavior::Skip` interval that runs
/// `run()` every tick, forever. Five sweeps used to repeat this loop verbatim
/// (wfe F38).
fn spawn_sweep<F, Fut>(name: &'static str, interval: std::time::Duration, mut run: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    tracing::info!("{name}: sweep every {interval:?}");
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            run().await;
        }
    });
}
```

- [ ] Implement: replace the five `spawn_*` function bodies with these, keeping each signature exactly as it is today:

```rust
pub fn spawn_turn_reminder_sweep(
    pool: PgPool,
    resend: Option<resend_rs::Resend>,
    http_client: reqwest::Client,
) {
    spawn_sweep("turn_reminder", sweep_interval(), move || {
        let pool = pool.clone();
        let resend = resend.clone();
        let http_client = http_client.clone();
        async move { sweep_once(resend.as_ref(), &pool, &http_client).await }
    });
}
```

```rust
pub fn spawn_unverified_email_sweep(pool: PgPool) {
    spawn_sweep("unverified_email_expiry", sweep_interval(), move || {
        let pool = pool.clone();
        async move { sweep_unverified_emails_once(&pool).await }
    });
}
```

```rust
pub fn spawn_invite_nudge_sweep(pool: PgPool, resend: Option<resend_rs::Resend>) {
    spawn_sweep("invite_nudge", sweep_interval(), move || {
        let pool = pool.clone();
        let resend = resend.clone();
        async move { sweep_invite_nudge_once(resend.as_ref(), &pool).await }
    });
}
```

```rust
pub fn spawn_invite_expiry_sweep(pool: PgPool, resend: Option<resend_rs::Resend>) {
    spawn_sweep("invite_expiry", sweep_interval(), move || {
        let pool = pool.clone();
        let resend = resend.clone();
        async move { sweep_invite_expiry_once(resend.as_ref(), &pool).await }
    });
}
```

```rust
pub fn spawn_invite_auto_decline_sweep(
    pool: PgPool,
    broadcaster: crate::websocket::GameBroadcaster,
) {
    spawn_sweep("invite_auto_decline", sweep_interval(), move || {
        let pool = pool.clone();
        let broadcaster = broadcaster.clone();
        async move { sweep_invite_auto_decline_once(&pool, &broadcaster).await }
    });
}
```

- [ ] Verify the boilerplate is gone: `rg -c 'MissedTickBehavior' rust/web/src/email/sweep.rs` — expect **1** (was 5). `rg -c 'tokio::spawn' rust/web/src/email/sweep.rs` — expect **1** (was 5).
- [ ] Verify nothing else changed shape: `rg -n 'pub fn spawn_' rust/web/src/email/sweep.rs` — expect the same six names as before (`spawn_turn_reminder_sweep`, `spawn_unverified_email_sweep`, `spawn_invite_nudge_sweep`, `spawn_invite_expiry_sweep`, `spawn_invite_auto_decline_sweep`, `spawn_periodic_sweeps`).
- [ ] Run: `cargo test -p web --features ssr email::sweep` — PASS (unchanged set).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/email/sweep.rs` ; message: `refactor(email): one spawn_sweep helper for the five periodic sweeps (wfe F38, WP-51)`

**Manual verification:** start the web binary and confirm the startup log still contains all five lines, one per sweep, with the same labels and the same interval rendering: `turn_reminder: sweep every 900s`, `unverified_email_expiry: sweep every 900s`, `invite_nudge: …`, `invite_expiry: …`, `invite_auto_decline: …`. Before this task those came from five separate `tracing::info!`s; after, from one.

---

## Task 5: gate `notify_owner_decline` like every other invite mailer (wd F32 minor)

**Problem (restated):** `notify_owner_decline` (`proposals.rs:286-328`) resolves the owner's recipient row and then sends, checking only `owner_recip.email` (`:297-299`). The other five mailer methods all apply `invite_recipient_should_send(&recip, suppressed)` after `suppress_for_web_presence` (`:193`, `:250`, `:344`, `:389`, `:434`), so an owner who turned invite emails off, or who is sitting on the proposal page (10-minute presence window, `outbound.rs:42-47`), is mailed anyway.

**Fix (re-derived):** copy `notify_owner_ready`'s block verbatim — it is the structural twin (same recipient, the owner; `:429-436`).

**Edge cases:**
- Placement matters: the gate goes **after** `let Ok(Some(owner_recip)) = …` and **before** `let Some(email) = owner_recip.email else { return };`, matching `notify_owner_ready`. The later `email` binding then becomes belt-and-braces (the gate already requires `email.is_some()`); keep it — it is what produces the `String` to send to.
- The invitee-name lookup (`:300-305`) stays where it is; it runs after the gate, so a suppressed send now avoids that query too.
- This is a deliberate behaviour change: an owner watching the page gets no decline email. That is the point, and the websocket `broadcast_proposal_update` at `:1269` already updates the page live before `notify_owner_decline` is fired at `:1274`. Do not add an exception for "important" notifications — `notify_owner_ready` has no such exception either.
- WP-46 will call `notify_owner_decline` from the auto-decline sweep (wfe F34); the gate applies there too, which is correct and consistent.

**Files:**
- Modify: `rust/web/src/proposals.rs` (`notify_owner_decline`)

**Steps:**

- [ ] Implement: in `notify_owner_decline` (locate by `fn notify_owner_decline`), insert immediately **after** the `let Ok(Some(owner_recip)) = fetch_invite_recipient(&pool, proposal.owner_user_id).await else { return; };` block and **before** `let Some(email) = owner_recip.email else {`:

```rust
            let suppressed = crate::email::outbound::suppress_for_web_presence(
                &pool,
                Some(proposal.owner_user_id),
            )
            .await;
            if !invite_recipient_should_send(&owner_recip, suppressed) {
                return;
            }
```

- [ ] Verify all six methods now gate: `rg -c 'invite_recipient_should_send' rust/web/src/proposals.rs` — expect **13** (was 12: 1 definition + 5 mailer sites + 6 test lines; now 6 mailer sites).
- [ ] Verify the placement: `rg -n -A3 'fn notify_owner_decline' rust/web/src/proposals.rs` then read the function — the order must be `find_proposal` -> `fetch_invite_recipient` -> `suppress_for_web_presence` -> `invite_recipient_should_send` -> `email` -> render -> send, i.e. the same order as `notify_owner_ready`.
- [ ] Run: `cargo test -p web --features ssr proposals::` — PASS, including `invite_notification_suppressed_by_recipient_presence` (`:2226-2251`) and `invite_recipient_should_send_truth_table` (`:2253-2273`), which document exactly this gate and are unmodified.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): apply the invite-email gates to owner decline notifications (wd F32, WP-51)`

**Manual verification (the mailer spawns a task and swallows its own outcome, so there is no automated hook):** run in dev with no `RESEND_API_KEY`. (a) Owner with `invite_emails_enabled = false`: have an invitee decline. Before this task a `==> GAME EMAIL for <owner>` block with subject `"{type} invite"` and header `"{invitee} declined your invite."` appears on stdout; after, nothing does. (b) Owner with `invite_emails_enabled = true` and `last_active_at` older than 10 minutes: the block appears both before and after.

NOTE: this task adds 8 lines inside `notify_owner_decline`, shifting everything below it. Tasks 6 and 7 locate by symbol name.

---

## Task 6: stop promising a reply that cannot work (wd F33 minor)

**Problem (restated):** four pure-notification mails set the reply address to `i-{proposal_id}@brdg.me` (`proposals.rs:324` `notify_owner_decline`, `:365` `notify_cancelled`, `:410` `notify_started`, `:460` `notify_owner_ready`) and end with `"Reply to this email to respond, or unsubscribe anytime."` (`:315`, `:356`, `:401`, `:451`). `Uuid`'s `Display` is hyphenated while proposal-player tokens are `Uuid::new_v4().simple().to_string()` (`:674`, `:1146`), so `find_proposal_player_by_email_token`'s `WHERE email_token = $1` (`:688-698`) can never match and `handle_invite_reply` logs "unknown invite token; no response" (`inbound.rs:596-599`). The user is invited to reply into a black hole.

**Fix (re-derived):** keep the `i-` route — it is what makes a stray reply inert — but make the dead address explicit and greppable, and drop the reply promise from exactly these four footers.

**Why not a bare `no-reply@brdg.me` (the finding's own suggestion):** `parse_reply_address` (`inbound.rs:37-51`) returns `None` for a local part with no `g-`/`i-`/`s-` prefix, and `resend_webhook` routes `None` to `handle_settings_reply_route` (`inbound.rs:484-486`). A reply to "no-reply" would therefore be parsed as a **settings command** authenticated only by the From header. That is strictly worse than the current dead-but-inert address, and it would collide with WP-56/D-1.

**Why not "the recipient's own token":** three of the four mails go to the owner or to already-accepted players. WP-44 Task 2 (wd F29) makes the owner unable to respond at all, and the other two mails describe a proposal that is already cancelled or started, so a *working* token would turn a silently-dropped reply into a rejection reply. No improvement, extra surface.

**Edge cases:**
- `send_invite` (`:219`, `:228`) and `notify_changed_reinvite` (`:271`, `:280`) **keep** both the reply-promise footer and `i-{token}@brdg.me`: those replies genuinely work (`token` there is the invitee's real `email_token`, passed in by the caller at `:1185`, `:1500`, `:1503`, `:1645`). **Do not touch those four lines.**
- The new footers must keep the word "unsubscribe": the `List-Unsubscribe` headers (`render.rs:235-242`) are still emitted, and WP-58/D-10 owns that surface.
- `"noreply"` can never collide with a real token: tokens are 32 hex characters (`Uuid::simple`), and the only non-generated values anywhere are test literals (`:2399` `"orig-token-d"`, `:2443` `"tok-1"`).
- If **WP-59 has already landed**, use `crate::email::notify::invite_reply_address("noreply")` instead of the literal, per WP-59 Task 8's `REPLY_DOMAIN` consolidation. If it has not, write the literal — do not create a second domain constant here.
- Line-count neutral: each edit replaces one line with one line.

**Files:**
- Modify: `rust/web/src/proposals.rs` (four footer strings, four reply addresses)

**Steps:**

- [ ] Implement: in each of `notify_owner_decline`, `notify_cancelled`, `notify_started` and `notify_owner_ready` (locate by `fn <name>`), replace that method's footer line

```rust
                footer: Some("Reply to this email to respond, or unsubscribe anytime.".into()),
```

  with

```rust
                // No reply channel: these are one-way notifications and the
                // proposal reply route needs a player email_token, which this
                // mail has none of (wd F33).
                footer: Some("Unsubscribe anytime.".into()),
```

  (`notify_cancelled` and `notify_started` build their content inside a `for` loop, so their footer lines carry four extra spaces of indentation — keep each file's existing indentation; `cargo fmt` will not fix a wrong indent inside a string-bearing struct literal for you.)
- [ ] Implement: in the same four methods, replace the reply-address argument

```rust
                &format!("i-{proposal_id}@brdg.me"),
```

  with

```rust
                "i-noreply@brdg.me",
```

  This changes the argument from `&String` to `&'static str`; `render_game_email`'s parameter is `reply_address: &str` (`render.rs:97`), so both coerce and no other change is needed.
- [ ] Verify: `rg -n 'i-\{proposal_id\}@brdg\.me' rust/web/src/proposals.rs` — expected **zero** hits (was 4, at `:324`, `:365`, `:410`, `:460`). `rg -c 'i-noreply@brdg\.me' rust/web/src/proposals.rs` — expect **4**.
- [ ] Verify the working paths were left alone: `rg -n 'i-\{token\}@brdg\.me' rust/web/src/proposals.rs` — expect **2** hits, in `send_invite` and `notify_changed_reinvite`. `rg -c 'Reply to this email to respond, or unsubscribe anytime' rust/web/src/proposals.rs` — expect **2** (was 6).
- [ ] Run: `cargo test -p web --features ssr proposals::` and `cargo test -p web --features ssr email::inbound` — PASS (no test asserts on these strings; `inbound.rs`'s `parse_reply_address("i-xyz@example.com")` test at `:1298-1303` is unaffected).
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): drop the dead reply promise from notification emails (wd F33, WP-51)`

**Manual verification:** in dev with no `RESEND_API_KEY`, trigger a decline, a cancel, a start and an all-accepted event. The `==> GAME EMAIL` printout (`outbound.rs:59-63`) shows `Reply-To:`. Before: `i-1b4e28ba-2fa1-11d2-883f-0016d3cca427@brdg.me` (a hyphenated proposal id) and a body ending "Reply to this email to respond, or unsubscribe anytime."; after: `i-noreply@brdg.me` and a body ending "Unsubscribe anytime.". Then reply to one of the *invite* mails with "accept" and confirm it still works — those were not touched.

---

## Task 7: log the mailer tasks' failures and stop blank substitutions (wd F34 minor)

**Problem (restated):** every `RealInviteMailer` method runs inside `tokio::spawn` and drops its errors on the floor:

- `let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else { return };` at `:197`, `:254`, `:290`, `:334`, `:377`, `:422`.
- `let Ok(Some(recip)) = fetch_invite_recipient(&pool, …).await else { return / continue };` at `:187`, `:244`, `:293`, `:339`, `:384`, `:425`.
- `proposal_game_type_name` (`:170-178`) returns `String::new()` when `find_game_version` fails **or** when the game-type lookup fails (`:176-177`'s `.unwrap_or(None).unwrap_or_default()`).
- Name lookups collapse to `unwrap_or_default()` at `:201-206` (owner) and `:300-305` (invitee).

So a DB blip is indistinguishable from "recipient opted out", leaves no trace, and — when it hits a name lookup — ships a subject reading `" invite from "` (`:208`) and a header reading `" invited you to play ."` (`:209-211`).

**Fix (re-derived):** two tiny logging helpers replace the twelve silent `else` arms, and the three blank fallbacks become named non-blank ones.

**Why not the finding's "skip the send rather than sending an email with blank substitutions":** `send_invite` is the *only* thing that tells an invitee they have been invited; the nudge sweep sets `nudged_at` unconditionally (wfe F33, WP-46) so there is no retry; and "Game invite from Someone" delivers the actionable link while a skipped send delivers nothing. Blank-vs-generic is a labelling problem, not a reason to drop mail. (Where the *proposal itself* cannot be loaded there is nothing to say and the method still returns early — that part of the finding stands.)

**Edge cases:**
- `proposal_game_type_name` keeps its `-> String` signature so none of its six call sites (`:200`, `:257`, `:306`, `:337`, `:380`, `:440`) changes. Its two other callers do not exist: `rg` shows `find_game_type_name` used from `stats/mod.rs:248` (a different function of the same name in `stats/queries.rs:32`), `inbound.rs:847` and `proposals.rs:1734` — none goes through `proposal_game_type_name`.
- `Ok(None)` and `Err` get **different** log levels: a missing row is a `warn` (a proposal can legitimately be deleted between the spawn and the query), a DB error is an `error`.
- The helpers take a `&str` label so the log names the mailer method; pass the method name exactly.
- `notify_cancelled` and `notify_started` iterate recipients — their recipient arms must stay `continue`, not `return`. Only the `find_proposal` arms are `return`.
- Do not add logging to `invite_recipient_should_send`'s `false` path: an opt-out is not an error, and it fires on every suppressed send.
- `UNKNOWN_GAME_TYPE_NAME` / `UNKNOWN_PLAYER_NAME` are last-resort labels for a failed lookup, not user-facing defaults for real data. Do not use them anywhere else.

**Files:**
- Modify: `rust/web/src/proposals.rs` (two helpers, two constants, `proposal_game_type_name`, twelve `else` arms, two name fallbacks, one new test)

**Steps:**

- [ ] Write the failing test. Add to the inline `mod tests` in `rust/web/src/proposals.rs` (next to `ready_to_start_requires_all_humans_accepted_and_valid_count`, which shows the `Proposal`-literal pattern):

```rust
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
```

- [ ] Run: `cargo test -p web --features ssr proposals::tests::proposal_game_type_name_falls_back` — expected FAIL to compile: `cannot find value UNKNOWN_GAME_TYPE_NAME in this scope`.
- [ ] Implement: add, immediately above `proposal_game_type_name` (locate by name):

```rust
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
```

- [ ] Implement: replace `proposal_game_type_name`'s body (keep the signature `async fn proposal_game_type_name(pool: &PgPool, proposal: &Proposal) -> String`) with:

```rust
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
```

- [ ] Run: `cargo test -p web --features ssr proposals::tests::proposal_game_type_name_falls_back` — PASSES.
- [ ] Implement: route all twelve silent arms through the helpers. In each method (locate by `fn <name>`), replace:

  - every `let Ok(Some(proposal)) = find_proposal(&pool, proposal_id).await else {\n                return;\n            };` with

```rust
            let Some(proposal) = mailer_proposal(&pool, proposal_id, "<method>").await else {
                return;
            };
```

  - `send_invite` / `notify_changed_reinvite`'s `let Ok(Some(recip)) = fetch_invite_recipient(&pool, invitee_user_id).await else { return; };` with

```rust
            let Some(recip) = mailer_recipient(&pool, invitee_user_id, "<method>").await else {
                return;
            };
```

  - `notify_owner_decline` / `notify_owner_ready`'s owner fetch with

```rust
            let Some(owner_recip) =
                mailer_recipient(&pool, proposal.owner_user_id, "<method>").await
            else {
                return;
            };
```

  - `notify_cancelled` / `notify_started`'s in-loop `let Ok(Some(recip)) = … else { continue };` with

```rust
                let Some(recip) = mailer_recipient(&pool, user_id, "<method>").await else {
                    continue;
                };
```

  substituting `<method>` with the enclosing method's name (`send_invite`, `notify_changed_reinvite`, `notify_owner_decline`, `notify_cancelled`, `notify_started`, `notify_owner_ready`).
- [ ] Implement: the two name fallbacks.
  1. In `send_invite`, replace `:201-206`'s

```rust
            let owner_name = fetch_invite_recipient(&pool, proposal.owner_user_id)
                .await
                .ok()
                .flatten()
                .map(|r| r.name)
                .unwrap_or_default();
```

  with

```rust
            let owner_name = mailer_recipient(&pool, proposal.owner_user_id, "send_invite")
                .await
                .map(|r| r.name)
                .unwrap_or_else(|| UNKNOWN_PLAYER_NAME.to_string());
```

  2. In `notify_owner_decline`, replace the equivalent `invitee_name` block (`:300-305`) with:

```rust
            let invitee_name = mailer_recipient(&pool, invitee_user_id, "notify_owner_decline")
                .await
                .map(|r| r.name)
                .unwrap_or_else(|| UNKNOWN_PLAYER_NAME.to_string());
```

- [ ] Verify no silent arm survives in the mailer block: `rg -n 'let Ok\(Some\(' rust/web/src/proposals.rs` — expected **zero** hits inside `impl InviteMailer for RealInviteMailer` (before: 12). Also `rg -n '\.ok\(\)$' rust/web/src/proposals.rs` — the two mailer hits at `:203` and `:302` are gone; the remaining one at `:793` is `cancel_proposal_for_expiry`, which is **WP-46's** (wd F39) — leave it.
- [ ] Verify no blank substitution remains: `rg -n 'unwrap_or_default\(\)' rust/web/src/proposals.rs` — no hit inside the mailer impl.
- [ ] Run: `cargo test -p web --features ssr proposals::` — all PASS, including the new test.
- [ ] `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Final package gate: run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass end to end (fmt, clippy workspace-minus-web, clippy web-ssr, `cargo sqlx prepare --check`, workspace tests, web ssr tests). The prepare check cannot be affected: no `sqlx::query!` macro was added or changed.
- [ ] Commit: `git add rust/web/src/proposals.rs` ; message: `fix(proposals): log invite-mailer lookup failures and stop blank subjects (wd F34, WP-51)`

**Manual verification:** in dev with no `RESEND_API_KEY`, point `DATABASE_URL` at a DB, create a proposal, then delete its `game_versions` row before the invite task runs (or simply invite on a proposal whose version was removed). Before: the printout's subject is `" invite from Alice"` and no log line explains it. After: the subject is `"Game invite from Alice"` and a `WARN invite mailer: game version … not found; using a generic label` line appears.

---

## Cross-package / newly discovered

Recorded, **not fixed** here. Every item was found while re-deriving this package's findings; none carries a review finding ID.

1. **An email-originated game move never notifies anybody (major, functional gap).** `dispatch_email_command`'s fall-through applies the move with `crate::game::execute_command` (`email/commands.rs:1264-1281`) and returns `CommandReply::GameMove`; `handle_game_reply` then mails **only the sender** a confirmation (`email/inbound.rs:575-587` -> `send_game_reply_response`). `rg -n 'notify_game_emails' rust/web/src/email/inbound.rs` returns **zero** hits. So when player A plays by email, player B gets no turn email — B's next prompt is the 24-hour reminder sweep, if reminders are on at all. This is a *wiring* omission, not a design choice: `notify.rs:2-3`'s own module doc says "`notify_game_emails` is wired at the same call sites as `trigger_bot_turns`", and `execute_command` calls `broadcast_and_trigger` (`game/mod.rs:169`) on every one of these moves. **After Task 1 the fix is five lines** (the pre-command snapshot is returned right there), which is exactly why this must be recorded rather than absorbed: it changes outbound email volume and needs the Lead's ruling. **Routing recommendation:** a new spec-time work package alongside WP-74/WP-75 ("notify_game_emails wiring gaps"), or an explicit Lead ruling to add it to WP-51 Task 1. It must NOT be folded into WP-59 (which owns `commands.rs` for error classification only) or WP-40 (which owns the three lifecycle verbs).
2. **No web game-start path notifies either (same class as item 1).** `broadcast_and_trigger` is called at `proposals.rs:1118` (solo-vs-bots direct creation), `proposals.rs:1360` (`start_proposal` after all humans accept) and `email/inbound.rs:791` (invite accepted by email, game starts) — and none of the three is followed by `notify_game_emails`. The accepted players do get `notify_started` ("The game has started!", `proposals.rs:373-416`), but that mail carries no board, no "You can", and no `g-` reply token, so the first player on turn is never prompted to play and cannot reply-to-play until some later event mints a token. Compare `email/commands.rs:427` and `:1120` and `server_fns.rs:1226`, which *do* notify after creating a game — the email `new`/`restart` paths and the web paths disagree. **Routing recommendation:** the same new package as item 1; the two are one decision ("which mutations notify?") and one fix shape.
3. **`is_turn_at` cannot be used as a "newly on turn" signal (note, not a defect).** `db::update_game_command_success` sets `is_turn_at = if is_turn { now } else { p_is_turn_at }` for **every** player row (`db.rs:1921`), so it is bumped for a player who was already on turn in a simultaneous-turn game. Recorded because it is the obvious-looking shortcut for anyone revisiting wfe F42 without the before-snapshot, and it does not work. (The same loop also resets `turn_reminder_sent_at = NULL` for every player, `db.rs:1934` — that is the reminder reset WP-46 reasons about.)
4. **`send_rendered_email` now has one caller left in `notify.rs`'s former shape.** After Task 2, `notify.rs` calls `try_send_rendered_email` directly and `outbound::send_rendered_email` (`outbound.rs:88-94`) survives only for `proposals.rs`' six mailer sends and `inbound.rs`' replies. Not a defect and not touched here; flagged so **WP-60** (which owns `outbound.rs`, wfe F46's metric-before-send) knows the caller set changed.
