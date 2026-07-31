# Unit 03a - Game crates: splendor-2, texas-holdem-2, acquire-1

Date: 2026-07-30. Read-only review. Findings numbered from F-36.

## Scope

- Crates: `rust/game/splendor-2`, `rust/game/texas-holdem-2`, `rust/game/acquire-1`
- Commits reviewed:
  - `614cf4f7` WP-17 splendor-2 onto `lib/cost`
  - `0688e03e` T3-B3 splendor-2/lib-cost follow-up hardening
  - `84b68b99` WP-18 texas-holdem-2
  - `07ad4760` WP-19 acquire-1
- Excluded (sub-unit 03b): cathedral-2/sushizock-2, lords-of-vegas-1, jaipur-2.
- Pre-ruled, not re-raised: F-21 (acquire-1 zero regression tests).

Files read in final form (7,920 lines total across the three crates):
`splendor-2/src/{lib.rs,cost.rs,player_board.rs,command.rs}` + render/card
signatures; `texas-holdem-2/src/{lib.rs,command.rs,render.rs}` +
`poker.rs` (`result`, `winning_hand_result`, `HandResult`, `Category`);
`acquire-1/src/lib.rs` + render/board/corp/command panic-and-render surfaces.
Cross-referenced against `00-sweeps.md` (sweeps 1-3) and the WP-17/WP-18/WP-19
specs recovered from git history.

Findings: 12 (1 High, 3 Medium, 8 Low), F-36 .. F-47.

## Findings

### F-36 (High) - `texas-holdem-2` has no `validate()` override, and it is the crate where that matters most (F-06 confirmation)

`rust/game/texas-holdem-2/src/lib.rs:663-814` - `impl Gamer for Game` defines
`start`, `pub_state`, `player_state`, `command`, `command_spec`, `status`,
`player_count`, `player_counts`, `points`, `rules`, `data_docs`,
`basic_strategy`, `advanced_strategy`. There is no `fn validate`, so WP-09b's
per-crate validate sweep never reached this crate and `Gamer::validate`'s
fail-open `Ok(())` default applies.

Why it matters more here than in the crates already filed (F-24/F-31/F-33):
`Game` carries seven parallel per-player vectors (`player_hands`,
`player_money`, `bets`, `folded_players`) whose length is only ever
established inside `new_hand()`, and every accessor indexes them raw:

- `remaining_players()` (`lib.rs:98-102`) indexes `player_money[p]` and
  `bets[p]` for `p in 0..self.players` - a deserialized state with
  `players: 8` and 2-element vectors panics on the first `status()` call,
  i.e. before any command validation can run.
- `player_hands[player]` in `player_state` (`lib.rs:709`) - a short
  `player_hands` panics on a plain state read.
- `next_player_in_set` (`lib.rs:143-155`) has two explicit `panic!`/`assert!`
  arms reachable from a state where `players` disagrees with the vectors.

`bets`/`folded_players` are additionally `Default`-initialised to empty vecs
in `start` (`lib.rs:679-687`) and only sized in `new_hand`, so "vectors match
`players`" is genuinely an invariant of the constructor, not of the type.

Remediation: add `fn validate(&self) -> Result<(), GameError>` asserting
`MIN_PLAYERS..=MAX_PLAYERS` contains `players`; that `player_hands`,
`player_money`, `bets` and `folded_players` all have length `players`; that
`current_player < players` and `current_dealer < players`; that
`community_cards.len()` is one of 0/3/4/5; that no `player_money[p] < 0` or
`bets[p] < 0`; and that `first_betting_player < players`.

### F-37 (Medium) - `splendor-2` has no `validate()` override (F-06 confirmation)

`rust/game/splendor-2/src/lib.rs:529-712` - same shape: no `fn validate` in
`impl Gamer for Game`. `splendor-2` indexes `self.board[row]` and
`self.decks[row]` for `row in 0..=2` (`lib.rs:374-396`, `456-460`) and
`self.player_boards[player]` throughout, all with lengths fixed only by
`start` (`lib.rs:543-575`). A deserialized `Game` with `board: []` or
`player_boards` shorter than `players` panics inside `buy`/`reserve`/
`pub_state` rather than being rejected at the trust boundary.

This is the crate the F-06 finding text specifically named as missing
`validate`, so it is confirmed still open after the whole remediation
programme.

Remediation: `validate` should check `players` in range, `decks.len() == 3`,
`board.len() == 3`, `board[l].len() <= 4`, `player_boards.len() == players`,
`current_player < players`, `player_boards[p].reserve.len() <= 3`, all
`tokens` counts `>= 0`, and `ended => end_triggered`.

### F-38 (Low) - `splendor-2` `visit_phase` turns an unreachable internal error into a public game log

`rust/game/splendor-2/src/lib.rs:237-239`:

```rust
1 => self
    .visit(self.current_player, can_visit[0])
    .unwrap_or_else(|e| vec![Log::public(vec![N::text(e.to_string())])]),
```

The auto-visit path cannot actually fail (`can_visit` holds because
`visit_phase` just set `Phase::Visit`, and `can_visit[0]` is by construction
a valid noble index), so this arm is dead defensive code. But if it ever did
fire, the consequences are the wrong ones: the noble is silently not awarded,
the phase is left in `Visit` with no log saying so, and a raw `GameError`
`Display` string is broadcast to every player as if it were a game event.
This is the same swallowed-error shape as F-07.

Remediation: either propagate (make `visit_phase` return
`Result<Vec<Log>, GameError>` like the command methods it is called from) or
drop the fallback and let the invariant assert, but do not publish internal
error text as a public log.

### F-39 (Low) - `splendor-2` `PubState` omits deck sizes, which are public information

`rust/game/splendor-2/src/lib.rs:79-97` - `PubState` carries `board` but not
`decks`, and the doc comment justifies it with "deck contents/sizes are never
rendered so are omitted". Deck *contents* are hidden and correctly excluded;
deck *sizes* are public in Splendor (the physical decks are visible, and
whether a bought slot refills is directly observable). Because the count is
absent, a client rendering only `PubState` cannot show remaining cards per
level and cannot explain why a level shrank from 4 slots to 3.

Same class as F-29 (modern-art-2 omitting public hand sizes): WP-10's shape
under-redacts nothing here, it over-redacts. Remediation: add
`deck_counts: [usize; 3]` (or `Vec<usize>`) to `PubState`, populated from
`self.decks[l].len()`.

### F-40 (Medium) - `acquire-1`'s "tolerate missing share keys" hardening silently mints bank shares instead of erroring

Three sites default a *missing* bank-share entry to `STARTING_SHARES` (25)
rather than 0:

- `rust/game/acquire-1/src/lib.rs:1119` (`take_shares`)
- `rust/game/acquire-1/src/lib.rs:1137` (`return_shares`)
- `rust/game/acquire-1/src/lib.rs:577` (`handle_found_command`)

`self.shares` is the bank's *remaining* pool, tracked explicitly. `or_insert(
STARTING_SHARES)` is only correct at game creation; used as a fallback it
fabricates 25 shares. Concretely, in `return_shares` a player selling `n`
shares of a corp whose bank key is absent produces a bank balance of `25 + n`
instead of `n`, breaking the 25-shares-per-corp conservation invariant that
share prices, majority bonuses and final scoring all depend on. In
`handle_found_command` a missing key mints 25 and then hands one to the
founder, leaving 24 phantom shares purchasable.

`take_shares` happens to be shielded by its own `unwrap_or_default()` (=0)
pre-check on line 1111, so the `or_insert(25)` there is dead but wrong;
`return_shares` and `handle_found_command` have no such shield - their guards
only look at the *player's* holdings and at `available_corps()` respectively.

Worse, the WP's own regression test locks the broken behaviour in.
`end_tolerates_missing_share_keys` (`lib.rs:1490-1497`) constructs exactly
this state (`g.shares.remove(&Corp::American)` plus
`players[0].shares.insert(Corp::American, 3)`), runs `end()`, and asserts
only `!logs.is_empty()` and `g.finished`. The bank ends up holding 28
American shares and nothing notices. The sibling test
`missing_share_key_errors_instead_of_panicking` passes only because it
removes the key from *both* maps, so the player-side guard fires first.

This is a symptom-papering fix: the acceptance criterion "does not panic on a
missing share key" was met while a share-conservation break was introduced
under it.

Remediation: change all three to `or_insert(0)` (or better, `get`/`get_mut`
with an explicit `GameError::Internal` on absence, matching how the same
commit treats an empty major-bonus set at `lib.rs:842-846`), and extend
`end_tolerates_missing_share_keys` to assert
`shares[&Corp::American] == 3` after the sell.

### F-41 (Low) - `acquire-1` `Status::Finished { stats: vec![] }` (F-35 extension)

`rust/game/acquire-1/src/lib.rs:204-207` constructs
`Status::Finished { placings, stats: vec![] }`. F-35 recorded this shape in
two Unit-02 crates; `acquire-1` is a further instance, so the F-35 crate list
is incomplete. Every other crate in this sub-unit uses a per-player vector of
empty maps (`texas-holdem-2/src/lib.rs:775`,
`splendor-2/src/lib.rs:583`), which at least matches the arity consumers
expect. `stats: vec![]` gives a zero-length stats vector for an N-player
game; any consumer that zips `placings` with `stats` sees a length mismatch.

Remediation: as F-35 - either populate stats or make the arity consistent
(`vec![Default::default(); self.players.len()]`) across all crates. See
`00-sweeps.md` sweep 2 for the full occurrence list.

### F-42 (Low) - `acquire-1` removes the played tile from the hand *after* running the whole merger cascade

`rust/game/acquire-1/src/lib.rs:452-519` - `handle_play_command` records
`pos` (the index of the played tile in `self.players[player].tiles`) at line
452, then runs the neighbouring-corp match, which in the merge arm calls
`choose_merger_phase` -> `handle_merge_command` -> `next_player_sell_trade` ->
`end_sell_trade_phase` -> `choose_merger_phase` again, and only then does
`self.players[player].tiles.swap_remove(pos)` on line 518.

Today none of those callees touches `players[player].tiles`, so `pos` stays
valid. But `draw_replacement_tiles` (`lib.rs:377-435`) *replaces* that vector
wholesale (`self.players[player].tiles = keep;` line 433), and it is one
`end_turn()` call away from this cascade. If any future path lets a merger
cascade reach `end_turn`, `swap_remove(pos)` will either delete an unrelated
(possibly freshly drawn) tile or panic with an out-of-bounds index. `pos`
being an index rather than a value is the whole hazard, and `swap_remove`
additionally shuffles the hand order for no reason.

Remediation: remove the tile before the cascade (the tile is already validated
as present at line 452, and the `0`-neighbour arm's "set the tile last as
errors can be thrown above" comment shows the ordering was reasoned about for
the board but not for the hand), or drop `pos` and use
`tiles.retain(|l| l != loc)` at the end so the operation is index-free.

### F-43 (Low) - `acquire-1` `handle_end_command` is the only handler that skips `assert_player_turn`

`rust/game/acquire-1/src/lib.rs:1167-1173` checks
`self.phase.main_turn_player() != player` instead of calling
`self.assert_player_turn(player)?` like the other eight handlers. During a
`Phase::SellOrTrade` sub-phase, `phase.whose_turn()` (and therefore
`status().whose_turn`) is the *selling* player, while `main_turn_player()` is
the turn player - so the turn player can issue `end` while `status()` says it
is someone else's turn.

This may well be deliberate ("the end trigger belongs to the turn, not the
sub-phase"), but nothing in the code says so, and it means `status()` is not a
truthful description of who may act. Remediation: add a comment stating the
intent, and make `command_parser` the single source of truth so the web layer
cannot disagree with `status()`.

### F-44 (Medium) - `texas-holdem-2` offers a `raise` command whose `Int` bounds are inverted (min > max)

`rust/game/texas-holdem-2/src/command.rs:47-69` builds the raise parser with

```rust
let behind_current_bet = self.current_bet() - self.bets[player];
let min = self.min_raise();                                  // max(minimum_bet, largest_raise)
let max = self.player_money[player] - behind_current_bet;
```

while `Game::can_raise` (`lib.rs:312-318`) gates the parser's *existence* on
`player_money[player] > current_bet - bets[player] + largest_raise`, i.e. on
`largest_raise` alone, deliberately omitting the `minimum_bet` floor. The two
therefore disagree, and the disagreement is trivially reachable:

- `minimum_bet = 10`, `largest_raise = 0`, `bets = [0, 10]`,
  `player_money[0] = 15`.
- `can_raise(0)`: `15 > (10 - 0) + 0` -> true, so `raise` is added to the
  parser set (`command.rs:31-33`).
- `raise_parser(0)`: `min = max(10, 0) = 10`, `behind = 10`,
  `max = 15 - 10 = 5`. The `Int` is constructed with `min: Some(10),
  max: Some(5)`.

`Int` does not panic (it errors), so the outcome is a `raise` command that is
advertised in `command_spec` and in the suggest/autocomplete UI but rejects
every possible input with the self-contradictory message "expected number
between 10 and 5" (`lib/game/src/command/parser/mod.rs:117`). Any player who
is short-stacked relative to the minimum bet sees a raise option they can
never use.

**This is a regression introduced by WP-18 itself, not a pre-existing quirk.**
The recovered T3-B3 checklist row for `c F1` reads: "Use `self.min_raise()`
instead of `self.largest_raise` for the `Int` parser's `min`, and rewrite the
doc comment (the real `LargestRaise` quirk lives in `Game::can_raise`, not
here)". Before that change the two sides *were* consistent by construction:
with `min = largest_raise`, `can_raise`'s guard
`money > behind + largest_raise` rearranges to `max > min`, so the bounds
could never invert. Raising `min` to `max(minimum_bet, largest_raise)`
without touching `can_raise` broke that invariant. The commit's own docstring
(lines 41-46) records the asymmetry and calls it "the separate, genuine
`LargestRaise` quirk" - the asymmetry was observed and rationalised rather
than followed through to `can_raise`.

Go parity is not a defence: the inconsistency produces a malformed
`CommandSpec` on the wire, which is a brdgme-layer contract, not a rules
question.

The regression test added for this area,
`raise_parser_min_bound_uses_min_raise` (`lib.rs:1002-1020`), sets
`player_money = vec![100, 100, 100]` so `max` is always comfortably above
`min` - it verifies the min bound and never exercises the inverted case.

Remediation: gate on the same quantity the parser uses -
`can_raise` should test
`player_money[player] > current_bet - bets[player] + self.min_raise()` (or
equivalently `max >= min`). If the Go `largest_raise` behaviour must be
preserved for rules parity, then clamp the parser instead
(`max: Some(max.max(min))` is wrong; the correct fix is to not offer the
parser when `max < min`) and record the decision in
`docs/decisions/`.

### F-45 (Low) - `texas-holdem-2` showdown log order is nondeterministic across runs

`rust/game/texas-holdem-2/src/poker.rs:348-377` - `winning_hand_result` builds
`next_pass` by iterating a `HashMap<i32, HandResult>`
(`for (id, hr) in hand_results`, line 351), so the returned winner vector is in
`HashMap` iteration order, which Rust deliberately randomises per process via
`RandomState`. `showdown` then iterates `for &winner in &winners`
(`lib.rs:482-491`) to emit the "X took $N (pair of aces)" lines.

Game state is unaffected (`pot_per_player` is order-independent and the
uneven-split remainder goes to `next_remaining_player_num_from(current_dealer)`),
so this is not a scoring bug. But brdgme replays a game from `(seed, commands)`,
and the log text for a split pot will not reproduce byte-for-byte between
replays, which undermines log-based diffing and makes any future log assertion
in this area flaky. The caller already converts to a `HashMap<i32, _>` purely to
match the Go signature (`lib.rs:476-479`), having built `hand_results` from a
deterministic `0..self.players` loop.

Remediation: sort the returned winners (`winners.sort_unstable()`) in
`showdown` before iterating, or change `winning_hand_result` to take a
`BTreeMap`/slice so the order is the seat order it was built in.

### F-46 (Low) - WP-19 Task 7 did not remove `acquire-1`'s `pub_state` deep clone; only the duplication half landed

Finding `c F20` (recovered spec): "`pub_state()` deep-clones the whole `Game`
just to compute 3 ints for `can_end` - paid 6+ times per request for a
6-player game via `renders()`."

The end state still deep-clones the whole `Game` on every call:

- `rust/game/acquire-1/src/lib.rs:226-228` -
  `fn pub_state(&self) -> Self::PubState { self.to_owned().into() }`
- `rust/game/acquire-1/src/lib.rs:1224-1236` -
  `impl From<Game> for PubState` takes `Game` **by value**, which is what
  forces the `to_owned()`.

`Game::to_owned()` clones `board`, every `Player` (money + `HashMap<Corp,
usize>` + `Vec<Loc>` hand), the whole `draw_tiles` bag (~100 `Loc`s at game
start), `shares` and the `GameRng`. Of those, `PubState` keeps only `board`,
`shares`, the per-player money/shares, and `draw_tiles.len()`. The hand
vectors and the tile bag are cloned and immediately dropped.

`player_state` (`lib.rs:230-236`) calls `self.pub_state()` and so pays the
same full clone again, once per player - which is exactly the "6+ times per
request" the finding described.

What *did* land is the good half: `can_end` was extracted to a free function
over `(&Board, bool, bool)` (`lib.rs:325-356`) and is now called from both
`PubState::can_end` (`lib.rs:99-103`) and `Game::can_end` (`lib.rs:1190-1192`),
with `game_can_end_matches_pub_state_can_end` guarding the equivalence. That
removes the duplicated metric (the F-19 hazard) but not the allocation the
finding was actually about.

Remediation: change the conversion to `impl From<&Game> for PubState` and make
`pub_state` `self.into()`, cloning only the four fields `PubState` keeps. Add
a `PubState` field-coverage assertion at the same time to close coverage gap 2.

### F-47 (Low) - WP-19's package-level test-count gate was not met

The recovered WP-19 spec states a hard final gate: "Final count after all
(non-dropped) tasks: 11 + 11 new tests = **23 passing** (Task 5 is dropped so
its test does not land; Tasks 6 and 9 add no tests)."

Measured at HEAD: `rust/game/acquire-1` has 21 tests - 14 in
`src/lib.rs`, 6 in `src/board.rs`, 1 in `tests/contract.rs`. At the commit's
parent (`07ad4760^`) the same files held 5 / 6 / 1 = 12. So 9 new tests landed
where the spec specified 11, and the stated final count is 2 short.

This is a paperwork finding on its own, but it compounds F-40 and F-46: the
two tasks whose behaviour this review found unfixed or wrongly fixed (Task 4's
share-key handling, Task 7's `pub_state` clone) are also the two whose test
coverage is thinnest -
`end_tolerates_missing_share_keys` asserts nothing about share counts, and
nothing asserts anything about `pub_state`'s allocation or field set. The
missing tests are plausibly the ones that would have caught both.

Remediation: reconcile against the spec's per-task test list, or record in
`docs/reviews/.../execution-state` which two tests were consciously dropped
and why.

## WP acceptance-criteria cross-check

Specs recovered from `868094a6:docs/reviews/2026-07-23-rust-review/planning/
specs/` and from the T3-B3 checklist at `43bcf72`; extraction notes in
`/tmp/.../scratchpad/03a-specs.md`. Test/clippy/fmt gates could not be
executed (read-only review; running Rust tests is prohibited in this session),
so only static criteria are judged.

### WP-17 (`614cf4f7`) + T3-B3 follow-up (`0688e03e`) - splendor-2 onto `lib/cost`

Closes `b F31`, `ls F39`, `dp F27` (decision D-25 option A).

| Criterion | Verdict |
|---|---|
| `grep -rn 'from_resources' rust/` -> zero hits | **Met.** The only remaining hits are `starship-catan-1`'s unrelated `transaction_from_resources` (`starship-catan-1/src/lib.rs:74,648`), which the substring grep catches spuriously. splendor-2's `from_resources` is gone. |
| `splendor-2/src/cost.rs` has only the free `can_afford` + a type alias, no `struct Cost` | **Met.** `cost.rs:3` is `pub type Cost = brdgme_cost::Cost<Resource>;`, `cost.rs:5-13` is the free function. |
| One `brdgme_cost` line in `splendor-2/Cargo.toml` | **Met** (`Cargo.toml:10`). |
| `grep -rn 'Gold\|gold' rust/lib/cost/src/lib.rs` -> zero hits (gold-joker logic must NOT move into the shared lib) | **Met.** Zero hits; the wildcard rule stayed in `splendor-2/src/cost.rs`. |
| D-25 binding test constraint: `lib/cost` carries its own `get`/`set` tests (6 enumerated cases) | **Met in form.** `lib/cost/src/lib.rs` has 31 tests and defines `get` (line 119) / `set` (line 123). Not individually verified against the 6 enumerated cases. |
| Gold-joker `test_can_afford` retained **and** extended to 6 named cases | **Met exactly.** `cost.rs:22-80` has precisely those six: exact payment no gold, gold covering shortfall exactly, gold one short, cost naming `Resource::Gold`, empty cost, shortfall across two resources. |
| serde round-trip test of a serialized splendor `Game` | **Met** (`cost.rs:82-88`). |
| `seven-wonders-1` untouched and non-regressed | **Met statically** - neither commit's file list touches it. |

Out-of-scope items (`b F30/F32/F34/F35`, `ls F38`) correctly excluded from
`614cf4f7`; `0688e03e` is the follow-up that lands the splendor-2 action-layer
hardening plus the `lib/cost` `new()` bound move. No findings against WP-17.

### WP-18 (`84b68b99`) - texas-holdem-2

Closes `c F1`, `c F3`, `c F4`, `c F5` (checklist rows; no formal DoD section).

| Row | Verdict |
|---|---|
| `c F1` - `raise_parser` `Int` min should use `self.min_raise()`, doc comment rewritten | **Applied, but it introduced F-44.** The min is now `min_raise()` (`command.rs:49`) and the doc comment was rewritten (`command.rs:41-46`), so the row is literally satisfied; the accompanying `can_raise` guard was not updated, so the bounds can now invert. See F-44. |
| `c F3` - `bet_up_to`'s `.expect()`: remove, or keep with an explicit Go-mirroring exception comment | **Met** via the permitted second option (`lib.rs:158-168`), and the comment names docs/CODING.md explicitly. |
| `c F4` - confirm the Go-mirroring panics still carry documenting comments | **Met.** `next_player_in_set` (`lib.rs:141-155`) documents both the `assert!` and the fallthrough `panic!` against their Go originals. |
| `c F5` - drop the `Option` from `HandResult.category`, `#[derive(Default)]` + `#[default]` on the variant, delete both `unwrap_or(Category::None)` sites | **Met.** `poker.rs:16-18` has `#[default] None`; `poker.rs:31-35` has a plain `category: Category`; zero `unwrap_or(Category::None)` remain (only two `!= Category::None` comparisons, at `poker.rs:59` and `poker.rs:352`). Note `poker.rs:352`'s filter is now provably dead, since `result()` always falls through to `Category::HighCard` (`poker.rs:119-124`) - harmless, but it is the sort of residue this row was meant to clear. |

Out-of-scope `c F2` (MAX_PLAYERS 8 vs Go 9) correctly left alone - not
re-raised here.

### WP-19 (`07ad4760`) - acquire-1

Closes `c F7, F8, F9, F10, F16, F17, F18, F19, F20, F21`; `c F11` dropped
(superseded by WP-81).

| Finding | Verdict |
|---|---|
| `c F7` `player_counts()` dropped 6-player | **Met.** `lib.rs:281-283` is `(MIN_PLAYERS..=MAX_PLAYERS).collect()`, with `player_counts_covers_min_to_max_players` (`lib.rs:1363-1383`) asserting every advertised count starts *and* that `player_state`/`command_spec` work for each seat. Good test. |
| `c F8` dummy D6 rolled `1..=5` | **Met.** `lib.rs:902` is `random_range(1..=6)`, and `dummy_shareholder_rolls_a_full_d6` (`lib.rs:1385-1406`) does 1000 draws and asserts all six faces appear - a real distribution test, not a bounds check. |
| `c F9` `panic!` in `pay_bonuses` | **Met.** Now `GameError::Internal` (`lib.rs:842-846`) with a test (`lib.rs:1408-1423`). |
| `c F10` 10-site `.expect()` cluster on share maps | **Half met - see F-40.** The panics are gone and the typo'd message with them, but three sites default a missing bank key to `STARTING_SHARES`, trading a panic for silent share inflation. The spec's own repo-wide constraint ("Never replace a silent wrong answer with a panic") was honoured in one direction and violated in the other. |
| `c F16` unused `thiserror` dep | **Met.** Zero hits in `acquire-1/Cargo.toml`. |
| `c F17` tautological `can_undo` in `handle_found_command` | **Met.** Returns a literal `true` (`lib.rs:591`). |
| `c F18` `.unwrap()` on a 1-element `HashSet` | **Met.** `let ... else` with `GameError::Internal` (`lib.rs:468-472`). |
| `c F19` double `.unwrap()` in render.rs width scan | **Met.** No `unwrap`/`expect` remains anywhere in `acquire-1/src/render.rs`. |
| `c F20` `pub_state()` deep-clones the whole `Game` | **Not met - see F-46.** Only the `can_end` deduplication landed; `self.to_owned()` is still there. |
| `c F21` nondeterministic corp ordering in found/merge parsers | **Met, and scope-extended as the spec required.** Both `found_parser_corp_order_is_canonical_and_stable` (`lib.rs:1509-1536`) and `merge_parser_corp_order_is_stable` (`lib.rs:1538-1561`) assert 50 consecutive identical `Spec` renderings plus canonical `CORPS` ordering. |
| Package gate: no panic operators outside `#[cfg(test)]`, with only the two `Phase::SellOrTrade` sites whitelisted | **Exceeded.** Both whitelisted panics were also converted (`lib.rs:951`, `lib.rs:980` return `GameError::internal`). Every remaining `unwrap`/`expect`/`panic!` in the crate is inside `#[cfg(test)]` (all 17 hits are at `lib.rs:1256+`). |
| Package gate: final count 23 passing tests | **Not met - see F-47.** 21 at HEAD (14 + 6 + 1) against 12 at the parent commit. |
| Binding constraint: no serialized type/field/shape changes | **Met statically.** `Game`, `Player`, `PubState`, `PubPlayer` and `Phase` field sets are unchanged by the commit's diff scope (`Cargo.toml`, `command.rs`, `lib.rs`, `render.rs`). |
| Out-of-scope `c F12`-`F15`, `c F2`, RULES.md tertiary bonus | Correctly untouched. Note `c F12` is the same `stats: vec![]` that F-41 records - it is parked in WP-20, so F-41 is filed as an F-35 crate-list correction, not as a new demand to fix it. |

## Verified good

### texas-holdem-2

- **`pub_state` redaction is correct and tested.** `PubState`
  (`lib.rs:55-73`) carries only `players`, `community_cards`, `pot`,
  `current_dealer`, `current_player`, `player_money`, `bets`,
  `folded_players`. `player_hands` and `deck` - the only hidden state - are
  both absent, and `pub_state()` (`lib.rs:692-703`) never derives anything
  from them. `pub_state_does_not_leak_hands_or_deck` (`lib.rs:1264-1273`)
  asserts this at the serde layer, and `player_state_carries_own_hand`
  (`lib.rs:1275-1280`) confirms the private channel. This is one of the three
  crates WP-10 3a actually completed, and it is complete.
- **The log layer does not leak hole cards.** All 13 `Log::public` sites were
  checked. `check`/`fold`/`call`/`raise`/`all_in` log only the action and the
  amount; `flop`/`turn`/`river` log the community cards as they become public;
  blinds and the dealer marker are public facts. The only site that renders
  `player_hands` is `showdown` (`lib.rs:457-467`), and it is correctly gated
  on `!self.folded_players[player_num]` *and* on `self.bets[player_num] != 0`
  for the pot being contested, so a folded player's hole cards are never
  published and a player not eligible for a side pot is excluded from that
  pot's table. This is the exact leak class that F-22/F-28/F-34 caught
  elsewhere; texas-holdem-2 is clean.
- **`render.rs` is structurally incapable of leaking.** Both `Renderer` impls
  (`render.rs:98-108`) funnel through `render(&PubState, Option<usize>,
  Option<&[Card]>)`; the private branch is `if let (Some(p), Some(h))` from
  `PlayerState`'s own `hand`. No `Game` reference exists in the module.
- **Side-pot arithmetic is correct and tested.** The `while self.pot() > 0`
  loop in `showdown` (`lib.rs:443-513`) drains `min(bet, smallest_bet)` per
  player per pot. Termination is guaranteed because the max-bet holder can
  never have folded (`can_fold` requires `bets[player] < current_bet`), so
  `smallest_bet()` over `active_players` is always non-zero while `pot() > 0`.
  Folded players' chips are correctly included as dead money.
  `side_pot_awarded_separately_when_one_player_is_all_in_below_bet`,
  `showdown_splits_pot_between_tied_hands` and
  `showdown_awards_uneven_split_remainder_to_next_remaining_from_dealer`
  (`lib.rs:1058-1262`) cover the three branches.
- **WP-08 transition gate present.** `command` (`lib.rs:728`, `757-759`)
  captures `was_finished` before dispatch and only runs `finish_epilogue` on
  the `!was_finished && self.is_finished()` edge, with
  `command_finish_appends_single_last_placings_log` (`lib.rs:1358-1433`)
  asserting exactly-one-and-last across three arms. F-18 does not apply.
- `Status::Finished` uses `vec![HashMap::new(); self.players]`
  (`lib.rs:775`), which is at least the right arity - not the F-35 shape.
- The two `panic!`/`expect` sites (`next_player_in_set` `lib.rs:143-155`,
  `bet_up_to` `lib.rs:158-168`) are both documented, both mirror an
  equivalent Go panic, and `bet_up_to`'s is genuinely unreachable because the
  amount is clamped on the line above. Acceptable under docs/CODING.md's
  documented-exception rule.

### splendor-2

- **`pub_state` redaction is correct and tested.** The game's only hidden
  information is other players' reserved cards. `PubPlayer`
  (`lib.rs:60-74`) exposes `reserve_count: usize` and nothing else about the
  reserve; `PlayerState.reserve` (`lib.rs:101-109`) carries only the viewing
  player's own cards. `test_pub_state_reserve_counts_no_content`
  (`lib.rs:1236-1244`) asserts the serialised `PubState` contains no
  `"reserve"` key, and `test_reserve_visibility_not_leaked_to_others`
  (`lib.rs:1059-1068`) checks the cross-player case explicitly. `decks` is
  also correctly excluded (deck *order* is hidden). This is a genuine WP-10
  completion.
- **The log layer does not leak.** All 9 `Log::public` sites reference only
  board cards (which are face-up before the action), token movements (bank
  totals are public), or nobles (public). `reserve` publishes the reserved
  card's identity (`lib.rs:442-446`) - correct, because this port only allows
  reserving from the face-up board (`reserve` rejects `row > 2`,
  `lib.rs:435-437`), never a face-down deck top, so the card was already
  public. `test_reserve_logs_full_card_detail_publicly` (`lib.rs:1051-1057`)
  pins that intent.
- **The WP-17 `lib/cost` migration is behaviour-preserving.** `cost.rs` is
  now a 3-line `pub type Cost = brdgme_cost::Cost<Resource>` alias plus the
  one genuinely Splendor-specific predicate, `can_afford`
  (`cost.rs:5-13`), which implements the gold-as-wildcard shortfall rule that
  does not belong in a generic cost library. Six dedicated `can_afford` tests
  cover exact payment, gold covering the shortfall exactly, one-short, a cost
  that names gold, empty costs, and shortfall spread across two resources.
  `test_game_serde_round_trip` (`cost.rs:82-88`) guards the representation
  change at the persistence boundary, which is the right test for a type
  migration.
- `pay` (`lib.rs:270-294`) and `can_afford` agree on the gold-fallback model,
  and `test_buy_gold_fallback_arithmetic` (`lib.rs:920-942`) pins the exact
  bank/player deltas including the double-adjustment of the bank gem count.
- **WP-08 transition gate present** (`lib.rs:640`, `669-671`), with
  `finish_epilogue_single_placings_log` and
  `non_finishing_command_has_no_placings_log` (`lib.rs:1321-1364`) covering
  both edges. F-18 does not apply.
- `next_player`'s end condition (`lib.rs:245-254`) correctly gives every
  player an equal number of turns: the trigger is detected after the
  triggering player's turn and `ended` is only set when the rotation wraps
  back to seat 0. `test_end_trigger_and_final_round` and
  `test_end_trigger_only_fires_once` (`lib.rs:1163-1202`) cover it.

### acquire-1

- **`pub_state` redaction is correct.** `PubState` (`lib.rs:81-97`) reduces
  `draw_tiles: Vec<Loc>` to `remaining_tiles: usize` and maps each `Player`
  to a `PubPlayer` carrying only `money` and `shares`, dropping `tiles`
  (`lib.rs:1224-1245`). `PlayerState.tiles` is the viewing player's hand
  only. `render.rs:26-47` takes `(&PubState, Option<usize>, &[Loc])`, so the
  renderer cannot reach `Game`. There is no test for any of this - see
  coverage gaps.
- **The log layer does not leak the tile bag or other players' hands.** Tile
  draws use `Log::private(..., vec![player])` (`lib.rs:412-431`) - the only
  `Log::private` use in the three crates. The two public tile-discard logs
  (`draw_replacement_tiles` `lib.rs:386-403`, `redraw_hand`
  `lib.rs:714-732`) reveal tiles that are simultaneously leaving the hand
  permanently, which matches the physical game (an unplayable tile is shown
  as it is discarded) and leaves no hidden residue.
- **WP-08 transition gate present and correct** (`lib.rs:250`, `262-268`).
  `acquire-1` is confirmed migrated; F-18 does not apply.
- The WP-19 "missing share key" hardening does correctly convert three former
  panic sites into `GameError`s: `sell`/`take_shares`/`return_shares` guards
  (`lib.rs:1029`, `1112`, `1130`), the empty-major-bonus `GameError::Internal`
  (`lib.rs:842-846`), and the two `Phase::SellOrTrade` destructures
  (`lib.rs:951`, `980`). All three have tests
  (`lib.rs:1409-1441`, `1470-1488`). The panic-removal half of the WP holds;
  see F-40 for the correctness regression it introduced underneath.
- `bonus_players` (`lib.rs:897-940`) implements the majority/minority tie
  rules correctly, including the 2-player dummy shareholder and the
  "multiple majors share the minority bonus too" case, and
  `dummy_shareholder_rolls_a_full_d6` (`lib.rs:1385-1406`) verifies the die
  is a real 1-6 uniform draw rather than the off-by-one range the review
  found elsewhere. `handle_merge_command` correctly forces
  `can_undo == false` for 2-player games because the dice roll consumed
  entropy (`lib.rs:821-822`).
- `can_end` (`lib.rs:325-356`) reads correctly under Rust's `&&`/`||`
  precedence, and `game_can_end_matches_pub_state_can_end`
  (`lib.rs:1499-1507`) pins that `PubState::can_end` and `Game::can_end` stay
  equivalent - the exact "metric built twice" hazard F-19 flagged elsewhere,
  handled properly here by sharing one free function.

## Coverage gaps

1. **No crate in this sub-unit tests the log layer for hidden-information
   leakage.** All three have `pub_state` redaction assertions (or, for
   acquire-1, none at all), but no test asserts anything about which player a
   `Log` is visible to. texas-holdem-2 has the machinery
   (`log_text` helper, `lib.rs:836-841`) and uses it only for pot arithmetic.
   The minimal missing test, for every hidden-hand crate: drive a hand to
   showdown and assert that no `Log` with `public == true` renders a card that
   is still in a *folded* player's hand. This is the gap that let F-22 and
   F-28 through.
2. **`acquire-1` has no `pub_state`/`player_state` redaction test at all**
   (confirmed by sweep 3 in `00-sweeps.md`). Its `From<Game> for PubState`
   (`lib.rs:1224-1236`) is the only thing standing between
   `draw_tiles`/`Player::tiles` and every client, and it is a field-by-field
   hand-written conversion with no assertion that a future field addition
   stays out. A one-line serde-key assertion mirroring
   `texas-holdem-2`'s `pub_state_does_not_leak_hands_or_deck` would close it.
   (Distinct from the pre-ruled F-21, which is about `acquire-1` having no
   WP-08 *equivalence* test.)
3. **No crate tests `validate()`, because none of the three has one** (see
   F-36, F-37, and F-41's sibling). Sweep 1 in `00-sweeps.md` shows 13 of 28
   crates still lack the override, so F-06's recorded count ("only 15 of 27")
   understated the crate list but not the ratio. All three crates in this
   sub-unit are in the missing set.
4. **`texas-holdem-2`'s `raise` bound tests never exercise the short-stack
   case** (`raise_parser_min_bound_uses_min_raise`, `lib.rs:1002-1020`, pins
   `player_money = 100`), which is how F-44 survived.
5. **`splendor-2`'s "quirk 1"** (visiting an unaffordable noble succeeds by
   index, `lib.rs:505-526`, `command.rs:138-155`) is preserved from the Go
   source and *is* tested
   (`test_visit_two_or_more_offers_choice_including_unaffordable`,
   `lib.rs:1132-1161`), so it is a deliberate parity decision rather than a
   defect. Flagging only so a later reviewer does not re-raise it. Related:
   `visit_parser` builds `Int { min: 1, max: nobles.len() }`
   (`command.rs:147-150`), which inverts if `nobles` is empty - unreachable
   today because `visit_phase` only leaves the phase in `Visit` when two or
   more nobles are affordable, but it is another reason `validate` should
   assert `!nobles.is_empty() || phase != Phase::Visit`.
6. **Sweep 2 classification correction:** the sweep report lists
   `texas-holdem-2` and `splendor-2` under "populate stats meaningfully".
   They do not - they emit `vec![HashMap::new(); players]` /
   `vec![Default::default(); players]`, i.e. per-player *empty* maps. They are
   not F-35 instances (the arity is right) but they carry no statistics
   either. Anyone acting on `00-sweeps.md` should re-derive that column.
