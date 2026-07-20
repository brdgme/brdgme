# Bot Efficacy Design (#43)

Date: 2026-07-20
Status: Draft
Relates to: #47 (concede with bot replacement), #25 (rules rendering)

## Problem

Bots struggle to parse the brdgme markup render. It is a large, noisy payload
designed for human terminal display - not structured data for LLM consumption.
The bot currently receives the full render as its primary game-state input,
leading to poor move quality and high token spend.

Additionally, bot "difficulty" is purely prompt wording (three behavioral
paragraphs in the system prompt). All difficulties share one model, one
provider, one reasoning budget. There is no per-difficulty configuration,
no failover, and no admin control.

## Goals

1. Improve bot move quality by replacing the render with structured data.
2. Reduce token spend (smaller, higher-signal prompts).
3. Make difficulty a complete bot configuration (model, provider, thinking,
   docs), not prompt wording.
4. Support multi-provider failover and load balancing per bot.
5. Design for a future admin GUI to manage bot configurations.

Non-goals (deferred):
- Concede -> bot takeover (#47) - separate design session.
- All-humans-gone -> stop game - separate design session.
- Admin GUI for bot config - future work, schema designed for it now.

---

## D1: Structured Data Instead of Render

The bot receives `pub_state` and `player_state` as YAML (re-serialized from
the JSON strings already returned by the game service `Status` endpoint).
The brdgme markup render is no longer included in the bot prompt.

- `pub_state` YAML: what a spectator sees (no hidden information).
- `player_state` YAML: what this player sees (includes hidden info like
  hands, private resources).

Both are passed so the bot can distinguish public from hidden information.

YAML chosen for consistency with the existing `command_spec` serialization
in the prompt.

## D2: Auto-Generated Data Documentation

Each game's `PubState` and `PlayerState` structs carry `///` doc comments
on every field explaining what the field represents. A derive macro
(`#[derive(DataDocs)]`) or codegen step extracts field names, types, and
doc comments into a static documentation string accessible at runtime.

The game service exposes this via a `data_docs()` method on the `Gamer`
trait (embedded at compile time, like `rules()`). The bot includes the
data dictionary in its prompt so the LLM understands what each YAML field
means.

No manual markdown file to maintain - docs are generated from the code
and always in sync with the actual types.

## D3: Per-Game Document Split

Each game crate (`rust/game/<name>-N/`) has:

| File | Audience | Bot receives? |
|------|----------|---------------|
| `RULES.md` | Humans + bots | Always |
| `BASIC_STRATEGY.md` | Humans + bots | If `bots.include_basic_strategy` (default true) |
| `ADVANCED_STRATEGY.md` | Humans + bots | If `bots.include_advanced_strategy` (default false) |
| Data docs (auto-generated) | Bots (+ humans later) | Always |

- `RULES.md`: pure rules of the game. No render explanation, no strategy.
- `BASIC_STRATEGY.md`: hard rules to avoid obviously terrible moves
  ("don't go all-in with 2-7", "don't take a discard lower than your
  played card in Lost Cities"). All bots get this.
- `ADVANCED_STRATEGY.md`: higher-level strategic advice for optimal play.
  Only Hard-configured bots get this.
- No `EXAMPLES.md` - render explanation is irrelevant once bots use
  structured data.

All docs embedded via `include_str!` and served through the game service
(same pattern as `rules()`). The `Gamer` trait gains:

```rust
fn basic_strategy() -> String { String::new() }
fn advanced_strategy() -> String { String::new() }
fn data_docs() -> String { String::new() }
```

## D4: Game Interface Versioning

The game HTTP contract is versioned to allow interface evolution without
rebuilding deployed game services.

- **V1** (all current Go games): New, Status, Play, PlayerCounts, Rules.
- **V2** (new Rust games): adds DataDocs, BasicStrategy, AdvancedStrategy.

The `GameVersion` CRD gains `interface_version: i32` (default 1). The
operator stores it in `game_versions.interface_version`.

The `game_client` crate abstracts versions completely. Callers receive a
common output type:

```rust
pub struct GameData {
    pub pub_state_yaml: String,
    pub player_state_yaml: String,
    pub data_docs: String,
    pub basic_strategy: String,
    pub advanced_strategy: String,
    pub command_spec: Option<Spec>,
    pub rules: String,
}
```

For V1 games, `data_docs`/`basic_strategy`/`advanced_strategy` contain
`"Not supported in game interface V1"`. Callers never see a version
number. When V1 is eventually deprecated, the placeholder logic is
deleted - zero caller changes.

New HTTP contract methods (V2 only):
- Request: `"DataDocs"` / Response: `{"DataDocs": {"data_docs": "..."}}`
- Request: `"BasicStrategy"` / Response: `{"BasicStrategy": {"strategy": "..."}}`
- Request: `"AdvancedStrategy"` / Response: `{"AdvancedStrategy": {"strategy": "..."}}`

## D5: Bot Configuration Database Schema

Difficulty becomes a named bot configuration, not a constraint.

```sql
CREATE TABLE bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    display_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    include_basic_strategy BOOLEAN NOT NULL DEFAULT true,
    include_advanced_strategy BOOLEAN NOT NULL DEFAULT false,
    temperature REAL NOT NULL DEFAULT 0.2,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE llm_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    api_key_encrypted BYTEA,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bot_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES llm_providers(id) ON DELETE CASCADE,
    model TEXT NOT NULL,
    reasoning_effort TEXT,
    extra_body JSONB,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (bot_id, provider_id, model)
);
```

`game_bots.difficulty` is renamed to `game_bots.bot_name` (TEXT). The
CHECK constraint ('easy'|'medium'|'hard') is dropped. If `bot_name`
does not match any row in `bots`, or the matched bot is disabled, the
bot slot is paused (no play made, no error).

### Three-layer enable gate

A bot makes a play only if ALL of:
1. `bots.enabled = true` (the bot config is active)
2. `bot_providers.enabled = true` (this bot+provider link is active)
3. `llm_providers.enabled = true` (the provider endpoint is active)

### Routing logic

1. Load bot config by `game_bots.bot_name`.
2. Load enabled `bot_providers` JOIN `llm_providers` (both enabled),
   ordered by `priority`.
3. Group by priority level.
4. Within same priority: round-robin (load balance).
5. On failure: next provider in same group, then next priority group
   (failover).

### Seed data

```sql
INSERT INTO bots (name, display_order, include_basic_strategy, include_advanced_strategy, temperature)
VALUES
    ('easy',   0, true,  false, 0.2),
    ('medium', 1, true,  false, 0.2),
    ('hard',   2, true,  true,  0.2);
```

Provider and bot_provider rows configured per deployment (dev: single
OpenRouter provider; prod: per the operator's desired config).

## D6: Credential Encryption

`llm_providers.api_key_encrypted` stores AES-256-GCM ciphertext (bytea).
The encryption key comes from `BOT_ENCRYPTION_KEY` env var (k8s
sealed-secret in prod, plain env in dev).

- Encrypt on write (admin GUI / seed script).
- Decrypt on read (bot crate, in-memory only).
- Plaintext never touches the database.
- Key rotation: re-encrypt all rows with new key (admin operation).

## D7: Bot Prompt Restructure

The prompt splits into static (cacheable) and dynamic (per-turn) content.

### System message (static per game + bot config)

1. Persona: "You are an expert board gamer. Play to win."
2. Task: "Respond with exactly one valid command as a single line of
   plain text. No explanation."
3. Game rules (`RULES.md`)
4. Basic strategy (`BASIC_STRATEGY.md`, if configured)
5. Advanced strategy (`ADVANCED_STRATEGY.md`, if configured)
6. Data dictionary (auto-generated field docs for PubState + PlayerState)
7. Command parser documentation (Spec variants - retained, still needed
   for the bot to understand command_spec YAML)

### User message (dynamic, per turn)

8. Players + scores table
9. Public data (YAML)
10. Your player data (YAML)
11. Command spec (YAML)
12. Recent logs (last N entries, trimmed)
13. Failed commands (if retrying, with error messages)
14. "Please provide your command now."

### Removed

- brdgme markup legend (bot no longer sees markup)
- The full render
- Difficulty behavioral paragraphs (replaced by strategy docs +
  model/thinking config)

### KV cache optimization

Sections 1-7 are identical across turns for the same game + bot config.
Providers supporting prompt caching (OpenAI, Anthropic, OpenRouter
passthrough) cache this prefix. Only sections 8-14 are re-processed
each turn. This significantly reduces per-turn token billing.

## D8: LLM Request Construction

The OpenAI-compatible chat completions request is built from:

```json
{
  "model": "<bot_providers.model>",
  "temperature": <bots.temperature>,
  "reasoning_effort": "<bot_providers.reasoning_effort>",
  "messages": [
    {"role": "system", "content": "<static prompt>"},
    {"role": "user", "content": "<dynamic prompt>"}
  ],
  "stream": false
}
```

- `reasoning_effort`: omitted from the request if NULL.
- `extra_body` (JSONB): merged into the top-level request object if
  non-NULL. Allows arbitrary provider-specific fields.
- Field names map 1:1 to the OpenAI chat completions API.

## D9: Bot Crate Changes Summary

The bot crate (`rust/bot/`) changes:

1. **Config source**: reads bot config from Postgres (`bots`,
   `llm_providers`, `bot_providers`). If the DB tables are empty
   (fresh dev environment), falls back to env vars (`LLM_URL`,
   `BOT_MODEL`, `REASONING_EFFORT`) as a single implicit provider.
   This keeps `devenv` zero-config for new contributors.
2. **Data fetch**: calls `game_client` for structured `GameData`
   (YAML states + docs + strategies) instead of extracting the render
   from `Status`.
3. **Prompt**: new template split (static system + dynamic user).
   MiniJinja template rewritten.
4. **Provider routing**: round-robin + failover logic per D5.
5. **Encryption**: AES-256-GCM decrypt of `api_key_encrypted` at
   startup / per-request.
6. **Retry**: existing 20-attempt loop with failed-command feedback
   retained. Provider failover integrates into the retry (try next
   provider on LLM API errors, not just on invalid commands).

## D10: Migration Path

1. Migration adds `bots`, `llm_providers`, `bot_providers` tables.
2. Migration renames `game_bots.difficulty` -> `game_bots.bot_name`,
   drops the CHECK constraint.
3. NATS `bot.turn` event payload field renamed `difficulty` ->
   `bot_name` (monolith publisher + bot consumer updated together).
4. Seed `bots` table with easy/medium/hard.
4. Seed `llm_providers` + `bot_providers` per environment (dev seed
   script; prod via sealed-secret + init job or manual insert).
5. `game_versions` gains `interface_version` column (default 1).
6. Operator updated to read `interfaceVersion` from CRD and store it.
7. Bot crate updated (new prompt, DB config, provider routing).
8. Game client crate updated (version-aware, common output type).
9. New Rust games implement V2 (data_docs, strategies).
10. Existing Rust games upgraded to V2 (add docs + strategies, bump
    CRD interfaceVersion to 2).
11. Go games remain V1 indefinitely until ported.

## D11: Interaction with Other Backlog Items

- **#25 (rules rendering)**: RULES.md split coordinates with #25's
  single-source render-time specialization. The split here (rules /
  basic strategy / advanced strategy) is the source; #25 decides how
  to present them to humans in web/email.
- **#47 (concede -> bot replacement)**: a conceding player's slot gets
  a `game_bot_id` with a `bot_name`. The bot plays out the rest. Full
  placings/ratings design deferred.
- **#31 (Rust-only repo)**: V1 deprecation aligns with Go stack
  removal. Once all games are Rust (V2), the V1 placeholder logic in
  game_client is deleted.

## D12: Future Admin GUI (Design-For, Not Build)

The schema supports a future admin GUI that can:
- CRUD bots (name, order, enable/disable, strategy flags, temperature)
- CRUD providers (name, URL, encrypted API key, enable/disable)
- CRUD bot_providers (assign models to bots with priority, enable/disable)
- Reorder bots (display_order)
- Test a provider (send a trivial completion request)

No GUI is built in this work item. The schema and bot-crate reads are
the foundation.
