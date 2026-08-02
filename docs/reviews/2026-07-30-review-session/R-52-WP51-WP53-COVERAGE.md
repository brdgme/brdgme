# R-52 - Unit 07b coverage of WP-51 (`dcd8844c`) and WP-53 (`3610b957`)

R-VER verification record, 2026-08-03. Re-walk of the two immutable commits to
settle unified-report 8.12's open question (did Unit 07b's re-dispatch cover the
whole WP-51/WP-53 surface?). Immutable snapshot evidence only - every range
below was read from the named commit's tree via `git show`. No HEAD line numbers
used; no source, plan, or progress changed.

**Result: coverage confirmed.** All three named WP-51 surfaces and the WP-53
residuals check out covered-clean against the immutable snapshots. **No new
technical finding.** F-144..F-149 stand exactly as recorded in
`07b-wp51-wp53-tail.md` and are not restated here.

| Commit snapshot | Surface / range | Checked result | Unit 07b evidence pointer | Outcome |
|---|---|---|---|---|
| `dcd8844c` | Six `RealInviteMailer` workflows: `send_invite_core` :257-347 (shared by the `send_invite`/:351 and `send_invite_now`/:362 wrappers), `notify_changed_reinvite` :372-445, `notify_owner_decline` :448-525, `notify_cancelled` :527-592, `notify_started` :594-659, `notify_owner_ready` :662-735 | All six gate on `invite_recipient_should_send`; `mailer_proposal`/`mailer_recipient` (:180-209) log `Ok(None)` vs `Err` at distinct levels (wd F34). wd F33: the four one-way mails carry `"Unsubscribe anytime."` (:488, :557, :625, :700) and `invite_reply_address("noreply")` (:520, :585, :653, :732); the two live-reply mails keep `i-{token}` addresses (:341, :441) and the reply footer (:314, :413) | 07b "Verified good" (wd F33, wd F34 arms); findings F-145/F-146 already stand on this surface | covered-clean; no new finding |
| `dcd8844c` | `spawn_sweep` collapse + all six production call sites: helper `sweep.rs:290-303`; call sites :311 (turn_reminder), :398 (bot_turn_sweep), :435 (unverified_email_expiry), :499 (invite_nudge), :526 (invite_expiry), :565 (invite_auto_decline); aggregator :573-584 | Single `tokio::spawn` (:296) and single `MissedTickBehavior::Skip` (:298); log shape `"{name}: sweep every {interval:?}"` preserved; parent `dcd8844c^` had six duplicated loops (:287, :382, :424, :492, :524, :564) - all six now route through the helper; `spawn_periodic_sweeps` calls all six unchanged | 07b "Verified good" (spawn_sweep entry) | covered-clean; no new finding |
| `dcd8844c` | `notify_owner_decline` gating: `proposals.rs:448-525`, gate :461-468 | `suppress_for_web_presence` (:461) + `invite_recipient_should_send` (:466) sit after recipient resolve and before the `email` binding - the same order as structural twin `notify_owner_ready` (:675-680), per spec task 5. Owner/invitee lookups route through `mailer_recipient` with `UNKNOWN_PLAYER_NAME` fallback (:475) | 07b "Verified good" (wd F32 entry) | covered-clean; no new finding |
| `3610b957` | Residual cosmetic (a): `encode_path_segment` doc comment deleted | `players.rs:34-43` in snapshot: the old `/// Percent-encodes...` doc (parent :34-36) was replaced by a mid-file `use percent_encoding::{...}` (:34) + `PATH_SEGMENT_ENCODE_SET` (:36-40); body delegated to `utf8_percent_encode` (:43). Escape set unchanged (removes `-._~` from `NON_ALPHANUMERIC` = old `A-Za-z0-9-._~` loop) | 07b "Coverage gaps" (D4) | covered-clean; cosmetic below finding threshold |
| `3610b957` | Residual cosmetic (b): mid-file `percent_encoding` import | Present at `players.rs:34`; `percent-encoding` is an explicit `web/Cargo.toml` entry per wd F65's required condition | 07b "Coverage gaps" (D4) | covered-clean; cosmetic below finding threshold |
| `3610b957` | Residual cosmetic (c): two-word `settings.rs` module-doc swap | `settings.rs:1-2`: "email placeholder" -> "email management". True of `EmailSection`, but does not enumerate add/confirm/make-active/remove as the wd F77 row (`T3-B5:111`) asked | 07b "Coverage gaps" (D5) | covered-clean; cosmetic below finding threshold |
| plan 98:1904-1907 | `restart_core` pool-read-under-`FOR UPDATE` | **Recorded as supplied, not re-derived:** NOT a deadlock (different table, no write in-transaction); off-convention residual vs neighbours at :1136/:1145 has no owner; close as a convention note | 07b "Verified good" (restart_core entry, matching disposition) | disposition accepted as supplied |

Note: 07b's own line references (e.g. `sweep.rs:314-328`, `proposals.rs:461-468`)
were to its walk's live numbering; the snapshot ranges above are the
reproducible `git show` evidence and are authoritative for this record.
