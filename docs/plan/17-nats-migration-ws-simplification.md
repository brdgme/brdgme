# 17: NATS Migration + WS simplification

**Status:** Pending

**Goal:** Replace Redis pub/sub with NATS Core, drop the monolith's Redis
dependency, and simplify the WebSocket broadcast path.

**Resequenced 2026-07-04: pre-cutover.** With the Phase 16 hard-cutover
decision the legacy stack is never deployed to prod, so there are no legacy
WS clients to serve and the fat-payload compat system (Phase 8) has no
production consumer. This phase now runs **before** go-live (after Phase 13,
which installs NATS), so cutover happens on the final skinny-payload
architecture instead of the compat system. The Phase 8 legacy-compat code
(`Legacy*` structs, per-player loop, `ws.{user_id}` channel) is deleted
here. Caveat: `LEGACY=1` dev mode loses cross-system live updates once the
monolith stops publishing Redis fat payloads - acceptable; the legacy stack
still self-publishes to Redis for its own clients, which is also why the
break-glass rollback (Phase 16) stays fully functional.

**Delegation gap:** the target state is clear but the implementation is only
sketched. Before delegating, specify:
- **Server subscription architecture:** per-WebSocket-connection NATS
  subscription vs one shared subscription per replica with in-process fan-out
  to connections (filtering by game id). Decide, with connection-count and
  resource reasoning.
- **Client-side refactor plan:** several components currently depend on the
  fat payload (`GamePage`'s WS-takes-precedence view logic, `GameLogs`/
  `RecentGameLogs` WS log preference, the `RwSignal<Option<BrdgmeGameUpdate>>`
  context). A component-by-component change list is needed: what replaces the
  context signal, how re-fetches are triggered per component, and coalescing
  behaviour when several skinny signals arrive in quick succession.
- **Test updates:** which Phase 11 tests change (broadcaster swaps from Redis
  to NATS in CI) and what new assertions cover the skinny-signal path.
- **Sequencing:** resolved 2026-07-04 - pre-cutover, after Phase 13 (see
  above). The old "after Phase 16 decommission" precondition is void: the
  legacy stack never runs in prod alongside the monolith.

### WS payload strategy change (fat → skinny)

During Phases 8-15, fat payloads are justified because legacy compat already
requires per-player `get_game_logs` DB queries and auth token lookups - the
`BrdgmeUpdate` comes for free. Post-decommission that cost exists solely to
serve the fat payload. Logs also grow unboundedly with game length.

The per-player complexity (different board HTML, `command_spec`, private logs
per player) is the root cause of most of `broadcast_game_update`'s weight.
Skinny payloads eliminate the need for player-specific messages entirely -
player-specific data comes back through the authenticated `get_game_details`
re-fetch, which is the right place for it anyway.

In this phase, `broadcast_game_update` reduces to a single publish:
- Publish `{"game_id": "..."}` once to `game.{id}` (no per-player loop)
- Client re-fetches `get_game_details` + `get_game_logs` on receipt
- Remove: all `Legacy*` structs, per-player loop, auth token lookup,
  per-player `get_game_logs` calls, session extraction in `ws_handler`,
  `ws.{user_id}` channel, `BrdgmeGameUpdate`/`WebSocketMessage` enum
- `/ws` handler reverts to simple `PSUBSCRIBE game.*`, no session needed

### Infrastructure

**Note:** NATS cluster installation (k8s manifests, Tiltfile, `async-nats`
dependency) is pulled forward into Phase 13 NATS bot eventing. By the time this
phase runs, NATS is already in the cluster. The remaining infrastructure tasks
here are the WS-specific migration:

- [ ] Replace Redis `PUBLISH`/`SUBSCRIBE` in `websocket.rs` with `async-nats`
      publish/subscribe. Subject naming: `game.{id}` and `ws.{user_id}`.
- [ ] Simplify `broadcast_game_update` to skinny signal (see above).
- [ ] Replace the hand-rolled client in `websocket_client.rs` (gloo-net +
      manual 2s reconnect loop) with `leptos-use`'s `use_websocket`
      (built-in `ReconnectLimit` reconnection, typed codecs; added
      2026-07-03 final pass). The skinny-payload model - re-fetch on any
      message - is exactly its shape; deletes ~60 lines of bespoke
      connection management. Do this here, not earlier: the current fat
      `BrdgmeUpdate` handling would fight its codec model.
- [ ] Remove the `redis` dependency from `rust/web/Cargo.toml`.
- [ ] Remove Redis from `k8s/base/brdgme/kustomization.yaml` and the
      default Tiltfile port-forwards, but do **not** delete
      `k8s/base/redis/`: the legacy stack (dev `LEGACY=1` mode and the
      Phase 16 break-glass rollback overlay) still needs it. The manifests
      are deleted in the Phase 16 decommission.

**Note:** JetStream is already enabled from Phase 13 (bot eventing). WS
fan-out deliberately stays on plain Core pub/sub subjects - ephemeral
at-most-once is correct here (clients re-fetch full state on reconnect); do
not route WS traffic through the `BOT` stream.

