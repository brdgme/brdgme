# 9: LLM Bots - Design

> Extracted 2026-07-08 from `docs/plan/09-llm-bots.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Complete

**Status note:** v1 (HTTP triggering) is complete and working. The two
follow-ups formerly tracked inside this phase now have their own phases:
ELO ratings → Phase 12, NATS bot eventing → Phase 13.

**Goal:** Add bot players backed by an LLM via an OpenAI-compatible inference
API. Bots receive the rendered game state and available commands, produce a
command string, and submit it via the monolith. The inference provider runs
outside the cluster.

## Design decisions

- **API**: OpenAI-compatible (`POST /v1/chat/completions`). Works with any
  provider: local Ollama (OpenAI-compat endpoint), OpenRouter, Groq, etc. Env
  vars `LLM_URL` (base URL) and `LLM_API_KEY` (optional Bearer token). Model
  configurable via `BOT_MODEL` env var.
- **Current dev/test provider**: OpenRouter (`https://openrouter.ai/api`).
  Note: OpenRouter's path is `/api/v1/...` not `/v1/...`, so `LLM_URL` must
  include `/api`. Local Ollama uses `http://localhost:11434` (no `/api` prefix).
- **Current model**: `openai/gpt-5.4-nano` on OpenRouter.
  OpenRouter model IDs differ from Ollama model names - check
  `GET /api/v1/models` for available models.
- **Machine-local config**: `.env` file (gitignored) holds `LLM_URL`,
  `LLM_API_KEY`, `BOT_MODEL`, `RUST_LOG`. `.env.example` documents all vars.
  Tiltfile sources `.env` at runtime in the bot serve_cmd so changing `.env`
  and restarting the bot resource (not full Tilt restart) picks up new values.
- **Bot caller**: separate Knative Service (`rust/bot` crate). Receives a
  trigger from the monolith, assembles the prompt, calls the LLM, validates the
  response, and submits the command back to the monolith via an internal API
  key. Can scale to zero - only active during bot turns.
- **Internal auth**: monolith accepts an `X-Internal-Key` header (env var
  `INTERNAL_API_KEY`) on bot command submission, bypassing session auth. The
  bot caller and monolith share this key via a Kubernetes Secret.
- **Difficulty**: `easy`, `medium`, `hard`. Stored in `game_bots`. Controls
  a section of the system prompt describing the expected play style.
- **Bot player storage**: bots are a first-class concept via a `game_bots`
  table (not fake user records). `game_players.user_id` is nullable;
  `game_players.game_bot_id` is a nullable FK to `game_bots`. A CHECK
  constraint enforces exactly one is non-null.
- **Retry on invalid command**: up to 20 attempts. Each failed attempt appends
  the rejected command and validation error to the next prompt. If all retries
  fail, the bot caller logs the failure and does nothing - the turn remains
  with the bot.
- **Bot triggering**: v1 uses direct HTTP (monolith POSTs trigger to bot
  caller). Replacing with NATS eventing is the next planned task (see
  [Phase 13](../plans/2026-07-05-13-nats-bot-eventing.md)).
- **Rendered state**: the bot receives `player_renders[n].render` in raw brdgme
  markup format. `{{player N}}` references are resolved to player names;
  all other markup tags pass through unchanged. The markup is more compact and
  semantic than HTML - estimated 5-8x reduction in render section token count.
- **Prompt logging**: full rendered prompt is logged at `tracing::trace!` level.
  Set `RUST_LOG=info,bot=trace` in `.env` to enable.

## Prompt structure [Complete]

`rust/bot/system_prompt.md` is a single MiniJinja template rendered at the
start of each LLM attempt. It covers the full prompt in one document:

1. **Persona**: expert board gamer, maximise fun and play to win.
2. **Task**: respond with exactly one plain-text command, no other text.
3. **Skill rating**: `{{ difficulty }}` injected; all three levels (easy/medium/
   hard) described so the model understands the full scale.
4. **brdgme markup legend**: static section documenting `{{b}}`, `{{fg rgb}}`,
   `{{bg rgb}}`, `{{player N}}` tags. Wrapped in `{% raw -%}...{%- endraw %}`
   to prevent minijinja treating the tag syntax as template variables. Player
   references are pre-resolved to names before the template is rendered.
5. **Command parser rules**: documentation for all 10 `Spec` variants (`Token`,
   `OneOf`, `Chain`, `Doc`, `Space`, `Opt`, `Enum`, `Int`, `Player`, `Many`)
   with YAML examples and plain-text command examples. Real Acquire `Buy`-phase
   spec included as a worked example.
6. **Game rules**: `{% if game_rules %}{{ game_rules }}{% endif %}` - omitted
   when the game has no rules text.
7. **Players**: `{% for player in players %}` loop with name, score, colour.
   Bot's own player marked `(you)`.
8. **Game render**: `{{ game_render }}` inside a ` ```text ` fence - raw brdgme
   markup with player refs resolved. `{{fg}}`/`{{bg}}` only support `rgb(r,g,b)`.
9. **Recent logs**: `{% for log in recent_logs %}` - raw brdgme markup.
10. **Command spec**: `{{ command_spec }}` inside a ` ```yaml ` fence -
    serialised via `serde_json::to_value` -> `serde_yaml::to_string` to produce
    mapping style (`Token: done`) rather than native YAML tags (`!Token`).
11. **Failed commands**: `{% if failed_commands %}` block listing each prior
    rejected command and its error. Omitted on the first attempt.

`rust/bot/src/prompt.rs` provides `markup_resolve_players`, `spec_to_yaml`,
`render_prompt`, and 14 unit tests covering all conditional sections, markup
rendering, player resolution, log rendering, player loop, and YAML spec format.

`BotContext` carries `render`, `command_spec_yaml`, `recent_logs`, and `points`
(scores). The retry loop accumulates `FailedCommand` entries and re-renders the
full template on each attempt rather than building a multi-turn conversation.

**KV cache restructure (future optimisation)**

The current design puts all content - including dynamic game state - into a
single system message re-rendered each turn. This is correct and simple, but
foregoes LLM prefix caching. A future restructure would split into multiple
messages with static content first (persona + parser docs → system; rules →
per-game-type user message) so the LLM can cache the long static prefix across
turns. Deferred until the bot is validated working well end-to-end.

## LLM provider configuration

The bot uses any OpenAI-compatible provider via `LLM_URL` + `LLM_API_KEY`.
No in-cluster inference deployment.

**Dev (hybrid mode - default `tilt up`):**

`rust/bot` runs as a local process. `.env` sets `LLM_URL`, `LLM_API_KEY`,
`BOT_MODEL`. Currently using OpenRouter:

```
LLM_URL=https://openrouter.ai/api
LLM_API_KEY=<openrouter key>
BOT_MODEL=openai/gpt-5.4-nano
RUST_LOG=info
```

Note: OpenRouter's path is `/api/v1/...` so `LLM_URL` must include `/api`.
For local Ollama use `LLM_URL=http://localhost:11434` (no `/api` prefix).

`BOT_SERVICE_URL` must point to the local bot process (default: `http://localhost:4000`);
if unset, bot auto-triggering is disabled. The Tiltfile `web` serve_cmd sets
`BOT_SERVICE_URL=http://localhost:4000` and uses `-f .mirrord/mirrord.json` to
ensure `ignore_localhost: true` is applied - without this mirrord routes
localhost through the cluster pod and bot trigger requests fail.

**Prod:**

Uses an OpenAI-compatible cloud provider. Set `LLM_URL`, `LLM_API_KEY`,
`BOT_MODEL` in the `bot-config` Kubernetes Secret.

**Home GPU over Tailscale (prod alternative):**

Ollama running on a home GPU, exposed via Tailscale. Set `LLM_URL` to the
Tailscale IP/hostname. No `LLM_API_KEY` needed (Tailscale handles network auth).

## Notes

- The bot caller does not need a game session. It authenticates to the monolith
  via `INTERNAL_API_KEY` only.
- Bot players have no `users` row. `game_players.user_id` is null for bots;
  `game_players.game_bot_id` points to the `game_bots` row for that game.
- mirrord intercepts all outgoing TCP including `localhost`. Both `rust/web` and
  `rust/bot` have `ignore_localhost: true` in their `.mirrord/mirrord.json`
  configs so localhost connections (LLM on 11434, bot on 4000) bypass the proxy
  and use the host network directly. The Tiltfile web `serve_cmd` must pass
  `-f .mirrord/mirrord.json` explicitly; without it mirrord may not load the
  config file and bot trigger requests to localhost:4000 will fail.
- Bot restart limitation: `restart_game` only collects human `opponent_ids`;
  bots are not recreated in the restarted game.
- XOR CHECK constraint on `game_players` enforces exactly one of `user_id` /
  `game_bot_id` is non-null.
- `BotSlot` is defined in `game/server_fns.rs` (not SSR-gated) and re-exported
  from `db.rs`. It must compile on WASM as a server fn parameter.
- The GameVersion CRD is managed outside Tilt in `setup-kind-cluster.sh` to
  avoid finalizer deadlocks. Do not add it back to any kustomization.
