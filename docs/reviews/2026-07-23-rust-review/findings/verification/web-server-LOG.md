# Verification LOG: web-server (2026-07-24)

Independent verification of `findings/web-server.md` (unit 9, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

67 findings total in web-server.md, numbered F1-F67 in document order.
The file has no explicit header tally; Lead recount from the blocks:
1 critical (F1 confirm-cap bypass) / 7 major (F2 email squat, F16 default
crypto key, F18 reorder_bots, F34 undo_game ratings, F35 db test gap,
F52 Secure cookie, F53 unsupervised bot consumer) / 37 minor / 22 nit = 67.
Four serial Workers (model fable per user override), split by file/theme:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | auth (`web/src/auth/server.rs`, `auth/session.rs`, related db.rs lines) | F1 concurrent confirm cap bypass (critical), F2 email-squat DoS (major), F3 no session rotation (minor), F4 blocked-domain enumeration (minor), F5 transient DB error clears session (minor), F6 24h cap over-counts (minor), F7 Resend failure silent (minor), F8 Turnstile fail-open (minor), F9 no email normalization (minor), F10 add_email_address discards refusal (minor), F11 tokens never expire (minor), F12 code gen/compare/storage nits (nit), F13 confirm_login no shape checks (nit), F14 reqwest client per call (nit), F15 logout swallows error (nit) | raw/web-server-auth.md |
| W2 | crypto + admin (`web/src/crypto.rs`, `main.rs`, `admin.rs`, migration 013) | F16 default key fallback (major), F17 no AAD/zeroize (nit); F18 reorder_bots non-atomic (major), F19 create_bot MAX+1 race (minor), F20 short-key mask leak (minor), F21 no key clearing (minor), F22 test_provider hardcodes gpt-4o-mini (minor), F23 uncapped upstream body (minor), F24 Action input/value race (minor), F25 no server-side validation (minor), F26 admin page hard-fails on one bad row (minor), F27 bot rename/delete strands games (minor), F28 admin-gate duplicated 15x (minor), F29 rows_affected unchecked (minor), F30 update_bot_provider missing updated_at (nit), F31 redirect matches error text (nit), F32 Action-value unwraps (nit), F33 BotProviderRow shadow (nit) | raw/web-server-crypto-admin.md |
| W3 | db (`web/src/db.rs`, migrations for triggers) | F34 undo_game keeps ratings (major), F35 ~20 untested public fns (major), F36 redundant updated_at (minor), F37 is_finished/finished_at mismatch (minor), F38 zero-change defeats idempotency guard (minor), F39 friend-request 23505 race (minor), F40 friend_recent_visible_game N+1 (minor), F41 insert_game_logs_tx row-at-a-time (minor), F42 db.rs grab-bag (minor), F43 build_game_type_user sentinel (minor), F44 is_turn_at reset (nit), F45 is_user_admin sqlx::Result (nit), F46 generate_unique_username race (nit), F47 interval via string interp (nit), F48 no self-request guard (nit), F49 choose_colors clone (nit), F50 all-pairs idiom (nit), F51 test-quality nits (nit) | raw/web-server-db.md |
| W4 | infra (`main.rs`, `router.rs`, `auth/session.rs`, `nats.rs`, `websocket.rs`, `websocket_client.rs`, `bin/import_game.rs`, `web/Cargo.toml`, k8s manifests) | F52 Secure cookie unset (major), F53 bot consumer unsupervised (major), F54 no rustls provider (minor), F55 graceful shutdown gaps (minor), F56 max_deliver strands silently (minor), F57 no config-drift reconcile (minor), F58 ack_wait vs processing time (minor), F59 /ws unauthenticated firehose (minor), F60 open() on visibilitychange (minor), F61 tungstenite default limits (nit), F62 no pong tracking (nit), F63 unbounded file read (nit), F64 gloo-net unused (minor), F65 tokio net/time undeclared (minor), F66 futures-util non-optional (nit), F67 dependency currency (nit) | raw/web-server-infra.md |

Notes carried from prior units: recommended fixes are themselves checked
for validity (three bad-recommendation catches so far); claims resting on
external evidence (crates.io versions, leptos-use source, Cloudflare
behavior) are flagged external-basis, not rejected. Cross-refs available:
`findings/web-domain.md` (bot command consumer — bears on F53/F56/F58
UNCERTAIN notes) and `findings/web-frontend-email.md`; W4 checks those for
overlap notes without re-reviewing them. Lead spot-checks all
REJECTED/ADJUSTED verdicts; if a Worker confirms everything, Lead
re-verifies its 1-2 hardest confirmations. Curated report:
verification/web-server.md.

### W1 dispatched — auth (F1-F15)

### W1 returned

14 CONFIRMED, 1 ADJUSTED (F1 severity critical -> major), 0 REJECTED.
Dump: raw/web-server-auth.md.
- F1 ADJUSTED: mechanism fully confirmed (SELECT :361, Rust-side cap check
  :374, mismatch UPDATE :378-385, all on the bare pool; the advisory lock
  covers only the send path). Severity downgraded critical -> major:
  failed-guess increments still commit, so concurrency yields a bounded
  race-window multiplier over the cap of 10 (limited by in-flight request
  / pool concurrency), not "effectively unbounded" against a 1e6 keyspace.
- F8 wording fix inside CONFIRMED: the verifier-error fail-open IS
  warn-logged (:252); only the unset-secret path is silent.
- Recommendation-validity catches: F6 option 1 (reset sent_count on code
  rotation) would under-count the global 24h cap (hourly rotation = 120
  real sends/day vs cap 50) — the windowed-counter option is the sound
  fix. F1's recommended increment-before-compare needs `>` not `>=` (else
  a correct 10th attempt after 9 failures is rejected — semantics change).
  F4's uniform-rejection alternative would lock out existing verified
  users on blocked domains. F14's expect_context must be called in
  server-fn scope, not inside the helper.

### Lead spot-checks (W1)

- F1 ADJUSTED upheld — Lead read server.rs:340-390: mismatch UPDATE at
  :378-385 commits per failed guess, so attempts accumulate and later
  SELECTs see the cap; the bypass is bounded by concurrent in-flight
  requests during the commit window. Major (not critical) is right; the
  original finding itself hedged on edge throttling.

### W2 dispatched — crypto + admin (F16-F33)

### W2 returned

14 CONFIRMED, 3 ADJUSTED (F18 severity down, F24 mechanism, F27 severity
up), 1 REJECTED (F30). Dump: raw/web-server-crypto-admin.md.
- F18 ADJUSTED: non-atomic loop + no unique constraint verified, but
  severity major -> minor — impact is bot display ordering only,
  admin-only surface, self-repairable by re-dragging.
- F27 ADJUSTED: severity minor -> major — the finding's UNCERTAIN worker
  fallback is resolved: bot/src/main.rs:171-187 silently skips the turn
  and returns Ok(()) (message acked, never retried) when the bot name is
  missing/disabled, so rename/delete/disable permanently deadlocks
  in-flight games using that bot.
- F24 ADJUSTED: code shape matches, but vendored reactive_graph 0.2.14
  clears `input()` to None when in_flight reaches 0, so the claimed
  cross-attribution cannot occur per strict source reading; runtime
  behavior UNVERIFIABLE in a read-only review. Bonus: the `dispatched`
  counter guarding is_latest is never incremented (vacuous guard). The
  recommendation (key off value() alone) is valid under both readings.
- F30 REJECTED: `bot_providers` has NO updated_at column (migration
  013:23-34, no later ALTER through 021) — nothing is omitted, and the
  recommended `updated_at = now()` would be a runtime SQL error. The
  finding's secondary trigger claim (bots/llm_providers lack triggers)
  is accurate but changes nothing.
- Admin-gate recount: all 15 server fns individually gated — the file's
  "Clean" claim stands.
- F16 confirmed with context: prod k8s mounts a database-encryption-key
  secret, but the fail-open fallback path is real. Major stands.

### Lead spot-checks (W2)

- F30 REJECTED upheld — Lead read migration 013:23-34 (bot_providers:
  created_at only, no updated_at; UNIQUE(bot_id,provider_id,model)) and
  admin.rs:446-457 (UPDATE lists 5 columns, correctly none of them
  updated_at). Rejection and bad-recommendation catch both correct.
- F27 upgrade upheld — Lead read bot/src/main.rs:160-189: None + table
  non-empty -> tracing::info "skipped" + `return Ok(())`, which acks the
  NATS message; no retry, no surfacing. Permanent stall confirmed.
- F18 downgrade accepted: mixed display_order degrades ordering only;
  admin re-reorder heals it.

### W3 dispatched — db (F34-F51)

### W3 returned

15 CONFIRMED, 3 ADJUSTED (F35, F36, F51 — detail corrections, severities
stand), 0 REJECTED. Dump: raw/web-server-db.md.
- F34 major upheld and strengthened: guard at :1554, undo_game
  (:1438-1449) touches no rating field, delete_game no-rewind comment at
  :1342; the undo server path (server_fns.rs:731-784) has no is_finished
  guard, so reachability is stronger than the finding's UNCERTAIN framing.
  Recommendation caveat: "recompute on next finish" alone double-counts
  unless game_type_users is also rewound.
- F35 ADJUSTED: coverage-gap thrust holds (8/8 spot-checked functions
  untested) but two sub-claims are wrong — choose_colors (:5735-5780) and
  the ELO helpers (:5167-5190) ARE tested; drop those from the finding
  and its recommendation.
- F36 ADJUSTED: 16/18 listed lines redundant, but :1357/:1363 target
  game_proposals (migration 015: updated_at columns, NO trigger) — those
  manual sets are REQUIRED; a sweep following the finding's line list
  verbatim would break game_proposals.updated_at.
- F51 ADJUSTED: part (1) rejected — the fixture does make the caller a
  game_players row, so self-exclusion IS mutation-covered and the rename
  is unneeded; parts (2)(3) confirmed.
- Recommendation flags: F48's silent-Ok would break the existing Err
  assertion at :3317; F40's inlining trades away a deliberately
  centralized visibility predicate.
- F44 noted as possible nit -> minor (is_turn_at orders the 22d digest at
  :2934); Lead keeps nit — semantics may be intended, as the finding says.

### Lead spot-checks (W3)

- F35 ADJUSTED upheld — Lead grep: choose_colors tests at db.rs:5735-5780
  and elo_rating_change tests at :5168-5190 exist. Correction correct.
- F36 ADJUSTED upheld — Lead read db.rs:1352-1367 (both UPDATEs target
  game_proposals) and migration 015 (updated_at columns present, no
  CREATE TRIGGER). Required-set correction and sweep hazard both correct.

### W4 dispatched — infra: main/router/nats/websocket/import/Cargo (F52-F67)

### W4 returned

14 CONFIRMED, 1 ADJUSTED (F58 severity minor -> nit), 1 UNVERIFIABLE
(F67 external crates.io claims; in-repo parts verified).
Dump: raw/web-server-infra.md.
- F52 major upheld: SECURE_COOKIE grep-confirmed in exactly 2 places
  (session.rs:32 + a completed plan doc); absent from every k8s manifest
  and .env.template.
- F53 major upheld and strengthened: JoinHandle discarded, Err only
  logged, and a clean Ok(()) stream-end exit isn't logged at all;
  /healthz is a static "OK".
- F56 UNCERTAIN resolved, minor stands: the consumer never term()s but
  acks all poison classes (parse error, UserError, Conflict); only
  transient `Other` is left unacked, so stranding is limited to
  3x-transient-failure messages — neither the finding's downgrade-to-nit
  nor upgrade-to-major condition fires.
- F58 ADJUSTED minor -> nit: ack cadence resolved — full-process-then-ack
  with a hard 10s HTTP timeout and bounded retries makes exceeding the
  5-minute ack_wait implausible.
- F60 upgraded from external-basis to source-verified against vendored
  leptos-use 0.19.0 (open() -> connect() closes unconditionally; onclose
  schedules the gratuitous ~3s reconnect).
- Recommendation flags: F52's base-manifest env var would also force
  secure cookies on the dev overlay (prefer default-secure-in-code or a
  prod patch); F66's optional futures-util breaks `cargo test` without
  `--features ssr` (ungated `use futures_util` in two integration
  tests); F60's gate must use `ready_state.get_untracked()` in the
  listeners.

### Lead spot-checks (W4)

- F56/F53 evidence re-verified — Lead read web/src/game/mod.rs:251-325:
  ack on parse-error (:277), Ok/Conflict (:299-303), UserError
  (:304-314); unacked only for Other (:315-321); no term() anywhere;
  loop end returns Ok(()) silently. Resolution and severities correct.
- F58 downgrade accepted on that basis plus the worker's bounded-retry
  trace of handle_bot_command_event.

## Curation complete (2026-07-25)

57/67 CONFIRMED, 8 ADJUSTED (F1, F18, F24, F27, F35, F36, F51, F58),
1 REJECTED (F30), 1 UNVERIFIABLE (F67). 66/67 findings survive.
Corrected unit tally: 0 critical / 8 major / 36 minor / 22 nit
(original recount was 1/7/37/22; F1 critical->major, F18 major->minor,
F27 minor->major, F58 minor->nit, F30 dropped).
Report: verification/web-server.md. LOG closed.
