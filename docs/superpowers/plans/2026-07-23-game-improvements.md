# Game Improvements Batch (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Seven independent improvements across the game layer and web sidebar.

---

## 1. Overview

| # | Item | Scope | Size |
|---|------|-------|------|
| 1 | Active games sorting | `rust/web/src/db.rs` | S |
| 2 | Farkle + Greed scoring tables | two game renders | S |
| 3 | Red7 log spaces + orange colour | `rust/game/red7-1` | S |
| 4 | Splendor take partials | `rust/game/splendor-2/src/command.rs` | XS |
| 5 | Tic Tac Toe log + player labels | `rust/game/tic-tac-toe-2` | S |
| 6 | Placings log for all 27 games | all game crates | L |
| 7 | Sushizock render table fix | `rust/game/sushizock-2/src/render.rs` | S |

---

## 2. Per-item detail

### 2.1 Active games sorting

**Current behaviour:**
`find_active_game_summaries` (`db.rs:583`) orders by:
```sql
ORDER BY me.is_turn DESC, g.updated_at DESC, g.id, opp.position
```
My-turn games come first (good) but within each group the secondary sort is
`g.updated_at` (game-level, not player-level). The user wants:
- MY-TURN games: `is_turn_at ASC` (longest-waiting first).
- NON-my-turn games: `last_turn_at DESC` (most recent own turn first).

**Relevant schema:**
- `game_players.is_turn_at` - trigger `update_is_turn_at` fires `BEFORE UPDATE
  WHEN old.is_turn = false AND new.is_turn = true`, stamps `now()`.
- `game_players.last_turn_at` - trigger `update_last_turn_at` fires `BEFORE
  UPDATE WHEN old.is_turn = true AND new.is_turn = false`, stamps `now()`.
- Both already exist on the `me` alias (the viewer's own player row).

**Files:** `rust/web/src/db.rs:587-607` (the `sqlx::query!` block).

**Change:**
Replace the ORDER BY with:
```sql
ORDER BY
    me.is_turn DESC,
    CASE WHEN me.is_turn THEN me.is_turn_at END ASC,
    CASE WHEN NOT me.is_turn THEN me.last_turn_at END DESC,
    g.id, opp.position
```
The `CASE` ensures each sub-sort only applies within its group. `g.id,
opp.position` remain as deterministic tiebreakers.

No struct changes needed - `GameSummary` already carries `is_turn_at`
(`server_fns.rs:29`) and the sidebar only uses it for the "Next game" button;
the sort order is what matters.

**Acceptance:**
- My-turn games appear longest-waiting first.
- Non-my-turn games appear most-recent-own-turn first.
- Deterministic ordering (no flicker on refresh).

**Tests:**
- `#[sqlx::test]` in `db.rs`: create a game with 2 players, manipulate
  `is_turn`/`is_turn_at`/`last_turn_at` via direct UPDATEs, assert the
  returned order from `find_active_game_summaries`.

---

### 2.2 Farkle and Greed scoring tables

**Current behaviour:**
Neither `farkle-2/src/render.rs` nor `greed-2/src/render.rs` shows a scoring
reference. The render ends after the player-score table.

**Pattern to follow:**
`love-letter-2/src/render.rs:132-153` (`help_table()`) builds a `Vec<Row>`
with a header row + one row per entry, then the main `render()` appends it via
`table_with_gap(&help_table(), COL_SPACING)` inside the outer `N::Table`.

**Files:**
- `rust/game/farkle-2/src/render.rs` (67 lines)
- `rust/game/greed-2/src/render.rs` (67 lines)
- Scoring data: `farkle-2/src/lib.rs:47-82` (`SCORES`), `greed-2/src/lib.rs:77-140` (`SCORES`)

**Change (Farkle):**
Add a `fn scoring_table() -> Vec<Row>` in `render.rs`:
```
Header: "Combination" (A::Left) | "Points" (A::Right)
Rows (from SCORES, display order):
  Single 1       | 100
  Single 5       |  50
  Three 1s       | 1000
  Three 2s       |  200
  Three 3s       |  300
  Three 4s       |  400
  Three 5s       |  500
  Three 6s       |  600
```
Left column right-aligned (`A::Right`), right column right-aligned.
Append in `PubState::render()` after the player table:
```rust
out.push(N::text("\n\n"));
out.push(table_with_gap(&scoring_table(), 2));
```

**Change (Greed):**
Same pattern. Scoring table:
```
Header: "Combination" (A::Left) | "Points" (A::Right)
Rows:
  Six of any     | 5000
  Four Ds        | 1000
  Straight ($GeEeD) | 1000
  Three $        |  600
  Three G        |  500
  Three R        |  400
  Three E/e      |  300
  Single D       |  100
  Single G       |   50
```
Use `render_die()` for the die-face column entries where appropriate (coloured
die glyphs). The "Combination" column can use plain text descriptions with
inline coloured die nodes.

**Acceptance:**
- Scoring table visible at the bottom of every Farkle/Greed game render.
- Table is neatly aligned (left column right-aligned, points right-aligned).
- No change to game logic.

**Tests:**
- Unit test: call `PubState::render()` on a default state, assert the output
  contains the expected scoring text nodes (e.g. "1000", "Single 1").

---

### 2.3 Red7 log readability + orange colour

**Current behaviour:**
- "You drew" log (`lib.rs:128-139`): builds `card_nodes` by mapping each card
  to `N::Fg(color, vec![N::Bold(...)])` and extends `content` directly - NO
  separator between cards. Result: "You drew R5Y4Y1G7G3B6V1".
- "Won round" log (`lib.rs:182-193`): same pattern, no separators.
- `render_cards()` (`render.rs:15-27`) already inserts `N::text(" ")` between
  cards and sorts them - but it is only used in the board render, not in logs.
- Orange suit maps to `NamedColor::Grey` (`card.rs:71`).

**Files:**
- `rust/game/red7-1/src/lib.rs:128-139` (drew log)
- `rust/game/red7-1/src/lib.rs:182-193` (won-round log)
- `rust/game/red7-1/src/render.rs:15-27` (`render_cards`)
- `rust/game/red7-1/src/card.rs:71` (orange colour)

**Change (log spaces):**
In both log sites, replace the manual `card_nodes` construction with a call to
`render::render_cards(&display)` (drew) / `render::render_cards(&sorted_pal)`
(won-round). `render_cards` already sorts, reverses, and inserts spaces. The
`render` module is already imported in `lib.rs` (used by `render.rs`'s
`Renderer` impl). If `render_cards` is not `pub`, make it `pub(crate)`.

Alternatively (if import direction is awkward): insert `N::text(" ")` between
card nodes in the existing loop, matching the `render_cards` pattern:
```rust
for (i, c) in display.iter().enumerate() {
    if i > 0 {
        card_nodes.push(N::text(" "));
    }
    card_nodes.push(N::Fg(...));
}
```

**Change (orange colour):**
`card.rs:71`: change `Suit::Orange => NamedColor::Grey` to
`Suit::Orange => NamedColor::Orange`. `NamedColor::Orange` exists in the
palette (`rust/lib/color/src/palette.rs:17`).

**Acceptance:**
- Drew log reads "You drew R5 Y4 Y1 G7 G3 B6 V1" (spaces between cards).
- Won-round log similarly spaced.
- Orange cards render in orange, not grey.

**Tests:**
- Unit test in `lib.rs`: play a draw action, assert the private log content
  contains `N::text(" ")` nodes between card nodes.
- Unit test in `card.rs`: assert `Suit::Orange.color() == NamedColor::Orange`.

---

### 2.4 Splendor take command partials

**Current behaviour:**
`token_parser` (`command.rs:157-165`) uses `Enum::exact(values)`. Players must
type the full resource name (e.g. "diamond") for autocomplete to match.

**Files:** `rust/game/splendor-2/src/command.rs:164`

**Change:**
```rust
// Before:
Map::new(Enum::exact(values), |c: TokenChoice| c.resource)
// After:
Map::new(Enum::partial(values), |c: TokenChoice| c.resource)
```
`Enum::partial` (`rust/lib/game/src/command/parser/mod.rs:570`) matches on
unique prefix. "d" matches "diamond", "s" matches "sapphire", "e" matches
"emerald". Ambiguous prefixes (if any) still require more characters.

Note: the `Many::some_spaced` delimiter suggest fix (Alhambra A6, tracked
separately in `docs/superpowers/plans/2026-07-23-alhambra.md`) will fix 2nd+ token
autocomplete. This change only enables prefix matching for each individual
token.

**Acceptance:**
- `take d s e` parses as take diamond, sapphire, emerald.
- `take diamond sapphire emerald` still works (full names match).
- Ambiguous prefixes are rejected with a helpful error.

**Tests:**
- Unit test in `command.rs`: parse "take d s e" and assert the resulting
  `Command::Take` contains `[Diamond, Sapphire, Emerald]`.
- Unit test: parse "take di" (ambiguous if "diamond" is the only d-word) works.

---

### 2.5 Tic Tac Toe log + player labels

**Current behaviour:**
- `play()` (`lib.rs:87-108`) returns `Ok(vec![])` - no log entry for a move.
- `render.rs:7-28` shows bare `x`/`o` glyphs with no indication of which
  player is which.
- `PubState.start_player` (`lib.rs:65`) records who goes first (plays as X).

**Files:**
- `rust/game/tic-tac-toe-2/src/lib.rs:101-107` (play method)
- `rust/game/tic-tac-toe-2/src/render.rs:30-34` (`Renderer for PubState`)

**Change (log):**
In `play()`, before `Ok(vec![])`, build and return a log:
```rust
let mark = if player == self.start_player { "X" } else { "O" };
Ok(vec![Log::public(vec![
    N::Player(player),
    N::text(" played "),
    N::Bold(vec![N::text(mark.to_string())]),
    N::text(" at "),
    N::Bold(vec![N::text(loc.to_string())]),
]])])
```
`Loc` implements `Display` (renders as a letter a-i, `lib.rs:35-41`).

**Change (player labels):**
In `render.rs`, after the board nodes, append a label line:
```rust
impl Renderer for PubState {
    fn render(&self) -> Vec<N> {
        let mut nodes = render_board(&self.board);
        nodes.push(N::text("\n"));
        let x_player = self.start_player;
        let o_player = 1 - self.start_player;
        nodes.push(N::Player(x_player));
        nodes.push(N::text(" is X, "));
        nodes.push(N::Player(o_player));
        nodes.push(N::text(" is O"));
        nodes
    }
}
```
Same for `PlayerState::render()` (delegates to `self.public`).

**Acceptance:**
- Each play produces a public log: "{Player} played X at c".
- Board render shows "{Player1} is X, {Player2} is O" below the grid.
- X/O assignment matches `start_player`.

**Tests:**
- Unit test: call `play()`, assert returned logs are non-empty and contain the
  player index and location.
- Unit test: render a `PubState` with `start_player = 1`, assert the label
  line maps player 1 to X and player 0 to O.

---

### 2.6 Show placings log for ALL games (per-game)

**Current behaviour:**
When a game finishes, `Status::Finished { placings, stats }` is computed and
stored, but NO log entry announces the result. Players must look at the
sidebar or game state to see who won.

**Design decision:** PER-GAME (not monolith-injected). Each game's engine adds
a final `Log::public` with placings + scores when the game transitions to
Finished. This keeps the log in the game's own narrative voice and allows
game-specific tiebreaker explanations.

**Pattern:**
In each game, at the point where the game detects it has finished (inside the
`command()` handler or the internal method that sets the finished flag), append:
```rust
if self.finished() {
    logs.push(Log::public(vec![/* placings announcement */]));
}
```

**Log content format (standard):**
```
{N::Player(winner)} wins! (or "It's a tie!" / "{P1} and {P2} tie for 1st")
```
Plus scores from `Status::Finished.stats` when available:
```
Final scores: {P1}: 42, {P2}: 38, {P3}: 35
```
For games with tiebreakers, explain: "{P1} wins on tiebreaker (fewer cards
remaining)".

**Helper (optional but recommended):**
Add a shared helper in `rust/lib/game/src/game_log.rs` or a new
`rust/lib/game/src/placings_log.rs`:
```rust
pub fn placings_log(placings: &[usize], scores: Option<&[i32]>) -> Log {
    // Build "X wins!" or "X and Y tie!" + optional score line
}
```
Games with special tiebreaker text can build their own instead.

**All 27 games (grouped for implementation):**

Sub-group A - simple score-based (placings from score, stats often empty):
1. `farkle-2` - scores in `self.scores`, finish in `done()`/`bust()`
2. `greed-2` - same pattern as farkle
3. `no-thanks-2` - finish when `remaining_cards.is_empty()`, in `take()`
4. `sushi-go-2` - finish after round 3 scoring
5. `sushizock-2` - finish when tiles exhausted
6. `category-5-2` - finish when cards exhausted
7. `zombie-dice-2` - finish at score threshold
8. `age-of-war-2` - finish when all dice placed
9. `roll-through-the-ages-2` - finish at victory condition
10. `for-sale-2` - finish when all cards played
11. `liars-dice-2` - finish when one player remains
12. `texas-holdem-2` - finish at showdown/fold

Sub-group B - placement/tiebreaker-heavy:
13. `splendor-2` - finish at 15+ points, tiebreak: fewest turns
14. `jaipur-2` - finish at 2 round wins
15. `love-letter-2` - finish at leader_points threshold
16. `modern-art-2` - finish after 4 rounds, complex scoring
17. `seven-wonders-1` - finish after 3 ages, multi-category scoring
18. `alhambra-1` - finish when tiles exhausted, multi-category scoring
19. `starship-catan-1` - finish at victory points
20. `lords-of-vegas-1` - finish when all properties claimed

Sub-group C - elimination/positional:
21. `red7-1` - finish when one player remains (already logs round wins)
22. `battleship-2` - finish when a fleet is sunk
23. `cathedral-2` - finish when a player can't place (already logs finish)
24. `tic-tac-toe-2` - finish on win/draw
25. `acquire-1` - finish at merger/end condition
26. `lost-cities-1` - finish when deck exhausted
27. `lost-cities-2` - finish when deck exhausted

**Per-game implementation notes:**
- Find the transition point: search for where `self.finished()` /
  `self.is_finished()` first becomes true after a command. Usually inside the
  method called by `command()` (e.g. `done()`, `take()`, `play()`).
- Append the placings log AFTER the last gameplay log, BEFORE returning.
- Use `N::Player(idx)` for player references (renders as the player's name).
- Include scores when the game tracks them (`self.scores`, `self.points()`,
  etc.).
- For tiebreakers: add a brief explanation node (e.g. "wins on tiebreaker:
  fewer turns taken").

**Acceptance:**
- Every game produces a final public log announcing the winner(s) and scores.
- Ties are clearly stated ("It's a tie between X and Y!").
- Tiebreakers are explained where relevant.
- The log appears in the game's chat/log feed immediately after the final move.

**Tests (per game):**
- Unit test: play a game to completion, assert the last log in the returned
  `Vec<Log>` contains the winner announcement.
- For tiebreaker games: test a tied scenario and assert the tiebreaker text.

---

### 2.7 Sushizock render misalignment

**Current behaviour:**
`render.rs:86-121`: dice faces and position labels are in separate flat
`N::Group` nodes. The `.game-render` CSS has `text-align: center`, so each
line centres independently. Multi-character die faces (e.g. the sushi
symbol) have different widths than single-digit labels, causing misalignment.

```
  Θ  X  ¥  X  Θ      <- dice (N::Group, centred)
  1  2  3  4  5      <- labels (N::Group, centred independently)
```

**Files:** `rust/game/sushizock-2/src/render.rs:86-121`

**Change:**
Replace the two flat `N::Group` nodes with a single `table_with_gap` where
each die is its own column:

```rust
// Build one Row for dice, one Row for labels
let dice_row: Row = pub_state.rolled_dice.iter().enumerate().map(|(i, d)| {
    (A::Center, vec![N::Bold(vec![die_node(*d)])])
}).collect();
let label_row: Row = pub_state.rolled_dice.iter().enumerate().map(|(i, _)| {
    (A::Center, vec![N::Fg(NamedColor::Grey.into(), vec![N::text((i + 1).to_string())])])
}).collect();
out.push(table_with_gap(&[dice_row, label_row], 2));
```

For kept dice (which have no labels), keep them as a separate group or add
them as additional columns with empty label cells.

The `table_with_gap` import already exists in this file (line 3).

**Acceptance:**
- Position labels align directly under their corresponding dice regardless of
  die-face character width.
- Kept dice still render correctly (no labels needed under them).
- No regression in the tiles section below.

**Tests:**
- Unit test: render a `PubState` with 5 rolled dice, assert the output
  contains an `N::Table` node (from `table_with_gap`) rather than two
  separate `N::Group` nodes for dice+labels.

---

## 3. Implementation units (grouped by dependency)

All units are independent of each other (no cross-dependencies). Each unit
ends with fmt + clippy green and its own commit.

### G1. Active games sorting
- **Files:** `rust/web/src/db.rs` (ORDER BY in `find_active_game_summaries`)
- **Scope:** single query change + one `#[sqlx::test]`
- **Risk:** low - pure sort change, no schema migration needed
- **Gate:** `cargo clippy -p web --all-targets --features ssr -- -D warnings`

### G2. Farkle + Greed scoring tables
- **Files:** `rust/game/farkle-2/src/render.rs`, `rust/game/greed-2/src/render.rs`
- **Scope:** add `scoring_table()` fn + append in `render()`
- **Risk:** low - additive render change, no game logic touched
- **Gate:** `cargo clippy -p farkle-2 -p greed-2 --all-targets -- -D warnings`

### G3. Red7 log spaces + orange colour
- **Files:** `rust/game/red7-1/src/lib.rs`, `rust/game/red7-1/src/card.rs`
- **Scope:** two log sites + one colour mapping
- **Risk:** low - cosmetic
- **Gate:** `cargo clippy -p red7-1 --all-targets -- -D warnings`

### G4. Splendor take partials
- **Files:** `rust/game/splendor-2/src/command.rs` (one word change)
- **Scope:** `Enum::exact` -> `Enum::partial`
- **Risk:** low - parser already supports partial; verify no ambiguous prefixes
  in the Splendor resource names (diamond, sapphire, emerald, ruby, onyx, gold
  - all have unique first letters except none conflict)
- **Gate:** `cargo clippy -p splendor-2 --all-targets -- -D warnings`

### G5. Tic Tac Toe log + labels
- **Files:** `rust/game/tic-tac-toe-2/src/lib.rs`, `rust/game/tic-tac-toe-2/src/render.rs`
- **Scope:** return log from `play()`, add label line in render
- **Risk:** low - additive
- **Gate:** `cargo clippy -p tic-tac-toe-2 --all-targets -- -D warnings`

### G6. Placings log for all 27 games
- **Files:** all 27 game crates' `lib.rs` (the `command()` handler or the
  internal finish method)
- **Scope:** largest unit. Recommend splitting into 3 sub-commits by the
  sub-groups above (A: 12 games, B: 8 games, C: 7 games).
- **Optional shared helper:** `rust/lib/game/src/placings_log.rs` - a
  `placings_log(placings, scores)` fn that builds the standard announcement.
  Games with special tiebreaker text override with custom content.
- **Risk:** medium - touches many crates, but each change is small and
  isolated. Main risk: missing a game or duplicating an existing finish log
  (cathedral already logs "The game is finished..." - check and align).
- **Gate:** `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
- **Note:** cathedral-2 already has a finish log (`lib.rs:1024` test asserts
  "The game is finished, remaining piece size is as follows:"). Either replace
  it with the standard placings format or keep it and ADD the placings line.
  Recommend: keep the existing detail log, add a separate placings summary.

### G7. Sushizock render table fix
- **Files:** `rust/game/sushizock-2/src/render.rs:86-121`
- **Scope:** replace flat groups with `table_with_gap`
- **Risk:** low - render-only change
- **Gate:** `cargo clippy -p sushizock-2 --all-targets -- -D warnings`

---

## 4. Gotchas

- **Migrations are immutable.** G1 needs NO migration (both `is_turn_at` and
  `last_turn_at` already exist on `game_players`). If any unit needs a schema
  change, next migration number is **021**.
- **SQLX_OFFLINE=true for clippy/check.** G1 changes a `sqlx::query!` macro
  invocation - the ORDER BY change does NOT alter the result columns, so no
  `.sqlx` cache regeneration is needed. If it did, run
  `cargo sqlx prepare -p web`.
- **clippy `--all-targets` gate is mandatory** for every commit.
- **DB tests need real Postgres.** Use `scripts/rust-test.sh` or the
  long-lived `brdgme-test-{pg,nats}-47116` containers.
- **`games.updated_at` is trigger-maintained** - irrelevant here (no game
  state changes in G1), but G6's placings log is part of the command response
  and does NOT trigger an extra `UPDATE games`.
- **`render_cards` visibility (G3):** `render_cards` in `red7-1/src/render.rs`
  is currently module-private (`fn render_cards`). If `lib.rs` calls it
  directly, it needs `pub(crate)`. Alternatively, inline the space-insertion
  in `lib.rs` to avoid the visibility change.
- **Greed die display (G2):** `Die::E1` and `Die::E2` both display as "E"/"e"
  (`name()` at `lib.rs:48-57`). The scoring table should show "Three E/e" as
  a single row (they score identically at 300).
- **Splendor resource names (G4):** verify all `TokenChoice` `to_string()`
  values have unique first characters. Current resources: Diamond, Sapphire,
  Emerald, Ruby, Onyx, Gold. "D", "S", "E", "R", "O", "G" - all unique. Safe
  for single-char partials.
- **G6 ordering:** the placings log must be the LAST log in the returned
  `Vec<Log>` so it appears at the bottom of the game feed. Append it after all
  other gameplay logs in the finishing command.
- **G6 undo interaction:** if a game supports undo (`can_undo: true`), the
  placings log is part of the undone state and will be removed with the undo.
  This is correct behaviour - no special handling needed.
- **Org is `brdgme`** (not `beefsack`) for any image/URL references.
- **Target single packages** for cargo work: `cargo clippy -p <crate>`.
  Never workspace-wide builds (except the `--workspace --exclude web` clippy
  gate).
