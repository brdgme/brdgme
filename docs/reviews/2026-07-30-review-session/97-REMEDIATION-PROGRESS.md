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
| R-11 | done(13ab0ffd) | 13ab0ffd3896f3b0804997a36b2b24a02c2c8147 | implement ws F55 second half (owner ruling 6.3b). AC1 tooth-4 historical amendment: WP-36's ws F55 fix and its regression test `rust/web/tests/websocket_hygiene.rs` were deleted by `efad81f92b0a1f585410e6f30fdd8de8a3dac518`; the WP-84 §3g successor proof test is `rust/web/tests/sse_events.rs:601-657` (`graceful_shutdown_ends_sse_stream_and_server_completes`) - I1 citation corrected from the keepalive test `:551-595` (`sse_stream_survives_past_request_timeout_with_keepalive`). AC2 shutdown signal threaded into bot consumer (`game/mod.rs:263,311`), advisory listener (`nats.rs:214`), supervisor (`nats.rs:280`), and all six sweeps (`sweep.rs:324,635`); shutdown-path tests call the real production paths: `bot_command_consume_loop_exits_on_shutdown` (`game/mod.rs:1284`), `sweep_stops_on_shutdown` (`sweep.rs:1736`), `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` (`nats.rs:467`), `supervisor_backoff_sleep_is_interrupted_by_shutdown` (`nats.rs:517`). AC3 detached SSE spawns bounded for the normal case by R-10's per-connection token + axum graceful shutdown (no `TaskTracker` reintroduced), proven by `graceful_shutdown_ends_sse_stream_and_server_completes` (`sse_events.rs:601-657`); documented residual: a task blocked in `client.subscribe()` under broken NATS is not bounded in-code (I2, owner confirmation recommended). Gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned). Comprehensive review ACCEPT, no Critical findings; targeted doc-only re-review (`review/R-11-TARGETED-REREVIEW.md`) PASS, resolving I1 (AC1 successor citation corrected from `:551-595` to `:601-657`); I2 residual owner confirmation recommended. |
| R-12 | done(c4c408c) | c4c408c9f9190e8140ba7ed07491f35b10a28a6f | AC1 real `logout_everywhere` + fail-first injected SessionStore error asserts Err and 2 tokens remain; AC2 healthy MemoryStore real call asserts Ok(true) and zero rows; gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned); F-86/R-37 same root cause but read-path behavior remains deferred - R-12 only adds a nonblocking/soft sequencing dependency and R-37 was blocked by 5.4 (now done, 973ea62); comprehensive review APPROVE with no Blocking/Important and three Minor notes |
| R-13 | done(afe85b2) | afe85b24290aca483a777088d57eb53291bdf87a | AC1 new workspace crate `brdgme_crypto` (`rust/lib/crypto`) holds the single hardened `encrypt`/`decrypt`/`load_key`; both consumers are thin re-export facades (`bot/src/crypto.rs:1` and `web/src/crypto.rs:1` = `pub use brdgme_crypto::*;`), so divergence cannot recur. AC2 bot-level panic-without-opt-in test `load_key_missing_env_panics_without_opt_in` (`bot/src/crypto.rs:11-22`, removes both envs, `catch_unwind` on `load_key().expect(..)` asserts `is_err()`, testing the real `main.rs:809` `.expect()`) plus opt-in test `load_key_missing_env_with_opt_in_loads_default` (`bot/src/crypto.rs:24-33`, sets `ALLOW_INSECURE_DEFAULT_KEY=true`, asserts key == `default_key()`); shared crate adds the unit-level MissingKey result test `load_key_missing_env_returns_missing_key` (`lib/crypto/src/lib.rs:130-135`, asserts `Err(CryptoError::MissingKey)`). AC3 old bot assertions/dispositions per review section 1: old missing-env `matches!(.., LoadedKey::Default(k) if k == default_key())` INVERTED (deleted from bot; replaced by bot panic test + crate `Err(MissingKey)` test; `LoadedKey` type eliminated); old valid-hex `matches!(.., LoadedKey::FromEnv(k) if k == [0xAB;32])` MOVED+inverted to crate `:121-127` `assert!(*load_key().unwrap() == [0xAB;32])` (`LoadedKey` ref removed); roundtrip MOVED to crate `:92-97` `encrypt_decrypt_roundtrip`; tamper MOVED to crate `:100-109` `decrypt_rejects_tampered_ciphertext`; invalid-hex MOVED to crate `:149-154`; wrong-length MOVED to crate `:157-162`; `wrong_key_fails` (decrypt with `[0xCD;32]`) DROPPED - review finding F-1 LOW, non-blocking (AEAD auth path already exercised by tamper test). AC4 F-187 four axes all resolved: (1) missing-key behaviour `Err(MissingKey)` unless `ALLOW_INSECURE_DEFAULT_KEY=true` (`lib.rs:57-65`), bot panics at boot via `.expect()`; (2) key material in memory `Zeroizing<[u8;32]>` + hex scratch buffer `bytes.zeroize()` after copy (`lib.rs:46-50,67-75`); (3) nonce source `getrandom::fill` with `?` propagation (`lib.rs:78-82`); (4) length check explicit `bytes.len() != 32` then `bytes.zeroize()` before `Err(InvalidKeyLength)` (`lib.rs:68-70`). Gates: shared `cargo test -p brdgme_crypto` 8/8 pass; bot R-13 facade tests both pass while full `cargo test -p bot` is 34 pass / 4 known DB `PoolTimedOut` failures (sqlx-core testing/mod.rs:227, pre-existing env condition, none touch crypto); `cargo clippy -p brdgme_crypto`/`-p bot --all-targets -- -D warnings` and per-package `cargo fmt -p brdgme_crypto`/`-p bot -- --check` all exit 0; allowed `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (only pre-existing `proc-macro-error2` future-incompat warning). Web tests/clippy/fmt NOT run (web build/test/run banned). Review verdict APPROVE with one non-blocking LOW (F-1 dropped `wrong_key_fails`) and F-190 incidentally closed (fail-fast `load_key().expect(..)` replaces the old warn-and-continue `None` degradation path; non-optional `encryption_key` field). |
| R-14 | done(f9173fe) | f9173fe51e46c4e3f53db7b0dafeb32a649be5d8 | AC1 focused `brdgme_nats` shared crate owns `BotTurnEvent`/`BotCommandEvent` and all protocol constants; bot and web consume it through thin re-export facades (bot unconditional dependency, web optional behind the `ssr` feature), so the protocol types and constants have a single definition site and cannot diverge. AC2 eight-test exact-JSON golden fixture covers serialization, deserialization, and round-trip for the protocol events. AC3 definition-focused grep found exactly one `pub const` definition each for `STREAM_NAME`, `SUBJECT_TURN`, `SUBJECT_COMMAND`, `CONSUMER_TURN`, `CONSUMER_COMMAND`, `MAX_TURN_ATTEMPTS`, `MAX_DELIVER`, `ACK_WAIT`; bot heartbeat uses `ACK_WAIT`. Gates: shared crate test/fmt/clippy pass; bot fmt/clippy pass; allowed `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` pass; bot test 34 pass plus the known four local sqlx `PoolTimedOut` failures (pre-existing local condition, not a regression). Review verdict APPROVE, no Critical/Important findings. |
| R-15 | done(4d6244c) | 4d6244c165f00db4ec3676b79385131f6eeaf979 | Closes F-101/F-102/F-105/F-107 (NATS delivery semantics). AC1 dedup: stream config sets `duplicate_window: 120s` (`nats.rs`) and `publish_bot_turns` sets a `Nats-Msg-Id` via `bot_turn_message_id(game_id, position, updated_at, attempt)` (`game/mod.rs:202-211`, call `:264`) published with `jetstream.send_publish(SUBJECT_TURN, message)`; key is `{game_id}:{position}:{updated_at}:{attempt}`; both AC1 duplicate sources publish attempt 0 (`trigger_bot_turns` and the sweep at `email/sweep.rs:437`), so identical turn state still collides to one delivery; test `duplicate_bot_turn_publish_collapses_to_one_delivery`. AC2 redelivery inside 5-min ack_wait: `process_bot_command_message` `Ok(info)` arm Naks with exponential backoff `2u64.saturating_pow(info.delivered.max(1))` -> 2s then 4s (`game/mod.rs:439-457`), delivery 3 still hits the existing `info.delivered >= MAX_DELIVER` Term branch, all well inside the 5-minute `ACK_WAIT`; test `transient_failure_redelivers_command_well_inside_ack_wait`. AC3 reconcile-not-create-if-absent: `ensure_stream_and_consumers` switched `get_or_create_stream`->`create_stream` and `get_or_create_consumer`->`create_consumer` (durable name in the desired config, `nats.rs:106,148-151`) with drift warnings reworded to "still drifted after reconciliation"; drift policy decision = automatic `create_stream`/`create_consumer` reconciliation selected over startup failure because on the pinned `nats:2.11-alpine` server `create_*` updates existing objects in place (probe-verified), so it is safe/idempotent and will not crash on pre-existing objects; test `ensure_stream_and_consumers_reconciles_drifted_config` deliberately drifts `duplicate_window` to 1s and the `bot-turn` consumer to `ack_wait: 30s`/`max_deliver: 1`, sanity-asserts the drift, then asserts exact restoration to `120s`/`ACK_WAIT`/`MAX_DELIVER` (a `get_or_create_*` impl would leave the drift and fail). AC4 remove `(future)` markers: `nats_protocol/src/lib.rs` `MAX_DELIVER` doc and the two `nats.rs` "future recovery" comments reworded; `rg '\(future\)' rust/web/src/nats.rs` exit 1 (zero hits). Design decisions: the 120s `duplicate_window` collapses only rapid re-publishes of the same turn state (broadcast races, conflict/user-error re-publish within 120s) while the 15-minute reconciliation sweep is a deliberate retry intentionally outside the window, and the message-id includes `attempt` so a real retry bumps the key and is not suppressed (this is the F-2 fix - the old attempt-less key silently deduped the deliberate retry after an invalid bot command); the 2s/4s Nak backoff is exponential (`pow(delivered.max(1))`) and stays well inside the 5-minute `ACK_WAIT`, with delivery 3 falling through to the existing Term ceiling. Gates (locally run, exact exit statuses): `cargo fmt -p brdgme_nats -- --check` exit 0; `cargo clippy -p brdgme_nats -- -D warnings` exit 0; `cargo test -p brdgme_nats` exit 0 (8 passed, 0 failed); `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); `git diff --check` exit 0. CI-deferred (WEB HARD BAN, not claimed passing locally): runtime red/green of the three live-NATS integration tests and the F-2 unit test `bot_turn_message_id_differs_by_attempt` (all compile-verified by the permitted `cargo check --all-targets --features ssr`), and the authoritative F-1 E0080 full-codegen confirmation - `cargo check` structurally defers the const-eval, so CI's `cargo test -p web --features ssr` is authoritative; the F-1 fix (`assume_utc()` lifting `PrimitiveDateTime`->`OffsetDateTime`) is byte-for-byte the reviewer's probe-verified recommendation. Review: comprehensive review verdict CHANGES REQUIRED with four findings - F-1 CRITICAL (`PrimitiveDateTime::format(&Iso8601::DEFAULT)` E0080 build break, invisible to `cargo check` but fatal under CI's full codegen), F-2 HIGH (dedup key omitted `attempt`, suppressing the deliberate retry), F-3 MEDIUM (test 3 non-discriminating), F-4 LOW (comment accuracy); all four resolved and the targeted re-review verdict APPROVED with no new blocker; final verification VERIFIED. No push. |
| R-16 | done(85fff2e) | 85fff2e784e49f0191a417a1dab2325d80b5df45 | hanamikoji-1 Dockerfile stage + bake target + k8s bundle shipped; delivery-list CI guard added; F-211 smoke assertion restored; comprehensive review PASS, no blocking findings |
| R-17 | done(2fa5b35) | 2fa5b356646d00bf120d2782a73aa15797c300d0 | closes F-150..F-156 (WP-52 stats/query-perf "Test? y" rows); gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime DB/web tests deferred to CI, not claimed passing; comprehensive review resolved two stale SQLX cache entries + F48/F21 entry-point coverage, targeted re-review no blockers |
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
| R-37 | pending | | 5.4 done (973ea62); unblocked |
| R-38 | blocked(5.3) | | 5.4 done (973ea62) |
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
| 5.3 require_admin true-path tests | pending | | 5.4 done (973ea62); unblocked |
| 5.4 request-parts test harness | done(973ea62) | 973ea62a3cb407127e527acbb64063305c65414d | in-crate request-parts harness (`crate::test_support::{anonymous, non_admin, admin}`); unblocks 5.3, R-37, R-38 |
| 5.5 13 crates validate+redaction tests | pending | | L; per-crate checklist |
| 5.8 bot test coverage (U11, U12) | pending | | sequence with R-35 |

## Deployment items (section 3, brdgme-config)

| Item | Status | Notes |
|------|--------|-------|
| F-96 Turnstile secret key (prod) | pending | GitOps repo |
| TURNSTILE_SITE_KEY startup check | pending | lands with F-96 |
| config::public_base_url() prod HTTPS | pending | |
| F-207 sqlx migrator reconcile | pending | |
| F-211 hanamikoji-1 delivery gap | done(85fff2e) | code half in R-16 (commit 85fff2e784e49f0191a417a1dab2325d80b5df45) |
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
| 4.7 delivery-list CI guard | done(85fff2e) | same as R-16 (commit 85fff2e784e49f0191a417a1dab2325d80b5df45) |
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

## R-11 evidence

Code commit `13ab0ffd3896f3b0804997a36b2b24a02c2c8147` (verified via
`git rev-parse HEAD`; tracked changes: `rust/web/src/nats.rs`,
`rust/web/src/email/sweep.rs`, `rust/web/src/game/mod.rs`,
`rust/web/src/main.rs`, plus the single R-11 tracker row). `websocket.rs` and
`events.rs` are NOT in the diff; no `TaskTracker`/`tokio-util` `rt` feature
reintroduced; no migration, no CI config, no `Cargo.toml` change.

- **Closes:** F-109 (detached background tasks not drained on shutdown - the bot
  consumer, the max-deliveries-advisory listener, and the six email sweeps).
- **AC1 (tooth-4 historical amendment):** WP-36's ws F55 fix and its regression
  test `rust/web/tests/websocket_hygiene.rs` were deleted by
  `efad81f92b0a1f585410e6f30fdd8de8a3dac518` (deletion independently confirmed:
  `websocket_hygiene.rs` absent; `efad81f9 --stat` deletes it, 153 lines). The
  WP-84 §3g successor proof test is
  `graceful_shutdown_ends_sse_stream_and_server_completes` at
  `rust/web/tests/sse_events.rs:601-657` (Group 5: Graceful shutdown;
  `begin_shutdown` at `:619`, "SSE stream did not end after graceful shutdown" at
  `:649`, "server task did not complete within 5s of shutdown" at `:655`).
  **I1 corrected here:** the prior citation named the Group-4 keepalive test
  `sse_events.rs:551-595` (`sse_stream_survives_past_request_timeout_with_keepalive`),
  which never triggers a graceful shutdown; the error originated in the survey
  (`R-11-SURVEY.md:58-59,334`).
- **AC2 (owner ruling 6.3b):** process `CancellationToken` threaded into
  `supervise_consumer` (`nats.rs:280`), `run_bot_command_consumer` /
  `run_bot_command_consume_loop` (`game/mod.rs:263,311`),
  `run_max_deliveries_advisory_listener` (`nats.rs:214`), and all six sweeps
  (`sweep.rs:324,635`); the eight handles are retained by `main`
  (`main.rs:118-125`) and joined under a 5s bounded drain (`main.rs:173-184`).
  Shutdown-path tests call the real production functions:
  `bot_command_consume_loop_exits_on_shutdown` (`game/mod.rs:1284`),
  `sweep_stops_on_shutdown` (`sweep.rs:1736`),
  `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` (`nats.rs:467`),
  `supervisor_backoff_sleep_is_interrupted_by_shutdown` (`nats.rs:517`). The
  advisory listener (a third F-109 §1c family AC2 does not name) is also wired
  up (bonus completeness).
- **AC3 (concrete harm F-109 cites):** detached SSE spawns are bounded for the
  normal case by R-10's committed mechanism - per-connection token
  (`events.rs:42-59`) + global `shutdown.cancelled()` arms
  (`events.rs:156-158,243-245`) + axum `with_graceful_shutdown` - proven by
  `graceful_shutdown_ends_sse_stream_and_server_completes`
  (`sse_events.rs:601-657`). No `TaskTracker` reintroduced (correct per task
  constraint; `websocket.rs`/`events.rs` absent from the diff). **Documented
  residual (I2):** a task blocked in `client.subscribe()` (`events.rs:86,93,206`)
  at shutdown under a broken NATS connection holds `tx`, so the response body
  never ends and axum's timeout-less graceful shutdown hangs until k8s SIGKILL;
  narrow, externally bounded, acknowledged by the survey
  (`R-11-SURVEY.md:73-76`), and not closable by the prescribed `TaskTracker` at
  its placed location. Owner confirmation recommended.
- **Runtime:** the four new tests are compile-verified only; runtime web tests
  deferred to CI by explicit ban (web build/test/run forbidden). They are
  pure-tokio (no DB/NATS) and should pass in CI as written (same disclosed
  limitation as R-08/R-09/R-10).
- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0 (one pre-existing `proc-macro-error2`
  future-incompat warning, unrelated; nothing from the R-11 files; no new
  `unused`/`unreachable`/`dead_code` warning).
- **Review:** comprehensive independent review
  (`review/R-11-COMPREHENSIVE-REVIEW.md`) verdict ACCEPT; no Critical
  findings; two Important findings, neither a functional defect in the committed
  code - I1 (AC1 successor citation, the doc correction applied in this row) and
  I2 (AC3 subscribe-blocked residual, owner confirmation recommended); three
  Minor notes (M1 advisory-listener unit-test gap, beyond AC2 scope; M2 backoff
  test timing nuance; M3 runtime unverified, disclosed). The committed code is
  accepted as-is. **The required targeted doc-only re-review
  (`review/R-11-TARGETED-REREVIEW.md`) returned PASS**, resolving I1: it confirms
  the AC1 successor citation correction from `:551-595` to `:601-657` against the
  actual source and verifies no unrelated tracker content was damaged; no code
  re-review was needed.

## R-16 evidence

Code commit `85fff2e784e49f0191a417a1dab2325d80b5df45` (verified via
`git rev-parse`; message `ci: enforce game delivery list parity (R-16, F-208,
F-211)`). Closes F-208 (built-but-unshipped game class) and the code half of
F-211 (hanamikoji-1 delivery gap); delivers process fix 4.7 (delivery-list CI
guard).

- **AC1 (hanamikoji-1 shipped):** `rust/Dockerfile:304` distroless stage
  `hanamikoji_1_http` (same SHA pin as the 26 prior game stages);
  `docker-bake.hcl:47` `"hanamikoji-1"` in the `tgt` matrix (alphabetical);
  `k8s/base/game/hanamikoji-1/` 5-file bundle (deployment, service,
  game-version, http-scaled-object, kustomization) copied field-by-field from
  the `jaipur-2` neighbor; registered in `k8s/base/game/kustomization.yaml:18`,
  `k8s/prod/app/kustomization.yaml:80-82`, and `Tiltfile:21`.
- **AC2/4.7 (CI guard):** `scripts/check-delivery-lists.sh` derives all four
  lists by pure text parsing, computes `expected = cargo_members - ALLOWLIST`,
  and runs three bidirectional set-equality checks (`comm -23`/`comm -13`),
  printing named offenders and `exit 1` on any mismatch. Wired as the first
  step after checkout in `.github/workflows/ci.yml:77-79` (step-level
  `working-directory: .` = repo root).
- **Post-allowlist comparable set counts (independently re-derived):**
  - Cargo game members (raw total): **28**
  - expected (members minus the one-entry allowlist): **27**
  - Dockerfile distroless game stages: **27**
  - docker-bake.hcl game targets (31 total minus web/migrate/bot/operator): **27**
  - k8s/base/game dirs Cargo-intersected (Rust): **27** (of **44** total dirs;
    the other **17** are Go v1 games with no Cargo entry, intersected out via
    `comm -12`, no Rust/Go name collision)
  - `comm` both ways for expected vs {docker, bake, k8s_rust}: all empty (equal).
- **Allowlist (exactly one entry):** `ALLOWLIST="lords-of-vegas-1"`
  (`check-delivery-lists.sh:22-24`), commented `WIP, owner-excluded (BACKLOG Out
  of Scope). Review: 2026-09-01.` It is a real Cargo member, so it genuinely
  subtracts one (28 -> 27).
- **Positive proof:** `bash scripts/check-delivery-lists.sh` on the real repo ->
  `OK`, exit 0. `bash scripts/check-delivery-lists.test.sh` -> `PASS`, exit 0.
- **Negative proof (fixture):** running the guard with
  `scripts/fixtures/delivery-lists-broken/` as CWD -> `MISMATCH: rust/Dockerfile
  stage / cargo game members with no rust/Dockerfile stage: bar-1`, exit 1.
  Diagnosis: `bar-1` is a Cargo member with a bake target and a k8s dir but no
  Dockerfile stage - exactly the F-208 built-but-unshipped class; the bake and
  k8s checks pass on the fixture, isolating the single clean mismatch.
- **Raw-count mismatch is NOT a finding:** per the remediation plan scope note
  (98-REMEDIATION-PLAN.md:574-576, F-208a refuted), the script header
  (`check-delivery-lists.sh:10-12`) states raw count differences are never
  reported on their own; only named set differences are. The `compare` function
  emits only named missing/stale entries, never counts. The 44-vs-28 raw k8s/Cargo
  difference is the expected Go-game asymmetry, not a defect.
- **F-211 e2e assertion:** `rust/web/end2end/tests/page-loads.spec.ts:8` restored
  to `getByRole("link", { name: "Start a game" })` (replacing the weakened
  sidebar-satisfied `getByRole("heading", { name: "brdg.me" })`). Committed but
  NOT executed: running it requires the full e2e stack (release web binary + game
  service + Postgres via run.sh/tilt/docker), all prohibited, and
  `end2end/node_modules` is absent. Advisory regardless - the e2e CI job is
  `continue-on-error: true` (ci.yml:155).
- **Review:** comprehensive independent review verdict **PASS, no blocking
  findings**. All four acceptance criteria met; the guard is correct, fails
  closed, and the three delivery lists are set-equal at 27 against the 27
  expected Cargo members. Four LOW/informational observations (F-211 red-handover
  rationale imprecision; k8s intersection assumes disjoint Rust/Go names; guard
  cannot detect a fully-orphaned k8s dir; guard runs only under the `rust` path
  filter) and six residual verification deferrals (e2e not executed; docker
  bake/build not run; kustomize/kubeconform deferred to CI; Tiltfile Starlark
  unvalidated; `scripts/rust-test.sh` skipped per laptop OOM ban, no Rust source
  changed; tracker update deferred to this row) - none blocking.

## R-17 evidence

Code commit `2fa5b356646d00bf120d2782a73aa15797c300d0` (verified via
`git rev-parse`; message `fix(web): correct stats queries and coverage (R-17,
F-150, F-151, F-152, F-153, F-154, F-155, F-156)`). Closes F-150 through F-156
(the seven WP-52 stats/query-perf "Test? y" rows and their follow-up findings).
Tracked changes: `rust/web/src/stats/queries.rs`, `rust/web/src/stats/mod.rs`,
`rust/web/src/db/social.rs`, `rust/web/src/game/server_fns.rs`,
`rust/web/src/index.rs`, `rust/web/src/test_support.rs`, plus two deleted
`.sqlx` cache JSON entries.

- **Checklist provenance:** the original WP-52/WP-53 checklist
  `docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B5-web-domain-stats-misc.md`
  was compacted (deleted) in `d89fa34` ("docs(review): compact 2026-07-23 rust
  review to summary", 206 lines removed). It is NOT reconstructed here. The
  replacement citation is `docs/reviews/2026-07-23-rust-review/SUMMARY.md:114`
  ("WP-52 - stats + query performance pass."). The seven WP-52 "Test? y" rows
  remediated here, as recorded from the pre-compaction checklist, are: `wd F50`,
  `wd F51`, `wd F55`, `wd F48`, `wd F52`, `wd F46`, `wd F21`.

- **Findings closed (F-150..F-156):**
  - **F-150** (all seven "Test? y" rows shipped with no test): each row now has
    a direct test - see the per-row citations below.
  - **F-151** (High, `wd F48` game-type filter on only one side of a FULL OUTER
    JOIN): the `gtu` rating side of `game_type_stats` now joins `game_types` and
    applies `($3::text IS NULL OR gt_f.name = $3)` (`queries.rs:91-150`), so a
    filtered request can no longer leak another type's rating/record.
  - **F-152** (Medium, `wd F55` `NULLS LAST` skipped the third byte-identical
    ordering): `recent_form_for_game_type`'s window ordering now carries
    `NULLS LAST` (`queries.rs:716`), matching `finished_games` (`:309`) and
    `recent_form` (`:630`).
  - **F-153** (Medium, `wd F50` dead `#[allow(dead_code)]` const used by zero
    sites): the `ELIGIBILITY_PREDICATE` const and its `#[allow(dead_code)]` are
    deleted; grep for `ELIGIBILITY_PREDICATE` and for `allow(dead_code)` in
    `stats/queries.rs` both return zero.
  - **F-154** (Medium, `wd F52` canonicalization turns an unknown `game_type`
    into no filter): `get_player_history` now returns `Ok(None)` when
    `find_game_type_name` resolves to `None` (`mod.rs:343-350`), matching
    `get_player_game_type_stats`' 404-on-unknown behaviour.
  - **F-155** (Low, `wd F53` justifying comment copy-pasted and wrong at one
    site): the stale `ELIGIBILITY_PREDICATE`/runtime-check comment block is
    removed; the surviving runtime-`query_as` comments are per-site and accurate.
  - **F-156** (Medium, `wd F74` bound truncates the friends feed
    alphabetically): the `.take(20)` alphabetical truncation is removed;
    `get_logged_in_index` now streams ALL friends through a bounded
    `.buffered(10)` (`index.rs:47-68`), preserving `list_friends`' stable
    alphabetical (`ORDER BY lower(u.name)`) output order while no longer dropping
    friends past position 20. Semantics: no alphabetical truncation, bounded
    concurrency `buffered(10)`, stable alphabetical output.

- **Test-first / pre-fix RED evidence (compile-verified; runtime deferred to
  CI, NOT claimed passing):**
  - **F-151:** `game_type_stats_explicit_filter_returns_only_requested_type_and_rating`
    (`queries.rs:1140`) fails pre-fix - the unfiltered FULL OUTER JOIN rating
    side yields BOTH types and the alphabetically-first ("Acquire") row orders
    first with the wrong rating; post-fix it asserts a single "Zebra Game" row
    with rating 1400.
  - **F-152:** `recent_form_for_game_type_null_finished_at_does_not_displace_recent`
    (`queries.rs:1925`) fails pre-fix - PostgreSQL defaults `DESC` to
    `NULLS FIRST`, so the NULL-`finished_at` legacy row sorts to `rn = 1` and
    displaces a dated game from the 3-game window.
  - **F-154:** `get_player_history_unknown_game_type_returns_none` (`mod.rs:408`)
    fails pre-fix - `find_game_type_name`'s `None` bound straight into
    `($3 IS NULL OR gt.name = $3)` as "no filter", returning the full history
    wrapped in `Ok(Some(_))`.
  - **F-156:** `get_logged_in_index_includes_late_alphabetical_friend_with_recent_game`
    (`index.rs:131`) fails pre-fix - `.take(20)` on the name-ordered list
    permanently drops `friend_21` (alphabetically last, holder of the most recent
    visible game); the >20 late-alphabetical regression asserts that friend is
    present with its game.
  - **NOT a RED test (truthful note):** `rating_before_aggregates_exclude_nulls`
    (`queries.rs:1344`, wd F51) was REWRITTEN to call `game_history` directly
    (the old body queried raw SQL and would have kept passing even if the LATERAL
    were deleted). It is a PASSING regression guarding the already-correct
    `LEFT JOIN LATERAL` NULL-exclusion semantics, not a pre-fix failure, and is
    not described as a RED test.

- **All seven WP-52 "Test? y" rows - function/test citations (compile-verified;
  runtime DB tests deferred to CI, NOT claimed passing):**
  - **wd F50** - all eight named stats query functions, current direct-test
    calls: `overall_totals` (`queries.rs:43`; tests `:977,:982,:1002,:1006`),
    `game_type_stats` (`:91`; tests `:1029,:1158`), `finished_games` (`:281`;
    tests `:1288,:1316,:1328`), `game_history` (`:416`; tests `:1398,:2026`),
    `game_history_count` (`:498`; tests `:2152,:2157`), `head_to_head` (`:528`;
    tests `:1487,:1512`), `recent_form` (`:613`; test `:1561`),
    `recent_form_for_game_type` (`:696`; tests `:1864,:1966`).
  - **wd F51** - `game_history` (`queries.rs:416`) via the rewritten
    `rating_before_aggregates_exclude_nulls` (`:1344`), which now calls
    `game_history` directly and asserts the LATERAL `match_elo` ignores the NULL
    seat (1200/1200/1200) while `player_count` still counts it, and the
    game-type filter excludes the "Duel" game.
  - **wd F55** - `finished_games` (`:309` `NULLS LAST`, existing direct calls
    `:1288,:1316,:1328`) and `recent_form` (`:630` `NULLS LAST`, existing direct
    call `:1561`), PLUS the new F-152 sibling
    `recent_form_for_game_type_null_finished_at_does_not_displace_recent`
    (`:1925`) covering the third ordering at `:716`.
  - **wd F48** - `game_type_stats` (`:91`) via
    `game_type_stats_explicit_filter_returns_only_requested_type_and_rating`
    (`:1158` call) PLUS the new entry-point test
    `get_player_game_type_stats_returns_requested_type_and_rating` (`mod.rs:496`)
    driving the real `get_player_game_type_stats` (`mod.rs:239`) and its
    defence-in-depth `.find(|s| s.game_type_name == canonical)` (`mod.rs:270`).
  - **wd F52** - `get_player_history` unknown-type test
    `get_player_history_unknown_game_type_returns_none` (`mod.rs:408`).
  - **wd F46** - `get_player_history` clamp test
    `get_player_history_clamps_page_bounds` (`mod.rs:452`): page 0 and negative
    clamp up to 1, page 1_000_001 clamps down to 1_000_000 (clamp already in
    place; this guards it).
  - **wd F21** - `should_hide_add_friend_many` (`db/social.rs:182`)
    batch-vs-singular equivalence test
    `should_hide_add_friend_many_matches_singular_per_row_state` (`social.rs:956`)
    PLUS the `get_game_details` ENTRY-POINT test
    `get_game_details_batch_add_friend_reflects_per_player_state`
    (`server_fns.rs:2799`), which drives the real server fn through the harness
    against an in-process local mock game service (canned PubRender; no real
    service) and asserts the batched affordance (`server_fns.rs:316,:385-388`)
    hides an accepted friend and shows a pending-INCOMING requester.

- **SQLX cache:** two obsolete `query!` cache entries deleted
  (`.sqlx/query-11b46fcc7ad564627ecea8b6bacf68a615e6e1388f.json` and
  `.sqlx/query-350faff674aef8d03893fe73be38809711dea11492.json`) because
  `game_type_stats` and `recent_form_for_game_type` were converted from the
  compile-time `sqlx::query!` macro to runtime `sqlx::query_as` (named `FromRow`
  structs `GameTypeStatsRow` / `RecentFormForGameTypeRow`); the macro cache
  entries no longer correspond to any `query!` invocation.

- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0. Runtime DB/web tests are deferred to CI and are NOT
  claimed passing here (web build/test/run banned; the new `#[sqlx::test]` cases
  need Postgres).

- **Review:** the comprehensive review identified, and the implementation then
  resolved, two stale SQLX cache entries (the two deleted JSON files above) plus
  the F48 (`get_player_game_type_stats`) and F21 (`get_game_details`) ENTRY-POINT
  coverage gaps (the two new entry-point tests). A targeted re-review found no
  blockers.

- **Transparency (command-constraint deviation):** during the targeted re-review
  a reviewer ran `cargo clean -p web` to force a full recompile. This violated
  the "only `cargo check -p web` variants" command constraint, but it only
  removed ignored web build artifacts (no source, no tracked files). It is
  recorded here rather than concealed. All actual verification checks were the
  permitted gate above.

## 5.4 evidence

Code commit `973ea62a3cb407127e527acbb64063305c65414d` (verified via
`git rev-parse`; tracked changes: `rust/web/src/test_support.rs` new 96 lines,
`rust/web/src/lib.rs` +3, `rust/web/src/admin.rs` +31; 130 insertions,
0 deletions).

- **Delivers:** reusable in-crate request-parts test harness
  (`rust/web/src/test_support.rs`), gated
  `#[cfg(all(test, feature = "ssr"))] pub(crate) mod test_support;`
  (`lib.rs:25-26`).
- **Helper API:** `crate::test_support::{anonymous, non_admin, admin}`, each
  `(&PgPool, async closure) -> T`. Private internals: `run_with_session`
  (the sanctioned `Request::new(()).into_parts()` +
  `parts.extensions.insert(session)` + `Owner::new()` + `provide_context` +
  `ScopedFuture` mechanism), `seed_user` (inserts real `users` + matching real
  `user_auth_tokens` rows), `authenticated` (MemoryStore-backed
  `tower_sessions::Session` with seeded `SessionUser`).
- **Representative consumer:**
  `admin_list_bots_distinguishes_anonymous_non_admin_admin` (`admin.rs:2471-2500`)
  calls the real `#[server]` fn `admin_list_bots` (`admin.rs:991-999`) directly
  in-process under three identities:
  - anonymous (empty MemoryStore session) -> `Err` containing "Not authenticated"
  - non-admin (real user + real `user_auth_tokens` row, `is_admin=false`) ->
    `Err(ServerError(msg))` with `msg == ADMIN_REQUIRED`
  - admin (real user + real `user_auth_tokens` row, `is_admin=true`) ->
    `Ok` and empty list (`DELETE FROM bots` first)
- **Three-identity evidence:** real `tower_sessions::Session` backed by
  `MemoryStore`, placed in real axum request `Parts` extensions; `Parts` +
  cloned `PgPool` provided into a Leptos `Owner`; closure run through the real
  `ScopedFuture` reactive mechanism. Real `users` and `user_auth_tokens` rows
  inserted; `get_current_user`, authorization, request parts, session
  extensions, and DB token validation all run for real. Real `admin_list_bots`
  results asserted. NATS-free.
- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0 (one pre-existing `proc-macro-error2`
  future-incompat warning, unrelated).
- **Runtime:** the seed test is compile-verified only; runtime web tests
  deferred to CI by explicit ban (web build/test/run forbidden). Same disclosed
  limitation as R-08..R-12.
- **Review:** comprehensive independent review verdict APPROVE; no
  Critical/High/Medium findings; two informational Low deferrals (L1: ad-hoc
  harness de-duplication of `auth/server.rs::with_session_context` and
  `proposals.rs::with_logged_in_context` deliberately deferred as a follow-up;
  L2: runtime test deferred to CI per hard constraint).
- **Unblocks:** 5.3 (require_admin true-path tests), R-37 (web auth
  hardening), R-38 (admin surface + db module). 5.4 is done; those items'
  "blocked by 5.4" condition is removed. 5.3/R-17/R-37/R-38 themselves remain
  pending or blocked on other dependencies - not marked done here.
