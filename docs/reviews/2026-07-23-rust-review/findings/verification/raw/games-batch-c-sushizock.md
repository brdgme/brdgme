# Verification: games batch C - sushizock-2 (F29-F34)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
Go original: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go/sushizock_1.
All paths below relative to the rust snapshot root unless absolute.

## F29 (major, correctness) - steal_blue/steal_red i32 overflow on n = i32::MIN

Verdict: CONFIRMED (major stands)

Full path traced:

1. Parser accepts arbitrary i32. game/sushizock-2/src/command.rs:72-76:
   ```rust
   Opt::new(AfterSpace::new(Doc::name_desc(
       "tile",
       "optional if you have 4 chopsticks, which tile to steal in the stack, 1 for top",
       Int::any(),
   ))),
   ```
   `Int::any()` sets `min: None, max: None` (lib/game/src/command/parser/mod.rs:81-85). The
   `Int` parse impl (mod.rs:122-150) accepts a leading `-` at position 0 followed by digits
   and delegates to `str::parse::<i32>`, so the input `steal <p> blue -2147483648` yields
   `Ok(i32::MIN)` - `"-2147483648".parse::<i32>()` is in-range and succeeds. No bound check
   applies since min/max are None (mod.rs:151-168 are skipped).

2. Gate does not constrain n. game/sushizock-2/src/lib.rs:338-342:
   ```rust
   pub fn can_steal_blue_n(&self, player: usize) -> bool {
       self.current_player == player
           && self.another_player_has_blue(player)
           && self.dice_counts_all().blue_chopsticks >= 4
   }
   ```
   It never receives or touches `n` - no arithmetic on n anywhere before the subtraction
   (the F29 caveat about can_steal_*_n doing arithmetic on n does not apply). Same for
   `can_steal_red_n` (lib.rs:344-348).

3. Remaining guards before the subtraction (lib.rs:451-458) are `player == target` and
   `player_blue_tiles[target].is_empty()` - neither involves n. So with 4+ blue chopsticks,
   a distinct target holding at least one blue tile, `n = i32::MIN` reaches lib.rs:459-460:
   ```rust
   let len = self.player_blue_tiles[target].len();
   let index = len as i32 - n;
   ```
   `len >= 1` (empty case already returned), so `len as i32 - i32::MIN` overflows i32
   (e.g. 1 - (-2147483648) = 2147483649 > i32::MAX).

4. Identical code in steal_red at lib.rs:501-502:
   ```rust
   let len = self.player_red_tiles[target].len();
   let index = len as i32 - n;
   ```

5. Reachable from requests: `Gamer::command` (lib.rs:749-751) dispatches
   `Command::Steal` straight to `self.steal_blue(player, target, num)?` /
   `steal_red` with the parsed `num`.

6. Build profiles. Workspace /home/beefsack/Development/brdgme-review-snapshot/rust/Cargo.toml:
   ```toml
   [profile.dev]
   debug = "line-tables-only"

   [profile.server-dev]
   inherits = "dev"
   ```
   No profile sets `overflow-checks`; grep for "overflow" in workspace and crate Cargo.toml
   found nothing. Therefore dev (and server-dev, android-dev, wasm-dev, test) keep the
   default overflow-checks = true and the subtraction panics; release/wasm-release default
   to false, the subtraction wraps to a negative value (1 + i32::MIN = -2147483647), and
   the guard at lib.rs:461 (`if index < 0 || index as usize >= len`) catches it as an
   invalid-input error. docs/CODING.md forbids panic paths reachable from requests, and
   server-dev is a served profile, so this is a real player-input-triggered panic, exactly
   as the finding states.

Severity: major is right - request-reachable panic in overflow-check builds, but not
data loss and self-healing in release.

## F30 (minor, correctness) - Roll arm missing finish check / placings_log

Verdict: CONFIRMED

- Take arm, lib.rs:732-737: after `take_blue`/`take_red`,
  ```rust
  if self.is_finished() {
      let scores: Vec<(usize, i32)> = (0..self.players)
          .map(|p| (p, self.player_score(p)))
          .collect();
      logs.push(placings_log(&self.placings(), Some(&scores)));
  }
  ```
- Steal arm, lib.rs:753-758: identical block.
- Roll arm, lib.rs:711-722: only
  ```rust
  let logs = self.roll_dice_cmd(player, &dice)?;
  Ok(CommandResponse { logs, ... })
  ```
  No is_finished check, no placings_log.
- Finish-inside-roll path: roll_dice_cmd lib.rs:607-614 - when the final roll locks in and
  the player can neither take nor steal, `logs.extend(self.take_worst());` (lib.rs:612).
  take_worst (lib.rs:527-566) removes a tile from `red_tiles` (lib.rs:538) or `blue_tiles`
  (lib.rs:556); if that was the last tile of the last non-empty pile, `is_finished()`
  (lib.rs:282-284, both piles empty) becomes true. take_worst's `next_player()` call does
  emit `log_game_end()` (lib.rs:365-368), but the `placings_log` entry the other two arms
  append is silently skipped. Minor stands.

## F31 (nit/cross-reference) - roll_parser Many max ignored by suggest engine

Verdict: CONFIRMED (both cross-reference points check out)

(a) Parser construction, game/sushizock-2/src/command.rs:47 (inside roll_parser(max) where
    max = rolled_dice.len() via command.rs:24):
    ```rust
    Many::bounded_spaced(Int::bounded(1, max as i32), 1, max),
    ```
(b) Suggest engine ignores the bound. lib/game/src/command/suggest.rs:109:
    ```rust
    Spec::Many { spec, delim, .. } => {
    ```
    The `min`/`max` fields are discarded by `..` and the loop (suggest.rs:111-144) keeps
    consuming parsed items and re-suggesting the item spec with no count limit. (Note the
    runtime *parse* side does honor max - parser/mod.rs:371-375 breaks at `parsed.len() ==
    max` - so this is suggestion-side only, consistent with the finding being tracked as a
    lib/game suggest bug by another unit. Not verified further per instructions.)

## F32 (nit, quality) - unwrap on choose over const array

Verdict: CONFIRMED

game/sushizock-2/src/lib.rs:150-152:
```rust
fn roll_dice(rng: &mut GameRng, n: usize) -> Vec<DieFace> {
    (0..n).map(|_| *DIE_FACES.choose(rng).unwrap()).collect()
}
```
lib.rs:34: `const DIE_FACES: [DieFace; 6] = [` - fixed 6-element const, never empty, and
`slice::choose` returns None only for an empty slice, so the unwrap is statically
infallible. Nit as stated.

## F33 (nit, simplicity) - take_worst hand-rolled min loops + bare [0] index

Verdict: CONFIRMED

Code shape, lib.rs:527-566: both branches contain an identical hand-rolled find-min loop:
```rust
let mut min_idx = 0;
let mut min_val = self.red_tiles[0].value;
for (i, t) in self.red_tiles.iter().enumerate() {
    if t.value < min_val { min_val = t.value; min_idx = i; }
}
```
(red at lib.rs:530-537, blue at lib.rs:548-555) followed by identical remove/push/log
blocks. The else branch reads `self.blue_tiles[0].value` at lib.rs:549 with no emptiness
check.

Invariant reasoning verified: take_worst's only caller is roll_dice_cmd lib.rs:612, which
is reachable only through `Gamer::command` -> `command_parser`, and command_parser returns
None when `is_finished()` (command.rs:19-21), i.e. only runs while at least one pile is
non-empty (lib.rs:282-284: finished = both piles empty). The else branch runs only when
`red_tiles` is empty (lib.rs:529), so `blue_tiles` must be non-empty and the `[0]` index
is safe - but only via this non-local invariant, which is the simplicity complaint.
(These loops could be `iter().enumerate().min_by_key(|(_, t)| t.value)`, which would also
remove the bare index.) Nit stands.

## F34 (nit, simplicity) - take_blue/take_red and steal_blue/steal_red duplication

Verdict: CONFIRMED

- take_blue lib.rs:399-415 vs take_red lib.rs:417-431: structurally identical; differ only
  in the gate (can_take_blue/can_take_red), the dice count field (`.sushi` at lib.rs:405 vs
  `.bones` at lib.rs:421), the pile (`blue_tiles`/`player_blue_tiles` vs
  `red_tiles`/`player_red_tiles`), and the error string.
- steal_blue lib.rs:433-473 vs steal_red lib.rs:475-515: line-for-line identical except
  gates (can_steal_blue/_n vs can_steal_red/_n), pile vecs, and error strings - including
  the duplicated `len as i32 - n` / guard / remove / push / steal_log / next_player tail.
- Go original mirrors this: sushizock.go has TakeBlue:258 / TakeRed:274, StealRed:379 /
  StealBlue:398, StealRedN:417 / StealBlueN:444 as parallel per-color functions, so the
  Rust port faithfully reproduced the duplication rather than parameterizing on TileType.
Nit stands.
