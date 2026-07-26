# WP-81: stats deletions

**Findings:** `c F12`, `e F39`, `e F40`. **Decision:** D-40 ANSWERED - **option
B, delete the dead stats machinery**; split out of WP-20/WP-30 so it lands ahead
of the rules review. **Status:** READY.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising. No line numbers are
> cited on purpose - the tree is under concurrent edit.**

**This is NOT a design statement about stats.** Michael wants game-specific
stats revisited later **from a clean slate**. Deletion is right *now* precisely
because there is no platform consumption path to wire into, and a future feature
must not inherit this shape. **Do not substitute option A (wiring them up).**

## 1. Findings verdicts - state these so nobody reverts them

- **`c F12` CORRECT.** `Stats::to_brdgme_stats` in
  `rust/game/acquire-1/src/stats.rs` has **zero callers workspace-wide**
  (confirmed by grepping the whole `rust/` tree for the symbol), and
  `Gamer::status`'s `Status::Finished` arm returns `stats: vec![]`. Every field
  of acquire-1's `Stats` is therefore **written but never read**. Its
  "either wire it up or delete it" recommendation is resolved as delete.
- **`e F39` CORRECT.** In *both* lost-cities-1 and lost-cities-2, `Stats`
  declares `investments`, which is **never touched** - no write, no read,
  anywhere in `rust/` (the only other `investments` hit in the tree is prose in
  `splendor-2/ADVANCED_STRATEGY.md`). `Stats.expeditions` is **written once,
  never read**. Its recommendation (start incrementing `investments`) is option
  A and is **not** applied.
- **`e F40` CORRECT, and it is the same code as the `expeditions` half of
  `e F39`.** The increment sits in `Game::play` and fires only when the player's
  **entire** `self.expeditions[player]` card vec is empty. That vec is a flat
  list of every card the player has played, reset per round in the round-start
  path - so the counter counts **rounds in which the player played at least one
  card**, not expeditions started. Deleting the field deletes the wrong
  increment; there is no separate third edit.

## 2. Deletion 1 - acquire-1

Files: `rust/game/acquire-1/src/stats.rs`, `rust/game/acquire-1/src/lib.rs`.

- **Delete `src/stats.rs` entirely** - both `Stats` and its `to_brdgme_stats`.
  The struct is not merely emptied, it becomes **entirely unused**, so the file
  goes rather than being left as a husk. Its `use std::collections::HashMap`,
  `use serde::{...}`, `use brdgme_game::Stat` and `use crate::Corp` go with it.
- In `lib.rs`: delete `mod stats;`, `use crate::stats::Stats;`, the
  `pub stats: Stats` field on `struct Player`, and `stats: Stats::default()` in
  `impl Default for Player`.
- Delete every `self.players[..].stats.<field> += ..` / `.push(..)` statement.
  They live in `handle_found_command` (`founds.push`), `handle_buy_command`
  (`buy_sum`, `buys`), `handle_merge_command` (`merges`), `pay_bonuses`
  (`major_bonus_sum`, `major_bonuses`, `minor_bonus_sum`, `minor_bonuses`),
  `sell` (`sell_sum`, `sells`) and `handle_trade_command` (`trades`,
  `trade_loss_sum`, `trade_gain_sum`). **Read each named function; delete only
  the stats statement**, not the surrounding gameplay.
- **Leave `status()`'s `stats: vec![]` exactly as it is.** That is
  `brdgme_game`'s field, not this crate's.
- Collateral verified: `Corp::name()` is used by `board.rs` and `render.rs`, so
  it stays. No Cargo dependency is exclusive to `stats.rs` (`serde`,
  `brdgme_game` are used throughout) - **no manifest change**.
- Dropping a field from a `Deserialize` struct is safe for persisted states:
  serde ignores unknown fields by default. No migration shim needed.

## 3. Deletion 2 - lost-cities-1 and lost-cities-2

File: `rust/game/lost-cities-{1,2}/src/lib.rs`. Identical change in both.

- From `struct Stats`, delete **only** `investments` and `expeditions`. The
  struct **survives**: `plays`, `discards`, `takes`, `draws` and `turns` are all
  genuinely read by `Gamer::player_stats`, which *is* wired into
  `Status::Finished { stats: .. }`. **Do not delete `Stats`, `Game.stats`,
  `player_stats`, or the `brdgme_game::Stat` import in these two crates.**
- In `Game::play`, delete the `if <player expedition>.is_empty() { ...
  stats[player].expeditions += 1; }` block. **Collateral, verify per crate:**
  the fallible lookup that feeds that `if` exists solely to serve it, and the
  code immediately below re-does the same lookup via `get_mut(player)` with its
  own error. So the binding/closure above it also goes -
  in lost-cities-2 that is the `let player_expedition = self.expeditions
  .get(player).ok_or_else(..)?;` binding; in lost-cities-1 it is the
  `let invalid_expedition = || { .. };` closure plus the `self.expeditions
  .get(player).ok_or_else(invalid_expedition)?.is_empty()` test. Confirm the
  second lookup is present before removing the first; if it is not, STOP.
- `self.stats[player].plays += 1;` later in the same function **stays**.

## 4. Scope guard

- **No gameplay or scoring change.** Nothing here is read by any rule.
- **Do not touch any `RULES.md`.** WP-20 (acquire-1) and WP-30 (lost-cities)
  remain `BLOCKED-ON-USER-RULES-REVIEW` for their rules halves; WP-81 is only
  their stats half.
- **Landing-order collision:** WP-19 (READY) carries `c F11`, "Trades stat
  reports merges", whose only subject is a line inside `stats.rs:to_brdgme_stats`.
  **WP-81 makes `c F11` moot.** Land WP-81 first and drop `c F11` from WP-19; if
  WP-19 lands first, its one-line fix is simply deleted here. **Whichever lands
  second must not resurrect `stats.rs`.**

## 5. Verification

Read-only, after the change - each must return **zero** hits:

- `grep -rn 'to_brdgme_stats' rust/`
- `ls rust/game/acquire-1/src/stats.rs` -> no such file;
  `grep -rn 'mod stats\|stats::Stats\|\.stats\.' rust/game/acquire-1/`
- `grep -rn 'investments' rust/game/lost-cities-1 rust/game/lost-cities-2`
- `grep -rn '\.expeditions += ' rust/`

Unchanged (guards against over-deletion):

- `grep -rn 'fn player_stats' rust/game/lost-cities-1 rust/game/lost-cities-2`
  -> one hit each.
- `grep -rc 'stats\[player\]' rust/game/lost-cities-{1,2}/src/lib.rs` -> the
  `plays`/`discards`/`takes`/`draws`/`turns` sites remain.

When implementing (legitimate builds - AGENTS.md forbids workspace-wide builds
on dev machines, so use `-p`):

- `cargo clippy -p acquire-1 --all-targets`, `cargo test -p acquire-1`
- `cargo clippy -p lost-cities-1 --all-targets`, `cargo test -p lost-cities-1`
- `cargo clippy -p lost-cities-2 --all-targets`, `cargo test -p lost-cities-2`

**A deletion's real test is that clippy's dead-code lints go quiet and the
existing tests still pass. Do not add new tests for removed code.**
