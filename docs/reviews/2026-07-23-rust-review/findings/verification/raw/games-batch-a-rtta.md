# Verification: roll-through-the-ages-2 (batch A)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5. Paths relative to rust/ unless noted.

## F1 — roll() re-matches self.phase after keep_skulls() may have advanced it

**Verdict: CONFIRMED** (major, correctness — severity appropriate)

Control flow traced:

- `roll()` (game/roll-through-the-ages-2/src/lib.rs:740-753):
  ```rust
  self.rolled_dice = rolled.into_iter().chain(kept).collect();
  logs.extend(self.keep_skulls());
  match self.phase {
      Phase::Roll => {
          self.remaining_rolls -= 1;
          if self.remaining_rolls == 0 {
              logs.extend(self.next_phase());
          }
      }
      Phase::ExtraRoll => {
          logs.extend(self.next_phase());
      }
      _ => {}
  }
  ```
- `keep_skulls()` (lib.rs:680-687) does call `next_phase()` when the post-reroll `rolled_dice` is empty (all skulls) and the `ExtraRoll && Leadership` exception does not hold:
  ```rust
  if self.rolled_dice.is_empty()
      && !(self.phase == Phase::ExtraRoll
          && self.boards[self.current_player]
              .developments
              .contains(&DevelopmentId::Leadership))
  {
      return self.next_phase();
  }
  ```
  So the `match self.phase` at 742 runs on a phase that keep_skulls may already have advanced. Both claimed scenarios check out:

**Scenario 1 (Leadership extra roll skipped): CONFIRMED.** In `Phase::Roll` with Leadership, an all-skull reroll empties `rolled_dice`; the guard's exception requires `phase == ExtraRoll` so it does not apply; `next_phase()` -> `roll_extra_phase()` (lib.rs:290-303) merges the kept skulls back into `rolled_dice`, sees Leadership, returns `vec![]` leaving `phase = ExtraRoll` awaiting the player's extra roll. Back in `roll()`, the match then hits the `Phase::ExtraRoll` arm and calls `next_phase()` again -> `collect_phase()`, consuming the Leadership extra roll without player input. This diverges from the `next`-command path: `next()` (lib.rs:778-783) calls `next_phase()` once with no trailing match, and `test_game_keep_skulls_all_disaster_leadership` (lib.rs:1891-1901) asserts that path ends at `Phase::ExtraRoll`:
  ```rust
  let res = g.command(MICK, "next", &test_players());
  ...
  assert_eq!(Phase::ExtraRoll, g.phase);
  ```

**Scenario 2 (next player's remaining_rolls corrupted): CONFIRMED.** Without Leadership, all-skull reroll of 5+ dice: keep_skulls -> `roll_extra_phase` (no Leadership -> next) -> `collect_phase` (skulls only, no food-or-workers dice -> next, lib.rs:336-338) -> `phase_resolve` -> 5+ skulls = Revolt zeroing the roller's goods (lib.rs:497-507) -> `next_phase` (lib.rs:510) -> `build_phase`. With no workers and goods zeroed, `can_build_or_trade` (lib.rs:197-199, backed by 166-194) is false -> `trade_phase` (ships/goods gone -> next) -> `buy_phase` (buying power < 10 -> next) -> `discard_phase` (goods_num <= 6 -> next) -> `next_turn()` (lib.rs:570-579) -> `start_turn` -> `preserve_phase` -> `roll_phase()` (lib.rs:281-287) which sets `self.remaining_rolls = 2; self.phase = Phase::Roll` and rolls the NEXT player's dice. Control returns to the previous player's `roll()`, whose match sees `Phase::Roll` and executes `self.remaining_rolls -= 1` — the next player starts with 1 reroll instead of 2. (If the cascade instead ends the game, phase stays `Discard` and the `_ => {}` arm is harmless.)

**Go-inherited: CONFIRMED.** brdgme-go/roll_through_the_ages_1/roll_command.go:46-55 has the identical structure (`logs = append(logs, g.KeepSkulls()...)` then `switch g.Phase`), and Go's `KeepSkulls` (roll_command.go:84-87) has the same `NextPhase()` call with the same exception. No quirk comment exists on the Rust `roll()` (lib.rs:707-755 doc is just "Port of `Roll`"; the only quirk note nearby is the `CanUndo` one at 757-758).

**Test coverage gap: CONFIRMED.** The only `roll`-command tests are lib.rs:1908, 1921, 1925 (`test_roll_command`, `test_roll_extra_command`); dice are seeded as `Die::Coins` and reroll outcomes are random — no test deterministically exercises an all-skull result of the `roll` command. The all-skull tests (1880, 1891) go through `next`, which bypasses `keep_skulls`.

**Severity:** major is appropriate. It is Go-faithful, but it is genuine cross-player state corruption (scenario 2) and loss of a purchased ability (scenario 1), and unlike other preserved Go quirks it is entirely undocumented — failing both Correctness and the crate's own quirk-documentation Consistency practice.

## F2 — RULES.md claims developments are globally exclusive; code is per-player

**Verdict: CONFIRMED** (minor, correctness — severity appropriate)

- RULES.md:44: "17 one-time purchases, each bought once (any development already owned by a player can't be bought again by anyone)".
- `buy_development` (lib.rs:1089): `if self.boards[player].developments.contains(&development)` — only the buyer's own set.
- `available_developments` (lib.rs:625-631): `.filter(|d| !self.boards[player].developments.contains(d))` — again only the given player's set.

Nothing anywhere checks other players' boards. Doc contradicts code; code matches the real game (developments are per-player). Minor is right: doc-only defect, no behaviour change.

## F3 — RULES.md says pestilence hits "every player"; code exempts the roller

**Verdict: CONFIRMED** (minor, correctness — severity appropriate)

- RULES.md:90: "3 skulls = pestilence (3 disaster points to every player, blocked per-player by Medicine)".
- lib.rs:408-411 (in `phase_resolve`, 3-skull arm):
  ```rust
  for p in 0..players {
      if p == cp {
          continue;
      }
  ```
- Test `resolve_pestilence_hits_other_players_without_medicine` (lib.rs:2164-2171) asserts `assert_eq!(0, g.boards[MICK].disasters);` for the roller and 3 for each other player.

Doc-only defect (code matches the real game's "all opponents" rule). Minor appropriate.

## F4 — RULES.md says skulls "can never be rerolled"; Leadership extra roll can reroll them

**Verdict: CONFIRMED** (minor, correctness — severity appropriate)

- RULES.md:87: "Any skulls rolled are locked in immediately (they can never be rerolled)."
- `roll_extra_phase` (lib.rs:290-295) merges locked skulls back into the rerollable pool:
  ```rust
  let mut kept = std::mem::take(&mut self.kept_dice);
  self.rolled_dice.append(&mut kept);
  self.kept_dice = vec![];
  ```
- The ExtraRoll parser (src/command.rs:388, `Many::bounded_spaced(Int::bounded(1, max_i), 1, max_u)` with `max_i = rolled_dice.len()`) admits any position including skull positions.
- Test `test_roll_extra_command` (lib.rs:1913-1927): `rolled_dice = vec![Die::Coins; 4]`, `kept_dice = vec![Die::Skull; 3]`; after `next` into ExtraRoll, `roll 7` (position 7 = a merged skull) succeeds: `assert!(res.is_ok());`.

Note RULES.md:88 ("reroll exactly one die from everything you're currently holding") already hints at the exception, but the absolute "never" in :87 is still wrong. Minor appropriate (doc-only; keep_skulls behaviour is also relevant to F1).

## F5 — 15-food cap and Preservation clipping undocumented

**Verdict: CONFIRMED** (nit, correctness — severity appropriate)

- lib.rs:350-359 (`phase_resolve`):
  ```rust
  if self.boards[cp].food > 15 {
      ...
      self.boards[cp].food = 15;
  }
  ```
  applied before feeding cities (lib.rs:362-381). `preserve` (lib.rs:803, `self.boards[player].food *= 2;`) has no cap, so doubled food above 15 is silently clipped at resolve.
- RULES.md contains no mention of a food maximum anywhere (the only 15s are development costs and the Great Pyramid size); RULES.md:86 describes Preservation with no clipping caveat.

Nit appropriate: doc omission of an edge case.

## F6 — command() repeats the finished-game scores/placings epilogue in every arm

**Verdict: CONFIRMED** (minor, simplicity — severity appropriate)

`command()` (lib.rs:1521-1755) has 11 arms (Next, Roll, Preserve, Build, Trade, Buy, Take, Discard, Invade, Sell, Swap), each ending with an identical 12-line block, e.g. lib.rs:1539-1550:

```rust
if self.finished {
    let scores: Vec<(usize, i32)> = self
        .scores()
        .iter()
        .enumerate()
        .map(|(i, &s)| (i, s))
        .collect();
    let metrics: Vec<Vec<i32>> =
        self.scores().into_iter().map(|s| vec![s]).collect();
    resp.logs
        .push(placings_log(&gen_placings(&metrics), Some(&scores)));
}
```

`grep -n "if self.finished"` shows the 11 copies at lib.rs:1539, 1559, 1579, 1599, 1619, 1639, 1659, 1679, 1699, 1719, 1739 (the 12th hit at 1775 is elsewhere). Roughly 11 x 12 = 132 lines that could be one helper wrapped around the dispatch. Minor/simplicity is the right classification.

## F7 — ship parser bound uses max(wood, cloth) instead of min, ignores 5-ship cap

**Verdict: CONFIRMED** (nit, correctness — severity appropriate)

- src/command.rs:186-188:
  ```rust
  let wood = b.goods.get(&Good::Wood).copied().unwrap_or(0);
  let cloth = b.goods.get(&Good::Cloth).copied().unwrap_or(0);
  let max = wood.max(cloth);
  ```
  feeding `Int::bounded(1, max)` (command.rs:194).
- `build_ship` (lib.rs:879-912) charges 1 wood AND 1 cloth per ship and rejects `amount > wood`, `amount > cloth`, and `self.boards[player].ships + amount > 5` — so the true parser bound is `min(wood, cloth).min(5 - ships)`. The parser admits amounts the action then rejects with an error; no state corruption.
- Go-faithful: brdgme-go/roll_through_the_ages_1/command.go:174-179 (`BuildTargetShipParser`) computes the same max-of-the-two bound. No quirk comment on the Rust side documents this.

Nit appropriate: parser over-admission only, action layer validates correctly.

## F8 — Quarrying bonus stone bypasses the good cap

**Verdict: CONFIRMED** (nit, correctness — severity appropriate)

- `gain_good` (src/player_board.rs:141-147) respects the cap:
  ```rust
  let max = good_maximum(good);
  let cur = self.goods.get(&good).copied().unwrap_or(0);
  if cur < max {
      self.goods.insert(good, cur + 1);
  }
  ```
- The Quarrying bonus in `gain_goods` (player_board.rs:129-135) does not:
  ```rust
  if good == Good::Stone
      && self.developments.contains(&DevelopmentId::Quarrying)
      && !quarrying_used
  {
      *self.goods.entry(good).or_insert(0) += 1;
      quarrying_used = true;
  }
  ```
  With stone already at the cap of 7, `gain_good` no-ops but the bonus still increments to 8. The adjacent doc comment (player_board.rs:119-121) documents the round-robin quirk (#3) but not this cap bypass. The test `gain_goods_respects_good_maximum_cap` (player_board.rs:320) covers `gain_good`'s cap, not the Quarrying path at the cap. Nit appropriate: rare edge, Go-faithful, worst effect is one extra stone.

## F9 — roll() dice-index guard admits n == 0

**Verdict: ADJUSTED** (nit, quality — severity appropriate)

- lib.rs:722-729:
  ```rust
  let l = self.rolled_dice.len() as i32;
  for &n in dice_num.iter() {
      if n < 0 || n > l {
          return Err(GameError::invalid_input(format!(
              "dice number must be between 1 and {}",
              l
          )));
      }
  }
  ```
  `n == 0` passes despite the message saying "between 1 and". Confirmed. Unreachable via the parser: confirmed — the roll parser (src/command.rs:388) uses `Many::bounded_spaced(Int::bounded(1, max_i), 1, max_u)`, so 0 can only arrive via direct `roll()` API use. Go-identical guard at brdgme-go/roll_through_the_ages_1/roll_command.go:33.

**Correction:** "silently no-op" is not quite right. `dice_num = vec![0]` matches no 1-based position, so no die is rerolled, but the call still emits a roll log and executes the phase match — in `Phase::Roll` it decrements `remaining_rolls` (lib.rs:744) and in `Phase::ExtraRoll` it advances the phase (lib.rs:750), i.e. it consumes the player's roll while rerolling nothing. Since it is parser-unreachable, nit/quality remains the right severity.

## F10 — pointless `let logs = vec![...]; let mut logs = logs;` rebind in discard()

**Verdict: CONFIRMED** (nit, quality — severity appropriate)

lib.rs:1245-1252:

```rust
let logs = vec![Log::public(vec![
    N::Player(player),
    N::text(" discarded "),
    N::Bold(vec![N::text(amount.to_string())]),
    N::text(" "),
    N::text(good.name()),
])];
let mut logs = logs;
```

`let mut logs = vec![...]` would do; the immutable binding plus shadowing rebind serves no purpose. Nit appropriate.

## Summary

- F1: CONFIRMED (major appropriate)
- F2: CONFIRMED (minor appropriate)
- F3: CONFIRMED (minor appropriate)
- F4: CONFIRMED (minor appropriate)
- F5: CONFIRMED (nit appropriate)
- F6: CONFIRMED (minor appropriate)
- F7: CONFIRMED (nit appropriate)
- F8: CONFIRMED (nit appropriate)
- F9: ADJUSTED (nit appropriate; not a pure no-op — it consumes the roll)
- F10: CONFIRMED (nit appropriate)
