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
- [ ] **Step 6.1:** Update Dockerfile
- [ ] **Step 6.2:** Update Skaffold
- [ ] **Step 6.3:** Remove legacy code

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

### [Date] Phase 5.1 & 5.2 Complete: App Shell & Server Functions
- **Completed:**
    - Refactored `rust/web/src/app.rs` and `rust/web/src/components/layout.rs` to use dynamic data.
    - Implemented `get_active_games` and `get_game_details` Server Functions in `rust/web/src/game/server_fns.rs`.
    - Established shared types (`GameSummary`, `GameViewData`) for type-safe frontend-backend communication.
    - Wired up `SidebarMenu` to display real active games and `GamePage` to render the live game board (HTML from ASCII).
- **Next Steps:**
    - **Phase 5.3: Build Game View Component**: Refactor the game board, meta-data, and logs into modular Leptos components.

### [Date] Phase 5 Complete: Leptos Frontend UI
- **Completed:**
    - Built a dynamic App Shell with modular components (`GameBoard`, `GameMeta`, `GameLogs`, `GameCommandInput`).
    - Implemented client-side command validation and suggestions using the `brdgme_game` parser compiled to WASM.
    - Integrated real-time live updates via WebSockets, using a global `WebSocketTrigger` context to automatically refresh Leptos Resources (`get_active_games`, `get_game_details`).
    - Successfully resolved WASM compatibility issues with `getrandom` and `brdgme_cmd` default features.
- **Next Steps:**
    - **Phase 6: Cutover & Cleanup**: Prepare for deployment by updating Docker/Skaffold and eventually removing legacy services.