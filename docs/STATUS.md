# Current Status

## Session: 2026-04-03 - Bot end-to-end testing, prompt markup switch, frontend fixes

### Where we are

The bot is fully operational end-to-end. Multiple manual bot commands were
successfully executed for game `faea1e29-16ff-4284-bd90-f2d7a973aeb6`. The bot
auto-trigger is now working (mirrord config fix). The prompt sends raw brdgme
markup instead of HTML, giving a 5-8x reduction in render section token count.
The current model is `openai/gpt-5.4-nano` on OpenRouter (changed from
`qwen/qwen3.6-plus:free` → `gpt-oss-20b` → `gpt-5.4-nano` during testing).

**Immediate next task:** NATS bot eventing (see PLAN.md Phase 9). Replaces
bi-directional HTTP between monolith and bot with NATS pub/sub. Full design
already in PLAN.md.

### What was done this session

**Prompt: HTML to brdgme markup [Complete]**
- `rust/bot/src/prompt.rs`: removed `markup_to_html` and all HTML-related
  imports. Added `markup_resolve_players(markup, names)` which only resolves
  `{{player N}}` tags via string replacement and passes all other markup through
  unchanged.
- `BotContext` fields renamed: `render_html` -> `render`, `recent_logs_html` ->
  `recent_logs`. Updated `main.rs` accordingly.
- `system_prompt.md` restructured: brdgme markup documentation moved to a
  dedicated `# brdgme markup` static section before any game-specific content.
  The markup legend uses `{% raw -%}...{%- endraw %}` to prevent minijinja from
  treating `{{b}}` etc. as template variables. Game render section changed from
  html fence to text fence. Logs section updated to reference markup docs.
- Confirmed: `{{fg}}` and `{{bg}}` only support `rgb(r,g,b)` - verified from
  parser source. `{{c colourname}}` is a Go-era backwards-compat alias not used
  in any current Rust or Go game code.
- All 14 prompt unit tests pass.

**Bot auto-trigger fix [Complete]**
- Root cause: `mirrord exec` without `-f` flag was not reliably loading
  `.mirrord/mirrord.json`, causing localhost:4000 connections to be routed
  through the cluster pod instead of the host network. Bot trigger requests to
  `http://localhost:4000/trigger` were failing with connection errors.
- Fix: added `-f .mirrord/mirrord.json` to the web `serve_cmd` in `Tiltfile`.
- After fix: bot triggers arrive at the bot process and commands execute
  successfully.

**Trigger logging in monolith [Complete]**
- `trigger_bot_turns` in `game/mod.rs`: added `tracing::debug!` per player
  (position, is_turn, is_bot), `tracing::info!` on trigger fire (position,
  difficulty, url), and tokio-spawned success/failure logs (`tracing::warn!` on
  non-2xx or connection error, `tracing::debug!` on success).
- `execute_command`: changed bare `if let Ok(Some(...))` to a full `match` with
  `tracing::warn!` on `None` and `Err` cases.
- `RUST_LOG` in `.env` simplified from `info,bot=trace` to `info`.

**Frontend: autofocus fix [Complete]**
- `GameCommandInput` in `components/game.rs`: replaced HTML `autofocus`
  attribute (only fires on browser page load) with a `NodeRef` + `Effect::new`
  that calls `el.focus()` on component mount. This fires on both hard refresh
  and client-side SPA navigation.
- Post-submit focus retained: existing Effect that clears the command on success
  also re-focuses the input.

**Frontend: hard refresh stuck loading fix [Complete]**
- `GamePage` in `app.rs`: changed `Resource::new` to `Resource::new_blocking`
  for `game_data`. Non-blocking caused SSR to send fallback HTML then rely on
  client re-fetch, which was failing silently. Blocking makes SSR wait for
  `get_game_details`, serialises result into HTML, and client has data
  immediately on hydration with no second fetch.

**Acquire RULES.md: 5 new strategy notes [Complete]**
- Added to the `## Strategy notes` section:
  - 13 shares = guaranteed majority; no need to buy more solely for majority.
  - Small-corp early investment: share value grows, shareholder bonuses, trade
    opportunities.
  - 4-share lead is generally safe majority (3-per-turn buy limit), except
    during mergers where bulk trading can shift majority.
  - Running out of money is dangerous - prioritise triggering mergers to earn
    bonuses and free up capital.
  - Diversify across corporations - aim for minor or major bonus position in as
    many as possible.

### Current .env

```
LLM_URL=https://openrouter.ai/api
LLM_API_KEY=<openrouter key>
BOT_MODEL=openai/gpt-5.4-nano
RUST_LOG=info
```

OpenRouter path requires `/api` in `LLM_URL` (their API is at `/api/v1/...`).
Local Ollama uses `LLM_URL=http://localhost:11434` (no `/api` prefix).

### Known open issues

- **Restart 500 error**: `restart_game` returns "Game service error: error
  parsing JSON response". Diagnostics in place (`client::request` reads body as
  text before parsing and includes it in the error). Needs a live restart to
  capture raw response.
- **Bot restart limitation**: `restart_game` only collects human `opponent_ids`;
  bots are not recreated in the restarted game.
- **Optimistic locking**: race condition in `execute_command` +
  `update_game_command_success` (design in PLAN.md).

### Next steps (in order)

1. **NATS bot eventing** (PLAN.md Phase 9) - replace bi-directional HTTP with
   NATS pub/sub. Full design in PLAN.md. Decouples bot from monolith, removes
   `BOT_SERVICE_URL`, `MONOLITH_URL`, `INTERNAL_API_KEY`.

2. **Continue bot quality testing on Acquire** - multiple games end-to-end
   before writing rules for other game types.

3. **Rules for other games** - `lords-of-vegas-1`, `lost-cities-1`,
   `lost-cities-2` once bot stable on Acquire.

4. **Optimistic locking** (PLAN.md Phase 9) - race condition in
   `execute_command` + `update_game_command_success`.

5. **Phase 6.5** - Production CD (ArgoCD + separate `brdgme-config` repo).

6. **Phase 7** - Side-by-side validation then legacy decommission.
