# Current Status

## Active work: bot system prompt documentation

Writing static markdown documentation for the command spec grammar to embed in the
bot system prompt, so the LLM can reliably produce valid plain text commands.

### What was done this session

- Added `RULES.md` for Acquire and implemented `Gamer::rules()` in `acquire-1/src/lib.rs`
  to include it via `include_str!`. First end-to-end test with rules context returned
  HTTP 200 after ~25 min (20 Ollama attempts).
- Fixed bot log visibility: `RUST_LOG=info` was missing from the bot `serve_cmd` in
  the Tiltfile. All `tracing::info!` calls were silently dropped. Now visible in Tilt.
- Identified that the bot was generating raw JSON command spec output instead of plain
  text commands. Root cause: the user prompt fed the raw JSON spec to the LLM with no
  explanation of how to interpret it.
- Queried the acquire-1 service for the current game state (game
  `63cd2468-a5d1-4f41-ad53-ba25c7390ccd`, player 1) and saved the full response to
  `docs/acquire1_status.json`.
- Extracted the command spec for player 1 and converted to YAML, saved to
  `docs/command_spec.yaml`. This is the `Buy` phase spec (buy shares or done).
- Created `rust/bot/system_prompt.md` as a static markdown file for the bot system
  prompt. Dynamic game context (render, logs, command spec) will be appended separately
  at runtime. Current content is the static preamble only.
- Next step: write command spec node documentation in `system_prompt.md` so the LLM
  understands how to read a grammar tree and produce a valid plain text command.

### Completed unstaged work (pre-existing, not yet committed)

#### Bot service (`rust/bot/`)

- Bot crate created at `rust/bot/` and added to the workspace (`rust/Cargo.toml`).
- `rust/Dockerfile` extended with a `bot` build target.
- `k8s/base/bot/` manifests created; added to `k8s/base/brdgme/kustomization.yaml`.
- Bot runs as a local Tilt resource (hybrid mode) with mirrord for cluster DNS access,
  and as an in-cluster Knative Service in `WEB_IN_CLUSTER=1` mode.
- `INTERNAL_API_KEY`, `MONOLITH_URL`, `OLLAMA_URL`, `BOT_MODEL` wired via env / Secret.

#### Bot trigger integration in web (`rust/web/`)

- `trigger_bot_turns` wired into `create_game`, `undo_game`, `concede_game`,
  `restart_game` in `server.rs`.
- `trigger_bot_turns` wired into `concede_game`, `restart_game`, `create_new_game`
  server fns in `server_fns.rs`.
- `BumpBotTurns` server fn added; "Bump bot to play" link shown in `GameMeta` actions
  panel when any bot player has `is_turn = true`. Auth-gated to game players only.
- `is_bot: bool` added to `PlayerViewData`; populated in both `server_fns.rs` and
  `websocket.rs`.
- Bot stale-turn race condition handled (two layers): `is_turn` re-checked from DB
  before submitting to Ollama, and again after Ollama responds.

#### Bot game creation UI (`rust/web/src/app.rs`)

- Per-opponent `OpponentSlot` enum: `Human(email)` or `Bot { name, difficulty }`.
- New game form updated with Human/Bot toggle per opponent slot, name and difficulty
  fields for bot slots.
- `opponent_emails` and `bot_slots` passed as `Option<Vec<_>>` to handle absent fields
  in URL encoding.

#### Database (`rust/web/src/db.rs`)

- `CreateGameOpts` extended with `bot_slots: &[BotSlot]`.
- `create_game_with_users` inserts `game_bots` rows and `game_players` rows with
  `game_bot_id` set and `user_id = NULL` for bot slots.

#### Infrastructure

- CRD moved out of Tilt and into `scripts/setup-kind-cluster.sh` to avoid a deadlock
  (Tilt cannot safely delete a CRD that has resources with operator finalizers while
  the operator is not yet running). `k8s/base/operator/kustomization.yaml` no longer
  references `crd.yaml`.
- `docs/DEV.md` updated: CRD ownership note and recovery procedure for a CRD stuck in
  terminating state.
- `devenv.nix`: added `poppler-utils`.

### Command spec parsers missing from command_spec.yaml

The captured `docs/command_spec.yaml` is from the `Buy` phase and only exercises 7 of
the 10 `Spec` variants. The three not present, with example YAML encodings:

#### `Opt`

Wraps an inner spec that may be omitted entirely.

```yaml
Opt:
  Token: undo
```

#### `Many`

Repeats an inner spec a number of times. `min` and `max` are nullable integers.
`delim` is a nullable inner spec used as a separator between repetitions.

```yaml
Many:
  spec:
    Token: item
  min: 1
  max: 3
  delim: null
```

With a delimiter:

```yaml
Many:
  spec:
    Enum:
      values:
        - American
        - Sackson
      exact: false
  min: null
  max: null
  delim:
    Token: ","
```

#### `Player`

A unit variant with no fields. Matches a player number (0-indexed) or player name.

```yaml
- Player
```

Or as the sole spec:

```yaml
Player
```

### Current command_spec.yaml (Buy phase, player 1)

See `docs/command_spec.yaml`. Parsers present: `OneOf`, `Chain`, `Doc`, `Space`,
`Token`, `Int`, `Enum`.

### Next steps (in order)

1. **Command spec documentation** — write documentation for every `Spec` node type in
   `rust/bot/system_prompt.md` with YAML examples and plain text command examples.

2. **Wire system prompt from markdown** — load `system_prompt.md` at runtime in the
   bot and append dynamic game context (render, logs, command spec as YAML).

3. **Switch command spec serialisation to YAML** — convert `command_spec` from JSON
   to YAML in `build_user_prompt` so it matches the documented format.

4. **Prompt restructure** (PLAN.md Phase 9) — restructure messages for Ollama KV cache
   reuse.

5. **Optimistic locking** (PLAN.md Phase 9) — `execute_command` +
   `update_game_command_success`.

6. **Rules for other games** — write `RULES.md` for lords-of-vegas-1, lost-cities-1,
   lost-cities-2 once bot is confirmed working end-to-end on Acquire.
