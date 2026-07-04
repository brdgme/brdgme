# Review findings 2026-07-04

**Status:** Resolved 2026-07-04

Source: docs/REVIEW-2026-07-04.md (full context, rationale, and file/line
references there). HIGH items block prod cutover (Phase 16). All are
delegable as specified unless noted.

### HIGH - fix before cutover

- [x] **`confirm_login` code lookup unscoped + unthrottled**
      (`rust/web/src/auth/server.rs:171`): the 6-digit code is matched
      globally (`WHERE login_confirmation = $1`) with no email/user scoping
      and no rate limit on the confirm step. Fix: change the server fn to
      `confirm_login(email, token)` scoped to that user's row; rate-limit
      confirm attempts using the existing `auth/rate_limit.rs` limiter
      infra; update the Leptos login form (it already collects the email in
      step 1 - the "I already have a login code" path must also ask for
      it). Tests: wrong-email + right-code rejected; confirm rate limit
      trips after burst.
- [x] **Bot prompt leaks private logs** (`rust/bot/src/main.rs:389`):
      `load_bot_context` fetches `game_logs` with no `is_public` /
      `game_log_targets` filter, so other players' private logs reach the
      LLM. Fix: filter public OR targeted to the bot's own `game_players.id`
      (same predicate as `db::get_game_logs`; needs the bot's game_player id
      which the trigger query can join).
- [x] **No probes or resource requests on any Deployment** (`k8s/base/**`):
      add readiness + liveness probes to `web` (TCP :3000 or a `/healthz`
      route), `bot` (TCP :4000), and game services (TCP :80); add resource
      requests/limits using the RSS figures quoted in VISION.md. Without
      readiness probes, `web` rolling updates (replicas: 2) drop traffic.
- [x] **Delete the unused REST game handlers** (`rust/web/src/game/server.rs`):
      verified 2026-07-04 that nothing calls `/api/game/*` - the Leptos
      frontend uses server fns, the bot uses only
      `/api/internal/game/{id}/command`, legacy clients use `rust/api`.
      Delete `create_game`, `get_game`, `play_command`, `undo_game`,
      `mark_read`, `concede_game`, `restart_game` and their routes/tests;
      keep `internal_play_command`. This also resolves the ~500-line
      duplication with `server_fns.rs` without building an abstraction.
      Resolved 2026-07-04: deleted the seven unused handlers, their request
      structs, and the three tests exercising them; kept
      `internal_play_command`, its route, and its 4-assertion auth test
      (correct/wrong/missing key, `INTERNAL_API_KEY` unset). `server.rs` is
      now ~170 lines. `cargo test -p web --features ssr` passes (43 tests,
      down from 46).

### MED

- [x] **reqwest timeouts**: `reqwest::Client::new()` has no timeout in
      `rust/web/src/main.rs` and `rust/bot/src/main.rs`. Web: ~10s (game
      service calls). Bot: generous overall timeout, or per-request timeout
      on the LLM call (minutes) with a shorter one for game service/monolith
      calls.
- [x] **Move logs written outside the move transaction**
      (`game/mod.rs::execute_command`): `create_game_logs` commits after
      `update_game_command_success` - a crash between them silently loses
      the move's logs. Fold log insertion into the same transaction.
- [x] **Restart flow non-atomic** (`server_fns.rs::restart_game`): new game
      creation and the `restarted_game_id` UPDATE are separate transactions;
      a failure in between leaves the old game restartable again. Wrap in
      one transaction.
- [x] **WS hardening** (`rust/web/src/websocket.rs`): (a) add a periodic
      server-side ping (~30s interval) so idle connections survive LB idle
      timeouts - also de-risks the Phase 14 DO LB prerequisite; (b) note for
      the Phase 17 NATS design: subscribe per-game/per-user instead of the
      current `game.*` firehose to every client.
      Resolved 2026-07-04: `handle_socket` now uses `tokio::select!` over the
      pubsub stream, a 30s ping interval, and the (previously dropped) WS
      receiver so pongs/close frames are processed; TODO(Phase 17 NATS)
      comment added above the `psubscribe("game.*")` call.
- [x] **websocket.rs tests**: zero coverage of legacy payload shape and the
      per-player private-log filtering (info-leak surface) that web-legacy
      relies on during Phase 16 side-by-side. `#[sqlx::test]` + Redis
      SUBSCRIBE assertions using the existing test infra.
      Resolved 2026-07-04: two `#[sqlx::test]` tests in `websocket.rs` assert
      the legacy `GameUpdate` JSON shape on `game.<id>` and per-player
      private-log filtering on the `user.<token>` channels via real Redis
      SUBSCRIBE.
- [x] **Swap `dotenv` -> `dotenvy`** (`rust/web/Cargo.toml`): dotenv is
      unmaintained (RUSTSEC-2021-0141). Drop-in replacement.
      Resolved 2026-07-04: `web/Cargo.toml` dependency and feature flag, and
      the `dotenv::dotenv()` call in `web/src/main.rs`, switched to
      `dotenvy`; only crate in the workspace that depended on `dotenv`.
- [x] **CI lint gates** (`.github/workflows/ci.yml`): add
      `cargo fmt --check` and `cargo clippy --workspace -- -D warnings` to
      `test-rust`.
      Resolved 2026-07-04: added `cargo fmt --all -- --check` plus clippy
      split as `--workspace --exclude web` / `-p web --features ssr`
      (mirroring the existing test split); toolchain step installs
      rustfmt/clippy. Fixed the mechanical clippy findings across the
      workspace. Also regenerated the stale `rust/web/.sqlx` offline query
      cache (4 files with whitespace-drifted query text) which had CI red on
      `cargo test -p web --features ssr` since before this change.
- [x] **CI build jobs -> matrix**: `build-web` and `build-rust-games`
      hand-roll 5 metadata+build-push pairs; convert to the matrix pattern
      `build-legacy` already uses.
      Resolved 2026-07-04: both jobs converted to `strategy.matrix.include`
      (image/target pairs, `file: rust/Dockerfile` constant); kept as two
      jobs to preserve job names/history. No behaviour change.
- [x] **db.rs row-mapping duplication**: shared helper for the duplicated
      GamePlayer row->struct blocks in `find_game_extended` /
      `find_active_games_for_user`; plus small helpers for the repeated
      Status destructure and the broadcast+trigger epilogue (only if the
      REST-handler deletion above leaves them still repeated).
      Resolved 2026-07-04: added `build_game_player_from_row` (db.rs),
      `status_fields` and `broadcast_and_trigger` (game/mod.rs).
      `execute_command`'s epilogue kept inline (it warns on reload failure,
      the other sites don't); `concede_game`/old-game rebroadcast left as-is
      (different control flow). fmt/clippy clean, 45/45 web tests pass.

### LOW (batch opportunistically) [Resolved 2026-07-04]

- [x] sqlx: drop unused `chrono` feature; bump `tower` 0.4 -> 0.5 (test-only
      usage); trim cargo-leptos template comments from `rust/web/Cargo.toml`.
- [x] `k8s/prod/kustomization.yaml`: `bases:` -> `resources:` (before
      kubeconform lands).
- [x] Gateway: add a port-80 HTTP listener with RequestRedirect so
      `http://brdg.me` redirects instead of hanging.
- [x] Self-host the Source Code Pro font (currently Google Fonts in
      `app.rs::shell` - violates the network-hostile-environments
      principle).
- [x] Bot: rename "Ollama" log messages to provider-neutral wording.
- [x] Operator: scope the deletion-path UPDATE by `(game_type_id, name)` to
      match the upsert key; CI: single-run triggers + concurrency group.
- [x] `concede_game` (db.rs): comment/debug_assert the 2-player assumption.
- [x] Tag `websocket.rs` legacy structs with a `DELETE at Phase 16` marker.

