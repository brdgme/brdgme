# Monolith Migration Plan

**Current focus (in order):**
Phase 11 testing foundation → Phase 12 ELO ratings
(blocks prod deploy) → Restart 500 error → 3-player render → Phase 22a
Resend outbound → Phase 14 drop Knative (incl. ctlptl) → Phase 13 NATS bot
eventing (JetStream) → Phase 19 CloudNativePG → Phase 15 ArgoCD +
sealed-secrets → Phase 20 external-dns → Phase 16 cutover + validation →
Phase 22b play-by-email → Phase 17 NATS WS migration → Phase 18 hardening
(VictoriaLogs). Phase 21 OpenTofu is human-paced and independent; highest
value if started before Phase 14's prod prerequisites.

Phases are numbered in assignment order, not execution order - see the focus
line for execution order. Phases 1-10 are complete; 11+ are pending.
(Renumbered 2026-07-02: 5.5→6, 5.6→7, old 6→8, 5.7→10, 6.5→ArgoCD, old
7→cutover, old 8→NATS WS; ELO and NATS bot eventing split out of Phase 9
into Phases 12 and 13. 2026-07-03: Phase 14 'Drop Knative' inserted; ArgoCD
14→15, cutover 15→16, NATS WS 16→17, hardening 17→18. 2026-07-03 tech
review: Quick wins section and Phases 19-21 added; JetStream, ctlptl,
sealed-secrets, and VictoriaLogs decisions folded into Phases 13/14/15/18.
2026-07-03: Phase 22 'Email via Resend' added, split 22a outbound /
22b play-by-email; 22a revised same day to the Resend HTTP API - DO blocks
outbound SMTP - superseding the Mailpit quick win. 2026-07-03 final pass:
Renovate/cargo-deny/kubeconform quick win, leptos-use in Phase 17,
tower_governor in 22a, stale root artifacts in the Phase 16 decommission.
2026-07-03: Phase 10 runtime panics completed.)

## Objective

Consolidate the `brdgme` platform into a single Rust-based monolithic
application using Axum (backend) and Leptos (frontend/WASM). This replaces the
Rocket API, Node.js WebSocket service, and TypeScript/React frontend.

## Strategy

Build the new system in `rust/web` in parallel with the existing services. The
old services (`rust/api`, `web`, `websocket`) remain untouched until cutover.

## Out of Scope (decided 2026-07-02)

- **Go game services**: the 17 Go games under `brdgme-go/` remain in production
  indefinitely behind the stable game HTTP contract. They are not part of this
  migration and there is no plan to port them to Rust. The contract is
  language-agnostic; Go and Rust games are built and deployed identically.
- **Chat**: legacy chat tables/queries (`rust/api` chat queries, `games.chat_id`)
  are not ported. Future work, not scheduled.
- **lords-of-vegas-1**: implemented in `rust/game/` but intentionally not
  deployed (no Tiltfile entry, no k8s manifests). Future work, not scheduled.
- **Play-by-email**: not part of the cutover itself, but now planned as
  Phase 22b (post-cutover). Outbound email moves to Resend pre-cutover
  (Phase 22a).

---

## Delegation Readiness (assessed 2026-07-02)

Which pending work is specified in enough detail to hand to a cheaper
agent/model as-is. Items marked "gap" have an inline **Delegation gap** note
in their section describing what must be fleshed out first (in a future
planning session - deliberately not done yet).

**Ready to delegate now:**
- Phase 11.1-11.5, 11.7 (testing) - stack decided, cases enumerated.
- Phase 12 (ELO) - algorithm, reference code, tasks, and tests specified.
- Phase 14 (drop Knative) - manifest inventory and tasks specified
  (2026-07-03); the "Prod prerequisites" subsection is operator-verified,
  everything else is delegable.
- Optimistic locking (Bug fixes) - 4-step design specified.
- Phase 16 http.ts apex-domain verification subtask (read-only code check).
- Phase 19 dev-side (Kind CNPG manifests, Tiltfile/mirrord updates) -
  specified 2026-07-03; the prod import is human-operated.
- Phase 22a (Resend outbound) - code change fully specified; the account
  creation and DNS records are human steps.
- Dependency automation + CI hygiene quick win (2026-07-03 final pass) -
  Renovate config, cargo-deny, kubeconform all specified.

**Gaps - needs detail before delegation:**
- Restart 500 bug - diagnosis task with no repro/capture procedure.
- Bot restart limitation - no expected-behaviour spec.
- 3-player Lost Cities render - tracked only in the focus line, no task body.
- Phase 11.6 (E2E harness) - scenario list is ready but the harness
  environment needs a design pass.
- Phase 13 (NATS bot eventing) - delivery guarantees, manifests, and attempt
  limits resolved 2026-07-03 (JetStream - see phase); rollout sequencing and
  test plan still open.
- Phase 15 (ArgoCD) - underspecified, and largely human-operated anyway.
- Phase 17 (NATS WS migration) - server subscription architecture and the
  client-side refactor are only sketched.
- Phase 18 (hardening) - VictoriaLogs chosen for logs (2026-07-03);
  ErrorBoundary scope, WASM source maps, and alerting destination still open.
- Phase 20 (external-dns) - manifests delegable; live DNS record adoption is
  human-operated.
- Phase 21 (OpenTofu) - human-operated by nature (account credentials).
- Phase 22b (play-by-email) - design specified, but the Resend inbound
  payload schema and webhook signature scheme must be confirmed against a
  live account before the endpoint work is delegated.

---

## Phase 1: Foundation & Shared Logic [Complete]

**Goal:** Make `brdgme_cmd` and `brdgme_game` compatible with WASM so they can
be used in the browser frontend.

- [x] Make `warp` an optional dependency in `rust/lib/cmd/Cargo.toml`.
- [x] Gate the `http` module in `rust/lib/cmd/src/lib.rs` behind
      `#[cfg(feature = "http-server")]`.
- [x] Verify WASM compilation: `cargo check --target wasm32-unknown-unknown -p brdgme_cmd`.

---

## Phase 2: Database Layer [Complete]

**Goal:** Establish an async data layer using SQLx, replacing Diesel.

- [x] Add `sqlx` to `rust/web/Cargo.toml` (postgres, runtime-tokio, uuid, chrono).
- [x] Create `rust/web/migrations/001_initial_schema.sql` as a baseline snapshot
      of the full existing schema. SQLx manages all future changes from this
      point.
- [x] Implement `rust/web/src/db.rs` with basic user and session queries.

**Notes:**
- Applied baseline migration to local PostgreSQL dev instance via `sqlx-cli`.
- Added `reqwest` (rustls) and `sqlx-cli` to `devenv.nix`.
- Configured `DATABASE_URL` in `devenv.nix`.

---

## Phase 3: Backend (Axum Core) [Complete]

**Goal:** Replicate auth and game orchestration logic in Axum.

Note: play-by-email is out of scope for this migration.

- [x] Implement auth routes in `rust/web/src/auth/` (login, register, logout)
      using `tower-sessions`.
- [x] Implement `rust/web/src/game/client.rs`: async HTTP client for
      communicating with external game microservices using the JSON contract.
- [x] Write a unit test mocking a game service response to verify contract
      serialization (`test_game_client_contract`).
- [x] Implement game API endpoints:
  - `POST /api/game/new`
  - `GET /api/game/{id}`
  - `POST /api/game/{id}/command`

**Notes:**
- Implemented `create_game_with_users`, `find_game_extended`, and
  `update_game_command_success` in `db.rs`.
- Refactored `rust/web/src/state.rs` to a combined `AppState` (LeptosOptions +
  PgPool) shared between Axum handlers and Leptos routes.

---

## Phase 4: WebSocket Integration [Complete]

**Goal:** Internalise real-time updates, removing the Node.js/Redis dependency.

Note: the in-process broadcast used here (`tokio::sync::broadcast`) does not
support multiple replicas. NATS Core will replace it in the post-migration
infrastructure phase. See `docs/VISION.md`.

- [x] Add `/ws` route using `axum::extract::ws::WebSocketUpgrade`.
- [x] Implement `GameBroadcaster` in `rust/web/src/websocket.rs` using
      `tokio::sync::broadcast`.
- [x] Integrate broadcast calls into `create_game` and `play_command` handlers.

**Notes:**
- Added `futures-util` for async stream management.

---

## Phase 5: Frontend (Leptos UI) [Complete]

**Goal:** Build the UI in Rust, replacing React.

- [x] Build app shell and layout components.
- [x] Implement shared types and server functions.
- [x] Build `GameBoard` (ASCII-to-HTML), `GameMeta`, `GameLogs`,
      `GameCommandInput` components.
- [x] Implement client-side command parsing using `brdgme_game` compiled to
      WASM, providing real-time suggestions and validation.
- [x] Implement WebSocket client hook (`websocket_client.rs`) that triggers
      resource refetches via a global `WebSocketTrigger` context.

**Known defects (all since resolved in Phase 7, listed for history):**
- Login is non-functional end-to-end: `on_email_submit` and `on_code_submit`
  in `app.rs` are not connected to the `Login` and `ConfirmLogin` server
  functions.
- Game creation and command submission are unauthenticated: Axum handlers use
  `Uuid::nil()` as the user ID.
- Turn enforcement is absent: any player can submit a command at any time.
- `GameLogs` is a stub placeholder.
- `DashboardPage` and `GamesPage` are stubs with no content.
- Autocomplete suggestions are not prefix-filtered: all alternatives show for
  every keystroke.
- Suggestions are not clickable.
- Command errors are silently discarded.

**Notes:**
- Added `cargo-leptos`, `dart-sass`, and `wasm-bindgen-cli` to `devenv.nix`.
- Pinned `wasm-bindgen` to `=0.2.108` (matches `wasm-bindgen-cli` in nixpkgs).
- Enabled `js`/`wasm_js` features for `getrandom`.
- Updated Axum routing to `{id}` syntax.

---

## Phase 6: Dev Environment Migration [Complete]

**Goal:** Replace the minikube-based dev environment with Kind + Cilium +
Knative, and replace skaffold with Tilt. This is a prerequisite for the
side-by-side validation phase, since the new `rust/web` service will run as a
Knative Service in production.

### Kind cluster + Cilium

- [x] Write a Kind cluster config with the default CNI disabled
      (`networking.disableDefaultCNI: true`). → `k8s/kind-config.yaml`
- [x] Install Cilium as the CNI into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [x] Verify pod networking and DNS work correctly.

### Knative Serving

- [x] Install Knative Serving into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [x] Configure Cilium as the Knative networking layer via `net-gateway-api`
      (Cilium's GatewayClass + Knative Gateway API ingress class). Setup
      automated in `scripts/setup-kind-cluster.sh`.
- [x] Verify a simple Knative Service deploys and is reachable.

### k8s manifests: rust/web as a Knative Service

- [x] Replace `k8s/base/web/deployment.yaml` and `k8s/base/web/service.yaml`
      with a Knative `Service` manifest (`serving.knative.dev/v1`).
- [x] Set `minScale: 1` - the monolith must not scale to zero.
- [x] Game microservices remain as plain Deployments for now.

### Tilt

- [x] Write a `Tiltfile` covering:
  - Deploy backing services (Postgres, Redis, SMTP, game microservices)
    to the Kind cluster via `k8s/dev-without-web` kustomize path.
  - Run `cargo leptos watch` as a local process for fast iteration (default
    hybrid mode).
  - Port-forwarding for Postgres (5432) and Redis (6379).
- [x] Full-cluster mode added (`WEB_IN_CLUSTER=1`): builds `brdgme/web` and
      deploys as Knative Service via `k8s/dev` kustomize path.
- [x] **Migrate CI to GitHub Actions**: `.github/workflows/ci.yml` created.
      Builds all service images and pushes to GHCR on master. `k8s/prod` image
      references updated to GHCR via kustomize `images` overlay.
- [x] Remove `skaffold.yaml` and `.travis.yml` once GitHub Actions CI is
      verified working on master.
- [x] **Local registry for Kind + Knative**: `registry:2` container +
      containerd mirror patch + `config-deployment` skip for `kind-registry:5000`
      + Tilt `default_registry`. Requires cluster recreation to take effect.

### Production builds

Production image builds and deploys are driven by GitHub Actions (see item
above). `kubectl apply -k k8s/prod` is still the deploy mechanism.

---

## Phase 7: Pre-Cutover Fixes [Complete]

**Goal:** Resolve all blockers and close critical gaps found in the parity
review before the `leptos` branch replaces production. (The review document
`docs/REVIEW.md` has since been deleted; its findings are the task lists below.)

### Blockers (must fix before cutover)

- [x] **Auth in Axum handlers** (`game/server.rs`): Replace `Uuid::nil()` in
      `create_game` and `play_command` with the authenticated user ID from the
      session.
- [x] **Login UI wired to server functions** (`app.rs`): Connect
      `on_email_submit` and `on_code_submit` to the `Login` and `ConfirmLogin`
      server functions via `Action`. Navigates to `/dashboard` on success.
      Error messages shown on failure.
- [x] **Confirmation token not exposed in response** (`auth/server.rs`): Token
      removed from response message.
- [x] **Persistent session store** (`auth/session.rs`):
      `tower-sessions-sqlx-store 0.15.0` + `PostgresStore`, table created via
      `store.migrate()` in `create_session_layer`.
- [x] **`with_secure` env-driven** (`auth/session.rs`): Reads `SECURE_COOKIE`
      env var (`"true"` = secure).
- [x] **Graceful SIGTERM shutdown** (`main.rs`): Added `shutdown_signal()`
      listening for SIGTERM and Ctrl+C.
- [x] **Turn enforcement** (`game/server.rs`): Rejects commands when
      `!player.game_player.is_turn`.
- [x] **Authenticate `GET /api/game/{id}`** (`game/server.rs`): Returns 401
      when no valid session.
- [x] **`GamePlayer` model missing fields** (`models/game.rs`): Added
      `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
      `rating_change`. Migration `002_game_player_fields.sql` applied.
- [x] **`update_game_command_success` writes all fields** (`db.rs`): Persists
      `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`, `points`,
      and `finished_at` on every command.
- [x] **`find_game_extended` handles missing `game_type_users` row** (`db.rs`):
      Returns default `GameTypeUser` with rating 1500 instead of erroring.
- [x] **Token expiry check in `validate_session_token`** (`auth/session.rs`):
      SQL query updated to `AND created_at > NOW() - INTERVAL '30 days'`.
      SQLx offline metadata regenerated.
- [x] **Email sending** (`auth/server.rs`): `send_login_email` originally
      implemented using `lettre 0.11` with `AsyncSmtpTransport` (plain
      SMTP, no TLS); replaced by Phase 22a with `resend-rs` over the
      Resend HTTP API. Reads `RESEND_API_KEY`, `EMAIL_FROM` from env.
      Logs the code instead of sending if `RESEND_API_KEY` unset (dev
      fallback).

### Missing endpoints (non-blocking, needed for feature parity)

- [x] **`POST /game/{id}/undo`**: Restore `undo_game_state`, call `Status` on
      the game service, clear all players' undo state, write a log entry,
      broadcast.
- [x] **`POST /game/{id}/mark_read`**: Set `is_read = true` on the calling
      player's `game_players` row.
- [x] **`POST /game/{id}/concede`**: Limited to 2-player games. Mark game
      finished, write log entry, broadcast.
- [x] **`POST /game/{id}/restart`**: Create new game with same players, link
      via `restarted_game_id`. Broadcasts `BrdgmeUpdate` for both the new game
      and the old game (with `restarted_game_id` now set), so all players see
      the "Go to new game" link. Restarting player navigates via server fn response.

### Operator (`rust/operator`) [Complete]

- [x] **`GameVersion` CRD** (`k8s/base/operator/crd.yaml`): Namespaced CRD
      with `typeName`, `playerCounts`, `weight`, `isDeprecated` fields.
- [x] **kube-rs operator**: Reconciles `GameVersion` CRs by upserting
      `game_types` and `game_versions` rows in PostgreSQL. Finalizer ensures
      `is_public = false` is written before CR deletion.
- [x] **`is_deprecated` support**: `lost-cities-1` CR has `isDeprecated: true`
      - kept running for in-progress games, excluded from new game creation.
- [x] **Migration 003**: Unique constraints on `game_types(name)` and
      `game_versions(game_type_id, name)` enabling `ON CONFLICT` upserts.
- [x] **20 `GameVersion` CR files** colocated with each game in
      `k8s/base/game/{name}/game-version.yaml`.
- [x] **Tilt integration**: `crd-ready` resource gates operator startup on CRD
      establishment; operator runs as `local_resource` with `RUST_LOG=info`.
- [x] **mirrord**: Added to `devenv.nix`; Tiltfile wraps `cargo leptos watch`
      with `mirrord exec --target pod/postgres-0 --target-namespace brdgme`
      so the local web server resolves `*.svc.cluster.local` without application
      changes or `/etc/hosts` modification.

### Frontend gaps (non-blocking)

- [x] **New-game creation UI** (`app.rs`, `GamesPage`): Game type selector,
      optional version selector, player count selector, opponent email inputs,
      submit → redirect to new game.
- [x] **Game log rendering** (`components/game.rs`): `GameLogs` and
      `RecentGameLogs` components fetch logs, render markup to HTML, group by
      10-minute windows, filter to logs since `last_turn_at`.
- [x] **Undo/concede/restart actions in `GameMeta`**: `UndoGame`,
      `ConcedeGame`, `RestartGame` server functions wired; visibility
      conditions enforced (`can_undo`, `is_finished`, `restarted_game_id`).
- [x] **"Whose turn" display** (`app.rs`): Shows specific player names
      from `players` filtered by `is_turn`.
- [x] **Mark-read on game page load** (`app.rs`): `mark_read` called via
      `Effect` on mount and game ID change.
- [x] **`GameRestarted` WebSocket navigation**: `GameRestarted` WS message
      triggers a refetch (same as `GameUpdate`). Player who clicked Restart
      navigates via `restart_action` effect. Other players see the finished
      game and a "Go to new game" link once `restarted_game_id` is populated.
- [x] **Command input: clear after server confirms** (`components/game.rs`):
      `Effect` runs `set_command("")` after `submit_action` succeeds.
- [x] **Command errors surfaced to user** (`components/game.rs`): `error_msg`
      memo observes `submit_action` result and displays errors.
- [x] **Clickable command suggestions** (`components/game.rs`): Click handler
      appends suggestion value to command input.
- [x] **Autocomplete prefix filtering** (`rust/lib/game`): `CommandSpec::suggest`
      called from `GameCommandInput`.

### Code quality (non-blocking)

- [x] **Dead code removed**: `New*` model structs, `chat.rs`, `friends.rs`,
      `PublicGameType` alias, `db::AppState` deleted or removed.
- [x] **`reqwest::Client` shared** (`game/client.rs`): Created once in `main.rs`,
      stored in `AppState`, provided as Leptos context; all `client::` fns take `&Client`.
- [x] **N+1 in `find_active_games_for_user`** (`db.rs`): Replaced loop with a
      single joined query across all required tables; SQLx cache updated.
- [x] **Duplicate command logic** (`game/server.rs` vs `server_fns.rs`):
      Extracted into `game::execute_command`; both callers delegate to it.
- [x] **`chrono` → `time`** (`models/`, `lib/game`, `lib/cmd`): Replaced
      `chrono::NaiveDateTime` with `time::PrimitiveDateTime` throughout all
      model structs, `brdgme_game::Log`, and `brdgme_cmd::CliLog`. Required
      because `tower-sessions-sqlx-store` enables `sqlx/time` which takes
      precedence over `sqlx/chrono` in type inference.
- [x] **Points persisted** (`db.rs`): `_points` suppression removed;
      points written per-player in `update_game_command_success`.
- [x] **Logout redirect/feedback** (`components/layout.rs`): Navigate to `/login` after logout action succeeds.
- [x] **WebSocket reconnection** (`websocket_client.rs`): Reconnects with a
      2-second delay after disconnect; `use_websocket` now runs a `spawn_local` loop.
- [x] **`finished_at` set when `is_finished = true`** (`db.rs`): Set via
      `COALESCE($arg, finished_at)` in `update_game_command_success`.
- [x] **`active_games` ownership moved to `SidebarMenu`** (formerly tracked as
      Phase 5.6.1): the `LocalResource` was hoisted to `App()` via context,
      breaking SSR resource tracking and causing a hydration crash. Now created
      directly in `SidebarMenu` (`components/layout.rs`), driven by
      `WebSocketTrigger`. Verified complete 2026-07-02.

---

## Bug fixes [Partially resolved]

- [ ] **Restart 500 error**: `restart_game` returns "Game service error: error
      parsing JSON response". Diagnostics improved: `client::request` now reads
      response body as text first and includes it in the error message. Root
      cause still unknown - needs a live restart attempt to capture the raw
      game service response.
      **Delegation gap:** this is a diagnosis task with no procedure. Before
      delegating, write: exact repro steps (game type, state, who restarts),
      how to capture the raw response (RUST_LOG settings, which Tilt resource
      logs to read, or a curl replay of the restart request), and what to do
      with the captured payload (fix criteria vs report back).
- [ ] **Bot restart limitation**: when a game is restarted, bots from the
      original game are not carried over to the new game. The `restart_game`
      handler (`game/server.rs`) copies players but does not check
      `game_players.game_bot_id` and create corresponding `game_bots` rows in
      the new game.
      **Delegation gap:** no expected-behaviour spec. Decide and document:
      are new `game_bots` rows created copying name + difficulty; do bots keep
      their positions or are positions reshuffled like humans; behaviour when
      the restarted game has different player-count constraints; and the test
      cases that define done.
- [ ] **3-player Lost Cities render**: `lost-cities-2/RULES.md` has a
      placeholder for the 3-player "Reading the Display" section.
      **Delegation gap:** no task body. Needs: how to obtain a representative
      3-player game state, the render extraction procedure (per `docs/RULES.md`
      and the game crate conventions), and which RULES.md sections must change.
- [ ] **Optimistic locking missing in `execute_command`**: two concurrent
      requests (e.g. two players submitting at the same instant, or a bot and a
      player) can both read the same game state, both call the game service, and
      both attempt to write back. The second write silently overwrites the first.
      Fix using `games.updated_at` (microsecond precision, set by trigger on
      every UPDATE) - no migration needed:
      1. Read `game.updated_at` in `execute_command` alongside `game_state`.
      2. Pass `expected_updated_at` to `update_game_command_success`.
      3. Change the UPDATE to
         `UPDATE games SET ... WHERE id = $1 AND updated_at = $expected`.
      4. `rows_affected == 0` → return a conflict error: a human player gets
         a "please retry" error; the bot treats it like a validation error and
         re-fetches fresh state via its existing post-LLM state-change
         detection.
      Changes: `execute_command` in `game/mod.rs`; signature and UPDATE query
      in `update_game_command_success` in `db.rs`.
- [x] **Concede confirmation**: Added `window.confirm("Are you sure you want to
      concede?")` in the click handler before dispatching `ConcedeGame`.
      `"Window"` added to web-sys features.
- [x] **Recent logs `is_new` always false**: `logged_at` (game service time) is
      set before `last_turn_at` is written to DB, so `logged_at > last_turn_at`
      was always false. Fixed: `log.created_at >= last_turn_at` (DB insert time,
      set after `last_turn_at` commits). Matches web-legacy.
- [x] **Suggestions/command input too narrow**: `game-command-input-container`
      wrapper div had no explicit width; as a centered flex child its children's
      `width: 63%` resolved against an unsized parent. Fixed: return a fragment
      `<>` from `GameCommandInput` so both elements are direct children of
      `.game-main` and correctly receive 63% of its width.
- [x] **Timestamp shown in recent logs**: `render_log_entries` now takes
      `show_timestamp: bool`; recent logs pass `false`, sidebar logs pass `true`.
      Also fixed: empty `log-time` divs (block elements adding blank lines) now
      only rendered when a label exists.
- [x] **Scroll to bottom**: `NodeRef` + `Effect::new` + `request_animation_frame`
      in `RecentGameLogs` (scrolls `.recent-logs`) and `GameLogs` (scrolls
      `.game-meta-logs-content` via `parent_element()`).
- [x] **Page flash on command submit**: Outer `Suspense` → `Transition` for the
      game data resource. `Transition` keeps previous content visible during
      re-fetches; `Suspense` was blanking the screen on every WebSocket update.
- [x] **Undo log plain text**: Was inserting `'Game undone.'` directly. Fixed:
      `db::undo_game` takes `player_position: usize` and inserts
      `{{player N}} used an undo` markup, rendered as the player name in color.
- [x] **No UI update after command/undo/concede**: rust/web relied solely on the
      WebSocket round-trip for re-fetches. Fixed: increment `trigger.last_update`
      immediately in the client-side `Effect` when any server action returns
      `Ok(())`. WebSocket still fires for other players as before.

---

## Quick wins (added 2026-07-03)

### Mailpit replaces namshi/smtp [Superseded 2026-07-03 - do not implement]

Superseded the same day by the Phase 22a revision: outbound email sends via
the Resend HTTP API, not SMTP (DigitalOcean blocks outbound SMTP ports
25/465/587 by default; unblocking is a discretionary support request). With
no SMTP in the app at all, dev needs no SMTP catcher: the existing log
fallback prints emails when `RESEND_API_KEY` is unset, and `k8s/base/smtp/`
is deleted in Phase 22a rather than upgraded.

### Dependency automation + CI hygiene (added 2026-07-03 final pass)

Independent and delegable. Reduces ongoing maintenance cost with off-the-shelf
tooling; no phase dependencies.

- [ ] Renovate (Mend GitHub App, free for open source; `renovate.json` with
      `config:recommended`): automated dependency-update PRs across Cargo,
      Go modules, Dockerfiles, GitHub Actions, and kustomize image tags.
      `ignorePaths` the legacy `web/` npm tree (deleted at Phase 16 anyway).
      `devenv.lock` is not supported - `devenv update` stays manual.
- [ ] cargo-deny in CI (`deny.toml`): RustSec advisories, license
      compliance (aligns with the everything-open-source principle), and
      duplicate-dependency checks.
- [ ] kubeconform in CI: validate `kustomize build` output for `k8s/dev`
      and `k8s/prod` so manifest breakage is caught before apply.

---

## Phase 8: Redis pub/sub + web-legacy WS compatibility [Complete]

**Goal:** Replace the in-process `tokio::sync::broadcast` WebSocket fan-out
with Redis pub/sub, and publish legacy-compatible payloads so `web-legacy`
React clients receive correct real-time updates without re-engineering.

### Redis pub/sub

- [x] Add `redis` to `rust/web/Cargo.toml` (`tokio-comp` feature).
- [x] Replace `GameBroadcaster`: publish to `game.{id}` via Redis `PUBLISH`.
- [x] Subscribe each `/ws` handler to `game.*` via Redis `PSUBSCRIBE` and
      forward raw payloads to the connected client.
- [x] Remove `tokio::sync::broadcast` from `AppState` and `GameBroadcaster`.
- [x] Read `REDIS_URL` env var (matches legacy config, default `redis://redis`).

### Web-legacy WS compatibility (fat payload publishing)

The legacy API publishes to:
- `game.{game_id}` - public ShowResponse, broadcast to all watching a game
- `user.{user_auth_token_id}` - private ShowResponse with `command_spec`

The `user.{token_id}` channel is non-negotiable for web-legacy compat. The
React reducer (`web/src/reducers/game.ts` line 26-28) explicitly skips public
channel updates when the user already has a private game view loaded
(`existing.game_player && !g.game_player` → return). Without private per-player
messages, React users on the game page would never see cross-system moves.

`rust/web` now publishes the same legacy-format JSON to both channels on every
game event. The legacy React reducer reads `game`, `game_type`, `game_version`,
`game_players`, `game_logs`, `html`, `game_player`, `command_spec` - all
populated correctly. Web-legacy clients receive full real-time updates from
either system during Phase 16 side-by-side operation.

- [x] Added legacy serialization structs to `websocket.rs` matching the
      exact JSON shape of the old `rust/api` `ShowResponse`.
- [x] `broadcast_game_update` publishes to `game.{id}` (public, no
      `command_spec`) and per-player to `user.{auth_token_id}` (private,
      with player-specific `html` and `command_spec`).
- [x] Per-player logs use `db::get_game_logs` with `game_log_targets` join
      (same filter as display path - no info leak, correct private logs).

### Leptos-specific WS channel (eliminates re-fetch)

`rust/web` also publishes `BrdgmeUpdate` (a `GameViewData` + `Vec<GameLogEntry>`
struct) to `ws.{user_id}` per player. The `/ws` handler is session-aware:
subscribes to both `game.*` and `ws.{user_id}`. The Leptos client handles
`BrdgmeUpdate` by setting a context signal directly - no server function
re-fetch needed for game state or logs. `active_games` sidebar still re-fetches
(cheap DB-only query) via `last_update` increment.

- [x] `WebSocketMessage` enum has only `BrdgmeUpdate` variant.
- [x] `/ws` handler extracts session, subscribes to `ws.{user_id}` if logged in.
- [x] `GamePage` resource keyed on `game_id` only; WS signal takes precedence.
- [x] `GameLogs`/`RecentGameLogs` prefer WS logs when `game_id` matches.
- [x] Restart publishes `BrdgmeUpdate` for BOTH old game (with
      `restarted_game_id` set) and new game - no special `GameRestarted` message
      needed (the game record already carries `restarted_game_id`).

---

## Phase 9: LLM Bots [Complete]

**Status note:** v1 (HTTP triggering) is complete and working. The two
follow-ups formerly tracked inside this phase now have their own phases:
ELO ratings → Phase 12, NATS bot eventing → Phase 13.

**Goal:** Add bot players backed by an LLM via an OpenAI-compatible inference
API. Bots receive the rendered game state and available commands, produce a
command string, and submit it via the monolith. The inference provider runs
outside the cluster.

### Design decisions

- **API**: OpenAI-compatible (`POST /v1/chat/completions`). Works with any
  provider: local Ollama (OpenAI-compat endpoint), OpenRouter, Groq, etc. Env
  vars `LLM_URL` (base URL) and `LLM_API_KEY` (optional Bearer token). Model
  configurable via `BOT_MODEL` env var.
- **Current dev/test provider**: OpenRouter (`https://openrouter.ai/api`).
  Note: OpenRouter's path is `/api/v1/...` not `/v1/...`, so `LLM_URL` must
  include `/api`. Local Ollama uses `http://localhost:11434` (no `/api` prefix).
- **Current model**: `openai/gpt-5.4-nano` on OpenRouter.
  OpenRouter model IDs differ from Ollama model names - check
  `GET /api/v1/models` for available models.
- **Machine-local config**: `.env` file (gitignored) holds `LLM_URL`,
  `LLM_API_KEY`, `BOT_MODEL`, `RUST_LOG`. `.env.example` documents all vars.
  Tiltfile sources `.env` at runtime in the bot serve_cmd so changing `.env`
  and restarting the bot resource (not full Tilt restart) picks up new values.
- **Bot caller**: separate Knative Service (`rust/bot` crate). Receives a
  trigger from the monolith, assembles the prompt, calls the LLM, validates the
  response, and submits the command back to the monolith via an internal API
  key. Can scale to zero - only active during bot turns.
- **Internal auth**: monolith accepts an `X-Internal-Key` header (env var
  `INTERNAL_API_KEY`) on bot command submission, bypassing session auth. The
  bot caller and monolith share this key via a Kubernetes Secret.
- **Difficulty**: `easy`, `medium`, `hard`. Stored in `game_bots`. Controls
  a section of the system prompt describing the expected play style.
- **Bot player storage**: bots are a first-class concept via a `game_bots`
  table (not fake user records). `game_players.user_id` is nullable;
  `game_players.game_bot_id` is a nullable FK to `game_bots`. A CHECK
  constraint enforces exactly one is non-null.
- **Retry on invalid command**: up to 20 attempts. Each failed attempt appends
  the rejected command and validation error to the next prompt. If all retries
  fail, the bot caller logs the failure and does nothing - the turn remains
  with the bot.
- **Bot triggering**: v1 uses direct HTTP (monolith POSTs trigger to bot
  caller). Replacing with NATS eventing is the next planned task (see NATS
  bot eventing section below).
- **Rendered state**: the bot receives `player_renders[n].render` in raw brdgme
  markup format. `{{player N}}` references are resolved to player names;
  all other markup tags pass through unchanged. The markup is more compact and
  semantic than HTML - estimated 5-8x reduction in render section token count.
- **Prompt logging**: full rendered prompt is logged at `tracing::trace!` level.
  Set `RUST_LOG=info,bot=trace` in `.env` to enable.

### Game contract extension

- [x] Add `Rules` request/response variants to `rust/lib/cmd/src/api.rs`.
- [x] Implement `Rules` handler in `rust/lib/cmd/src/requester/gamer.rs`.
- [x] Add `fn rules() -> String` to `Gamer` trait in `rust/lib/game/src/game.rs`
      (default: empty string).
- [x] Add empty stub `fn rules()` to all 4 Rust games (acquire-1, lords-of-vegas-1,
      lost-cities-1, lost-cities-2). Rules text deferred - will use
      `include_str!("../rules.md")` pattern when written.

### Database changes

- [x] Migration `003_game_bots.sql`: `game_bots` table, nullable `user_id`,
      `game_bot_id` FK, XOR CHECK constraint. Migration applied to dev DB.
- [x] `GamePlayer.user_id: Option<Uuid>` in `rust/web/src/models/game.rs`.
- [x] `GameBot` struct added to `rust/web/src/models/game.rs`.
- [x] `GamePlayerExtended` updated: `user: Option<User>`, `game_bot: Option<GameBot>`,
      `name()` helper. In `rust/web/src/db.rs`.
- [x] `find_game_extended` and `find_active_games_for_user` queries updated to
      LEFT JOIN users + LEFT JOIN game_bots. SQLx cache regenerated.
- [x] All callsites updated: `mod.rs`, `server.rs`, `server_fns.rs`, `websocket.rs`.
      `user.id` accesses guarded with `as_ref().is_some_and()`. Bot players skipped
      for WS publishing. `name()` used everywhere instead of `user.name`.

### Monolith changes (`rust/web`)

- [x] `execute_command` refactored to take `player_position: usize` instead of
      `user_id`. `play_command` and `submit_command` server fn do a lightweight
      position lookup first.
- [x] `POST /api/internal/game/{id}/command` added to `server.rs`. Auth via
      `X-Internal-Key` header checked against `INTERNAL_API_KEY` env var.
      Calls `execute_command` with position from request body.
- [x] `trigger_bot_turns` helper added to `mod.rs`. Reads `BOT_SERVICE_URL`
      env var (disabled if unset). For each bot player with `is_turn = true`,
      spawns background `tokio::spawn` POSTing to `BOT_SERVICE_URL/trigger`.
- [x] `trigger_bot_turns` called from `execute_command` (after broadcast).
- [x] `trigger_bot_turns` called from `server.rs`: `create_game`, `undo_game`,
      `concede_game`, `restart_game`.
- [x] `trigger_bot_turns` called from `server_fns.rs`: `concede_game`,
      `restart_game`, `create_new_game` server fns.
- [x] New game creation with bot slots: `CreateGameOpts` extended with
      `bot_slots: &[BotSlot]`. Handler inserts `game_bots` rows then
      `game_players` rows with `game_bot_id` set and `user_id = NULL`.
- [x] New game UI: per-opponent slot Human/Bot toggle with name + difficulty
      fields. `opponent_emails` and `bot_slots` use `Option<Vec<_>>` to handle
      absent form fields in URL encoding.
- [x] `is_bot: bool` added to `PlayerViewData`; populated in both
      `server_fns.rs` and `websocket.rs`.
- [x] "Bump bot to play" link in `GameMeta` actions panel: shown when any bot
      player has `is_turn = true`. Dispatches `BumpBotTurns` server fn which
      calls `trigger_bot_turns`. Auth-gated to game players only.
- [x] Bot stale-turn race condition (two layers):
  - Pre-LLM: check `is_turn` from initial DB fetch; bail early if false.
  - Post-LLM: re-query `is_turn` and `game_state` after each LLM response.
    If not their turn: bail. If game state changed (e.g. undo): refresh render,
    logs, command spec via `load_bot_context` helper; reset conversation and
    retry LLM with fresh context. Attempt counter continues across refreshes.

### Bot caller service (`rust/bot`)

New Rust binary crate. Deployed as a Knative Service. Receives trigger POSTs
from the monolith, assembles the full prompt, calls the LLM, retries on
failure, then submits the command to the monolith's internal endpoint.

- [x] `rust/bot/` crate: minimal Axum server on port 4000, `POST /trigger`.
- [x] Request body: `{"game_id": "uuid", "player_position": 0, "difficulty": "medium"}`.
- [x] On trigger: fetch game + player names + logs from DB; call game service
      `Status` for render + command spec; call `Rules`; assemble prompt; POST to
      LLM `/v1/chat/completions`; retry up to 20 times on validation error; on
      hard failure log error and leave turn with bot.
- [x] `load_bot_context` helper: fetches render, command spec, logs. Called at
      start and on mid-loop state refresh.
- [x] Env vars: `DATABASE_URL`, `BOT_MODEL`, `LLM_URL`, `LLM_API_KEY`,
      `MONOLITH_URL`, `INTERNAL_API_KEY`. Dynamic sqlx queries (no .sqlx cache).
- [x] `rust/bot` Dockerfile target added to `rust/Dockerfile`.
- [x] `rust/bot` added to workspace `Cargo.toml`.

### Acquire rules [Complete]

- `rust/game/acquire-1/RULES.md` written: board layout, corporations, pricing,
  mergers, bonuses, rendering guide, command reference.
- `fn rules()` updated to `include_str!("../RULES.md").to_string()`.
- 2-player special rule corrected: dummy share count uses a D6 roll (1-6), not
  a drawn tile's column number (1-12). Rules now match the game implementation.
- 5 strategy notes added: 13-share majority guarantee, small-corp early
  investment, 4-share safe majority (bulk merge exception), capital management,
  portfolio diversification across corporations.

### Prompt structure [Complete]

`rust/bot/system_prompt.md` is a single MiniJinja template rendered at the
start of each LLM attempt. It covers the full prompt in one document:

1. **Persona**: expert board gamer, maximise fun and play to win.
2. **Task**: respond with exactly one plain-text command, no other text.
3. **Skill rating**: `{{ difficulty }}` injected; all three levels (easy/medium/
   hard) described so the model understands the full scale.
4. **brdgme markup legend**: static section documenting `{{b}}`, `{{fg rgb}}`,
   `{{bg rgb}}`, `{{player N}}` tags. Wrapped in `{% raw -%}...{%- endraw %}`
   to prevent minijinja treating the tag syntax as template variables. Player
   references are pre-resolved to names before the template is rendered.
5. **Command parser rules**: documentation for all 10 `Spec` variants (`Token`,
   `OneOf`, `Chain`, `Doc`, `Space`, `Opt`, `Enum`, `Int`, `Player`, `Many`)
   with YAML examples and plain-text command examples. Real Acquire `Buy`-phase
   spec included as a worked example.
6. **Game rules**: `{% if game_rules %}{{ game_rules }}{% endif %}` - omitted
   when the game has no rules text.
7. **Players**: `{% for player in players %}` loop with name, score, colour.
   Bot's own player marked `(you)`.
8. **Game render**: `{{ game_render }}` inside a ` ```text ` fence - raw brdgme
   markup with player refs resolved. `{{fg}}`/`{{bg}}` only support `rgb(r,g,b)`.
9. **Recent logs**: `{% for log in recent_logs %}` - raw brdgme markup.
10. **Command spec**: `{{ command_spec }}` inside a ` ```yaml ` fence -
    serialised via `serde_json::to_value` -> `serde_yaml::to_string` to produce
    mapping style (`Token: done`) rather than native YAML tags (`!Token`).
11. **Failed commands**: `{% if failed_commands %}` block listing each prior
    rejected command and its error. Omitted on the first attempt.

`rust/bot/src/prompt.rs` provides `markup_resolve_players`, `spec_to_yaml`,
`render_prompt`, and 14 unit tests covering all conditional sections, markup
rendering, player resolution, log rendering, player loop, and YAML spec format.

`BotContext` carries `render`, `command_spec_yaml`, `recent_logs`, and `points`
(scores). The retry loop accumulates `FailedCommand` entries and re-renders the
full template on each attempt rather than building a multi-turn conversation.

**KV cache restructure (future optimisation)**

The current design puts all content - including dynamic game state - into a
single system message re-rendered each turn. This is correct and simple, but
foregoes LLM prefix caching. A future restructure would split into multiple
messages with static content first (persona + parser docs → system; rules →
per-game-type user message) so the LLM can cache the long static prefix across
turns. Deferred until the bot is validated working well end-to-end.

### Bot k8s manifests

- [x] `k8s/base/bot/service.yaml`: Knative Service for `brdgme/bot` image.
      Reads `postgres-config` + `bot-config` secrets.
- [x] `k8s/base/bot/` added to `k8s/base/brdgme/kustomization.yaml`.
- [x] Tiltfile: bot `local_resource` in hybrid mode sources `.env` for
      `LLM_URL`, `LLM_API_KEY`, `BOT_MODEL`, sets `MONOLITH_URL` and
      `INTERNAL_API_KEY`. Web gets `BOT_SERVICE_URL` and `INTERNAL_API_KEY`.
      Full-cluster mode builds `brdgme/bot` and creates `bot-config` secret.

### LLM provider configuration

The bot uses any OpenAI-compatible provider via `LLM_URL` + `LLM_API_KEY`.
No in-cluster inference deployment.

**Dev (hybrid mode - default `tilt up`):**

`rust/bot` runs as a local process. `.env` sets `LLM_URL`, `LLM_API_KEY`,
`BOT_MODEL`. Currently using OpenRouter:

```
LLM_URL=https://openrouter.ai/api
LLM_API_KEY=<openrouter key>
BOT_MODEL=openai/gpt-5.4-nano
RUST_LOG=info
```

Note: OpenRouter's path is `/api/v1/...` so `LLM_URL` must include `/api`.
For local Ollama use `LLM_URL=http://localhost:11434` (no `/api` prefix).

`BOT_SERVICE_URL` must point to the local bot process (default: `http://localhost:4000`);
if unset, bot auto-triggering is disabled. The Tiltfile `web` serve_cmd sets
`BOT_SERVICE_URL=http://localhost:4000` and uses `-f .mirrord/mirrord.json` to
ensure `ignore_localhost: true` is applied - without this mirrord routes
localhost through the cluster pod and bot trigger requests fail.

**Prod:**

Uses an OpenAI-compatible cloud provider. Set `LLM_URL`, `LLM_API_KEY`,
`BOT_MODEL` in the `bot-config` Kubernetes Secret.

**Home GPU over Tailscale (prod alternative):**

Ollama running on a home GPU, exposed via Tailscale. Set `LLM_URL` to the
Tailscale IP/hostname. No `LLM_API_KEY` needed (Tailscale handles network auth).

### Bot service logging improvements [Complete]

Structured logging at key points in `run_bot_turn`. A `trace_id`
(`Uuid::new_v4()`) is generated at the start of each trigger request and
threaded through all log entries so concurrent bot turns can be correlated.

- **On trigger received**: `tracing::info!` with trace_id, game_id, game name,
  player position, player name, difficulty.
- **On each LLM response**: `tracing::trace!` with trace_id, attempt number,
  full rendered prompt (visible at `RUST_LOG=bot=trace`).
- **On command submitted**: `tracing::info!` with trace_id, attempt, command,
  HTTP status. `tracing::warn!` if rejected, with validation error body.
- **On retry**: `tracing::warn!` with trace_id, attempt, validation error.
- **On state change mid-loop**: `tracing::warn!` with trace_id, attempt.
- **On hard failure**: `tracing::error!` with trace_id, error chain (Debug),
  attempt count.

All fields are structured key-value pairs for machine-parseable log aggregation.

Monolith `trigger_bot_turns` also logs: `tracing::debug!` per player (position,
is_turn, is_bot), `tracing::info!` on trigger fire (position, difficulty, url),
and tokio-spawned `tracing::warn!`/`tracing::debug!` on HTTP error/success.

### Notes

- The bot caller does not need a game session. It authenticates to the monolith
  via `INTERNAL_API_KEY` only.
- Bot players have no `users` row. `game_players.user_id` is null for bots;
  `game_players.game_bot_id` points to the `game_bots` row for that game.
- mirrord intercepts all outgoing TCP including `localhost`. Both `rust/web` and
  `rust/bot` have `ignore_localhost: true` in their `.mirrord/mirrord.json`
  configs so localhost connections (LLM on 11434, bot on 4000) bypass the proxy
  and use the host network directly. The Tiltfile web `serve_cmd` must pass
  `-f .mirrord/mirrord.json` explicitly; without it mirrord may not load the
  config file and bot trigger requests to localhost:4000 will fail.
- Bot restart limitation: `restart_game` only collects human `opponent_ids`;
  bots are not recreated in the restarted game.
- XOR CHECK constraint on `game_players` enforces exactly one of `user_id` /
  `game_bot_id` is non-null.
- `BotSlot` is defined in `game/server_fns.rs` (not SSR-gated) and re-exported
  from `db.rs`. It must compile on WASM as a server fn parameter.
- The GameVersion CRD is managed outside Tilt in `setup-kind-cluster.sh` to
  avoid finalizer deadlocks. Do not add it back to any kustomization.


---

## Phase 10: Eliminate runtime panics in rust/web [Complete]

**Goal:** Replace all panic-prone code in `rust/web/src` that could crash the
server process or the WASM frontend at runtime with proper error handling.
Startup panics (`main.rs`, `auth/session.rs`, `db.rs` env var) are
intentional and excluded.

**Background:** An audit of `rust/web/src` found 47 instances of `.unwrap()`,
`.expect()`, `unreachable!()`, and `panic!()`. Most are either in tests or in
`#[cfg(not(feature = "ssr"))]` stubs (correct). The following are runtime risks:

### Cases to fix

- [x] **`db.rs:407` - `games.last_mut().unwrap()`** (`find_active_games_for_user`):
  replaced with `.ok_or_else(...)` returning a descriptive `anyhow::Error`
  instead of panicking.

- [x] **`db.rs:250-261` and `db.rs:393-404` - co-nullable LEFT JOIN unwraps**:
  extracted `build_user_from_row` and `build_game_bot_from_row` helpers
  (`db.rs`) using `ok_or_else` + `?`; both call sites (`find_game_extended`,
  `find_active_games_for_user`) now propagate an `anyhow::Error` instead of
  panicking if a LEFT JOIN row is malformed.

- [x] **`app.rs:112` and `app.rs:119` - `NodeRef::get().unwrap()` in form
  submit handlers**: replaced with `.get().map(|el| el.value())` +
  `let...else` that logs a warning via `leptos::logging::warn!` and returns
  early on `None`.

- [x] **`websocket_client.rs:21-23` - `window()`, `protocol()`, `host()`
  `.expect()` calls**: `window()` kept as `.expect()` (guaranteed in WASM).
  `protocol()` falls back to `"ws:"` with a logged warning on `Err`; `host()`
  logs a warning and aborts the connect attempt on `Err` (no valid URL can be
  built without it).

### Excluded (intentional)

- `main.rs`, `auth/session.rs`, `db.rs:55`: startup failures where the
  process cannot run without the resource. Panicking at boot is correct.
- `game/client.rs`: all within `#[cfg(test)]`. Panics in tests are fine.
- `server_fns.rs` `unreachable!()` in `#[cfg(not(feature = "ssr"))]` stubs:
  these paths are never compiled into the server and are never reachable on
  the client, so they are correct as-is.
- `components/game.rs:373` - `values.into_iter().next().unwrap()`: guarded by
  `values.len() == 1` on the preceding line; `unwrap()` is provably safe.

---

## Phase 11: Testing Foundation [Pending]

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
- [ ] After Phase 12: rating updates asserted here too (human-only game
      rated; game with a bot player not rated).
- [ ] After optimistic locking lands: concurrent-write conflict returns the
      conflict error and preserves the first write.

Implemented as `rust/web/src/game/mod.rs::tests` (8 tests). The last two
items are deferred pending their respective phases, as originally scoped -
not omissions.

### 11.4 Handler auth tests (Axum `tower::ServiceExt::oneshot`) [Complete]

- [x] `POST /api/internal/game/{id}/command`: correct `X-Internal-Key` →
      executes; wrong key → 401; missing key → 401; `INTERNAL_API_KEY` env
      unset → rejects all.
- [x] `GET /api/game/{id}` with no session → 401.
- [x] `POST /api/game/{id}/command` with no session → 401; with a session for
      a non-player → 403.
- [x] Login flow logic (`auth/server.rs`, function-level): invalid email
      rejected; valid email creates a user and sets a 6-digit confirmation
      token; wrong code and expired confirmation are both rejected before the
      session step. `login`/`confirm_login` invoked directly in a
      `leptos::reactive::owner::Owner` scope with `PgPool` provided via
      context (no HTTP layer needed - the `#[server]` macro body is callable
      as a plain async fn). `confirm_login`'s "creates a session user" path
      needs a Leptos request-scoped `Parts` context (only available in a real
      request), so it isn't asserted end-to-end here; E2E covers that.

Implemented as `rust/web/src/game/server.rs::tests` (4 tests) and
`rust/web/src/auth/server.rs::tests` (4 tests). Added `tower = { features =
["util"] }` for `oneshot`/`ServiceExt` in tests.

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

### 11.6 E2E smoke suite (Playwright)

Purpose: catch hydration panics and full-stack wiring breaks. Keep it small
and fast (target < 5 minutes); it is a smoke suite, not a regression suite.

Environment (scripted in `e2e/`): Postgres container + one game service
(`lost_cities_2_http` binary) + the release-built web binary
(`cargo leptos build --release`). Seed `game_types`/`game_versions` rows by
SQL pointing at the local game service (no operator needed). SMTP unset;
tests read `login_confirmation` from the DB to complete login.

**Delegation gap (11.6 only):** the scenario list below is ready, but the
harness itself needs a design pass first: process management for the three
services (ports, startup ordering, readiness checks, teardown), the exact DB
seed SQL, where the Playwright project lives relative to the Rust workspace,
and how CI caches the release build to keep the job under budget.

- [ ] Harness + stack boot script + DB seed.
- [ ] Hard-load `/`, `/login`, `/dashboard`, `/games`, and an active
      `/game/{id}` - assert zero browser console errors (hydration panics
      surface via `console_error_panic_hook`).
- [ ] Full game flow with two browser contexts: create a 2-player human game,
      player 1 submits a valid command, player 2 sees the updated render via
      WebSocket without reloading.
- [ ] Invalid command shows the error message; input is not cleared.
- [ ] Undo; concede (accept the confirm dialog); restart navigates the
      restarting player to the new game.
- [ ] Hard refresh on the game page mid-game - the highest-risk hydration
      scenario (real async data + Suspense).
- [ ] CI: separate job (needs the release build); run on pull requests if the
      runtime allows, otherwise on master + nightly.

### 11.7 Testing conventions (add to `docs/CODING.md`)

- [ ] New or changed logic in `db.rs`, `game/mod.rs`, and `auth/` must land
      with tests. Reviewers/agents reject changes to these files without them.
- [ ] Game service HTTP is always mocked in `rust/web` tests; the LLM is
      never called in any test.
- [ ] Use `#[sqlx::test]` for anything touching the DB; never share state
      between tests.
- [ ] E2E scenarios are added only for user-visible flows, and the suite must
      stay under its time budget.

**Build order for delegation:** 11.1 → 11.2 → 11.3 → 11.4 → 11.5 → 11.6.
Each sub-phase is independently delegable once 11.1 is merged. Phase 12 (ELO)
should be implemented after 11.2 so it lands with tests.

---

## Phase 12: ELO Ratings [Complete except backfill decision - 2026-07-03]

**Why blocking:** the legacy `rust/api` updates ratings when a game finishes;
`rust/web` does not. Both systems share the DB during side-by-side operation,
and the legacy idempotency guard (skip if any player already has
`rating_change` set) means a game finished via `rust/web` is never rated - not
even retroactively. Every game finished through the new system before this
lands is permanently unrated. Decided 2026-07-02: implement before the new
system serves real games in production (not yet deployed, so no live bleed).

**Reference implementation:** `rust/api/src/db/query/mod.rs:718-846`
(`update_game_placings`, `elo_rating_change`, `elo_expected_score`). Port the
logic, do not redesign it.

**Algorithm (from legacy, keep identical for human-only games):**
- Runs when a game transitions to `Finished` with non-empty `placings`.
- Idempotency guard: skip entirely if any `game_players.rating_change` is
  already non-null for the game.
- For every unordered pair of players (a, b): score `a_score` = 1.0 if a placed
  better (lower placing) than b, 0.5 if equal, 0.0 if worse.
- `expected = 10^(a_rating/400) / (10^(a_rating/400) + 10^(b_rating/400))`
- `change = round(K * (a_score - expected))` with `K = 32.0`; add `change` to
  a's accumulator and subtract from b's.
- Ratings come from `game_type_users` (create the row with default rating 1500
  if missing - `rust/web` currently only fabricates a default in memory on
  read; the write path must INSERT). Implemented as a bare
  `INSERT ... ON CONFLICT DO NOTHING` with no explicit rating column, so the
  actual DB column default (1200, per `game_type_users.rating integer DEFAULT
  1200`) applies - matching `create_game_with_users`'s existing insert
  pattern and legacy's own NULL-lets-column-default-apply behavior. The 1500
  figure here matches only the in-memory fallback in `build_game_type_user`
  used for display when no row exists yet, not the real column default.
- Apply accumulated changes to `game_type_users.rating` and store per-player
  `game_players.rating_change`. Skip zero changes.
- Legacy never updates `peak_rating` (writes only NULL on creation). Optional
  improvement while here: `peak_rating = GREATEST(peak_rating, new_rating)`.

**New rule (not in legacy - legacy predates bots):** any game that includes at
least one bot player must not affect any rating. Leave `rating_change` NULL for
all players in such games. Only human-vs-human games are rated.

**Also broken - concede path:** legacy `concede_game`
(`rust/api/src/db/query/mod.rs:572`) assigns placings (non-conceder 1,
conceder 2) and rates the game. `rust/web`'s `db::concede_game` (db.rs:688)
only sets `is_finished = true`: conceded games get no `place`, no
`rating_change`, no rating update. Fix as part of this task.

**Tasks:**
- [x] Add rating update to the finish path of `update_game_command_success`
      (or a helper it calls) in `rust/web/src/db.rs`, inside the same
      transaction as the placings write.
- [x] Fix `db::concede_game` to write `game_players.place` (non-conceder 1,
      conceder 2, matching legacy) and run the same rating update helper.
      (`concede_game` already wrote `place`; only the rating call was
      missing.)
- [x] Port `elo_rating_change` + `elo_expected_score` + the
      `elo_rating_change_works` unit test from `rust/api`.
- [x] INSERT `game_type_users` row when missing, ON CONFLICT DO NOTHING (bare
      column-list insert, matching the existing `create_game_with_users`
      pattern - the DB column default is 1200, not 1500; see report).
- [x] Skip rating updates entirely when any `game_players.game_bot_id` is
      non-null in the game.
- [x] Regenerate SQLx offline metadata (`cargo sqlx prepare -- --features ssr`).
- [x] Tests: unit tests for the pairwise math (ported `elo_rating_change_works`
      plus a 3-player pairwise case); `#[sqlx::test]` integration: 2-player and
      3-player games rated correctly on finish; idempotency guard (second
      finish write does not re-rate); game with a bot player not rated;
      `game_type_users` row created on first rated game; concede assigns
      places and rates.
- [ ] Decide whether to backfill unrated games finished via `rust/web` before
      this change (list: finished games where all `rating_change` are NULL and
      no bot players). Optional - low game volume may not justify it.

---

## Phase 13: NATS Bot Eventing [Pending]

v1 bot triggering uses direct HTTP (monolith POSTs to bot service, bot POSTs
command back to monolith). This creates bi-directional HTTP coupling: the bot
needs `MONOLITH_URL` and `INTERNAL_API_KEY` just to submit a move. Replace with
NATS eventing.

**Precondition:** Phase 14 (drop Knative) runs first - the bot becomes an
always-on Deployment, which is what lets it hold a NATS subscription at all.
The former scale-to-zero-vs-subscriber conflict flagged here is resolved by
that decision (2026-07-03).

**Decisions resolved 2026-07-03 (tech review):**
- **Delivery guarantees: JetStream from day one.** NATS Core is
  at-most-once; a `bot.turn` lost during a bot deploy or NATS restart is a
  stuck turn until a human clicks "bump". JetStream makes bot eventing
  at-least-once for the cost of a server config flag and a small PVC. WS
  fan-out (Phase 17) deliberately stays Core pub/sub - ephemeral is correct
  there.
- **Stream design:** one stream `BOT` capturing `bot.>`, WorkQueue
  retention, two durable pull consumers with non-overlapping filters:
  `bot-turn` (filter `bot.turn`, fetched by the bot) and `bot-command`
  (filter `bot.command`, fetched by monolith replicas). Explicit ack after
  processing; `ack_wait` ~5 min (a turn including LLM retries must complete
  or be redelivered); `max_deliver: 3` as a poison-message backstop. Stream
  and consumers are created idempotently by the monolith on startup
  (async-nats jetstream API), not by manifests.
- **`k8s/base/nats/` manifests:** official `nats:2.11-alpine` image,
  StatefulSet with 1 replica + 1Gi PVC (JetStream file store), JetStream
  enabled via config, ClusterIP Service on 4222. No auth in-cluster
  (consistent with the Redis/Postgres posture). Monitoring port 8222
  exposed for the readiness probe (`/healthz`).
- **Attempt limits:** 20 LLM attempts per turn (unchanged); 3 turn-level
  re-publishes on state conflict (`attempt` field); re-publish immediately,
  no delay (conflicts are rare and the LLM loop itself is slow).

**Delegation gap (remaining):**
- **Rollout sequencing:** big-bang swap or both paths behind an env flag
  during transition; ordering relative to Phase 16 cutover.
- **Test plan:** how the new flow is tested (NATS service container in CI,
  mocked LLM, assertions for the conflict/re-publish path).

**Design:**

```
Monolith  --[bot.turn]--> NATS
Bot       <-- subscribes to bot.turn
Bot       --> DB (fetch game state + game service URI)
Bot       --> game service Status (render + command_spec)
loop:
  Bot     --> LLM (get command)
  Bot     --> game service Play (validate - stateless, no DB commit)
  if invalid: accumulate FailedCommand, retry LLM
  if valid: break
Bot       --[bot.command]--> NATS
Monolith  <-- subscribes to bot.command
Monolith  --> game service Play + DB save
if stale state: Monolith --[bot.turn]--> NATS (increment attempt counter)
```

Key design decisions:
- **Bot validates against game service directly.** `Play` calls are stateless
  (return new state but don't persist). The bot can use them to validate without
  side effects. This keeps the retry loop entirely inside the bot with no
  monolith round-trip per attempt.
- **State conflict handled by monolith.** If the game state changes between the
  bot's validation and the monolith's commit (e.g. undo), the monolith detects
  the conflict and re-publishes `bot.turn`. An attempt counter in the event
  payload provides an overall retry limit.
- **Bot loses HTTP server entirely.** No `/trigger` endpoint, no `MONOLITH_URL`,
  no `INTERNAL_API_KEY`. The bot's only dependencies become DB, game service,
  LLM provider, and NATS.
- **Monolith loses `BOT_SERVICE_URL`.** `trigger_bot_turns` is replaced by a
  NATS publish. No outbound HTTP to bot service.

**NATS subjects:**
- `bot.turn` — payload: `{game_id, player_position, difficulty, attempt}`
- `bot.command` — payload: `{game_id, player_position, command}`

**Exactly-one-instance delivery (required for correctness):** each message
must be processed by exactly one subscriber instance - the monolith runs
multiple replicas, and a plain subscribe to `bot.command` would execute every
bot command once per replica. With JetStream durable pull consumers (decided
2026-07-03, above) this falls out naturally: all replicas fetch from the same
durable consumer, each message goes to exactly one fetcher, and a missed ack
triggers redelivery.

**Tasks:**

Infrastructure (pulled forward from Phase 17 - NATS is needed here regardless
of when the WS migration happens):
- [ ] Add NATS (JetStream enabled) to the Kind cluster: `k8s/base/nats/`
      manifests per the resolved design above.
- [ ] Add NATS to `k8s/base/brdgme/kustomization.yaml`.
- [ ] Add NATS to Tiltfile (deploy + port-forward).
- [ ] Add `async-nats` to `rust/web/Cargo.toml` and `rust/bot/Cargo.toml`.
- [ ] Add `NATS_URL` env var to monolith and bot (Tiltfile + k8s secrets).

Monolith changes:
- [ ] Replace `trigger_bot_turns` HTTP POST with NATS publish to `bot.turn`.
- [ ] Remove `BOT_SERVICE_URL` env var.
- [ ] Subscribe to `bot.command` on startup; handler calls `execute_command`
      and saves to DB. On stale state conflict: re-publish `bot.turn` with
      `attempt` incremented. Enforce overall attempt limit (e.g. 3 turn-level
      retries before giving up).
- [ ] Remove `POST /api/internal/game/{id}/command` endpoint and
      `INTERNAL_API_KEY` (no longer needed for bot auth).

Bot changes:
- [ ] Remove Axum HTTP server (`/trigger` endpoint, port 4000).
- [ ] Subscribe to `bot.turn` on startup; process each message as a turn.
- [ ] Replace game service `Status` + LLM + monolith POST retry loop with:
      Status → LLM → game service `Play` (validate) → retry LLM on
      invalid → publish `bot.command` when valid.
- [ ] Remove `MONOLITH_URL` and `INTERNAL_API_KEY` from `AppState` and env.
- [ ] Update k8s `bot-config` secret to remove those vars, add `NATS_URL`.

---

## Phase 14: Drop Knative - Plain Deployments + Gateway API [Pending]

**Decision (2026-07-03):** remove Knative Serving and Kourier entirely.
Knative is healthy as a project (CNCF graduated 2025-09, quarterly releases -
this is not a Rocket-style maintenance risk), but it is a poor fit at brdgme's
scale:

- The Serving control plane requests ~630m CPU / ~400Mi (activator,
  autoscaler, autoscaler-hpa, controller, webhook) plus Kourier plus a
  queue-proxy sidecar in every pod. The workloads it scales to zero - ~20
  idle Go/Rust game services at roughly 5-25Mi RSS each - cost less to just
  run always-on. Scale-to-zero is negative-value at this scale.
- Turn-based ASCII games have no load spikes worth request-based autoscaling.
- DOKS now provides a managed Gateway API implementation on Cilium
  (pre-installed on clusters >= 1.33 with VPC-native networking,
  auto-provisions the DO load balancer, no controller to run). This replaces
  Kourier + DomainMapping with standard `Gateway`/`HTTPRoute` resources.
- Removes dev-environment complexity that exists only for Knative: the local
  registry digest-resolution requirement, the `k8s_kind('Service', ...)` Tilt
  workaround, the Kourier NodePort patch, and the Knative install in
  `setup-kind-cluster.sh`.
- Makes the bot an always-on Deployment, which resolves the Phase 13
  scale-to-zero vs NATS-subscriber conflict.

Alternatives considered: KEDA + HTTP add-on (add-on still beta v0.15.x with
an interceptor proxy in the request path; saves nothing vs always-on here;
KEDA's JetStream scaler remains the right tool if a genuinely heavy
scale-to-zero consumer ever appears, e.g. in-cluster LLM inference); FaaS
frameworks (wrong packaging model). Eventing is unaffected: NATS Core was
already chosen over Knative Eventing.

**Sequencing:** run this phase before Phase 13 (NATS bot eventing), Phase 15
(ArgoCD), and Phase 16 (cutover), so manifests are rewritten once and the
ArgoCD config repo + cutover/rollback procedures are written against the
final infrastructure.

**Current Knative surface (audited 2026-07-03):**
- ksvc manifests: `k8s/base/web/service.yaml` (minScale 1),
  `k8s/base/bot/service.yaml`, and the legacy trio `k8s/base/web-legacy/`,
  `k8s/base/api/`, `k8s/base/websocket/`.
- `k8s/base/domain-mapping/` - 4 DomainMappings with
  `networking.knative.dev/certificate-class: cert-manager.io` annotations.
- `k8s/prod/knative-serving/` - config-domain, config-certmanager,
  config-network patches.
- `k8s/base/cert-manager/cluster-issuer.yaml` - HTTP01 solver bound to the
  Kourier ingress class.
- `scripts/setup-kind-cluster.sh` - Knative Serving + Kourier install, webhook
  waits, Kourier NodePort 31080 patch.
- `Tiltfile` - `k8s_kind('Service', api_version='serving.knative.dev/v1', ...)`,
  mode comments, `*.brdgme.lvh.me:8080` links that route through Kourier.
- Game services are already plain Deployments + NodePort Services - unaffected
  by this phase apart from optional Service-type cleanup.

### Manifests

- [ ] `k8s/base/web/`: replace ksvc with a Deployment (target `replicas: 2`
      to match the multi-replica vision; `replicas: 1` acceptable while the
      cluster is small - Redis/NATS fan-out already handles multi-replica)
      plus a ClusterIP Service on port 3000. Preserve the existing env/secret
      wiring.
- [ ] `k8s/base/bot/`: replace ksvc with a Deployment (`replicas: 1`,
      always-on) plus a ClusterIP Service on port 4000. Preserve
      `postgres-config`/`bot-config` secret refs and `LISTEN_ADDR`.
- [ ] Legacy trio (`web-legacy`, `api`, `websocket`): ksvc → Deployment +
      Service. Temporary manifests - deleted at Phase 16 decommission - so
      minimal effort, no replicas tuning.
- [ ] Delete `k8s/base/domain-mapping/`. Create `k8s/base/gateway/`: one
      `Gateway` with an HTTPS listener per hostname (`brdg.me`,
      `legacy.brdg.me`, `api.brdg.me`, `ws.brdg.me`) and one `HTTPRoute` per
      hostname routing to the matching Service. Update
      `k8s/prod/app/kustomization.yaml` (currently includes
      `../../base/domain-mapping`).
- [ ] Delete `k8s/prod/knative-serving/` and remove it from
      `k8s/prod/kustomization.yaml`.
- [ ] cert-manager for Gateway API: enable the Gateway API feature
      (`config.enableGatewayAPI: true` / `--enable-gateway-api`), annotate the
      `Gateway` with `cert-manager.io/cluster-issuer`, switch the
      ClusterIssuer HTTP01 solver from the Kourier ingress class to
      `gatewayHTTPRoute` solvers referencing the Gateway.
- [ ] Optional cleanup: game service Services from NodePort → ClusterIP (only
      the monolith calls them in-cluster).

### Dev environment (Kind)

- [ ] `scripts/setup-kind-cluster.sh`: remove the Knative Serving + Kourier
      install blocks, webhook waits, and Kourier NodePort patch. Keep Cilium
      (CNI), the GameVersion CRD, and the Kind cluster/registry logic.
- [ ] Enable Cilium Gateway API in Kind: install the Gateway API CRDs and set
      `gatewayAPI.enabled=true` in the Cilium install values. Expose the
      Gateway via NodePort 31080 (already mapped to host 8080 in
      `k8s/kind-config.yaml` `extraPortMappings`) to preserve the
      `{service}.brdgme.lvh.me:8080` dev URLs. If Cilium's Gateway NodePort
      exposure proves awkward in Kind, fall back to Tilt port-forwards and
      update DEV.md accordingly - decide during implementation.
- [ ] `Tiltfile`: remove the `k8s_kind('Service', ...)` registration; update
      the mode comments; verify `WEB_IN_CLUSTER=1` and `LEGACY=1` modes
      deploy and route correctly.
- [ ] Local registry: no longer mandatory (the digest-resolution requirement
      was Knative's) but kept - it is faster than `kind load`. Replace the
      hand-rolled cluster + registry bootstrap (the docker run / network
      connect / KEP-1755 ConfigMap blocks in `setup-kind-cluster.sh`) with
      ctlptl (Tilt-team tool; decided 2026-07-03): a committed `ctlptl.yaml`
      declares the Kind cluster (default CNI disabled) and `kind-registry`.
      Add `ctlptl` to `devenv.nix`. The script shrinks to: `ctlptl apply` →
      Cilium install (Gateway API enabled) → GameVersion CRD.
- [ ] Verify hybrid mode is unaffected (web/bot run as local processes;
      game services and backing services unchanged).

### Prod prerequisites

- [ ] Verify the DOKS cluster is >= Kubernetes 1.33 with VPC-native
      networking so the managed Gateway API (GatewayClass) is available;
      plan a cluster upgrade if not.
- [ ] Confirm behaviour of the auto-provisioned DO load balancer for
      WebSockets: long-lived connection support and idle timeout
      configuration (the monolith holds a WS per connected client).
- [ ] Remove the Knative/net-certmanager one-time `kubectl apply`
      prerequisites from the Phase 16 notes; cert-manager alone remains.

### Verification

- [ ] Kind, full-cluster mode (`WEB_IN_CLUSTER=1`): web reachable through the
      Gateway; login, game creation, command flow, and a WebSocket session
      all work through the Gateway route.
- [ ] Kind, `LEGACY=1`: legacy trio reachable via their hostnames.
- [ ] Prod TLS issuance (HTTP01 through the Gateway) is verified as part of
      the Phase 16 cutover checklist, not here.

### Docs

- [ ] Update `docs/DEV.md`: Kourier/Knative references, lvh.me routing
      explanation, setup script description.
- [ ] `docs/VISION.md` and `docs/ARCHITECTURE.md` already reflect the target
      state (updated 2026-07-03).
- [ ] Update the operator long-term goal wherever stated: it manages
      Deployment/Service lifecycle per game version, not Knative Services.

---

## Phase 15: Production CD (ArgoCD) [Pending]

**Goal:** Replace manual `kubectl apply -k k8s/prod` with ArgoCD for GitOps
continuous delivery. GitHub Actions handles CI (build + push to GHCR). ArgoCD
handles CD (sync cluster state to Git). Database migrations run as an ArgoCD
PreSync hook so a failed migration halts the sync before any pods are replaced.

**Delegation gap:** most of this phase needs production cluster credentials
and judgement calls - treat it as human-operated with agent assistance, not
delegable. Before delegating even the assistable parts, specify:
- **`brdgme-config` repo layout:** exact directory structure, what is copied
  from `k8s/prod`, and how per-service image tags are pinned/edited.
- **GitHub Actions deploy step:** the workflow changes (job YAML), which
  secrets/deploy keys exist and how they are provisioned.
- **ArgoCD exposure:** LoadBalancer vs Ingress vs port-forward-only admin
  access, domain, and TLS.
- **PreSync verification procedure:** concrete steps to prove a failing
  migration halts the sync (e.g. a deliberately broken migration in a
  throwaway branch) - "verify" currently has no procedure.

### ArgoCD installation (production cluster)

- [ ] Install ArgoCD into the production cluster via the official manifest:
      `kubectl apply -n argocd -f
      https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml`
- [ ] Expose the ArgoCD API server (LoadBalancer or Ingress).
- [ ] Store the initial admin password securely and rotate it.

### ArgoCD Application manifest

- [x] Create `k8s/argocd/brdgme-app.yaml`: an `Application` resource pointing
      to this repo, `k8s/prod` kustomize path, auto-sync enabled, prune
      enabled, self-heal enabled.
- [x] Commit the `Application` manifest to the repo so ArgoCD manages itself
      (app-of-apps pattern is not needed at this scale - a single Application
      is sufficient).

### Database migration PreSync hook

- [x] Create `k8s/base/migrate/job.yaml`: a `Job` that runs
      `sqlx migrate run` using the `brdgme/migrate` image (dedicated
      Dockerfile target in `rust/Dockerfile`) and the `postgres-config` secret.
      Annotate with:
      - `argocd.argoproj.io/hook: PreSync`
      - `argocd.argoproj.io/hook-delete-policy: BeforeHookCreation`
- [x] Add `k8s/base/migrate/` to `k8s/base/brdgme/kustomization.yaml`.
- [ ] Verify: a failed migration halts the ArgoCD sync and leaves the running
      pods untouched.

### Secrets management: sealed-secrets (added 2026-07-03)

GitOps makes the config repo the source of truth, but the app secrets
(`postgres-config`, `bot-config`, `INTERNAL_API_KEY`, and later the
external-dns DO token) currently exist only as manually created cluster
Secrets - previously unaddressed. Decision: bitnami-labs/sealed-secrets.
Asymmetric encryption; `SealedSecret` CRs are safe to commit; no external
store. (External Secrets Operator rejected - no external secret store
exists to back it; SOPS+age rejected - key distribution and editor
integration overhead for a solo operator.)

- [ ] Install the sealed-secrets controller into the prod cluster via
      kustomize in `brdgme-config` (ArgoCD-managed like everything else).
- [ ] Add `kubeseal` to `devenv.nix`.
- [ ] Convert each prod secret to a `SealedSecret` committed to
      `brdgme-config`; delete the manually created Secrets once the
      controller has unsealed replacements.
- [ ] Back up the controller's sealing key pair to the same offline store as
      other cluster credentials - losing it means re-sealing everything.
- [ ] Dev unaffected: Tilt continues creating plain Secrets in Kind.

### Image update flow (separate config repo)

Image tags are tracked in a dedicated `brdgme-config` repo (separate from this
source repo). GitHub Actions pushes images to GHCR then commits the updated
tags to `brdgme-config`. ArgoCD watches `brdgme-config`, not this repo.

Rationale: committing tags back to the source repo creates CI loop risk and
mixes deployment history with code history. A separate config repo keeps
rollback simple (revert the tag commit, ArgoCD syncs) without any additional
tooling. If a more integrated official mechanism ships with ArgoCD in future it
should be evaluated then. (Evaluated 2026-07-03: ArgoCD Image Updater remains
argoproj-labs, v1.1.x, explicitly not recommended for critical production
workloads, and not merged into core - the custom Actions step stands.)

- [ ] Create a `brdgme-config` repository containing the `k8s/prod` kustomize
      manifests (copy from this repo). This becomes the single source of truth
      for what is running in production.
- [ ] Update the ArgoCD `Application` to point to `brdgme-config` instead of
      this repo.
- [ ] Add a GitHub Actions deploy step: after pushing images to GHCR, clone
      `brdgme-config`, run `kustomize edit set image` for each updated image,
      commit, and push. ArgoCD auto-sync picks up the change.
- [ ] Grant the GitHub Actions bot write access to `brdgme-config` via a
      deploy key or fine-grained PAT scoped to that repo only.
- [ ] To roll back: revert the relevant commit in `brdgme-config`. ArgoCD
      syncs to the previous tag. No tooling changes required.

### Notes

- Migrations are forward-only. A migration that removes a column still read
  by the running version will break live traffic. Use expand/contract: add the
  new column in one deploy, remove the old column in a later deploy after all
  pods are on the new version.
- ArgoCD does not replace Tilt for local dev. Tilt remains the dev environment
  tool; ArgoCD is production-only.

---

## Phase 16: Production Cutover & Side-by-Side Validation [Pending]

**Goal:** Run old and new systems simultaneously against the same database so
they can be compared directly before committing to cutover. Legacy services
(`rust/api`, `web`, `websocket`) are kept alive until `rust/web` is proven in
production.

**Delegation note:** this phase is operator-driven by nature (production
deploys, DNS, live verification) - not agent-delegable. The two
agent-delegable subtasks are the `http.ts` apex-domain verification (in the
rollback section below, ready now) and the final source/manifest deletion in
the decommission list (ready once the validation gate passes).

Both systems share PostgreSQL, Redis, and the game microservices. Auth
mechanisms are different (Bearer token vs session cookie) so each requires a
separate login - this is acceptable for testing. Both systems publish to Redis
`game.{id}` and `user.{token_id}` channels, so a move in either UI triggers
correct real-time WebSocket updates for clients on the other system.

**Note:** If a move is made via the legacy `rust/api`, the rust/web Leptos
frontend will not receive a `ws.{user_id}` update (the old api does not publish
to that channel). The game page will show stale state until manual refresh.
This is acceptable for the validation period.

### Risks

- `web/Dockerfile` bumped to `node:20` (was `node:14.7.0`, EOL). Build
  verified working.

### Image naming

The old React frontend (`web/Dockerfile`) and the new Leptos SSR app
(`rust/Dockerfile` `web` target) previously shared the image tag `brdgme/web`.
The new Leptos app keeps `brdgme/web`. The old React frontend is renamed to
`brdgme/web-legacy`.

### Infra changes needed

**Superseded note (2026-07-03):** the checked items below referencing Knative
(`DomainMapping`, `config-domain`, Kourier TLS, `net-certmanager`) were built
before the Phase 14 decision to drop Knative. Phase 14 replaces them with
plain Deployments + Gateway API `HTTPRoute`s + cert-manager Gateway
integration. The hostname table and the validation/rollback/decommission
sections below remain correct; only the routing mechanism changed.

- [x] New Leptos app: `rust/Dockerfile` `web` target → `brdgme/web`. k8s
      manifests in `k8s/base/web/` unchanged.
- [x] Add `brdgme/web-legacy` image build to the Tiltfile (from
      `web/Dockerfile`, final stage `web`, tagged `brdgme/web-legacy`).
- [x] Add `brdgme/api` and `brdgme/websocket` image builds to the Tiltfile.
- [x] Create `k8s/base/web-legacy/` manifests (Deployment + Service) using
      `image: brdgme/web-legacy`. Mirror the structure of `k8s/base/web/` but
      with `name: web-legacy`.
- [x] Create `k8s/base/legacy/kustomization.yaml` grouping `web-legacy`, `api`,
      and `websocket` as the legacy stack.
- [x] Restore `api` and `websocket` manifests to an active kustomization overlay
      alongside the legacy frontend. (`k8s/base/brdgme` now includes `../legacy`)
- [x] Configure Knative domain to `brdg.me` (patch `config-domain` in
      `knative-serving`). (`k8s/prod/knative-serving/config-domain.yaml`)
- [x] Create Knative `DomainMapping` resources (one per service) to assign
      custom hostnames. All services are already Knative Services, so Kourier
      routes by hostname automatically:
      - `brdg.me` → `web`
      - `legacy.brdg.me` → `web-legacy`
      - `api.brdg.me` → `api`
      - `ws.brdg.me` → `websocket`
      (`k8s/base/domain-mapping/`, included in `k8s/prod/app/`)
- [x] Remove `k8s/base/ingress/` (nginx Ingress) from `k8s/base/brdgme` -
      Kourier is the sole external entry point via DomainMappings.
- [x] TLS: cert-manager with per-DomainMapping certificates via
      `networking.knative.dev/certificate-class: cert-manager.io` annotation.
      `k8s/base/cert-manager/cluster-issuer.yaml`: Let's Encrypt `ClusterIssuer`
      using HTTP01 solver with `kourier.ingress.networking.knative.dev` ingress
      class. `k8s/prod/knative-serving/`: `config-certmanager.yaml` (issuer ref)
      and `config-network.yaml` (auto-tls: enabled, http-protocol: redirected).
      Prerequisites (one-time, not in kustomize - cluster infrastructure):
        kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.17.2/cert-manager.yaml
        kubectl apply -f https://github.com/knative/net-certmanager/releases/download/knative-v1.21.0/release.yaml
- [x] Verify the old React frontend's API base URL is configured to point to
      the `api` service - confirmed: `http.ts` derives URL by replacing first
      subdomain with `api` (`legacy.brdg.me` → `api.brdg.me`).

### Validation criteria (gate for decommission)

Note this phase is cutover-first: `brdg.me` points at the new system
immediately; the legacy stack on `legacy.brdg.me` is the fallback. "Proven in
production" means all of the following, over a validation window of at least
4 weeks:

- [ ] Every user-facing flow exercised on the new system in production: login
      (email + code), game creation (human opponents and bot slots), command
      submission with autocomplete, undo, concede, restart, mark-read, game
      logs, sidebar active games, live WebSocket updates.
- [ ] At least one game of each deployed game type (Rust and Go) played to
      completion via the new UI.
- [ ] Ratings update correctly on game finish and concede (requires the ELO
      pre-cutover task).
- [ ] Cross-system WS updates verified: a move made in the new UI appears live
      in a legacy React client on the same game, and vice versa.
- [ ] No unexplained monolith 5xx responses or WASM client panics in the
      window (restart 500 bug must be fixed or explained first).
- [ ] Bots complete turns reliably in production (no stuck bot turns needing
      manual bumps).

### Rollback procedure

Both systems share the database, so rollback is routing-only; no data
migration in either direction. Sessions are separate (cookie vs Bearer token)
so users re-login after a swap.

- [ ] Verify before relying on it: the legacy React frontend derives its API
      URL by replacing the first subdomain with `api` (`web/src/.../http.ts`).
      Confirm it produces `api.brdg.me` when served from the apex `brdg.me`,
      not only from `legacy.brdg.me`. If it does not, rollback requires a
      frontend config change - test this while legacy is still deployed.
- To roll back: edit the `brdg.me` `HTTPRoute` (`k8s/base/gateway/`, after
  Phase 14) to point its `backendRef` at the `web-legacy` Service instead of
  `web`, apply, and verify. The TLS certificate is bound to the Gateway
  listener, not the backend, so no re-issue is needed. Keep the
  `legacy.brdg.me` route intact.
- Games created or finished via the new system remain valid for legacy (same
  schema); no cleanup needed.

### Decommission (once validation criteria above are met)

Remove the legacy stack in this order:

- [ ] Remove `api`, `websocket`, and `web-legacy` from the kustomization and
      delete their k8s manifests.
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds from the Tiltfile.
- [ ] Delete stale root build artifacts (added 2026-07-03 final pass):
      `WORKSPACE` (Bazel era), `build.sh`/`test.sh` (docker builds of
      legacy targets), `docker-compose.yml` (pre-Kind dev environment).
      Verify nothing references them (CI, docs) before deleting.

Redis remains after this step - it is still used by `rust/web`. Removal
happens in Phase 17.

**Notes (Build & Dev Environment):**
- Switched to `cargo-binstall` in Dockerfile to avoid `serde` compilation
  errors when installing `cargo-leptos`.
- Fixed `dart-sass` path handling in Dockerfile.
- Isolated `cargo chef cook` for the `web` crate to prevent non-WASM
  dependencies (`mio`, `socket2`) from breaking the WASM build graph.
- Implemented `SQLX_OFFLINE=true` support via `.sqlx` metadata. Added
  Skaffold port-forwarding to allow local builds to verify queries against
  the K8s Postgres instance.
- Refactored `skaffold.yaml`: default profile deploys only backing services
  (Postgres, Redis, game services), skipping the slow `web` build. Use
  `skaffold dev -p with-web` for a full cluster test.

**2025-12-22: Fixed database connection pool in server functions**
- Server functions were failing with "Database pool not found" errors.
- Root cause: `leptos_axum::extract()` with `State<AppState>` had no state
  context in the server function scope.
- Fix: switched to Leptos context-based dependency injection.
  - `leptos_routes_with_context()` instead of `leptos_routes()`.
  - `PgPool` and `GameBroadcaster` provided via `provide_context()`.
  - Server functions use `use_context::<PgPool>()` instead of Axum state
    extraction.
  - `use_context()` with error handling instead of `expect_context()`.

---

## Phase 17: NATS Migration + WS simplification [Pending]

**Goal:** Replace Redis pub/sub with NATS Core, remove Redis, and simplify
the WebSocket broadcast path now that legacy compat is gone.

**Delegation gap:** the target state is clear but the implementation is only
sketched. Before delegating, specify:
- **Server subscription architecture:** per-WebSocket-connection NATS
  subscription vs one shared subscription per replica with in-process fan-out
  to connections (filtering by game id). Decide, with connection-count and
  resource reasoning.
- **Client-side refactor plan:** several components currently depend on the
  fat payload (`GamePage`'s WS-takes-precedence view logic, `GameLogs`/
  `RecentGameLogs` WS log preference, the `RwSignal<Option<BrdgmeGameUpdate>>`
  context). A component-by-component change list is needed: what replaces the
  context signal, how re-fetches are triggered per component, and coalescing
  behaviour when several skinny signals arrive in quick succession.
- **Test updates:** which Phase 11 tests change (broadcaster swaps from Redis
  to NATS in CI) and what new assertions cover the skinny-signal path.
- **Sequencing:** explicitly after Phase 16 decommission (legacy channels
  must be dead) - state this as a hard precondition.

### WS payload strategy change (fat → skinny)

During Phases 8-15, fat payloads are justified because legacy compat already
requires per-player `get_game_logs` DB queries and auth token lookups - the
`BrdgmeUpdate` comes for free. Post-decommission that cost exists solely to
serve the fat payload. Logs also grow unboundedly with game length.

The per-player complexity (different board HTML, `command_spec`, private logs
per player) is the root cause of most of `broadcast_game_update`'s weight.
Skinny payloads eliminate the need for player-specific messages entirely -
player-specific data comes back through the authenticated `get_game_details`
re-fetch, which is the right place for it anyway.

In this phase, `broadcast_game_update` reduces to a single publish:
- Publish `{"game_id": "..."}` once to `game.{id}` (no per-player loop)
- Client re-fetches `get_game_details` + `get_game_logs` on receipt
- Remove: all `Legacy*` structs, per-player loop, auth token lookup,
  per-player `get_game_logs` calls, session extraction in `ws_handler`,
  `ws.{user_id}` channel, `BrdgmeGameUpdate`/`WebSocketMessage` enum
- `/ws` handler reverts to simple `PSUBSCRIBE game.*`, no session needed

### Infrastructure

**Note:** NATS cluster installation (k8s manifests, Tiltfile, `async-nats`
dependency) is pulled forward into Phase 13 NATS bot eventing. By the time this
phase runs, NATS is already in the cluster. The remaining infrastructure tasks
here are the WS-specific migration:

- [ ] Replace Redis `PUBLISH`/`SUBSCRIBE` in `websocket.rs` with `async-nats`
      publish/subscribe. Subject naming: `game.{id}` and `ws.{user_id}`.
- [ ] Simplify `broadcast_game_update` to skinny signal (see above).
- [ ] Replace the hand-rolled client in `websocket_client.rs` (gloo-net +
      manual 2s reconnect loop) with `leptos-use`'s `use_websocket`
      (built-in `ReconnectLimit` reconnection, typed codecs; added
      2026-07-03 final pass). The skinny-payload model - re-fetch on any
      message - is exactly its shape; deletes ~60 lines of bespoke
      connection management. Do this here, not earlier: the current fat
      `BrdgmeUpdate` handling would fight its codec model.
- [ ] Remove the `redis` dependency from `rust/web/Cargo.toml`.
- [ ] Remove Redis from `k8s/base/brdgme/kustomization.yaml` and delete
      `k8s/base/redis/`.
- [ ] Remove Redis port-forward from the Tiltfile.

**Note:** JetStream is already enabled from Phase 13 (bot eventing). WS
fan-out deliberately stays on plain Core pub/sub subjects - ephemeral
at-most-once is correct here (clients re-fetch full state on reconnect); do
not route WS traffic through the `BOT` stream.

---

## Phase 18: Production Hardening [Pending]

**Goal:** Ensure errors are visible and diagnosable in production, where
optimised WASM strips debug info and panics are otherwise silent.

**Delegation gap:** every open item here needs a decision or spec first:
- **`ErrorBoundary`:** which components get boundaries (list them), what each
  fallback renders, and whether recovery (retry/reload) is offered.
- **WASM source maps:** currently an investigation ("check the option") - do
  the research, then write the resulting config task.
- **Log aggregation:** decided 2026-07-03 - VictoriaLogs (see the task
  below). Remaining spec work: exact manifests, retention figure, and which
  structured fields become stream fields.
- **Alerting:** thresholds, alert destinations (email? something else?), and
  what tool evaluates the rules.

### WASM client

- [x] **`console_error_panic_hook`**: already installed in `lib.rs::hydrate()`.
      Panics write the message and location to the browser console before
      aborting, even in release builds.

- [ ] **`ErrorBoundary`**: wrap key page sections (`GamePage`, `DashboardPage`)
      in Leptos `<ErrorBoundary>` components so a component error renders a
      fallback instead of silently breaking the UI. Without this, a panic or
      unhandled error in the game view leaves the user with a blank or frozen
      component and no indication of what happened.

- [ ] **WASM source maps**: configure `cargo-leptos` to emit source maps in
      release builds. This makes browser console stack traces show Rust source
      locations rather than raw WASM offsets. Check `Cargo.toml`
      `[package.metadata.leptos]` for the `source-map` option when evaluating.

### Server (SSR / Axum)

- [ ] **Structured log aggregation (VictoriaLogs - decided 2026-07-03)**:
      single-node VictoriaLogs Deployment + PVC (~10Gi, ~30d retention) and
      a Vector DaemonSet shipping container stdout with Kubernetes metadata.
      The JSON structured fields already emitted (trace_id, game_id, etc.)
      map directly onto VictoriaLogs fields; query via its built-in web UI
      (Grafana datasource optional later). Chosen over Loki (roughly 5-10x
      heavier at rest in published benchmarks - does not fit 2GB nodes) and
      Datadog (not open source). This is infrastructure config, not code.

- [ ] **Error rate alerting**: alert on elevated `tracing::error!` rate or
      HTTP 5xx rate from the monolith, via vmalert evaluating LogsQL queries
      against VictoriaLogs. Alert destination still undecided.

### Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.

---

## Phase 19: CloudNativePG [Pending]

**Decision (2026-07-03 tech review):** replace the hand-rolled Postgres
StatefulSet (`k8s/base/postgres/`) with CloudNativePG (CNCF operator, the de
facto standard for Postgres on Kubernetes - adopted as the recommended
pattern by major clouds). Gains: declarative provisioning, scheduled backups
+ WAL archiving + PITR to DO Spaces (S3-compatible) via the Barman Cloud
plugin, a config-change path to replicas + automated failover, and identical
database infrastructure in dev and prod (same operator, same `Cluster` CR in
Kind and DOKS).

**Sequencing:** after Phase 14 (manifests rewritten once, against final
infrastructure), before Phase 15 (so ArgoCD manages the final shape from day
one) and Phase 16 (cutover happens onto CNPG, not migrated again after).
Dev-side work is delegable; the prod data import is human-operated.

### Dev (Kind)

- [ ] Install the CNPG operator in `setup-kind-cluster.sh` (same manifest
      install as prod).
- [ ] Replace `k8s/base/postgres/` with a `Cluster` CR: `instances: 1`,
      `imageName: ghcr.io/cloudnative-pg/postgresql:17`, small storage
      request, `bootstrap.initdb` creating the `brdgme` database and owner
      role with the current `postgres-config` credentials (via
      `bootstrap.initdb.secret`) so app config is unchanged.
- [ ] Update service references: CNPG exposes `<cluster>-rw`/`<cluster>-ro`
      Services - `DATABASE_URL`/`postgres-config` host changes accordingly.
- [ ] Tiltfile: update the Postgres port-forward target and the mirrord
      target (`pod/postgres-0` → the CNPG instance pod, e.g.
      `pod/<cluster>-1`).
- [ ] Dev data is disposable: recreate the cluster, run migrations, no
      import needed.

### Prod (DOKS)

- [ ] Install the CNPG operator via kustomize (ArgoCD-managed once Phase 15
      lands).
- [ ] `Cluster` CR: `instances: 1` initially (matches today's posture;
      `instances: 2` + automated failover is a later config change), storage
      on DO block storage.
- [ ] Backups: Barman Cloud plugin → DO Spaces bucket (endpoint
      `https://syd1.digitaloceanspaces.com`), daily scheduled base backup +
      continuous WAL archiving. Verify a PITR restore into a scratch
      `Cluster` before relying on it.
- [ ] Data import: `bootstrap.initdb.import` (CNPG logical import) from the
      existing StatefulSet instance during a maintenance window - no shared
      PVC. Verify row counts and app login, then retire the StatefulSet.
- [ ] Delete the old `k8s/base/postgres/` StatefulSet manifests.

---

## Phase 20: external-dns [Pending]

**Decision (2026-07-03 tech review):** manage DO DNS records from the
cluster with external-dns (DigitalOcean provider, `gateway-httproute`
source). Records for `brdg.me`/`legacy.`/`api.`/`ws.` follow the `HTTPRoute`
hostnames created in Phase 14, so the Phase 16 cutover and its rollback
become pure git operations - no manual edits in the DO control panel.

**Sequencing:** after Phase 14 (needs the Gateway + HTTPRoutes), ideally
after Phase 15 (the DO API token lands as a SealedSecret), and before the
Phase 16 cutover to deliver its value. Manifests are delegable; adoption of
the live DNS records is human-operated.

- [ ] `k8s/prod/external-dns/`: Deployment + RBAC. Args:
      `--provider=digitalocean`, `--source=gateway-httproute`,
      `--domain-filter=brdg.me`, `--registry=txt`,
      `--txt-owner-id=brdgme-prod`, `--policy=upsert-only` initially.
- [ ] DO API token (DNS-scoped) as a SealedSecret.
- [ ] Adopt existing manually created records deliberately: external-dns
      only manages records it owns via TXT registry entries. Audit the live
      zone for conflicts and take ownership record-by-record rather than
      letting the first sync surprise.
- [ ] Flip `--policy=upsert-only` → `sync` once ownership is verified, so
      deleted HTTPRoutes clean up their records (needed when Phase 16
      decommission removes `legacy.`/`api.`/`ws.`).
- [ ] Not used in dev (Kind uses lvh.me; nothing to reconcile).

---

## Phase 21: OpenTofu Infrastructure as Code [Pending - human-paced]

**Decision (2026-07-03 tech review):** describe the DigitalOcean account
infrastructure in OpenTofu (Linux Foundation Terraform fork; open source,
matching project principles). Scope is only what Kubernetes cannot
self-describe: the DOKS cluster, the VPC, the `brdg.me` DNS zone (the zone
belongs to tofu, records to external-dns), the Spaces bucket for CNPG
backups, and the Spaces bucket for tofu state. The Gateway-provisioned load
balancer is NOT managed here - DOKS owns it.

**Sequencing:** independent of all other phases and entirely human-operated
(account credentials). Highest value before Phase 14's prod prerequisites -
"cluster >= 1.33, VPC-native" becomes a fact encoded in code instead of a
checklist item - but blocks nothing.

- [ ] Add `opentofu` to `devenv.nix`.
- [ ] `infra/` directory: DO provider, S3 backend against a Spaces bucket.
- [ ] `tofu import` the existing resources (cluster, VPC, domain) - do not
      recreate. `tofu plan` must show no changes after import before
      anything else is done.
- [ ] Encode the Phase 14 prerequisite: cluster version >= 1.33 with
      VPC-native networking.
- [ ] Create new resources (CNPG backup bucket for Phase 19, state bucket)
      via tofu from the start.

---

## Phase 22: Email via Resend [Pending - high priority]

**Decision (2026-07-03):** all platform email moves to Resend (resend.com).
Outbound replaces the self-managed SMTP relay path (spam-filter /
deliverability pain); inbound (play-by-email replies) uses Resend receiving,
which POSTs parsed email to a webhook. Chosen for: inbound webhooks on every
tier including free (3,000 emails/mo combined sent+received, 100/day cap),
an official Rust SDK (`resend-rs`, built on the `svix` crate that also
verifies its webhooks), and the strongest hobby/OSS adoption of the current
providers. Rejected: Postmark (inbound locked to Pro tier), Mailgun
(1 inbound route on Basic; full routing $35/mo), SES (cheapest at scale but
S3/SNS plumbing + AWS account overhead - revisit only if sustained volume
exceeds ~10k/mo, where SES is ~$1 vs Resend's $20).

**Split:** 22a (outbound swap - small, independent of the k8s phases, run
early) and 22b (turn notifications + inbound replies - a new feature, after
cutover). Free-tier watch item: the 100/day combined cap is the binding
constraint; the escape hatch is Pro at $20/mo.

### 22a: Outbound via Resend API [high priority - run before Phase 16]

**Revised 2026-07-03 (same day):** send via the Resend HTTP API with
`resend-rs`, not SMTP. DigitalOcean blocks outbound SMTP ports (25/465/587)
by default and unblocking is a discretionary support request; the API over
443 sidesteps the whole problem class. This drops `lettre` entirely and
deletes the in-cluster smtp relay, and the same `resend-rs`/`svix` stack
verifies the 22b inbound webhooks - a net dependency reduction.
(Supersedes the Mailpit quick win and the earlier SMTP-transport version of
this section.)

**Dev story:** no email infrastructure in dev at all. `RESEND_API_KEY`
unset → the existing log fallback prints the email (already how login codes
are read in dev). For work on real email content (22b templates), set a
Resend test-mode API key in `.env` - the same pattern the bot uses for
`LLM_API_KEY`.

- [ ] Create the Resend account; verify `brdg.me` as the sending domain:
      add the SPF, DKIM, and DMARC DNS records. Zone-level records belong
      to OpenTofu (Phase 21) once it exists; if 22a runs first, add them
      manually and note them for import. *(human/infra - not done here)*
- [x] Replace `lettre` with `resend-rs` in `rust/web`:
      `send_login_email` sends via the Resend client (`resend_rs::Resend`,
      held in `AppState` alongside the existing shared `reqwest::Client`).
      Env: `RESEND_API_KEY` (unset = log fallback, replacing the
      `SMTP_HOST`-unset fallback) and `EMAIL_FROM` (default
      `login@brdg.me`). Removed `SMTP_HOST`/`SMTP_PORT`/`SMTP_FROM`
      handling, the `lettre` dependency, and updated `.env.template`.
- [ ] Prod config (SealedSecret once Phase 15 lands; plain Secret before):
      `RESEND_API_KEY`, `EMAIL_FROM`. *(human/infra - not done here)*
- [x] Delete `k8s/base/smtp/` from the new-system overlays and the
      Tiltfile. Checked whether the legacy stack (`rust/api`) sends
      through the in-cluster `smtp` Service: it does not (`Mail::Relay`/
      `Mail::Smtp` in `rust/api/src/config.rs` is constructed but never
      read - the one call site in `controller/auth.rs` is commented out),
      so `k8s/base/smtp/` was deleted outright rather than moved to a
      legacy overlay.
- [ ] Verify: a prod login email lands in a real Gmail inbox - not spam -
      with SPF, DKIM, and DMARC all passing (inspect the received headers).
      *(human/infra - not done here)*
- [x] Rate-limit the login endpoint (the only email-sending route today)
      with `tower_governor` per client IP (added 2026-07-03 final pass):
      without it, anyone hammering the login form drains the 100/day Resend
      quota and locks every player out of logging in. Implementation note:
      `Login` is a Leptos server function auto-mounted by `leptos_axum`
      alongside every other server fn/page route in one opaque `Router`
      build step, so a `GovernorLayer` can't be scoped to just that route
      without either rate-limiting the whole app or all of `/api`. Instead
      `auth/rate_limit.rs` builds the same `governor` rate limiter
      `tower_governor` uses internally and checks it directly inside the
      `login()` handler body, keyed by `SmartIpKeyExtractor` (falls back
      through `X-Forwarded-For`/`X-Real-Ip`/`Forwarded` to the TCP peer
      address). Burst 5, replenishes 1 every 20s, per IP. Caveat carried
      forward unresolved: verify real client IPs survive the DO LB +
      Cilium Gateway path (externalTrafficPolicy/PROXY protocol - a known
      DOKS consideration; fold into Phase 14's LB prerequisite check). If
      source IPs are not preserved, the limiter keys on the LB address and
      throttles everyone collectively - configure the LB first.

Delivers the founding VISION principle: a full game playable from an email
client. Sequencing: after the Phase 16 cutover (additive feature; fine to
build during the validation window). Depends on 22a.

**Design:**

- **Inbound domain:** dedicated subdomain `play.brdg.me` with MX records
  pointing at Resend receiving - keeps `brdg.me`'s own MX untouched.
- **Reply addressing / sender auth:** each `game_players` row gets a random
  unique `email_token` (migration). Notification emails set
  `Reply-To: g-{email_token}@play.brdg.me`. On receipt the token maps to
  (game, player), and the From address must also match that player's user
  email - the token authorises, the From check is defence in depth. From
  alone is never trusted (trivially spoofable).
- **Turn notification email:** sent when a player's `is_turn` transitions
  to true - same call sites as `trigger_bot_turns` (execute_command;
  create/undo/concede/restart handlers). Content: game type + opponents,
  the player's render converted markup → plain text, command examples from
  the command spec, and reply instructions. Bot players skipped.
- **Webhook endpoint:** `POST /api/webhooks/resend` on the monolith.
  Resend webhooks are svix-signed: verify with the `svix` crate (the same
  library `resend-rs` builds on) against the raw request body; secret via
  env/SealedSecret; 401 on failure. Svix verification also rejects
  timestamps more than 5 minutes old (replay protection).
- **Reply parsing:** take the text/plain part; drop quoted lines (`>`
  prefix), everything from the first `On ... wrote:` line onward, and
  anything after a `-- ` signature marker. Remaining non-empty lines are
  commands, executed in order via `execute_command`; stop at the first
  error. If Resend delivers raw MIME rather than parsed JSON, use the
  `mail-parser` crate for MIME handling rather than hand-rolling it;
  quote-stripping stays bespoke either way (a few lines, no well-maintained
  crate covers it).
- **Response email:** every inbound reply gets one: the updated render on
  success (move confirmed), or the validation error + current render on
  failure. This closes the loop - inbox-only play.
- **Idempotency:** the provider retries webhooks on non-2xx. Store
  processed webhook event ids (small table, unique constraint, periodic
  cleanup); a duplicate id returns 200 without re-executing.

**Tasks:**

- [ ] Confirm the live Resend inbound payload schema and signature scheme
      against their docs/account before delegating the endpoint work (the
      shapes above are from 2026-07 documentation, not verified in anger).
- [ ] Migration: `game_players.email_token` + `processed_webhook_events`.
- [ ] Resend receiving config for `play.brdg.me` (MX records via
      tofu/manual per the 22a note; webhook URL + secret).
- [ ] Markup → plain-text render path: check what `brdgme_markup` already
      provides; the bot's `markup_resolve_players` is reusable.
- [ ] `notify_turn_emails` alongside `trigger_bot_turns`. Check whether the
      legacy `users` table already has an email-notification pref column
      before adding an opt-out flag.
- [ ] Webhook endpoint: signature verification + parser + executor +
      response email.
- [ ] Tests (Phase 11 patterns): parser unit tests across quoting styles
      (Gmail, Outlook, plain `>`); endpoint integration tests with JSON
      fixtures and a fixed webhook secret (`#[sqlx::test]` + mock game
      service); sender-auth rejection cases (bad token, From mismatch,
      bad signature); duplicate-event idempotency.
- [ ] Quota guard: count outbound sends; alert via Phase 18 vmalert as
      volume approaches 100/day or 3k/mo.

---

## Development Workflow

Requires a Kind cluster with Knative (Kourier ingress). Run once per workstation:

```
bash scripts/setup-kind-cluster.sh
```

### Hybrid (Fast Iteration) - default

```
tilt up
```

Deploys backing services (Postgres, Redis, SMTP, game microservices, operator)
to Kind. Runs `cargo leptos watch` locally via mirrord so the web server can
resolve cluster-internal DNS (`*.brdgme.svc.cluster.local`) without any
application changes. Hot-reloading at `http://localhost:3000`.

Note: in this mode there is no `web` Knative Service in the cluster, so
`web.brdgme.lvh.me:8080` is not available. Use `localhost:3000` instead.

### Full Cluster Test

```
WEB_IN_CLUSTER=1 tilt up
```

Builds and deploys `brdgme/web` as a Knative Service into Kind alongside all
backing services.
