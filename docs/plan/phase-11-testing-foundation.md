# Phase 11: Testing Foundation

**Status:** Complete (completed 2026-07-04)

**Goal:** Build test coverage over the critical orchestration, data, and auth
paths so routine work can be delegated to cheaper models/agents safely. The
tests are the guardrail: they must fail when core game flows break, and they
must run in CI on every push.

**Current state (audited 2026-07-02):** good unit coverage in the libraries
(`brdgme_markup` parser/transform, `brdgme_game` command parser + suggest,
game crate logic, bot prompt rendering - 14 tests). `rust/web` has exactly one
test (`game/client.rs::test_game_client_contract`, in-process mocked game
service). Zero coverage of `db.rs`, `auth/`, `game/mod.rs::execute_command`,
Axum handlers, server fns, and `websocket.rs` - the code agents touch most.
CI already runs `cargo test --workspace --exclude web` and
`cargo test -p web --features ssr` but provides no Postgres, so DB-backed
tests cannot exist yet.

**Stack decisions:**
- **DB tests:** `#[sqlx::test]` (sqlx 0.8 built-in). Gives each test an
  isolated, auto-created database with `rust/web/migrations/` applied.
  Needs a live Postgres via `DATABASE_URL` at test runtime (compilation stays
  `SQLX_OFFLINE=true`).
- **Game service mock:** in-process Axum server returning canned `Response`
  JSON - the pattern already used in `game/client.rs`. No new dependency.
  Never call real game services from `rust/web` tests (except E2E).
- **Broadcast:** tests that exercise `execute_command` need a
  `GameBroadcaster`; run a Redis service container in CI (swaps to NATS in
  Phase 16 with a one-line test-env change).
- **LLM:** never called in any test. Bot loop integration tests are deferred
  to Phase 13 (the NATS rewrite restructures the loop; tests written now
  would be throwaway).
- **E2E:** Playwright driving a real compiled site. This is the only layer
  that catches SSR/hydration panics - the project's most dangerous known
  failure mode (see `docs/CODING.md`) - because they only manifest on hard
  page loads in a real browser.

### 11.1 CI: backing services for integration tests

- [x] Add `postgres:17` and `redis:7` service containers to the `test-rust`
      job in `.github/workflows/ci.yml`, matching `DATABASE_URL`
      (`postgres://postgres:postgres@localhost/brdgme`) and `REDIS_URL`.
- [x] Keep `SQLX_OFFLINE=true` for compilation; `#[sqlx::test]` uses
      `DATABASE_URL` at runtime and manages its own per-test databases.
- [x] Verify a trivial `#[sqlx::test]` passes in CI before building out the
      suite. (`db::tests::migrations_apply_and_pool_connects` in
      `rust/web/src/db.rs`; confirmed passing in CI run 28654432277.)

### 11.2 DB layer tests (`rust/web/src/db.rs`, `#[sqlx::test]`)

Build a small fixture helper first (create user / game type + version /
game with N human players and M bot players), then cover:

- [x] `create_game_with_users`: positions and colors assigned sequentially;
      creator + opponents rows created; bot slots create `game_bots` rows with
      `user_id = NULL` and `game_bot_id` set (XOR constraint holds); initial
      `is_turn` matches `whose_turn` from the game service response.
- [x] `find_game_extended`: round-trips a mixed human/bot game; user fields
      populated for humans, `game_bot` for bots; missing `game_type_users`
      row yields default rating 1500; nonexistent game id returns `Err`, not
      a panic.
- [x] `find_active_games_for_user`: user in several games gets correctly
      grouped results (regression guard for the `db.rs:407` `last_mut`
      grouping logic); finished games excluded; user with no games returns
      empty vec; `is_turn`/`is_read` flags correct per game. Player order is
      randomized by `create_game_with_users`, so the test asserts turn state
      by position, not by creator/opponent role.
- [x] `update_game_command_success`: on Active status writes `whose_turn`,
      `is_turn_at`, `last_turn_at`, `is_eliminated`, `points`,
      `undo_game_state` per player; on Finished writes `is_finished`,
      `finished_at`, `place`; `finished_at` COALESCE only guards
      `is_finished = false` calls - repeated `is_finished = true` calls do
      advance the timestamp (tested as-is; not changed).
- [x] `undo_game`: `game_state` restored from `undo_game_state`; undo state
      cleared for all players; turn flags recomputed from Status response;
      `{{player N}}` undo log row inserted.
- [x] `concede_game`: `is_finished`/`finished_at` set (placings + rating
      assertions deferred to Phase 12 as planned).
- [x] `create_game_logs` + `get_game_logs` + `get_all_game_logs`: public log
      visible to every player; private log (`to` targets) visible only to
      targets via `game_log_targets`; ordering by insertion time.
- [x] Auth queries (`auth/`): login confirmation token stored and validated.
      **Deviations from the original spec, tested as the code actually
      behaves:** default `game_type_users` rating is 1200 when a row exists,
      1500 only when no row exists at all (both cases tested); login
      confirmation token expiry is 1 hour, not 29/31 days; there is no DB-side
      30-day session `created_at` check - that window is enforced by
      `tower_sessions` cookie config, not the DB layer, so no DB test for it.

### 11.3 Game orchestration tests (`game/mod.rs::execute_command`) [Complete]

Combine `#[sqlx::test]` DB + in-process mock game service + real
`GameBroadcaster` against test Redis. Critical cases - each asserts both the
returned result AND the resulting DB state:

- [x] Happy path: valid command → state saved, logs persisted, turn flags
      updated, `can_undo`/`undo_game_state` stored.
- [x] Not the player's turn → `Err`, game row unchanged.
- [x] Game already finished → `Err`, game row unchanged.
- [x] Game service returns `UserError` → error propagated verbatim, no DB
      write.
- [x] Game service returns `SystemError` / malformed JSON → error, no DB
      write.
- [x] `remaining_input` non-empty → `Err`, no DB write.
- [x] Play response with `Finished` status → `place`, `is_finished`,
      `finished_at` persisted (+ ratings once Phase 12 lands).
- [x] `trigger_bot_turns` with `BOT_SERVICE_URL` unset → no-op, no error.
- [x] After Phase 12: rating updates asserted here too (human-only game
      rated; game with a bot player not rated). Resolved 2026-07-04:
      rating assertions added to `finished_status_persists_placings`
      (+16/-16 at K=32 from the 1200 default; game_type_users 1216/1184)
      and new `finished_game_with_bot_player_is_not_rated` test.
- [x] After optimistic locking lands: concurrent-write conflict returns the
      conflict error and preserves the first write.

Implemented as `rust/web/src/game/mod.rs::tests` (now 11 tests including
the Phase 12 rating assertions). All items complete.

### 11.4 Handler auth tests (Axum `tower::ServiceExt::oneshot`) [Complete]

- [x] `POST /api/internal/game/{id}/command`: correct `X-Internal-Key` →
      executes; wrong key → 401; missing key → 401; `INTERNAL_API_KEY` env
      unset → rejects all.
- [x] ~~`GET /api/game/{id}` with no session → 401.~~ Superseded 2026-07-04:
      the `/api/game/*` REST handlers were deleted as unused (nothing calls
      them; the Leptos frontend uses server fns). No longer applicable.
- [x] ~~`POST /api/game/{id}/command` with no session → 401; with a session
      for a non-player → 403.~~ Superseded 2026-07-04, same reason.
- [x] Login flow logic (`auth/server.rs`, function-level): invalid email
      rejected; valid email creates a user and sets a 6-digit confirmation
      token; wrong code and expired confirmation are both rejected before the
      session step. `login`/`confirm_login` invoked directly in a
      `leptos::reactive::owner::Owner` scope with `PgPool` provided via
      context (no HTTP layer needed - the `#[server]` macro body is callable
      as a plain async fn). `confirm_login`'s "creates a session user" path
      needs a Leptos request-scoped `Parts` context (only available in a real
      request), so it isn't asserted end-to-end here; E2E covers that.

Implemented as `rust/web/src/game/server.rs::tests` (1 test) and
`rust/web/src/auth/server.rs::tests` (4 tests). Added `tower = { features =
["util"] }` for `oneshot`/`ServiceExt` in tests. The `game/server.rs` test
count dropped from 4 to 1 on 2026-07-04 when the unused `/api/game/*` REST
handlers (and their tests) were deleted; only the
`internal_play_command` auth test remains.

### 11.5 Game contract regression harness (Rust game crates) [Complete]

A generic test helper (in `brdgme_cmd` as a `test-support` feature or a small
dev-dependency crate) that drives any `Gamer` implementation through the full
contract, instantiated per game crate:

- [x] `PlayerCounts` returns non-empty; `New` succeeds for every advertised
      count and fails for an unadvertised one.
- [x] `New` → serialize state → `Status` round-trip: same status, renders
      non-empty, `player_renders` length matches player count.
- [x] `Play` with garbage input returns `UserError` (never `SystemError`,
      never a panic).
- [x] `Rules` returns non-empty text.
- [x] Instantiate for `acquire-1`, `lost-cities-1`, `lost-cities-2`,
      `lords-of-vegas-1`. (Go games excluded - covered by their own
      `go-test` CI job and the frozen contract.)

Implemented as `brdgme_cmd::test_support::assert_gamer_contract` (`rust/lib/cmd/src/test_support.rs`),
gated behind a `test-support` Cargo feature (not compiled into release
builds) since `brdgme_cmd` is already a normal dependency of every game
crate - a separate dev-dependency-only crate would have duplicated that
wiring for no benefit. Instantiated as `tests/contract.rs` in `acquire-1`,
`lost-cities-1`, `lost-cities-2`, and `lords-of-vegas-1`, all green.

While wiring up `lords-of-vegas-1` the harness caught a real gap:
`Gamer::rules()` returned `String::new()` (unlike the other three crates,
this game had no `RULES.md` at compile time). Added
`rust/game/lords-of-vegas-1/RULES.md` and wired it via
`include_str!("../RULES.md")` per `docs/CODING.md` "Embed rules at compile
time". The rules text documents only what the current implementation plays
out (building casinos on owned lots, boss-tie rerolls, turn passing) - the
game engine doesn't yet implement sprawl/remodel/reorg/gamble/raise,
scoring, or an end-of-game trigger (the `Command` parser never wires those
variants in, and `Game::command`'s match arms for them are `unimplemented!()`
dead code), so the doc says so explicitly under "Implementation status"
rather than describing rules the code doesn't play.

### 11.6 Frontend/page testing (redesigned 2026-07-04)

**Direction change (2026-07-04, second design pass):** the original scope
was a Playwright E2E suite covering full game flows in the browser. A
working implementation was built (see "Committed state" below)
and immediately demonstrated the classic E2E failure modes: multi-context
WebSocket choreography races, re-render timing races on the command input,
browser-provisioning friction (Nix read-only `PLAYWRIGHT_BROWSERS_PATH`
vs Playwright's downloaded chromium needing system libs), and 60-120s
timeout-style debugging. Michael's judgement: he does not want slow, flaky,
heavy-dependency tests in this project.

**Revised strategy - two layers replacing the one big E2E suite:**

The only thing that genuinely requires a real browser is client-side
hydration (hydration mismatches and WASM panics only manifest on a hard
page load in a browser, surfacing via `console_error_panic_hook`).
Everything else the old scenario list covered is already tested at the
Rust layer (11.2-11.4: commands, turn enforcement, undo, concede,
restart, broadcast payloads) or can be covered without a browser:

- **11.6a - in-process SSR page tests (no browser; primary layer).**
  `#[sqlx::test]` + `tower::ServiceExt::oneshot` against the real
  Axum/Leptos router (built the same way `main.rs` builds it:
  `generate_route_list(App)` + `leptos_routes_with_context` + fallback +
  session layer; factor a small router-construction helper out of
  `main.rs` if needed so tests and prod share it). Request `/`, `/login`,
  `/dashboard`, `/games`, and `/games/{id}` for a seeded game (fixtures
  from the existing db.rs/game tests; game service HTTP mocked in-process
  per 11.7 conventions - `find_game_extended`-only pages may not need the
  mock at all). Assert 200, `text/html`, body contains a page-specific
  marker string, and - the key assertion - the SSR body does NOT contain
  a rendered Leptos error/panic marker. Cover both anonymous and
  logged-in requests where the page renders differently (the
  `get_active_games` 500 found by the E2E work was exactly an
  anonymous-hard-load bug: `SidebarMenu`'s server fn errored for
  no-session users). Authenticated requests need a session cookie:
  either drive `POST` login server-fn endpoints via `oneshot` first, or
  insert a `tower-sessions` row directly - implementer's choice,
  document it. These tests catch SSR panics, route breakage, and
  server-fn 500s in milliseconds with zero new dependencies, and run in
  the existing `test-rust` CI job unchanged.

- **11.6b - minimal Playwright hydration smoke (browser; residue only).**
  One spec file, single browser context, chromium only: hard-load `/`,
  `/login`, then log in (DB-read of `login_confirmation`), `/dashboard`,
  `/games`, create one bot-opponent game (no second human context
  needed), hard-load the game page, hard-reload it once, assert zero
  console errors / `pageerror`s throughout. Target well under 1 minute
  of Playwright time. NO multi-context tests, NO WebSocket-propagation
  assertions, NO command/undo/concede/restart driving - that logic is
  Rust-tested; the browser layer exists solely to catch hydration
  breakage. The existing harness (run.sh, seed.sql, helpers.ts,
  playwright.config.ts, CI e2e job) is kept as-is; only the spec files
  shrink. `tests/game-flow.spec.ts` is deleted (git history preserves it
  if a pre-release manual checklist ever wants the choreography back).

**Committed state (as of 2026-07-04, working tree on `leptos`):** the work
described below was uncommitted at the time this section was written and has
since been committed (commits 8ca8c8f, a316a00). The list describes what
landed:

- `rust/web/end2end/run.sh` - full stack boot: resets `brdgme_e2e` DB
  (sqlx-cli), applies `seed.sql`, `cargo leptos build --release`
  (`E2E_SKIP_BUILD=1` to skip), builds `lost_cities_2_http`, starts game
  service (127.0.0.1:8100) + release web binary (127.0.0.1:3010, env:
  `LEPTOS_OUTPUT_NAME=web`, `LEPTOS_SITE_ROOT=<rust>/target/site`,
  `LEPTOS_ENV=PROD`, `DATABASE_URL`, `REDIS_URL`; `RESEND_API_KEY`/
  `BOT_SERVICE_URL` unset), `/dev/tcp` readiness polling, `trap` teardown,
  then `npx playwright test`. Never runs `playwright install` locally.
- `rust/web/end2end/seed.sql`, `tests/helpers.ts` (login via `pg`
  DB-read incl. `user_emails` join, `uniqueEmail`,
  `collectConsoleErrors`), `tests/page-loads.spec.ts` (already close to
  the 11.6b target scope), `tests/game-flow.spec.ts` (to DELETE per the
  redesign), `playwright.config.ts` (chromium-only, `workers: 1`,
  baseURL 3010, optional `E2E_CHROMIUM_PATH` executablePath escape hatch
  with `--no-sandbox` for Nix environments), `package.json` pinned
  `@playwright/test 1.60.0` to match devenv's Nix browsers, `pg` +
  `@types/pg` added; `tests/example.spec.ts` deleted.
- `rust/web/src/game/server_fns.rs` - REAL BUG FIX found by the E2E
  work, keep regardless of testing direction: `get_active_games`
  refactored to `active_games_summary(user: Option<AuthUser>, pool)`;
  anonymous users get `Ok(vec![])` instead of a "Not authenticated"
  `ServerFnError` (which rendered as HTTP 500 on every hard load of any
  page containing `SidebarMenu`). Includes two `#[sqlx::test]`s
  (anonymous -> empty; bot-opponent game maps correctly) and two new
  `rust/web/.sqlx/query-*.json` cache files.
- `.github/workflows/ci.yml` - new `e2e` job: postgres:18/redis:8
  services (`E2E_DATABASE_URL=postgres://postgres:postgres@localhost/brdgme_e2e`),
  stable toolchain + wasm32 target, Swatinem/rust-cache,
  taiki-e/install-action (cargo-leptos, wasm-bindgen-cli@0.2.121 - keep
  in sync with the workspace pin - and sqlx-cli), apt `binaryen`
  (wasm-opt, required by `cargo leptos build --release`), node 22,
  Playwright browser cache keyed on the 1.60.0 pin,
  `npx playwright install chromium --with-deps`, `npm ci`, `./run.sh`.
- `devenv.nix` - `binaryen` added to packages (was missing; release
  builds failed on `wasm-opt`). Already reloaded locally.

**Environment gotchas for whoever picks this up (hard-won):**
- devenv sets `PLAYWRIGHT_BROWSERS_PATH` to a read-only Nix store path
  containing browsers for Playwright **1.60.0** (chromium-1223). The npm
  pin must stay in lockstep or Playwright tries to download into the
  read-only path and hangs/fails. Never `npx playwright install` locally.
- Playwright-downloaded chromium in `~/.cache/ms-playwright` does NOT
  launch on this machine (missing system libs, no root/apt). If the
  Nix-provided browser fails to launch, use
  `E2E_CHROMIUM_PATH="$(command -v chromium)"` (system Nix chromium; the
  config adds `--no-sandbox`). CI (ubuntu-latest) has neither problem.
- Any stale `rust/target/release/web` binary predating the
  `server_fns.rs` fix must be rebuilt before an E2E run (no
  `E2E_SKIP_BUILD=1` on the first run).

**Verification status (2026-07-04, verified):** all four remaining-work
items below are done and green. `cargo fmt --all -- --check`,
`cargo clippy -p web --all-targets --features ssr -- -D warnings`, and
`cargo test -p web --features ssr` (54 tests, including the 6 new 11.6a
`tests/ssr_pages.rs` tests) all pass. `rust/web/end2end`: `npm ci`,
`npx tsc --noEmit`, `bash -n run.sh` all clean. `./run.sh` ran green twice
(first with a full release build, second with `E2E_SKIP_BUILD=1`);
Playwright portion ~2s each run, well under the 1-minute budget. The `e2e`
CI job parses with `yq` and its steps match the local `run.sh` flow
(same env vars, same version pins). Two stale `lost_cities_2_http`/`web`
processes left over from an earlier interrupted run were found squatting
on ports 8100/3010 during verification (caused one run to falsely report
green via pre-existing listeners) - killed before the runs recorded above;
not a defect in the harness itself.

**Remaining work (delegable as one task; original scenario checklist
below is superseded):**

- [x] Verify + land the current uncommitted work, trimmed to the 11.6b
      scope: delete `tests/game-flow.spec.ts`; keep/adjust
      `page-loads.spec.ts` (add one hard reload of the game page; ensure
      it stays single-context); run `npm ci`/`npx tsc --noEmit`/
      `bash -n run.sh`; run `cargo fmt --all -- --check`,
      `cargo clippy -p web --features ssr -- -D warnings`,
      `cargo test -p web --features ssr`; full `./run.sh` green twice
      (second with `E2E_SKIP_BUILD=1`), Playwright portion < 1 min.
- [x] Implement 11.6a in-process SSR page tests as specified above
      (new test module, e.g. `rust/web/src/ssr_pages.rs` tests or
      `rust/web/tests/`; router-construction helper shared with
      `main.rs` if extraction is needed). Implemented as
      `rust/web/tests/ssr_pages.rs` plus a `web::router::build_router`
      helper (`rust/web/src/router.rs`) factored out of `main.rs` so both
      share the exact same route/session/fallback wiring. Authenticated
      requests are driven by inserting a `tower-sessions` row directly via
      the same `PostgresStore` the app uses (documented in the test
      module doc comment) rather than driving the `Login`/`ConfirmLogin`
      server fns over HTTP, since their routes carry a compile-time hash
      suffix that isn't practical to hardcode in a test.
- [x] CI: `e2e` job validated (yq parse + step-by-step sanity check
      against the local flow); 11.6a tests run in the existing
      `test-rust` job automatically (no CI changes needed - `cargo test -p
      web --features ssr` already picks up `tests/ssr_pages.rs`).
- [x] Update `docs/CODING.md` 11.7 conventions if the E2E budget wording
      needs to reflect the new two-layer split (browser layer: hydration
      smoke only, < 1 min). Replaced the old single-suite budget paragraph
      with a two-layer description (SSR primary, Playwright residue).

**Superseded original design + scenarios (kept for reference; the
harness bullets below were implemented and remain accurate for run.sh):**

- **Location:** `rust/web/end2end/` - the existing cargo-leptos Playwright
  scaffold (already wired via `end2end-cmd`/`end2end-dir` in
  `rust/web/Cargo.toml`). Bump `@playwright/test` to latest. Chromium only
  (drop the firefox/webkit projects), `workers: 1`, `baseURL`
  `http://127.0.0.1:3010`.
- **Stack (4 processes, not 3):** the web binary requires Redis at startup
  (GameBroadcaster). Postgres + Redis are NOT started by the harness: local
  runs reuse the devenv/Tilt services (localhost:5432/6379), CI reuses the
  existing service-container pattern from `test-rust`.
- **Entry point** `rust/web/end2end/run.sh`:
  1. Env (overridable): `E2E_DATABASE_URL` (default
     `postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_e2e`;
     CI sets `postgres://postgres:postgres@localhost/brdgme_e2e`),
     `REDIS_URL` (default `redis://localhost:6379`).
  2. Drop + recreate the `brdgme_e2e` database, apply `rust/web/migrations/`
     (sqlx-cli), apply `end2end/seed.sql`.
  3. `cargo leptos build --release` (skippable via `E2E_SKIP_BUILD=1`) and
     `cargo build --release -p lost-cities-2 --bin lost_cities_2_http`.
  4. Start game service with `ADDR=127.0.0.1:8100`; start the release web
     binary with `LEPTOS_SITE_ADDR=127.0.0.1:3010`, `LEPTOS_SITE_ROOT`
     pointing at the built site dir, `DATABASE_URL=$E2E_DATABASE_URL`,
     `REDIS_URL`; `RESEND_API_KEY`/`BOT_SERVICE_URL` unset (login codes are
     logged + stored in DB; no bot triggering). Ports 8100/3010 avoid the
     dev stack's 80/3000.
  5. Readiness: poll both HTTP ports; teardown via shell `trap` killing
     both PIDs. Then `npx playwright test`.
- **Seed SQL** (`end2end/seed.sql`): one `game_types` row (fixed UUID,
  name `Lost Cities`, `player_counts '{2,3}'`) + one `game_versions` row
  (fixed UUID, `uri 'http://127.0.0.1:8100'`, `is_public true`,
  `is_deprecated false`). No operator involved.
- **Login helper** (`tests/helpers.ts`): submit the email form, then read
  `users.login_confirmation` directly from Postgres via the `pg` npm
  package (`E2E_DATABASE_URL`), enter the code. Unique per-run emails
  (timestamp suffix) keep tests independent.
- **Console-error assertion helper:** collect `console` messages of type
  `error` plus `pageerror` events per page; assert empty at test end
  (hydration panics surface via `console_error_panic_hook`).
- **CI:** new `e2e` job in `.github/workflows/ci.yml`, runs on the same
  triggers as `test-rust` (PRs + master). postgres/redis service
  containers copied from `test-rust`; toolchain + wasm32 target;
  cargo-leptos and a `wasm-bindgen-cli` matching the workspace pin
  installed via a binary-install action (not `cargo install` from
  source); `Swatinem/rust-cache` for the release build;
  `actions/cache` on `~/.cache/ms-playwright` keyed on the Playwright
  version; `npx playwright install chromium --with-deps`. Budget: suite
  under 5 minutes excluding the release build.

- [x] Harness + stack boot script + DB seed. (Implemented, uncommitted;
      kept under the redesign.)
- [x] Hard-load `/`, `/login`, `/dashboard`, `/games`, and an active
      `/game/{id}` - assert zero browser console errors (hydration panics
      surface via `console_error_panic_hook`). (= 11.6b scope; implemented
      as `page-loads.spec.ts`, verified green via `./run.sh` 2026-07-04.)
- ~~Full game flow with two browser contexts~~ Superseded 2026-07-04:
  covered by Rust tests (11.3); browser version was implemented, proved
  racy, and is deleted under the redesign.
- ~~Invalid command shows the error message; input is not cleared.~~
  Superseded 2026-07-04: move to 11.6a/Rust coverage if desired; not a
  browser concern.
- ~~Undo; concede (accept the confirm dialog); restart navigates the
  restarting player to the new game.~~ Superseded 2026-07-04: covered by
  Rust tests (11.2/11.3).
- [x] Hard refresh on the game page mid-game - the highest-risk hydration
      scenario (real async data + Suspense). (Kept in 11.6b; verified
      green 2026-07-04.)
- [x] CI: separate job (needs the release build); runs on pull requests +
      master. (Implemented; `yq`-parsed and sanity-checked against the
      local `run.sh` flow 2026-07-04.)

### 11.7 Testing conventions (add to `docs/CODING.md`) [Complete]

- [x] New or changed logic in `db.rs`, `game/mod.rs`, and `auth/` must land
      with tests. Reviewers/agents reject changes to these files without them.
- [x] Game service HTTP is always mocked in `rust/web` tests; the LLM is
      never called in any test.
- [x] Use `#[sqlx::test]` for anything touching the DB; never share state
      between tests.
- [x] E2E scenarios are added only for user-visible flows, and the suite must
      stay under its time budget.

**Build order for delegation:** 11.1 → 11.2 → 11.3 → 11.4 → 11.5 → 11.6.
Each sub-phase is independently delegable once 11.1 is merged. Phase 12 (ELO)
should be implemented after 11.2 so it lands with tests.

