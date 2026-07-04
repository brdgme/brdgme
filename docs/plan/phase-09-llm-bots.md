# Phase 9: LLM Bots

**Status:** Complete

**Status note:** v1 (HTTP triggering) is complete and working. The two
follow-ups formerly tracked inside this phase now have their own phases:
ELO ratings → Phase 12, NATS bot eventing → Phase 13.

**Goal:** Add bot players backed by an LLM via an OpenAI-compatible inference
API. Bots receive the rendered game state and available commands, produce a
command string, and submit it via the monolith. The inference provider runs
outside the cluster.

### Design decisions

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
  [Phase 13](phase-13-nats-bot-eventing.md)).
- **Rendered state**: the bot receives `player_renders[n].render` in raw brdgme
  markup format. `{{player N}}` references are resolved to player names;
  all other markup tags pass through unchanged. The markup is more compact and
  semantic than HTML - estimated 5-8x reduction in render section token count.
- **Prompt logging**: full rendered prompt is logged at `tracing::trace!` level.
  Set `RUST_LOG=info,bot=trace` in `.env` to enable.

### Game contract extension

- [x] Add `Rules` request/response variants to `rust/lib/cmd/src/api.rs`.
- [x] Implement `Rules` handler in `rust/lib/cmd/src/requester/gamer.rs`.
- [x] Add `fn rules() -> String` to `Gamer` trait in `rust/lib/game/src/game.rs`
      (default: empty string).
- [x] Add empty stub `fn rules()` to all 4 Rust games (acquire-1, lords-of-vegas-1,
      lost-cities-1, lost-cities-2). Rules text deferred - will use
      `include_str!("../rules.md")` pattern when written.

### Database changes

- [x] Migration `003_game_bots.sql`: `game_bots` table, nullable `user_id`,
      `game_bot_id` FK, XOR CHECK constraint. Migration applied to dev DB.
- [x] `GamePlayer.user_id: Option<Uuid>` in `rust/web/src/models/game.rs`.
- [x] `GameBot` struct added to `rust/web/src/models/game.rs`.
- [x] `GamePlayerExtended` updated: `user: Option<User>`, `game_bot: Option<GameBot>`,
      `name()` helper. In `rust/web/src/db.rs`.
- [x] `find_game_extended` and `find_active_games_for_user` queries updated to
      LEFT JOIN users + LEFT JOIN game_bots. SQLx cache regenerated.
- [x] All callsites updated: `mod.rs`, `server.rs`, `server_fns.rs`, `websocket.rs`.
      `user.id` accesses guarded with `as_ref().is_some_and()`. Bot players skipped
      for WS publishing. `name()` used everywhere instead of `user.name`.

### Monolith changes (`rust/web`)

- [x] `execute_command` refactored to take `player_position: usize` instead of
      `user_id`. `play_command` and `submit_command` server fn do a lightweight
      position lookup first.
- [x] `POST /api/internal/game/{id}/command` added to `server.rs`. Auth via
      `X-Internal-Key` header checked against `INTERNAL_API_KEY` env var.
      Calls `execute_command` with position from request body.
- [x] `trigger_bot_turns` helper added to `mod.rs`. Reads `BOT_SERVICE_URL`
      env var (disabled if unset). For each bot player with `is_turn = true`,
      spawns background `tokio::spawn` POSTing to `BOT_SERVICE_URL/trigger`.
- [x] `trigger_bot_turns` called from `execute_command` (after broadcast).
- [x] `trigger_bot_turns` called from `server.rs`: `create_game`, `undo_game`,
      `concede_game`, `restart_game`.
- [x] `trigger_bot_turns` called from `server_fns.rs`: `concede_game`,
      `restart_game`, `create_new_game` server fns.
- [x] New game creation with bot slots: `CreateGameOpts` extended with
      `bot_slots: &[BotSlot]`. Handler inserts `game_bots` rows then
      `game_players` rows with `game_bot_id` set and `user_id = NULL`.
- [x] New game UI: per-opponent slot Human/Bot toggle with name + difficulty
      fields. `opponent_emails` and `bot_slots` use `Option<Vec<_>>` to handle
      absent form fields in URL encoding.
- [x] `is_bot: bool` added to `PlayerViewData`; populated in both
      `server_fns.rs` and `websocket.rs`.
- [x] "Bump bot to play" link in `GameMeta` actions panel: shown when any bot
      player has `is_turn = true`. Dispatches `BumpBotTurns` server fn which
      calls `trigger_bot_turns`. Auth-gated to game players only.
- [x] Bot stale-turn race condition (two layers):
  - Pre-LLM: check `is_turn` from initial DB fetch; bail early if false.
  - Post-LLM: re-query `is_turn` and `game_state` after each LLM response.
    If not their turn: bail. If game state changed (e.g. undo): refresh render,
    logs, command spec via `load_bot_context` helper; reset conversation and
    retry LLM with fresh context. Attempt counter continues across refreshes.

### Bot caller service (`rust/bot`)

New Rust binary crate. Deployed as a Knative Service. Receives trigger POSTs
from the monolith, assembles the full prompt, calls the LLM, retries on
failure, then submits the command to the monolith's internal endpoint.

- [x] `rust/bot/` crate: minimal Axum server on port 4000, `POST /trigger`.
- [x] Request body: `{"game_id": "uuid", "player_position": 0, "difficulty": "medium"}`.
- [x] On trigger: fetch game + player names + logs from DB; call game service
      `Status` for render + command spec; call `Rules`; assemble prompt; POST to
      LLM `/v1/chat/completions`; retry up to 20 times on validation error; on
      hard failure log error and leave turn with bot.
- [x] `load_bot_context` helper: fetches render, command spec, logs. Called at
      start and on mid-loop state refresh.
- [x] Env vars: `DATABASE_URL`, `BOT_MODEL`, `LLM_URL`, `LLM_API_KEY`,
      `MONOLITH_URL`, `INTERNAL_API_KEY`. Dynamic sqlx queries (no .sqlx cache).
- [x] `rust/bot` Dockerfile target added to `rust/Dockerfile`.
- [x] `rust/bot` added to workspace `Cargo.toml`.

### Acquire rules [Complete]

- `rust/game/acquire-1/RULES.md` written: board layout, corporations, pricing,
  mergers, bonuses, rendering guide, command reference.
- `fn rules()` updated to `include_str!("../RULES.md").to_string()`.
- 2-player special rule corrected: dummy share count uses a D6 roll (1-6), not
  a drawn tile's column number (1-12). Rules now match the game implementation.
- 5 strategy notes added: 13-share majority guarantee, small-corp early
  investment, 4-share safe majority (bulk merge exception), capital management,
  portfolio diversification across corporations.

### Prompt structure [Complete]

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

### Bot k8s manifests

- [x] `k8s/base/bot/service.yaml`: Knative Service for `brdgme/bot` image.
      Reads `postgres-config` + `bot-config` secrets.
- [x] `k8s/base/bot/` added to `k8s/base/brdgme/kustomization.yaml`.
- [x] Tiltfile: bot `local_resource` in hybrid mode sources `.env` for
      `LLM_URL`, `LLM_API_KEY`, `BOT_MODEL`, sets `MONOLITH_URL` and
      `INTERNAL_API_KEY`. Web gets `BOT_SERVICE_URL` and `INTERNAL_API_KEY`.
      Full-cluster mode builds `brdgme/bot` and creates `bot-config` secret.

### LLM provider configuration

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

### Bot service logging improvements [Complete]

Structured logging at key points in `run_bot_turn`. A `trace_id`
(`Uuid::new_v4()`) is generated at the start of each trigger request and
threaded through all log entries so concurrent bot turns can be correlated.

- **On trigger received**: `tracing::info!` with trace_id, game_id, game name,
  player position, player name, difficulty.
- **On each LLM response**: `tracing::trace!` with trace_id, attempt number,
  full rendered prompt (visible at `RUST_LOG=bot=trace`).
- **On command submitted**: `tracing::info!` with trace_id, attempt, command,
  HTTP status. `tracing::warn!` if rejected, with validation error body.
- **On retry**: `tracing::warn!` with trace_id, attempt, validation error.
- **On state change mid-loop**: `tracing::warn!` with trace_id, attempt.
- **On hard failure**: `tracing::error!` with trace_id, error chain (Debug),
  attempt count.

All fields are structured key-value pairs for machine-parseable log aggregation.

Monolith `trigger_bot_turns` also logs: `tracing::debug!` per player (position,
is_turn, is_bot), `tracing::info!` on trigger fire (position, difficulty, url),
and tokio-spawned `tracing::warn!`/`tracing::debug!` on HTTP error/success.

### Notes

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

