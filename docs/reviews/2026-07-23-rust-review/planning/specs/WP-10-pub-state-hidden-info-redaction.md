# WP-10: pub_state hidden-info redaction

**Findings:** f F1 (major), f F13 (major), plus one routed-in item from WP-13's
Non-Goals (starship-catan-1 `peeking`). **Decision:** D-33 answered option A -
public view data carries counts/aggregates only; per-player secrets live in
`player_state` for the entitled viewer. **READY** - the 2026-07-25 D-35
rules-parity park does *not* apply (D-33 is explicitly unaffected; this is
hidden-information leakage, not port parity).

**This WP decides the redaction shape once for every game crate.** Later crates
copy section 3a verbatim; do not re-litigate it per crate.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** `rust/` is under
> concurrent edit. No line numbers are cited on purpose; the few offered
> elsewhere are approximate, verify.

## 1. Problem

- **f F1** - `rust/game/zombie-dice-2/src/lib.rs`, `Gamer::pub_state` clones
  `Game::cup` verbatim into `PubState::cup`, documented "in draw order".
  `take_dice` drains from the front and the cup is shuffled only at turn start
  / refill, so the head of the vector *is* the next draw - any API/bot client
  reads the next dice colours exactly.
- **f F13** - `rust/game/for-sale-2/src/lib.rs`, `Gamer::pub_state` clones
  `Game::bids` verbatim. During `Phase::Selling`, `play()` stores the secretly
  chosen building into `bids[player]`, so every already-played opponent's
  choice is public JSON before the reveal.
- **Routed in (WP-13 Non-Goals, not in WP-10's declared 2-finding scope)** -
  `rust/game/starship-catan-1/src/lib.rs`, `Gamer::player_state` sets
  `peeking: self.peeking.clone()` for **both** players. Only the current player
  may peek (`can_put` requires `current_player == player`), so the opponent's
  `PlayerState` JSON carries the peeked Sensor cards.

## 2. Why it's wrong

- **f F1 is correct as written.** Re-verified live: the clone, the front-drain
  in `take_dice`, and shuffle-only-at-turn-start are all still present.
  `render_cup` (`rust/game/zombie-dice-2/src/render.rs`) already collapses the
  cup to per-colour counts, so no UI depends on the order. `DATA_DOCS.md` also
  claims Zombie Dice "has no hidden information per player" - inaccurate.
- **f F13 is correct as written.** Re-verified live. Go leaks identically; Go
  parity is irrelevant here (hidden info, not rules).
- **The starship-catan-1 item is real in live code.** WP-13 Task 5 has **not**
  landed (`render()` still takes `_peeking` unused), and even once it does it
  only guards the *human render*. The data-level exposure is WP-10's.

## 3. Required end state

### 3a. The canonical redaction shape (copy this in later crates)

1. `pub_state()` **constructs** its fields. It must never `.clone()` a private
   `Game` field straight through.
2. Hidden ordered or per-player data appears in `PubState` as **counts /
   aggregates only** - never the ordered vector, never the per-seat values.
3. The private detail is re-added in `player_state(player)`, scoped to that one
   player. A whole-game private field must never be cloned into every player's
   state.
4. Renderers read the redacted public field for public info and the
   `PlayerState` field for the viewer's own secret.
5. Heuristic: if a field is an ordered deck or a `Vec` indexed by player, it may
   not be cloned into `PubState`.

### 3b. `rust/game/zombie-dice-2` - cup as counts

- In `src/lib.rs`, replace `PubState::cup: Vec<Dice>` with
  `cup_counts: Vec<(Colour, usize)>`, always exactly three entries in the fixed
  order Green, Yellow, Red (zeros included). `pub_state()` builds it by counting
  `self.cup`; `Game::cup` itself is unchanged.
- In `src/render.rs`, `render_cup` takes `&[(Colour, usize)]`, skips zero
  entries, and renders grey "None" when every count is zero. Delete the now
  unused `cup_counts` helper. Output markup must be byte-identical to today's.
- `DATA_DOCS.md`: update the `cup` bullet to describe counts, and correct the
  "no hidden information per player" sentence (the cup composition is public,
  the draw order is not).

### 3c. `rust/game/for-sale-2` - selling plays redacted until reveal

- In `src/lib.rs`, `pub_state()`: when `self.current_phase() == Phase::Selling`,
  emit `bids: vec![0; self.players]`. Other phases clone as today - the buying
  phase is an open auction and its bids are legitimately public.
- `finished_bidding` stays public in all phases: who has already played is
  public knowledge at the table.
- Add `pub bid: i32` to `PlayerState`, set from `self.bids[player]` in
  `player_state()`, doc-commented as the viewer's own bid/played building.
- In `src/render.rs`, the `Phase::Selling` arm of `render` reads the viewer's
  own play from the `PlayerState` (`own.bid`), not `pub_state.bids[p]`. The
  `Phase::Buying` arm and `highest_bid` are unchanged.

### 3d. `rust/game/starship-catan-1` - peek is the peeker's alone

- In `src/lib.rs`, `player_state()` sets `peeking` to `self.peeking.clone()`
  only when `player == self.current_player`, otherwise `vec![]`. Nothing else
  in the crate changes.

## 4. Non-goals

- **No gameplay or rules change, in any crate, for any reason.** Do not touch
  any `RULES.md`. The parity findings in these same crates (f F2, f F14, f F15
  and the rest of WP-11) are parked - leave them.
- **Not WP-09.** WP-09 hardens *inbound* trust in deserialized state (bounds
  checks, validate hook). WP-10 is *outbound* redaction only; add no bounds
  checks beyond what section 3 states.
- **Not WP-13 Task 5.** Do not add the peeked-cards render table (WP-13's, may
  land concurrently). WP-10 touches only starship-catan-1's `player_state`.
- No change to `Game`'s own persisted serde shape in any of the three crates.

## 5. Regression test cases

- `rust/game/zombie-dice-2/src/lib.rs`, inline `#[cfg(test)] mod test`: assert
  `pub_state().cup_counts` equals the fixed-order triple matching `g.cup`'s
  composition, and that the composition is preserved while order is not
  recoverable. Update the existing `test_pub_state_captures_rendered_fields`,
  which currently asserts `g.cup == ps.cup`.
- `rust/game/for-sale-2/src/lib.rs`, inline `#[cfg(test)] mod test`: drive a
  game into `Phase::Selling`, have one player `play`, then assert
  `pub_state().bids` is all zeros while `player_state(that_player).bid` is the
  played building and `player_state(opponent).bid` is 0. Assert a Buying-phase
  `pub_state().bids` still mirrors `g.bids`. Existing
  `test_pub_state_redacts_hands_and_cheques` runs at game start (Buying) and
  should keep passing - verify, do not weaken it.
- `rust/game/starship-catan-1/src/lib.rs`, inline `#[cfg(test)] mod tests`
  (which already has a `peeking` test): with `g.peeking` non-empty, assert
  `player_state(g.current_player).peeking` is non-empty and
  `player_state(1 - g.current_player).peeking` is empty.

## 6. Riders

None - all three items are major and in scope above.
