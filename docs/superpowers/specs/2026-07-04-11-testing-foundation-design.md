# 11: Testing Foundation - Design

> Extracted 2026-07-08 from `docs/plan/11-testing-foundation.md` (superpowers layout
> migration). Content dates from 2026-07-04; this is a point-in-time decision
> record, not a living document.

**Status:** Complete (completed 2026-07-04)

## Goal

Build test coverage over the critical orchestration, data, and auth
paths so routine work can be delegated to cheaper models/agents safely. The
tests are the guardrail: they must fail when core game flows break, and they
must run in CI on every push.

## Current state (audited 2026-07-02)

Good unit coverage in the libraries
(`brdgme_markup` parser/transform, `brdgme_game` command parser + suggest,
game crate logic, bot prompt rendering - 14 tests). `rust/web` has exactly one
test (`game/client.rs::test_game_client_contract`, in-process mocked game
service). Zero coverage of `db.rs`, `auth/`, `game/mod.rs::execute_command`,
Axum handlers, server fns, and `websocket.rs` - the code agents touch most.
CI already runs `cargo test --workspace --exclude web` and
`cargo test -p web --features ssr` but provides no Postgres, so DB-backed
tests cannot exist yet.

## Stack decisions

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

## 11.6 Frontend/page testing (redesigned 2026-07-04)

**Direction change (2026-07-04, second design pass):** the original scope
was a Playwright E2E suite covering full game flows in the browser. A
working implementation was built (see "Committed state" in the
implementation plan) and immediately demonstrated the classic E2E failure
modes: multi-context WebSocket choreography races, re-render timing races on
the command input, browser-provisioning friction (Nix read-only
`PLAYWRIGHT_BROWSERS_PATH` vs Playwright's downloaded chromium needing
system libs), and 60-120s timeout-style debugging. Michael's judgement: he
does not want slow, flaky, heavy-dependency tests in this project.

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

### Superseded original design (kept for reference; the harness bullets below were implemented and remain accurate for run.sh)

The superseded original scenario checklist is retained in the
implementation plan.

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
