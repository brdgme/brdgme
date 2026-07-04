# Phase 5: Frontend (Leptos UI)

**Status:** Complete

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

