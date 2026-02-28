# Axum + Leptos Implementation Review

Comprehensive review of `rust/web` - the Axum + Leptos monolith.

---

## Cargo.toml

### Observations

**Unused dependencies:**
- `email = "0.0.21"` - last published 2014, almost certainly abandoned. Listed
  under `ssr` features but email is explicitly out of scope. Should be removed.
- `redis = "0.28.2"` - listed under `ssr` features. Redis is being replaced by
  NATS per the vision. Should be removed once the WebSocket fan-out is
  updated.
- `tower-sessions-memory-store` - in-memory session store is not suitable for
  multi-replica deployments. Sessions will be lost on pod restart or when a
  request hits a different replica. Needs a persistent store (e.g.
  `tower-sessions-sqlx-store`) before multi-replica is viable.
- `async-trait` - largely unnecessary since Rust 1.75 (RPITIT). May be a
  legacy inclusion.
- `dotenv` - fine for development; in production, environment variables should
  come from Kubernetes secrets/configmaps, not a `.env` file. Not a problem to
  keep for dev but should not be relied on in the container image.

**Feature gate inconsistencies:**
- `websocket` module is declared in `lib.rs` without any feature gate, but it
  uses `tokio::sync::broadcast` which is server-only. The client-side
  `websocket_client.rs` is also ungated. These need careful feature gating or
  the WASM build will fail if server-only types leak in.
- `gloo-net` and `web-sys` are unconditional dependencies - they are WASM
  browser APIs and should be gated under `hydrate`. They will not compile for
  native targets without the right cfg guards in the code.

**Version pinning:**
- `wasm-bindgen = "=0.2.100"` - exact pin noted in the plan as necessary for
  WASM compatibility. This is correct but should be revisited when upgrading
  Leptos to ensure the pin is still the right version.

**Minor:**
- `rand = "0.9.2"` is unconditional - check whether it is used in
  WASM-compiled code, as `rand` requires a WASM-compatible RNG source.
  `getrandom` with `wasm_js` feature is already present which should cover
  this.

---

## `src/lib.rs`

### Observations

- `websocket` module is declared without a feature gate. If
  `websocket.rs` contains `tokio::sync::broadcast` or any other
  `ssr`-only type at the module level, this will break the WASM build. Needs
  investigation (covered when `websocket.rs` is reviewed).
- `websocket_client` is also ungated. Correct if it only uses `gloo-net` and
  `web-sys`, but those are in turn unconditional dependencies - see Cargo.toml
  note above.
- `auth` and `game` modules are ungated. If they contain server functions
  (which they do), Leptos handles the SSR/hydrate split via its own macros,
  so this is likely fine - but worth verifying no raw `#[cfg(feature = "ssr")]`
  guards are missing inside those modules.
- The `hydrate()` entry point is clean and correct.

---

## `src/main.rs`

### Observations

- Structure is correct. `dotenv().ok()` is appropriate - silently ignores
  missing `.env` in production (where env vars come from the cluster).
- `GameBroadcaster::new(1024)` - the channel capacity of 1024 is hardcoded.
  Not a problem at current scale, but worth making configurable or at least
  documented.
- Session layer is applied after the routes (`.layer(create_session_layer())`).
  In Axum, layers are applied in reverse order, so the session middleware will
  correctly wrap all routes. This is fine.
- The fallback uses `file_and_error_handler` correctly for serving static
  assets from the Leptos site root.
- No graceful shutdown handling. `axum::serve` will be killed hard on pod
  termination. For a Kubernetes deployment, a graceful shutdown hook
  (`axum::serve(...).with_graceful_shutdown(...)`) listening for SIGTERM would
  prevent dropped connections during rolling updates.
- Comments like `// run our app with hyper` and `// Load environment variables
  from .env file` describe what the code is doing rather than why. Per project
  conventions these should be removed.

---

## `src/state.rs`

### Observations

- Clean and minimal. `AppState` holds exactly what is needed.
- `FromRef` implementations for all three fields are correct and necessary for
  Axum's state extraction to work with sub-state types.
- All three fields implement `Clone` - `PgPool` and `GameBroadcaster` are both
  cheaply cloneable (internal Arc), so this is fine.
- No issues.

---

## `src/db.rs`

### Observations

**Duplicate `AppState`:**
- `db.rs` defines its own `AppState { db_pool: PgPool }` (lines 24-34) that is
  entirely unused. The real `AppState` lives in `state.rs`. This dead struct
  should be deleted.

**`find_active_games_for_user` - N+1 query:**
- Fetches game IDs first, then calls `find_game_extended` for each in a loop.
  `find_game_extended` itself issues 3+ queries per game. For a user with 10
  active games this is 30+ round-trips. Should be a single query joining all
  required tables.

**`find_game_extended` - manual struct construction:**
- The large inline struct-construction block is a consequence of SQLx's
  `query!` macro not supporting nested struct mapping. Acceptable but verbose.
  Splitting into separate focused queries (game + players + users) would be
  cleaner than one enormous `JOIN` requiring manual field mapping.

**`create_game_with_users` - `find_game_version` inside a loop:**
- `find_game_version(pool, ...)` is called inside the player loop, issuing the
  same query once per player. It should be fetched once before the loop.

**`create_game_with_users` - mixing transaction and pool references:**
- `get_user_by_email` on line 323 uses `pool` directly, not the transaction
  `tx`. All reads within a write transaction should use the transaction
  connection to avoid race conditions with concurrent requests.

**`update_game_command_success` - unused parameters:**
- `_game_player_id` and `_points` are intentionally unused, meaning point
  tracking is not written to the database after a command. This is a functional
  gap that should be tracked as a known issue.

**`create_pool` - runs migrations on startup:**
- SQLx uses an advisory lock so concurrent startup across replicas is safe, but
  this is worth knowing when debugging slow pod startup.

**`SELECT *` usage:**
- Several queries use `RETURNING *` or `SELECT *`, bypassing SQLx compile-time
  column verification. Prefer explicit column lists.

---

## `src/websocket.rs`

### Observations

**Feature gating is correct:**
- `WebSocketMessage` is shared (no gate). `GameBroadcaster` and handler are
  gated inside `mod ssr`. This is the right pattern.

**`handle_socket` - broadcasts all updates to all clients:**
- Every connected client receives every `GameUpdate` for every game regardless
  of which game they are viewing. Client must filter. Acceptable at small scale;
  server-side filtering by game ID is a future improvement.

**`handle_socket` - inbound messages discarded:**
- `_receiver` is unused. The server never reads client messages. Fine for the
  current one-way model but will need rework if client-initiated events are
  added.

**`broadcast` silently ignores send errors:**
- `let _ = self.sender.send(message)` - the only error is "no receivers" which
  is safe to ignore. Fine.

**`GameRestarted` variant:**
- Defined in `WebSocketMessage` but the client treats it identically to
  `GameUpdate`. The distinction is currently unused on the client side.

---

## `src/websocket_client.rs`

### Observations

**No reconnection logic:**
- The WebSocket is opened once inside an `Effect`. If the connection drops the
  client will not reconnect. Tolerable for async turn-based play; users can
  refresh. Known gap.

**`expect_context::<WebSocketTrigger>()` will panic:**
- If the context is not provided before this is called the process panics.
  `WebSocketTrigger` must be provided in the app root before any component
  calls `use_websocket`.

**URL construction is correct:**
- Built from `window.location` protocol and host, correctly handling `ws://`
  vs `wss://`.

**`GameRestarted` handled identically to `GameUpdate`:**
- Both variants just increment the trigger counter. The extra variant
  information is discarded.

---

## `src/auth/`

### `session.rs`

**`MemoryStore` is not suitable for production:**
- `create_session_layer()` uses `tower-sessions-memory-store`. Sessions are
  stored in process memory. On pod restart all sessions are lost (users logged
  out). With multiple replicas, a user's session only exists on the replica
  that created it; any subsequent request routed to a different replica will
  see no session. This must be replaced with a persistent store before
  multi-replica deployment. Options: `tower-sessions-sqlx-store` (uses the
  existing PostgreSQL pool, no extra infrastructure) or a Redis store.

**`with_secure(false)`:**
- Comment says "Set to true in production with HTTPS." This will send session
  cookies over plain HTTP. Must be `true` before production deployment. Should
  be driven by an environment variable, not a code change.

**`SESSION_AUTH_TOKEN_KEY` is redundant:**
- The session already stores `SessionUser` which includes `auth_token_id`.
  Storing the auth token ID separately under its own key duplicates data in the
  session. `SESSION_AUTH_TOKEN_KEY` is set in `set_user_session` but never read
  anywhere else in the codebase.

**`set_user_session` inserts the same `auth_token_id` twice:**
- Once inside the `SessionUser` struct, and once independently under
  `SESSION_AUTH_TOKEN_KEY`. The second insert is unused and should be removed.

### `server.rs`

**`login` - user creation not transactional:**
- The `INSERT INTO users` and `INSERT INTO user_emails` are two separate
  queries with no transaction wrapping them. If the second insert fails, an
  orphaned user row without an email will exist in the database.

**`login` - confirmation token exposed in response:**
- Line 111: the confirmation token is returned in the response message as
  `"For testing, your confirmation token is: {}"`. This is a significant
  security issue and must be removed before production. The token should only
  ever travel via email.

**`login` - email validation is too weak:**
- `!email.contains('@')` is not email validation. An input like `"@"` passes.
  Use a minimal regex or the `email_address` crate. Validation here is at the
  boundary so it matters.

**`confirm_login` - multiple unguarded queries without a transaction:**
- Three separate mutations happen sequentially: insert auth token, clear login
  confirmation, set session. If any intermediate step fails the database is
  left in a partially updated state. These should be wrapped in a transaction.

**`confirm_login` - `SELECT *` on `users` and `user_emails`:**
- Both queries use `SELECT *` / `query_as!` with wildcard, bypassing compile-
  time column verification.

**`get_current_user` - database hit on every request:**
- Every page load or server function call invokes `get_current_user` which hits
  the database to validate the auth token. This is a query on every request.
  Consider caching the validation result in the session itself with a short
  TTL, or only re-validating periodically.

**`confirm_login` and `get_current_user` - `pub use server::*` in `mod.rs`:**
- All server function names (`Login`, `ConfirmLogin`, `GetCurrentUser`,
  `Logout`) are re-exported via `pub use server::*`. Leptos server function
  names must be globally unique across the crate. The wildcard re-export makes
  it harder to audit for name collisions.

---

## `src/game/`

### `client.rs`

**`reqwest::Client::new()` on every request:**
- A new `Client` is created for every call to `request()`. `reqwest::Client`
  holds a connection pool internally and is designed to be created once and
  reused. Creating it per-request discards the pool on every call. The client
  should be created once (in `AppState` or as a `lazy_static`/`OnceLock`) and
  shared.

**`render`, `pub_render`, `player_render` not used from `server.rs`:**
- These functions exist in `client.rs` but `server.rs` only calls `request()`
  directly. `server_fns.rs` uses `client::render()`. The two call-sites use the
  client inconsistently.

**Contract test is solid:**
- `test_game_client_contract` spins up a real mock HTTP server and verifies
  serialization round-trip. Good coverage of the most failure-prone boundary.
  The test covers `New` but not `Play`, `Status`, or `PlayerCounts`.

### `server.rs` (Axum handlers)

**`user_id = Uuid::nil()` in `create_game` and `play_command`:**
- Both handlers have `// TODO: get from session` with `Uuid::nil()` as the
  placeholder. These endpoints are currently unauthenticated and will associate
  all games with a nil UUID. These are non-functional as shipped and must be
  completed before cutover.

**Authorization not enforced on `get_game`:**
- `GET /api/game/{id}` returns full `GameExtended` including all player data to
  any caller without authentication. This exposes game state to unauthenticated
  requests.

**`play_command` - turn enforcement not checked:**
- The handler verifies the user is a player but does not verify it is their
  turn (`game_player.is_turn`). A player can submit commands on any turn. The
  game service will reject invalid moves if it checks turns internally, but
  this should also be enforced at the API layer.

**Duplicate game logic between `server.rs` and `server_fns.rs`:**
- `play_command` in `server.rs` and `submit_command` in `server_fns.rs`
  implement essentially the same operation: fetch game, find player, call game
  service, update DB, broadcast. This is a significant duplication. The server
  function should delegate to the Axum handler logic, or both should share a
  common service function.

**Error responses leak internal details:**
- `format!("Game service error: {}", e)` and similar are returned directly in
  HTTP response bodies. In production this exposes internal error messages to
  clients. Errors should be logged server-side and a generic message returned.

### `server_fns.rs` (Leptos server functions)

**`#[cfg(feature = "ssr")] / #[cfg(not(feature = "ssr"))] unreachable!()` pattern:**
- The entire function body is wrapped in `#[cfg]` blocks ending with
  `unreachable!()`. This is a known workaround for Leptos server functions
  that need SSR-only imports. It works but is verbose. Leptos 0.7+ has cleaner
  patterns for this - worth revisiting.

**`get_active_games` calls `get_current_user()` which issues a DB query:**
- `get_current_user` validates the auth token against the database on every
  call. `get_active_games` is called on every page load of the game list. This
  means every game list render is at minimum 2 DB queries before fetching any
  game data (auth validation + game IDs).

**`get_game_details` - no authorization check:**
- Any authenticated user can fetch the details of any game by ID, including
  player-specific renders. A player should only receive their own player render.
  The `player` variable correctly selects the current user's player, but there
  is no check preventing a non-player from calling this and receiving the public
  render (which is fine) vs their own player render (which is also fine for
  non-players since they get `None`). This is acceptable but worth documenting.

**`PlayerViewData.points` hardcoded to `0.0`:**
- Line 124: `points: 0.0, // TODO: add points to db`. Points are not persisted
  (as noted in the `db.rs` review). This is a known gap.

**`#[cfg(feature = "ssr")]` inline imports:**
- Each server function re-imports `PgPool`, `get_current_user`, `client`, etc.
  at the top of its `#[cfg(feature = "ssr")]` block. These could be hoisted to
  the module level under a single `#[cfg(feature = "ssr")]` use block for
  clarity.

---

## `src/app.rs`

### Observations

**`LoginPage` - email submit does not call the `login` server function:**
- `on_email_submit` sets local state (`show_code_input = true`) but never
  calls the `Login` server function. No email is sent and no confirmation token
  is generated from the UI. The server function exists and works, but the form
  is not wired to it. This is a significant functional gap.

**`LoginPage` - `on_code_submit` is a no-op:**
- `on_code_submit` calls `ev.prevent_default()` and nothing else. The
  `ConfirmLogin` server function is never called from the UI. Login via the
  web UI is currently completely non-functional end-to-end.

**`LoginPage` - code input type is `tel`:**
- The confirmation token is a UUID string (`Uuid::new_v4().to_string()`), not
  a numeric code. Using `type="tel"` with `pattern="[0-9]*"` will reject UUID
  input on mobile devices. The input type should match the token format, or the
  token format should change to a short numeric code.

**Google Fonts loaded from external CDN:**
- `shell()` loads Source Code Pro from `fonts.googleapis.com`. This is a
  third-party request on every page load. For a lo-fi project with a privacy
  focus, self-hosting the font is preferable. It also introduces a dependency
  on Google's CDN for availability.

**`DashboardPage` and `GamesPage` are stubs:**
- Both pages render placeholder text. Neither fetches or displays real data.
  `DashboardPage` says "Use the sidebar to navigate" which is reasonable, but
  `GamesPage` has no game listing or creation UI at all.

**`GamePage` - `game_id` parsing silently falls back to error:**
- If the URL parameter is not a valid UUID, `game_id()` returns `None` and the
  resource returns `Err(ServerFnError::new("Invalid Game ID"))`. This is
  handled, but the error is displayed inside a `MainLayout` with no further
  action or redirect. Fine for now.

**`WebSocketTrigger` context dependency:**
- `GamePage` calls `expect_context::<WebSocketTrigger>()` which panics if not
  provided. It is provided in `App`, so any component rendered under `App`
  is safe. This is correct.

**Resource reactivity is correct:**
- `game_data` takes `(game_id(), trigger.last_update.get())` as its key. When
  a WebSocket update arrives, `last_update` increments, the resource refetches.
  This is the right pattern.

---

## `src/components/`

### `layout.rs`

**`SidebarMenu` - `logout_action` result is not handled:**
- `ServerAction::<Logout>::new()` is created and dispatched on click, but the
  result (success/failure) is never observed. On logout, there is no redirect
  or UI state change. The user remains on the current page with no feedback
  that they have been logged out.

**`SidebarMenu` - always renders regardless of auth state:**
- The sidebar renders the active games list and logout button unconditionally.
  Unauthenticated users will see an error from `get_active_games` ("Not
  authenticated") rendered as "Error loading games". The sidebar should
  conditionally render based on auth state.

**Menu buttons are `input type="button"` placeholders:**
- "Menu" and "Sub menu" buttons in the header have no event handlers. These
  are unimplemented UI chrome.

**`MainLayout` - `is_my_turn`, `has_sub_menu`, `has_next_game` props:**
- `has_sub_menu` and `has_next_game` control whether placeholder buttons render.
  Since those buttons do nothing, these props have no functional effect beyond
  rendering inert HTML. Fine for now but worth noting as incomplete.

### `game.rs`

**`GameBoard` - `inner_html`:**
- Setting `inner_html` directly from server-generated markup is correct here
  since the HTML is produced by `brdgme_markup::html()` from controlled markup.
  Not a user-controlled XSS risk.

**`GameLogs` - stub:**
- Renders `"Log entries would go here"`. No actual log data is fetched or
  displayed. The `GameLog` model and `game_logs` table exist; this just is not
  connected yet. Notable gap for gameplay usability.

**`GameMeta` - "Concede" link is not wired:**
- `<a>"Concede"</a>` has no `href` or event handler. Unimplemented.

**`GameCommandInput` - command cleared before confirming success:**
- `set_command.set(String::new())` is called immediately after dispatching the
  action, before the server response is received. If the command fails, the
  user loses their input with no feedback. The clear should happen on success.

**`GameCommandInput` - `submit_action` result not observed:**
- The `ServerAction` result is never read. Errors from `SubmitCommand` are
  silently discarded. The user receives no feedback on command failure.

**`GameCommandInput` - suggestion logic is minimal:**
- The parser is called on every keystroke (reactive signal). If `spec.parse`
  succeeds with empty `remaining`, suggestions show `"<enter>"`. On parse
  error, the `expected` tokens are shown. This is a reasonable starting point
  but the UX is very basic - no autocomplete, no visual differentiation of
  suggestion types.

**`command_spec` is an `Option` moved into a closure:**
- `command_spec: Option<brdgme_game::command::Spec>` is passed as a prop.
  The `suggestions` closure captures it by reference (`if let Some(ref spec)`).
  Since `Spec` is behind an `Option` this is fine in terms of ownership, but
  `Spec` may be large (it encodes the full command grammar). Cloning it per
  render call in the closure could be expensive for complex games.

---

## `src/models/`

### Observations

**`models/mod.rs` - wildcard re-exports:**
- `pub use user::*`, `pub use game::*`, etc. re-export everything from all
  model modules. This pollutes the `models` namespace and makes it hard to see
  where a type comes from. The `New*` insert structs and read models share the
  same namespace.

**`User` derives `Serialize` + `Deserialize`:**
- `User` includes `login_confirmation` (the raw token) and
  `login_confirmation_at`. If a `User` struct is ever accidentally serialized
  into a response, the confirmation token is exposed. Consider a dedicated
  public-facing type that excludes sensitive fields (there is `PublicUser` but
  it still includes `created_at` and `updated_at` which are rarely needed by
  clients).

**`NaiveDateTime` throughout:**
- All timestamps use `chrono::NaiveDateTime` (no timezone). PostgreSQL stores
  `TIMESTAMP WITH TIME ZONE`. SQLx will map `timestamptz` to
  `chrono::DateTime<Utc>` unless explicitly mapped to `NaiveDateTime`. Using
  `NaiveDateTime` here means timezone information is discarded. This should be
  `DateTime<Utc>` throughout for correctness.

**`PublicGameType = GameType` type alias:**
- `pub type PublicGameType = GameType;` in `game.rs`. This adds nothing - the
  alias is unused in the codebase. The `uri` field of `GameVersion` is stripped
  in `PublicGameVersion` (correct, internal routing URLs should not be exposed),
  but `GameType` has no sensitive fields so the alias is unnecessary.

**`New*` structs:**
- Several `New*` structs (`NewGameType`, `NewGameVersion`, `NewGame`,
  `NewGamePlayer`, `NewChat`, `NewChatMessage`, `NewChatUser`, `NewFriend`)
  exist in the models but are not used anywhere in `db.rs`. The database
  queries in `db.rs` construct inline values rather than using these structs.
  These are either dead code or were intended for an ORM-style API that was not
  implemented. Should be removed unless there is a plan to use them.

**`chat.rs` and `friends.rs` - fully unused:**
- No code in `db.rs`, `server.rs`, or `server_fns.rs` references `Chat`,
  `ChatUser`, `ChatMessage`, `Friend`, or their `New*` variants. These are
  dead code at the application layer. The tables exist in the database but
  the feature is not implemented.

---

## Summary: Blockers Before Cutover

The following issues are functional blockers or security issues that must be
resolved before the `leptos` branch can replace production:

1. **Authentication not wired in Axum handlers** (`server.rs`): `create_game`
   and `play_command` use `Uuid::nil()` as the user ID. All game creation and
   command submission is effectively anonymous.

2. **Login UI not connected to server functions** (`app.rs`): `on_email_submit`
   and `on_code_submit` do not call the `Login` or `ConfirmLogin` server
   functions. Web login is non-functional end-to-end.

3. **Confirmation token exposed in login response** (`auth/server.rs`): The
   token is returned in the JSON response body for "testing." This must be
   removed before production.

4. **Session store is in-memory** (`auth/session.rs`): Sessions are lost on
   pod restart and do not work across replicas. Must be replaced with
   `tower-sessions-sqlx-store` or equivalent.

5. **`with_secure(false)` on session cookies** (`auth/session.rs`): Must be
   driven by environment before production deployment.

6. **No graceful shutdown** (`main.rs`): Pods receive SIGTERM on Kubernetes
   rolling updates; active connections will be dropped hard.

7. **Turn enforcement not checked** (`game/server.rs`): Players can submit
   commands when it is not their turn.

8. **`unauthenticated GET /api/game/{id}`** (`game/server.rs`): Returns full
   game data to any caller.

## Summary: Known Gaps (Non-blocking)

9. **Game logs not displayed** (`components/game.rs`): `GameLogs` is a stub.

10. **Points not persisted** (`db.rs`, `server_fns.rs`): `_points` is unused
    in `update_game_command_success`.

11. **`reqwest::Client` created per-request** (`game/client.rs`): Connection
    pool is discarded on every game service call.

12. **N+1 query in `find_active_games_for_user`** (`db.rs`): Should be a
    single joined query.

13. **Duplicate game command logic** (`game/server.rs` vs `game/server_fns.rs`):
    The same operation is implemented twice.

14. **WebSocket no reconnection** (`websocket_client.rs`): Client does not
    reconnect on drop.

15. **`NaiveDateTime` should be `DateTime<Utc>`** (`models/`): Timezone
    information is discarded on all timestamps.

16. **Dead code**: Unused `New*` model structs, `chat.rs`, `friends.rs`,
    `PublicGameType` alias, `SESSION_AUTH_TOKEN_KEY`, `db::AppState`.

17. **`DashboardPage` and `GamesPage` are stubs**: No game listing or creation
    UI exists yet.

18. **Logout has no redirect or UI feedback** (`components/layout.rs`).

19. **`GameCommandInput` clears input before server confirms success.**

20. **`submit_action` result not observed** - command errors are silently
    discarded.

---

## Command Parser: Autocomplete Parity

The old `web` frontend had its own TypeScript command parser (`web/src/command.ts`)
that implemented partial/prefix matching for autocomplete suggestions. The Rust
parser in `rust/lib/game/src/command/parser/mod.rs` is the canonical backend
parser. Both parsers implement the same `CommandSpec` grammar, but they handle
incomplete input differently.

### How each parser generates suggestions

**TypeScript (old):** Three-state match result (`MATCH_FULL`, `MATCH_PARTIAL`,
`MATCH_ERROR`). Builds a full parse result tree from the entire input. A
separate `suggestions(result, at)` function traverses the tree at the cursor
position and returns `{ value, offset, length }` suggestion objects. Prefix
checking is done per-token: `commonPrefix(input, token)` must be non-zero for
a suggestion to be generated.

**Rust (new):** Binary `Result<Output, GameError>`. On failure, `GameError::Parse
{ expected: Vec<String>, offset, .. }` carries what was expected next. The
`expected()` method on each parser variant also provides this. The current
`GameCommandInput` uses the `expected` list from parse errors as suggestion
strings.

### What works correctly already

`Enum::partial` and `Player` (which delegates to `Enum::partial`) handle prefix
matching correctly. `Enum::partial(["sackson", "hilton"]).parse("sa")` returns
`Ok { value: "sackson" }`. For games whose next token is an enum value (the
most common case), suggestions are correct.

### The `Token` false-positive problem (correctness gap)

`Token::parse` fails whenever `input.len() < token.len()`, regardless of whether
the input is a prefix of the token:

```rust
if input.len() < self.token.len()
    || UniCase::new(&input[..t_len]) != UniCase::new(&self.token) {
    return Err(GameError::Parse { expected: vec![self.token.clone()], .. });
}
```

Both `Token("buy").parse("b")` and `Token("sell").parse("b")` return
`Err(expected: ["buy"])` and `Err(expected: ["sell"])` respectively. A user
typing `"b"` gets ALL alternatives suggested by `OneOf`, not just the ones
whose tokens start with `"b"`.

TypeScript `parseToken("b", 0, "sell")` computed `commonPrefix("b", "sell") = ""`
(length 0) → `MATCH_ERROR` with no suggestion. `parseToken("b", 0, "buy")`
computed `commonPrefix("b", "buy") = "b"` (length 1) → `MATCH_PARTIAL { value: "buy" }`.
Only `"buy"` was suggested.

For a game with 15 commands, every keystroke shows all 15 suggestions rather than
the filtered subset. This is the primary usability regression versus the old
frontend.

### Other differences (acceptable)

**Cursor position:** TypeScript was cursor-position-aware; suggestions were shown
for the token under the cursor. Rust suggests for the rightmost parse failure
point. Mid-command cursor editing shows suggestions for the end of input.
Acceptable regression for async turn-based play.

**`Int` suggestions:** TypeScript generated example values `["1", "2", "3", "4",
"5"]` as clickable links. Rust `Int::expected()` returns a description string like
`"number 1 or higher"`. A minor UX difference.

**Clickable completions:** TypeScript suggestions carried `{ offset, length }`
for in-place replacement. Rust `expected: Vec<String>` has no position info.
Implementing click-to-complete would require tracking consumed length separately.

### Required change: add `suggest` to `CommandSpec`

**Do not change `Token::parse`.** Backend game logic relies on the current
behavior. Instead, add a separate `suggest(input: &str, names: &[String]) ->
Vec<String>` method to `CommandSpec` in `rust/lib/game` that is called only from
the frontend suggestion UI.

This method diverges from `parse` in the following ways:

- `Token`: include the token in suggestions only when `input` is a non-empty
  prefix of the token (i.e. when `shared_prefix(input, token) == input.len()`).
- `Chain`: consume links that succeed fully, then call `suggest` on the first
  failing link with the remaining input.
- `OneOf`: try all branches, collect suggestions from branches with the maximum
  consumed offset (same "best effort" logic as parse errors, but with prefix
  filtering applied).
- `Enum`/`Player`: existing prefix matching is already correct.
- `Int`: return example values from `min` to `min+4` (matching TypeScript behavior)
  or the existing description string.
- `Opt`/`Many`/`Doc`: delegate to inner spec's `suggest`.

This is a pure addition to `rust/lib/game` - no existing code changes. Since
`brdgme_game` is already compiled to WASM and used in the frontend, `suggest`
is immediately available to `GameCommandInput` once added.

### Impact on blocker/gap list

Add to known gaps (non-blocking):

38. **`Token` produces false-positive autocomplete suggestions** (`rust/lib/game`):
    `CommandSpec::Token` includes the token in `expected` whenever
    `input.len() < token.len()`, regardless of whether the input is a prefix
    of that token. In `OneOf` contexts (all game command roots), this shows all
    alternatives as suggestions even when the user's input rules them out. Fix:
    add `CommandSpec::suggest(input, names) -> Vec<String>` with prefix-aware
    token filtering. Do not change `parse()`.

---

## Feature Parity Check: rust/api + web + websocket vs rust/web

Full read of `rust/api`, `web` (React frontend), and `websocket` (Node.js)
compared against `rust/web`. Findings below are additional to the per-file
review above.

---

### API Endpoints

#### Old `rust/api` endpoints

| Endpoint | New equivalent | Status |
|---|---|---|
| `POST /auth` | `Login` server fn | Partial - confirmation token exposed in response |
| `POST /auth/confirm` | `ConfirmLogin` server fn | Partial - auth model changed (see below) |
| `GET /init` | None | Missing |
| `GET /game/{id}` | `GET /api/game/{id}` + `GetGameDetails` server fn | Partial - unauthenticated Axum handler |
| `POST /game/` | `POST /api/game/new` | Partial - unauthenticated, missing logs |
| `POST /game/{id}/command` | `POST /api/game/{id}/command` + `SubmitCommand` server fn | Partial - unauthenticated Axum handler |
| `POST /game/{id}/undo` | None | **Missing** |
| `POST /game/{id}/mark_read` | None | **Missing** |
| `POST /game/{id}/concede` | None | **Missing** |
| `POST /game/{id}/restart` | None | **Missing** |

#### `GET /init` - no equivalent

The old API bootstrapped the client with a single call returning:
- All public, non-deprecated `game_version_types` (for the new-game form).
- The authenticated user's active games.
- The current user object.

There is no equivalent endpoint or server function in `rust/web`. Until this
data is available, the new-game form cannot be populated with available game
types, and the user profile is not accessible.

#### Public game versions list - no equivalent

`query::public_game_versions` in the old API returned all available game types
and versions. No server function or endpoint provides this in `rust/web`. The
new-game UI cannot be built without it.

---

### `GamePlayer` Model Gaps

The new `GamePlayer` struct (`models/game.rs`) is missing fields that exist in
the database schema and were used by the old system:

| Field | Old type | Purpose | Impact of absence |
|---|---|---|---|
| `last_turn_at` | `NaiveDateTime` | Time of player's last turn | "Recent logs" feature broken |
| `is_eliminated` | `bool` | Player elimination status | Elimination state not tracked |
| `is_read` | `bool` | Whether player has read latest updates | `mark_read` impossible without it |
| `points` | `Option<f32>` | Game-specific score | Points not stored per-player |
| `undo_game_state` | `Option<String>` | Saved state for undo | Undo entirely impossible without it |
| `rating_change` | `Option<i32>` | ELO change after game end | Post-game rating delta unavailable |

These fields exist in the database (the migration baseline was taken from the
old schema). The model must be updated to include them before undo, mark_read,
and related features can be implemented.

---

### `update_game_command_success` Missing Updates

Old `query::update_game_command_success` wrote:
- `game_state`, `is_finished` on `games` - present in new
- `is_turn`, `is_turn_at`, `last_turn_at` on `game_players` - new only writes `is_turn`
- `is_eliminated` on `game_players` - new omits this
- `undo_game_state` per player (set or cleared based on `can_undo`) - new omits this
- `points` per player - new has `_points` parameter (intentionally unused)
- `place` per player - present in new
- `is_read = false` reset (implicitly via old game update logic) - not reset in new

The `can_undo` return value from the game service is captured but discarded
(`_can_undo`) in both `server.rs` and `server_fns.rs`. The undo_game_state
column is never written.

---

### `find_game_extended` Migration Risk

In `db.rs`, `find_game_extended` errors if a player does not have a
`game_type_users` record:

```rust
return Err(anyhow::anyhow!("Game type user missing for player {}", p.u_id));
```

The old system handled a missing `game_type_users` row with `LEFT JOIN` and
used a default struct. Any game in the existing database where a player lacks a
`game_type_users` record will fail to load in the new system. This is a data
migration risk that must be verified and resolved before cutover.

---

### `validate_session_token` Does Not Check Token Expiry

Old `authenticate` filtered tokens by `created_at > NOW() - 30 days`:

```rust
.filter(user_auth_tokens::created_at.gt(Utc::now().naive_utc() - *TOKEN_EXPIRY))
```

New `validate_session_token` only checks existence:

```rust
"SELECT id FROM user_auth_tokens WHERE id = $1"
```

No expiry filter. Auth tokens issued by the new system are permanent (deleted
only on logout). The session layer provides a 24-hour inactivity timeout, but
once a session is rebuilt from a persistent store (after the MemoryStore is
replaced), a token from years ago would still validate. Auth tokens should have
a time-bounded validity check.

---

### Auth Model Change: Bearer Token - Session Cookie

Old system used `Authorization: Bearer <uuid>` on every request. Auth tokens
were stored in `localStorage` in the browser and sent as HTTP headers. This
enabled non-browser clients (email integration, bots) to authenticate.

New system uses session cookies (`tower-sessions`, `MemoryStore`). No bearer
token support exists. Implications:

- Email-based play (play-by-email POST to API) cannot authenticate with the
  new system without additional work.
- Any scripts or tooling that used Bearer tokens will stop working.
- This is the intended direction (session cookies are correct for a browser
  app), but it is a breaking API change.

---

### Login Confirmation: Format and Expiry Differences

| Property | Old | New |
|---|---|---|
| Code format | 6-digit numeric string (e.g. `"342819"`) | UUID v4 string |
| Expiry window | 30 minutes | 1 hour |
| Code cleared on confirm | No (reusable within window) | Yes (one-time use) |
| Old code reuse | Same 6-digit code re-sent if still valid | UUID regenerated each time |

The new UUID format is more secure (unguessable), and single-use is better.
However, the UUID is returned in the response body for "testing" which negates
the security benefit entirely (blocker #3 in existing list).

The old confirm endpoint required both email + code. The new confirm endpoint
requires only the token (UUID). The new flow is: request by email, receive UUID
in email (when email sending is implemented), confirm with UUID only. This is
simpler.

---

### WebSocket: Subscription Model vs Broadcast-All

#### Old model (`websocket` Node.js + Redis)

- Client explicitly subscribes to `game.{game_id}` and `user.{auth_token_id}`
  channels by sending typed messages to the WebSocket server.
- Server only delivers messages matching the client's active subscriptions.
- Messages contained **full personalized game state** (`ShowResponse` with
  rendered HTML, logs filtered per player, and `command_spec`).
- Redis pub/sub for cross-server fan-out.
- Client reconnect loop in `ws.ts` (5-second retry on disconnect).
- Node.js server sent ping every 30 seconds to keep connections alive.

#### New model (`rust/web`)

- Client is a passive receiver - no subscription messages.
- Server broadcasts ALL updates to ALL connected clients.
- Messages carry only `game_id`; client re-fetches full state via server fn.
- `tokio::sync::broadcast` (in-process only, no multi-replica support).
- No client reconnection logic.
- No heartbeat/ping.

The broadcast-all model is simpler and acceptable at current scale, with the
planned NATS migration providing cross-replica fan-out. The extra re-fetch
round-trip is acceptable. The absence of reconnection and heartbeat is a known
gap (items 14 and below in the known-gaps list).

#### `GameRestarted` WebSocket message is non-functional

Old system sent `GameRestarted { game_id, restarted_game_id }` which the old
React client used to navigate to the new game's URL. The new `WebSocketMessage`
has a `GameRestarted` variant but `websocket_client.rs` treats it identically
to `GameUpdate` (increments the trigger counter). No navigation occurs. Once
the `restart` endpoint is implemented, the client must also handle
`GameRestarted` by navigating to the new game URL.

---

### Frontend Parity: Missing Pages and Features

#### New-game creation UI - missing

Old `web/src/components/game/new.tsx` provided:
- Dropdown to select game type.
- Add/remove opponent email inputs.
- Submit to `POST /api/game/`.
- Redirect to new game on success.

The new `GamesPage` is a stub (`"Browse active games and create new ones."`)
with none of this implemented.

#### Game logs - stub

Old frontend rendered all game logs with timestamps and time-grouping logic
(logs grouped by 10-minute windows), filtered to "recent logs since last turn"
(using `last_turn_at`). The new `GameLogs` component renders a static
placeholder string.

#### Undo and restart actions in game meta - missing

Old `game/show.tsx` rendered:
- "Concede" link (visible if player is in game and game has 2 players or
  fewer) - new `GameMeta` has a static `<a>"Concede"</a>` with no handler.
- "Undo" link (visible if `game_player.can_undo`) - entirely absent in new.
- "Restart" link (visible if game finished and not already restarted) -
  entirely absent in new.
- "Go to restarted game" link (visible if `restarted_game_id` is set) -
  entirely absent in new.

#### "Whose turn" indicator

Old frontend showed "Waiting on [player name]" or "Your turn!" with colored
player names. New `GamePage` shows a generic `"Waiting on opponents..."` string
with no player name or color.

#### Player rating change after game

Old `renderMetaPlayer` showed `rating_change` with an up/down arrow after a
game finished. New `PlayerInfo` only shows current rating. `rating_change` is
not in the new `GamePlayer` model.

#### Mark-read on page load

Old `game/show.tsx` called `onMarkRead(gameId)` on `componentDidMount` and
when the game ID changed. The new `GamePage` has no equivalent call. The
`is_read` field cannot be set without the `mark_read` endpoint, and the field
is not even in the new `GamePlayer` model.

#### Command input disabled when not your turn

Old frontend disabled the command input if `game_player.is_turn === false` or
if the game was finished. New `GamePage` conditionally renders
`GameCommandInput` with `<Show when=move || is_my_turn>`, which hides the input
entirely rather than disabling it. The visible "Waiting on opponents" text is
less informative than the old "Waiting on [player name]" display.

#### Suggestion clicking - missing

Old frontend had clickable suggestion links that appended the suggestion text
to the command input, advancing the cursor position. New frontend shows
suggestion strings as static `<div>` elements with no click handler.

---

### Chat

Old `ShowResponse` included `chat: Option<PublicChatExtended>` with full
message history. This was part of the game view. The new `GameExtended` struct
explicitly omits chat with a comment: `// Chat is omitted for now`. There is
no plan to implement chat in the current migration scope and this is acceptable.

---

## Updated Summary: Blockers Before Cutover

Items 1-8 remain from the existing list. Additional blockers discovered in this
parity review:

21. **`GamePlayer` model missing critical fields** (`models/game.rs`):
    `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
    `rating_change` are all absent. These fields exist in the database. The
    struct must be expanded before undo, mark_read, and points tracking can
    work.

22. **`update_game_command_success` does not update all required fields**
    (`db.rs`): `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`
    are not written on each command. Points are explicitly suppressed. The
    function must be brought to parity with the old implementation.

23. **`find_game_extended` will error on games where a player lacks a
    `game_type_users` record** (`db.rs`): This is a migration risk. Any such
    game from the old system would become unloadable after cutover. The LEFT
    JOIN must be handled gracefully (use a default rating rather than erroring).

24. **`validate_session_token` does not enforce token expiry** (`auth/session.rs`):
    Auth tokens are treated as permanent. An expiry check matching the old
    30-day window should be added.

## Updated Summary: Known Gaps (Non-Blocking)

Items 9-20 remain from the existing list. Additional gaps from this review:

25. **`POST /game/{id}/undo` not implemented.** Requires `undo_game_state` in
    model and db layer.

26. **`POST /game/{id}/mark_read` not implemented.** Requires `is_read` in
    model.

27. **`POST /game/{id}/concede` not implemented.** UI placeholder exists but
    has no handler.

28. **`POST /game/{id}/restart` not implemented.** `GameRestarted` WebSocket
    variant also needs client-side navigation.

29. **No public game version listing.** Cannot build the new-game form without
    a server function or endpoint that returns available game types and versions.

30. **New-game creation UI missing.** `GamesPage` is a stub. No form, no game
    type selector, no opponent email input.

31. **Game log rendering missing.** `GameLogs` is a static placeholder. Actual
    log display requires fetching logs, rendering markup to HTML, grouping by
    time, and filtering by `last_turn_at` for "recent logs".

32. **Undo/restart/concede action links not wired in `GameMeta`.**
    "Concede" anchor has no handler. Undo and restart links are absent
    entirely.

33. **"Whose turn" display is generic.** Shows "Waiting on opponents..." instead
    of the specific player name and color.

34. **Clickable command suggestions not implemented.** Suggestions display as
    static text with no click-to-complete behaviour.

35. **Mark-read not called on game page load.** `is_read` is not reset when
    a player views a game.

36. **`GameRestarted` WebSocket message does not navigate client to new game.**

37. **`finished_at` may not be set when `is_finished = true`.** The new
    `update_game_command_success` sets `is_finished` but does not set
    `finished_at`. Verify whether the schema has a trigger or default that
    handles this; if not, `finished_at` is always NULL.
