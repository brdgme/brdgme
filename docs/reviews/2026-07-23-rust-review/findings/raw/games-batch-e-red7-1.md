# Raw findings: game/red7-1 (games batch E)

Reviewer: Worker subagent, review-only audit of the SNAPSHOT worktree
(`/home/beefsack/Development/brdgme-review-snapshot/rust/game/red7-1`).
No Go original exists for red7 (`brdgme-go/` confirmed absent), so all rules
correctness was judged against the official Red7 rulebook (Asmadi; cross-checked
via yucata.de/en/Rules/Red7 and asmadigames.com PDFs), NOT against a Go port.
The crate implements Advanced Red7 (canvas-draw rule + round scoring with
40/35/30 targets), not the optional Expert odd-card actions — consistent with
its RULES.md. No PORTING_NOTES.md exists.

Files reviewed: `src/lib.rs` (750 lines), `src/card.rs` (317), `src/command.rs`
(106), `src/render.rs` (186), `tests/contract.rs`, `Cargo.toml`, the 4 binaries,
`RULES.md`, `DATA_DOCS.md`.

## Verified-correct areas (no findings)

- All 7 color-rule evaluation functions in `card.rs` (`highest_card`,
  `cards_of_one_number`, `cards_of_one_color`, `most_even_cards`,
  `cards_of_different_colors`, `cards_that_form_a_run`,
  `most_cards_below_4`) produce the correct winning sets, and `leader()`
  (card.rs:297) compares count-then-highest-card (rank then color ordinal, Red
  highest) matching the official tie-break — including within-player best-group
  selection for Orange/Yellow (first max-length group in
  rank_key-descending insertion order provably holds the highest top card).
- Turn flow matches official Advanced Red7: play once, then optionally discard
  (play+discard combo allowed, discard force-ends turn, which is equivalent
  since no further legal action exists); `done` without acting eliminates;
  empty hand at turn start eliminates (lib.rs:146-152); discard must leave you
  winning (official actions 2/3 require this — lib.rs:325-330 is correct);
  canvas draw rule `card.rank > palette.len()` (lib.rs:346) matches the
  official advanced draw rule including palette-counting after a same-turn
  palette play.
- Setup: 7 hand + 1 palette each, canvas starts Red, player after the leader
  goes first (lib.rs:100-101) — matches official "player to the leader's left".
- Scoring: winner scores exactly the rule-meeting palette cards
  (lib.rs:176-179), scored cards leave circulation, deck < 8/player ends the
  game, targets 40/35/30 (lib.rs:22-24) — all match the official rulebook.
- Serde views: `PubState` exposes only deck_len, hand sizes, public palettes,
  discard pile, scores — no hand/deck/RNG leak; a unit test
  (`pub_state_does_not_leak_hidden_info`, lib.rs:729-749) guards this.
- `rules()` returns `include_str!("../RULES.md")` (lib.rs:532-534) per project
  convention; `data_docs`/strategy docs also embedded per V2 convention.
- `render.rs`: clean; no panics, no leaks (eliminated palettes masked).
- Binaries (`red7_1_cli/http/repl/fuzz`): byte-for-byte standard boilerplate;
  NO deviations from the systemic pattern (issues already captured elsewhere).
- `Cargo.toml`: matches the systemic boilerplate exactly (brdgme_cmd/brdgme_fuzz
  as `[dependencies]`, tokio "full" — already captured by another worker). No
  crate-specific deviations.

## Findings

### CardParser panics on non-ASCII input via byte-index string slicing
- severity: critical
- category: correctness
- location: game/red7-1/src/command.rs:31 (also 34-35)
- finding: `CardParser::parse` checks `chars.len() >= 2` but then slices by
  BYTES: `Card::parse(&input[..2])`, `consumed: &input[..2]`,
  `remaining: &input[2..]`. Any input whose second byte is not a char boundary
  panics with "byte index 2 is not a char boundary". Reachable from the Play
  endpoint with e.g. `play r€` (bytes [0x72, 0xE2, 0x82, ...] — index 2 splits
  the 3-byte €) or `play €5` (first char 3 bytes). A panic in the game service
  kills the request, violating CODING.md "no panicking code in runtime paths".
  This is a crate-LOCAL panic path (distinct from the already-captured core
  parser non-ASCII panics in rust/lib/game).
- recommendation: Slice on char boundaries, e.g. take the first two chars with
  `let prefix: String = input.chars().take(2).collect()` and compute the
  byte length from `prefix.len()` for `consumed`/`remaining`, or reuse
  `Card::parse` on a `char_indices()`-derived boundary.

### Player with zero rule-fulfilling cards is treated as winning (official: cannot win)
- severity: major
- category: correctness
- location: game/red7-1/src/card.rs:297-316 (`leader`), consumed at
  game/red7-1/src/lib.rs:154-164 (`end_turn`) and lib.rs:325-330 (`discard`
  pre-check)
- finding: Judged against official Red7 rules (no Go port). The official
  rulebook states: "If you don't have a card fulfilling the rule (can happen
  for green or purple), you cannot win." In `leader()`, when ALL palettes have
  an empty winning set (possible under Green = most even cards, and Violet =
  most cards below 4), the count comparison ties at 0 and the rank_key maxes
  tie at `(0, 0)`, so the strict `>` at card.rs:311 keeps the FIRST
  non-eliminated player as "leader" with zero qualifying cards. Consequences:
  (1) that player can end their turn with `done` and survive under a rule
  where officially nobody is winning and they should be eliminated;
  (2) the `discard` pre-check `leader_with_suit(card.suit) == player`
  (lib.rs:325-330) lets the lowest-index player discard INTO a Green/Violet
  rule where no one has qualifying cards — officially an illegal discard
  ("you must be winning the new game"); (3) at round end such a "winner"
  scores 0 points with an empty card list in the log. Example reachable state:
  Green rule, palettes p0=[b5], p1=[r7] — crate calls p0 the leader; official
  rules say neither can be winning.
- recommendation: In `leader()` (or `leader_with_suit`), treat empty winning
  sets as non-winning: skip players whose rule set is empty when selecting the
  leader, and define explicit behavior when all sets are empty (e.g. return
  `Option`, make `end_turn` eliminate the current player, and reject the
  discard pre-check). If the deviation is deliberate, document it in RULES.md
  and DATA_DOCS.md.

### DATA_DOCS.md tie-break description contradicts both the code and official rules
- severity: minor
- category: consistency
- location: game/red7-1/DATA_DOCS.md:36
- finding: "Ties within a rule are broken by the highest card in the winning
  set, then by the highest card overall in the palette." The second clause is
  not implemented anywhere (`leader()` in card.rs:297-316 only compares the
  winning set's max rank_key; on fully-empty ties it keeps the first player by
  index, never looking at overall palette cards) and also does not exist in
  the official rules (which instead say a player with no rule-fulfilling card
  cannot win). Bots consuming DATA_DOCS.md get a wrong model of tie-breaking.
- recommendation: Rewrite the line to describe actual behavior (ideally after
  fixing the empty-set finding above): ties broken by highest card among the
  rule-fulfilling cards, value then color.

### RULES.md undersells the turn structure and misdescribes scoring
- severity: minor
- category: consistency
- location: game/red7-1/RULES.md:21-29 (Turn), 46-50 (Scoring)
- finding: (1) The Turn section lists "1. Play ... 2. Discard ... 3. Done" as
  alternatives and never states that the officially-sanctioned combo — play to
  palette AND THEN discard to canvas in the same turn — is allowed (the code
  permits it: `can_discard` at lib.rs:287-289 ignores `has_played`). A player
  reading only RULES.md would not discover the strongest move in the game.
  (2) "the remaining player (the leader) scores their palette cards" — the
  code scores only the palette cards MEETING the current rule
  (lib.rs:176-179), per official rules; the rest return to the deck.
- recommendation: Add an explicit "you may play one card to your palette and
  then discard one card to the canvas" item, and change the scoring line to
  "scores the cards in their palette that meet the current rule".

### Aliased re-export `PubCard`/`PubSuit` deviates from sibling crates and is unused
- severity: nit
- category: consistency
- location: game/red7-1/src/lib.rs:16
- finding: `pub use card::{Card as PubCard, Suit as PubSuit};` — other card
  game crates in the workspace (alhambra-1, seven-wonders-1, starship-catan-1)
  use plain `pub use card::*;`, and a workspace-wide grep shows `PubCard`/
  `PubSuit` are referenced nowhere. Dead, non-conventional public API surface.
- recommendation: Switch to `pub use card::*;` (or drop the re-export) to
  match the sibling-crate convention.

### `leader_with_suit` indexes `player_map[l_index]` — panics if all players eliminated
- severity: nit
- category: quality
- location: game/red7-1/src/lib.rs:250-251
- finding: `card::leader()` returns `(0, vec![])` for an empty palette list,
  so `player_map[l_index]` panics when every player is eliminated. All current
  call sites guarantee at least one non-eliminated player (end_turn skips the
  check for an eliminated current player; end_round requires exactly one
  remaining; discard/start_round run pre-elimination), so this is unreachable
  today — but the invariant is implicit and fragile.
- recommendation: Either return `Option<(usize, Vec<Card>)>`, or add a short
  comment documenting the non-empty precondition (per CODING.md comment
  discipline: hidden constraint).

### `end_points` arithmetic underflows for player counts above 10
- severity: nit
- category: quality
- location: game/red7-1/src/lib.rs:22-24
- finding: `(50 - players * 5) as u32` panics on usize underflow in debug
  builds for `players > 10` (and wraps in release). All in-crate callers pass
  validated 2..=4, so unreachable in practice, but it is a `pub fn` with no
  guard or documented precondition.
- recommendation: Document the 2..=4 precondition in a doc comment, or clamp /
  take a validated type. Lowest priority.
