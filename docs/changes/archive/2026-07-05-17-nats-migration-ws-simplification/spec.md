# 17: NATS Migration + WS simplification - Design

> Extracted 2026-07-08 from `docs/plan/17-nats-migration-ws-simplification.md` (superpowers layout
> migration). Content dates from 2026-07-05; this is a point-in-time decision
> record, not a living document.

**Status:** Complete (2026-07-05)

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

**Resolved decisions (2026-07-05):**
- **Server subscription architecture:** per-WebSocket-connection NATS
  subscription (each `/ws` connection calls `client.subscribe("game.>")` on
  the shared `async_nats::Client`) - no shared/broadcast fan-out per replica.
- **Client-side refactor:** the fat-payload context (`RwSignal<Option<BrdgmeGameUpdate>>`)
  is replaced by a skinny `RwSignal<Option<(Uuid, u64)>>` (game_id, monotonic
  seq) context, bumped from `websocket_client.rs`'s `on_message_raw` handler on
  every WS message. Each game-scoped component derives its own `Memo` filtering
  the context to its `game_id` and yielding just the seq (PartialEq-deduping so
  other games' updates don't retrigger it), then keys its data
  `Resource`/`LocalResource` on `(game_id, that memo)`. The existing
  `WebSocketTrigger` global counter is kept, still bumped on every WS message
  and by existing post-action bumps, but only the layout header keys on it -
  game resources refetch only on their own game's WS signals. The
  game-changed context now has two sources: the server WS push, and a local
  bump from the submit/undo/concede success effects, so own actions refetch
  even if the WS is down or reconnecting - not just via the server's flushed
  publish. Both sources call one shared `bump_game_update` helper
  (`websocket_client.rs`) that derives the next seq from the current context
  value (prev + 1) rather than a separate counter, so the two sources can
  never coincidentally reuse a seq and get silently deduped by a component's
  PartialEq-based memo. Coalescing behaviour: latest-fetch-wins, no debounce
  - each new seq simply re-keys the resource.
- **Tests:** the two legacy Redis tests in `websocket.rs` are replaced by a
  single NATS test asserting the skinny publish lands only on `game.{id}`
  (not `user.>`/`ws.>`). Private-log filtering stays pinned at the
  `db::get_game_logs` layer by the existing
  `db::tests::game_logs_public_and_private_visibility_and_order` test - no
  separate WS-layer private-log test is needed since the skinny payload
  carries no log data at all.
- **Sequencing:** pre-cutover, after Phase 13 (see above). The old "after
  Phase 16 decommission" precondition is void: the legacy stack never runs in
  prod alongside the monolith.

## WS payload strategy change (fat → skinny)

During Phases 8-15, fat payloads are justified because legacy compat already
requires per-player `get_game_logs` DB queries and auth token lookups - the
`BrdgmeUpdate` comes for free. Post-decommission that cost exists solely to
serve the fat payload. Logs also grow unboundedly with game length.

The per-player complexity (different board HTML, `command_spec`, private logs
per player) is the root cause of most of `broadcast_game_update`'s weight.
Skinny payloads eliminate the need for player-specific messages entirely -
player-specific data comes back through the authenticated `get_game_details`
re-fetch, which is the right place for it anyway.
