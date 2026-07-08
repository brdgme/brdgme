# 17: NATS Migration + WS simplification - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/17-nats-migration-ws-simplification.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete (2026-07-05)

**Spec:** `docs/superpowers/specs/2026-07-05-17-nats-migration-ws-simplification-design.md`

## Scope of change

In this phase, `broadcast_game_update` reduces to a single publish:
- Publish `{"game_id": "..."}` once to `game.{id}` (no per-player loop)
- Client re-fetches `get_game_details` + `get_game_logs` on receipt
- Remove: all `Legacy*` structs, per-player loop, auth token lookup,
  per-player `get_game_logs` calls, session extraction in `ws_handler`,
  `ws.{user_id}` channel, `BrdgmeGameUpdate`/`WebSocketMessage` enum
- `/ws` handler reverts to simple `PSUBSCRIBE game.*`, no session needed

## Infrastructure

**Note:** NATS cluster installation (k8s manifests, Tiltfile, `async-nats`
dependency) is pulled forward into Phase 13 NATS bot eventing. By the time this
phase runs, NATS is already in the cluster. The remaining infrastructure tasks
here are the WS-specific migration:

- [x] Replace Redis `PUBLISH`/`SUBSCRIBE` in `websocket.rs` with `async-nats`
      publish/subscribe. Subject naming: `game.{id}` only - the `ws.{user_id}`
      channel is deleted along with the fat-payload/session-auth path.
- [x] Simplify `broadcast_game_update` to skinny signal (see spec).
- [x] Replace the hand-rolled client in `websocket_client.rs` (gloo-net +
      manual 2s reconnect loop) with `leptos-use`'s `use_websocket`
      (built-in `ReconnectLimit` reconnection, typed codecs; added
      2026-07-03 final pass). The skinny-payload model - re-fetch on any
      message - is exactly its shape; deletes ~60 lines of bespoke
      connection management. Do this here, not earlier: the current fat
      `BrdgmeUpdate` handling would fight its codec model.
- [x] Remove the `redis` dependency from `rust/web/Cargo.toml`.
- [x] Remove Redis from `k8s/base/brdgme/kustomization.yaml` and the
      default Tiltfile port-forwards, but do **not** delete
      `k8s/base/redis/`: the legacy stack (dev `LEGACY=1` mode and the
      Phase 16 break-glass rollback overlay) still needs it. The manifests
      are deleted in the Phase 16 decommission.

**Note:** `GameBroadcaster::broadcast_game_update` calls `client.flush().await`
after `publish` (logging, not propagating, errors) - `async-nats` buffers
publishes internally and its background flush task can otherwise delay
delivery under load, which matters for WS update latency.

**Note:** JetStream is already enabled from Phase 13 (bot eventing). WS
fan-out deliberately stays on plain Core pub/sub subjects - ephemeral
at-most-once is correct here (clients re-fetch full state on reconnect); do
not route WS traffic through the `BOT` stream.
