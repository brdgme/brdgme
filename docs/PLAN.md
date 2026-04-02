# Monolith Migration Plan

## Objective

Consolidate the `brdgme` platform into a single Rust-based monolithic
application using Axum (backend) and Leptos (frontend/WASM). This replaces the
Rocket API, Node.js WebSocket service, and TypeScript/React frontend.

## Strategy

Build the new system in `rust/web` in parallel with the existing services. The
old services (`rust/api`, `web`, `websocket`) remain untouched until cutover.

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

## Phase 5: Frontend (Leptos UI) [Defective]

**Goal:** Build the UI in Rust, replacing React.

- [x] Build app shell and layout components.
- [x] Implement shared types and server functions.
- [x] Build `GameBoard` (ASCII-to-HTML), `GameMeta`, `GameLogs`,
      `GameCommandInput` components.
- [x] Implement client-side command parsing using `brdgme_game` compiled to
      WASM, providing real-time suggestions and validation.
- [x] Implement WebSocket client hook (`websocket_client.rs`) that triggers
      resource refetches via a global `WebSocketTrigger` context.

**Known defects (tracked in Phase 5.6):**
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

## Phase 5.5: Dev Environment Migration [Complete]

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

## Phase 5.6: Pre-Cutover Fixes [In Progress - blockers done, frontend gaps remain]

**Goal:** Resolve all blockers and close critical gaps found in the parity
review before the `leptos` branch replaces production. Full review in
`docs/REVIEW.md`.

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
- [x] **Email sending** (`auth/server.rs`): `send_login_email` implemented
      using `lettre 0.11` with `AsyncSmtpTransport` (plain SMTP, no TLS).
      Reads `SMTP_HOST`, `SMTP_PORT` (default 25), `SMTP_FROM` from env.
      Logs warning with token if `SMTP_HOST` unset (dev fallback).

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

---

## Bug fixes [Partially resolved]

- [ ] **Restart 500 error**: `restart_game` returns "Game service error: error
      parsing JSON response". Diagnostics improved: `client::request` now reads
      response body as text first and includes it in the error message. Root
      cause still unknown - needs a live restart attempt to capture the raw
      game service response.
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

## Phase 6: Redis pub/sub + web-legacy WS compatibility [Complete]

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
either system during Phase 7 side-by-side operation.

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

## Phase 9: LLM Bots [In Progress]

**Goal:** Add bot players backed by an external LLM via the Ollama HTTP API.
Bots receive the rendered game state and available commands, produce a command
string, and submit it via the monolith. Ollama runs outside the cluster - on
the dev host in development, and via Ollama Cloud (or a home GPU over
Tailscale) in production.

### Design decisions

- **Model**: `qwen3.5:4b` with thinking disabled (`think: false` option). Small
  enough for fast inference; qwen3.5 instruction-following quality is strong at
  this size. Configurable via `BOT_MODEL` env var. Default is `qwen3.5:4b`
  (matches Ollama Cloud prod). Override locally to e.g. `qwen3.5:9b` on an
  RTX 5080 for better quality.
- **Thinking disabled**: qwen3 models support an extended thinking mode that
  produces verbose chain-of-thought output. Disabling it (`think: false` in
  the Ollama `/api/chat` request) gives faster, more concise responses
  which is what we want for a single command string.
- **Inference server**: External Ollama instance. Interact purely via Ollama's
  HTTP API (`/api/chat`). No in-cluster Ollama deployment.
- **Ollama Cloud (prod)**: Uses Ollama Cloud as the inference backend. Requires
  a Bearer token (`OLLAMA_API_KEY` env var). The free tier quotas (5 hours / 7
  days) are sufficient for async turn-based play with a small model and thinking
  disabled. Home GPU over Tailscale is a viable fallback (see notes).
- **Dev**: Ollama runs as a service on the dev host. Bot caller points to
  `http://localhost:11434` by default (hybrid mode) or to the host gateway IP
  when running fully in-cluster. No cluster-side Ollama deployment needed.
- **Bot caller**: separate Knative Service (`rust/bot` crate). Receives a
  trigger from the monolith, assembles the prompt, calls Ollama, validates the
  response, and submits the command back to the monolith via an internal API
  key. Can scale to zero - it is only active during bot turns.
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
  with the bot. Resolution of stuck bot turns (broader randomisation, concede,
  etc.) is deferred to a later iteration.
- **Bot triggering**: v1 uses direct HTTP (monolith POSTs trigger to bot
  caller). Future: event-driven via NATS after Phase 8 - monolith publishes
  "bot turn" event, bot service subscribes, publishes "bot command" event back.
  This fits naturally once NATS is in place.
- **Rendered state**: the bot receives `player_renders[n].render` (the ASCII
  render from the game service `Status` response) - the same output shown to
  human players. No raw state structure documentation needed.

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
  - Pre-Ollama: check `is_turn` from initial DB fetch; bail early if false.
  - Post-Ollama: re-query `is_turn` and `game_state` after each Ollama response.
    If not their turn: bail. If game state changed (e.g. undo): refresh render,
    logs, command spec via `load_bot_context` helper; reset conversation and
    retry Ollama with fresh context. Attempt counter continues across refreshes.

### Bot caller service (`rust/bot`)

New Rust binary crate. Deployed as a Knative Service. Receives trigger POSTs
from the monolith, assembles the full prompt, calls Ollama, retries on failure,
then submits the command to the monolith's internal endpoint.

- [x] `rust/bot/` crate: minimal Axum server on port 4000, `POST /trigger`.
- [x] Request body: `{"game_id": "uuid", "player_position": 0, "difficulty": "medium"}`.
- [x] On trigger: fetch game + player names + logs from DB; call game service
      `Status` for render + command spec; call `Rules`; assemble prompt; POST to
      Ollama `/api/chat`; retry up to 20 times on validation error; on hard
      failure log error and leave turn with bot.
- [x] `load_bot_context` helper: fetches render, command spec, logs. Called at
      start and on mid-loop state refresh.
- [x] Env vars: `DATABASE_URL`, `BOT_MODEL`, `OLLAMA_URL`, `OLLAMA_API_KEY`,
      `MONOLITH_URL`, `INTERNAL_API_KEY`. Dynamic sqlx queries (no .sqlx cache).
- [x] `rust/bot` Dockerfile target added to `rust/Dockerfile`.
- [x] `rust/bot` added to workspace `Cargo.toml`.

### Acquire rules [Complete]

- `rust/game/acquire-1/RULES.md` written: board layout, corporations, pricing,
  mergers, bonuses, rendering guide, command reference.
- `fn rules()` updated to `include_str!("../RULES.md").to_string()`.
- 2-player special rule corrected: dummy share count uses a D6 roll (1-6), not
  a drawn tile's column number (1-12). Rules now match the game implementation.

### Prompt structure [Complete]

`rust/bot/system_prompt.md` is a single MiniJinja template rendered at the
start of each Ollama attempt. It covers the full prompt in one document:

1. **Persona**: expert board gamer, maximise fun and play to win.
2. **Task**: respond with exactly one plain-text command, no other text.
3. **Skill rating**: `{{ difficulty }}` injected; all three levels (easy/medium/
   hard) described so the model understands the full scale.
4. **Command parser rules**: documentation for all 10 `Spec` variants (`Token`,
   `OneOf`, `Chain`, `Doc`, `Space`, `Opt`, `Enum`, `Int`, `Player`, `Many`)
   with YAML examples and plain-text command examples. Real Acquire `Buy`-phase
   spec included as a worked example.
5. **Game rules**: `{% if game_rules %}{{ game_rules }}{% endif %}` — omitted
   when the game has no rules text.
6. **Players**: `{% for player in players %}` loop with name, score, colour.
   Bot's own player marked `(you)`.
7. **Game render**: `{{ game_render }}` inside a ` ```html ` fence — markup
   converted to HTML via `brdgme_markup::html` with player references resolved.
8. **Recent logs**: `{% for log in recent_logs %}` — each log body converted
   from brdgme markup to HTML.
9. **Command spec**: `{{ command_spec }}` inside a ` ```yaml ` fence —
   serialised via `serde_json::to_value` → `serde_yaml::to_string` to produce
   the mapping style (`Token: done`) rather than native YAML tags (`!Token`).
10. **Failed commands**: `{% if failed_commands %}` block listing each prior
    rejected command and its error. Omitted on the first attempt.

`rust/bot/src/prompt.rs` provides `markup_to_html`, `spec_to_yaml`,
`render_prompt`, and 15 unit tests covering all conditional sections, HTML
render quality, log rendering, player loop, and YAML spec format.

`BotContext` now carries `render_html`, `command_spec_yaml`, `recent_logs_html`,
and `points` (scores). The retry loop accumulates `FailedCommand` entries and
re-renders the full template on each attempt rather than building a multi-turn
conversation.

**KV cache restructure (future optimisation)**

The current design puts all content — including dynamic game state — into a
single system message re-rendered each turn. This is correct and simple, but
foregoes Ollama prefix caching. A future restructure would split into multiple
messages with static content first (persona + parser docs → system; rules →
per-game-type user message) so Ollama can cache the long static prefix across
turns. Deferred until the bot is validated working well end-to-end.

### Bot k8s manifests

- [x] `k8s/base/bot/service.yaml`: Knative Service for `brdgme/bot` image.
      Reads `postgres-config` + `bot-config` secrets.
- [x] `k8s/base/bot/` added to `k8s/base/brdgme/kustomization.yaml`.
- [x] Tiltfile: bot `local_resource` in hybrid mode with `MONOLITH_URL`,
      `INTERNAL_API_KEY`, `OLLAMA_URL`; web gets `BOT_SERVICE_URL` and
      `INTERNAL_API_KEY`. Full-cluster mode builds `brdgme/bot` and creates
      `bot-config` secret.

### Ollama configuration

Ollama runs outside the cluster. No in-cluster Ollama deployment.

**Dev (hybrid mode - default `tilt up`):**

`rust/bot` runs as a local process. Ollama is already running as a system
service on the dev host. Set `OLLAMA_URL=http://localhost:11434` in the local
environment (or accept the default). Pull the model once:

```sh
ollama pull qwen3:4b
```

No Kind cluster changes needed. `BOT_SERVICE_URL` must point to the local bot
process; if unset, bot triggering is disabled and only the model can be tested
manually.

**Prod:**

Uses Ollama Cloud. Set `OLLAMA_URL` to the Ollama Cloud API endpoint and
`OLLAMA_API_KEY` to the API key (Kubernetes Secret). Model is specified via
`BOT_MODEL` env var - Ollama Cloud hosts the model remotely, no local pull
needed.

**Home GPU over Tailscale (prod alternative):**

Ollama running on an RTX 5080/3080 or RX 9060 XT at home, exposed via
Tailscale. Set `OLLAMA_URL` to the Tailscale IP/hostname of the home machine.
No `OLLAMA_API_KEY` needed (Tailscale handles network auth). Tradeoffs: depends
on home machine being on and Tailscale connection being stable. Suitable as a
fallback or for cost reduction once validated.

### Bot service logging improvements [Complete]

Structured logging at key points in `run_bot_turn`. Each log entry should
include a `trace_id` (a `Uuid::new_v4()` generated at the start of each
trigger request) so all log lines for a single bot turn can be correlated
even when multiple bots are running concurrently.

**On trigger received:**
- trace_id, game_id, game name (e.g. "Acquire"), player position, player name,
  difficulty.

**On each Ollama response:**
- trace_id, attempt number, full raw response text from Ollama (so invalid
  commands and hallucinations can be inspected).

**On command submitted to monolith:**
- trace_id, attempt number, command string, HTTP status code.
- If rejected: validation error body returned by the monolith.
- If accepted: log success.

**On retry due to validation failure:**
- trace_id, attempt number, reason (validation error summary).

**On game state change detected mid-loop:**
- trace_id, attempt number, note that game state changed while Ollama was
  thinking and context is being refreshed.

**On hard failure (all attempts exhausted):**
- trace_id, final error, number of attempts made.

Implementation notes:
- Use `tracing::info!` for normal flow, `tracing::warn!` for retries and state
  refreshes, `tracing::error!` for hard failures.
- All fields as structured key-value pairs (not interpolated into the message
  string) so they are machine-parseable in production log aggregation.
- The trace_id should be passed through `run_bot_turn` and all helpers rather
  than stored in a global.

### ELO ratings (future)

`update_game_command_success` does not yet update `game_type_users.rating`,
`game_type_users.peak_rating`, or `game_players.rating_change`. When ELO is
implemented the rule must be: **only update ratings for human players, and only
with respect to other human players in the same game**. Any game that includes
at least one bot player must not affect any human rating.

### Optimistic locking (pending)

`execute_command` reads game state, makes an external HTTP call to the game
service, then writes back. A concurrent undo or second bot trigger could modify
the row between read and write. Fix using optimistic locking on `games.updated_at`
(microsecond-precision timestamp, set by trigger on every UPDATE):

1. Read `game.updated_at` in `execute_command` alongside `game_state`.
2. Pass `expected_updated_at` to `update_game_command_success`.
3. Change the UPDATE to:
   `UPDATE games SET ... WHERE id = $1 AND updated_at = $expected`
4. Check `rows_affected == 1`; if 0 return a conflict error to the caller.
   - Human player gets a "please retry" error.
   - Bot retries via its existing retry loop (the conflict is treated like a
     validation error - the next attempt re-fetches fresh state via the
     post-Ollama state-change detection already in place).

No migration needed - `updated_at` already exists on `games` with the required
precision. Changes required: `execute_command` in `game/mod.rs`, signature and
UPDATE query in `update_game_command_success` in `db.rs`.

### NATS eventing (future)

v1 bot triggering uses direct HTTP (monolith POSTs to bot service). This will
be replaced with NATS eventing in a later phase:
- Monolith publishes "bot turn needed" event to NATS.
- Bot service subscribes, processes, publishes "bot command" event back.
- Eliminates the direct coupling and makes bot triggering resilient to bot
  service restarts. Also naturally handles simultaneous bot turns and chaining.
The in-process `tokio::sync::broadcast` WebSocket fan-out is also a candidate
for replacement with NATS Core (noted in Phase 4).

### Notes

- The bot caller does not need a game session. It authenticates to the monolith
  via `INTERNAL_API_KEY` only.
- Bot players have no `users` row. `game_players.user_id` is null for bots;
  `game_players.game_bot_id` points to the `game_bots` row for that game.
- `think: false` must be set in the Ollama `/api/chat` request body for
  qwen3.5 models to suppress chain-of-thought output and keep responses fast
  and concise. It is a top-level field alongside `model` and `messages`.
- mirrord intercepts all outgoing TCP including `localhost`. Both `rust/web` and
  `rust/bot` have `outgoing.filter.local: ["127.0.0.1"]` in their mirrord
  configs so localhost connections (Ollama on 11434, cross-service calls) bypass
  the proxy and use the host network directly.
- Bot restart limitation: `restart_game` only collects human `opponent_ids`;
  bots are not recreated in the restarted game.
- XOR CHECK constraint on `game_players` enforces exactly one of `user_id` /
  `game_bot_id` is non-null.
- `BotSlot` is defined in `game/server_fns.rs` (not SSR-gated) and re-exported
  from `db.rs`. It must compile on WASM as a server fn parameter.
- The GameVersion CRD is managed outside Tilt in `setup-kind-cluster.sh` to
  avoid finalizer deadlocks. Do not add it back to any kustomization.


---

## Phase 6.5: Production CD (ArgoCD) [Pending]

**Goal:** Replace manual `kubectl apply -k k8s/prod` with ArgoCD for GitOps
continuous delivery. GitHub Actions handles CI (build + push to GHCR). ArgoCD
handles CD (sync cluster state to Git). Database migrations run as an ArgoCD
PreSync hook so a failed migration halts the sync before any pods are replaced.

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

### Image update flow (separate config repo)

Image tags are tracked in a dedicated `brdgme-config` repo (separate from this
source repo). GitHub Actions pushes images to GHCR then commits the updated
tags to `brdgme-config`. ArgoCD watches `brdgme-config`, not this repo.

Rationale: committing tags back to the source repo creates CI loop risk and
mixes deployment history with code history. A separate config repo keeps
rollback simple (revert the tag commit, ArgoCD syncs) without any additional
tooling. If a more integrated official mechanism ships with ArgoCD in future it
should be evaluated then.

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

## Phase 7: Side-by-Side Validation [Pending]

**Goal:** Run old and new systems simultaneously against the same database so
they can be compared directly before committing to cutover. Legacy services
(`rust/api`, `web`, `websocket`) are kept alive until `rust/web` is proven in
production.

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

### Decommission (once rust/web is proven in production)

When the new system has run in production without issues, remove the legacy
stack in this order:

- [ ] Remove `api`, `websocket`, and `web-legacy` from the kustomization and
      delete their k8s manifests.
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds from the Tiltfile.

Redis remains after this step - it is still used by `rust/web`. Removal
happens in Phase 8.

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

## Phase 8: NATS Migration + WS simplification [Pending]

**Goal:** Replace Redis pub/sub with NATS Core, remove Redis, and simplify
the WebSocket broadcast path now that legacy compat is gone.

### WS payload strategy change (fat → skinny)

During Phase 6-7, fat payloads are justified because legacy compat already
requires per-player `get_game_logs` DB queries and auth token lookups - the
`BrdgmeUpdate` comes for free. Post-decommission that cost exists solely to
serve the fat payload. Logs also grow unboundedly with game length.

The per-player complexity (different board HTML, `command_spec`, private logs
per player) is the root cause of most of `broadcast_game_update`'s weight.
Skinny payloads eliminate the need for player-specific messages entirely -
player-specific data comes back through the authenticated `get_game_details`
re-fetch, which is the right place for it anyway.

In Phase 8, `broadcast_game_update` reduces to a single publish:
- Publish `{"game_id": "..."}` once to `game.{id}` (no per-player loop)
- Client re-fetches `get_game_details` + `get_game_logs` on receipt
- Remove: all `Legacy*` structs, per-player loop, auth token lookup,
  per-player `get_game_logs` calls, session extraction in `ws_handler`,
  `ws.{user_id}` channel, `BrdgmeGameUpdate`/`WebSocketMessage` enum
- `/ws` handler reverts to simple `PSUBSCRIBE game.*`, no session needed

### Infrastructure

- [ ] Add NATS Core to the Kind cluster dev environment (Tiltfile + k8s
      manifests in `k8s/base/nats/`).
- [ ] Add NATS to `k8s/base/brdgme/kustomization.yaml`.
- [ ] Replace Redis `PUBLISH`/`SUBSCRIBE` in `websocket.rs` with `async-nats`
      publish/subscribe. Subject naming: `game.{id}` and `ws.{user_id}`.
- [ ] Simplify `broadcast_game_update` to skinny signal (see above).
- [ ] Remove the `redis` dependency from `rust/web/Cargo.toml`.
- [ ] Remove Redis from `k8s/base/brdgme/kustomization.yaml` and delete
      `k8s/base/redis/`.
- [ ] Remove Redis port-forward from the Tiltfile.

**Note:** NATS Core → JetStream upgrade path (persistent delivery) requires
only a config flag change and a volume. No code change needed.

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
