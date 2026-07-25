# web-server unit — Lead review log

Lead session started 2026-07-24. Snapshot: `/home/beefsack/Development/brdgme-review-snapshot`
(HEAD `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`). Review-only; no code edits.

## Per-module LOC split (snapshot, paths relative to `rust/web/`)

| Module | File | LOC |
|---|---|---:|
| main | `src/main.rs` | 225 |
| router | `src/router.rs` | 216 |
| state | `src/state.rs` | 46 |
| config | `src/config.rs` | 6 |
| db | `src/db.rs` | 6,380 |
| nats | `src/nats.rs` | 97 |
| error | `src/error.rs` | 16 |
| crypto | `src/crypto.rs` | 69 |
| admin | `src/admin.rs` | 2,299 |
| auth | `src/auth/mod.rs` | 6 |
| auth | `src/auth/server.rs` | 1,530 |
| auth | `src/auth/session.rs` | 94 |
| auth | `src/auth/blocked_domains.rs` | 8,152 |
| websocket | `src/websocket.rs` | 235 |
| websocket | `src/websocket_client.rs` | 88 |
| import_game | `src/bin/import_game.rs` | 36 |
| **Total** | | **19,495** |

Total exceeds the ~15k guideline, BUT `auth/blocked_domains.rs` (8,152) is a
vendored static data table (disposable-email domain list) with ~zero reviewable
logic. **Effective reviewable LOC ≈ 11,343** — under budget; proceeding without
splitting. `blocked_domains.rs` gets a skim-only check (data freshness/vendoring).

## Worker plan (serial per orchestrate skill)

1. **auth-crypto**: `auth/server.rs`, `auth/session.rs`, `auth/mod.rs`,
   `crypto.rs` — deepest scrutiny (login tokens, sessions, AES-GCM).
2. **admin**: `admin.rs` (2,299) — admin gating, LLM provider/bot config,
   crypto usage.
3. **db-a**: `db.rs` first half (lines 1–~3,200).
4. **db-b**: `db.rs` second half (~3,200–6,380).
5. **infra**: `main.rs`, `router.rs`, `state.rs`, `config.rs`, `nats.rs`,
   `error.rs`, `websocket.rs`, `websocket_client.rs`, `bin/import_game.rs`,
   skim `blocked_domains.rs`, web `Cargo.toml` ssr deps.

## Dispatch / return log

- 2026-07-24: LOC split computed (above). Unit proceeds unsplit (effective ~11.3k).
- Worker 1 (auth-crypto, agent-46) RETURNED. Raw: `findings/raw/web-server-auth-crypto.md`.
  Full coverage of auth/server.rs, auth/session.rs, auth/mod.rs, crypto.rs.
  Headlines: CRITICAL concurrent-confirm bypass of 6-digit-code attempt cap
  (check-then-act, server.rs:354-390); MAJOR email-squatting DoS
  (server.rs:789-831); MAJOR insecure default crypto key warn-only
  (crypto.rs:42-57); ~10 minor (session fixation, enumeration via blocked-domain
  check, Secure-cookie default off, transient-DB-error mass logout, etc.).
  Pending Lead verification against snapshot before curation.
- Worker 2 (admin, agent-47) RETURNED. Raw: `findings/raw/web-server-admin.md`.
  All 2,299 lines read. Headlines: MAJOR reorder_bots non-atomic multi-UPDATE
  (admin.rs:184-197); minors: create_bot display_order race (:136), API-key mask
  leaks short keys + fabricates sk- prefix (:228-236), test_provider hardcodes
  gpt-4o-mini (:497), uncapped upstream bodies to client, wrong-row test
  attribution race (:1424), admin-gate boilerplate 15x, one bad key row kills
  whole admin page, bot rename/delete strands in-flight games, no server-side
  input validation. CLEAN: all 15 admin fns properly gated server-side, no
  plaintext key exposure, SQL parameterized, no panics in request paths.
- Worker 3 (db-a, agent-48) RETURNED. Raw: `findings/raw/web-server-db-a.md`.
  Covered db.rs lines 1-3312. Headlines: MAJOR(uncertain) undo_game vs
  apply_rating_changes rating desync (db.rs:1407-1463 x 1536-1680); minors:
  pervasive redundant manual updated_at sets vs trigger (~20 sites),
  zero-change rating_change NULL defeating idempotency guard (:1646-1677),
  send_friend_request mutual-request race (:1877-1925), N+1 in
  friend_recent_visible_game (:2316-2342), concede_game 2-player constraint
  only debug_assert (:1308), test gaps (choose_colors, ELO helpers). CLEAN: no
  injection, no panics in non-test fns, multi-statement writes transactional,
  optimistic concurrency guard correct.
- Worker 4 (db-b, agent-49) RETURNED. Raw: `findings/raw/web-server-db-b.md`.
  Range 3313-6380 turned out to be ENTIRELY the test module (non-test code
  ends ~2959). Headlines: MAJOR ~20 public DB fns untested incl
  find_active_turn_games, generate_unique_username, is_user_admin,
  set_user_name, pref-color accessors; minor: test at :4685 enshrines possible
  un-finish desync (is_finished=false + finished_at set) with dangling "see
  report" comment; minor: no DB-side auth-token expiry (:5140, 40-day token
  still validates) - cross-ref worker 1's user_auth_tokens GC nit. Reconcile
  with worker 3's undo/rating finding during curation.
- Worker 5 (infra, agent-50) RETURNED. Raw: `findings/raw/web-server-infra.md`.
  Full coverage of main/router/state/config/nats/error/websocket/ws_client/
  import_game + Cargo.toml. Headlines: MAJOR SECURE_COOKIE defaults false and
  set NOWHERE (not in k8s manifests) (session.rs:32-36); MAJOR bot command
  consumer task unsupervised - silent pod-wide bot death (main.rs:55-74);
  minors: JetStream max_deliver=3 no DLQ, stream/consumer config drift never
  reconciled, /ws unauthenticated firehose to anonymous clients, leptos-use
  open() tears down healthy sockets on visibilitychange, no rustls
  CryptoProvider guard, gloo-net unused dep, async-nats/svix behind. CLEAN:
  router middleware ordering, startup panics, publish+flush pattern,
  state/config/error.
- All 5 workers returned. Lead now verifying critical/major candidates against
  the snapshot before curating.

## Lead verification pass (against snapshot, all confirmed by direct read)

- VERIFIED critical: validate_confirmation_code check-then-act attempt-cap
  bypass (server.rs:354-390) - read in full; SELECT/compare/UPDATE are indeed
  separate unlocked statements.
- VERIFIED major: email-squatting - pending branch returns uniform error even
  with a valid code (server.rs:428-436 read).
- VERIFIED major: crypto default key warn-only (crypto.rs:42-57 read).
- VERIFIED major: reorder_bots loop without transaction (admin.rs:184-197 read).
- VERIFIED major: undo_game does not touch rating_change/game_type_users
  (db.rs:1407-1463 read) and apply_rating_changes guard trips on any
  rating_change (db.rs:1554 read). Kept major with reachability caveat
  (finish-then-undo depends on engine can_undo).
- VERIFIED major: SECURE_COOKIE - grep across whole snapshot: only session.rs
  and an old plan doc; not in k8s manifests or .env.template. Merged worker 1's
  minor + worker 5's major into ONE major finding.
- VERIFIED major: unsupervised bot consumer (main.rs:55-74 read).
- VERIFIED major: test-coverage gap - grep confirms find_active_turn_games,
  is_user_admin, set_user_name, generate_unique_username have no test refs.
- VERIFIED minor: update_game_command_success writes is_finished=$2
  unconditionally (db.rs:1716) - un-finish desync plausible; kept minor.
- RECONCILED trigger question: update_updated_at triggers exist ONLY on
  migration-001 tables; bots/llm_providers/bot_providers (migration 013) have
  none. So worker 3's "redundant updated_at" finding (db.rs, 001 tables) and
  worker 2's "update_bot_provider missing updated_at" nit (013 table) are BOTH
  valid and non-contradictory.
- MERGED: user_auth_tokens no-expiry nit (worker 1) + db-b :5140 test finding
  into one minor.
- Now curating findings/web-server.md.

## Curation complete (2026-07-24)

Wrote `findings/web-server.md`: 67 findings (1 critical / 7 major / 37 minor
/ 22 nit). By category: correctness 30, quality 28, simplicity 3,
consistency 4, dependencies 2. Grouped under ## auth, ## crypto, ## admin,
## db, ## main/router/state/config/error, ## nats, ## websocket,
## import_game, ## Cargo.toml, ## blocked_domains (clean). All critical/major
findings were verified by the Lead directly against the snapshot before
inclusion. Unit complete.
