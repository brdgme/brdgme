# 8: Redis pub/sub + web-legacy WS compatibility

**Status:** Complete

**Goal:** Replace the in-process `tokio::sync::broadcast` WebSocket fan-out
with Redis pub/sub, and publish legacy-compatible payloads so `web-legacy`
React clients receive correct real-time updates without re-engineering.

### Redis pub/sub

- [x] Add `redis` to `rust/web/Cargo.toml` (`tokio-comp` feature).
- [x] Replace `GameBroadcaster`: publish to `game.{id}` via Redis `PUBLISH`.
- [x] Subscribe each `/ws` handler to `game.*` via Redis `PSUBSCRIBE` and
      forward raw payloads to the connected client.
- [x] Remove `tokio::sync::broadcast` from `AppState` and `GameBroadcaster`.
- [x] Read `REDIS_URL` env var (matches legacy config, default `redis://redis`).

### Web-legacy WS compatibility (fat payload publishing)

The legacy API publishes to:
- `game.{game_id}` - public ShowResponse, broadcast to all watching a game
- `user.{user_auth_token_id}` - private ShowResponse with `command_spec`

The `user.{token_id}` channel is non-negotiable for web-legacy compat. The
React reducer (`web/src/reducers/game.ts` line 26-28) explicitly skips public
channel updates when the user already has a private game view loaded
(`existing.game_player && !g.game_player` → return). Without private per-player
messages, React users on the game page would never see cross-system moves.

`rust/web` now publishes the same legacy-format JSON to both channels on every
game event. The legacy React reducer reads `game`, `game_type`, `game_version`,
`game_players`, `game_logs`, `html`, `game_player`, `command_spec` - all
populated correctly. Web-legacy clients receive full real-time updates from
either system during Phase 16 side-by-side operation.

- [x] Added legacy serialization structs to `websocket.rs` matching the
      exact JSON shape of the old `rust/api` `ShowResponse`.
- [x] `broadcast_game_update` publishes to `game.{id}` (public, no
      `command_spec`) and per-player to `user.{auth_token_id}` (private,
      with player-specific `html` and `command_spec`).
- [x] Per-player logs use `db::get_game_logs` with `game_log_targets` join
      (same filter as display path - no info leak, correct private logs).

### Leptos-specific WS channel (eliminates re-fetch)

`rust/web` also publishes `BrdgmeUpdate` (a `GameViewData` + `Vec<GameLogEntry>`
struct) to `ws.{user_id}` per player. The `/ws` handler is session-aware:
subscribes to both `game.*` and `ws.{user_id}`. The Leptos client handles
`BrdgmeUpdate` by setting a context signal directly - no server function
re-fetch needed for game state or logs. `active_games` sidebar still re-fetches
(cheap DB-only query) via `last_update` increment.

- [x] `WebSocketMessage` enum has only `BrdgmeUpdate` variant.
- [x] `/ws` handler extracts session, subscribes to `ws.{user_id}` if logged in.
- [x] `GamePage` resource keyed on `game_id` only; WS signal takes precedence.
- [x] `GameLogs`/`RecentGameLogs` prefer WS logs when `game_id` matches.
- [x] Restart publishes `BrdgmeUpdate` for BOTH old game (with
      `restarted_game_id` set) and new game - no special `GameRestarted` message
      needed (the game record already carries `restarted_game_id`).

