# WP-45: bot-slot validation choke point

**Findings:** wd F27 (major), wfe F18 (major). **Decision:** D-8 answered option
C - validate at all entry points **and** at game start. Reconciled with D-5:
**validate on write, tolerate on read.**

**Landing order:** WP-41 must land first (shared `db.rs`).

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

**Naming trap** (documented atop `rust/web/src/proposals.rs`): in `BotSlot`,
`name` is the **display name**, `bot_name` is the **bot type** that must exist
in the `bots` table. Do not swap them.

## 1. Problem

- **wd F27** - `create_proposal` and `add_proposal_player`
  (`rust/web/src/proposals.rs`) and `restart_core`
  (`rust/web/src/game/server_fns.rs`) take client-supplied `BotSlot { name,
  bot_name }` and insert it as an auto-`"accepted"` slot with no check against
  the enabled-bots list. `start_proposal_tx` then feeds those strings into game
  creation, which also stores them unvalidated.
- **wfe F18** - `classify_opponent` (`rust/web/src/email/commands.rs`) returns
  `OpponentToken::Bot(inner)` for **any** `bot:`-prefixed token without
  consulting `bot_names`; the bare-token branch below it does check. So
  `new chess bot:garbage` by email creates a game with a nonexistent bot.
  Either way the game wedges on that bot's turn forever.

## 2. Why it's wrong

- **wd F27 is correct as written.** Verified live: all three call sites pass
  `bot.name` / `bot.bot_name` straight into `insert_proposal_player` /
  `CreateGameSeed.bot_slots`; `db::find_enabled_bots` is never called there.
- **wfe F18 is correct as written.** Verified live in `classify_opponent`: the
  `strip_prefix("bot:")` branch returns early, before the
  `bot_names.iter().any(...)` check that guards bare tokens.
- The wedge *recovery* is WP-38's job (D-5) - context only, do not touch
  turn-time behaviour here.

## 3. Required end state

### 3a. `rust/web/src/db.rs` - one shared validator

Add one function next to the existing `find_enabled_bots`:
`validate_bot_slots(executor, bot_slots: &[BotSlot]) -> Result<Option<String>>`

- Returns `Ok(None)` when every slot is valid, `Ok(Some(user_facing_message))`
  otherwise - mirroring the existing `roster_error` shape already used by
  `create_proposal` and `restart_core`, so call sites stay uniform.
- Rules: `bot.name` non-empty after trimming; `bot.bot_name` must match an entry
  from `find_enabled_bots` **case-insensitively** (the email path lowercases its
  token, so exact matching would wrongly reject). The message names the
  offending value and lists the valid bot types.
- Callable both with a `&PgPool` and with an in-transaction `&mut PgConnection`
  (`start_proposal_tx` has only the latter): make it generic over
  `sqlx::Executor<'_, Database = Postgres>`, adding a `_tx` variant only if the
  generic form does not compile cleanly.

### 3b. Call sites - validate on WRITE

Five functions call the validator and return its message as a user error before
any insert: `create_proposal`, `add_proposal_player` and `start_proposal_tx` in
`rust/web/src/proposals.rs`; `restart_core` in
`rust/web/src/game/server_fns.rs`; `run_new_command` in
`rust/web/src/email/commands.rs`.

- `create_proposal` / `restart_core`: beside the existing `roster_error` check,
  before `pool.begin()` / before any insert.
- `add_proposal_player`: in the `else if let Some(bot) = bot` arm, before
  `insert_proposal_player` - validate a one-element slice.
- `run_new_command`: after the `for tok in &opponent_tokens` loop has built
  `bot_slots`, before `check_duplicate_players`; reject with
  `CommandError::User`. `find_enabled_bots` is already fetched there - the
  validator refetching it is fine, do **not** hand-roll a second check.
- `start_proposal_tx`: after `bot_slots` is assembled from the accepted proposal
  rows, before `create_game_from_service`. Invariant backstop; game start counts
  as write time, so rejecting here is correct.
- `restart_core` is reached by both `restart_game_with_roster` and the email
  restart path, so one check covers both. It rebuilds slots from a finished
  game's persisted bots - rejecting a now-disabled one is intended feedback.

### 3c. Tolerate on READ

Do **not** add validation to any path that merely reads or replays an
already-persisted bot slot: game rendering, `get_restart_prefill`, the bot-turn
consumer in `rust/web/src/game/mod.rs`, `game/import.rs`. A bot disabled *after*
the game exists must fall into D-5's dangling-name no-op plus admin warning,
never a rejection or a panic.

## 4. Non-goals

- The bot-turn wedge-recovery gap (WP-38 / D-5) - context only.
- Bot replacement/`can_replace_humans`, bot difficulty semantics, the `bots`
  admin CRUD, the `db.rs` split (ws F42), renaming `BotSlot`'s fields.

## 5. Regression test cases

- `rust/web/src/db.rs` `#[cfg(test)] mod tests`: `validate_bot_slots` accepts an
  enabled bot, accepts a case-mismatched enabled bot, rejects an unknown type, a
  disabled (`enabled = false`) type and an empty/whitespace display `name`, and
  accepts an empty slice. Insert fixtures with `INSERT INTO bots (name, ...)`
  as the existing db tests do.
- `rust/web/src/proposals.rs` `#[cfg(test)] mod tests`: `start_proposal_tx` on a
  proposal whose stored bot type was disabled after creation fails rather than
  creating a wedged game. (`create_proposal` / `add_proposal_player` are
  `#[server]` fns needing leptos context - if the existing test module cannot
  call them, cover those two via `rust/web/tests/ssr_pages.rs` instead.)
- `rust/web/src/game/server_fns.rs` `#[cfg(test)] mod tests`: extend the
  existing `edited_roster_*` restart tests - a bogus bot in the edited roster is
  rejected and creates no new game.
- `rust/web/src/email/commands.rs` `#[cfg(test)] mod tests`:
  `new <type> bot:garbage` returns a user error naming the valid bots and
  creates no game; `new <type> bot:<enabled>` still succeeds.

## 6. Riders

None - both findings are major and in scope above.
