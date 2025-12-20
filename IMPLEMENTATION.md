# Implementation Log

This document tracks the execution of the migration plan defined in `PLAN.md`. It serves as a living record of changes, technical hurdles, and design decisions.

## Current Status

**Current Phase:** Phase 3 (The New Backend (Axum Core))
**Goal:** Replicate the core API logic (Auth, Game Orchestration) in Axum.

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
- [ ] **Step 3.2:** Implement Game Client Adapter
- [ ] **Step 3.3:** Implement Game API Endpoints

#### Phase 4: WebSocket Integration
- [ ] **Step 4.1:** Implement WebSocket Handler (`axum::extract::ws`)
- [ ] **Step 4.2:** Implement Broadcast System (`tokio::sync::broadcast`)

#### Phase 5: The Frontend (Leptos UI)
- [ ] **Step 5.1:** Build App Shell & Layout
- [ ] **Step 5.2:** Build Game View Component
- [ ] **Step 5.3:** Implement Client-side Command Parsing
- [ ] **Step 5.4:** Implement Live Updates (WebSocket connection)

#### Phase 6: Cutover & Cleanup
- [ ] **Step 6.1:** Update Dockerfile
- [ ] **Step 6.2:** Update Skaffold
- [ ] **Step 6.3:** Remove legacy code

---

## Log

### [Date] Phase 1 Initialization
- Started work on refactoring `brdgme_cmd`.
- **Challenge:** Need to separate HTTP logic from core logic in `brdgme_cmd` without breaking existing games that might rely on the HTTP server being present by default (until they are updated).
- **Decision:** We will use a default feature `http-server` in `Cargo.toml`. This ensures existing backend code continues to work without modification, while the frontend can opt-out by disabling default features.

### [Date] Phase 1 Complete / Phase 2 Start
- **Completed:** Refactored `brdgme_cmd` to make `warp` dependency optional. Verified compilation without default features (WASM-ready).
- **Started:** Phase 2 (Database Layer).
- **Plan:**
  1. Create `rust/web/migrations` directory.
  2. Create `20250101000000_initial_setup.sql` translating the existing Diesel schema to SQLx-compatible SQL.
  3. This includes tables: `users`, `games`, `game_versions`, `game_players`, `game_logs`, `chats`, etc.
- **Pending:** Awaiting confirmation to proceed with filesystem modifications for migrations.

### [Date] Phase 2 Complete & Phase 3 Start
- **Completed:**
    - Established SQLx database layer in `rust/web`.
    - Migrated initial schema.
    - Implemented `db.rs` with user and email management functions.
    - Implemented Authentication (`auth/`) including `login`, `confirm_login`, `logout`, and `get_current_user` with `tower-sessions`.
- **Infrastructure:**
    - Updated `devenv.nix` to use `stable` Rust channel (resolving `rustc` version mismatch).
    - Updated dependencies to latest stable versions:
        - `leptos`: `0.8.14`
        - `axum`: `0.8.7`
        - `tower-sessions`: `0.14.0`
- **Next Steps:**
    - Proceed with **Phase 3.2: Implement Game Client Adapter**.