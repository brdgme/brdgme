# Verification: games batch C - acquire-1 (F7-F21)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5. All paths relative to `game/acquire-1/` unless noted. No Go source exists for Acquire; official-rulebook premises rest on the reviewer's rules knowledge and are noted per finding. In-crate RULES.md was checked as the closest in-repo rules source.

## F7 - player_counts() excludes 6 - CONFIRMED (major, correctness)

- `src/lib.rs:312-314`:
  ```rust
  fn player_counts() -> Vec<usize> {
      (2..6).collect()
  }
  ```
  `(2..6)` is exclusive: `[2, 3, 4, 5]`.
- `src/lib.rs:25`: `pub const MAX_PLAYERS: usize = 6;`
- `src/lib.rs:186-192`: `if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)` - inclusive range, so `start(6, ...)` is accepted.
- Trait: `lib/game/src/game.rs:64` declares `fn player_counts() -> Vec<usize>;` as a required `Gamer` method. It is the advertised supported-count set: `lib/cmd/src/requester/gamer.rs:56-58` serves it via `handle_player_counts` (`player_counts: G::player_counts()`), and `web/` stores/queries `game_types.player_counts` to gate game creation (e.g. `web/src/game/server_fns.rs:543`). So 6-player Acquire is playable by the engine but never offered by the platform.
- Severity: major stands - functional capability silently lost through the API surface.

## F8 - 2p dummy roll is 1..=5, log says D6 - CONFIRMED (major, correctness)

- `src/lib.rs:901-904`:
  ```rust
  if self.players.len() == 2 {
      dummy_shares = self.rng.random_range(1..=5);
  ```
  Range is 1-5; a D6 would be `1..=6`.
- In-crate start log `src/lib.rs:220-224`: "A dice (D6) is rolled to determine the dummy player's shares."
- Additional in-repo contradiction: `RULES.md:153-154`: "roll a six-sided die (D6): the result (1-6) is the dummy's share count".
- The code contradicts two in-repo rules statements; the dummy can never hold 6 shares, skewing 2-player bonus outcomes. Severity major stands.

## F9 - panic! in pay_bonuses - CONFIRMED (minor, correctness)

- `src/lib.rs:840-842`:
  ```rust
  if major_len == 0 {
      panic!("expected some major bonus players");
  }
  ```
- Runtime path: `pay_bonuses` is called from `handle_merge_command` (`src/lib.rs:805`) and `end` (`src/lib.rs:678`), both reachable from `Gamer::command` - a request-reachable path, contrary to docs/CODING.md:46-49 ("No panicking code in runtime paths").
- "Appears unreachable" reasoning: mostly holds under game invariants. In 2p, `bonus_players` always pushes `DUMMY_PLAYER_OFFSET` (`src/lib.rs:901-903`), so `major` is non-empty. In 3+, `pay_bonuses` runs only for corps with size > 0; founding grants the founder a share whenever the bank has one (`src/lib.rs:572-577`), and the bank can only be empty if players hold all 25 (in which case some player has shares and becomes major). The invariant is not enforced for deserialized or hand-constructed states, so the panic is a latent defensive assert in a request path rather than provably dead code. Minor stands.

## F10 - .expect() cluster on shares lookups - CONFIRMED (minor, correctness)

All cited sites verified, each with message quoted:

- `src/lib.rs:680-683` (in `end`): `.get(corp).expect("could not get player shares")`
- `src/lib.rs:1003-1006` (`handle_sell_command`): `.get(&corp).expect("could not get player shares")`
- `src/lib.rs:1023-1026` (`sell`): `.get(&corp).expect("could not get player shares")`
- `src/lib.rs:1072-1076` (`handle_trade_command`): `.cloned().expect("could not get player shares")`
- `src/lib.rs:1084-1087` (`handle_trade_command`): `.cloned().expect("could not get into shares")`
- `src/lib.rs:1119-1122` (`take_shares`): `.expect("could not get corp share count")`
- `src/lib.rs:1136-1139` (`return_shares`): `.expect("could not get player share count")`
- `src/command.rs:56-59` (`command_parser`, SellOrTrade arm): `.expect("could not get player shares")`
- `src/command.rs:157-163` (`player_shares_parser`): `.expect("could not et player shares")` - typo "et" confirmed at line 163.
- `src/render.rs:78`: `self.shares.get(c).expect("expected corp to have shares")`

Mitigating context confirmed: `corp_hash_map` (`src/lib.rs:1224-1230`) pre-populates all 7 corp keys for `Game::default().shares` and `Player::default().shares`, and `take_shares`/`return_shares`/`found` use `.entry().or_insert()` so keys are never removed. But the crate demonstrably anticipates legacy serialized state (rng migration shim, `src/lib.rs:156-159`: `#[serde(default = "GameRng::from_entropy")]`), and there is no serde default for the share maps, so a legacy/foreign state missing keys panics in request paths.

Internal inconsistency confirmed: `handle_buy_command` uses `self.shares.get(&corp).cloned().unwrap_or(0)` (`src/lib.rs:616`), and `render.rs:138` uses `.cloned().unwrap_or(0)`; `bonus_players` (`src/lib.rs:909`), `handle_merge_command` (`src/lib.rs:813`), and `next_player_sell_trade` (`src/lib.rs:965`) also use `unwrap_or(0)`. Same lookup, two failure policies. Minor stands.

## F11 - "Trades" stat inserts merges - CONFIRMED (minor, correctness)

- `src/stats.rs:45-46`:
  ```rust
  s.insert("Merges".to_string(), Stat::Int(self.merges as i32));
  s.insert("Trades".to_string(), Stat::Int(self.merges as i32));
  ```
  Line 46 uses `self.merges` where `self.trades` is intended - classic copy-paste defect.
- `self.trades` is maintained at `src/lib.rs:1095`: `self.players[player].stats.trades += receive;` so the field exists and is populated but never reported.
- Note impact is currently latent because `to_brdgme_stats` has no callers (see F12), but the defect is real. Minor stands.

## F12 - stats plumbing dead - CONFIRMED (minor, quality)

- `src/lib.rs:235-239`: `Status::Finished { placings: self.placings(), stats: vec![] }` - stats hardcoded empty.
- Workspace-wide grep for `to_brdgme_stats` matches only its definition at `src/stats.rs:27`. No callers anywhere; the entire `Stats::to_brdgme_stats` conversion (stats.rs:27-84) is dead code, and per-player stats accumulated during play are never surfaced. Minor stands.

## F13 - random start player - CONFIRMED (minor, correctness; external premise noted)

- `src/lib.rs:212-214`:
  ```rust
  let start_player = g.rng.random_range(0..players);
  g.phase = Phase::Play(start_player);
  ```
- Initial tiles are placed at `src/lib.rs:199-202` (`g.draw_tiles.drain(0..players)` -> `Tile::Unincorporated`) but the drawn tiles are not associated with players, so "closest to 1-A plays first" cannot even be computed from this code.
- In-crate RULES.md does not specify start-player determination, so there is no in-repo contradiction. The official-rules premise (initial tile draw determines start player) rests on the reviewer's rules knowledge. Code claim verified as stated; minor stands as a rules-fidelity/design note.

## F14 - redraw_hand discards temporarily-unplayable tiles - CONFIRMED (minor, correctness)

- Rejection reasons in `board.rs:130-142` (`assert_loc_playable`):
  1. `loc_neighbours_multiple_safe_corps(loc)` (line 131) - would merge two safe corps: **permanently** unplayable (safe corps never shrink).
  2. `loc_founds(loc) && available_corps().is_empty()` (line 136) - would found an 8th chain: **temporarily** unplayable (becomes playable again after any merger frees a corp).
- `src/lib.rs:693-708` (`start_turn`): if no hand tile passes `assert_loc_playable`, calls `redraw_hand`.
- `src/lib.rs:710-735` (`redraw_hand`): `self.board.set_discarded(&self.players[player].tiles)` (line 730) then `self.players[player].tiles = vec![]` (line 731) - the **entire** hand, including type-2 tiles, is permanently discarded (`set_discarded` at board.rs:169-173 sets `Tile::Discarded`; nothing ever un-discards).
- Asymmetry confirmed: end-of-turn `draw_replacement_tiles` (`src/lib.rs:375-380`) partitions on `loc_neighbours_multiple_safe_corps` only - i.e. discards only the permanent type and keeps temporarily-unplayable tiles.
- Consequence: a player whose hand is all temporarily-unplayable (e.g. all tiles would found an 8th chain) loses those tiles permanently and drains the bag by up to 6, even though the same tiles would have been kept by the end-of-turn path. Official Acquire discards only permanently-unplayable tiles; the all-founding-tiles case is a rules edge (official rules have the player simply unable to play a tile that founds when no chain is available - editions differ), so the exact correct behavior carries an external-rules premise, but the internal asymmetry between the two code paths is fully in-repo. Minor stands.

## F15 - game force-ends when bag cannot refill hand - CONFIRMED (minor, correctness; external premise noted)

- `src/lib.rs:403-408`:
  ```rust
  let remaining = TILE_HAND_SIZE - keep.len();
  if self.draw_tiles.len() < remaining {
      // End of game
      logs.extend(self.end()?);
      return Ok((logs, true));
  }
  ```
  When the bag has fewer tiles than needed to refill to 6, `self.end()` runs immediately - final scoring mid-flow, regardless of whether either official end condition holds.
- In-crate RULES.md "Ending the game" (lines 135-147) lists only the voluntary 41+ / all-safe end conditions and says nothing about tile exhaustion, so the code behavior is at least undocumented in-repo. The claim that official rules continue play without drawing when the bag is empty rests on the reviewer's rules knowledge (it matches standard editions). Code claim verified; minor stands.
- Note: also reachable via `redraw_hand` -> `draw_replacement_tiles`, compounding F14 (a mass hand discard can trigger the premature end).

## F16 - thiserror unused - CONFIRMED (minor, dependencies)

- `Cargo.toml:14`: `thiserror = "2.0.18"`.
- `grep -rn "thiserror" game/acquire-1/ --include='*.rs'` over src/ and tests/: zero matches. Declared, never used. Minor stands.

## F17 - can_undo always true in handle_found_command - CONFIRMED (nit, simplicity)

- `src/lib.rs:579`: `self.buy_phase(player);` - unconditional, and `buy_phase` (`src/lib.rs:516-521`) unconditionally assigns `self.phase = Phase::Buy { .. }`.
- `src/lib.rs:586`: `matches!(self.phase, Phase::Buy { .. })` as the returned `can_undo` - always `true` given line 579. Nit stands.

## F18 - iter().next().unwrap() in 1-arm - CONFIRMED (nit, consistency)

- `src/lib.rs:464-466`:
  ```rust
  match neighbouring_corps.len() {
      1 => {
          let n_corp = neighbouring_corps.iter().next().unwrap();
  ```
  Guarded by the `1 =>` arm so safe by construction, but an unwrap in a request path contrary to house style. Nit stands.

## F19 - start.unwrap() twice in width scan - CONFIRMED (nit, consistency)

- `src/render.rs:266-271`: inside `Tile::Corp(tc) if tc == *c` where `start` was set to `Some(col)` if `None` (lines 263-265):
  ```rust
  Some((
      start.unwrap(),
      row,
      col - start.unwrap() + 1,
  ))
  ```
  Unwraps at 268 and 270, safe by construction (start assigned just above on the same match arm). Nit stands.

## F20 - player_can_end deep-clones the game - CONFIRMED (nit, quality)

- `src/lib.rs:1200-1202`:
  ```rust
  fn player_can_end(&self, player: usize) -> bool {
      self.phase.main_turn_player() == player && self.pub_state().can_end() == CanEnd::True
  }
  ```
- `src/lib.rs:258-260`: `fn pub_state(&self) -> Self::PubState { self.to_owned().into() }` - clones the whole `Game` (board, all players, share maps, draw pile, rng) then converts.
- Call chain: `command_parser` (`src/command.rs:67`) calls `player_can_end` on every parser build - i.e. every `command()` and every `command_spec()`. Same pattern at `src/lib.rs:1184` (`handle_end_command`).
- `PubState::can_end` (`src/lib.rs:103-134`) reads only `self.finished`, `self.last_turn`, and `self.board.corp_size(...)` - board plus two bools; the clone of players/shares/draw_tiles/rng is pure waste. Nit stands (correct but wasteful).

## F21 - foundable-corp parser order nondeterministic - CONFIRMED (nit, consistency)

- `board.rs:55`: `pub fn available_corps(&self) -> HashSet<Corp>` - HashSet, iteration order unspecified.
- `src/command.rs:33-35`: `self.found_parser(self.board.available_corps().into_iter().collect())` - collects in HashSet iteration order.
- `src/command.rs:96-107`: `found_parser` feeds that Vec straight into `Enum::partial(corps)` (line 103), so the found-command spec/suggestion ordering varies run to run. Contrast: `buy_parser` uses the fixed `CORPS.to_vec()` (command.rs:122). Nit stands.

## Summary

| ID | Verdict | Severity |
|----|---------|----------|
| F7 | CONFIRMED | major |
| F8 | CONFIRMED | major |
| F9 | CONFIRMED | minor |
| F10 | CONFIRMED | minor |
| F11 | CONFIRMED | minor |
| F12 | CONFIRMED | minor |
| F13 | CONFIRMED (external premise noted) | minor |
| F14 | CONFIRMED | minor |
| F15 | CONFIRMED (external premise noted) | minor |
| F16 | CONFIRMED | minor |
| F17 | CONFIRMED | nit |
| F18 | CONFIRMED | nit |
| F19 | CONFIRMED | nit |
| F20 | CONFIRMED | nit |
| F21 | CONFIRMED | nit |
