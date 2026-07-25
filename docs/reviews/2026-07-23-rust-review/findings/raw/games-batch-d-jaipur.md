# Raw findings — games-batch-d — jaipur-2

Crate: `rust/game/jaipur-2` (snapshot at brdgme-review-snapshot). Reviewed 2026-07-23.
Scope: `src/lib.rs`, `src/command.rs`, `src/render.rs`, `tests/contract.rs`, Cargo.toml; binaries skimmed only.

Notes on method:
- No `jaipur` source exists in `brdgme-go` in the snapshot (searched by name), so several
  findings below are judged directly against the official Jaipur rulebook (fetched from
  https://www.rulespal.com/jaipur/rulebook) rather than against a Go port.
- Official rulebook facts used: 55 cards = 6/6/6 diamond/gold/silver + 8/8 cloth/spice +
  10 leather + **11 camels**; 38 goods tokens; 18 bonus tokens (7x3-sale, 6x4-sale, 5x5-sale);
  bonus token awarded "if you sell 3 or more cards"; camel token worth 5; tie-break = most
  bonus tokens then most goods tokens; NEW ROUND: "The player who lost the previous round
  starts."

### Deck has 8 camels / 52 cards; official game has 11 camels / 55 cards
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:105
- finding: `Good::card_count` returns 8 for `Good::Camel`, giving a 52-card deck (test
  `deck_has_52_cards` at lib.rs:838 bakes this in). The official Jaipur rulebook material list
  is 55 cards with 11 camels (6 diamond, 6 gold, 6 silver, 8 cloth, 8 spice, 10 leather,
  11 camels). All other card counts match official; only camels are short by 3. Judged against
  official rules, not a Go port (no Go Jaipur exists in the snapshot). Fewer camels shifts
  game balance (camel trades, camel bonus, market refresh frequency).
- recommendation: Change `Good::Camel => 8` to `=> 11` in `card_count` (lib.rs:97-107), and
  update the affected tests (`deck_has_52_cards` -> 55, `start_deck_is_40` -> 43) and any
  docs that quote deck size. If 8 camels was deliberate, document it as a house rule in
  RULES.md and DATA_DOCS.md.

### No bonus token awarded for selling 6 or 7 cards at once
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:521
- finding: `sell()` awards a bonus only via `self.bonuses.get_mut(&quantity)`, whose map keys
  are exactly 3, 4, 5 (`bonus_sizes()` = 3..=5). A sale of 6 or 7 cards (possible with
  leather/cloth/spice under the 7-card hand limit) gets no bonus token. Official rulebook
  Step 3 says "If you sell 3 or more cards, take the corresponding bonus token" — with only
  3/4/5 piles existing, a 6+ sale takes from the 5-sale pile (this is the common
  interpretation; the crate's own renderer agrees: render.rs:153 labels the 5-bonus column
  "5 or more", and DATA_DOCS.md says bonuses are "awarded when selling 3+ of a good at once").
  So the code contradicts both the official rulebook and its own UI/docs. Judged against
  official rules.
- recommendation: Map quantities >= 5 to key 5 when awarding the bonus, e.g.
  `let bonus_key = if quantity >= MAX_TRADE_BONUS { MAX_TRADE_BONUS } else { quantity };`
  then `self.bonuses.get_mut(&bonus_key)`. Add a regression test selling 6+ leather with a
  non-empty 5-bonus pile.

### Next-round starting player is not the round loser
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:638
- finding: Official rulebook NEW ROUND: "The player who lost the previous round starts."
  `end_round()` never adjusts `current_player`. When a round ends via `sell()` (3 depleted
  token piles), `next_player()` is skipped (lib.rs:571-575), so the seller — usually the
  round winner, since their sale ended the round — starts the next round. When a round ends
  via deck exhaustion in `replenish_market()`, `next_player()` is also skipped
  (lib.rs:343-345, 474-476), so the player who took cards starts. Neither path implements
  "loser starts"; the common case (sale ends round) is the exact opposite of the rule.
  Judged against official rules.
- recommendation: Track the round loser in `end_round()` (opponent of `winner`; on a full
  tie, keep current behavior or alternate) and set `self.current_player = loser` before
  calling `start_round()`.

### Camel token counted as a "bonus token" for end-of-round tie-breaks
- severity: minor
- category: correctness
- location: game/jaipur-2/src/lib.rs:598
- finding: `end_round()` does `self.bonus_tokens[cw] += 1` when awarding the 5-point camel
  token, and `bonus_tokens` is the first tie-break (lib.rs:617-620). Per the official
  rulebook the camel token is a distinct component (1 camel token vs 18 bonus tokens) and
  the tie-break is "the player with the most bonus tokens" — so the camel-token winner gets
  an arguably unwarranted edge in the first tie-break. Ambiguous wording, but the material
  list separates the two token kinds. Judged against official rules.
- recommendation: Track the camel token separately (e.g. a `camel_token: [bool; 2]` or add
  its 5 points to a score accumulator rather than `tokens`), keep `bonus_tokens` counting
  only 3/4/5-sale bonus tokens, and use that for the tie-break.

### RULES.md is a one-line stub; `Gamer::rules()` returns just "# Jaipur"
- severity: minor
- category: quality
- location: game/jaipur-2/RULES.md:1
- finding: RULES.md contains only the heading `# Jaipur` (1 line), so `Gamer::rules()`
  (lib.rs:816-818) serves players an empty rules page, while the crate otherwise ships full
  BASIC_STRATEGY.md / ADVANCED_STRATEGY.md / DATA_DOCS.md. Players have no in-app way to
  learn the game.
- recommendation: Write a real RULES.md (setup, take/sell actions, bonus tokens, camel bonus,
  round end, best-of-3 match) consistent with the implemented rules.

### `sell dia gold` (mixed types) silently becomes "sell 2 diamonds"
- severity: minor
- category: correctness
- location: game/jaipur-2/src/command.rs:76-85
- finding: The second sell sub-parser takes `Many::some_spaced(trade_good_parser())` and maps
  to `Command::Sell { good: goods.first()..., quantity: goods.len() }`, discarding the rest
  of the type list. Typing `sell dia gold lea` produces `Sell { Diamond, 3 }`; the resulting
  error ("you only have N of that good") or — worse — a successful unintended sale of N
  diamonds confuses the player. The parser should reject mixed-type sales outright.
- recommendation: In the `Map` closure, validate `goods.iter().all(|&g| g == goods[0])` and
  fail the parse otherwise (or add a dedicated mixed-type check in `sell()` with a clear
  error message).

### Dead branch `if parsers.is_empty()` in command_parser
- severity: nit
- category: simplicity
- location: game/jaipur-2/src/command.rs:16-23
- finding: `parsers` is unconditionally populated with take and sell parsers, so
  `parsers.is_empty()` can never be true; the `None` arm is dead code (the real `None`
  condition is handled earlier at line 13). Looks copied from games whose parser list is
  state-dependent.
- recommendation: Replace the `if/else` with `Some(Box::new(OneOf::new(parsers)))`.

### Silent `unwrap_or(Good::Diamond)` fallback in sell parser
- severity: nit
- category: quality
- location: game/jaipur-2/src/command.rs:79
- finding: `goods.first().copied().unwrap_or(Good::Diamond)` defaults to Diamond on an empty
  vec. `Many::some_spaced` guarantees at least one element, so the fallback is unreachable,
  but a silent arbitrary default would mask a parser regression. Not a player-reachable
  panic, so not a rule violation — just fragile style.
- recommendation: Use `goods[0]` / `goods.first().expect(...)` (test-acceptable) or
  destructure so the non-empty invariant is enforced loudly.

### Placings-log block duplicated between Take and Sell arms
- severity: nit
- category: simplicity
- location: game/jaipur-2/src/lib.rs:754
- finding: The `if self.is_finished() { ... gen_placings ... placings_log ... }` block is
  copy-pasted verbatim in the `Command::Take` arm (lib.rs:754-764) and the `Command::Sell`
  arm (lib.rs:777-787). Both arms are otherwise identical apart from the state mutation.
- recommendation: Collapse the two match arms to compute `logs` first
  (`match value { Take{..} => ..., Sell{..} => ... }`) and share the single
  is_finished/placings block afterwards.

### "N rounds remaining" overstates remaining rounds
- severity: nit
- category: correctness
- location: game/jaipur-2/src/render.rs:174
- finding: `remaining_rounds = 3 - (round_wins[0] + round_wins[1])` assumes all 3 rounds will
  be played. After a 1-0 first round it renders "There are 2 rounds remaining", but the match
  ends after round 2 if the same player wins again. Cosmetic misstatement of match state.
- recommendation: Render something like "first to 2 round wins" or compute
  `2 - max(round_wins)` (rounds the leader still needs), or reword to avoid a numeric claim.

### Opponent camel display leaks exact-zero information
- severity: nit
- category: consistency
- location: game/jaipur-2/src/render.rs:40-42
- finding: `camel_display` maps 0 -> "no" and everything else -> "some", so the opponent view
  ("no camels" vs "some camels") hides counts but reveals exactly when a herd is empty —
  a meaningful tactical fact (opponent cannot camel-trade). Meanwhile `PubState.camels`
  exposes exact counts over the JSON API anyway (test `pub_state_camels_are_exact`), so the
  obfuscation is inconsistent: hidden in the renderer, public in the data. The rulebook only
  says players are not *required* to disclose counts.
- recommendation: Pick one policy: either show exact camel counts in the renderer (simplest,
  consistent with PubState) or clamp in PubState too.

## Cross-references (not findings)

- `src/bin/jaipur_2_http.rs:11` `.expect("Invalid socket address")` and tokio `full` feature
  in Cargo.toml: process-startup panic + heavy dep, part of the systemic boilerplate-binary
  pattern tracked in the dependencies unit — not reported per-binary.
- All 4 binaries (`_cli`, `_repl`, `_fuzz`, `_http`) match the systemic boilerplate exactly;
  no per-crate deviation observed.
- `render.rs:17` comment references Go's `strings.Join(RenderGoods(...), "  ")` — a Go-port
  lineage comment, but no Go Jaipur source exists in the snapshot to compare against.
- Command-parser combinator usage (`OneOf`, `Many::some_spaced`, `Doc`, etc.) is the
  deliberate lib/game parser design — not flagged.

## Clean areas (verified, no findings)

- No `.unwrap()` / `.expect()` / `panic!()` / `unreachable!()` in runtime paths reachable
  from player commands (all such uses are in `#[cfg(test)]` or the startup-only http bin).
  Slicing (`goods_pile[..num_tokens]`) is length-clamped; array indexing uses fixed `[T; 2]`
  with player indices already gated by `command_parser`.
- Token tables match official exactly: goods token values/counts (diamond 7,7,5,5,5; gold
  6,6,5,5,5; silver 5x5; cloth/spice 5,3,3,2,2,1,1; leather 4,3,2,1x6; 38 total), bonus
  piles (3-sale: 3,3,2,2,2,1,1; 4-sale: 6,6,5,5,4,4; 5-sale: 10,10,9,8,8), camel token = 5.
- Core rules correct: market of 5 with 3 initial camels; single-good take; take-all-camels;
  multi-good exchange with equal counts, no same-type swap, 2-for-2 minimum; hand limit 7
  (camels exempt); rare-good minimum sale of 2; round end on 3 depleted piles or deck
  exhaustion; camel bonus only on strict majority; tie-break order points -> bonus tokens ->
  goods tokens; best-of-3 with first-to-2.
- `take_goods` performs all validation before any mutation (verified: mutations start at
  lib.rs:447, after every error path); failed commands leave state untouched (also covered
  by test `take_goods_rejects_single_camel_without_mutation`).
- Bonus token value is revealed privately only to the seller (lib.rs:554-559), matching the
  hidden-bonus rule; `PubState` exposes only bonus pile counts, and the no-hand-leak property
  is test-covered.
- Test coverage is strong: ~45 unit tests plus the shared `assert_gamer_contract` harness in
  tests/contract.rs; parser, rules-validation, tie-break, render, and serde round-trip paths
  all covered.
- Dependencies lean: rand, serde, brdgme_* path deps; serde_json dev-only; tokio is pulled
  solely for the boilerplate http binary (systemic, cross-referenced above).
