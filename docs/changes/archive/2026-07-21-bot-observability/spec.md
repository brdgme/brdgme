# Bot Observability - Design

Date: 2026-07-21
Status: Draft

## Problem

The bot service (`rust/bot`) has near-zero observability into turn latency.
Logs don't show which bot played, which LLM provider/model was used, how long
each phase took, or whether the turn succeeded. When bots are slow (tens of
seconds to minutes), there is no way to determine whether the bottleneck is
the LLM provider, the game service, DB queries, or NATS delivery.

## Goals

1. Every bot turn logs a clear START and END (success or failure) with
   identifying information and total elapsed time.
2. Every outgoing LLM provider request logs a START and END with provider,
   model, attempt number, and elapsed time.
3. All logs carry enough structured fields to filter by game, bot, provider,
   and model without parsing free-text.
4. Minimal code change - instrument the existing `run_bot_turn` and `call_llm`
   functions using the `tracing` crate already in use.

## Non-goals

- Prometheus metrics / histograms (can be added later on top of structured
  logs).
- Sentry integration for the bot (separate concern).
- Changing the NATS consumer configuration or retry semantics.

## Design

### Two levels of instrumentation

| Level | Start trigger | End trigger | Key fields |
|-------|--------------|-------------|------------|
| Turn | NATS message parsed, `run_bot_turn` entered | Command published (success) or hard error (failure) | trace_id, game_id, player_position, bot_name, total elapsed_ms, outcome |
| LLM request | About to call `call_llm` | `call_llm` returns (Ok or Err) | trace_id, game_id, player_position, bot_name, provider_url, model, attempt, elapsed_ms, outcome |

### Turn-level logging

At the top of `run_bot_turn`, after the initial DB fetch confirms the turn is
still active:

```
INFO bot_turn_start
  trace_id, game_id, player_position, bot_name, attempt (NATS-level)
```

At every exit point (success via `publish_bot_command`, or hard failure):

```
INFO bot_turn_end
  trace_id, game_id, player_position, bot_name, attempt,
  elapsed_ms, outcome = "success", command

ERROR bot_turn_end
  trace_id, game_id, player_position, bot_name, attempt,
  elapsed_ms, outcome = "failure", error
```

Implementation: capture `Instant::now()` at entry. Use a `tracing::Span` on
`run_bot_turn` carrying the identity fields so all child logs inherit them.
Log the end event at each `return` site (or restructure into a single exit
with a result).

### LLM request-level logging

Inside the retry loop, immediately before and after `call_llm`:

```
INFO llm_request_start
  trace_id, game_id, player_position, bot_name,
  provider_url, model, attempt (loop index)

INFO llm_request_end
  trace_id, game_id, player_position, bot_name,
  provider_url, model, attempt, elapsed_ms, outcome = "success"

WARN llm_request_end
  trace_id, game_id, player_position, bot_name,
  provider_url, model, attempt, elapsed_ms, outcome = "error", error
```

Implementation: `Instant::now()` before `call_llm`, log after. The
provider_url and model are already available from the `ProviderRouter` at that
point.

### Structured fields

All fields are emitted as `tracing` structured fields (key = %value), not
interpolated into message strings. This keeps them machine-parseable by any
log backend (Loki, stdout JSON, etc.).

Fields present on every log line within a turn (via the span):
- `trace_id` (UUID, already exists)
- `game_id` (UUID)
- `player_position` (i32)
- `bot_name` (String)

Fields added per-event:
- `provider_url`, `model`, `attempt` (LLM request logs)
- `elapsed_ms` (u128, end logs)
- `outcome` ("success" | "failure" | "error")
- `command` (turn success only)
- `error` (failure only)

### Game service call timing

The `fetch_game_data` and Play-validation calls also contribute to latency.
The bot uses `tracing_subscriber::fmt::init()` which does NOT emit span
durations by default, so the existing `#[tracing::instrument]` on
`brdgme_game_client::request()` won't show timing in stdout logs. Add
explicit `elapsed_ms` fields at the two call sites in `run_bot_turn`
(`load_bot_context` and the Play-validation call) using the same
`Instant::now()` pattern. No change to the game_client library itself.

### What changes in code

1. `rust/bot/src/main.rs`:
   - Add a `#[tracing::instrument]` span on `run_bot_turn` with identity
     fields (or construct the span manually for the dynamic fields).
   - Capture `Instant` at entry, log `bot_turn_start`.
   - At each return path, log `bot_turn_end` with elapsed + outcome.
   - In the retry loop, wrap `call_llm` with start/end logs including
     provider_url, model, attempt, elapsed_ms.

2. `rust/lib/game_client/src/lib.rs`:
   - Verify the existing `#[tracing::instrument]` on `request()` emits
     duration in the log output. If the subscriber format doesn't show span
     duration, add explicit elapsed_ms at the `run_bot_turn` call sites
     instead (simpler, no library change needed).

### Log output examples

```
INFO bot_turn_start trace_id=abc game_id=def player_position=1 bot_name="AcquireBot" attempt=0
INFO llm_request_start trace_id=abc game_id=def player_position=1 bot_name="AcquireBot" provider_url="https://api.deepseek.com" model="deepseek-v4-flash" attempt=0
INFO llm_request_end trace_id=abc game_id=def player_position=1 bot_name="AcquireBot" provider_url="https://api.deepseek.com" model="deepseek-v4-flash" attempt=0 elapsed_ms=2340 outcome="success"
INFO bot_turn_end trace_id=abc game_id=def player_position=1 bot_name="AcquireBot" attempt=0 elapsed_ms=4100 outcome="success" command="buy 2 sackson"
```

Failure case:

```
INFO bot_turn_start trace_id=abc game_id=def player_position=0 bot_name="AcquireBot" attempt=0
INFO llm_request_start trace_id=abc ... provider_url="https://api.deepseek.com" model="deepseek-v4-flash" attempt=0
WARN llm_request_end trace_id=abc ... provider_url="https://api.deepseek.com" model="deepseek-v4-flash" attempt=0 elapsed_ms=5000 outcome="error" error="LLM returned 503: service unavailable"
INFO llm_request_start trace_id=abc ... provider_url="https://fallback.example.com" model="gpt-4o-mini" attempt=1
INFO llm_request_end trace_id=abc ... provider_url="https://fallback.example.com" model="gpt-4o-mini" attempt=1 elapsed_ms=1800 outcome="success"
INFO bot_turn_end trace_id=abc ... elapsed_ms=9200 outcome="success" command="place C4"
```

## Testing

- Unit: not applicable (logging is side-effectful; correctness is verified by
  reading structured output).
- Integration: run the bot against a test game with a mock LLM, confirm logs
  contain all required fields at both levels.
- Manual: deploy to beta, trigger a bot game, verify logs in `kubectl logs`
  show the full turn lifecycle with timing.
