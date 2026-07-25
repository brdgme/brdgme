# Verification: game/splendor-2 (F29-F35)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (f8763a5).
Go tree present at /home/beefsack/Development/brdgme-review-snapshot/brdgme-go.
No code changes made; no tests/builds run.

## F29 - Prestige ties broken by most cards instead of fewest
- verdict: CONFIRMED
- severity: minor (agree; correctness deviation from official rules, but
  Go-parity and only bites on exact prestige ties)
- evidence:
  - rust/game/splendor-2/src/lib.rs:199-202: metric is
    `vec![self.player_boards[p].prestige(), self.player_boards[p].cards.len() as i32]`.
  - rust/lib/game/src/game.rs:154-172 (`gen_placings`): keys sorted ascending
    via `cmp_fallback`, then iterated `keys.iter().rev()` assigning place 1
    first - i.e. higher metric vectors place better. So on equal prestige,
    MORE cards -> better placing. Official rule: fewest cards wins the tie.
  - Test lock-in confirmed: lib.rs:1221-1234
    `test_placings_tie_broken_by_card_count` gives p0 two cards (prestige
    5+0) and p1 one card (prestige 5), asserts `placings[0] == 1`,
    `placings[1] == 2` - the inverted direction.
  - Go-parity confirmed: brdgme-go/splendor_1/game.go:147-155 uses the
    identical `[Prestige, len(Cards)]` metric;
    brdgme-go/brdgme/placings.go:70 `sort.Sort(sort.Reverse(...))`.

## F30 - take() never validates tokens are gems
- verdict: CONFIRMED
- severity: minor (agree; defense-in-depth only, not reachable via
  Gamer::command, but `take` is a pub fn)
- evidence:
  - lib.rs:293-347 `take()`: 2-token path checks only equality
    (lib.rs:301) and `self.tokens.get(tokens[0]) < 4` (lib.rs:306). With
    `&[Gold, Gold]`: bank gold starts at MAX_GOLD = 5 (lib.rs:26,545), so
    5 >= 4 passes and the player receives 2 gold. 3-token path likewise only
    checks distinctness and bank > 0; a gold among three distinct would pass.
  - Gold excluded solely by the parser: command.rs:183 `tokens_parser(false)`
    inside `take_parser` (command.rs:172-188); `token_parser` adds Gold only
    when `include_gold` (command.rs:157-165).
  - Sole non-test producer of `Command::Take` is command.rs:186; `command()`
    dispatch (lib.rs:683-700) only receives it from the parser. So not
    reachable through `Gamer::command` - matches the finding's framing.

## F31 - Local cost.rs vs lib/cost consolidation assessment
- verdict: CONFIRMED (one wording imprecision, does not change the claim)
- severity: minor / dependencies (agree)
- evidence per sub-claim:
  - (a) lib/cost lacks get/set: CONFIRMED. Full pub API of
    rust/lib/cost/src/lib.rs: `new`, `from_keys`, `add`, `inv`, `sub`,
    `pos_neg`, `can_afford`, `take`, `drop`, `is_zero`, `trim`, `sum`,
    `keys`, `to_keys`, free `can_afford_perm`. No `get`, no `set`.
    Splendor usage: `.get(`/`.set(` grep hits: lib.rs 50, render.rs 7,
    player_board.rs 0, card.rs 0 - "~50 sites" is accurate; cited sites
    (pay lib.rs:268-285, reserve gold lib.rs:436-442, start lib.rs:544-548,
    render.rs) all verified present.
  - (b) no lib/cost gold-joker equivalent: CONFIRMED. splendor's free
    `can_afford(a, c)` (cost.rs:79-87) folds gold into per-resource
    shortfall. lib/cost's only free function is `can_afford_perm`
    (lib.rs:165+), a permutation/substitute-cost solver for seven-wonders -
    a different mechanism, not a gold-joker check.
  - (c) serde-safe: CONFIRMED with imprecision. splendor's is
    `pub struct Cost(pub HashMap<Resource, i32>)` (cost.rs:13); lib's is
    generic `pub struct Cost<K: Hash + Eq>(pub HashMap<K, i32>)`
    (lib/cost/src/lib.rs:8), not literally the same declaration as the
    finding states - but instantiated at `K = Resource` it is the same
    newtype-over-HashMap serde shape, so the "no persisted-game breakage"
    conclusion holds.
  - (d) invasiveness: CONFIRMED. Exactly 4 source files touch `Cost`
    (lib.rs, render.rs, player_board.rs, card.rs). `cost!` macro
    (card.rs:69-72) builds `Cost(HashMap::from([...]))` through the pub
    tuple field, which lib's `Cost<Resource>` (pub field) accepts unchanged.
    player_board.rs uses only `new/add/sub/can_afford/bonuses` composition -
    no get/set, consistent with the migration being blocked only on
    adding get/set for lib.rs/render.rs.

## F32 - reserve parser offers row-3 locations; test comment wrong
- verdict: CONFIRMED
- severity: nit (agree)
- evidence:
  - command.rs:85-102 `loc_parser` appends the player's reserve slots as
    row 3 (`ParsedLoc { row: 3, col }`, name `{'A'+col}4`) whenever
    `reserve.len() > 0`.
  - command.rs:118-134 `reserve_parser` reuses `self.loc_parser(player)`
    (command.rs:129) with no filtering, so `reserve A4` parses and appears
    in autocomplete; rejection happens only at lib.rs:423-425
    (`if row > 2 { return Err("that is not a valid row") }`).
  - Test comment lib.rs:1061-1063: "the loc parser never offers row 3 as a
    `reserve` target either" - factually wrong given the above.

## F33 - finished-game epilogue duplicated across five command arms
- verdict: CONFIRMED
- severity: nit (agree)
- evidence: lib.rs:635-640, 653-658, 671-676, 689-694, 707-712 - identical
  `if self.is_finished() { let scores... placings_log... }` block in all
  five match arms of `command()`, each followed by an identical
  `CommandResponse` construction. "~50 lines total" is a fair count of the
  duplicated arm bodies (the epilogue proper is ~6 lines x 5 plus the
  repeated response construction).

## F34 - "remaning" typo
- verdict: CONFIRMED
- severity: nit (agree)
- evidence: lib.rs:326 `"there aren't enough tokens remaning to take that"`.
  Go-parity confirmed: brdgme-go/splendor_1/take_command.go:77 has the
  identical misspelling.

## F35 - .expect() in visit_phase auto-visit
- verdict: CONFIRMED (unreachability trace holds)
- severity: nit (agree)
- evidence trace (lib.rs:222-235):
  - `visit_phase` sets `self.phase = Phase::Visit` (lib.rs:223) immediately
    before calling `self.visit(self.current_player, can_visit[0])`.
  - `visit()` (lib.rs:496-514) can fail only three ways:
    1. `assert_not_finished` - `ended` is mutated only in `next_player`
       (lib.rs:241-242), which is reached via Discard -> next_player, never
       before entering Visit in the same chain; every path into
       `visit_phase` (next_phase from take/buy/reserve/discard, each after
       its own passing `assert_not_finished` at lib.rs:294/356/418/460)
       leaves `ended` untouched in between.
    2. `can_visit` (lib.rs:489-491) - player IS `current_player` and phase
       was just set to Visit, so true.
    3. `noble >= self.nobles.len()` - `can_visit[0]` comes from
       `(0..self.nobles.len()).filter(...)` (lib.rs:225-227), so in range.
  - Hence the `.expect` at lib.rs:230-232 is genuinely unreachable.
  - (Go panics identically per finding; not independently re-traced in Go -
    immaterial to the verdict.)

## Summary
- F29 CONFIRMED minor
- F30 CONFIRMED minor
- F31 CONFIRMED minor (wording nit: lib/cost's newtype is generic Cost<K>,
  not Resource-specific; conclusion unaffected)
- F32 CONFIRMED nit
- F33 CONFIRMED nit
- F34 CONFIRMED nit
- F35 CONFIRMED nit
- No severity changes.
