# R-11 Tracker Draft - final post-review record

Worker: R-11 tracker-update worker. Scope: update ONLY
`docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md` from the
R-11 comprehensive review (`review/R-11-COMPREHENSIVE-REVIEW.md`). No
production/test code, no review artefact, no unrelated file touched. Not staged,
not committed.

## File changed

`docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md`
(git diff --stat: 70 insertions, 1 deletion; file grew 235 -> 304 lines).

## Exact changed lines

### Line 36 - R-11 work-package table row (replaced, 1 deletion / 1 insertion)

Before:
- Status `pending`, empty Commit(s) column.
- Notes named the WRONG WP-84 §3g successor proof test as
  `rust/web/tests/sse_events.rs:551-595`
  (`sse_stream_survives_past_request_timeout_with_keepalive`) - the Group-4
  keepalive test (the I1 defect).

After:
- Status `done(13ab0ffd)`; Commit(s)
  `13ab0ffd3896f3b0804997a36b2b24a02c2c8147` (verified `git rev-parse HEAD`).
- **I1 corrected:** WP-84 §3g successor proof test now cited as
  `rust/web/tests/sse_events.rs:601-657`
  (`graceful_shutdown_ends_sse_stream_and_server_completes`), with an explicit
  note that the citation was corrected from the keepalive test `:551-595`.
- **AC2** paths (`game/mod.rs:263,311`, `nats.rs:214`, `nats.rs:280`,
  `sweep.rs:324,635`) and the four shutdown-path test names
  (`bot_command_consume_loop_exits_on_shutdown` `game/mod.rs:1284`,
  `sweep_stops_on_shutdown` `sweep.rs:1736`,
  `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` `nats.rs:467`,
  `supervisor_backoff_sleep_is_interrupted_by_shutdown` `nats.rs:517`).
- **AC3** R-10 per-connection-token + axum graceful-shutdown evidence, explicit
  "no `TaskTracker` reintroduced", proof test
  `graceful_shutdown_ends_sse_stream_and_server_completes` (`sse_events.rs:601-657`),
  and the documented subscribe-blocked residual (I2).
- Compile gate `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`
  exit 0 (allowed); runtime web tests deferred to CI (web build/test/run banned).
- Comprehensive review verdict CONDITIONAL ACCEPT, no Critical findings; the one
  required targeted doc-only re-review (I1 citation correction) pending; I2
  residual owner confirmation recommended.

### Lines 237-304 - new "## R-11 evidence" section (appended)

Appended immediately after the existing "## R-10 evidence" section (which still
ends at line 235), mirroring the R-09/R-10 evidence-block convention. Contents:
- Header + commit `13ab0ffd3896f3b0804997a36b2b24a02c2c8147` (verified
  `git rev-parse HEAD`); diff scope (nats.rs, email/sweep.rs, game/mod.rs,
  main.rs + the single tracker row); `websocket.rs`/`events.rs` NOT in diff; no
  `TaskTracker`/`tokio-util` `rt`; no migration/CI/Cargo.toml change.
- Closes F-109.
- AC1 bullet: deletion by `efad81f92b0a1f585410e6f30fdd8de8a3dac518` confirmed;
  corrected successor citation `sse_events.rs:601-657`
  (`graceful_shutdown_ends_sse_stream_and_server_completes`); I1 origin noted
  (`R-11-SURVEY.md:58-59,334`).
- AC2 bullet: token threading paths + the four real-production shutdown-path
  tests; advisory listener noted as bonus completeness.
- AC3 bullet: R-10 mechanism + proof test; no TaskTracker; I2 residual
  (`events.rs:86,93,206`; `R-11-SURVEY.md:73-76`); owner confirmation
  recommended.
- Runtime bullet: compile-verified only; deferred to CI under the ban.
- Gate bullet: exact `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` exit 0 (pre-existing `proc-macro-error2` warning only).
- Review bullet: CONDITIONAL ACCEPT; no Critical; I1 + I2 Important; M1/M2/M3
  Minor; committed code accepted as-is; one required targeted doc-only re-review
  pending (confirm `:551-595` -> `:601-657`); no code re-review needed.

## Verification performed

- Read `review/R-11-COMPREHENSIVE-REVIEW.md` in full.
- Read the tracker in full; edited only the R-11 row and appended one section.
- Confirmed test `graceful_shutdown_ends_sse_stream_and_server_completes` spans
  `rust/web/tests/sse_events.rs:601-657` (read the file).
- Confirmed `git rev-parse HEAD` == `13ab0ffd3896f3b0804997a36b2b24a02c2c8147`
  and that commit is `fix(web): drain background tasks on shutdown (R-11, F-109)`.
- `git status --short`: only `97-REMEDIATION-PROGRESS.md` modified; the `??`
  entries are pre-existing untracked files, untouched. Not staged, not committed.
