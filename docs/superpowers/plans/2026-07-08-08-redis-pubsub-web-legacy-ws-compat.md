# 08: Redis pub/sub + web-legacy WS compatibility - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/08-redis-pubsub-web-legacy-ws-compat.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete

**Goal:** Replace the in-process `tokio::sync::broadcast` WebSocket fan-out
with Redis pub/sub, and publish legacy-compatible payloads so `web-legacy`
React clients receive correct real-time updates without re-engineering.

**Spec:** `docs/superpowers/specs/2026-07-08-08-redis-pubsub-web-legacy-ws-compat-design.md`

## Redis pub/sub

- [x] Add `redis` to `rust/web/Cargo.toml` (`tokio-comp` feature).
- [x] Replace `GameBroadcaster`: publish to `game.{id}` via Redis `PUBLISH`.
- [x] Subscribe each `/ws` handler to `game.*` via Redis `PSUBSCRIBE` and
      forward raw payloads to the connected client.
- [x] Remove `tokio::sync::broadcast` from `AppState` and `GameBroadcaster`.
- [x] Read `REDIS_URL` env var (matches legacy config, default `redis://redis`).

## Web-legacy WS compatibility (fat payload publishing)

- [x] Added legacy serialization structs to `websocket.rs` matching the
      exact JSON shape of the old `rust/api` `ShowResponse`.
- [x] `broadcast_game_update` publishes to `game.{id}` (public, no
      `command_spec`) and per-player to `user.{auth_token_id}` (private,
      with player-specific `html` and `command_spec`).
- [x] Per-player logs use `db::get_game_logs` with `game_log_targets` join
      (same filter as display path - no info leak, correct private logs).

## Leptos-specific WS channel (eliminates re-fetch)

- [x] `WebSocketMessage` enum has only `BrdgmeUpdate` variant.
- [x] `/ws` handler extracts session, subscribes to `ws.{user_id}` if logged in.
- [x] `GamePage` resource keyed on `game_id` only; WS signal takes precedence.
- [x] `GameLogs`/`RecentGameLogs` prefer WS logs when `game_id` matches.
- [x] Restart publishes `BrdgmeUpdate` for BOTH old game (with
      `restarted_game_id` set) and new game - no special `GameRestarted` message
      needed (the game record already carries `restarted_game_id`).
