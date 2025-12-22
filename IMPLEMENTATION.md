# Implementation Log

This document tracks the execution of the migration plan defined in `PLAN.md`. It serves as a living record of changes, technical hurdles, and design decisions.

## Current Status

**Current Phase:** Phase 6 (Cutover & Cleanup)
**Goal:** Switch to the new system and remove legacy services.

### Progress Checklist

#### Phase 1: Foundation & Shared Logic
- [x] **Step 1.1:** Refactor `rust/lib/cmd/Cargo.toml` (Make `warp` optional)
- [x] **Step 1.2:** Gate HTTP logic in `rust/lib/cmd/src/lib.rs`
- [x] **Step 1.3:** Verify WASM compilation for `brdgme_cmd`

#### Phase 2: Database Layer (Async/SQLx)
- [x] **Step 2.1:** Setup SQLx in `rust/web`
- [x] **Step 2.2:** Migrate schemas from Diesel to SQLx
- [x] **Step 2.3:** Implement `db.rs` (Async Data Access)

#### Phase 3: The New Backend (Axum Core)
- [x] **Step 3.1:** Implement Authentication (`auth/`)
- [x] **Step 3.2:** Implement Game Client Adapter
- [x] **Step 3.3:** Contract Verification (Simple Mock)
- [x] **Step 3.4:** Implement Game API Endpoints

#### Phase 4: WebSocket Integration
- [x] **Step 4.1:** Implement WebSocket Handler (`axum::extract::ws`)
- [x] **Step 4.2:** Implement Broadcast System (`tokio::sync::broadcast`)

#### Phase 5: The Frontend (Leptos UI)
- [x] **Step 5.1:** Build App Shell & Layout
- [x] **Step 5.2:** Shared Types & Server Functions
- [x] **Step 5.3:** Build Game View Component
- [x] **Step 5.4:** Implement Client-side Command Parsing
- [x] **Step 5.5:** Implement Live Updates (WebSocket connection)

#### Phase 6: Cutover & Cleanup
- [x] **Step 6.1:** Update Dockerfile
- [x] **Step 6.2:** Update Skaffold
- [ ] **Step 6.3:** Remove legacy code (Deferred for reference)

---

## Log

### [Date] Phase 2 Baseline & Phase 3 Adapter Complete
- **Completed:**
    - Successfully executed **Phase 2 Baseline Migration**: Applied `001_initial_schema.sql` to local PostgreSQL dev instance via `sqlx-cli`.
    - **Phase 3.2 & 3.3 Complete**: Implemented `rust/web/src/game/client.rs` to handle communication with external game microservices. 
    - Verified the adapter contract with a mock server test (`test_game_client_contract`).
    - Added `reqwest` (with `rustls`) and `sqlx-cli` to the project environment (`devenv.nix`).
    - Configured `DATABASE_URL` in `devenv.nix` for seamless development.
- **Next Steps:**
    - **Phase 3.4: Implement Game API Endpoints**: Wire the client adapter into Axum routes for creating and playing games.

### [Date] Phase 3.4 Complete: Game API Endpoints
- **Completed:**
    - Implemented core game database logic in `rust/web/src/db.rs` using SQLx, including `create_game_with_users`, `find_game_extended`, and `update_game_command_success`.
    - Created `rust/web/src/game/server.rs` with Axum handlers for `POST /api/game/new`, `GET /api/game/:id`, and `POST /api/game/:id/command`.
    - Refactored application state in `rust/web/src/state.rs` to support a combined `AppState` (LeptosOptions + PgPool) shared between pure Axum handlers and Leptos routes.
    - Successfully verified compilation with `cargo check`.
- **Next Steps:**
    - **Phase 4: WebSocket Integration**: Implement real-time game updates using `tokio::sync::broadcast` and Axum WebSockets.

### [Date] Phase 4 Complete: WebSocket Integration
- **Completed:**
    - Created `rust/web/src/websocket.rs` with `GameBroadcaster` using `tokio::sync::broadcast` for internal real-time event distribution.
    - Implemented Axum WebSocket handler `/ws` to stream filtered game updates to clients.
    - Integrated broadcasting into `create_game` and `play_command` endpoints.
    - Added `futures-util` dependency for asynchronous stream management.
- **Next Steps:**
    - **Phase 5: The Frontend (Leptos UI)**: Begin building the actual user interface in Rust, starting with the App Shell and Layout.

### [Date] Phase 5 Complete: Leptos Frontend Implementation
- **Completed Frontend UI:**
    - Developed a complete App Shell with modular components: `GameBoard` (ASCII-to-HTML), `GameMeta`, `GameLogs`, and `GameCommandInput`.
    - Implemented Server Functions for active games, game details, and command submission.
    - Ported legacy styling into the Leptos layout for visual consistency.
- **Completed Interactivity:**
    - Implemented client-side command parsing using `brdgme_game` parsers compiled to WASM, providing real-time suggestions and validation.
    - Built a client-side WebSocket hook (`websocket_client.rs`) that triggers automatic resource refetches via a global `WebSocketTrigger` context.
- **Infrastructure & Compatibility:**
    - Updated `devenv.nix` with `cargo-leptos` and `dart-sass`.
    - Resolved WASM target issues: pinned `wasm-bindgen` to `0.2.100`, enabled `js`/`wasm_js` features for `getrandom`, and gated non-WASM dependencies in `brdgme_cmd`.
    - Updated Axum routing to modern `{id}` syntax.
- **Next Steps:**
    - **Phase 5.5: Functional Verification**: Perform end-to-end testing of gameplay flows using restored production data.
    - **Phase 6: Cutover & Cleanup**: Containerization and decommissioning of legacy services.

### [Date] Phase 6 Refinement: Build & Dev Environment Stability
- **Resolved Build Issues:**
    - **cargo-leptos**: Switched to `cargo-binstall` in Dockerfile to avoid `serde` compilation errors during tool installation.
    - **dart-sass**: Fixed pathing issues in Dockerfile by preserving directory structure.
    - **WASM Build**: Isolated `cargo chef cook` for the `web` crate to prevent non-WASM dependencies (like `mio`, `socket2`) from breaking the WASM build graph.
    - **Database/SQLx**: Implemented `SQLX_OFFLINE=true` support by generating `.sqlx` metadata. Added port-forwarding to Skaffold to allow local `cargo build` to verify queries against the K8s Postgres instance.
- **Optimized Development Loop:**
    - Refactored `skaffold.yaml` to implement a "fast by default" workflow.
    - **Default Profile**: Deploys only backing services (Postgres, Redis) and Game Services, skipping the slow `web` build.
    - **Port Forwarding**: Automatically forwards Postgres (5432) and Redis (6379) to localhost.
    - **Hybrid Workflow**: Established a pattern of running `skaffold dev` for backend services while running `cargo leptos watch` locally for instant UI feedback.

### 2025-12-22: Fixed Database Connection Pool in Server Functions
- **Problem Diagnosed:**
    - Server functions were failing with "Database pool not found" errors at runtime.
    - The issue was caused by attempting to use Axum `State` extraction (`State<AppState>`) with `leptos_axum::extract()` in server functions.
    - The `extract()` call had no state context, causing it to fail when trying to extract `AppState`.
- **Solution Implemented:**
    - Switched to Leptos's recommended **context-based dependency injection** pattern.
    - Updated `main.rs` to use `leptos_routes_with_context()` instead of `leptos_routes()`.
    - Provided `PgPool` and `GameBroadcaster` via `provide_context()` in the routing setup.
    - Updated all server functions in `server_fns.rs` to use `use_context::<PgPool>()` and `use_context::<GameBroadcaster>()` instead of Axum state extraction.
    - Changed from `expect_context()` (which panics) to `use_context()` with proper error handling.
- **Outcome:**
    - `cargo leptos build` now compiles successfully.
    - `cargo leptos watch` runs without database connection errors.
    - Server functions can now properly access the database pool and broadcaster.
- **Technical Note:**
    - This follows the Leptos best practice of using reactive context for dependency injection rather than Axum's state extraction system.
    - The context is provided at the router level and automatically available to all server functions within that scope.