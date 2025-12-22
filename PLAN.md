# Detailed Migration Plan: Rust Monolith (Axum + Leptos)

## Objective
Consolidate the `brdgme` platform into a single Rust-based monolithic application using **Axum** (Backend) and **Leptos** (Frontend/WASM). This replaces the Rocket API, Node.js WebSocket service, and TypeScript/React frontend.

## Strategy
We will perform the migration **in parallel** where possible, building the new system in the existing `rust/web` directory (which will be the new monolith). The old services (`rust/api`, `web`, `websocket`) will remain untouched until the new system is ready for a cutover.

## Phase 1: Foundation & Shared Logic
**Goal:** Ensure core logic libraries (`brdgme_cmd`, `brdgme_game`) are compatible with WASM (WebAssembly) so they can be used in the frontend.

1.  **Refactor `brdgme_cmd` dependencies:**
    *   Edit `rust/lib/cmd/Cargo.toml`: Make `warp` an optional dependency.
    *   Define a generic feature (e.g., `http-server`) that enables `warp`.
2.  **Gate HTTP Logic:**
    *   Edit `rust/lib/cmd/src/lib.rs`: Gate the `http` module declaration behind `#[cfg(feature = "http-server")]`.
    *   Ensure the core parsing logic (`cli.rs` or similar) is *not* behind this feature.
3.  **Verification:**
    *   Run `cargo check --target wasm32-unknown-unknown -p brdgme_cmd` to confirm WASM compatibility.

## Phase 2: Database Layer (Async/SQLx) [COMPLETE]
**Goal:** Establish the data layer for the new Axum application, moving from Diesel (Sync) to SQLx (Async).

1.  **Setup SQLx in `rust/web`:**
    *   Verify `sqlx` dependency in `rust/web/Cargo.toml` (postgres, runtime-tokio, uuid, chrono).
    *   Create a `.env` file in `rust/web` for the database URL.
2.  **Migrate Schemas (Baseline Strategy):**
    *   Initialize `rust/web/migrations`.
    *   **Action:** Create a single "baseline" migration file (e.g., `20250101000000_init.sql`) containing a snapshot of the *entire* existing database schema.
    *   *Rationale:* This establishes the initial state without needing to port historical Diesel migrations. SQLx will manage all *future* schema changes relative to this baseline.
3.  **Implement Data Access:**
    *   Create `rust/web/src/db.rs`.
    *   Implement basic user retrieval and session storage queries using SQLx.

## Phase 3: The New Backend (Axum Core)
**Goal:** Replicate the core API logic (Auth, Game Orchestration) in Axum.
*Note: "Play by Email" functionality is out of scope for this migration and will be re-implemented later.*

1.  **Authentication:**
    *   Implement routes in `rust/web/src/auth/` for `login`, `register`, and `logout`.
    *   Use `tower-sessions` (already in Cargo.toml) for managing user sessions.
2.  **Game Client Adapter:**
    *   Create `rust/web/src/game/client.rs`.
    *   Implement an async client (using `reqwest`) to communicate with the external Game Microservices (Go/Rust games).
    *   *Constraint:* This must still use JSON to communicate with the external services as defined in `ARCHITECTURE.md`.
3.  **Contract Verification (Simple Mock):**
    *   **Action:** Write a unit test that mocks a Game Service response.
    *   Verify that `client.rs` correctly serializes commands and deserializes the specific JSON structure expected by the game services.
4.  **Game API Endpoints:**
    *   Implement `POST /api/game/new`: Create a new game, call Game Service, save initial state to DB.
    *   Implement `POST /api/game/{id}/command`: Receive command, validate, send to Game Service, update DB.
    *   Implement `GET /api/game/{id}`: Retrieve game state.

## Phase 4: WebSocket Integration
**Goal:** Internalize real-time updates, removing the Node.js/Redis dependency for basic deployments.

1.  **WebSocket Handler:**
    *   Add `ws` route in `rust/web/src/main.rs` using `axum::extract::ws::WebSocketUpgrade`.
2.  **Broadcast System:**
    *   Create a `GameBroadcaster` struct using `tokio::sync::broadcast`.
    *   When the Game API (Phase 3) updates a game, it should push a message to this broadcaster.
    *   The WebSocket handler subscribes to this channel and pushes JSON updates to the connected client.

## Phase 5: The Frontend (Leptos UI) [COMPLETE]
**Goal:** Build the UI in Rust, replacing React.

## Phase 5.5: Functional Verification (Pending)
**Goal:** Run the new application alongside production data and verify all features.
1.  **Walkthrough:** Test game navigation, rendering, and move submission.
2.  **Live Sync:** Verify that moves made in the new UI reflect instantly via WebSockets.
3.  **UI Fidelity:** Ensure styling and lo-fi ASCII aesthetics match or improve upon the legacy frontend.

## Phase 6: Cutover & Cleanup
**Goal:** Switch to the new system.

1.  **Containerization:**
    *   Update `rust/Dockerfile` to build the `web` binary (optimizing for size/layers).
2.  **Deployment:**
    *   Update `skaffold.yaml` to deploy the `web` image as the main entry point.
    *   Remove `api` (Rust/Rocket) and `websocket` (Node) deployments.
3.  **Cleanup:**
    *   Delete `rust/api/`, `web/`, and `websocket/` directories once verified.

## Development Workflow (Hybrid)

To achieve fast iteration cycles while maintaining production parity for dependencies, we use a hybrid workflow:

1.  **Backing Services (K8s):**
    *   Run `skaffold dev --port-forward`.
    *   This deploys Postgres, Redis, and all Game Microservices to the local Kubernetes cluster.
    *   It **skips** building the `web` application to save time.
    *   It automatically forwards Postgres (5432) and Redis (6379) to your host machine.

2.  **Web Application (Host):**
    *   Run `cargo leptos watch` inside `rust/web`.
    *   This runs the monolithic web server locally on port 3000.
    *   It connects to the backing services via the forwarded ports (`localhost:5432`).
    *   It provides sub-second hot reloading for UI changes.

**Full Cluster Test:**
To test the full containerized stack (including the web container), run: `skaffold dev -p with-web`.