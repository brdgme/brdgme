# Remediation progress tracker

Execution log for `98-REMEDIATION-PLAN.md`. Session started 2026-07-31.
Orchestrated per the sop/orchestrate skills: one Lead per package, serial.

(Restored 2026-07-31 after accidental deletion by the R-06 Lead subagent,
which ran `git add -A`, swept this file into its commit, then `rm -f`'d it.
Reconstructed from session history. 96-HANDOVER-PROMPT.md intentionally not
restored per owner instruction.)

## Machine constraints (owner instruction, 2026-07-31)

- No `cargo test` workspace-wide; no `scripts/rust-test.sh`.
- `cargo test -p <crate>` allowed for single crates, EXCEPT `web`.
- Check/clippy permitted per owner instruction (see session rulings below).
- Commit after each completed item (owner instruction 2026-07-31). Never push.

## Status legend

- `pending` / `blocked(<reason>)` / `in-progress` / `done(<commit>)` / `owner-gap`

## Work packages

| R | Status | Commit(s) | Notes |
|---|--------|-----------|-------|
| R-01 | done(a8029b8) | a8029b824e6e9c85d6fd28e1eb759c2d4ed92b82 | ships with R-02; compile-verified post-hoc (check/clippy/fmt all pass) |
| R-02 | done(ebb2c5b) | ebb2c5b61ff17cf90fcfbf08473683f26577c01b | rate-limit deferred to R-37 (recorded in code on handle_settings_reply) |
| R-03 | done(fb0d1d3) | fb0d1d38cd1eb86c4f8f248ba72b0f1669eae62d | AC5 partial: E2E asserts canonicalization accepted; full bot-turn assertion defers to CI |
| R-04 | done(c338d13) | c338d133027b57d5a7638a652bac6d3c0cbc811f | 5/5 game-creation entry points call validate_bot_slots |
| R-05 | done(f3a87b7) | f3a87b7d282f432867ab9db2e366cbd7363a459b | AC5: sweep GATES on is_turn, so R-05 is the only F-119 control |
| R-06 | done(3cd727e) | 3cd727eba4173f44276c3fae07c400e463c57ad3 | 4 lifecycle writers enumerated; CODING.md rule rewritten as property |
| R-07 | blocked(prod Kubernetes API unreachable) | 1e19d05f0506aa6e92cc16764d4f8c2f148eb022 (impl HEAD pre-tracker) | production Kubernetes API connectivity failure (TLS handshake EOF) before backup and mutation; Backup postgres-pre-repair-r07-20260801-01 not applied; no database action |
| R-08 | done(899814f) | 899814f7528d719b2b46131e74129520b52f30ed | AC1 explicit exhaustive named matches (no wildcard); AC2 and AC3 persistence-mark tests; gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned); comprehensive review APPROVE with only two non-blocking Minor notes |
| R-09 | done(61f9f4e) | 61f9f4eee5af657b108a11e5722155f82d4260c8 | AC1 single named contract `transient_failure` called by both routes; literal-Done grep 26 constructions commented (two non-constructions: match arm, doc prose); AC2 invite lock-timeout DB-error test asserts Retry; AC3 settings closed-pool DB-error test asserts Retry; gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned); comprehensive review APPROVE with two non-blocking Minor notes |
| R-10 | done(a9ea19d) | a9ea19d5e9f4640b8d6cafe64068fbcbbbe6cf3c | AC1 30s periodic session re-validation arm + revocation test; AC2 per-connection CancellationToken on SseStream::Drop + idle gauge-drop test; AC3 public handler per-id subscribe (no game.>) + VisibilityCache + subsz test; AC4 F-163 #[ignore] removed, #[serial] added; gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned; needs Postgres/NATS); comprehensive review ACCEPT, no Critical/Important findings |
| R-11 | pending | | implement ws F55 second half (owner ruling) |
| R-12 | pending | | |
| R-13 | pending | | |
| R-14 | pending | | blocks R-15 |
| R-15 | pending | | after R-14 |
| R-16 | pending | | |
| R-17 | pending | | needs 5.4 for server-fn tests |
| R-18 | pending | | |
| R-19 | pending | | |
| R-20 | pending | | |
| R-21 | blocked(R-22..R-26, R-48) | | closing commit of game family |
| R-22 | pending | | |
| R-23 | pending | | |
| R-24 | pending | | |
| R-25 | pending | | L; re-size per crate |
| R-26 | pending | | |
| R-27 | pending | | |
| R-28 | pending | | |
| R-29 | pending | | |
| R-30 | pending | | |
| R-31 | pending | | |
| R-32 | pending | | |
| R-33 | pending | | |
| R-34 | pending | | after R-30 (same file) |
| R-35 | pending | | sequence with 5.8 |
| R-36 | pending | | |
| R-37 | blocked(5.4) | | |
| R-38 | blocked(5.4, 5.3) | | |
| R-39 | pending | | |
| R-40 | pending | | |
| R-41 | pending | | |
| R-42 | pending | | |
| R-43 | pending | | |
| R-44 | blocked(5.2) | | |
| R-45 | pending | | start early; feeds owner 6.1 |
| R-46 | pending | | EXPANDED scope: eliminate all 22 lint overrides (owner ruling) |
| R-47 | pending | | |
| R-48 | pending | | |
| R-49 | pending | | |
| R-50 | pending | | F-83 follows 5.1 |
| R-51 | pending | | |
| R-52 | pending | | owner approved |
| R-53 | pending | | |
| R-54 | pending | | U8 sequenced last within package |
| R-55 | pending | | review package; before R-05/R-06 if capacity |

## Coverage items

| Item | Status | Commit(s) | Notes |
|------|--------|-----------|-------|
| 5.1 roll-through-the-ages-2 crate pass | pending | | |
| 5.2 session_store test module | pending | | blocks R-44 |
| 5.3 require_admin true-path tests | pending | | after 5.4 |
| 5.4 request-parts test harness | pending | | blocks 5.3, R-37, R-38 |
| 5.5 13 crates validate+redaction tests | pending | | L; per-crate checklist |
| 5.8 bot test coverage (U11, U12) | pending | | sequence with R-35 |

## Deployment items (section 3, brdgme-config)

| Item | Status | Notes |
|------|--------|-------|
| F-96 Turnstile secret key (prod) | pending | GitOps repo |
| TURNSTILE_SITE_KEY startup check | pending | lands with F-96 |
| config::public_base_url() prod HTTPS | pending | |
| F-207 sqlx migrator reconcile | pending | |
| F-211 hanamikoji-1 delivery gap | pending | code half in R-16 |
| Pre-rollout checklist file in brdgme-config | pending | |

## Process fixes (section 4)

| Item | Status | Notes |
|------|--------|-------|
| 4.1 four-tooth sign-off script | pending | |
| 4.2 "Test? y with no test" sweep | pending | |
| 4.3 WP spec/checklist sign-off gate | pending | |
| 4.4 deferral-state mechanism | pending | |
| 4.5 4b second-reviewer rule | pending | docs/CODING.md |
| 4.6 STOP-AND-REPORT escalation rule | pending | docs/CODING.md |
| 4.7 delivery-list CI guard | pending | same as R-16 |
| 4.8 vendoring "known defects" spec section | pending | needs owner 6.1 |
| 4.9 named-pattern sign-off sweeps | pending | |

## Owner decisions

| Decision | Ruling | Date |
|----------|--------|------|
| 6.1 vendoring policy | Forbidden except when no alternative and completely blocked; becomes docs/CODING.md rule + mandatory 4.8 spec section | 2026-07-31 |
| 6.2 R-52 worth doing? | Yes, run R-52 | 2026-07-31 |
| 6.3a F-203/F-204: rider 1 vs 3b | 3b stands (bare-major stays; rider 1 struck). F-203: add [workspace.lints.rust]. R-46 EXPANDED: eliminate all 22 lint overrides via workspace clippy.toml threshold + case fixes + no-new-allows CODING.md rule | 2026-07-31 |
| 6.3b ws F55 second half (R-11) | Implement it (shutdown signal for bot consumer + email sweep, with tests) | 2026-07-31 |
| 6.3c ALLOW_INSECURE_DEFAULT_KEY split | Leave as-is; note in pre-rollout checklist only | 2026-07-31 |
| F-59 status | excluded (default) | |
| Web crate commands | cargo check/clippy -p web ALLOWED; build/test/run against web banned | 2026-07-31 |
| Commit policy | commit after each item; never push | 2026-07-31 |
| Review-dir edits | Allowed (restriction lifted 2026-07-31); agents must never delete/move files or changes outside their own work scope - leave unrelated working-tree changes alone | 2026-07-31 |

## Incident log

- 2026-07-31: R-06 Lead subagent deleted 96-HANDOVER-PROMPT.md and this tracker
  via `git add -A` + `rm -f` (misread "never modify review-dir files" as a
  deletion mandate). Tracker restored from session history; 96 not restored per
  owner. Lead briefs hardened: no `git add -A`/`.`, stage named files only,
  never remove or revert changes outside the agent's own work.
- 2026-07-31: R-07 production-query Worker performed one unapproved
  schema-metadata check; exposed no sensitive output and made no modifications.

## R-09 evidence

Code commit `61f9f4eee5af657b108a11e5722155f82d4260c8` (only tracked change:
`rust/web/src/email/inbound.rs`, 277+/58-).

- **Root cause:** F-162 (invite) and F-169 (settings) were the same defect
  class - a transient (retryable) DB failure collapsed into `RouteOutcome::Done`,
  so svix saw 200 and never redelivered, violating D-2 at-least-once. Invite's
  six pre-`tx.commit()` errors and settings' two lookup errors now map to
  `Retry`; an uncommitted transaction rolls back on drop, so retry is safe.
- **AC1:** single named contract `transient_failure` (logs error, returns
  `Retry`) is called by both routes - invite at six sites, settings at two.
  Literal-`Done` grep: 26 constructions, each with an immediately preceding
  non-transient justification comment; the only two non-constructions are the
  `:682` dispatch match arm and the `:772` doc-comment prose.
- **AC2:** `invite_route_transient_db_error_is_retry` calls the direct invite
  route; a `lock_timeout='100ms'` pool plus a blocker holding `FOR UPDATE` on
  the player row makes `update_proposal_player_response` time out (transient),
  asserting `RouteOutcome::Retry`.
- **AC3:** `settings_route_transient_db_error_is_retry` calls the direct
  settings route; `state.pool.close()` makes `find_user_by_settings_token`
  error (closed pool), and the route now propagates the inner outcome,
  asserting `RouteOutcome::Retry`.
- **Runtime:** the two new tests are compile-verified only; runtime web tests
  deferred to CI by explicit ban (web build/test/run forbidden; DB tests need
  Postgres/NATS).
- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0 (one pre-existing `proc-macro-error2`
  future-incompat warning, unrelated).
- **Review:** comprehensive independent review verdict APPROVE, with two Minor
  non-blocking notes (settings command-dispatch `Internal` error stays `Done`,
  pre-existing/out-of-scope; runtime behavior of the two tests unverified, a
  disclosed limitation for CI to confirm).

## R-10 evidence

Code commit `a9ea19d5e9f4640b8d6cafe64068fbcbbbe6cf3c` (verified via
`git rev-parse`; tracked changes only: `rust/web/src/events.rs` production,
`rust/web/tests/sse_events.rs` tests).

- **Closes:** F-158 (session re-validation), F-159 (task/subscription leak),
  F-160 (public firehose + uncached query, in-scope halves), F-131 (concretised
  as F-158), F-163 (`#[ignore]`d regression test).
- **AC1 (F-158):** production adds a guarded 30s `interval` re-validation arm
  (`events.rs:102-103,145-152`) that re-runs `validate_session_token` and breaks
  the loop unless `Ok(true)`; period matches the `VisibilityCache` TTL and is
  under the 45s AC bound; anonymous connections skip it (guard
  `auth_token_id.is_some()`). Test `auth_stream_terminates_after_token_revocation`
  (`sse_events.rs:668-739`) drives the real handler, asserts a frame arrives,
  revokes via `invalidate_auth_token`, asserts termination within 45s.
- **AC2 (F-159):** production wraps the response in `SseStream` whose `Drop`
  fires a per-connection `CancellationToken` (`events.rs:42-59`); both loops add
  a `task_disconnected.cancelled()` select arm (`events.rs:153-155,240-242`), so
  an idle/no-event stream wakes and breaks on disconnect, dropping the task, the
  NATS subscription(s), and decrementing the gauge. Test
  `idle_anonymous_connection_releases_task_on_disconnect`
  (`sse_events.rs:750-792`) drops the client and asserts the `sse_connections`
  gauge falls within 10s.
- **AC3 (F-160):** public handler now subscribes per-id `game.{id}` via
  `select_all` and never `game.>` (`events.rs:204-214`), and routes the
  visibility check through a per-task `VisibilityCache` (`events.rs:216,234`),
  mirroring the auth handler. Test
  `public_handler_subscribes_per_game_not_firehose` (`sse_events.rs:803-834`)
  reads NATS `subsz?subs=1` and asserts `game.{A}` present, `game.>` absent.
- **AC4 (F-163):** `#[ignore]` removed (grep for `ignore` in `sse_events.rs`
  returns nothing); `sse_stream_survives_past_request_timeout_with_keepalive`
  now `#[sqlx::test] #[serial]` with unchanged 32s body
  (`sse_events.rs:551-595`), reachable in CI (tooth 2 restored).
- **Auth-vs-public handler difference justification (AC3):** the full exhaustive
  per-row difference table with justifications is in
  `docs/reviews/r-10-comprehensive-review.md` §4. Categories covered: session
  extraction, viewer/token resolution, game subscription, proposal subscription,
  visibility cache, game visibility predicate, topic filtering, topic cap,
  session re-validation, disconnect detection, shutdown observation, gauge guard,
  return type, KeepAlive, and rate limiting. The two previously unjustified rows
  (public `game.>` firehose, public missing cache) are now resolved; all
  surviving differences are by-design auth-vs-public semantics or correct
  predicate/return-type distinctions.
- **Runtime:** the new tests are compile-verified only; runtime web tests
  deferred to CI by explicit ban (web build/test/run forbidden) and because they
  need Postgres and NATS (with `-m 8222` monitoring), unavailable in a plain
  local run (AGENTS.md; BACKLOG #40).
- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0 (one pre-existing `proc-macro-error2`
  future-incompat warning, unrelated; nothing from `events.rs`/`sse_events.rs`).
- **Review:** comprehensive independent review
  (`docs/reviews/r-10-comprehensive-review.md`) verdict ACCEPT; no Critical or
  Important findings; four Minor observations and two test-reliability residual
  risks, all non-blocking (intended behaviour or disclosed/deferred to CI).
- **Out of scope / unchanged:** F-94 rate limiting deferred (R-37/edge), not
  added; F-123 (`VisibilityCache` cross-user leak) remains refuted (each cache is
  a per-task local: `events.rs:101` auth, `events.rs:216` public); no
  `TaskTracker` added - the spawns are unchanged in shape so R-11 can still
  register them (R-11 intentionally untouched).
