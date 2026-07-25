# Verification: web-server (unit 9)

Independent verification of `findings/web-server.md` (originally reviewed
by Kimi K3), performed 2026-07-24/25 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Scope: `web` crate server infrastructure — `main.rs`, `router.rs`,
`state.rs`, `config.rs`, `db.rs`, `nats.rs`, `error.rs`, `crypto.rs`,
`admin.rs`, `auth/`, `websocket.rs`, `websocket_client.rs`,
`bin/import_game.rs`, manifest-level `Cargo.toml`. Raw verdict dumps:
`raw/web-server-auth.md`, `raw/web-server-crypto-admin.md`,
`raw/web-server-db.md`, `raw/web-server-infra.md`. Process log:
`web-server-LOG.md`.

Cross-unit evidence: two of the original file's UNCERTAIN cross-checks
(F56 poison-message handling, F58 ack cadence) were settled by reading
the bot command consumer (`web/src/game/mod.rs:251-325`) directly, and
F27's uncertainty by reading `bot/src/main.rs:160-189`. Claims resting on
external evidence (crates.io versions as of 2026-07-24, Cloudflare edge
behavior) are flagged external-basis, not rejected; leptos-use and
reactive_graph claims were checked against vendored registry sources.

## Per-finding verdicts

### auth (`web/src/auth/`)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | Concurrent confirm requests bypass per-code attempt cap | critical | ADJUSTED (severity -> major) | Mechanism fully confirmed: SELECT (:361), Rust cap check (:374), mismatch UPDATE (:378-385) all on the bare pool; advisory lock covers the send path only. But failed-guess increments still commit, so concurrency is a bounded race-window multiplier over the cap of 10 (limited by in-flight/pool concurrency), not "effectively unbounded" vs a 1e6 keyspace. Recommendation caveat: increment-before-compare must use `>` not `>=`, else a correct 10th attempt after 9 failures is rejected |
| F2 | Email-squatting DoS via unverified add_email_address | major | CONFIRMED | Pending-row rejection at :428-436; insert_unverified_email (db.rs:2797) has no cap or expiry gating creation |
| F3 | No session ID rotation on login | minor | CONFIRMED | No cycle_id call; API exists in vendored tower-sessions-core 0.14 (session.rs:843) |
| F4 | Blocked-domain check leaks account existence | minor | CONFIRMED | Differential response at server.rs:302-307. Recommendation caveat: the uniform-rejection alternative would lock out existing verified users on blocked domains — inferior option |
| F5 | Transient DB error in get_current_user clears session | minor | CONFIRMED | `unwrap_or(false)` + session clear at :512-524 |
| F6 | Global 24h send cap over-counts historical sends | minor | CONFIRMED | sent_count never reset (upsert :215); the crate's own test pins accumulation (:1106). Recommendation caveat: option 1 (reset on rotation) UNDER-counts — hourly rotation yields 120 real sends/day vs the 50 cap; only the windowed-counter option is sound |
| F7 | Resend failure silent, consumes quota | minor | CONFIRMED | Commit :225 precedes send :229; error only logged (:101-103) |
| F8 | Turnstile fails open; unset secret silent | minor | CONFIRMED (detail) | Verifier-error fail-open IS warn-logged (:252); only the unset-secret path is truly silent — wording correction, minor stands |
| F9 | Emails not normalized (case/whitespace) | minor | CONFIRMED | Raw strings throughout; text PK (migration 005) + case-sensitive text UNIQUE (001:274) |
| F10 | add_email_address discards send-cap refusal | minor | CONFIRMED | `?` drops the success:false LoginResponse at :829 |
| F11 | Auth tokens never expire DB-side | minor | CONFIRMED | Pure existence check; db.rs test asserts a 40-day token still validates |
| F12 | Code gen: modulo bias / non-CT compare / plaintext | nit | CONFIRMED | Bias math checks out (2^32 mod 1e6 = 967296, ~0.023%) |
| F13 | confirm_login no shape validation | nit | CONFIRMED | No format checks vs sibling login() |
| F14 | New reqwest::Client per Turnstile call | nit | CONFIRMED | :239; shared client in context used at :883. Recommendation caveat: expect_context must be called in server-fn scope, client passed into the helper |
| F15 | Logout swallows invalidation error; no flush | nit | CONFIRMED | `let _ =` at :531-547; no session.flush() |

### crypto (`web/src/crypto.rs`)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F16 | Hardcoded fallback key when DATABASE_ENCRYPTION_KEY unset | major | CONFIRMED | default_key() fallback + presence-only check verified; prod k8s does mount a database-encryption-key secret, but the fail-open path is real and silent beyond one warn. Major stands |
| F17 | No AAD binding, no zeroize | nit | CONFIRMED | Both hardening gaps as described |

### admin (`web/src/admin.rs`)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F18 | reorder_bots not atomic | major | ADJUSTED (severity -> minor) | Non-atomic per-row loop and absence of a unique constraint (migration 013) verified — but impact is bot display ordering only, on an admin-only surface, self-repairable by re-dragging. Not a "clear defect" at major weight |
| F19 | create_bot MAX+1 race | minor | CONFIRMED | Read-modify-write at :134-145 |
| F20 | Short-key mask leaks whole key; fabricated sk- prefix | minor | CONFIRMED | Both cited sites; leans nit but minor accepted |
| F21 | No way to clear a provider API key | minor | CONFIRMED | No NULL-setting path; UI text says "leave blank to keep" |
| F22 | test_provider hardcodes gpt-4o-mini | minor | CONFIRMED | Literal at :497 |
| F23 | Upstream test body/headers uncapped | minor | CONFIRMED | Uncapped text() + all headers; 10s timeout bounds duration, not size |
| F24 | Test result attributed to wrong row (Action race) | minor | ADJUSTED (mechanism) | Code shape matches, but vendored reactive_graph 0.2.14 clears `input()` to None once in_flight hits 0 — the claimed cross-attribution cannot occur per strict source reading (which would also imply the result panel never renders; runtime semantics UNVERIFIABLE read-only). Bonus defect found: the `dispatched` counter is never incremented, so the is_latest guard is vacuous. Recommendation (key off value() alone) is valid under both readings |
| F25 | No server-side validation of bot/provider inputs | minor | CONFIRMED | All constraints client-side only |
| F26 | Admin page hard-fails on one undecryptable key | minor | CONFIRMED | Per-row `?` in list_providers; full-page error render at :1022-1025 |
| F27 | Bot rename/delete strands in-flight games | minor | ADJUSTED (severity -> major) | The finding's UNCERTAIN worker-fallback is resolved: bot/src/main.rs:171-187 silently skips the turn and returns Ok(()) (message acked, never retried) when the bot name is missing/disabled — rename/delete/disable permanently deadlocks games using that bot |
| F28 | Admin-gate boilerplate duplicated 15x | minor | CONFIRMED | Exactly 15 verbatim gates counted |
| F29 | Mutations don't verify rows_affected | minor | CONFIRMED | All cited helpers discard rows_affected() |
| F30 | update_bot_provider omits updated_at = now() | nit | REJECTED | `bot_providers` has NO updated_at column (migration 013:23-34, no later ALTER through 021) — nothing is omitted, and the recommended `updated_at = now()` would be a runtime SQL error. The secondary claim (bots/llm_providers have the column but no trigger) is accurate but changes nothing here |
| F31 | Redirect matches on error-message text | nit | CONFIRMED | contains("Admin access required") at :1005-1012 |
| F32 | Action-value unwraps in Effects | nit | CONFIRMED | 10 double-get-unwrap occurrences + 2 input-pairing variants |
| F33 | Local BotProviderRow alias shadows public struct | nit | CONFIRMED | Tuple alias inside test_bot_provider vs struct at :53-65 |

### db (`web/src/db.rs`)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F34 | undo_game keeps voided ratings; re-finish skips re-rating | major | CONFIRMED (strengthened) | Guard at :1554; undo_game (:1438-1449) touches no rating field; delete_game no-rewind comment at :1342. Reachability is STRONGER than the finding's UNCERTAIN framing: the undo server path (game/server_fns.rs:731-784) has no is_finished guard. Recommendation caveat: "recompute on next finish" alone double-counts unless game_type_users is also rewound |
| F35 | ~20 public DB functions untested | major | ADJUSTED (detail; major stands) | Coverage-gap thrust holds — 8/8 spot-checked functions have no test, concede_game has no 3+ player coverage — but two sub-claims are wrong: choose_colors (:5735-5780) and the ELO helpers (:5167-5190) ARE tested. Drop those from the finding and its recommendation |
| F36 | Redundant updated_at = NOW() on trigger-maintained tables | minor | ADJUSTED (detail; minor stands) | 16/18 listed lines confirmed redundant, but :1357/:1363 target game_proposals (migration 015: updated_at columns, NO trigger — only migration-001 tables have one) — those manual sets are REQUIRED. A sweep following the finding's line list verbatim would break game_proposals.updated_at |
| F37 | is_finished=false with finished_at set possible | minor | CONFIRMED | Unconditional `is_finished = $2` + COALESCE at :1716; test :4685-4711 never asserts is_finished; dangling "see report" comment at :4684 |
| F38 | Zero-change results defeat rating idempotency guard | minor | CONFIRMED | Both write loops skip change==0; equal-rating exact tie yields all-zero changes and an unarmed guard |
| F39 | send_friend_request opposite-direction 23505 race | minor | CONFIRMED | friends_pair_key is LEAST/GREATEST unique (010_friends.sql:7-9), so opposite-direction inserts collide raw |
| F40 | friend_recent_visible_game N+1 | minor | CONFIRMED | Per-candidate is_game_visible_to_user calls. Recommendation caveat: inlining duplicates a deliberately centralized visibility predicate — keep a cross-ref if inlined |
| F41 | insert_game_logs_tx row-at-a-time | minor | CONFIRMED | Sequential awaited INSERTs in tx |
| F42 | db.rs 6.4k-line grab-bag | minor | CONFIRMED | As described; well-sectioned |
| F43 | build_game_type_user nil-id sentinel | minor | CONFIRMED | Synthetic 1200-rating row with Uuid::nil() on NULL join |
| F44 | is_turn_at reset for continuing-turn players | nit | CONFIRMED | Trigger fires only false->true (001:454-458); :1746 overwrites every command. Borderline minor (is_turn_at orders the 22d digest, :2934) — nit kept since the finding itself allows "may be intended" |
| F45 | is_user_admin returns sqlx::Result | nit | CONFIRMED | Sole non-test sqlx::Result in the file |
| F46 | generate_unique_username race (mitigated) | nit | CONFIRMED | users_name_lower_key exists (009:41) as claimed |
| F47 | Interval via string interpolation of bound param | nit | CONFIRMED | `($1 || ' seconds')::interval`; make_interval fix valid |
| F48 | No app-level self-request guard | nit | CONFIRMED | DB CHECK only. Recommendation caveat: the suggested silent-Ok would break the existing Err assertion at :3317 — the test must change deliberately with it |
| F49 | choose_colors clones prefs vec each pass | nit | CONFIRMED | Suggested rewrite borrow-checks, behavior-identical |
| F50 | apply_rating_changes all-pairs idiom | nit | CONFIRMED | Suggested slice form equivalent |
| F51 | Test-quality nits (3 bundled) | nit | ADJUSTED (part 1 wrong; nit stands) | Part (1) rejected: the fixture DOES make the caller a game_players row, so self-exclusion is mutation-covered and the test name does not over-promise — rename unneeded. Parts (2) 3-player visibility gap and (3) format!-built count_rows confirmed |

### main / router / session / k8s

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F52 | Session cookies lack Secure flag in prod | major | CONFIRMED | SECURE_COOKIE grep-confirmed in exactly 2 places (session.rs:32 + a completed plan doc); absent from every k8s manifest and .env.template — prod runs with_secure(false). Recommendation caveat: setting it in k8s base/ also forces the dev overlay (dev kustomization includes ../base/web); prefer default-secure-in-code with explicit opt-out, or a prod-only patch |
| F53 | Bot command consumer unsupervised | major | CONFIRMED (strengthened) | Spawn at main.rs:55-74 discards the JoinHandle; Err only logged; a clean Ok(()) stream-end exit (game/mod.rs:322-324) is not even logged; /healthz is a static "OK" |
| F54 | No rustls CryptoProvider installed | minor | CONFIRMED | No install anywhere in web/src; both aws-lc-rs and ring in the workspace Cargo.lock; CODING.md:408-423 documents the rule. Which provider wins in web's graph stays UNVERIFIABLE without cargo tree (no builds) — the defensive recommendation stands regardless |
| F55 | Graceful shutdown misses WS/background tasks | minor | CONFIRMED | Detached spawns as described |

### nats (`web/src/nats.rs`)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F56 | max_deliver=3 exhaustion strands silently | minor | CONFIRMED (uncertainty resolved; minor stands) | The consumer never term()s but acks all poison classes (parse error game/mod.rs:277, UserError :304-314, Conflict :299); only transient `Other` is left unacked — stranding is limited to messages failing transiently 3x. Neither the finding's downgrade-to-nit nor upgrade-to-major condition fires |
| F57 | get_or_create never reconciles config drift | minor | CONFIRMED | Existing object returned untouched |
| F58 | ack_wait=5min may be shorter than processing | minor | ADJUSTED (severity -> nit) | Ack cadence resolved: full-process-then-ack with a hard 10s shared-client HTTP timeout and bounded retries — exceeding 5 minutes is implausible, so the risk is theoretical |

### websocket

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F59 | /ws unauthenticated site-wide firehose | minor | CONFIRMED | ws_handler takes no Session; two core-NATS subscriptions (`game.>`, `proposal.>`) per connection |
| F60 | Client open() on visibilitychange tears down healthy sockets | minor | CONFIRMED (evidence upgraded) | Verified against vendored leptos-use 0.19.0 source (no longer external-basis): open() -> connect() unconditionally closes any existing socket; the old socket's onclose schedules a gratuitous ~3s reconnect. Recommendation caveat: the gate must read `ready_state.get_untracked()` inside the non-reactive event listeners |
| F61 | Tungstenite default message/frame limits | nit | CONFIRMED | No max_message_size/max_frame_size on on_upgrade |
| F62 | No dead-connection detection beyond send failure | nit | CONFIRMED | Pings sent, pongs never checked |

### import_game / Cargo.toml

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F63 | Unbounded file read in import_game | nit | CONFIRMED | read_to_string, dev-only tool |
| F64 | gloo-net unused | minor | CONFIRMED | Zero gloo_net references in src/ or tests/ |
| F65 | tokio net/time features undeclared | minor | CONFIRMED | tokio::net at main.rs:103/184, tokio::time in websocket.rs and sweep.rs; feature list has neither |
| F66 | futures-util non-optional but ssr/test-only | nit | CONFIRMED | Usage all ssr-side/tests. Recommendation caveat: making it optional breaks `cargo test` without `--features ssr` — tests/websocket_hygiene.rs:14 and tests/nats_bot_eventing.rs:19 have ungated top-level `use futures_util`; needs a dev-dependency or [[test]] required-features alongside |
| F67 | Dependency currency spot-check | nit | UNVERIFIABLE (external basis) | Version-currency claims rest on crates.io as of 2026-07-24 (outside the snapshot); all in-repo declared versions and documented holdbacks/pins match the finding's description |

## Summary

- Findings verified: 67
- CONFIRMED: 57, ADJUSTED: 8 (F1, F18, F24, F27, F35, F36, F51, F58),
  REJECTED: 1 (F30), UNVERIFIABLE: 1 (F67). 66/67 findings survive.
- Corrected tallies for the unit: 0 critical / 8 major / 36 minor /
  22 nit. (Original file has no header tally; Lead recount of its blocks
  was 1 critical / 7 major / 37 minor / 22 nit = 67. Net changes: F1
  critical -> major, F18 major -> minor, F27 minor -> major, F58
  minor -> nit, F30 rejected.)
- Lead spot-checked every REJECTED/ADJUSTED verdict directly against the
  snapshot (F1 server.rs:340-390 increment semantics; F30 migration
  013:23-34 + admin.rs:446-457; F27 bot/src/main.rs:160-189 silent skip;
  F35 choose_colors/ELO tests at db.rs:5735/:5168; F36 db.rs:1352-1367 +
  migration 015 no-trigger; F56/F58 via game/mod.rs:251-325). W4's F56
  resolution and W2's F27 resolution were independently re-read by the
  Lead in full.

## Notable corrections

- F1 (confirm-cap race) downgraded critical -> major: the mechanism is
  real (no lock or transaction on the validate path), but failed-guess
  increments do commit, so concurrency buys a bounded multiplier over
  the cap of 10 — not an effectively unbounded brute-force against the
  1e6 keyspace. The original finding itself hedged on edge throttling.
- F27 (bot rename/delete) upgraded minor -> major: the original review
  left the bot worker's fallback UNCERTAIN; bot/src/main.rs:171-187
  shows a missing/disabled bot name is silently skipped and the message
  acked — in-flight games using that bot deadlock permanently.
- F30 (update_bot_provider updated_at) REJECTED — the only outright
  rejection: `bot_providers` has no updated_at column at all, so nothing
  is omitted, and the recommended fix would be a runtime SQL error.
  Fourth recommendation-validity catch across the verified units.
- F36 (updated_at sweep) carries a live hazard: 2 of the 18 listed lines
  target game_proposals, which has NO trigger — a sweep following the
  finding's line list verbatim would silently stop maintaining
  game_proposals.updated_at.
- F6's first recommended fix (reset sent_count on code rotation) would
  under-count the global 24h Resend cap (hourly rotation = 120 real
  sends/day vs the 50 cap); only the windowed-counter alternative is
  sound. Similarly flawed alternatives flagged inside confirmed
  findings: F1 (`>=` vs `>` off-by-one), F4 (uniform rejection locks out
  verified users), F48 (breaks an existing test assertion), F52 (base
  env var forces secure cookies on dev), F66 (breaks non-ssr cargo
  test), F60/F14 (reactive/context scope details).
- F24 (admin Action race): per vendored reactive_graph 0.2.14 source the
  claimed cross-attribution cannot occur (input() clears on completion);
  runtime semantics unverifiable read-only, but a real adjacent defect
  surfaced — the `dispatched` guard counter is never incremented. The
  finding's recommendation remains the right fix either way.
- F56/F58 (NATS UNCERTAINs) both resolved by reading the consumer:
  poison messages are acked (F56 stays minor; stranding limited to
  3x-transient failures) and processing is bounded well under ack_wait
  (F58 drops to nit).

Overall assessment: a high-accuracy unit — 66/67 findings survive, and
every major/critical claim reproduced mechanically. The K3 review's main
weaknesses were severity calibration at the extremes (one critical
overstated, one minor understated after its uncertainty resolved) and
recommendation validity: this unit contributed one flat-out invalid fix
(F30) plus an unusually high count of flawed alternative fixes flagged
inside otherwise-confirmed findings, which matters for the remediation
plan more than for the findings themselves.
