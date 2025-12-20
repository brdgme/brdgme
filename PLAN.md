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

## Phase 2: Database Layer (Async/SQLx)
**Goal:** Establish the data layer for the new Axum application, moving from Diesel (Sync) to SQLx (Async).

1.  **Setup SQLx in `rust/web`:**
    *   Verify `sqlx` dependency in `rust/web/Cargo.toml` (postgres, runtime-tokio, uuid, chrono).
    *   Create a `.env` file in `rust/web` for the database URL.
2.  **Migrate Schemas:**
    *   Initialize `rust/web/migrations`.
    *   Manually copy/convert schema definitions from `rust/api/migrations` (Diesel) to SQLx SQL files.
    *   Key Tables: `users`, `games`, `game_versions`, `game_players`, `game_logs`.
3.  **Implement Data Access:**
    *   Create `rust/web/src/db.rs`.
    *   Implement basic user retrieval and session storage queries using SQLx.

## Phase 3: The New Backend (Axum Core)
**Goal:** Replicate the core API logic (Auth, Game Orchestration) in Axum.

1.  **Authentication:**
    *   Implement routes in `rust/web/src/auth/` for `login`, `register`, and `logout`.
    *   Use `tower-sessions` (already in Cargo.toml) for managing user sessions.
2.  **Game Client Adapter:**
    *   Create `rust/web/src/game/client.rs`.
    *   Implement an async client (using `reqwest`) to communicate with the Game Microservices (Go/Rust games).
    *   Functionality: Send `New`, `Play`, `Status` commands to game services.
3.  **Game API Endpoints:**
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

## Phase 5: The Frontend (Leptos UI)
**Goal:** Build the UI in Rust, replacing React.

1.  **App Shell:**
    *   Set up `rust/web/src/app.rs` with `leptos_router`.
    *   Create `Layout` component (Navbar, Footer).
2.  **Game View Component:**
    *   Fetch game data from the Axum API (Server Functions or Resource loading).
    *   Render the ASCII board. *Note: Use `brdgme_markup` crate (shared) to convert markup to HTML.*
3.  **Interactive Command Input:**
    *   **Crucial Step:** Import `brdgme_cmd` in the frontend code.
    *   Use the parser *client-side* to validate input and show suggestions as the user types (Autocomplete).
    *   On submit, send the command string to the Axum API.
4.  **Live Updates:**
    *   Connect to the WebSocket endpoint established in Phase 4.
    *   Update the local Leptos Signals when a game update message is received.

## Phase 6: Cutover & Cleanup
**Goal:** Switch to the new system.

1.  **Containerization:**
    *   Update `rust/Dockerfile` to build the `web` binary (optimizing for size/layers).
2.  **Deployment:**
    *   Update `skaffold.yaml` to deploy the `web` image as the main entry point.
    *   Remove `api` (Rust/Rocket) and `websocket` (Node) deployments.
3.  **Cleanup:**
    *   Delete `rust/api/`, `web/`, and `websocket/` directories once verified.