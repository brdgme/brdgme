# WP-09b: per-crate deserialized-state trust sweep

**Findings:** d F5, d F38, e F2, e F3, e F4, e F10, e F22, f F4, f F9, f F19,
f F26, f F29, f F34, f F44, f F46, f F53, f F55 (all minor/nit), plus one
**no-finding-id** item added at spec time: **red7-1's remaining `num_players`
trust** (routed out of the WP-29 spec's cross-package item 4). **Decision:**
D-36 answered option A. The two request-reachable majors are **WP-09a's**.

**Crates:** `rust/game/{lords-of-vegas-1, modern-art-2, love-letter-2,
age-of-war-2, lost-cities-2, zombie-dice-2, battleship-2, for-sale-2,
category-5-2, greed-2, farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2,
red7-1}`.

**Landing order:** **WP-09a lands first** - it adds the `Gamer::validate` hook
this package fills in. Do not start until it is in.

> **Read the named function before editing. If it does not match what this
> spec describes, STOP and report rather than improvising.** This tree is under
> concurrent edit; no line numbers are cited on purpose. Everything below is a
> three-to-ten-line change - if a row looks like it needs a refactor, stop.

## 1. Problem

Every game `Game` derives `Deserialize` with all-`pub` fields and rides the DB
as a JSON blob, but each crate's runtime code trusts cross-field invariants
that only `Gamer::start` establishes: that `players` is one of the crate's
legal counts, that every per-player `Vec` has exactly `players` entries, and
that stored player indices are in range. Seventeen findings across fifteen
crates are the same shape - an unchecked index, an `unreachable!()`/`expect`,
or an unguarded `usize` subtraction that panics on a state no legal play can
produce. None is reachable from player command text today.

## 2. Why it's wrong

A stored or forwarded blob is untrusted input at the requester boundary. After
WP-09a the boundary rejects a bad *player index* but nothing checks the *state*
itself, so each of these sites is one corrupt row away from panicking the game
process - and several sit in renderers, where a panic bricks the game page for
every viewer rather than failing one command.

**Verified correct as written against live code:** d F5, d F38, e F2, e F3,
e F4, e F10, e F22, f F4, f F9, f F19, f F26, f F29, f F34, f F44, f F46,
f F53, f F55, and the red7-1 item. **Already fixed: none.** **Judged
incorrect: none.** Two verification corrections are binding: greed-2 f F34's
`points()` citation is a **mis-citation** (`points()` iterates and cannot
panic) - fix only the three real sites plus `render.rs`; no-thanks-2 f F53
**undercounts** - the renderer's `chips[p]` and `final_scores[p]` are unchecked
too.

## 3. Required end state

**One pattern, two tiers. Never change behaviour for a valid state.**

**Tier 1 - `Gamer::validate` (the default).** Every crate in the table except
lords-of-vegas-1 gains a `fn validate(&self) -> Result<(), GameError>` on its
`impl Gamer for Game`, overriding WP-09a's defaulted no-op. It checks exactly
the invariants that crate's panicking sites assume, and nothing more:

- `self.players` (red7-1: `self.num_players`) is one of the values the crate's
  own `player_counts()` returns;
- every per-player `Vec` field has `len() == players`;
- every stored player index field (`current_player`, `first_player`,
  `start_player`, `bid_player`, `current_turn`, ...) is `< players`.

Each failure returns `GameError::internal("<crate>: <what was inconsistent>")`
(`GameError::internal` is in `rust/lib/game/src/errors.rs`). **Add no other
checks** - not deck contents, not phase consistency. The panicking sites
themselves are then left as-is: `validate` makes them unreachable from the
request path, which is what "defence in depth" means here.

**Tier 2 - in-place guard.** Only for sites reachable *without* going through a
deserialized `Game`: parsers, `pub` helpers taking raw args, and pure fns.
Three permitted forms - `return Err(GameError::invalid_input(...))` in a fn
that already returns `Result`; `checked_sub`/`saturating_sub` for arithmetic;
`.get(i)` with a benign fallback in a renderer. Introduce no new panic and no
new `unwrap`.

| Finding | Crate | File + fn | Fix | Test |
|---|---|---|---|---|
| d F5 | lords-of-vegas-1 | `board.rs` `Loc::parse_str` | Tier 2: reject `lot < 1` or `lot > block.max_lot()` with the existing `Err(String)`; `neighbours`' `self.lot - 1` then cannot underflow. No `validate` impl for this crate. | Y |
| d F38 | modern-art-2 | `lib.rs` `round_cards`, `impl Gamer` | Tier 1: `validate` bounds `players` to 3..=5, checks `round < 4`, and checks `player_money`/`player_hands`/`player_purchases` lengths `== players` and `current_player < players`. Leave `round_cards`' `unreachable!()` and `table[round]` alone. | Y |
| e F2 | love-letter-2 | `lib.rs` `end_score`, `impl Gamer` | Tier 1: `validate` bounds `players` to 2..=4, checks `current_player < players`, and checks `discards`/`player_points`/`eliminated`/`protected` lengths `== players`. Leave the `unreachable!()`. | Y |
| e F3 | love-letter-2 | `lib.rs` `end_round` | Tier 1: the same `validate` also checks `hands.len() == players` and that each non-eliminated player's hand is non-empty. `hands[p][0]` unchanged. | N |
| e F4 | love-letter-2 | `lib.rs` `assert_target` | Tier 2: add `if target >= self.players { return Err(GameError::invalid_input("that is not a player in this game")) }` as the **first** check in the fn. | Y |
| e F10 | age-of-war-2 | `lib.rs` `scores`/`check_end_of_turn`/`line_action`, `command.rs` `line_parser`, `render.rs` `render_castle`/`render_castles` | Tier 1: `validate` checks `current_player < players`, that `conquered` and `castle_owners` both have `castle::castles().len()` entries, and that every `conquered[i]` has `castle_owners[i].is_some()` (this crate has no per-player vecs). Tier 2 for `line_action` only: replace its `.expect("can_line implies currently_attacking")` with `ok_or_else(\|\| GameError::internal("no attack in progress"))?` (fn already returns `Result`). Leave the render and `line_parser` `expect`s. | Y |
| e F22 | lost-cities-2 | `lib.rs` `expedition_cost`/`hand_size`/`expedition_bonus_size`, `impl Gamer` | Tier 1: `validate` bounds `players` to 2..=3 and checks `hands`/`scores`/`expeditions`/`stats` lengths `== players` and `current_player < players`. Leave all three `unreachable!()`s. | Y |
| f F4 | zombie-dice-2 | `lib.rs` `take_dice`/`next_player`/`end_turn`/`placings`, `render.rs` | Tier 1: `validate` checks `players >= 2`, `scores.len() == players`, `current_turn < players`, and every entry of `roll_off_players` `< players`. `drain(..n)`'s `n` is already bounded by the refill path - do not touch it. | Y |
| f F9 | battleship-2 | `lib.rs` `is_finished`/`status`/`placings`/`place_ship` | Tier 1: `validate` checks `boards.len() == players`, `left_to_place.len() == players` and `current_player < players`. Do **not** convert these to `.get()` - the crate's split style stays. | Y |
| f F19 | for-sale-2 | `lib.rs` `start_buying_round`/`start_selling_round`/`play`/`take_first_open_card` | Tier 1: `validate` checks `hands`/`chips`/`cheques`/`bids`/`finished_bidding` lengths `== players` and `bidding_player < players` (the index field is `bidding_player`, not `current_player`). Tier 2: guard the two `split_off(len - n)` calls with `if len < n { return vec![] }` (both return `Vec<Log>`). Leave `remove(0)`. | Y |
| f F26 | category-5-2 | `lib.rs` `resolve_plays`/`choose_row` (the three `expect`s) | Tier 1: `validate` checks every `board` row is non-empty (`board` is a fixed `[Vec<Card>; ROWS]`, so its length needs no check), `player_points`/`hands`/`player_cards`/`plays` lengths `== players`, and `choose_player < players`. Leave the `expect`s. | N |
| f F29 | category-5-2 | `lib.rs` `Card`, `impl Gamer` | Tier 1: the same `validate` also checks every `Card(n)` in `board`/`hands`/`player_cards`/`plays`/`deck`/`discard` has `1 <= n <= 104`. Do **not** change `Card`'s tuple-field visibility or add a serde validator. | Y |
| f F34 | greed-2 | `lib.rs` `done`/`placings`, `render.rs` | Tier 1: `validate` checks `scores.len() == players`, `current_player < players` and `first_player < players`. **`points()` is a mis-citation - do not touch it.** | Y |
| f F44 | farkle-2 | `render.rs` (the `0..self.players` score table) | Tier 1: `validate` checks `scores.len() == players`, `current_player < players`, `first_player < players`. | N |
| f F46 | tic-tac-toe-2 | `lib.rs` `impl Gamer`, `render.rs` (`1 - start_player`) | Tier 1: `validate` checks `players == NUM_PLAYERS` and `start_player < NUM_PLAYERS`, closing the forged-`players` alloc and the `1 - start_player` underflow together. Do not change the render output. | Y |
| f F53 | no-thanks-2 | `render.rs` (the `unwrap`, `hands[p]`, `chips[p]`, `final_scores[p]`), `lib.rs` `player_state` | Tier 1: `validate` checks `player_hands`/`player_chips` lengths `== players` and `currently_moving < players` (the index field is `currently_moving`). Tier 2: the renderer's `current_card.unwrap()` becomes `if let Some(c)` with the line omitted when `None`. | Y |
| f F55 | liars-dice-2 | `lib.rs` `next_player`/`call`/`placings`/`player_state` | Tier 1: `validate` checks `player_dice.len() == players`, `current_player < players`, `bid_player < players`. The existing empty-dice guard stays. | Y |
| *(no id)* | red7-1 | `lib.rs` (`0..self.num_players` loops), `render.rs` | Tier 1: `validate` checks `num_players` in 2..=4 and that `hands`/`palettes`/`scored_cards`/`eliminated` all have `len() == num_players` and `current_player < num_players`. **WP-29 Task 2's saturating `end_points` is separate and already landed - do not widen or revisit it.** | Y |

## 4. Non-goals

- **WP-09a** - the `validate` hook itself, the requester bounds check, both
  lost-cities `player_state` majors, acquire-1's panics, sushizock-2's `target`.
  This package only *implements* `validate`; it does not change the trait or
  `rust/lib/cmd`.
- **WP-10** - outbound `pub_state` redaction. WP-09 is the **inbound** trust
  direction only. Hide no information, redact nothing.
- **WP-29 Task 2** (red7-1 `end_points` saturating arithmetic) - finalized.
- **All rules parity.** WP-11/12/16/20/26/30 are BLOCKED-ON-USER-RULES-REVIEW.
  **Change no gameplay and edit no `RULES.md` or `DATA_DOCS.md`** - those docs
  are themselves under user review and some content is AI-generated.
- No serde attributes on any persisted type. `#[serde(default)]` on a stored
  enum is how f F18 nearly corrupted live for-sale games; do not reach for it.
- No signature changes, no field-visibility changes, no `.get()` conversions
  beyond the two Tier 2 rows that name one.
- Test-module renaming (e F9) belongs to WP-65.

## 5. Regression test cases

Each crate gets one test of the same shape: build a `Game` by hand (or mutate a
`Game::start(...)` result) into an inconsistent state - `players` out of range,
a per-player `Vec` one entry short, or a stored index `== players` - assert
`game.validate()` is `Err(GameError::Internal { .. })`, and assert a
freshly-started `Game::start(n, seed).unwrap().0.validate()` is `Ok(())`. The
two Tier 2 parser/renderer rows get a direct call instead: lords-of-vegas-1
asserts `Loc::parse_str("a0")` and a beyond-`max_lot` string are `Err`;
love-letter-2 asserts `assert_target` with `target == self.players` returns
`Err(GameError::InvalidInput { .. })`. Every crate in the table already has an
inline test module, so **no new test module is needed anywhere** - but the
naming is inconsistent across the workspace, so **read the file and match it**:
`mod tests` in lords-of-vegas-1, tic-tac-toe-2 and red7-1; `mod test` in
love-letter-2, age-of-war-2, zombie-dice-2, battleship-2, for-sale-2,
category-5-2, greed-2, farkle-2, no-thanks-2 and liars-dice-2; modern-art-2 and
lost-cities-2 have **both** - put the new test in the one in `lib.rs`. No
existing test may be edited; all must stay green.

## 6. Riders

None - the whole package is the table above.
