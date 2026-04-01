# Phase 9 Implementation Status

## What is complete

### DB migration + model changes
- `rust/web/migrations/003_game_bots.sql` - created, applied to dev DB.
- `GamePlayer.user_id: Option<Uuid>` in models/game.rs.
- `GameBot` struct in models/game.rs.
- `GamePlayerExtended`: `user: Option<User>`, `game_bot: Option<GameBot>`, `name()` helper.
- `find_game_extended` and `find_active_games_for_user`: LEFT JOIN users + LEFT JOIN game_bots.
- SQLx cache regenerated (`rust/web/.sqlx/`). Compile verified clean.

### Game contract - Rules endpoint
- `rust/lib/cmd/src/api.rs`: `Rules` added to `Request` and `Response` enums.
- `rust/lib/cmd/src/requester/gamer.rs`: `handle_rules::<G>()` dispatch added.
- `rust/lib/game/src/game.rs`: `fn rules() -> String` on `Gamer` trait (default empty).
- All 4 Rust games: empty stub `fn rules() -> String { String::new() }`.
- Rules text deferred. When writing: `include_str!("../rules.md")` in each game,
  file at `rust/game/<name>/rules.md`.

### Callsite fixes (all files)
- `game/mod.rs`: `is_some_and`, `name()`.
- `game/server.rs`: all handlers updated.
- `game/server_fns.rs`: all handlers updated.
- `websocket.rs`: `build_markup_players`, `build_legacy_game_players` (synthetic LegacyUser
  for bots), `LegacyGamePlayer.user_id: Option<Uuid>`, per-player broadcast loop wrapped in
  `if let Some(ref user) = gpe.user`, `PlayerViewData.name` uses `p.name()`.

### execute_command refactor
- `execute_command` now takes `player_position: usize` (not `user_id`).
- `play_command` (server.rs) and `submit_command` server fn (server_fns.rs) do a lightweight
  `SELECT position FROM game_players WHERE game_id = $1 AND user_id = $2` before calling it.

### Internal command endpoint
- `POST /api/internal/game/{id}/command` in `server.rs`.
- Auth: `X-Internal-Key` header vs `INTERNAL_API_KEY` env var.
- Body: `{ "player_position": N, "command": "..." }`.
- Route registered in `api_routes()`.

### trigger_bot_turns helper
- `pub async fn trigger_bot_turns(http_client, ge)` in `game/mod.rs`.
- Reads `BOT_SERVICE_URL` env var - returns immediately if unset.
- For each bot player with `is_turn = true`: spawns background tokio task posting
  `{ "game_id": "...", "player_position": N, "difficulty": "..." }` to `BOT_SERVICE_URL/trigger`.
- Called from `execute_command` after broadcast (covers play/undo via server_fns).
- Called from `create_game` handler in `server.rs` after broadcast.

---

## What is incomplete / next steps (in order)

### 1. Add trigger_bot_turns to remaining server.rs handlers - IMMEDIATE NEXT

The session ended mid-edit in `server.rs`. These three handlers broadcast but do NOT yet
call `trigger_bot_turns`:

**`undo_game`** - find the block:
```rust
if let Ok(Some(updated_ge)) = db::find_game_extended(&pool, id).await {
    let all_logs = db::get_all_game_logs(&pool, id).await.unwrap_or_default();
    broadcaster.broadcast_game_update(&pool, &updated_ge, &all_logs, &public_render, &player_renders).await;
}

StatusCode::OK.into_response()
```
Add after the broadcast:
```rust
    super::trigger_bot_turns(&http_client, &updated_ge).await;
```

**`restart_game`** - the new game broadcast block:
```rust
if let Ok(Some(new_ge)) = db::find_game_extended(&pool, new_game.id).await {
    let all_logs = db::get_all_game_logs(&pool, new_game.id).await.unwrap_or_default();
    broadcaster.broadcast_game_update(&pool, &new_ge, &all_logs, &public_render, &player_renders).await;
}
```
Add after the broadcast:
```rust
    super::trigger_bot_turns(&http_client, &new_ge).await;
```

**`concede_game`** - the broadcast block:
```rust
Ok(Response::Status { public_render, player_renders, .. }) => {
    broadcaster.broadcast_game_update(&pool, &updated_ge, &all_logs, &public_render, &player_renders).await;
}
```
Add after the broadcast (inside the `Ok` arm):
```rust
super::trigger_bot_turns(&http_client, &updated_ge).await;
```

### 2. Add trigger_bot_turns to server_fns.rs handlers

In `concede_game` server fn: after the broadcast block (same pattern as server.rs).
In `restart_game` server fn: after the new game broadcast block.
In `create_new_game` server fn (if it exists): after broadcast.

### 3. New game creation with bot slots

- Add `BotSlot { name: String, difficulty: String }` struct to `db.rs`.
- Extend `CreateGameOpts` with `bot_slots: &[BotSlot]`.
- In `create_game_with_users` (db.rs): for each bot slot, INSERT into `game_bots`
  then INSERT into `game_players` with `game_bot_id` set and `user_id = NULL`.
- Extend `CreateGameRequest` in `server.rs` to accept optional `bot_slots`.
- Extend `create_new_game` server fn in `server_fns.rs` similarly.
- Update new game UI component to offer bot difficulty selection per opponent slot.

### 4. rust/bot crate

New workspace member. Minimal Axum server with `POST /trigger`.
- Reads: `DATABASE_URL`, `BOT_MODEL` (default `qwen3.5:4b`), `OLLAMA_URL`
  (default `http://localhost:11434`), `OLLAMA_API_KEY` (optional Bearer token),
  `MONOLITH_URL`, `INTERNAL_API_KEY`.
- On trigger: fetch game from DB, call game service `Status`, call `Rules`,
  fetch last 30 logs, assemble prompt, POST to `OLLAMA_URL/api/chat` with
  `{ "model": "...", "think": false, "stream": false, "messages": [...] }`.
  If `OLLAMA_API_KEY` set, add `Authorization: Bearer <key>` header.
  Parse response command, POST to monolith internal endpoint. Retry up to 5x
  on validation error (append rejected command + error to conversation).
- Add `rust/bot/` to workspace `rust/Cargo.toml`.
- Add Dockerfile target to `rust/Dockerfile` (reuse chef/planner pattern).

### 5. k8s manifests

- `k8s/base/bot/`: Knative Service for `brdgme/bot` image.
  Env: `OLLAMA_URL`, `OLLAMA_API_KEY` (Secret), `MONOLITH_URL`,
  `INTERNAL_API_KEY` (Secret), `DATABASE_URL` (Secret), `BOT_MODEL`.
- Add to `k8s/base/brdgme/kustomization.yaml`.

### 6. Tiltfile

Add `local_resource` for `rust/bot` in hybrid mode:
```python
local_resource(
    "bot",
    serve_cmd="cd rust/bot && SQLX_OFFLINE=true mirrord exec --target pod/postgres-0 --target-namespace brdgme -- cargo run",
    resource_deps=["postgres"],
)
```

### 7. Rules text (deferred)

After end-to-end working, write `rust/game/<name>/rules.md` for each game and
update each `fn rules()` stub to `include_str!("../rules.md").to_string()`.
Go games need separate handling in their codebase.

---

## Key decisions

- **No in-cluster Ollama** - external Ollama API only (local, Tailscale, or Ollama Cloud).
- **Model**: `qwen3.5:4b` default, `think: false` always, `BOT_MODEL` env var overrides.
- **API**: `/api/chat` endpoint (not `/api/generate`) - consistent across all backends.
- **execute_command takes position** - not user_id. User-facing endpoints do a lightweight
  position lookup first. Internal endpoint passes position from request body directly.
- **XOR CHECK constraint** on game_players enforces exactly one of user_id/game_bot_id.
- **Synthetic LegacyUser for bots** in websocket.rs: uses bot.id as user id, empty pref_colors.
- **Bot restart limitation**: bots not recreated on restart - only human opponent_ids collected.
- **Go games Rules**: not implemented; bot caller handles empty rules response gracefully.
