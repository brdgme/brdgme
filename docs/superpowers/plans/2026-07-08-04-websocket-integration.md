# 04: WebSocket Integration - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/04-websocket-integration.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete

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
