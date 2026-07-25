# Verification: games-batch-e, lost-cities-1 (F36-F44)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
All line refs below re-read from the snapshot; quoted code verified verbatim.

## F36 - player_state() unchecked hands[player] index (major)

Verdict: CONFIRMED. Severity: major is right.

Evidence:
- `game/lost-cities-1/src/lib.rs:562-568`:
  ```rust
  fn player_state(&self, player: usize) -> Self::PlayerState {
      PlayerState {
          public: self.pub_state(),
          player,
          hand: self.hands[player].clone(),
      }
  }
  ```
  No bounds check; direct index.
- `lib/cmd/src/requester/gamer.rs:44-46`:
  ```rust
  Request::PlayerRender { player, ref game } => {
      let game = serde_json::from_str(game)?;
      Ok(handle_player_render::<G>(player, &game))
  }
  ```
  and `gamer.rs:170-174`: `handle_player_render` calls
  `game.player_state(player)` directly. The `player` value comes straight
  from the deserialized request with no validation anywhere on the path.
- Hands always has exactly 2 entries: `start_round` (lib.rs:124-128) pushes
  one hand per `0..PLAYERS` (`PLAYERS = 2`, lib.rs:25), and `start_round`
  runs before any request can carry a serialized game. So any
  `player >= 2` panics.
- Contrast: every other accessor in the crate uses `.get()/.get_mut()`
  with `GameError::internal` (e.g. `remove_player_card` lib.rs:286-290,
  `assert_has_card` lib.rs:319-323).

Severity: major fits the charter (clear defect: request-reachable panic
kills the handler). Not critical - requires a crafted/malformed request
from the API layer, not normal player input, and matches the major rating
given to the identical lost-cities-2 defect.

## F37 - draw_hand_full drops draw logs when draw empties deck (major)

Verdict: ADJUSTED - defect confirmed, severity should be minor (align
with the identical lost-cities-2 finding).

Evidence:
- `game/lost-cities-1/src/lib.rs:397-439`: `draw_hand_full` builds `logs`
  containing the public "P drew ... N remaining" log (pushed at
  lib.rs:427) and the private "You drew ..." log (pushed at lib.rs:430),
  then:
  ```rust
  if self.deck.is_empty() {
      self.end_round()
  } else {
      Ok(logs)
  }
  ```
  (lib.rs:434-438). On the deck-empty branch the return value is
  `end_round()`'s logs only; the local `logs` vec is dropped.
- `end_round` (lib.rs:141-169) confirms it builds its own log vec from
  scratch (score logs, then either `start_round` logs or the game-over
  log) - the caller's draw logs are never merged in.
- Game state remains correct: cards were already moved into the hand
  before the branch (lib.rs:407-409); only the log stream loses the
  final draw of each round.

Severity: the sibling lost-cities-2 finding (same defect, inherited from
this code) was rated minor. The impact is identical - cosmetic log loss,
no state corruption, no player-facing error. Major is not justified here;
the two should be aligned at minor. I'd assign minor.

## F38 - PlayerState.hand documented "sorted" but never sorted (minor)

Verdict: CONFIRMED. Severity: minor is right.

Evidence:
- `game/lost-cities-1/src/lib.rs:92-93`:
  ```rust
  /// Cards currently in this player's hand, sorted by expedition then value.
  pub hand: Vec<Card>,
  ```
- `game/lost-cities-1/DATA_DOCS.md:18`: "Cards are sorted by expedition
  then value."
- `player_state()` (lib.rs:562-568) returns `self.hands[player].clone()`
  raw. The hand is never sorted anywhere: in `draw_hand_full`, cards are
  pushed to the hand in draw order (lib.rs:407-409) and `drawn.sort()`
  (lib.rs:411) sorts only the local `drawn` vec used for the private log.
  `take` pushes the taken discard to the end (lib.rs:257-260). Only
  `render_hand` (render.rs:206-209) sorts a display copy.
- Note: the finding text's "never sorted" is even more accurate for -1
  than for -2 (in -1 not even the per-draw batch lands sorted in the
  hand; the sort is log-only).

## F39 - Stats.investments never written / expeditions write-only (minor)

Verdict: CONFIRMED. Severity: minor is right.

Evidence:
- `game/lost-cities-1/src/lib.rs:44-45`: `pub investments: usize,` /
  `pub expeditions: usize,` in serde-derived `Stats` (line numbers as
  cited).
- Grep over the crate: the only occurrence of `investments` is the field
  declaration at lib.rs:44 (the `INVESTMENTS` const at lib.rs:22 is
  unrelated). Never incremented - always 0 in every serialized state.
- `expeditions` is incremented exactly once (lib.rs:376) but
  `player_stats()` (lib.rs:448-482) surfaces only Plays, Discards,
  Draws, Takes - `expeditions` (and `investments`) never leave the
  struct.

## F40 - stats.expeditions increment condition mismatch (minor)

Verdict: CONFIRMED. Severity: minor is right.

Evidence:
- `game/lost-cities-1/src/lib.rs:370-377` in `play()`:
  ```rust
  if self
      .expeditions
      .get(player)
      .ok_or_else(invalid_expedition)?
      .is_empty()
  {
      self.stats[player].expeditions += 1;
  }
  ```
- `self.expeditions[player]` is a single flat `Vec<Card>` holding every
  card the player has played this round across all five expeditions
  (declared lib.rs:56; cleared per round at lib.rs:122). `.is_empty()`
  is therefore true only before the player's first play of the round -
  the counter counts "rounds in which the player played at least one
  card", not expeditions started. To count expeditions started it would
  need to check emptiness of the specific `c.expedition` subset (cf.
  `highest_value_in_expedition` lib.rs:332-339 which does filter by
  expedition).
- Harmless today only because F39 shows the stat is never surfaced.
  Minor (correctness of a latent stat) is fair; nit would also be
  defensible, but minor matches the -2 rating.

## F41 - HAND_SIZE - hand.len() underflow (nit)

Verdict: CONFIRMED. Severity: nit is right.

Evidence:
- `game/lost-cities-1/src/lib.rs:401`:
  `let mut num = HAND_SIZE - hand.len();` (`HAND_SIZE = 8`, lib.rs:28).
  Underflows (debug panic, release wrap into a huge `drain` range which
  is clamped by the `num > dl` check at 403-405 - so in release it
  actually self-corrects to draining the whole deck, still wrong) if a
  hand ever exceeds 8. Unreachable through the normal turn cycle
  (verified: play/discard remove one, take adds one only in DrawOrTake
  after a removal). Latent; nit fits.

## F42 - hardcoded literal 2 vs PLAYERS const (nit)

Verdict: CONFIRMED (with a completeness note). Severity: nit is right.

Evidence - all four cited sites verified, plus the const:
- lib.rs:25: `const PLAYERS: usize = 2;`
- lib.rs:144: `for p in 0..2 {` (end_round)
- lib.rs:230: `self.current_player = (self.current_player + 1) % 2;`
- lib.rs:501: `(player + 1) % 2` (pub fn opponent)
- lib.rs:616: `(0..2).map(|p| (p, self.player_score(p) as i32))`
- `PLAYERS` is genuinely used elsewhere (lib.rs:124 `0..PLAYERS`,
  lib.rs:509 start validation, lib.rs:634 `points()`).

Note: the list is not exhaustive - `player_count()` returns bare `2`
(lib.rs:642), `player_counts()` returns `vec![2]` (lib.rs:638), and
`start()` uses literal `min: 2, max: 2` (lib.rs:511-512). Does not change
the substance or severity.

## F43 - score() is_none()-guarded unwrap() (nit)

Verdict: CONFIRMED. Severity: nit is right.

Evidence - `game/lost-cities-1/src/lib.rs:680-687`:
```rust
expeditions().iter().fold(0, |acc, &e| {
    let cards = exp_cards.get(&e);
    if cards.is_none() {
        return acc;
    }
    acc + (exp_sum.get(&e).unwrap_or(&0) - 20) * (exp_inv.get(&e).unwrap_or(&0) + 1)
        + if cards.unwrap() >= &8 { 20 } else { 0 }
})
```
Safe only via the early return; `if let Some(cards)` is the idiomatic
form. Pure style; nit correct.

## F44 - render.rs throwaway empty Vecs in map lookups (nit)

Verdict: CONFIRMED. Severity: nit is right.

Evidence - `game/lost-cities-1/src/render.rs:185` and :196:
```rust
largest = cmp::max(largest, by_exp.get(&e).unwrap_or(&vec![]).len());
...
match by_exp.get(&e).unwrap_or(&vec![]).get(row_i) {
```
Both allocate a temporary empty `Vec<Card>` per lookup (5 expeditions x
rows). Render-only, trivial cost; nit correct.

## Summary

| Finding | Verdict | Severity |
|---------|-----------|----------------------------|
| F36 | CONFIRMED | major (keep) |
| F37 | ADJUSTED | major -> minor (align -2) |
| F38 | CONFIRMED | minor (keep) |
| F39 | CONFIRMED | minor (keep) |
| F40 | CONFIRMED | minor (keep) |
| F41 | CONFIRMED | nit (keep) |
| F42 | CONFIRMED | nit (keep; site list not exhaustive) |
| F43 | CONFIRMED | nit (keep) |
| F44 | CONFIRMED | nit (keep) |
