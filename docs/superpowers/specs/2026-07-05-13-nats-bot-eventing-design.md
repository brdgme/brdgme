# 13: NATS Bot Eventing - Design

> Extracted 2026-07-08 from `docs/plan/13-nats-bot-eventing.md` (superpowers layout
> migration). Content dates from 2026-07-05; this is a point-in-time decision
> record, not a living document.

**Status:** Complete (2026-07-05)

## Problem

v1 bot triggering uses direct HTTP (monolith POSTs to bot service, bot POSTs
command back to monolith). This creates bi-directional HTTP coupling: the bot
needs `MONOLITH_URL` and `INTERNAL_API_KEY` just to submit a move. Replace with
NATS eventing.

**Precondition:** Phase 14 (drop Knative) runs first - the bot becomes an
always-on Deployment, which is what lets it hold a NATS subscription at all.
The former scale-to-zero-vs-subscriber conflict flagged here is resolved by
that decision (2026-07-03).

## Decisions resolved 2026-07-03 (tech review)

- **Delivery guarantees: JetStream from day one.** NATS Core is
  at-most-once; a `bot.turn` lost during a bot deploy or NATS restart is a
  stuck turn until a human clicks "bump". JetStream makes bot eventing
  at-least-once for the cost of a server config flag and a small PVC. WS
  fan-out (Phase 17) deliberately stays Core pub/sub - ephemeral is correct
  there.
- **Stream design:** one stream `BOT` capturing `bot.>`, WorkQueue
  retention, two durable pull consumers with non-overlapping filters:
  `bot-turn` (filter `bot.turn`, fetched by the bot) and `bot-command`
  (filter `bot.command`, fetched by monolith replicas). Explicit ack after
  processing; `ack_wait` ~5 min (a turn including LLM retries must complete
  or be redelivered); `max_deliver: 3` as a poison-message backstop. Stream
  and consumers are created idempotently by the monolith on startup
  (async-nats jetstream API), not by manifests.
- **`k8s/base/nats/` manifests:** official `nats:2.11-alpine` image,
  StatefulSet with 1 replica + 1Gi PVC (JetStream file store), JetStream
  enabled via config, ClusterIP Service on 4222. No auth in-cluster
  (consistent with the Redis/Postgres posture). Monitoring port 8222
  exposed for the readiness probe (`/healthz`).
- **Attempt limits:** 20 LLM attempts per turn (unchanged); 3 turn-level
  re-publishes on state conflict (`attempt` field); re-publish immediately,
  no delay (conflicts are rare and the LLM loop itself is slow).

## Resolved 2026-07-05

- **Rollout sequencing:** big-bang swap - no dual path. The HTTP
  trigger/callback flow is removed outright rather than kept behind a flag.

## Design

```
Monolith  --[bot.turn]--> NATS
Bot       <-- subscribes to bot.turn
Bot       --> DB (fetch game state + game service URI)
Bot       --> game service Status (render + command_spec)
loop:
  Bot     --> LLM (get command)
  Bot     --> game service Play (validate - stateless, no DB commit)
  if invalid: accumulate FailedCommand, retry LLM
  if valid: break
Bot       --[bot.command]--> NATS
Monolith  <-- subscribes to bot.command
Monolith  --> game service Play + DB save
if stale state: Monolith --[bot.turn]--> NATS (increment attempt counter)
```

Key design decisions:
- **Bot validates against game service directly.** `Play` calls are stateless
  (return new state but don't persist). The bot can use them to validate without
  side effects. This keeps the retry loop entirely inside the bot with no
  monolith round-trip per attempt.
- **State conflict handled by monolith.** If the game state changes between the
  bot's validation and the monolith's commit (e.g. undo), the monolith detects
  the conflict and re-publishes `bot.turn`. An attempt counter in the event
  payload provides an overall retry limit.
- **Bot loses HTTP server entirely.** No `/trigger` endpoint, no `MONOLITH_URL`,
  no `INTERNAL_API_KEY`. The bot's only dependencies become DB, game service,
  LLM provider, and NATS.
- **Monolith loses `BOT_SERVICE_URL`.** `trigger_bot_turns` is replaced by a
  NATS publish. No outbound HTTP to bot service.

## NATS subjects

- `bot.turn` — payload: `{game_id, player_position, difficulty, attempt}`
- `bot.command` — payload: `{game_id, player_position, command, attempt}`
  (`attempt` echoes the originating `bot.turn` event's counter, so the
  turn-level retry cap survives the round-trip through the bot)

## Exactly-one-instance delivery (required for correctness)

Each message must be processed by exactly one subscriber instance - the
monolith runs multiple replicas, and a plain subscribe to `bot.command` would
execute every bot command once per replica. With JetStream durable pull
consumers (decided 2026-07-03, above) this falls out naturally: all replicas
fetch from the same durable consumer, each message goes to exactly one
fetcher, and a missed ack triggers redelivery.
