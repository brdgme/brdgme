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
- Added `cargo-leptos` and `dart-sass` to `devenv.nix`.
- Pinned `wasm-bindgen` to `0.2.100`.
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
- [ ] Install Cilium as the CNI into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [ ] Verify pod networking and DNS work correctly.

### Knative Serving

- [ ] Install Knative Serving into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [x] Configure Cilium as the Knative networking layer via `net-gateway-api`
      (Cilium's GatewayClass + Knative Gateway API ingress class). Setup
      automated in `scripts/setup-kind-cluster.sh`.
- [ ] Verify a simple Knative Service deploys and is reachable.

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
- [ ] **Migrate CI to GitHub Actions** (prerequisite for skaffold and Travis
      removal): Replace `.travis.yml` with GitHub Actions workflows. Build all
      service images and push to GHCR (`ghcr.io/beefsack/brdgme/*`). Update
      `k8s/prod` image references to GHCR. Deprecates both Travis CI and
      `skaffold.yaml`.
- [ ] Remove `skaffold.yaml` and `.travis.yml` once GitHub Actions CI is
      verified working.

### Production builds

Production image builds and deploys are driven by GitHub Actions (see item
above). `kubectl apply -k k8s/prod` is still the deploy mechanism.

---

## Phase 5.6: Pre-Cutover Fixes [In Progress]

**Goal:** Resolve all blockers and close critical gaps found in the parity
review before the `leptos` branch replaces production. Full review in
`docs/REVIEW.md`.

### Blockers (must fix before cutover)

- [ ] **Auth in Axum handlers** (`game/server.rs`): Replace `Uuid::nil()` in
      `create_game` and `play_command` with the authenticated user ID from the
      session.
- [ ] **Login UI wired to server functions** (`app.rs`): Connect
      `on_email_submit` and `on_code_submit` to the `Login` and `ConfirmLogin`
      server functions.
- [ ] **Confirmation token not exposed in response** (`auth/server.rs`): Remove
      the token from the login response body.
- [ ] **Persistent session store** (`auth/session.rs`): Replace `MemoryStore`
      with `tower-sessions-sqlx-store`. Requires a new SQLx migration to create
      the sessions table (refer to `tower-sessions-sqlx-store` docs for the
      schema) and adding the crate to `Cargo.toml`.
- [ ] **`with_secure` env-driven** (`auth/session.rs`): Read from environment,
      not hardcoded `false`.
- [ ] **Graceful SIGTERM shutdown** (`main.rs`): Add
      `axum::serve(...).with_graceful_shutdown(...)` listening for SIGTERM.
- [ ] **Turn enforcement** (`game/server.rs`): Reject commands when it is not
      the authenticated player's turn.
- [ ] **Authenticate `GET /api/game/{id}`** (`game/server.rs`): Require a
      valid session.
- [ ] **`GamePlayer` model missing fields** (`models/game.rs`): Add
      `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
      `rating_change`. Required before undo, mark_read, and points work.
- [ ] **`update_game_command_success` writes all fields** (`db.rs`): Persist
      `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`, and
      points on every command.
- [ ] **`find_game_extended` handles missing `game_type_users` row** (`db.rs`):
      Use a LEFT JOIN with a default rating rather than erroring. Migration risk
      for any existing game where a player row is absent.
- [ ] **Token expiry check in `validate_session_token`** (`auth/session.rs`):
      Auth tokens are currently permanent. Add an expiry check (30-day window
      matching the old system).
- [ ] **Email sending not implemented** (`auth/server.rs`): The confirmation
      token is generated but never emailed. The old system sent the code via the
      in-cluster SMTP service. The new system must do the same before real users
      can log in. The SMTP service is already deployed.

### Missing endpoints (non-blocking, needed for feature parity)

- [ ] **`POST /game/{id}/undo`**: Restore `undo_game_state`, call `Status` on
      the game service, clear all players' undo state, write a log entry,
      broadcast.
- [ ] **`POST /game/{id}/mark_read`**: Set `is_read = true` on the calling
      player's `game_players` row.
- [ ] **`POST /game/{id}/concede`**: Limited to 2-player games. Mark game
      finished, write log entry, broadcast.
- [ ] **`POST /game/{id}/restart`**: Create new game with same players, link
      via `restarted_game_id`, broadcast `GameRestarted`. Client must navigate
      to the new game URL on receipt.

### Frontend gaps (non-blocking)

- [ ] **New-game creation UI** (`app.rs`, `GamesPage`): Game type selector,
      opponent email inputs, submit → redirect to new game. Requires a server
      function returning available game types from `game_versions`.
- [ ] **Game log rendering** (`components/game.rs`): Replace `GameLogs` stub
      with actual log display: fetch logs, render markup to HTML, group by
      10-minute windows, filter to logs since `last_turn_at`.
- [ ] **Undo/concede/restart actions in `GameMeta`**: Wire the "Concede" anchor
      and add "Undo" and "Restart" links with correct visibility conditions
      (`can_undo`, game finished, `restarted_game_id` absent).
- [ ] **"Whose turn" display** (`app.rs`): Replace generic "Waiting on
      opponents..." with the specific player name(s) and color.
- [ ] **Mark-read on game page load** (`app.rs`): Call `mark_read` when
      `GamePage` mounts and when the game ID changes.
- [ ] **`GameRestarted` WebSocket navigation** (`websocket_client.rs`): On
      receipt of `GameRestarted`, navigate to the new game URL rather than
      just incrementing the trigger counter.
- [ ] **Command input: clear after server confirms** (`components/game.rs`):
      Move `set_command("")` to run after `submit_action` succeeds, not before.
- [ ] **Command errors surfaced to user** (`components/game.rs`): Observe
      `submit_action` result and display errors.
- [ ] **Clickable command suggestions** (`components/game.rs`): Clicking a
      suggestion appends it to the command input.
- [ ] **Autocomplete prefix filtering** (`rust/lib/game`): Add
      `CommandSpec::suggest(input, names) -> Vec<String>` with prefix-aware
      `Token` filtering. Do not change `parse()`. Update `GameCommandInput` to
      call `suggest` instead of using `expected` from parse errors.

### Code quality (non-blocking)

- [ ] **Dead code removed**: `New*` model structs, `chat.rs`, `friends.rs`,
      `PublicGameType` alias, `SESSION_AUTH_TOKEN_KEY`, `db::AppState`.
- [ ] **`reqwest::Client` shared** (`game/client.rs`): Create once at startup,
      store in `AppState`, reuse across requests.
- [ ] **N+1 in `find_active_games_for_user`** (`db.rs`): Replace loop with a
      single joined query.
- [ ] **Duplicate command logic** (`game/server.rs` vs `server_fns.rs`):
      Consolidate into one path.
- [ ] **`NaiveDateTime` → `DateTime<Utc>`** (`models/`): Preserve timezone
      throughout.
- [ ] **Points persisted** (`db.rs`): Remove `_points` suppression in
      `update_game_command_success`.
- [ ] **Logout redirect/feedback** (`components/layout.rs`).
- [ ] **WebSocket reconnection** (`websocket_client.rs`).
- [ ] **`finished_at` set when `is_finished = true`** (`db.rs`): Verify schema
      has no trigger; if not, set `finished_at` explicitly.

---

## Phase 6: NATS Integration [Pending]

**Goal:** Replace the in-process `tokio::sync::broadcast` WebSocket fan-out
with NATS Core pub/sub, enabling the monolith to run as multiple replicas.
This also unblocks Redis removal.

### Infrastructure

- [ ] Add NATS Core to the Kind cluster dev environment (Tiltfile + k8s
      manifests in `k8s/base/nats/`).
- [ ] Add NATS to the `k8s/base/brdgme/kustomization.yaml` alongside Postgres
      and Redis.

### Application changes (`rust/web`)

- [ ] Add `async-nats` to `rust/web/Cargo.toml` under the `ssr` feature.
- [ ] Replace `GameBroadcaster` in `websocket.rs`: publish game updates to a
      NATS subject (`game.{id}`) instead of a tokio broadcast channel.
- [ ] Subscribe each WebSocket handler to the relevant NATS subject and forward
      messages to the connected client.
- [ ] Remove the `tokio::sync::broadcast` channel from `AppState` and
      `GameBroadcaster`.
- [ ] Remove the `redis` dependency from `Cargo.toml` (was listed but unused).

### Cleanup

- [ ] Remove Redis from `k8s/base/brdgme/kustomization.yaml` and delete
      `k8s/base/redis/`.
- [ ] Remove Redis port-forward from the Tiltfile.

**Note:** NATS Core → JetStream upgrade path (for persistent message delivery)
requires only a config flag change and a volume for persistence. No code change
needed. Out of scope for this phase.

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

- [ ] Create `k8s/argocd/brdgme-app.yaml`: an `Application` resource pointing
      to this repo, `k8s/prod` kustomize path, auto-sync enabled, prune
      enabled, self-heal enabled.
- [ ] Commit the `Application` manifest to the repo so ArgoCD manages itself
      (app-of-apps pattern is not needed at this scale - a single Application
      is sufficient).

### Database migration PreSync hook

- [ ] Create `k8s/base/migrate/job.yaml`: a `Job` that runs
      `sqlx migrate run` using the `brdgme/web` image and the `postgres-config`
      secret. Annotate with:
      - `argocd.argoproj.io/hook: PreSync`
      - `argocd.argoproj.io/hook-delete-policy: BeforeHookCreation`
- [ ] Add `k8s/base/migrate/` to `k8s/base/brdgme/kustomization.yaml`.
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

Both systems share PostgreSQL and the game microservices. Auth mechanisms are
different (Bearer token vs session cookie) so each requires a separate login -
this is acceptable for testing. Real-time WebSocket updates do not cross system
boundaries (a move in one UI will not push a notification to the other), but
both UIs show correct state on next page load.

### Risks

- `web/Dockerfile` pins `node:14.7.0` (EOL April 2023). npm install may fail
  against current registries. The old frontend build should be verified early
  in this phase; if it fails, the Node version will need bumping.

### Image naming

The old React frontend (`web/Dockerfile`) and the new Leptos SSR app
(`rust/Dockerfile` `web` target) previously shared the image tag `brdgme/web`.
The new Leptos app keeps `brdgme/web`. The old React frontend is renamed to
`brdgme/web-legacy`.

### Infra changes needed

- [x] New Leptos app: `rust/Dockerfile` `web` target → `brdgme/web`. k8s
      manifests in `k8s/base/web/` unchanged.
- [ ] Add `brdgme/web-legacy` image build to the Tiltfile (from
      `web/Dockerfile`, final stage `web`, tagged `brdgme/web-legacy`).
- [ ] Add `brdgme/api` and `brdgme/websocket` image builds to the Tiltfile.
- [ ] Create `k8s/base/web-legacy/` manifests (Deployment + Service) using
      `image: brdgme/web-legacy`. Mirror the structure of `k8s/base/web/` but
      with `name: web-legacy`.
- [ ] Create `k8s/base/legacy/kustomization.yaml` grouping `web-legacy`, `api`,
      and `websocket` as the legacy stack.
- [ ] Restore `api` and `websocket` manifests to an active kustomization overlay
      alongside the legacy frontend.
- [ ] Add hostname-based routing to the ingress: primary domain → `web`;
      legacy subdomain (e.g. `old.brdgme.com`) → `web-legacy` + `api`.
- [ ] Verify the old React frontend's API base URL is configured to point to
      the `api` service, not `web`.

### Decommission (once rust/web is proven in production)

When the new system has run in production without issues, remove the legacy
stack in this order:

- [ ] Remove `api`, `websocket`, and `web-legacy` from the kustomization and
      delete their k8s manifests.
- [ ] Remove Redis (no longer needed once NATS replaces the old WebSocket
      fan-out path).
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds from the Tiltfile.

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

## Development Workflow

Requires a Kind cluster with Cilium and Knative. Run once per workstation:

```
bash scripts/setup-kind-cluster.sh
```

### Hybrid (Fast Iteration) - default

```
tilt up
```

Deploys backing services (Postgres, Redis, SMTP, game microservices) to Kind
and port-forwards Postgres (5432) and Redis (6379) to localhost. Run
`cargo leptos watch` inside `rust/web` for hot-reloading on port 3000.

### Full Cluster Test

```
WEB_IN_CLUSTER=1 tilt up
```

Builds and deploys `brdgme/web` as a Knative Service into Kind alongside all
backing services.
