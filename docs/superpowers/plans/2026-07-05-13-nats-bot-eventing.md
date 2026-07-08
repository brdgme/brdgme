# 13: NATS Bot Eventing - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/13-nats-bot-eventing.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete (2026-07-05)

**Spec:** `docs/superpowers/specs/2026-07-05-13-nats-bot-eventing-design.md`

## Test plan (resolved 2026-07-05)

- Integration tests run against a real NATS/JetStream server
  (docker-compose service) with the LLM mocked out, covering the happy path,
  the stale-state-conflict re-publish, attempt-limit exhaustion, and
  exactly-once delivery across two fetchers.

## Tasks

Infrastructure (pulled forward from Phase 17 - NATS is needed here regardless
of when the WS migration happens):
- [x] Add NATS (JetStream enabled) to the Kind cluster: `k8s/base/nats/`
      manifests per the resolved design above.
- [x] Add NATS to `k8s/base/brdgme/kustomization.yaml`.
- [x] Add NATS to Tiltfile (deploy + port-forward).
- [x] Add `async-nats` to `rust/web/Cargo.toml` and `rust/bot/Cargo.toml`.
- [x] Add `NATS_URL` env var to monolith and bot (Tiltfile + k8s secrets).

Monolith changes:
- [x] Replace `trigger_bot_turns` HTTP POST with NATS publish to `bot.turn`.
- [x] Remove `BOT_SERVICE_URL` env var.
- [x] Subscribe to `bot.command` on startup; handler calls `execute_command`
      and saves to DB. On stale state conflict: re-publish `bot.turn` with
      `attempt` incremented. Enforce overall attempt limit (e.g. 3 turn-level
      retries before giving up).
- [x] Remove `POST /api/internal/game/{id}/command` endpoint and
      `INTERNAL_API_KEY` (no longer needed for bot auth).

Bot changes:
- [x] Remove Axum HTTP server (`/trigger` endpoint, port 4000).
- [x] Subscribe to `bot.turn` on startup; process each message as a turn.
- [x] Replace game service `Status` + LLM + monolith POST retry loop with:
      Status → LLM → game service `Play` (validate) → retry LLM on
      invalid → publish `bot.command` when valid.
- [x] Remove `MONOLITH_URL` and `INTERNAL_API_KEY` from `AppState` and env.
- [x] Update k8s `bot-config` secret to remove those vars, add `NATS_URL`.

## Implementation notes

The monolith acks a `bot.command` message only on
success or a stale-state conflict (the conflict is resolved by re-publishing
`bot.turn`, so the original message is done); a transient
(`ExecuteCommandError::Other`) failure is left unacked so JetStream
redelivers it (`ack_wait` 5m, `max_deliver: 3` backstop). Symmetrically, the
bot leaves a failed `bot.turn` unacked rather than acking and swallowing the
error. One side effect: the bot Deployment now has no HTTP port, so it has no
health probe - tracked as a follow-up in Phase 18.
