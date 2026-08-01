# R-08 Review - Transient errors must not be classified permanent

**Verdict: APPROVE** (two Minor notes, no Critical / no Important findings)

| Item | Value |
|------|-------|
| Reviewer | R-08 end-of-package review Worker (sole execution) |
| Date | 2026-08-01 |
| HEAD | `1159ebe19b5a0b1c097084de1239bacaf44c8a5e` |
| Change under review | unstaged working-tree diff over HEAD |
| Files | `rust/web/src/email/sweep.rs` (+181/-1 net), `rust/web/src/proposals.rs` (+39/-6 net) |
| Closes | F-136 (High), F-145 (Medium) |
| Spec | `98-REMEDIATION-PLAN.md:318-344` |
| Handover | `R-08-CONTEXT-HANDOVER.md` |
| Gate | `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` - exit **0** |

---

## 1. Gate (run exactly once)

Command (run from `rust/`):

```
SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr
```

Full output:

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s
warning: the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
GATE_EXIT=0
```

- Exit code: **0**.
- The `proc-macro-error2` future-incompat warning is a transitive-dependency
  notice, pre-existing and unrelated to this change; it is a warning, not an
  error, and does not affect the exit code.
- The run completed as an incremental no-op ("Finished ... in 0.36s", no
  `Compiling` lines). This is a valid confirmation of the current working tree:
  cargo fingerprints each unit by source mtime/size, and a skip means the exact
  current source (including both new `#[cfg(all(test, feature = "ssr"))]` tests,
  which are in scope under `--all-targets`) was already checked clean. Had the
  modified `sweep.rs` / `proposals.rs` differed from the cached fingerprint,
  cargo would have recompiled. Exit 0 is therefore authoritative for the diff
  under review.
- No other cargo/rustc command and no web test run was executed, per the brief.

---

## 2. Acceptance / spec compliance

### AC1 - neither site has a `_ =>` arm; exhaustive over named variants - PASS

All four classifier sites are now three-arm `match`es over
`Result<Option<T>, E>` with `Ok(Some(_))`, `Ok(None)`, `Err(_)` and **no** `_`
arm. That triple is the complete reachable shape of `Result<Option<T>, E>`, so
each match is exhaustive by construction.

| Site | Location | Arms | `_ =>`? |
|------|----------|------|---------|
| F-136 reminder recipient classifier | `rust/web/src/email/sweep.rs:134-146` | `Ok(Some(r))` / `Ok(None)` / `Err(e)` | no |
| F-145 invitee recipient classifier | `rust/web/src/proposals.rs:268-279` | `Ok(Some(r))` / `Ok(None)` / `Err(e)` | no |
| F-145 proposal classifier | `rust/web/src/proposals.rs:291-302` | `Ok(Some(p))` / `Ok(None)` / `Err(e)` | no |
| F-145 email-token classifier | `rust/web/src/proposals.rs:306-316` | `Ok(Some(pp))` / `Ok(None)` / `Err(e)` | no |

Verified by grep: the only `_ =>` remaining in `sweep.rs` is at line 48 inside
`parse_duration` (a duration-unit string match, unrelated to error
classification). There is no `_ =>` anywhere in `proposals.rs:257-372`
(`send_invite_core`).

### AC2 - test calls the sweep's classifier with a transient DB error and asserts NOT marked sent - PASS

`turn_reminder_transient_recipient_lookup_error_leaves_row_unmarked`
(`rust/web/src/email/sweep.rs:1536-1568`):

- Seeds a game via the existing `seed_reminder_game` helper.
- Pre-fault guard: asserts the seeded player IS a `fetch_candidates` result
  before injecting the fault (so a vacuous pass - no candidate - is impossible;
  the test would panic here first).
- Injects the fault by `ALTER TABLE user_emails RENAME TO user_emails_r08_hidden`.
- Drives the full `sweep_once(None, &pool, &http)` path.
- Restores the table, then asserts `turn_reminder_sent_at IS NULL`.

I traced the SQL of every function on the path to confirm the fault lands
**precisely** on the F-136 classifier, not earlier:

- `fetch_candidates` (`sweep.rs:59-84`) joins `game_players` + `users` only -
  no `user_emails`. Still returns the candidate after the rename.
- `sweep_once` claim query (`sweep.rs:265-269`) is `SELECT id FROM game_players
  ... FOR UPDATE SKIP LOCKED` - no `user_emails`. Claim succeeds.
- `send_reminder` -> `find_game_extended` (`db/games.rs:88`, player query
  `:126-129`) joins `users` / `game_type_users` / `game_bots` - no
  `user_emails`. Succeeds.
- `send_reminder` -> `fetch_email_recipient` (`email/outbound.rs:188-209`) does
  `LEFT JOIN user_emails ue ...` - this is the first query to touch
  `user_emails`. After the rename it raises "relation does not exist" ->
  `anyhow::Error` wrapping `sqlx::Error` -> the `Err(e)` arm at `sweep.rs:138`
  -> `ReminderOutcome::Retry`.
- `sweep_once` (`sweep.rs:296-315`): `Retry` => no `mark_reminder_sent_tx`, tx
  dropped (rollback). Row stays unmarked.

So the test genuinely reaches the current transient path at `sweep.rs:134-146`
and verifies the persistence mark (`turn_reminder_sent_at`) is NOT set.

### AC3 - test calls the proposals nudge path with a transient send error and asserts NOT marked delivered - PASS

`invite_nudge_transient_lookup_error_leaves_proposal_unmarked`
(`rust/web/src/email/sweep.rs:1577-1700`):

- Seeds a game type / version, an owner and an invitee (each with a verified
  primary `user_emails` row), an `open` proposal created 48h ago, and two
  proposal players (owner `accepted`, invitee `pending` with an email token).
- Pre-fault guard: asserts the proposal IS a `fetch_nudge_candidates(&pool,
  86400)` result before the fault (vacuous-pass guard).
- Injects the same `user_emails` rename fault.
- Drives `sweep_invite_nudge_once(None, &pool)`.
- Restores the table, asserts `game_proposals.nudged_at IS NULL`.

Path trace confirming the fault lands on the F-145 classifier:

- `fetch_nudge_candidates` (`proposals.rs:999-1018`) touches only
  `game_proposals` + `game_proposal_players` - no `user_emails`. Still returns
  the candidate (invitee, `pending`, token set) after the rename.
- `sweep_invite_nudge_once` (`sweep.rs:506-528`) calls
  `send_invite_now(proposal_id, invitee, Some(token))` ->
  `send_invite_core`.
- `send_invite_core`: `email_token` is `Some`, so it proceeds; the first
  `user_emails`-touching query is `fetch_invite_recipient`
  (`proposals.rs:145-158`, `LEFT JOIN user_emails ue ...`) at site 1
  (`proposals.rs:268`). After the rename -> `Err(e)` arm -> `return false`.
- `sweep_invite_nudge_once`: `all_sent[pid] = true &= false = false`, so
  `mark_proposal_nudged` (`proposals.rs:1021-1030`, the `nudged_at = NOW()`
  UPDATE) is NOT called. `nudged_at` stays NULL.

So the test genuinely reaches the current transient path and verifies the
persistence mark (`nudged_at`) is NOT set.

**Note on test execution:** the brief restricts the gate to `cargo check` and
prohibits running web tests, so runtime pass/fail of the two tests was not
executed here. Compliance above is established by static path analysis plus the
fact that both tests compile under `--all-targets --features ssr` (gate exit 0)
and each carries a pre-fault assertion that fails loudly rather than passing
vacuously. (Per `AGENTS.md`, DB-backed tests require the dedicated harness and
fail in a plain local run regardless.)

---

## 3. Correctness

- The `Err(_) => Retry` / `return false` semantics restore at-least-once
  delivery at both sites: a transient DB failure no longer collapses into the
  same outcome as a legitimate permanent condition, so neither
  `turn_reminder_sent_at` nor `nudged_at` is set when nothing was sent. This is
  exactly the F-136 / F-145 defect class (pattern 5, `_ => <default>`).
- `Ok(None)` correctly remains a permanent skip (`true` / `PermanentSkip`):
  slot/user/proposal/token genuinely absent => do not retry. Behaviour on the
  `Ok` paths is unchanged from before, so there is no regression to legitimate
  sends or legitimate skips.
- The `send_invite_core` contract (doc comment `proposals.rs:250-256`) is
  honoured: `true` = sent or permanently unsendable, `false` = transient only.
  The three new `Err => return false` arms are the only new `false` returns and
  each corresponds to a genuine transient (a failed lookup).
- `sweep_once` mark+commit (`sweep.rs:296-315`) and the nudge mark
  (`sweep.rs:523-527`) are unchanged; the fix is entirely in the classifier
  outcomes feeding them.

## 4. Quality / simplicity / maintainability / readability

- The fix is minimal and local: four `match` blocks, each logging the error at
  `error` level with the relevant id before returning the retry outcome. No new
  types, no signature churn, no new error enum (consistent with handover D-2).
- The implementation diverges from the handover's "change
  `mailer_recipient`/`mailer_proposal` to return `Result<Option<T>, ()>`"
  suggestion (Approach A) and instead inlines the underlying
  `fetch_invite_recipient` / `find_proposal` calls directly at the three
  `send_invite_core` sites. This is an equally valid, arguably simpler
  realisation of the same contract and keeps the shared helpers intact for the
  fire-and-forget mailers.
- `mailer_recipient` / `mailer_proposal` (`proposals.rs:180-208`) are still
  referenced by 12 call sites (the spawned `notify_*` mailers and the
  owner-name lookup at `proposals.rs:324`), so there is no new dead code. Their
  retained `Err(_) => None` collapse is correct for those fire-and-forget paths
  (no mark is fed by them; handover D-6) and is out of R-08 scope.
- Both tests reuse existing helpers (`seed_reminder_game`,
  `insert_proposal_player`, `fetch_candidates`, `fetch_nudge_candidates`) and
  follow the module's house style for `#[sqlx::test]` cases. Fault injection via
  table rename is deterministic and needs no new infrastructure.

## 5. Reliability

- The two new tests are robust against the env-var races present in this test
  binary: `sweep_once` / `sweep_invite_nudge_once` read `TURN_REMINDER_AFTER` /
  `INVITE_REMINDER_AFTER`, and the parser unit tests transiently set those to
  `2h` / `1d`. Both values are still well under the 48h age of the seeded
  fixtures, so a race cannot make the candidate disappear and force a vacuous
  pass (and the pre-fault assertions guard that anyway).
- A renamed-table error is technically a "relation not found" `DatabaseError`,
  not a pool-timeout transient - but the classifiers (correctly) do not
  discriminate error kinds: every `Err(_)` is a retry. The test therefore
  exercises exactly the guarantee the fix provides (any lookup error => retry,
  never mark), which is the acceptance intent.

## 6. Security

- No new secret material, no new user-controlled input paths, no authz change.
  The added log lines emit only internal UUIDs and the error `Display`, which is
  already the established pattern throughout this module. No concern.

## 7. Regressions

- `Ok` paths unchanged => no change to legitimate sends/skips.
- No public signature changed; `send_invite_now`, `send_invite_core`,
  `sweep_once`, `sweep_invite_nudge_once`, `fetch_*`, `mark_*` keep their
  shapes.
- No deletion of guards or tests; the diff is purely additive plus the four
  arm-rewrites. Gate exit 0 across `--all-targets`.

---

## 8. Findings

No Critical findings. No Important findings.

### Minor

- **M-1 - permanent-skip (`Ok(None)`) no longer logs in `send_invite_core`.**
  `rust/web/src/proposals.rs:270`, `:293`, `:308`. The previous
  `mailer_recipient` / `mailer_proposal` helpers logged a `warn` on `Ok(None)`
  ("... not found; no email"). The inlined matches return `true` silently on
  `Ok(None)` (only the `Err` arm logs). This drops a low-value observability
  line for the "row genuinely absent" case. It does not affect the mark/send
  contract and matches the silent `Ok(None) => PermanentSkip` already used in
  the F-136 fix (`sweep.rs:137`), so it is consistent rather than wrong.
  Optional: add a `tracing::warn!` on the `Ok(None)` arms if on-call wants the
  signal back.

- **M-2 - stray trailing blank line in the proposals test module.**
  `rust/web/src/proposals.rs:4124` (the diff adds an empty line before the
  closing `}` of `mod tests`). Purely cosmetic; `cargo fmt` would normalise it.
  No functional impact.

Neither Minor finding blocks approval.

---

## 9. Verdict

**APPROVE.** AC1/AC2/AC3 are all satisfied: every in-scope classifier is an
exhaustive named-variant match with no `_ =>` arm; both tests genuinely reach
the current transient paths (fault lands exactly on `fetch_email_recipient` /
`fetch_invite_recipient`, with the preceding candidate/claim queries verified
to not touch `user_emails`) and assert the persistence marks
(`turn_reminder_sent_at`, `nudged_at`) are left unset. The gate
`SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exits 0.
Two Minor notes (M-1 lost `Ok(None)` warn log, M-2 stray blank line) are
non-blocking.
