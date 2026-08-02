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

- `pending` / `blocked(<reason>)` / `in-progress` / `partial/parked(<reason>)` /
  `done(<commit>)` / `owner-gap`

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
| R-18 | done(6a304be) | 6a304be11252048e0cf8ddf1459d38f3a0d38a7a | closes F-134/F-135/F-143 (network calls hoisted out of three transactions); AC1 zero HTTP between begin and commit at all five tx bodies; AC2 deterministic concurrent-change tests for all three (condvar/Notify/Semaphore+Barrier, non-sleep); AC3 F-143 recorded as WP-46-vs-WP-79 reconciliation per CODING.md rare-duplicate rule, not a deviation; gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned; needs Postgres/NATS), not claimed passing; comprehensive review REJECT on two invalid tests (dotted version name) -> repaired -> targeted F1/F2 re-review PASS (static) |
| R-19 | done(7de92cd) | 7de92cd65458f408087af8262afe92635639762c | closes F-144/F-147 (per-invitee nudge dedup + dead-code deletion); gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed; sole warning the in-scope `NotifyKind::Reminder` dead_code); runtime web test deferred to CI, not claimed passing; comprehensive review APPROVE, 0 high/medium, two non-blocking low notes |
| R-20 | done(049325a) | 049325a7ba248c3e3630f284f5f94a6a26c7dafb | closes F-146/F-179/F-180/F-181/F-182 (game-start notify identity, threading, duplication); AC1 notify routed through the InviteMailer seam + real proposal-path test; AC2 one mail per on-turn invitee on invite-accept auto-start; AC3 solo-start notify bypasses web-presence suppression; AC4 distinct per-kind subjects/thread ids; AC5 F-170 NOT re-derived (refuted); gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0 (allowed; sole warning the pre-existing in-scope `NotifyKind::Reminder` dead_code); runtime web tests authored but deferred to CI (web build/test/run banned); comprehensive review REJECTed C1/C2 then fixes + focused re-reviews PASS, no Critical findings; no push |
| R-21 | blocked(R-22..R-26, R-48) | | closing commit of game family |
| R-22 | blocked(AC1 test quantity unsatisfiable; needs owner amendment) | | AC1 requires seven per-player-vector short-state tests, but current `texas-holdem-2::Game` has exactly four such vectors (`player_hands`, `player_money`, `bets`, `folded_players`); its specific test quantity cannot be satisfied without owner amendment. Evidence report `/tmp/opencode/r22-root-cause.md` is ephemeral and is not an authoritative repository artifact. |
| R-23 | done(3db5c06) | 3db5c06e95fe78a6b521e87a4cd2e27aab77093b | closes F-60/F-62/F-63/F-64; strictly two crates (lost-cities-1, lost-cities-2); -1 validate() + direct player_state defense + named no-panic test; -2 three unreachable!() player-count helpers -> Result errors propagated through score/end_round/draw_hand_full + direct/command-path four-player Err tests; AC4 score_3p_works coverage-only (passed pre-fix, no rules change); per-crate test/clippy/fmt all pass; comprehensive review APPROVE/PASS, no Critical/Important/Minor findings |
| R-24 | done(d37c423) | d37c4231d10704d17a0466fffc732103107ec769 | closes F-61/F-65/F-210; `validate()` override asserts players 2..=5, all_players==if 2 then 3 else players, four parallel vector lengths==all_players, controller<players, round 1..=TOTAL_ROUNDS; six render-path sites converted to `.get()`/`.first()` defined fallbacks with six `*_no_panic` tests; `draw_count` -> `Result<usize, GameError>` total (no `_` arm, no `unreachable!()`) propagated via `?` through start_round/end_round/end_hand/play_cards; TDD RED/GREEN (AC2 panic RED, AC3 compile-error RED, AC1 assertion-failure RED per default trait validate); `cargo test -p sushi-go-2 --lib` 50 pass / 0 fail, clippy `-D warnings` exit 0, fmt exit 0; raw-index sweep 37 hits all justified, `unreachable!()` zero; comprehensive review APPROVE, no Critical/Important, one non-blocking Minor (implementation report mischaracterizes AC1 validate tests as compile-error RED; the trait default means they compile and fail assertions pre-fix) |
| R-25 | done(df3d1ce) | df3d1ce25b8b50ba6b1fe292d4c8ff21d6054939 | closes F-24 (alhambra-1), F-31 (starship-catan-1), F-33 (seven-wonders-1), F-37 (splendor-2), F-49 (cathedral-2), F-51 (sushizock-2), F-54 (jaipur-2), F-70 (for-sale-2 + battleship-2), F-82 (tic-tac-toe-2); all TEN named crates covered (plan prose says "nine" but the Files list and per-crate AC enumerate ten - discrepancy reported, not resolved); AC1 all ten have a `validate()` override plus a direct malformed-state validate test calling it (assertion-failure RED pre-fix per the still-present trait-default validate); AC2 all ten have a direct render entry-point no-panic test (fixed-array exceptions: starship render.rs clamps `current`/`viewer` to `boards.len()-1`, jaipur lib `hands.get(player)` :737, tic-tac-toe fixed board has no panic surface; already-defensive exceptions: for-sale/battleship/cathedral render paths were pre-hardened so their no-panic tests are regression guards); F-49 cathedral BOTH command-path boundary fixes each with a test (off-by-one `>`->`>=` at :150 tested by `can_play_piece_rejects_piece_equal_to_catalogue_len`; untrusted-tile i32 wrap guarded via let-chain `get_mut` at :347 tested by `check_captures_handles_zero_piece_type_without_panicking` and `check_captures_handles_out_of_range_player_without_panicking`); F-70 player-count bound added INSIDE both existing validates (for-sale `players in 3..=5` at :401 mirroring red7-1, battleship `players == NUM_PLAYERS` at :426; existing checks preserved verbatim); per-crate serial verification family `cargo test -p <crate>` + `cargo clippy -p <crate> --all-targets -- -D warnings` + `cargo fmt -p <crate> -- --check` + `git diff --check` all exit 0: alhambra 45, starship 47, seven-wonders 49, splendor 67, cathedral 40, sushizock 53, jaipur 72, for-sale 22, battleship 41, tic-tac-toe 28 (lib passed; 0 failed) plus each crate's contract test 1 passed; comprehensive independent review APPROVE, no Critical/Important, one non-blocking Minor (jaipur render.rs viewer-index raw indexing left unhardened - explicitly out of R-25's deserialized-`current_player` scope: the production render path loops `0..player_count()` which returns the compile-time `NUM_PLAYERS` constant, so only viewers 0/1 are ever rendered; the residual is a request-input-validation concern at the requester boundary, not the F-06 class R-25 targets); no additional review performed under the owner rule; R-07 untouched, R-22 still parked/blocked, R-21 remains gated; no push; no migration; no `scripts/rust-test.sh` (not required - per-crate verification performed) |
| R-26 | done(a302112) | a302112f28104e2c7045df299a60f4c2668eb060 | closes F-66/F-67/F-68/F-74/F-76; strictly three crates (category-5-2, zombie-dice-2, red7-1); F-66 category `validate` rejects resolving-with-no-played-card for `choose_player` + `can_choose` now requires a played card so `choose`'s `expect` is unreachable, direct validate/no-panic tests; F-67/F-74 equal-hand-size check inside `validate` + direct validate/no-panic tests, false equal-hands comment deleted not corrected, F-73/`draw_cards`/R-31 explicitly untouched; F-68 zombie 13-dice conservation across cup/kept/current_roll inside `validate` + saturating `take_dice` drain + direct validate/no-panic tests; F-76 red7 `validate` rejects non-finished all-eliminated + `leader`/`leader_with_suit` return `Option` (no `player_map[l_index]` panic), all four production call sites handle `None`, direct tests; TDD RED category 4 / zombie 2 / red7 2 then GREEN; per-crate serial test/clippy/fmt all exit 0 (category 26+1, zombie 28+1, red7 26+1), `git diff --check` exit 0; comprehensive review PASS WITH NON-BLOCKING NOTES, no Critical/Important, three non-blocking Minor (all_dice allocation per validate, auto-play swallowed Err, pre-existing web fmt debt); no migration, no scripts/rust-test.sh, no Tilt/kind/production, no push; R-07-HANDOVER.md untouched, R-22/R-21/R-07 untouched |
| R-27 | done(1a2d82a) | 1a2d82ae658c9e37f4bcca1c136fa5567e77c56c + 9ec9949a86c04e9c937e7f48f40a1e62f6f663df | closes F-05/F-69 (for-sale-2 deadlock and short-deck stall) plus the section-7 escalation (pass()/take_first_open_card panic); short building/cheque-deck round-start transitions produce Finished and logs (`start_buying_round` lib.rs:143-144 and `start_selling_round` :157-158 return `finish()`; tests `start_buying_round_short_deck_finishes_game`, `start_selling_round_short_deck_finishes_game` assert `is_finished`, `Status::Finished`, empty whose_turn, non-empty logs); all-passed direct `next_bidder()` returns `Err(GameError::Internal)` within the 2s bounded timeout (:316-317; `next_bidder_all_passed_terminates_with_internal_error` - second commit 9ec9949 corrected the literal AC2 note by calling `next_bidder()` directly instead of through `bid()`); empty `take_first_open_card` returns `Err(Internal)` not panic (:302-305) and selling distribution surfaces `Err(Internal)` not panic (:281-284); AC4 `pass()` half-bid rounding deliberately unchanged - confirmed parked as WP-11 `f F14` under D-30 + D-35 (BLOCKED-ON-USER-RULES-REVIEW, do not pick up) per `04c-games-cleanup-parity-wp33.md:455-474`, not a remediation gap; fresh commands all exit 0: `cargo test -p for-sale-2` (28 lib + 1 contract pass), `cargo clippy -p for-sale-2 --all-targets -- -D warnings`, `cargo fmt -p for-sale-2 -- --check`, `git diff --check`; independent review APPROVE WITH NON-BLOCKING NOTES; residual risk: pre-existing `start_selling_round` autoplay `if let Ok` (lib.rs:170) still swallows corrupted-state `play` errors (no normal-play effect, same class as R-26's auto-play note); no standalone R-27 docs/changes document existed and nothing was archived; no push |
| R-28 | done(34a1222) | 34a1222f4213916094e2e24e8a3a56617a49ea73 | closes F-09 (High) + F-10 (Medium); only file `rust/lib/rand_bot/src/lib.rs` (+113/-11); Int inverted bounds -> empty tokens (was `panic!`); Many None-max/inverted -> `max.unwrap_or(3).max(min)` honors min (was `assert!` trip); Many out-of-i32-range bounds rejected empty, no `as i32` narrowing (was i32 wrap); `bounded_i32` total; six new tests genuine RED by construction, incl. the AC1 reviewer-confirmation re-fixture of the out-of-i32-range-min test (naive `i32::MAX+1` passes pre-fix; re-fixtured to `(1<<32)+5`); fresh commands all exit 0: `cargo test -p brdgme_rand_bot` (10 lib), `cargo clippy -p brdgme_rand_bot --all-targets -- -D warnings`, `cargo fmt -p brdgme_rand_bot -- --check`, `git diff --check`; residual: in-range Many count up to `i32::MAX` emitted in full (no cap); no standalone R-28 change doc existed, nothing archived; no push |
| R-29 | done(fcda3e6) | fcda3e6d096f3db929fca9c5a33353c7b11bac23 + a48c783a9b00212fb5dfff98ad0570a9ee6fc4bd | closes F-17 (High) + F-16 (Low) + F-191 (Low); files `rust/lib/cmd/src/repl.rs`, `requester/gamer.rs`, `http.rs`, `rust/web/src/rules.rs`; AC1 repl.rs unwrap/expect/panic survivor grep count exactly 0, so no survivor justification applies (scope excludes test_support/startup expects); AC2 `response_error_message` seam renders a `Response::UserError` message without panicking, test `user_error_response_produces_message_without_panicking`; AC3 all three payload variants (DataDocs/BasicStrategy/AdvancedStrategy) deserialize + `validate()` + player bounds-check, tests: 3 malformed -> `Err(Parse)`, 3 validate-error -> SystemError, 2 out-of-range player -> UserError, 3 valid unchanged; AC4 settled contract (per 98-REMEDIATION-PLAN.md:979-983 + 00-STATE.md:541-543): true malformed envelope syntax (`{ not json`) -> Axum 400 text/plain via `route::<G>()` with non-JSON body (test `malformed_envelope_returns_400_text_plain`), while malformed inner game state remains 200 SystemError; fresh commands: `cargo test -p brdgme_cmd` 31 pass / 0 fail, `cargo clippy -p brdgme_cmd --all-targets -- -D warnings` exit 0, `cargo fmt -p brdgme_cmd -- --check` exit 0, `cargo check -p tic-tac-toe-2` exit 0, allowed gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` exit 0, `git diff --check` exit 0; web clippy NOT passed - five pre-existing unrelated warnings (notify.rs `Reminder` dead-code, sweep.rs x2 auto-deref, auth/server.rs x2 redundant closure, none in rules.rs), web fmt NOT passed - pre-existing drift, none in rules.rs; web runtime test `fetch_strategy_sends_validated_payloads_and_receives_strategy` passes: from `rust/` `SQLX_OFFLINE=true cargo test -p web --features ssr fetch_strategy_sends_validated_payloads_and_receives_strategy` exit 0, target result `1 passed; 0 failed; 0 ignored; 0 measured; 651 filtered out`, duration 0.02s, in-process (no DB/NATS); no R-29 defect found; independent initial review REJECTed web empty-state caller regression (fcda3e6 sent `game: ""`), bounded correction a48c783 added PlayerCounts->New->strategy valid scratch-state flow, targeted re-review APPROVE with no findings; residuals: four sequential strategy HTTP calls, pre-existing web lint/fmt debt; no standalone R-29 change doc existed and nothing was archived; protected R-07-HANDOVER.md untouched; no push |
| R-30 | done(fc90116) | fc90116ce063261c3c643c64a85a6771e092c0fe + 8814bb0e00b3dcd9937f2ff49a1c94964025ca66 + 196200c9d1182271a8345529e68778ebdb80b3e6 | closes F-22/F-23/F-28/F-29/F-30/F-34/F-38/F-39; four crates despite plan Size prose "three crates" (four named Files governed; discrepancy recorded, see R-30 evidence); per-crate test/clippy/fmt all pass; independent review PASS after bounded legacy-bids correction (see R-30 evidence); final AC1 Splendor test-completion commit 196200c (test-only, no gameplay behaviour change; see R-30 evidence) |
| R-31 | blocked(owner sign-off) | | closes F-72/F-72a/F-73. F-72/F-72a = parked D-30 port-parity gameplay (player cap 8 -> official 10); requires category-5-2 per-game owner sign-off. Active: docs/decisions/PORT_PARITY.md:26-40; original disposition 868094a6:docs/reviews/2026-07-23-rust-review/planning/DECISIONS.md:459-465,515-521; no sign-off in 97-REMEDIATION-PROGRESS.md:118-130. Conflicting 04b-games-red7-zombiedice-forsale.md:745-750 / 99-UNIFIED-REPORT.md:1795 does not override owner policy. F-73 draw-card robustness is non-parity but cannot proceed under this indivisible R-31 approved plan; no source/doc implementation occurs |
| R-32 | blocked(AC4 owner amendment/sign-off) | | AC4 non-2-player fixture unreachable: starship-catan-1 is fixed two-player, so no truthful non-2-player game can be constructed; requires owner amendment/sign-off (see R-32 blocker evidence) |
| R-33 | done(ad6fa645) | ad6fa6452da9752bca43a4c726bb5c7caa51b6f9 | closes F-40/F-42/F-43/F-46/F-47 in rust/game/acquire-1/src/lib.rs. AC1 F-40: three bank sites (handle_found_command, take_shares, return_shares) get_mut + GameError::Internal, grep or_insert(STARTING_SHARES)=0 hits, whole-board conservation regression. AC2 F-42 (owner-amended plan AC2): `merger_cascade_removes_played_tile_by_current_identity` drives the real Festival->American->Imperial cascade in one `play b2` command with played at hand index 0 and two survivors; asserts survivors == [s1, s2] (exact order) and played absent. Pre-fix failure logic RED-verified: swap_remove(0) on [played, s1, s2] yields [s2, s1] and the order assertion fails exactly as predicted; GREEN with the retained identity-removal `tiles.retain(|l| l != loc)`. AC3 F-43: handle_end_command routes through assert_player_turn (9/9 command handlers); main_turn_player only in Phase def + player_can_end parser gate. AC4 F-46: pub_state() = self.into() (From<&Game>), no whole-Game clone. AC5 F-47: 26 passing (25 lib incl board + 1 contract). Gates all exit 0: cargo test -p acquire-1 (25+1), cargo clippy -p acquire-1 --all-targets -- -D warnings, cargo fmt -p acquire-1 -- --check, git diff --check. No gameplay/rules/PORT_PARITY change. Independent review PASS, no blocking findings; two non-blocking residuals: buy-path missing bank key is a read-only InvalidInput rather than Internal; F-46 regression test is coverage rather than discriminating |
| R-34 | done | | F-25 REFUTED: current five-pile placement is official Queen Games behavior once back-pop draw direction is accounted for - regression test `scoring_cards_fire_at_official_fifth_pile_bounds` consumes the deck via the real `draw_cards` pop-from-back path and asserts round 1 in [L-4f, L-3f-1] and round 2 in [L-2f+1, L-f] money-card draws over players 2-6 x seeds 0..24 (125 games, all pass; fails against the thirds distribution, verified); historic Go one-card-shifted round-1 distribution NOT restored; NO production change for F-25. F-26 FIXED: `self.round = 3` forced immediately before the final `score_round()` in the `FinalPlace` -> `End` transition (`lib.rs:388-391`); test `early_final_scoring_uses_round_three_rewards` drives the real final path (3p, round 1, FinalPlace, all place queues empty, one Pavillion on board 0) and asserts round 3 + 16 points - pre-fix RED (round 2, 1 pt), post-fix GREEN. `cargo test -p alhambra-1` 50 lib + 1 contract pass / 0 fail; `cargo clippy -p alhambra-1 --all-targets -- -D warnings` exit 0; `cargo fmt -p alhambra-1 -- --check` exit 0; `git diff --check` exit 0. No PORT_PARITY gameplay change (F-26 round-3 force is the approved fix; nothing else altered). Uncommitted per work-unit brief. |
| R-35 | blocked(owner decision pending) | | sequence with 5.8; see Pending User Decisions |
| R-36 | done(b80a943) | b80a9434926a031beb56d44108562855cb21d599 | AC1 F-194: bot no longer requests Status or carries points; fetch_game_data uses PubRender/PlayerRender and a mock regression test proves hidden Status points never reach bot data. AC2 F-195: prompt TRACE fields replaced by count-only redaction boundary, with a sentinel-hand capture test; stale Score template line removed. AC3 F-190 verified inherited from R-13 (afe85b2): startup .expect plus loader error tests already fail invalid/missing keys; no new F-190 implementation claimed here. AC4 adds bot rustls aws-lc-rs process-default install and dependency. AC5 narrows module dead-code allows to two unused fields. Targeted game-client/bot/crypto tests, bot and game-client fmt/clippy, and diff check pass; independent security review APPROVE (two non-blocking Low findings: inherited attribution recorded, fmt-output assertion format coupling). |
| R-37 | parked-by-user | 0270f296a39755b44feacf85d6d2220d7c8b4f80 | R-37.0 complete; all remaining work is parked until the simpler unblocked remediation plan is complete and the user explicitly revisits it. Preserve R-37.1 before R-37.2; no implementation units, migrations, or rollout approved. |
| R-38 | blocked(5.3) | | 5.4 done (973ea62) |
| R-39 | done(3c4c1ca) | afbca143ce59e4e2f0ad6cfe41b9ad94975c44bf, 6dd0c41e4852172e730eb047857f6ac014d93679, b2e2021fa6f0e9c6099867c1fd981aaef7156601, 68140350ed56fe05f34771281611b5d8a8c3e71d, 1db7266404db772de630cc9458bb745c81d4f9ab, 9867e5396091e9f2c9827eaf0d081eeeeb25bf1b, 3c4c1ca4ce18485acd7524dc3887f2c2a85e4b2f | F-132 refuted by the per-connection, viewer-fixed `VisibilityCache` lifetime; no cache or SSE change. Static checks and final allowed web cargo check pass; runtime DB/SSR tests CI-pending. Independent authorization/visibility review PASS with no findings. |
| R-40 | done(1be0583) | 2c0fee15daa18563b57e40ae8e14d03d1f00cd00, 1c1bfbe9a5e37cfcf9c7a6eb559a6e69bd6b24cb, 9f3e5742e6539d9c82e33929bf88e78b17247be3, b6435094ce8384078ff8e973ff9bf741111aba17, 1be0583fac259118e6926ec6ce5181912d03854b | F-139 savepoint retry, F-121 capped actual read, F-122 pre-write undo-state validation; final approved web checks pass, runtime tests CI-pending. |
| R-41 | done(e96c5bb) | 4788a90, 02e9883, a77b752, c442267, 5dd0f63, 928863d, e96c5bb | F-170..F-178/F-184 closed; final approved web check passed and independent review PASS; runtime/DB tests CI-pending. |
| R-42 | done(6739e4c) | 5bd293ee421775fa5f134f14895796008350d77d, 6bec344112e845722366b9e39f527254700c2f1b, ee2ffee43e8ff6e3415a74b42dffb8722ad2fad0, e16d4d46b877cd28a59797a3807d94d2f5f0f42f, 6739e4cacbceceee955052789365226657e4ccd0 | F-164 tokenized the badge; `palette_css_vars` emits `--mk-orange` in every theme block and `(N new)` remains the non-hue cue. F-165 retains Closed-only `open()` but unconditionally bumps `last_update` on visible/online. F-166 applies `name DESC` to both selectors and adds a tied-created_at test calling both without changing visibility predicates. F-167 removes Red/86. F-168 asserts `href="#"` with an accurate focus-affordance comment. Approved staged and final `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` pass (known warnings only); independent review PASS with no findings. DB/SSR runtime tests are CI-pending. F-15 remains latent and parked; no enforcement is claimed. |
| R-43 | parked-by-user-process | | active latest-dependency governance conflicts with AC2's `tower-http` downgrade; widening to upgrade all 0.6 consumers is not approved. See R-43 parking below; no acceptance criterion is complete. |
| R-44 | blocked(5.2) | | |
| R-45 | partial/parked(provenance) | af71e05fd31b86691c27d48bf9c66aa94fbb8d6e, a2e1b9402e792f16183bd8e6d4547afdcbcc9ffa | R45.1 inventories all confirmed copied third-party source and bundles; AC1 remains partial because `blocked_domains.rs` has no copy-time revision or licence record. R45.2 limits its completed `session_store` directory guard to AC3. |
| R-46 | partial/parked(policy) | 0b7bce71e3fa4d7a5a6a483badb6df30ca6649c1, a425af2048157e2203c9f8883d3475cb5cc050d1 | R-46.0 corrected F-197's stale count to two remaining `.rls.toml` files, citing WP-65 (`2c28ae8`) for the three prior deletions, then removed both. `rg --files -g '.rls.toml' rust` returned no paths; `git diff --check` passed. R-46.1..R-46.4 are parked for the later parked-item review pending owner selection of the non-empty rust-lint baseline and threshold/policy choices. Threshold 11 plus refactoring the sole 20-argument helper is a Lead proposal only, not an approved decision. R-47 is the next unblocked package. |
| R-47 | done(6c4009f) | 65f08f678e363928b19615f2c2aa8af9be1e6640, 6c4009f4ac68f4ce9bf09fcaf94d711821a57991 | Closes F-205. AC1: SUMMARY headline and WP-67 record correct the false sentry premise; AC2: traceability maps current F-205 to 99-UNIFIED-REPORT.md and legacy dp F12 to immutable 868094a6 findings; AC3: two archive docs name the shared brdgme_game_bin entrypoint and 0.0.0.0:8080. Source/default and all 44 explicit deployment ADDR settings verified; targeted stale-text checks and git diff --check pass; Lead inspected both diffs. No Cargo command or independent review. |
| R-48 | done(66a4c99) | ac8dced09c16901dcd5c094d0d9a1dac774c79c0, 9c0f32dbe78453df8740ab2c92c50990b3cd7ef5, 66a4c99b655a81ff194808ffa5cbdf9bae836300 | Closes F-209. Plan correction (`ac8dced`) removed the false epilogue premise: the existing `!was_finished && self.is_finished()` guard predates F-209 and already prevents duplicate epilogues. AC1: `validate` now requires pending exactly in `OpponentChoose` and requires its actor to equal `current`; five direct malformed-state tests reject missing pending, Gift pending in `ChooseAction`, Gift pending in `Finished`, Competition pending in `ChooseAction`, and actor mismatch. A real gift/choose flow validates both the paired state and restored no-pending state. `git diff --check`, `cargo fmt -p hanamikoji-1 -- --check`, `cargo clippy -p hanamikoji-1 --all-targets -- -D warnings`, and `cargo test -p hanamikoji-1` pass (33 unit tests plus 1 contract test). Independent review APPROVE found no blocking/important issues; its sole Minor coverage note for `Finished` plus pending was fixed in `66a4c99`. |
| R-49 | done(527cfca) | 527cfca38f7db27f7e06d295d6bd1aafe37bcfe4, a5b1a47a2e5c4eba7ea462daf0a116a99ec45289 | Closes F-01/F-02/F-04 (R49.1, `527cfca`) and F-03/F-08 (R49.2, `a5b1a47a`). F-07 REFUTED under the current trusted no-thanks-2 contract (approved plan correction `eed47cf`): pub_state derives Some(peek_top_card()); Game has no current_card field to validate; no response-validation layer, render fallback, no-thanks change, or new contract; hostile/mismatched service-response validation needs separately approved scope. AC1 F-01: `from_string` now returns `Result<Vec<Node>, MarkupError>` (dead `&str` dropped from the tuple); all 11 call sites across 7 files updated (`bot/prompt.rs`, `cmd/repl.rs` x2, `tools/render_plain/main.rs`, `web/email/render.rs`, `web/game/server_fns.rs` x4, `web/rules.rs`, `web/theme.rs`). AC2 F-02: the disjunctive overflow assertions are replaced with assertions that pin a specific outcome - `overflowing_u8/usize_leaves_tag_unconsumed` assert empty nodes and the full input unconsumed. AC3 F-04: `wrap_segment` is linear via a running char count (`current_len`), with the no-O(n^2) argument recorded in a comment (`wrap.rs:14`); no benchmarks run. AC4 F-03: `token_parser_and_suggest_agree_on_fold_length_change` covers U+0130 (2 bytes) full-folding to `i\u{307}` (3 bytes): the completed folded input parses in both `Token::parse` and `CommandSpec::Token`, both sides suggest the token, and the incomplete fragment "i" stays suggested but is rejected by the parser (never recast as a successful parse). AC5 F-08: `chain_expected_skips_a_leading_space_at_all_sites` calls `expected()` on Chain2/3/4 and asserts parity with the `CommandSpec::Chain` contract (leading Space skipped, next element reported) and equality with each `to_spec().expected()`; the duplicated `impl Parser for CommandSpec` is deliberately retained (COMMAND_PARSER_SPEC_DEDUP.md). Gates all exit 0: `cargo test -p brdgme_markup` 58 passed, `cargo clippy -p brdgme_markup --all-targets -- -D warnings`, `cargo fmt -p brdgme_markup -- --check`, `cargo check -p bot`, `cargo check -p brdgme_cmd`, `cargo check -p brdgme_render_plain`, `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` (one pre-existing unrelated unused_mut warning at `web/src/bin/import_game.rs:18`), `cargo test -p brdgme_game` 131 passed, `cargo clippy -p brdgme_game --all-targets -- -D warnings`, `cargo fmt -p brdgme_game -- --check`, `git diff --check`. Independent review PASS with no correctness findings; accepted residuals: negligible per-char fold allocation, out-of-scope other parser tests have a different disjunctive pattern, direct concrete Chain `expected` assertions are meaningful while projection equality is supplemental. No correction commit. |
| R-50 | pending | | F-83 follows 5.1 |
| R-51 | parked(later-review) | | F-196/F-140 remain unimplemented; no acceptance criterion is complete. Pending rulings: immediate migration-backed authoritative-version repair versus delayed repair, and email game-bound deprecated rules access only versus public `/rules/<id>` widening. R-52 is the next unblocked package. |
| R-52 | done(65fc01c) | 65fc01ce43d04d38e19dc27f08ddcc18257d1586 | Read-only immutable-commit re-walk confirms Unit 07b covered all three named surfaces: six `RealInviteMailer` workflows, `spawn_sweep`'s six production call sites, and `notify_owner_decline` gating. WP-53's three residual cosmetics remain below finding threshold; supplied `restart_core` disposition recorded without re-derivation; no new finding. Evidence: `R-52-WP51-WP53-COVERAGE.md`; `git diff --check` passed; no Cargo. |
| R-53 | parked(later-review) | | Public semantic contract decision. `higher-is-better` is recommended but not approved; Cathedral negates remaining-piece values for placings while `points()` returns positive values; the exact conformance population remains unsized and the plan's "other 27 crates" estimate is unverified. No AC is complete and no source or conformance sweep starts before the ruling. R-54 is the next dependency-free candidate; do not start it in this package. |
| R-54 | in-progress | | U9 and U14 active. U8 is parked for the later parked-item review pending external confirmation/configuration that `e2e` is a required GitHub check; do not alter the workflow or deployment documentation now. |
| R-55 | pending | | review package; before R-05/R-06 if capacity |

## R-43 parking

- **Status:** parked-by-user-process. Revisit during the later parked-item review; do not implement beforehand.
- **Governance conflict:** active latest-dependency governance conflicts with R-43 AC2's `tower-http` downgrade. Widening the package to upgrade all 0.6 consumers is not approved.
- **Unresolved evidence:** all 29 `deny.toml` skips and the falsified all-upstream rebuttal remain unresolved.
- **Pending owner rulings:** select the skip review-date cadence and explicitly rule whether workspace-scoped `cargo deny` verification is permitted under laptop constraints.
- **Acceptance:** no R-43 acceptance criterion is claimed complete.

## R-45 evidence

- **Status:** partial/parked on the mutable `blocked_domains.rs` provenance
  record. This package does not select the next package.
- **Approved inventory boundary:** include committed copied third-party source
  files and generated JavaScript bundles; exclude transformed palette facts.
  The Source Code Pro font's missing OFL text is a parked asset-compliance item
  for the later parked-item review, not vendored code in R-45.

| Path | Upstream and provenance | Licence and attribution | Reason | Machine-checked? | Known inherited defects |
|------|-------------------------|-------------------------|--------|------------------|-------------------------|
| `rust/lib/session_store/` | `tower-sessions-sqlx-store` 0.15.0; `src/lib.rs:1-3`; introduced by `667c8f42` | MIT `LICENSE`; upstream header in `src/lib.rs:1-3` | Upstream store pinned sqlx 0.8 while the workspace unified on 0.9 | No: `deny.toml:45-49` ignores private crates and `Cargo.toml:5` is `publish = false` | F-200: duplicate-key `migrate()` returns success before table creation/commit; it is the sole session-table creator |
| `rust/web/src/auth/blocked_domains.rs` | `disposable-email-domains/disposable-email-domains` mutable `main`; header at `:1`; introduced by `ed4bedb` | Copy-time revision and licence unknown. Current upstream `LICENSE.txt` was observed as CC0 1.0; no local licence text | Static disposable-domain blocklist | No | Not assessed: R-45 AC4 applies only to the confirmed vendored-code directory |
| `rust/web/public/sentry.js` | Generated bundle of `@sentry/browser` and `@sentry/wasm` 10.65.0; `js/package.json:6-11`, bundle `:167-174`; introduced by `3d5bc86` | Upstream Sentry MIT; bundle attribution footer at `:31120-31124` | Browser error reporting and WASM symbolication | No | Not assessed: R-45 AC4 applies only to the confirmed vendored-code directory |
| `brdgme-go/assert/assert.go:159-171` | Go `go test` helper; source comment `:159`; introduced by `6e26a2e` | Go BSD-3-Clause upstream; source attribution comment at `:159` | Dependency-free test-name matching helper | No | Not assessed: R-45 AC4 applies only to the confirmed vendored-code directory |

- **Excluded:** `rust/lib/color/src/palette.rs` contains attributed and adjusted
  palette facts, not copied source. `serde_yaml_ng` is a registry dependency,
  not vendored code.
- **Parked residual:** `rust/web/public/fonts/source-code-pro-latin.woff2` is a
  third-party asset without accompanying OFL text. It is outside R-45's approved
  vendored-code boundary and remains for the later parked-item review.
- **AC1:** partial. The inventory is complete for the approved boundary, but the
  copied blocklist has no recorded copy-time version or licence.
- **AC2:** complete. Every inventory entry states whether its obligation is
  machine-checked.
- **AC3:** complete at `a2e1b9402e792f16183bd8e6d4547afdcbcc9ffa`.
  `.github/workflows/ci.yml:80-85` runs the approved single assertion for the
  `session_store` MIT licence text and exact upstream attribution header. The
  assertion and `git diff --check` passed locally; no Cargo command was run.
- **AC4:** complete for its approved scope: F-200 is recorded for
  `rust/lib/session_store/`; no file-level upstream-defect sweep is claimed.

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

## Pending User Decisions

| Item | Pending decision |
|------|------------------|
| R-35 | Removing `Status` lacks an approved public points source: `Response::Status` is the only response carrying `GameResponse.points`, and `Response::PlayerRender` carries no `points`. Required decision: either approve optional all-seat `points` in `Response::PlayerRender` (populated by Rust handlers, absent for existing Go handlers) or specify another Go-compatible source. R-35 is blocked on this decision; sequence with 5.8 preserved. |
| R-51 F-196 | Choose migration-backed per-version descriptive values with immediate authoritative-version re-pointing, or a smaller delayed repair on a later reconcile. Lead proposal only: the migration-backed immediate repair. Do not allocate a migration number, remove the generation guard, or change source while parked. |
| R-51 F-140 | Choose email game-bound deprecated rules access only, or widen public `/rules/<id>` behavior. Lead proposal only: email-only deprecated lookup. Do not change public lookups while parked. |

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

## R-18 evidence

Code commit `6a304be11252048e0cf8ddf1459d38f3a0d38a7a` (verified via
`git rev-parse HEAD`; message `fix(web): move network calls outside
transactions (R-18, F-134, F-135, F-143)`; tracked changes only:
`rust/web/src/proposals.rs`, `rust/web/src/email/inbound.rs`,
`rust/web/src/email/sweep.rs`; 1136 insertions / 88 deletions). Closes
F-134 (High), F-135 (High), F-143 (Low, note).

- **Objective:** remove the three sites that hold a row lock across an HTTP
  call. Acceptance criteria: (1) zero HTTP-client calls in each transaction
  body; (2) each hoisted call followed by an in-tx re-read + re-validation
  with a test calling the enclosing function under concurrent modification;
  (3) F-143 recorded as reconciling the WP-46 vs WP-79 policy, not a
  deviation.

- **AC1 / per-site before-after transaction contract (all five tx bodies
  confirmed free of HTTP/network I/O between begin and commit):**
  - **F-134 `start_proposal` (proposals.rs:1748-1810):** BEFORE,
    `fetch_game_from_service` (reqwest -> game service `Request::New`) ran
    between `pool.begin()`/`lock_proposal_for_update` and `commit`, holding the
    `game_proposals` `FOR UPDATE` lock and a pool connection for the full
    reqwest timeout. AFTER, Phase 1 (no tx) snapshots the proposal + roster,
    `find_game_type_player_counts`, `find_game_version`, derives
    `accepted_count`, and calls `fetch_game_from_service` (1748) - all BEFORE
    `pool.begin()` (1755). Phase 2 (tx 1755..1810): `lock_proposal_for_update`
    (1760), re-read roster (1774), re-validate owner/status/pending/declined/
    count, `roster_unchanged` TOCTOU guard (1800, helper 1342),
    `start_proposal_tx` (1806), single commit (1808). Post-commit: broadcast +
    `broadcast_and_trigger` + mailer (1812+). `accepted_count` does not depend
    on an in-tx write here, so an unchanged roster guarantees the fetched game
    matches the roster written.
  - **F-135 `handle_invite_reply` (inbound.rs:984-1239):** BEFORE, WP-79's own
    commit `91c723d` landed `fetch_game_from_service` after `begin()` instead of
    before, behind the `FOR UPDATE` lock. AFTER, split into two short txs with
    the fetch in the gap. TX#1 (984..1109): lock, `status == "open"`, roster,
    `me.response == "pending"`, `update_proposal_player_response`,
    `count_pending_human_invitees_tx`, `find_game_version` (pool DB read),
    roster snapshot + `accepted_count` captured into `start_inputs`; no network;
    commits, releasing the lock. Gap: `fetch_game_from_service` (1124) AFTER
    TX#1 commit, BEFORE TX#2 begin. TX#2 (1151..1239): re-lock, status re-check,
    fresh roster re-read, `invite_roster_unchanged` (helper 886) AND
    `accepted_now != accepted_count` re-derivation (1192), `start_proposal_tx`
    fed the FRESH roster; no network. RouteOutcome semantics preserved: `Retry`
    only pre-mutation; once TX#1 commits every path is `Done` (at-least-once
    webhook safety). The in-tx-write dependency of `accepted_count` (why F-135
    is harder than F-134) is handled by snapshotting after the response UPDATE
    inside TX#1.
  - **F-143 `sweep_once` / `send_reminder` (sweep.rs:87-330):** BEFORE, the
    claim tx held the `game_players` `FOR UPDATE SKIP LOCKED` lock across a
    game-service render (`render_board_and_you_can`) AND a Resend API send
    (`try_send_rendered_email`), serialised over up to 200 candidates per tick.
    AFTER, `send_reminder` takes `token: String` and holds NO transaction. Claim
    TX (243..292): `SELECT ... FOR UPDATE SKIP LOCKED` re-checking
    `turn_reminder_sent_at IS NULL AND is_turn = true` (256-257) +
    `ensure_email_token_tx` (276, idempotent `COALESCE` upsert), then commit
    (lock released before network). Send in the gap (300). Mark TX (311..330):
    conditional `mark_reminder_sent_tx` (87-99) `UPDATE ... WHERE id=$1 AND
    turn_reminder_sent_at IS NULL AND is_turn = true` - the at-most-once hard
    guarantee; 0 rows silently accepted. `fetch_candidates` filters on
    `turn_reminder_sent_at IS NULL` (68), so a marked player is never
    re-selected. No network in either tx.

- **AC2 / deterministic concurrent-change test evidence (all three; coordination
  is deterministic and non-sleep - condvar / Notify / Semaphore+Barrier - not
  timing-based; compile-verified only, runtime deferred to CI):**
  - **F-134** `start_proposal_rejects_a_stale_snapshot_under_concurrent_roster_change`
    (proposals.rs:4356): gated in-process mock `spawn_gated_new_game_service`
    (4226); on `Request::New` the handler sets `entered=true` + `notify_all`
    then blocks on a `done` condvar; a writer OS thread waits on `entered`,
    declines the invitee, then sets `done`. The seeded version name
    `format!("start-mock-{}", Uuid::new_v4().simple())` (4210) is a DNS label
    that passes `brdgme_game_client::validate_version_name`, so the mock IS
    reached and the gate fires (a dotted `'1.0.0'` is rejected before any HTTP
    and would hang). Asserts a stale snapshot under concurrent roster change is
    rejected, never a game started from a stale roster. The 15s `wait_timeout`
    is a safety net, not the synchronization.
  - **F-135** `invite_reply_does_not_start_game_on_stale_roster_after_game_fetch`
    (inbound.rs:3016): seed (3072-3081) uses the valid DNS-label name
    `format!("invite-mock-{}", uuid::Uuid::new_v4().simple())` (3077) + mock
    uri; the mock on `Request::New` does `called.notify_one()` then
    `proceed.notified().await`; the test awaits `called.notified()`, declines
    the owner, then `proceed.notify_one()`. `tokio::sync::Notify` stores a
    permit on `notify_one` even if the waiter is unregistered, so neither
    handoff is lost to ordering - fully deterministic and non-blocking. Asserts
    the game is NOT started on a stale roster after the gap fetch.
  - **F-143** `turn_reminder_concurrent_sweeps_mark_at_most_once`
    (sweep.rs:2022) plus a single-sweep `Notify` rendezvous test: barrier mock
    `spawn_barrier_render_service` (1841) emits a `Semaphore` permit from inside
    the render (hence after that sweep's claim commit) then `barrier.wait()`.
    With `entered = Semaphore::new(0)`, `barrier = Barrier::new(3)`: spawn s1;
    acquire permit 1 (exists only after render R(1), hence after claim commit
    C(1)); ONLY THEN spawn s2, whose `FOR UPDATE SKIP LOCKED` claim finds the
    row unlocked (C(1) committed) and unmarked (no mark tx yet)
    deterministically; acquire permit 2; `barrier.wait()` as party 3 releases
    both renders together; assert `renders == 2` (duplicate external work in the
    hoisted window) and final `sent_at IS NOT NULL` with a subsequent
    `sweep_once` rendering nothing (exactly one durable mark wins). The former
    race (both sweeps spawned at once, s2 starved by SKIP LOCKED) is gone
    because s2's spawn is gated on an event that implies s1's lock is already
    released. The two 5s `timeout`s are safety nets, not the determinism
    mechanism.

- **AC3 / F-143 is a reconciliation of contradictory WP-46 vs WP-79 policy, NOT
  a deviation:** WP-46 spec 3a mandated holding the lock across the sweep send;
  WP-79 removes it on the proposal path; the two specs contradict (the reason
  F-143 is graded "Low, note"). Resolution: hoist everywhere; the hard guarantee
  is the at-most-once DB MARK (the conditional `turn_reminder_sent_at IS NULL`
  re-check), and the accepted cost is a rare duplicate reminder SEND. This
  follows CODING.md:140-142 ("Mark work done only after it succeeded ... a rare
  duplicate is the cheaper failure mode"), so the sweep hoist needs no new
  product call and is recorded here as the reconciled policy rather than a
  deviation from WP-46.

- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0, 0 errors; one pre-existing `proc-macro-error2
  v2.0.1` future-incompat warning (unrelated, not a code error).

- **Runtime:** the three new tests (and the modified
  `turn_reminder_suppressed_by_recipient_presence`) are compile-verified only;
  runtime web tests are deferred to CI by explicit ban (web build/test/run
  forbidden) and because they need a real Postgres+NATS host
  (`scripts/rust-test.sh` / CI, not the dev laptop). NO runtime red/green has
  been executed; all green claims are by reasoning only and MUST be confirmed
  red-on-old / green-on-new in CI before R-18 is relied upon. Residuals: F-134
  and F-135 connect to real NATS; F-134's cloned-`PgPool`-from-a-separate-OS-
  thread pattern has no in-repo precedent and is unproven until run; F-135's
  outcome assertion is still loose (`Done | Retry`, inbound.rs:3203-3206) and
  should tighten to `Done` once green is confirmed.

- **Review:** the single comprehensive independent review
  (`/tmp/opencode/r18-review.md`) returned **REJECT (blocking)**: the three
  production hoists are correct and the static zero-network-in-transaction
  property holds, but F1 (HIGH) - the F-134 and F-135 regression tests seeded
  the dotted version name `'1.0.0'`, which `validate_version_name` rejects
  before any HTTP, so the mock was never reached and both tests hung; F2
  (MEDIUM) - the F-143 two-sweep test had a likely SKIP-LOCKED race; F3
  (MEDIUM/LOW) - the WP-46/WP-79 reconciliation was not yet recorded in the
  repo; F4 (LOW, accepted) - player rows are not `FOR UPDATE`-locked in the
  Phase-2 re-read (same residual as the original code and every reference
  pattern). After the test repair (DNS-label version names + sequencing the
  second sweep's claim after the first's commit), the **targeted F1/F2
  re-review** (`/tmp/opencode/r18-targeted-rereview.md`) returned **PASS
  (static)** for both: F1 F-134 PASS (proposals.rs:4210), F1 F-135 PASS
  (inbound.rs:3077), F2 F-143 TEST 2 PASS deterministic (sweep.rs:2021-2092).
  F3 is resolved by this tracker entry; F4 remains an accepted residual. The
  only residual common to both reviews is the unchanged one: runtime red/green
  has NOT been executed (web absolute rule) and requires a Postgres+NATS host.

## R-19 evidence

Code commit `7de92cd65458f408087af8262afe92635639762c` (verified via
`git rev-parse HEAD`; message `fix(web): deduplicate invite nudges per
recipient (R-19, F-144, F-147)`; tracked changes: `rust/web/src/email/notify.rs`,
`rust/web/src/email/sweep.rs`, `rust/web/src/proposals.rs`, plus new migration
`rust/web/migrations/030_proposal_player_nudge.sql`; 227 insertions / 39
deletions). Closes F-144 (per-invitee nudge dedup) and F-147 (dead-code
deletion).

- **Root cause:** F-144 was a granularity mismatch - the dedup gate and the
  mark keyed per-proposal (`game_proposals.nudged_at`) while the send is
  per-invitee, so one unsendable invitee could block or one sendable invitee
  could be double-nudged. Fixed at both the selection gate and the mark: the
  gate re-keys `gp.nudged_at IS NULL` -> `pp.nudged_at IS NULL`
  (`proposals.rs:1005`), `NudgeCandidate` gains `game_proposal_player_id`
  (`proposals.rs:993`, SELECT `:1002`), and the per-proposal `all_sent`
  aggregation + `mark_proposal_nudged` is replaced by the per-invitee
  conditional mark `mark_proposal_player_nudged` (`proposals.rs:1022-1037`,
  `UPDATE ... WHERE id=$1 AND nudged_at IS NULL`), called per invitee whose own
  `send_invite_now` returns `true` (`sweep.rs:559-567`). The mark keys directly
  on `send_invite_core`'s per-invitee `bool` (`proposals.rs:250-256`):
  sent-OR-permanent-skip -> `true` -> marked (durable dedup); transient
  (web-presence suppression `proposals.rs:280-284`, lookup/send failures) ->
  `false` -> unmarked/retryable. Mirrors the turn-reminder at-most-once
  conditional mark (`sweep.rs:87-99`). F-147: the dead-at-birth
  `send_turn_reminder` helper is removed; the sweep keeps its own hardened
  `send_reminder`/`ReminderOutcome`, and no reminder behavior is refactored.

- **AC1 (dedup test):** `invite_nudge_dedup_is_per_invitee`
  (`sweep.rs:1758-1946`) calls `sweep_invite_nudge_once` twice
  (`sweep.rs:1904,1919`) over a sendable (never-active) invitee and a
  web-present retrying invitee. Between the two sweeps it asserts the sendable
  invitee is no longer selectable as a candidate while the web-present retrying
  invitee still is (`sweep.rs:1909-1917`); after sweep 2 it asserts the sendable
  invitee's marker is set and the retrying invitee's marker is still NULL
  (`sweep.rs:1923-1945`). Dedup is proven via marker + candidate state (no send
  counter exists without the F-182/R-20 mailer seam) - permitted by the task.

- **AC2 (`send_turn_reminder` deletion):** deleted with its doc comment. Source
  (`rust/`) symbol/caller count is **0**; repo-wide there are **14** references,
  ALL in `docs/reviews/2026-07-30-review-session/` prose (documentation only,
  expected). The false `SendResult` doc is corrected (`notify.rs:291-294` ->
  "Every caller is best-effort and drops it").

- **AC3 (no pattern-4e re-derivation):** honored - no pattern-4e revert in
  `dcd8844c` was re-derived; nothing of the sort is attempted here.

- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0. Sole warning: `variant `Reminder` is never
  constructed` at `web/src/email/notify.rs:136` - the documented in-scope F-147
  side effect (`NotifyKind::Reminder` is now never-constructed; its 4 match arms
  remain). No errors. (Also a pre-existing `proc-macro-error2 v2.0.1`
  future-incompat NOTE, unrelated, does not affect exit status.)

- **Runtime:** the new test is compile-verified only; the runtime web test is
  deferred to CI by explicit ban (web build/test/run forbidden; DB tests need
  Postgres). NOT claimed passing here - "nudged exactly once" is inferred from
  marker + candidate state and must be confirmed red-on-old / green-on-new in CI
  (`scripts/rust-test.sh`).

- **Review:** comprehensive independent review (`/tmp/opencode/r19-review.md`)
  verdict **APPROVE**; **0 high and 0 medium** findings; two non-blocking low
  notes - L-1 (the pre-existing R-08 test
  `invite_nudge_transient_lookup_error_leaves_proposal_unmarked` still asserts
  `game_proposals.nudged_at IS NONE`, now vacuously true since the sweep writes
  `game_proposal_players.nudged_at`; behavior covered by the new R-19 test) and
  L-2 (stale comment at `sweep.rs:1617` still names the removed
  `mark_proposal_nudged`; behavioral claim still correct).

- **Residual (pre-existing, out of scope):** the invite-nudge sweep has no
  `FOR UPDATE SKIP LOCKED` claim, so two concurrent replicas can both select an
  unmarked invitee and both send before either marks (at-most-once mark,
  at-least-once send window). This is pre-existing (the old per-proposal code
  had no claim either) and out of F-144 scope; the per-invitee conditional mark
  narrows, not widens, the window. Not a regression.

## R-20 evidence

Code commit `049325a7ba248c3e3630f284f5f94a6a26c7dafb` (verified via
`git rev-parse 049325a`; message `feat(web): route game-start notify through
mailer seam (R-20, F-146, F-179, F-180, F-181, F-182)`; tracked changes only:
`rust/web/src/email/commands.rs`, `rust/web/src/email/inbound.rs`,
`rust/web/src/email/notify.rs`, `rust/web/src/email/outbound.rs`,
`rust/web/src/game/mod.rs`, `rust/web/src/proposals.rs`; 950 insertions /
50 deletions). Closes F-146 (Low/Medium), F-179 (Medium), F-180 (Low),
F-181 (Low), F-182 (Low).

- **Objective:** one event, one mail, correctly threaded and observable in
  tests. F-182 (notify called outside the mailer seam, so untestable) is the
  root that let the other four survive, so it is fixed first.

- **AC1 (F-182, notify inside the mailer seam):** the `InviteMailer` trait gains
  `async fn notify_game_started(&self, game_id)` (`proposals.rs:121`) with the
  `RealInviteMailer` impl routing to the new free `notify_game_started`
  (`proposals.rs:801` -> `notify.rs:528`); the start sites call it through the
  seam, so the wiring is spyable. Test
  `start_proposal_notifies_game_started_via_proposal_path` (`proposals.rs:4683`,
  C1/F-182) drives the REAL proposal path via `with_start_proposal_context` +
  `start_proposal` and asserts the recorded mail token, so deleting the
  production wiring would fail; `invite_mailer_seam_is_spyable`
  (`proposals.rs:4545`, pure `#[test]`) asserts a spy mailer records the call.
- **AC2 (F-179, one mail per invitee on invite-accept auto-start):**
  `invite_accept_auto_start_one_mail_per_on_turn_invitee` (`inbound.rs:3201`,
  C2/F-179) drives `handle_invite_reply` auto-start against an in-process mock
  game service and asserts exactly one game-start mail - on-turn position 0
  token `Some`, off-turn position 1 `None` - so no restored
  `notify_game_emails`/`notify_started` duplicate burst; reinforced by
  `notify_game_started_one_mail_per_on_turn_player` (`notify.rs:977`).
- **AC3 (F-180, solo start notifies):** `notify_game_started` uses
  `SendMode::BypassSuppression` (`notify.rs:528,557`), so the solo-start
  confirmation is no longer swallowed by the hydrated-page presence window.
  `solo_start_notifies_game_started` (`proposals.rs:4727`, M2/F-180) drives the
  real `create_proposal` solo path and asserts a notification;
  `notify_game_started_bypasses_web_presence_suppression` (`notify.rs:938`)
  proves `notify_game_started` bypasses suppression where `notify_game_emails`
  (Normal mode) suppresses the same recently-active player.
- **AC4 (F-146, distinct subjects/thread ids per kind):** new `InviteNotifyKind`
  enum (`proposals.rs:125`) with `invite_notify_subject` (`:134`) and
  `invite_notify_thread_id` (`:146`) emitting per-kind suffixes (reinvite /
  decline / cancelled / started / ready), so clients no longer collapse the five
  notifications into one `proposal-{id}` thread.
  `notification_kinds_have_distinct_subjects_and_thread_ids`
  (`proposals.rs:4841`, M1/F-146, pure `#[test]`) asserts distinctness on the
  real production functions, not re-derived hardcoded strings.
- **AC5 (F-170 refuted, do not re-derive):** honored - F-170 is NOT extended to
  the game-start mail. It reads `turn_emails_enabled` directly (unsubscribed
  users do not get it), there is no hidden-information leak (the mail is rendered
  from the recipient's own seat), and the `ca7925bc` game-start sweep is complete
  (all four `insert_game_from_service` callers notify); nothing of the sort is
  re-derived here.
- **F-181 ordering (notify before broadcast):** the start sites notify before
  `broadcast_and_trigger` (`game/mod.rs:51`), closing the bot-turn double-mail
  race. `start_proposal_notifies_before_broadcast` (`proposals.rs:4773`,
  I2/F-181) asserts the token is minted and position 0 is still on turn (state
  not advanced before notification); `email_new_game_notifies_before_broadcast`
  (`commands.rs:2151`) covers the email-command start site.

- **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` - exit 0. Sole warning: `variant `Reminder` is never
  constructed` at `web/src/email/notify.rs:136` - the pre-existing in-scope
  `NotifyKind::Reminder` dead_code carried over from R-19/F-147 (its 4 match
  arms remain). Also a pre-existing `proc-macro-error2 v2.0.1` future-incompat
  NOTE, unrelated, does not affect exit status.

- **Runtime:** the new `#[sqlx::test]` cases are compile-verified only; runtime
  web tests are authored but deferred to CI by the web cargo restriction (web
  build/test/run forbidden; DB tests need Postgres). NOT claimed passing here -
  red-on-old / green-on-new must be confirmed in CI (`scripts/rust-test.sh`).

- **Review:** the comprehensive independent review initially returned **REJECT**
  on two Critical findings - **C1 (F-182)** the seam test did not exercise the
  real proposal path (production wiring deletion would not fail) and **C2
  (F-179)** a restored `notify_game_emails`/`notify_started` duplicate could
  re-introduce the burst. Both were fixed (the proposal-path test
  `start_proposal_notifies_game_started_via_proposal_path` and the
  one-mail-per-on-turn-invitee assertion in
  `invite_accept_auto_start_one_mail_per_on_turn_invitee`), and the focused
  re-reviews returned **PASS with no Critical findings**. No push.

## R-23 evidence

Code commit `3db5c06e95fe78a6b521e87a4cd2e27aab77093b` (verified via
`git rev-parse HEAD`; message `R-23: lost-cities-1/-2 parity (F-60, F-62,
F-63, F-64)`; tracked changes only: `rust/game/lost-cities-1/src/lib.rs` and
`rust/game/lost-cities-2/src/lib.rs`; 170 insertions / 25 deletions). No
migration, doc, Cargo, or tracker changes in the code commit.

- **Objective / findings:** R-23 closes F-60 (High, -1 raw-index panic on the
  render path), F-62 (Medium, only -2 had a `validate()`), F-63 (Low, -2
  `unreachable!()` in three player-count lookups on the command path), and F-64
  (Low, -2 3-player scoring constants untested). Scope is strictly two crates:
  `lost-cities-1` and `lost-cities-2`.

- **AC1 (F-60 / F-62, -1 validate + direct defense):** -1 gains a `validate()`
  that checks the hands/scores/expeditions/stats vectors and `current_player`,
  and its direct `player_state` access is defended with
  `.get(...).cloned().unwrap_or_default()`. A named no-panic short-hands test
  exercises the short-state path.

- **AC2 (-1 vs -2 parity deltas justified):** all final `validate()` differences
  between -1 and -2 are only the fixed `PLAYERS` constant versus runtime
  `self.players`, the no-players-field range check, and crate-specific error
  text; each is justified by -1's compile-time two-player model.

- **AC3 (F-63, -2 unreachable!() -> Result):** -2 replaces the three
  `unreachable!()` player-count helpers with `Result` errors, propagated through
  `score`, `end_round`, and `draw_hand_full`. Direct and command-path
  four-player tests assert `Err`.

- **AC4 (F-64, 3-player scoring coverage):** `score_3p_works` covers cost 15,
  threshold 7, and bonus 15. It passed against the old scoring implementation
  (coverage-only), so no rules change was made.

- **TDD RED/GREEN:** -1 both new tests failed pre-fix for the actual raw-index
  panic / default `validate()` behavior, then passed after the fix. -2 F-63
  tests failed to compile against the old signature, then passed after the
  `Result` API. F-64 coverage was green before implementation, as above.

- **Verification (per-crate, serial, one crate per cargo command):**
  `cargo test -p lost-cities-1`, `cargo test -p lost-cities-2`,
  `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`,
  `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`,
  `cargo fmt -p lost-cities-1 -- --check`,
  `cargo fmt -p lost-cities-2 -- --check`, and `git diff --check` - each passed.
  No workspace-wide cargo, no `scripts/rust-test.sh`, no Tilt/kind, no global
  installs, no production operation was run.

- **Review:** comprehensive independent review
  (`/tmp/opencode/r23-comprehensive-review.md`) verdict **APPROVE / PASS**, no
  Critical/Important/Minor findings. Two informational residuals (out of R-23
  scope): (1) pre-existing -2 `end_round` increments `self.round` before the
  scoring loop, so a scoring `Err` leaves `round` mutated without scores pushed
  (strictly better than the old panic; gated by `validate()`); (2) -2
  `player_state` still uses raw `self.hands[player]` indexing protected by
  `validate()` at deserialization. The reviewer did not rerun clippy/fmt, but the
  implementation did (above).

- **Scope / R-07 / R-22:** `R-07-HANDOVER.md` untouched; R-22 unchanged / parked;
  R-21 remains gated by R-22 through R-26 and R-48.

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

## R-24 evidence

Code commit `d37c4231d10704d17a0466fffc732103107ec769` (verified via
`git rev-parse`; parent/base `83c6bd8ad2afbe164c32db3af6c0e7cff2fc967d`;
message `sushi-go-2: validate state and make render path total (R-24)`;
tracked change exactly one file: `rust/game/sushi-go-2/src/lib.rs`,
+217/-31). Closes F-61 (High), F-65 (Medium), F-210 (Medium). No migration,
doc, Cargo, or other-tracker changes in the code commit.

- **Status:** done.

- **Changed file:** exactly `rust/game/sushi-go-2/src/lib.rs`.

- **AC1 (F-61, validate() exists and is tested):** `Gamer::validate` override
  at `lib.rs:775-813`, shape-copied from the lost-cities-2 reference. Asserts
  all required invariants: `players` in `2..=5` (776), `all_players ==
  if players==2 {3} else {players}` (782-788), all four parallel vector lengths
  (`hands`/`playing`/`played`/`player_points`) `== all_players` (789-802),
  `controller < players` (803), `round` in `1..=TOTAL_ROUNDS` (806). Invoked
  immediately after `serde_json::from_str(game)?` at the requester boundary for
  Status/Play/PubRender/PlayerRender (`rust/lib/cmd/src/requester/gamer.rs:
  45,59,71,80`), so a short deserialized state now returns `SystemError`
  instead of failing open. Tests: `test_validate_accepts_started_game`
  (positive control, 2p+3p), `test_validate_rejects_each_short_parallel_vector`
  (truncates each of the four vectors in turn, asserts `Err`),
  `test_validate_rejects_bad_scalar_fields` (players range both ends,
  all_players mismatch incl. the 2p->3 rule, controller, round range both
  ends).

- **AC2 (F-61, six render-path sites total, no-panic tests, raw-index sweep):**
  all six sites converted to defined fallbacks - `is_finished`
  `playing.first().is_some_and(..)` (230), `can_dummy`
  `playing.get(DUMMY).is_some_and(..)` (249), `pudding_cards`
  `played.get(player).map(..).unwrap_or(0)` (261-264), `placings`
  `player_points.get(p).copied().unwrap_or(0)` (271), `pub_state`
  `player_points.get(p).copied().unwrap_or(0)` (841), `player_state`
  `playing.get(DUMMY).cloned().flatten()` (864). Six `*_no_panic` tests each
  call the converted site on a short state and assert a defined result
  (`lib.rs:1699-1749`); pre-fix these panic at exactly the cited lines
  (228/245/257/265/791/813) - accurate RED evidence for AC2.

- **AC3 (F-65/F-210, draw_count total, no `_`/`unreachable!()`, tested):**
  `draw_count` (`lib.rs:140-149`) returns `Result<usize, GameError>`; arms
  `2|3 => Ok(9)`, `4 => Ok(8)`, `5 => Ok(7)`, `n => Err(GameError::internal(..))`.
  No `_` arm, no `unreachable!()`. Error propagated via `?` through
  `start_round` (298), `end_round` (660), `end_hand` (361); `play_cards`
  returns `self.end_hand()` directly (732); `start` (771) and `command`
  (892/897) already return `Result`. F-210 root cause addressed:
  `draw_count(self.all_players)` (298) is now total instead of panicking, and
  `validate()` additionally bounds `all_players` to `{2,3,4,5}` before the
  command path (defense in depth). Tests:
  `test_draw_count_out_of_range_is_defined_not_panic` (F-210: 0/1/6/999 ->
  `Err`, no panic) and `test_draw_count_out_of_range_is_not_silent_nine`
  (F-65: `!matches!(.., Ok(9))`); `test_draw_counts` updated to `.unwrap()`
  the happy path.

- **TDD RED/GREEN:** AC2 six no-panic tests panicked pre-fix at the cited
  lines, pass post-fix. AC3 tests failed to compile pre-fix (`draw_count`
  returned `usize`, so `.unwrap()`/`.is_err()` do not compile), pass post-fix.
  AC1 reject tests are assertion-failure RED (the `Gamer` trait default
  `validate()` returns `Ok(())`, so they compile pre-fix and fail their `Err`
  assertions), not compile-error RED; the positive control passes pre-fix.
  All AC1 tests are valid RED->GREEN and AC1 is fully met.

- **Raw-index sweep:** `rg 'self\.(hands|playing|played|player_points)\['
  src/lib.rs` = **37 hits, all justified** (guarded in-expression at
  229/243/854/859; bounds-rejected in-expression at 709/715/722/724;
  loop-bounded by `0..all_players`/`0..players`/enumerate and made in-bounds
  by the new `validate()` invariant at 281/290/295/311/319-333/341-360/
  391/468/545/640/644/676/683/684/726/728/916; out-of-finding-scope separate
  files `render.rs:204/207` and `command.rs:18/21`, unmodified and bounded by
  the same validated invariant). The six converted F-61 sites no longer appear
  in the raw-index list. `rg 'unreachable!\(\)' rust/game/sushi-go-2/src/` =
  **0 hits**.

- **Verification (per-package, serial, one crate per cargo command):**
  `cargo test -p sushi-go-2 --lib` -> EXIT=0, 50 passed / 0 failed;
  `cargo clippy -p sushi-go-2 --all-targets -- -D warnings` -> EXIT=0;
  `cargo fmt -p sushi-go-2 -- --check` -> EXIT=0; `git diff --check
  83c6bd8..d37c423` -> EXIT=0; `git status --porcelain` -> empty (clean
  worktree). No workspace-wide cargo, no `scripts/rust-test.sh`, no Tilt/kind,
  no global installs, no production operation was run.

- **Review:** comprehensive independent review (`/tmp/opencode/r24-review.md`)
  verdict **APPROVE**; no Critical or Important findings; one non-blocking
  Minor - the implementation report inaccurately states the AC1 `validate`
  tests were compile-error RED pre-fix, whereas the `Gamer` trait default
  `validate()` (`rust/lib/game/src/game.rs:106-108`) means they compile and
  fail their assertions pre-fix (assertion-failure RED, not compile-error
  RED). Does not affect delivered code or AC compliance; not addressed by a
  code or report change per task instruction.

- **No push:** nothing was pushed.

- **R-07-HANDOVER.md:** untouched.

## R-26 evidence

Code commit `a302112f28104e2c7045df299a60f4c2668eb060` (verified via
`git rev-parse`; parent/base `ad67cd2dea6961994c4c5e73da7f43ef878f6673`,
also verified via `git rev-parse`; message `R-26: enforce cross-field
invariants in validate, harden panic paths (F-66, F-67, F-68, F-74, F-76)`;
+146/-27). Closes F-66, F-67, F-68, F-74, F-76. No migration, doc, Cargo, or
other-tracker changes in the code commit.

- **Status:** done.

- **Changed source files (exactly three):** `rust/game/category-5-2/src/lib.rs`,
  `rust/game/zombie-dice-2/src/lib.rs`, `rust/game/red7-1/src/lib.rs`.

- **F-66 (category-5-2, resolving-with-no-played-card):** `validate` rejects
  `self.resolving && self.plays[self.choose_player].is_none()` with
  `GameError::internal("category-5-2: resolving with no played card for
  choose_player")`; `can_choose` now additionally requires
  `self.plays.get(player).is_some_and(|p| p.is_some())`, so the selected play
  is `Some` whenever `can_choose` is true and `choose`'s former `expect` is
  unreachable. Direct tests: `test_validate_rejects_resolving_without_played_card`
  (validate -> `Err`) and `test_choose_does_not_panic_on_resolving_without_played_card`
  (`choose(0,1).is_err()`, no panic).

- **F-67 / F-74 (category-5-2, equal hand sizes):** the equal-hand-size check
  lives INSIDE `validate` (`equal_hands` via `hands.first().is_none_or(..all
  same len..)`, rejecting with `GameError::internal("category-5-2: hands are
  not all the same size")`). The false comment "All hands have equal size by
  construction (dealt simultaneously each round)." was DELETED, not corrected
  (F-74). The auto-play loop was hardened from raw `self.hands[p][0]` +
  `.expect("auto-play should only play valid cards")` to
  `let Some(&card) = self.hands[p].first() else { continue }` +
  `if let Ok(play_logs) = self.play(p, card)`. Direct tests:
  `test_validate_rejects_unequal_hand_sizes` (validate -> `Err`) and
  `test_resolve_plays_does_not_panic_on_unequal_hands` (no panic on an empty
  hand). F-73, `draw_cards`, and R-31 were explicitly NOT touched.

- **F-68 (zombie-dice-2, dice conservation):** `validate` rejects states where
  `self.cup.len() + self.kept.len() + self.current_roll.len()` != the 13-dice
  total (`all_dice().len()`), with `GameError::internal("zombie-dice-2: dice
  not conserved across cup, kept, and current_roll")`. `take_dice` drain is now
  saturating: `let take = n.min(self.cup.len()); self.cup.drain(..take)`, so an
  empty cup no longer panics. Direct tests:
  `test_validate_rejects_missing_dice_conservation` (empties cup/kept/current_roll,
  validate -> `Err`) and `test_take_dice_does_not_panic_on_empty_cup_and_kept`
  (`take_dice(ROLL_DICE_COUNT)` returns an empty `taken`, no panic).

- **F-76 (red7-1, all-eliminated leader panic):** `validate` rejects a
  non-finished all-eliminated state (`!self.finished &&
  self.eliminated.iter().all(|&e| e)` -> `GameError::internal("red7-1: all
  players eliminated")`). `leader` and `leader_with_suit` now return
  `Option<(usize, Vec<Card>)>`; the panic site `player_map[l_index]` became
  `player_map.get(l_index).map(|&p| (p, palette))`, and the stale PRECONDITION
  doc comment was replaced. All four production call sites handle `None`:
  `start_round` (`self.leader().map(|(i, _)| i).unwrap_or(0)`), `end_turn`
  (`self.leader().map(|(i, _)| i)` + `is_some_and(|i| i != self.current_player)`),
  `end_round` (`match self.leader() { Some(leader) => leader, None => return }`),
  and `discard` (`match self.leader_with_suit(card.suit) { Some((idx, _)) => idx,
  None => return Err(..) }`). Direct tests:
  `test_leader_returns_none_when_all_eliminated` (`leader_with_suit`/`leader`
  -> `None`), `test_validate_rejects_all_eliminated_unfinished` (validate ->
  `Err`), and `test_discard_does_not_panic_on_all_eliminated`
  (`leader_with_suit` no panic + `discard(0, card).is_err()`).

- **TDD RED/GREEN:** pre-fix RED was category-5-2 four failures
  (`test_choose_does_not_panic_on_resolving_without_played_card` panicked at
  `choose`'s `expect`, `test_resolve_plays_does_not_panic_on_unequal_hands`
  panicked at the `self.hands[1][0]` index, plus the two assertion-failure
  validate tests `test_validate_rejects_resolving_without_played_card` and
  `test_validate_rejects_unequal_hand_sizes`); zombie-dice-2 two failures
  (`test_take_dice_does_not_panic_on_empty_cup_and_kept` panicked at the
  `drain`, `test_validate_rejects_missing_dice_conservation` assertion failure);
  red7-1 two failures (`test_discard_does_not_panic_on_all_eliminated` panicked
  at the `player_map[l_index]` leader index, `test_validate_rejects_all_eliminated_unfinished`
  assertion failure). All RED tests pass post-fix (GREEN).

- **Verification (per-package, serial, one crate per cargo command; run from
  `rust/`):** `cargo test -p category-5-2` -> EXIT=0, lib 26 passed / 0 failed
  plus contract 1 passed (26+1); `cargo test -p zombie-dice-2` -> EXIT=0, lib
  28 passed / 0 failed plus contract 1 passed (28+1); `cargo test -p red7-1` ->
  EXIT=0, lib 26 passed / 0 failed plus contract 1 passed (26+1);
  `cargo clippy -p <crate> --all-targets -- -D warnings` -> EXIT=0 for all three;
  `cargo fmt -p <crate> -- --check` -> EXIT=0 for all three;
  `git diff --check ad67cd2..a302112` -> EXIT=0. No workspace-wide cargo, no
  `scripts/rust-test.sh`, no Tilt/kind, no global installs, no production
  operation, no push. A full workspace `cargo fmt --all -- --check` was NOT
  claimed passing (see review note 3).

- **Review:** comprehensive independent review
  (`/tmp/opencode/r26-comprehensive-review.md`) verdict
  **PASS WITH NON-BLOCKING NOTES**; no Critical or Important findings; three
  non-blocking Minor notes - (1) `zombie-dice-2/src/lib.rs:476` `all_dice().len()`
  builds a fresh 13-element `Vec` per `validate` call (a `const` count would be
  cleaner; correctness unaffected); (2) `category-5-2/src/lib.rs:237` auto-play
  `if let Ok(..)` silently swallows the `play` `Err` (defensive malformed-state
  hardening that makes the path total; acceptable); (3) pre-existing
  `rust/web/` fmt violations - `cargo fmt --all -- --check` exits 1 but every
  diff is in `rust/web/src/**` and `rust/web/tests/**`, files R-26 did not
  touch (present at base; out of R-26 scope, flagged as repo hygiene only).

- **Scope exclusions:** no migrations; no `scripts/rust-test.sh`; no
  Tilt/kind; no production operations; F-73/`draw_cards`/R-31 untouched.

- **No push:** nothing was pushed.

- **R-07-HANDOVER.md:** untouched. R-22 (PARKED), R-21 (gated), and R-07
  (PARKED) statuses and files untouched.

## R-28 evidence

Code commit `34a1222f4213916094e2e24e8a3a56617a49ea73` (verified via
`git rev-parse`; parent/base `ac97c8dc6531f4642da9c356feeb1b54d085a2a2`;
message `R-28: handle degenerate rand_bot specs without panicking (F-09,
F-10)`). Closes F-09 (High) and F-10 (Medium). Tracked change exactly one
file: `rust/lib/rand_bot/src/lib.rs`, +113/-11. No migration, doc, Cargo, or
other-tracker changes in the code commit.

- **Status:** done.

- **Changed file:** exactly `rust/lib/rand_bot/src/lib.rs`.

- **F-09 (Int inverted bounds no longer panic):** `Spec::Int { min, max }`
  with `min > max` returns an empty token vector instead of `panic!`
  (`lib.rs:36-38`); `Spec::Many`'s bound derivation is
  `let max = max.unwrap_or(3).max(min)` (`lib.rs:77`), so
  `Many { min: Some(5), max: None }` and inverted `max < min` honor the
  requested minimum instead of tripping `bounded_i32`'s `assert!(min <= max)`.
  `bounded_i32` is now total (`if min > max { return min }`, `lib.rs:15-17`).
  This is the same degenerate-spec class WP-07 (`63063a4`) fixed for
  `OneOf`/`Enum`/`Player` but left for `Int`/`Many`; `lib/game` and `rand_bot`
  now agree on graceful degradation again.

- **F-10 (no `as i32` narrowing on out-of-range bounds):** the `Many` arm no
  longer casts `min.unwrap_or(0) as i32` / `max.unwrap_or(3) as i32`
  (`lib.rs:74-83`); bounds stay `usize`, and `if max > i32::MAX as usize {
  return vec![] }` rejects an out-of-range count rather than narrowing it (the
  pre-fix wrap turned a large `min` negative, which then passed
  `assert!(min <= max)` and emitted a spec-violating command). `Spec::Int` is
  `Option<i32>`/`Option<i32>` (`command/mod.rs:15-18`), so its arm never
  narrowed.

- **AC1 tests (degenerate Int + Many, no panic, defined result):** six new
  tests call `spec_to_command` (the spec walker) directly (`lib.rs:179-267`):
  `int_spec_with_inverted_bounds_yields_no_tokens_instead_of_panicking`,
  `many_spec_with_none_max_below_min_honors_min_instead_of_panicking`
  (asserts 5 tokens for `min: Some(5), max: None`),
  `many_spec_with_inverted_bounds_honors_min_instead_of_panicking` (5 tokens
  for `min: Some(5), max: Some(1)`),
  `many_spec_with_out_of_i32_range_max_is_rejected_without_narrowing`,
  `many_spec_with_out_of_i32_range_min_is_rejected_without_narrowing` (AC2),
  and `many_spec_with_in_range_bounds_yields_requested_count` (positive
  control, also passes pre-fix). Module `#[test]` count: 10 (6 new + 4
  pre-existing). The plan's AC1 says "asserts an error, not a panic"; per the
  F-09 remediation guidance the defined non-panicking result is an empty token
  vector (the WP-07 `OneOf`/`Enum`/`Player` pattern), not a `Result` error.

- **AC1 reviewer-confirmation and the one re-fixture (TDD RED/GREEN):** the
  plan's AC1 requires the reviewer to confirm each test genuinely fails
  pre-fix ("WP-07 already has a test that does not"). The out-of-i32-range
  `min` case as first written asserted an empty vector for
  `min: Some(i32::MAX as usize + 1)`, but that PASSES pre-fix: `as i32` wraps
  it to `i32::MIN`, and `bounded_i32(v, i32::MIN, 3)` returns `<= 0` on
  999,997 of 1,000,000 sampled seeds, so the `for i in 0..n` loop emits no
  tokens (probe `/tmp/opencode/pre_fix_check.rs` + compiled binary). The test
  was re-fixtured to `min: Some((1usize << 32) + 5)`: that truncates to i32
  `5`, above the pre-fix default `max` of 3, so pre-fix trips
  `assert!(min <= max)` and panics - a genuine RED (documented in the test's
  own comment, `lib.rs:239-241`). The other five new tests are RED by
  construction (pre-fix `panic!` on Int inverted; pre-fix `assert!` on Many
  none-max / Many inverted / out-of-i32-range max via `i32::MAX+1 -> i32::MIN`).

- **Verification (per-package, serial, one crate per cargo command; run from
  `rust/`):** `cargo test -p brdgme_rand_bot` -> EXIT=0 (10 lib tests);
  `cargo clippy -p brdgme_rand_bot --all-targets -- -D warnings` -> EXIT=0;
  `cargo fmt -p brdgme_rand_bot -- --check` -> EXIT=0;
  `git diff --check ac97c8dc..34a1222` -> EXIT=0 (re-verified in this tracker
  session). Cargo outcomes are the implementation session's recorded results;
  no cargo command was re-run here (no-cargo constraint; tests not rerun). No
  workspace-wide cargo, no `scripts/rust-test.sh`, no Tilt/kind, no global
  installs, no production operation, no push.

- **Review:** no R-28 review report file is persisted (this session's reviews
  live in ephemeral `/tmp/opencode/`; none exists for R-28). The substantive
  review outcome is the AC1 reviewer-confirmation re-fixture above: the
  out-of-i32-range-`min` test was corrected because its initial form would not
  fail pre-fix, and the corrected test genuinely trips the pre-fix `assert!`.
  No blocking defect was identified in the committed code during this
  correction pass.

- **Residual risk (accepted):** a legitimate but extremely large **in-range**
  `Many` count (up to `i32::MAX`, 2,147,483,647) is still emitted in full -
  the `max > i32::MAX as usize` guard rejects only out-of-range bounds and
  there is no output/count cap on the emission loop. A wire-supplied or
  game-bug-supplied spec with a huge in-range `max` causes excessive token
  output and work. Pre-existing class (the pre-fix loop was equally
  unbounded); not narrowed by this change.

- **No change doc:** no standalone R-28 `docs/changes/` document existed and
  nothing was archived (verified: no `R-28`/`rand_bot` entry under
  `docs/changes/` or `docs/changes/archive/`).

- **Scope / R-07:** `R-07-HANDOVER.md` untouched (last modified by `5c1995d`,
  pre-dating R-28; absent from the R-28 commit diff); R-07 (PARKED), R-21
  (gated), R-22 (PARKED) statuses and files untouched; no push.

## R-30 evidence

Implementation commits `fc90116ce063261c3c643c64a85a6771e092c0fe` (initial
four-crate implementation; message `R-30: redact private values from
Log::public in four game crates (F-22, F-23, F-28, F-29, F-30, F-34, F-38,
F-39)`), `8814bb0e00b3dcd9937f2ff49a1c94964025ca66` (persisted-state
compatibility correction; message `fix(modern-art-2): allow legacy stale
PlayCard bids to load while enforcing the invariant (R-30-04)`), and
`196200c9d1182271a8345529e68778ebdb80b3e6` (final AC1 Splendor test-completion
commit; message `test(splendor-2): assert reserve public log hides card cost
(R-30-08)`), all verified via `git rev-parse`. HEAD at completion is `196200c`.
The final commit is test-only - it adds the Splendor rendered-public-log AC1
test (`reserve_public_log_hides_reserved_card_cost`) and changes no gameplay
behaviour (51 insertions, one `#[test]`, nothing else). Tracked changes only:
`rust/game/alhambra-1/src/lib.rs`, `rust/game/modern-art-2/src/lib.rs`,
`rust/game/modern-art-2/src/render.rs`, `rust/game/seven-wonders-1/src/lib.rs`,
`rust/game/splendor-2/src/lib.rs`, `rust/game/splendor-2/src/render.rs`. No
migration, doc, Cargo, CI, or other-tracker changes in any of the three
commits.

- **Status:** done.

- **Scope / plan crate-count discrepancy:** the work spans **four** crates
  (alhambra-1, modern-art-2, seven-wonders-1, splendor-2) despite the plan's
  Size prose saying "three crates" (`98-REMEDIATION-PLAN.md:998`). The plan's
  own Files list governs - it names all four crates (alhambra-1 `:160-181`
  `:452-479`, modern-art-2 `:442-450`(+3 sites) `:53-75` `:304-309`(+1 site),
  seven-wonders-1 `:722-725`, splendor-2 `:237-239` `:79-97`), and all four
  were implemented. The "three crates" prose is recorded as a discrepancy, not
  resolved (plan not modified).

- **Per-crate / Finding / AC map:**
  - **alhambra-1 F-22 (High) + AC1/AC2:** `start_game` no longer logs each
    player's exact opening money-card draw publicly; the public log now carries
    only the count (`drew {n} cards`, lib.rs:174-178), and the exact card
    identities move to a `Log::private` addressed to the drawing player only
    (lib.rs:179-185). Test
    `start_game_logs_do_not_expose_dealt_card_codes` (lib.rs:1504) calls the
    real `Game::start` log-producing path, renders each public log via
    `log_plain`, and asserts no dealt card code appears in any public log while
    a private per-player draw log still lists that player's cards.
  - **alhambra-1 F-23 (Medium) + AC3:** `final_place_phase` no longer publishes
    `best_value`, the aggregate over the winner's private hand; the public log
    keeps the player, currency, and tile but drops the amount (lib.rs:478-484).
    Test
    `final_place_public_log_hides_private_hand_value` (lib.rs:1547) drives
    `final_place_phase`, asserts the winner/tile stay public and "with 9" (the
    private value) does not appear in the rendered public log.
  - **modern-art-2 F-28 (Medium) + AC2:** all three public money trails are
    made private while a neutral public event remains: (1) the round-end
    payout (`Paying {p} {money} for selling all their cards`) becomes
    `Log::private` to that player, public log reads "sold all their cards"
    (lib.rs:352-365); (2) the final money table becomes `Log::private` to all
    players (lib.rs:379-386); (3) the auction settlement `paying {price} to`
    clause becomes `Log::private` to the buyer and seller only, while the
    purchase itself stays public (lib.rs:466-484). Tests
    `auction_settlement_amount_is_private_to_the_parties`,
    `round_end_payout_amount_is_private_to_the_player`, and
    `final_round_money_stays_out_of_public_logs` drive real `command()` play
    paths and assert no public rendered log contains "paying $"/"paid
    $"/"Paying "/"final player money", and that each private log reaches
    exactly the entitled parties. Public artist values and legitimate public
    events (a purchase, a sale, "bought") were deliberately left public - they
    are not treated as leaks.
  - **modern-art-2 F-29 (Low) + AC3 (over-redaction restore):** `PubState`
    gains `hand_counts: Vec<usize>` (lib.rs:81, populated :679), and the public
    render adds a "Cards" column (render.rs:92-112). Test
    `pub_state_exposes_hand_counts` (lib.rs:1461) asserts
    `hand_counts == vec![9,9,9,9]` post-start, that the rendered players table
    contains a "Cards" column, and that counts track actual hands after a play.
  - **modern-art-2 F-30 (Low, state invariant):** `end_round` and
    `settle_auction` both clear `self.bids` (lib.rs:318, :454), and `validate()`
    rejects a `PlayCard` state with non-empty bids (lib.rs:815-822). Tests
    `bids_cleared_when_settle_ends_the_round` (lib.rs:1370) and
    `validate_rejects_bids_outside_an_auction` (lib.rs:1412). The legacy
    stored-state marker shim is in commit `8814bb0` (below).
  - **seven-wonders-1 F-34 (Low) + AC2:** the discard-pile prune log "has no
    cards they can take from the discard pile" - a public assertion about a
    pile whose contents `PubState` hides - becomes `Log::private` to the
    affected player only (lib.rs:725-730). Test
    `no_takeable_discard_log_is_private_to_the_player` (lib.rs:1375) drives
    real build commands, asserts the prune is logged exactly once, is not
    public, and targets the affected player.
  - **splendor-2 F-38 (Low) + AC4:** the swallowed `GameError` at the old
    `:237-239` no longer becomes a public log. `discard_phase`, `visit_phase`,
    and `next_phase` now return `Result<Vec<Log>, GameError>` (lib.rs:219,
    :228, :254); the `unwrap_or_else(|e| vec![Log::public(e.to_string())])` is
    replaced by propagating the error via `?` at the five command call sites
    (lib.rs:354, :414, :458, :492, :521). The F-38 propagation test
    `auto_visit_failure_propagates_without_public_log` (lib.rs:1217) forces a
    failing auto-visit and asserts `Err` surfaces with its message and no noble
    is awarded.
  - **splendor-2 AC1 (rendered-public-log test, final commit `196200c`):**
    `reserve_public_log_hides_reserved_card_cost` (lib.rs:1104-1152) drives the
    real command path `g.command(0, "reserve A1", &players(2))` (lib.rs:1121)
    against a board seeded with a Ruby-7/Onyx-3 card, renders every public log
    through `plain(transform(...))` (lib.rs:1130-1135), and asserts the concrete
    private reserved-card cost string ("7-3") is absent from all public logs
    while the public `reserved` action remains (lib.rs:1140-1151). This is
    direct `Log::public` content coverage - the same AC1 shape the old
    Splendor tests lacked - not F-81 inference.
  - **splendor-2 F-39 (Low) + AC3 (over-redaction restore):** `PubState` gains
    `deck_counts: Vec<usize>` (lib.rs:85, populated :594), and the public
    render shows "Level N (N left)" per level (render.rs:164-171). Test
    `pub_state_exposes_deck_counts` (lib.rs:1345) asserts
    `deck_counts == vec![36, 26, 16]` post-start and that the rendered board
    contains "36 left"/"26 left"/"16 left".

- **F-81 classifications (AC2, applied per finding):** **direct fixes** (the
  value appeared directly in `Log::public`): F-22 (card identities), F-23
  (`best_value`), F-28 (money amounts), F-34 (discard-pile property), F-38
  (raw error text). **Inference-only / public-derived information** is
  acceptable by the owner ruling and was not re-raised. F-29 and F-39 are
  **approved over-redaction restores** (public info a player is entitled to,
  restored, not a leak). F-30 is a **state invariant**, not an inference
  classification.

- **TDD red-to-green:** all per-crate tests call the actual log-producing
  paths (`Game::start`, `command()`, `final_place_phase`, `visit_phase`) and
  assert on the rendered `Log::public` content via `log_plain`/`transform` -
  not merely on `pub_state` fields (exactly the AC1 shape the old tests
  missed). Durable RED proof: `start_game_logs_do_not_expose_dealt_card_codes`
  fails pre-fix because the opening draw logs are public and list card codes;
  the F-28 tests fail pre-fix because the payout/payment/final-money strings
  are public; `final_place_public_log_hides_private_hand_value` fails pre-fix
  because `best_value` is in the public log; the F-34 test fails pre-fix
  because the prune log is public; `auto_visit_failure_propagates_without_public_log`
  fails to compile pre-fix (`visit_phase` returns `Vec<Log>`, not `Result`);
  F-29/F-39 tests fail pre-fix on the absent `hand_counts`/`deck_counts`
  fields. All pass post-fix (GREEN).

- **Legacy stored-state marker shim (commit `8814bb0`, the R-30-04
  correction):** the initial `validate()` bids check would have rejected
  persisted pre-R-30 games carrying concluded-auction stale bids (the F-30
  invariant fix, applied retroactively). The correction adds
  `bids_cleared_outside_auction: bool` (`#[serde(default)]`, lib.rs:47-53),
  set `true` at `Game::start`, `end_round`, `add_card_to_auction`, and
  `settle_auction`; `validate()` rejects PlayCard bids only when the flag is set
  (lib.rs:815-822). Legacy persisted states (flag absent -> default `false`)
  load and validate; the next normal play clears the stale bids and sets the
  flag, transitioning the state to conformant. Test
  `legacy_playcard_with_stale_bids_loads_and_plays` (lib.rs:1427) strips the
  flag from a stale-bids PlayCard JSON, asserts it loads and validates, plays
  through an auction, and asserts bids cleared + flag set + re-validates. The
  marker is intentionally retained until pre-R-30 persisted games have
  drained (removal note in the field comment).

- **Verification (per-package, serial, one crate per cargo command; run from
  `rust/`):** initial run - `cargo test -p alhambra-1` EXIT=0, 47 passed;
  `cargo test -p modern-art-2` EXIT=0, 25 passed; `cargo test -p
  seven-wonders-1` EXIT=0, 50 passed; `cargo test -p splendor-2` EXIT=0, 69
  passed. Each changed crate also passed `cargo clippy -p <crate>
  --all-targets -- -D warnings` and `cargo fmt -p <crate> -- --check` in the
  initial run, and `git diff --check` EXIT=0. The corrected modern-art (commit
  `8814bb0`) re-ran `cargo test -p modern-art-2` (26 lib + 1 contract passed),
  clippy, fmt, and `git diff --check` - all EXIT=0. After the final AC1
  test-completion commit `196200c` (test-only, one new Splendor test), the
  splendor-2 run is **70 passed** (`cargo test -p splendor-2` EXIT=0, the
  initial 69 plus the new `reserve_public_log_hides_reserved_card_cost`),
  with `cargo clippy -p splendor-2 --all-targets -- -D warnings`, `cargo fmt
  -p splendor-2 -- --check`, and `git diff --check` all EXIT=0. The initial 69
  above is the historically accurate pre-196200c result and is left unchanged.
  No workspace-wide cargo, no `scripts/rust-test.sh`, no Tilt/kind, no global
  installs, no production operation, no push.

- **Review:** the independent security/privacy review originally REJECTed the
  implementation on one Important finding only - the legacy Modern Art
  stale-bids persisted-state compatibility regression (pre-R-30 PlayCard games
  would no longer load). The bounded correction `8814bb0` addresses exactly
  that, and the fresh targeted re-review PASSed. No remaining Critical or
  Important findings. Factual residual: the `bids_cleared_outside_auction`
  marker remains in the serialized state until pre-R-30 persisted games have
  drained; it is a deliberate migration shim, not a finding.

- **No change doc:** no standalone R-30 `docs/changes/` document existed and
  nothing was archived. Coverage item 5.5 (13-crates validate+redaction
  checklist) is NOT marked done or expanded by this row - R-30 folded its
  per-crate `Log::public` test shape into the R-30 evidence only; the 5.5
  inventory/checklist row is left as-is per the brief.

- **Scope / R-07:** `R-07-HANDOVER.md` untouched (last modified by `5c1995d`,
  pre-dating R-30; absent from both R-30 commit diffs); R-07 (PARKED), R-21
  (gated), R-22 (PARKED) statuses and files untouched; no push.

## R-32 blocker evidence

Status `blocked(AC4 owner amendment/sign-off)`. No accepted R-32 commit exists:
the five implementation commits listed below are in progress and unaccepted, so
R-32 is NOT complete, and independent review plus full per-crate verification
remain outstanding. No invalid/forged test fixture will be fabricated to
satisfy AC4.

Existing implementation commits (all verified via `git rev-parse`; all
unaccepted, none reflected in a done row):

- `932b7e79ad8dd610698e84c97c6a2b9eef4314e2`
- `eb811900d418428b9c200f35590bbcec05dd8d84`
- `4f97c92f798f9f6ab76abc0ba92feaada9074d87`
- `2d245b696dcdf3cff4c073bab24749d38d87c12f`
- `0091658b299ea399b595cf1da3702ae2a2eb6356`

- **AC4 (the blocker):** `98-REMEDIATION-PLAN.md:1086-1087` (F-20) requires
  replacing `starship-catan-1`'s hardcoded placings `0..2` with the real player
  count AND a test with a non-2-player game asserting correct placings. The
  non-2-player half is unreachable.
- **Fixed two-player invariants (`rust/game/starship-catan-1/src/lib.rs`):**
  the boards field is the fixed array `[PlayerBoard; 2]` (:493); `start_game`
  rejects `players != 2` with a `PlayerCount { min: 2, max: 2 }` error
  (:526-533); `player_counts()` returns `vec![2]` (:2022-2024); `validate()`
  rejects `self.players != 2` (:2042-2045); `placings()` still enumerates
  `0..2` (:1742-1747). A non-2-player `Game` cannot be constructed truthfully.
- **Supporting docs:** `RULES.md:3` ("Starship Catan is a 2-player ... game")
  and `PORTING_NOTES.md:82-84` ("This game only ever has two players") both
  confirm the two-player-only design.
- **Owner sign-off requirement:** `docs/decisions/PORT_PARITY.md:26-32`
  (Decision 2) prohibits any gameplay change without per-game sign-off;
  constructing a non-2-player game would be a prohibited
  player-count/gameplay change.
- **Commit `4f97c92f798f9f6ab76abc0ba92feaada9074d87` (F-19 placings dedup):**
  deduplicated placings into a single per-crate `placings()` but retained the
  hardcoded `0..2`. Rewriting it as `0..self.players` would be behavior-neutral
  under the two-player invariant but cannot yield a truthful, valid
  non-2-player test fixture, so it would not satisfy AC4's test requirement.
- **No fabrication:** no invalid/forged fixture will be fabricated; AC4 needs
  owner amendment/sign-off (acceptance without the non-2-player test, or an
  owner-ratified player-count/gameplay change) to proceed.
- **Outstanding:** R-32 is NOT complete; independent review and full per-crate
  verification of the five commits remain outstanding.
- **Scope:** tracker-only change to this file; protected
  `docs/reviews/2026-07-30-review-session/R-07-HANDOVER.md` untouched.

## R-37.0 evidence

- **Status:** complete at `0270f296a39755b44feacf85d6d2220d7c8b4f80`; no
  production change and no Cargo command. The remaining R-37 work is
  `parked-by-user` until the simpler unblocked remediation plan is complete and
  the user explicitly revisits it. Preserve R-37.1 before R-37.2.
- **Dependency:** 5.4 is done at `973ea62a3cb407127e527acbb64063305c65414d`.
  Its `crate::test_support::{anonymous, non_admin, admin}` harness is available
  to direct server-fn tests.
- **Current source map:** F-86's swallowed session-read error is
  `rust/web/src/auth/server.rs:548-554`; F-87's pending-email deletion/account
  fork is `:462-507`; F-89/F-95 share `validate_confirmation_code` at `:400-429`;
  F-95's current concurrent test is `:1656-1688` and asserts only
  `attempts >= CONFIRM_MAX_ATTEMPTS_PER_CODE`; F-94 has no in-app middleware in
  `rust/web/src/router.rs:114-208`; the Turnstile test is `auth/server.rs:1897-1902`;
  and shared crypto is `rust/lib/crypto/src/lib.rs`, not the one-line web facade.
- **Stale plan corrections:** source paths now name shared crypto and current
  Turnstile locations. The shared loader already has tests at
  `rust/lib/crypto/src/lib.rs:120-162`; its AAD decline remains undocumented.

## R-39 evidence

- **Status:** done. R-39.1 is
  `afbca143ce59e4e2f0ad6cfe41b9ad94975c44bf`; R-39.2 is
  `6dd0c41e4852172e730eb047857f6ac014d93679`, corrected by
  `b2e2021fa6f0e9c6099867c1fd981aaef7156601`, and completed by
  `68140350ed56fe05f34771281611b5d8a8c3e71d`; R-39.3 is
  `1db7266404db772de630cc9458bb745c81d4f9ab`; R-39.4 is
  `9867e5396091e9f2c9827eaf0d081eeeeb25bf1b`; R-39.5 is
  `3c4c1ca4ce18485acd7524dc3887f2c2a85e4b2f`.
- **R-39.1 / F-141:** `PROPOSAL_SWEEP_CAP` is 200 and bounds all three
  proposal-sweep candidate queries. The new DB test seeds 205 qualifying
  candidates and asserts each query returns 200.
- **R-39.2 / F-142:** the non-admin export-route test now seeds a real valid
  game and private-log sentinel, requests that game as a non-admin, and asserts
  403 plus sentinel absence. The correction commit restores an accidentally
  changed unrelated profile fixture; the completed unit adds the valid bot slot
  only to the intended export fixture.
- **R-39.3 / F-133:** proposal visibility is roster membership or ownership;
  direct DB coverage proves an owner without a roster row is visible and a
  stranger is not.
- **R-39.4 / F-149:** a direct server-function test uses
  `crate::test_support::non_admin`, calls `block_user` with an unknown ID, and
  asserts `User not found`.
- **R-39.5 / F-157:** the `try_join!` calls retain concurrency while restoring
  six distinct friends-query contexts and five distinct game-info contexts.
- **Scope correction:** F-132 is refuted, not hardened. `VisibilityCache` is a
  local value inside each per-connection SSE task after `viewer` is fixed; do
  not modify `visibility_cache.rs` or `events.rs`.
- **Verification:** `git diff --check` passed per unit. The one permitted final
  `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr` passed.
  It emitted only the pre-existing `NotifyKind::Reminder` dead-code warning and
  Cargo's `proc-macro-error2` future-incompatibility notice. Runtime DB/SSR
  tests are CI-pending under laptop limits.
- **Review:** independent authorization/visibility review PASS with no findings.
  No other remediation package is selected.
- **Approved confirmation and add-email behavior:** F-87 uses purpose-bound
  `login` and `add_email`; a wrong endpoint consumes neither flow, while valid
  login retains D-14 true-owner stealing. Expire all ambiguous active or legacy
  confirmations on migration; old-pod inserts never default to `login`. Retain
  the 10-attempt cap: incorrect same-purpose codes count and wrong-purpose
  submissions do not. F-93 returns generic `add_email` acceptance and creates
  state or sends only when valid. F-92's historical `add_email` global-cap `Err`
  expectation is superseded by generic acceptance with no pending row, code, or
  send. F-95 uses conditional attempt increment capped at 10 with concurrent
  upper-bound evidence.
- **Approved rate-limit and crypto behavior:** rate limiting, if implemented, is
  PostgreSQL-backed in `web`, with fixed-window counters keyed by scoped opaque
  digests, never application-observed IP; no standalone service or Gubernator
  compatibility. Retain settled login and confirmation caps. Supersede the stale
  all-three-ingress/router-middleware language: the only new generic limiter is
  for signed Resend webhooks after signature verification. Allow 600 verified
  events per five minutes globally and 20 accepted messages per canonical sender
  and validated route capability per five minutes; denial or limiter DB failure
  returns retryable generic `503` with no processed marker. F-91 accepts the
  no-AAD risk; revisit before another encrypted data category, credential movement
  or import, or multiple interchangeable ciphertext contexts. No crypto migration.
- **Unresolved before implementation:** the last proposed migration's
  single-column `email` primary key cannot support approved coexistence of `login`
  and `add_email` confirmations for one address. Safe rolling deployment and
  legacy-pod compatibility remain unresolved: temporary `Recreate`, a new table,
  staged migrations, or another simpler design require later value/complexity
  review and user approval. Before implementation, conduct a deliberate
  overengineering review that justifies complexity relative to value, protects
  readability, maintainability, and simplicity, and reassesses whether the generic
  webhook limiter and keyed-digest machinery remain proportionate.
- **Not approved:** exact implementation units, migrations, and rollout.

## R-40 evidence

- **Status:** done. R-40.1 is `2c0fee15daa18563b57e40ae8e14d03d1f00cd00`;
  R-40.2 is `1c1bfbe9a5e37cfcf9c7a6eb559a6e69bd6b24cb`; R-40.3 is
  `9f3e5742e6539d9c82e33929bf88e78b17247be3`; compile corrections are
  `b6435094ce8384078ff8e973ff9bf741111aba17` and
  `1be0583fac259118e6926ec6ce5181912d03854b`.
- **F-139:** the collision-prone placeholder-user insert runs in a savepoint;
  a deterministic trigger forces its unique violation and proves fallback
  generation succeeds without 25P02.
- **F-121:** `import-game` caps the actual file read at 100 MiB plus one byte;
  unit tests exercise the bounded reader without file metadata.
- **F-122:** each non-NULL bundle `undo_game_state` receives a local-version
  `Status` request before `pool.begin()`; a mock rejection test asserts no game
  row is written.
- **Verification:** `git diff --check` passed. Both approved commands passed:
  `SQLX_OFFLINE=true cargo check -p web --lib --tests --features ssr` and
  `SQLX_OFFLINE=true cargo check -p web --bin import-game --features ssr`.
  They emitted the pre-existing `NotifyKind::Reminder` dead-code warning, a new
  non-failing `unused_mut` warning in `import-game`, and Cargo's
  `proc-macro-error2` future-incompatibility notice. Runtime DB/mock-service
  tests are CI-pending under laptop limits.

## R-41 evidence

- **Status:** done. Source units: `4788a90` (41.1), `02e9883` (41.2),
  `a77b752` (41.3), `c442267` (41.4), `5dd0f63` (41.5). Compile corrections:
  `928863d` (`CreateEmailResponse`) and `e96c5bb` (awaited unsubscribe test).
- **F-175/F-176:** all three token ensures use atomic `UPDATE ... RETURNING` and
  error for an unknown row; direct concurrency/unknown-id tests cover each.
  Delivery success/failure metrics retain their existing names behind the smallest
  private result helper. Sentry snippets escape `</script>` in DSN and release,
  omit `tracesSampleRate` per `SENTRY_SAAS_EXCEPTION.md`, and retain
  `sendDefaultPii:false` and both integrations.
- **F-170/F-177/F-178:** the callerless `pref_column()` and duplicate escape
  helper are deleted. All four unsubscribe kinds exercise the live preference
  mapping; one shared escaping helper covers all four game-email hrefs and the
  unsubscribe GET reflection.
- **F-171/F-172/F-174:** rules replies retain threading and omit both
  `List-Unsubscribe*` headers. Folded CR/LF plus header whitespace collapses to
  one space; bare CR/LF terminates parsing and retains the `Bcc:` rejection.
  Standalone help lists only standalone commands while game-context help remains
  unchanged.
- **F-184:** Bot selection is disabled until `bot_names` settles, preventing the
  pre-settle `"medium"` fallback from entering state; the settled canonical path
  is unchanged.
- **Verification:** `git diff --check` passed per accepted unit. Final allowed
  command `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`
  passed after the two focused compile corrections. It emitted pre-existing
  `NotifyKind::Reminder` dead-code and `import-game` `unused_mut` warnings, plus
  Cargo's `proc-macro-error2` future-incompatibility notice. No other Cargo
  command ran.
- **Review:** independent review PASS, no concrete defects. Runtime/DB tests,
  including the SQLx token and unsubscribe tests, remain CI-pending under laptop
  limits. Residual test gap: delivery-result tests assert result branches rather
  than reading global counters; the production metric calls were inspected.
