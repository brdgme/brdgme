# Findings — games-batch-d (2026-07-23 review)

Crates: `game/lords-of-vegas-1` (2,025 LOC), `game/jaipur-2` (1,957),
`game/sushi-go-2` (1,923), `game/modern-art-2` (1,700).
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot`, HEAD
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. Review-only; no builds run.
Raw worker findings (uncurated, including verified-clean notes and
cross-references) are preserved under `findings/raw/games-batch-d-*.md`.

Summary: 1 critical, 9 major, 14 minor, 22 nit = 46 findings.
The 4 boilerplate binaries per crate were skimmed in each crate; all match the
systemic pattern exactly with no per-crate deviation (systemic issues tracked
in the dependencies unit).

## lords-of-vegas-1

Context: deliberately partial port — RULES.md "Implementation status" and
in-code comments state only `build`/`done` are implemented (no card draws,
payouts, scoring, or endgame). Documented WIP items are cross-references in
the raw file, not findings. Go source not in the snapshot; judged against
official Lords of Vegas rules where noted.

### unimplemented!() panic macros in runtime command dispatch
- severity: major
- category: quality
- location: game/lords-of-vegas-1/src/lib.rs:182-186
- finding: The `command()` dispatch maps `Command::Remodel`, `Reorg`,
  `Sprawl`, `Gamble` and `Raise` to `unimplemented!()`. Repo rules forbid
  panic macros in runtime paths reachable from player commands. Today these
  arms are unreachable only because `command_parser()` (command.rs:20-27)
  wires in just the build/done parsers — but the five other parsers
  (command.rs:49-152) are fully written, `pub`, and one line away from being
  added to the `OneOf`. The moment any is wired in, a valid player command
  panics the process instead of returning a `GameError`.
- recommendation: Replace each `unimplemented!()` arm with
  `Err(GameError::InvalidInput { message: "not yet implemented".into() })`
  until the real implementation lands, so dispatch can never panic regardless
  of parser wiring.

### Nondeterministic HashMap/HashSet iteration feeds RNG-dependent boss-tie resolution
- severity: major
- category: correctness
- location: game/lords-of-vegas-1/src/board.rs:248, game/lords-of-vegas-1/src/board.rs:278, game/lords-of-vegas-1/src/board.rs:314-344
- finding: `casino_at()` pops BFS candidates from a `HashSet<Loc>` via
  `queue.iter().next()` (board.rs:248) and `casinos()` iterates `TILES.keys()`
  (board.rs:278), a `HashMap` — both have per-process random iteration order.
  `resolve_boss_ties()` rerolls boss dice in that order (board.rs:330-332),
  consuming the seeded `GameRng` stream. Two processes replaying the same game
  from the same seed/state can reroll tied tiles in different orders,
  producing divergent game states. Breaks deterministic replay/audit.
- recommendation: Make iteration deterministic: collect candidate locs into a
  `Vec`, `sort()` them (`Loc` derives `Ord`), and iterate that — both in the
  `casino_at` BFS and in `casinos()` — or switch `TILES`/`Board` to `BTreeMap`.

### resolve_boss_ties never populates its log output
- severity: minor
- category: correctness
- location: game/lords-of-vegas-1/src/board.rs:314-344
- finding: `resolve_boss_ties` builds a `logs: Vec<Log>` but nothing is ever
  pushed — the die returned by `self.reroll_at(&bt.loc, rng)` is discarded at
  board.rs:331. On a boss tie the function returns `Some(vec![])`; `build()`
  (lib.rs:308-311) extends logs with the empty vec and sets `can_undo =
  false`. Dice are silently rerolled, the player sees no record, and undo is
  disabled for a change they were never told about.
- recommendation: Push a public log per reroll (e.g. "boss tie at <casino>,
  <player>'s die at <loc> rerolled to <n>") using the value from `reroll_at`,
  matching the cascade described in RULES.md.

### usize underflow panics in renderer when supplies are exceeded
- severity: minor
- category: correctness
- location: game/lords-of-vegas-1/src/render.rs:80, game/lords-of-vegas-1/src/render.rs:85, game/lords-of-vegas-1/src/render.rs:117
- finding: `PLAYER_DICE - used.dice`, `PLAYER_OWNER_TOKENS - used.tokens`, and
  `CASINO_TILES - self.board.casino_tile_count(*casino)` can underflow `usize`
  (panic in debug, huge number in release) because `build()` (lib.rs:251-313)
  never enforces the die supply (12/player), token supply (10/player), or the
  9-tiles-per-casino supply — and the 3 strip lots can be built as any colour.
  Latent (hard to reach in the current partial implementation).
- recommendation: Enforce supply limits in `build()` (return
  `GameError::InvalidInput` when out of dice/tokens/casino tiles), and/or use
  `saturating_sub` in the renderer.

### Loc::parse_str accepts out-of-range lots; neighbours() underflows on lot 0
- severity: minor
- category: quality
- location: game/lords-of-vegas-1/src/board.rs:80-91, game/lords-of-vegas-1/src/board.rs:106-108
- finding: `Loc::parse_str` (used by the `Deserialize` impl, i.e. on loaded
  game state) accepts any numeric lot — "A0", "A99" — without validating
  `1..=block.max_lot()`. `Loc::neighbours()` then computes `self.lot - 1`
  (board.rs:107), which underflows for lot 0 and can emit nonexistent locs.
  Not reachable from player commands (the parser uses `Enum::exact` over valid
  locs), only from crafted/corrupt state.
- recommendation: Validate the lot range in `parse_str` (`if lot < 1 || lot >
  block.max_lot() { return Err(...) }`).

### lazy_static used for TILES instead of std OnceLock/once_cell
- severity: minor
- category: dependencies
- location: game/lords-of-vegas-1/src/tile.rs:3,23-25, game/lords-of-vegas-1/Cargo.toml:15
- finding: `lazy_static` is in maintenance mode; the ecosystem prefers
  `once_cell` or `std::sync::OnceLock`. `TILES` is the only use in the crate.
- recommendation: Replace with `static TILES: OnceLock<TileMap> =
  OnceLock::new()` plus a getter, or `once_cell::sync::Lazy`, matching
  whatever the rest of the workspace settles on.

### unreachable!() in starting-cash fold during game start
- severity: nit
- category: quality
- location: game/lords-of-vegas-1/src/lib.rs:118
- finding: `Card::GameEnd => unreachable!()` when summing starting cash.
  Provably unreachable — `shuffled_deck` inserts GameEnd at position >= 39
  while players drain at most 12 cards from the front — but the invariant
  lives in another file (card.rs:31-33).
- recommendation: Add a short comment stating the invariant, or an
  `unreachable!` message explaining why GameEnd cannot be in a starting hand.

### serde_json is a runtime dependency but only used in tests
- severity: nit
- category: dependencies
- location: game/lords-of-vegas-1/Cargo.toml:18, game/lords-of-vegas-1/src/lib.rs:350
- finding: The only `serde_json` use is the `json_works` unit test, yet it is
  declared under `[dependencies]`, needlessly adding it to dependents' builds.
- recommendation: Move `serde_json` to `[dev-dependencies]`.

### Redundant `use std::iter::FromIterator` import
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/board.rs:3
- finding: `FromIterator` is in the prelude for edition 2024; the explicit
  import (used for `HashSet::from_iter` at board.rs:320) is redundant.
- recommendation: Delete the import.

### Hardcoded literal 3 instead of BLOCK_WIDTH in renderer
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/render.rs:154-155
- finding: `render_block` computes tile coordinates with `(lot - 1) % 3` /
  `(lot - 1) / 3` while board.rs:16 defines `BLOCK_WIDTH: usize = 3` for
  exactly this. If the grid width ever changed, logic and rendering would
  silently disagree.
- recommendation: Use `BLOCK_WIDTH` (re-export it from board.rs or duplicate
  the constant near the render code).

### Casino colours in code don't match RULES.md descriptions
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/casino.rs:28-34
- finding: RULES.md describes Sphinx as "Tan/olive" and Pioneer as "Brick
  red", but `Casino::color()` maps Sphinx to `NamedColor::Orange` and Pioneer
  to `NamedColor::Brown`. Presumably a `brdgme_color` palette limitation, but
  doc and UI now tell the player different colour names.
- recommendation: Adjust RULES.md wording to the actual rendered colours, or
  note the approximation; no code change required.

### Player counts 2-6 deviate from official rules (2-4)
- severity: nit
- category: correctness
- location: game/lords-of-vegas-1/src/lib.rs:97-103, game/lords-of-vegas-1/src/lib.rs:225-227
- finding: Judged against official rules, the base game supports 2-4 players;
  this implementation allows 2-6. May be a deliberate house extension (Go
  source not in the snapshot to confirm). Deck math handles 6 players fine;
  rules-fidelity note, not a bug.
- recommendation: Confirm intended player-count range; if 2-6 is deliberate,
  mention it in RULES.md.

Verified clean: tile data table (48 lots, 9 cards/casino + 3 strip lots, die
values/costs match RULES.md); `Loc::neighbours` grid geometry; `casino_at`
flood fill; `build()` validation/error paths; `shuffled_deck` GameEnd
placement; command parsing and `command_spec` gating; serde round-trip. No
other player-reachable panics. Test coverage is thin (no build-flow or
tie-resolution tests), acceptable for an explicitly partial crate but worth
noting.

## jaipur-2

Context: no Go Jaipur source exists in the snapshot, so rule findings are
judged directly against the official Jaipur rulebook. Otherwise a strong
crate: ~45 unit tests plus the contract harness, no player-reachable panics,
token tables verified exact against official.

### Deck has 8 camels / 52 cards; official game has 11 camels / 55 cards
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:105
- finding: `Good::card_count` returns 8 for `Good::Camel`, giving a 52-card
  deck (test `deck_has_52_cards` at lib.rs:838 bakes this in). The official
  material list is 55 cards with 11 camels (6/6/6 diamond/gold/silver, 8/8
  cloth/spice, 10 leather, 11 camels). All other card counts match official;
  only camels are short by 3. Judged against official rules (no Go port to
  compare). Fewer camels shifts game balance (camel trades, camel bonus,
  market refresh frequency).
- recommendation: Change `Good::Camel => 8` to `=> 11` in `card_count`
  (lib.rs:97-107) and update affected tests (`deck_has_52_cards` → 55,
  `start_deck_is_40` → 43) and docs quoting deck size. If 8 was deliberate,
  document it as a house rule in RULES.md and DATA_DOCS.md.

### No bonus token awarded for selling 6 or 7 cards at once
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:521
- finding: `sell()` awards a bonus only via `self.bonuses.get_mut(&quantity)`,
  whose map keys are exactly 3, 4, 5. A sale of 6 or 7 cards (possible with
  leather/cloth/spice under the 7-card hand limit) gets no bonus token.
  Official rules: "If you sell 3 or more cards, take the corresponding bonus
  token" — with only 3/4/5 piles existing, a 6+ sale takes from the 5-sale
  pile; the crate's own renderer agrees (render.rs:153 labels the column "5 or
  more") and DATA_DOCS.md says bonuses are "awarded when selling 3+ of a
  good". Code contradicts both the official rulebook and its own UI/docs.
- recommendation: Clamp the bonus key (`let key = quantity.min(5);`) when
  awarding, and add a regression test selling 6+ leather with a non-empty
  5-bonus pile.

### Next-round starting player is not the round loser
- severity: major
- category: correctness
- location: game/jaipur-2/src/lib.rs:638
- finding: Official rulebook, NEW ROUND: "The player who lost the previous
  round starts." `end_round()` never adjusts `current_player`. When a round
  ends via `sell()` (3 depleted token piles), `next_player()` is skipped
  (lib.rs:571-575), so the seller — usually the round winner — starts the next
  round; deck-exhaustion endings (lib.rs:343-345, 474-476) likewise leave the
  active player in place. The common case is the exact opposite of the rule.
- recommendation: Track the round loser in `end_round()` (opponent of
  `winner`; define behaviour on a full tie) and set `self.current_player =
  loser` before `start_round()`.

### Camel token counted as a "bonus token" for end-of-round tie-breaks
- severity: minor
- category: correctness
- location: game/jaipur-2/src/lib.rs:598
- finding: `end_round()` does `self.bonus_tokens[cw] += 1` when awarding the
  5-point camel token, and `bonus_tokens` is the first tie-break
  (lib.rs:617-620). Per the official material list the camel token is a
  distinct component from the 18 bonus tokens, and the tie-break is "most
  bonus tokens" — so the camel-token winner gets an arguably unwarranted edge
  in the first tie-break. Judged against official rules.
- recommendation: Track the camel token separately (e.g. add its 5 points to a
  score accumulator) and keep `bonus_tokens` counting only 3/4/5-sale bonus
  tokens for the tie-break.

### RULES.md is a one-line stub; `Gamer::rules()` returns just "# Jaipur"
- severity: minor
- category: quality
- location: game/jaipur-2/RULES.md:1
- finding: RULES.md contains only the heading `# Jaipur`, so `rules()`
  (lib.rs:816-818) serves players an empty rules page, while the crate ships
  full BASIC_STRATEGY.md / ADVANCED_STRATEGY.md / DATA_DOCS.md.
- recommendation: Write a real RULES.md (setup, take/sell actions, bonus
  tokens, camel bonus, round end, best-of-3 match) consistent with the
  implemented rules.

### `sell dia gold` (mixed types) silently becomes "sell N diamonds"
- severity: minor
- category: correctness
- location: game/jaipur-2/src/command.rs:76-85
- finding: The second sell sub-parser takes
  `Many::some_spaced(trade_good_parser())` and maps to `Command::Sell { good:
  goods.first()..., quantity: goods.len() }`, discarding the rest of the type
  list. `sell dia gold lea` produces `Sell { Diamond, 3 }` — a confusing
  error or, worse, a successful unintended sale.
- recommendation: In the `Map` closure, validate `goods.iter().all(|&g| g ==
  goods[0])` and fail the parse otherwise (or reject mixed-type sales in
  `sell()` with a clear error message).

### Dead branch `if parsers.is_empty()` in command_parser
- severity: nit
- category: simplicity
- location: game/jaipur-2/src/command.rs:16-23
- finding: `parsers` is unconditionally populated with take and sell parsers,
  so `parsers.is_empty()` can never be true; the `None` arm is dead code (the
  real `None` condition is handled earlier at line 13).
- recommendation: Replace the `if/else` with `Some(Box::new(OneOf::new(parsers)))`.

### Silent `unwrap_or(Good::Diamond)` fallback in sell parser
- severity: nit
- category: quality
- location: game/jaipur-2/src/command.rs:79
- finding: `goods.first().copied().unwrap_or(Good::Diamond)` defaults to
  Diamond on an empty vec. `Many::some_spaced` guarantees at least one
  element, so the fallback is unreachable, but a silent arbitrary default
  would mask a parser regression.
- recommendation: Index `goods[0]` or destructure so the non-empty invariant
  is enforced loudly.

### Placings-log block duplicated between Take and Sell arms
- severity: nit
- category: simplicity
- location: game/jaipur-2/src/lib.rs:754
- finding: The `if self.is_finished() { ... gen_placings ... placings_log
  ... }` block is copy-pasted verbatim in the `Command::Take` arm
  (lib.rs:754-764) and the `Command::Sell` arm (lib.rs:777-787).
- recommendation: Collapse the two match arms to compute `logs` first and
  share the single is_finished/placings block afterwards.

### "N rounds remaining" overstates remaining rounds
- severity: nit
- category: correctness
- location: game/jaipur-2/src/render.rs:174
- finding: `remaining_rounds = 3 - (round_wins[0] + round_wins[1])` assumes
  all 3 rounds will be played. After a 1-0 first round it renders "There are 2
  rounds remaining", but the match ends after round 2 if the same player wins
  again. Cosmetic misstatement of match state.
- recommendation: Render "first to 2 round wins" or reword to avoid a numeric
  claim.

### Opponent camel display leaks exact-zero information
- severity: nit
- category: consistency
- location: game/jaipur-2/src/render.rs:40-42
- finding: `camel_display` maps 0 → "no" and everything else → "some", hiding
  counts but revealing exactly when a herd is empty — while
  `PubState.camels` exposes exact counts over the JSON API anyway (test
  `pub_state_camels_are_exact`). The obfuscation is inconsistent: hidden in
  the renderer, public in the data.
- recommendation: Pick one policy: show exact camel counts in the renderer
  (simplest, consistent with PubState) or clamp in PubState too.

Verified clean: goods/bonus/camel token tables match official exactly; core
rules correct (market of 5, single-good take, take-all-camels, multi-good
exchange rules, hand limit 7, rare-good minimum sale, round-end triggers,
camel-bonus majority, tie-break order, best-of-3); `take_goods` validates
fully before mutating; bonus value revealed privately only to the seller; no
player-reachable panics.

## sushi-go-2

Context: original Sushi Go (108-card set, 2-5 players, published 2-player
dummy variant). Card counts, hand sizes, and per-type scoring verified correct
against official Gamewright rules; Go port (`brdgme-go/sushi_go_1`)
cross-referenced for inherited quirks. No major findings.

### Round 2 passes hands to the right; official Sushi Go passes left every round
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:361
- finding: `end_hand` passes hands left in odd rounds and right in even
  rounds; official Sushi Go passes left every round (alternating direction is
  a 7 Wonders mechanic). Inherited verbatim from the Go port and documented in
  the shipped RULES.md, so it is self-consistent and communicated to players;
  flagged only as a published-rules deviation. Irrelevant in 2-player games.
- recommendation: If rules fidelity is desired, pass left every round (drop
  the `round % 2` branch). Otherwise close as a deliberate, documented
  deviation.

### Pudding scoring: all-tied case awards nothing; official rules split the +6
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:492
- finding: In round-3 pudding scoring, if `first == last` no points are
  awarded. In 3-5 player games this nets to 0 either way, but in a 2-player
  game (no fewest-pudding penalty) an all-tied table should still split the +6
  "most puddings" award; this code awards 0. Additionally the dummy slot is
  included in the pudding comparison in 2p games and can win/dilute the +6 —
  rules-ambiguous, Go port behaves the same. Edge case, low impact.
- recommendation: In the `first == last` branch, for `players == 2` award `6 /
  first_players.len()` to the tied slots (or document the deviation); decide
  and document whether the dummy participates in pudding scoring.

### Final placings break score ties by pudding count; official rules leave ties standing
- severity: minor
- category: correctness
- location: game/sushi-go-2/src/lib.rs:273-278
- finding: `placings()` uses `[points, pudding_cards]`, breaking points ties
  by pudding count; published rules specify no tiebreaker. Inherited from the
  Go port and locked in by `test_placings_pudding_tiebreaker` (lib.rs:1461).
  Defensible for a ranked platform, but undocumented in RULES.md.
- recommendation: Document the pudding tiebreak in RULES.md, or drop the
  pudding component and let `gen_placings` share placings on tied points.

### `test_hand_passing_left` is a vacuous, self-contradicting test
- severity: minor
- category: quality
- location: game/sushi-go-2/src/lib.rs:1398-1416
- finding: The test deliberately triggers an `unwrap_err()`, calls
  `g.end_hand()` directly, carries comments admitting the scenario doesn't
  work, and asserts nothing. It provides no coverage and misleads readers into
  thinking hand-passing is tested there (real coverage is
  `test_passing_direction` at lib.rs:1418).
- recommendation: Delete the test, or replace its body with real assertions on
  hand passing.

### `draw_count` has an unreachable (2, 9) entry and a silent `.unwrap_or(9)` fallback
- severity: minor
- category: quality
- location: game/sushi-go-2/src/lib.rs:140-150
- finding: `player_draw_counts()` includes `(2, 9)`, but `draw_count` is only
  ever called with `self.all_players` (3 in 2-player games) — the 2-player
  entry is dead. The `.unwrap_or(9)` fallback would silently mask any future
  out-of-range caller. (Hand sizes themselves are correct: 3p=9, 4p=8, 5p=7,
  2p dummy deals as 3-player.)
- recommendation: Replace the lookup with an exhaustive `match players { 2 | 3
  => 9, 4 => 8, 5 => 7, ... }` or drop the dead entry and document why.

### Pudding hint text claims "least -6" which is false in 2-player games
- severity: nit
- category: correctness
- location: game/sushi-go-2/src/lib.rs:123
- finding: `Card::Pudding.explanation()` returns "end: most 6, least -6" and
  is shown in every player's hand table, but the fewest-pudding penalty is
  (correctly) not applied in 2-player games (lib.rs:513). 2p players see a
  rule hint that doesn't apply to their game.
- recommendation: Make the explanation context-aware, or note "(no penalty in
  2p)".

### Maki second-place cap `second_players.len() <= 3` is an arbitrary, silent guard
- severity: nit
- category: correctness
- location: game/sushi-go-2/src/lib.rs:440
- finding: Second-place maki points are only awarded if at most 3 players tie
  for second. The only excluded case (4-way second-place tie in a 5p game)
  would split 3 points rounding down — 0 each — so the score outcome is
  identical; the only effect is the explanatory log line is silently omitted.
  The cap looks like a rule but isn't one.
- recommendation: Drop the `<= 3` condition (integer division already yields
  0), or comment that it deliberately suppresses the log for that scenario.

### `render_name` logic duplicated between lib.rs and render.rs
- severity: nit
- category: consistency
- location: game/sushi-go-2/src/lib.rs:251-260, game/sushi-go-2/src/render.rs:39-48
- finding: `Game::render_name` and the free `render_name` in render.rs are
  byte-for-byte the same "<dummy>"-vs-`N::Player` logic. Both also compute
  `player > players - 1`, which would underflow if ever called with `players
  == 0` — not reachable in practice, but fragile.
- recommendation: Share one helper and write the check as `player >= players`.

### Two-card "save one for the dummy" guard evaluates `playing[DUMMY]` before the `players == 2` check
- severity: nit
- category: quality
- location: game/sushi-go-2/src/lib.rs:675-683
- finding: The guard chain reads `self.playing[DUMMY].is_none()` before
  `self.players == 2`; in 3-5 player games that reads an unrelated player's
  slot before short-circuiting. Memory-safe and logically harmless today, but
  confusing and would panic if `all_players` ever dropped below 3.
- recommendation: Reorder so `self.players == 2` comes first, matching the
  guard style in `can_dummy` (lib.rs:248).

### `command()` duplicates the finished-game placings-log block across both match arms
- severity: nit
- category: simplicity
- location: game/sushi-go-2/src/lib.rs:838-874
- finding: The `Command::Play` and `Command::Dummy` arms each repeat the same
  12-line finished-check/placings-log/CommandResponse sequence; only the
  handler call differs.
- recommendation: Bind the `Result<Vec<Log>, GameError>` from either arm in
  one `match`, then run the finished-check/response construction once.

Verified clean: no player-reachable panics (all indexing bounds-checked;
`can_play`/`can_dummy` guard every `playing[player]` access); scoring verified
against official rules (maki tie-splitting, tempura/sashimi, dumpling cap,
wasabi-triples-next-nigiri, pudding 2p exemption, chopsticks flows); hand-size
synchronization traced for normal and chopsticks double-play sequences;
hidden-information handling correct. Context note: the known lib/game suggest
`Many`-ignores-`max` defect is cosmetically user-visible via `play_parser`
(command.rs:42) — parse/validation enforces the real bound; tracked
cross-unit, not re-reported.

## modern-art-2

Context: Go port source (`brdgme-go/modern_art_1/modern_art.go`) present in
the snapshot and used for port-parity comparison; every logic deviation
flagged is Go-inherited (noted per finding). Judged against official Modern
Art (Knizia) rules where stated. `card.rs`, `command.rs`, and the contract
test are clean; test coverage otherwise good but does not exercise the round-4
empty-hand scenarios below.

### Infinite busy-loop (hang + unbounded log growth) when all hands empty after a settle
- severity: critical
- category: correctness
- location: game/modern-art-2/src/lib.rs:452
- finding: `settle_auction` advances `current_player` then loops `while
  self.player_hands[self.current_player].is_empty() { ...; self.next_player();
  }` with no guard for the case where *every* player has an empty hand — the
  loop never terminates, pushing a "Skipping ..." log entry every iteration
  (unbounded memory growth → worker hang/OOM). Reachable via legal play: the
  round only ends when a 5th card of an artist is played; in round 4 no cards
  are dealt, and players can collectively play out their hands with at most 4
  per artist (no 5-card trigger). When the last auction settles, all hands are
  empty and the loop spins. Judged against official rules the game is also
  stuck here, so a fix must decide the round ends. Port-inherited (Go
  `modern_art.go:690` identical).
- recommendation: Check whether all players' hands are empty before/inside the
  skip loop; if so call `end_round()` (or bound the loop to `self.players`
  iterations and break into `end_round()` on a full cycle).

### Round 4 can start on an empty-handed player, soft-locking the game
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:368
- finding: `end_round` does `self.round += 1; self.next_player();
  logs.extend(self.start_round());`. Round 4 deals 0 cards, so if the next
  player has an empty hand (played out in round 3 — hands persist across
  rounds), the game enters `State::PlayCard` with a `current_player` who has
  no cards: no parser can produce a card, `pass` is unavailable in PlayCard
  state, and `whose_turn_players` returns only that player — deadlock. The
  empty-hand skip exists only in `settle_auction`, not on the round-transition
  path. Go port identical (`modern_art.go:432-434`).
- recommendation: After `next_player()` in `end_round` (or at the end of
  `start_round`), skip players with empty hands the same way `settle_auction`
  does; combined with the all-hands-empty fix above, this terminates in
  `end_round` when nobody can play.

### End-of-round payout pays cumulative value for ALL purchased cards, including non-top-3 artists
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:336
- finding: Judged against official Modern Art rules, not the Go port. The
  payout loop pays `self.suit_value(c.suit)` (cumulative cross-round value)
  for every card in `player_purchases`. Official rules: only paintings of the
  three artists that placed this season pay out; the other two artists'
  paintings are worthless that season even if the artist earned value in
  earlier rounds (CMON rulebook: "If artist is in TOP 3, add all previous
  round's values"). Concretely: an artist placing 1st in round 1 (+30) but not
  placing in round 2 — official pays $0 for its round-2 purchases, this
  implementation pays $30 each. Materially changes game economy. Go port
  identical (`modern_art.go:406-415`) and RULES.md:103-104 documents the
  implemented behavior — adjudicate port parity vs official rules.
- recommendation: Either pay only for cards whose suit is in the current
  round's `values` map (top 3), or explicitly document the deviation as an
  intentional house rule (in-code comment + RULES.md note).

### Artists with zero cards played are ranked and awarded $20/$10
- severity: major
- category: correctness
- location: game/modern-art-2/src/lib.rs:318
- finding: Judged against official Modern Art rules, not the Go port. In
  `end_round` the ranking loop initialises `highest_count = -1`, so artists
  with 0 cards on the table are still selected for 2nd ($20) and 3rd ($10)
  when fewer than 3 artists had cards played (common: a fast round ending
  5-0-0-0-0). Official rules only rank artists that actually had paintings
  played. The awarded values land in `value_board` and inflate cumulative
  values for all later rounds. Go port identical (`modern_art.go:389-403`).
- recommendation: Skip candidates with `counts[&s] == 0` (start
  `highest_count` at 0 and require strictly greater, or filter zero counts).
  If deliberate, document it.

### `unreachable!()` and unchecked indexing in `round_cards`
- severity: minor
- category: quality
- location: game/modern-art-2/src/lib.rs:95
- finding: `round_cards` hits `unreachable!()` for any player count outside
  3..=5 and indexes `table[round]` unchecked (panics for round > 3). Guarded
  in practice (`Game::start` validates player count; `end_round` finishes at
  `ROUNDS - 1`), but repo rules forbid panic macros in runtime paths and
  `Game` is `Deserialize`, so a defensive fallback is cheap insurance against
  corrupt state.
- recommendation: Use `.get(round).copied().unwrap_or(0)` and a default arm
  for unexpected player counts, or return a `GameError`.

### RULES.md says the auction winner takes the next turn; implementation (and official rules) pass clockwise from the seller
- severity: minor
- category: consistency
- location: game/modern-art-2/RULES.md:76
- finding: "The winner adds the card(s) to their purchases ... and it becomes
  their turn next" reads as the winner becoming the next auctioneer. The
  implementation (`settle_auction` → `next_player()`, lib.rs:451) passes the
  turn clockwise from the seller, matching official rules and the Go port.
  Only the doc is wrong/misleading.
- recommendation: Reword to "and the player to the seller's left auctions
  next".

### RULES.md Double-auction section omits Once Around as a valid added card
- severity: minor
- category: consistency
- location: game/modern-art-2/RULES.md:63
- finding: "Double - Works like Open, Fixed Price, or Sealed depending on the
  second card added" — but `add_card` (lib.rs:396) only rejects another
  Double, so a Once Around card can also be added (the auction then runs as
  Once Around). Implementation matches official rules; the doc is incomplete.
- recommendation: Add "Once Around" to the list in RULES.md.

### Game-ending mid-auction leaves `state = State::Auction`, so final render shows a stale auction
- severity: minor
- category: correctness
- location: game/modern-art-2/src/lib.rs:366
- finding: When the game ends (5th card played in round 4), `end_round` sets
  `self.finished = true` but never resets `self.state`, which is still
  `State::Auction`. `whose_turn_players`/`status` short-circuit on `finished`
  so there is no deadlock, but `pub_state().is_auction` stays true and the
  PubState renderer (render.rs:57) prints "<player> is auctioning <cards>"
  with an empty `auctioning` vec on the final game screen.
- recommendation: Set `self.state = State::PlayCard` in the `finished` branch
  of `end_round`.

### "Current bid: $0 by <auctioneer>" rendered before anyone has bid
- severity: nit
- category: consistency
- location: game/modern-art-2/src/render.rs:62
- finding: For any non-Sealed auction, `pub_state` sets `current_bid =
  Some(self.highest_bidder())` (lib.rs:628) and `highest_bidder` returns
  `(auctioneer, 0)` when no bids exist, so the render shows "Current bid: $0
  by <auctioneer>" even though the auctioneer has not bid. Cosmetic; Go port
  rendered the same.
- recommendation: Only render the current-bid line when `bid > 0`, or have
  `pub_state` return `None` until a real bid exists.

### Sealed/once-around bid ties are broken in favor of the auctioneer
- severity: nit
- category: correctness
- location: game/modern-art-2/src/lib.rs:193
- finding: `highest_bidder` iterates turn order starting at `current_player`
  (the auctioneer) with a strictly-greater comparison, so on tied bids the
  auctioneer wins, then the player closest clockwise. Common editions break
  ties starting from the player to the auctioneer's *left* (auctioneer loses
  ties); exact edition rule unconfirmed and Go port identical
  (`modern_art.go:496-506`), so flagged as a nit for adjudication.
- recommendation: If the official tie-break should exclude the auctioneer on
  ties, iterate from `current_player + 1` and handle the auctioneer last;
  otherwise document.

### `can_add` allocates a throwaway `Vec` via `unwrap_or(&vec![])`
- severity: nit
- category: quality
- location: game/modern-art-2/src/lib.rs:260
- finding: `!self.player_hands.get(player).unwrap_or(&vec![]).is_empty()`
  heap-allocates an empty Vec on every call purely as a fallback.
- recommendation: `self.player_hands.get(player).is_some_and(|h|
  !h.is_empty())`.

### Guarded `bid.unwrap()` in the Open-auction arm of `whose_turn_players`
- severity: nit
- category: quality
- location: game/modern-art-2/src/lib.rs:152
- finding: `p != highest_bidder && (bid.is_none() || *bid.unwrap())` — safe
  due to `||` short-circuiting, but repo rules forbid `.unwrap()` in runtime
  paths and `Option::is_none_or` expresses it without the lint risk.
- recommendation: `bid.is_none_or(|b| b > 0)`.

### Redundant `use std::default::Default;` import
- severity: nit
- category: consistency
- location: game/modern-art-2/src/lib.rs:2
- finding: `Default` is in the standard prelude; the explicit import is dead
  weight.
- recommendation: Delete the line.

Cross-cutting note for the Orchestrator: every Modern Art rules deviation
above (all-cards payout, zero-count artists placing, settle_auction infinite
loop, missing empty-hand skip on round transition, auctioneer-favorable
tie-break) is inherited verbatim from the Go port — a cross-unit decision is
needed on whether port parity or official rules win. Recommend adding
round-4 empty-hand regression tests when the critical/major items are fixed.
