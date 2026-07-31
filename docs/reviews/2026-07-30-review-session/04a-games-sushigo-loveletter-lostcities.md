# Unit 04a review - sushi-go-2, love-letter-2, age-of-war-2, lost-cities-1, lost-cities-2

Status: COMPLETE.

Scope: Unit 04 commits touching only these five crates. Sub-unit 04b (red7,
zombie-dice, battleship, for-sale, category-5, cleanup, parity) excluded.

**Commits reviewed** (established by `git log f0589894..HEAD` restricted to the five crate
paths, so nothing touching them is missed):

| Commit | WP | Crates touched in this unit |
|---|---|---|
| `ed88fab9` | WP-28 | lost-cities-1, lost-cities-2 |
| `63f4aa91` | WP-81 | lost-cities-1, lost-cities-2 |
| `c078c3ee` | WP-09b | age-of-war-2, lost-cities-2, love-letter-2 (**not** lost-cities-1, **not** sushi-go-2) |
| `f13450a1` | WP-08 | age-of-war-2, love-letter-2, sushi-go-2 |
| `eb49ceca` | WP-27 | age-of-war-2, love-letter-2 |
| `66053159` | WP-24 | sushi-go-2 |
| `3174b3fc` | T3-B4 | love-letter-2 |
| `ae04843c` | (BACKLOG #59) | sushi-go-2 |
| `22d00b8d` | WP-73 | all five (bin boilerplate only) |
| `4fb252da` / `2c28ae85` | WP-64 / WP-65 | all five (manifest only) |

`650e924e` (WP-83 parity) and `90dae6d2` (WP-10) touch none of these five crates.

**Files reviewed in final form:** `lost-cities-1/src/lib.rs`, `lost-cities-2/src/lib.rs`,
`sushi-go-2/src/lib.rs`, `love-letter-2/src/lib.rs` (partial: log/reveal paths, `end_round`,
`eliminate`, `discard_card`), plus `rust/lib/cmd/src/requester/gamer.rs` as the
trust-boundary cross-check.

**Specs recovered from git history** (`868094a6:.../planning/`):
`specs/WP-28-lost-cities-shared-fixes.md` (737 lines),
`specs/WP-10-pub-state-hidden-info-redaction.md`, `specs/WP-81-stats-deletions.md`,
`checklists/T3-B4-sushigo-loveletter-ageofwar.md` (107 lines).
**No WP-24 or WP-27 spec has ever existed** in any commit - confirmed by full tree listing
at `868094a6` plus `git log --all --diff-filter=A -- '*WP-24*' '*WP-27*'`. Those two work
packages' only written acceptance criteria are the one-line-per-row `Fix` column and
`Test?` flag in the T3-B4 checklist, which is what F-65 is measured against.

Findings numbered from F-60.

## Findings

### F-60 (High) lost-cities-1: raw-index panic on render path, explicitly routed to WP-09 and never picked up

`rust/game/lost-cities-1/src/lib.rs:545-559`

```rust
fn player_state(&self, player: usize) -> Self::PlayerState {
    PlayerState {
        public: self.pub_state(),
        player,
        // Documented (and DATA_DOCS.md) contract: sorted by expedition
        // then value, which is exactly Card's derived Ord. Indexing is
        // left unchecked deliberately - the bounds fix for a crafted
        // PlayerRender is WP-09's (e F18 / e F36), not ours.
        hand: {
            let mut hand = self.hands[player].clone();
```

This is the "routed to WP-NN" pattern (cf. F-55, F-57) in its purest form: the
remediating commit left an in-code comment declaring the bounds problem to be
WP-09's, and WP-09b's per-crate `validate()` sweep never covered lost-cities-1
(sweep 1 confirms: lost-cities-1 is one of the 13 crates with no
`Gamer::validate` override, so the trust boundary is fail-open here per F-06).
The result is that neither package owns the fix and the raw index survives.

Why it matters: `player_state()` is called by the web layer on every render.
A persisted/deserialized `Game` with `hands` shorter than the player index
(e.g. `hands: []`, which `serde` accepts because `Vec<Vec<Card>>` has no
length constraint and there is no `validate()` to reject it) panics the render
path for every viewer, not just the acting player. Reachable from render alone,
which is strictly worse than an uncommandable state.

Same class, command path, same file: `self.stats[player]` is indexed raw at
lines 211, 212, 261, 262, 308 and 373 (`draw`, `take`, `discard`, `play`), while
`player_stats()` at line 433 defensively guards the *same* vector with
`if player >= self.stats.len()`. The guard proves the author knew the vector
could be short, and then indexed it raw five lines away.

**Routing chain traced end to end (this is the load-bearing part).** WP-28's spec
(`868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/WP-28-lost-cities-shared-fixes.md`,
Non-Goals) not only defers e F18 / e F36 to WP-09, it *forbids* fixing it here:

> **Task 3 edits the very line that panics - it MUST keep the `self.hands[player]`
> indexing form and must NOT switch to `.get(player)`.** Writing
> `self.hands.get(player).cloned().unwrap_or_default()` here would silently discharge a
> major finding owned by another package [...] and make WP-09's own red test
> un-reproducible.

and states the receiving fix as "one bounds check at `lib/cmd/src/requester/gamer.rs`".
WP-09a did land that check - `check_player` at
`rust/lib/cmd/src/requester/gamer.rs:24-36`, wired into `Request::Play` and
`Request::PlayerRender`, with `validate()` called on all four state-carrying request
types. **But `check_player` compares `player` against `game.player_count()`, and
lost-cities-1's `player_count()` returns the `PLAYERS` constant (`lib.rs:633-635`), not
`self.hands.len()`.** So for this crate the boundary check is vacuous against the actual
defect: `player` is always 0 or 1 and always passes, while `hands` can still be empty.

The complementary half - a per-crate `validate()` rejecting the short vector - is exactly
what WP-09b was for, and WP-09b (`c078c3ee`) touched age-of-war-2, lost-cities-2 and
love-letter-2 but **not** lost-cities-1. The panic is therefore live today via
`Request::Status` -> `handle_status` -> `renders()` (`gamer.rs:113-114`, loops
`0..player_count()`) -> `player_state(0)` -> `self.hands[0]`.

Remediation: implement `Gamer::validate` for lost-cities-1 asserting
`hands.len() == PLAYERS && scores.len() == PLAYERS && expeditions.len() ==
PLAYERS && stats.len() == PLAYERS && current_player < PLAYERS`, and change
`player_state`'s `self.hands[player]` to
`self.hands.get(player).cloned().unwrap_or_default()`. Do not rely on
`validate()` alone for the render path - it is only invoked on the
deserialization boundary, and defence in depth is the whole point of the D-36
decision.

### F-61 (High) sushi-go-2: `player_state()` was bounds-hardened but `status()` / `pub_state()` on the same render path were left raw-indexed

`rust/game/sushi-go-2/src/lib.rs:798-818` vs `224-268`, `780-796`

`player_state()` is carefully defensive - three separate `if player < self.hands.len()` /
`if player < self.playing.len()` guards:

```rust
hand: if player < self.hands.len() { self.hands[player].clone() } else { vec![] },
playing: if player < self.playing.len() { self.playing[player].clone() } else { None },
dummy_playing: if self.players == 2 && player == self.controller {
    self.playing[DUMMY].clone()          // <-- raw index, same fn, no length guard
} else { None },
```

and then, in the very same function, `self.playing[DUMMY]` (i.e. `self.playing[2]`)
is indexed raw behind a guard that tests `players` and `controller` but never
`playing.len()`.

The rest of the render path was not hardened at all:

- `is_finished()` (line 224): `self.hands[0]` is guarded by `!self.hands.is_empty()`, but
  `self.playing[0]` on the next line is not guarded by anything. `is_finished()` is called
  from both `status()` and `pub_state()`.
- `can_dummy()` (line 245): `self.playing[DUMMY]` raw; reached from `whose_turn_inner()`
  -> `status()`.
- `pudding_cards()` (line 257): `self.played[player]` raw; reached from `placings()` ->
  `status()`.
- `placings()` (line 265): `self.player_points[p]` raw.
- `pub_state()` (line 791): `self.player_points[p]` raw.
- `draw_count()` (line 145): `unreachable!()` for any `all_players` outside 2..=5, reached
  from `end_round()` -> `start_round()` on the command path.

sushi-go-2 has no `Gamer::validate` override (sweep 1 confirms; it is one of F-06's
missing-13), so `Gamer::validate` defaults to `Ok(())` and nothing rejects a persisted
state whose `playing` / `played` / `player_points` vectors are shorter than
`all_players`. `status()` and `pub_state()` are called by the web layer on every render,
so a single malformed persisted state takes down the page for every viewer of the game,
not just the acting player.

Why this is a finding in its own right rather than just an F-06 instance: someone did
walk this crate's boundary and add bounds checks - they hardened exactly the one function
(`player_state`) and stopped, leaving four raw-index sites on the *same* render path plus
a raw index inside the function they hardened. That is a partial/symptom-papering fix, and
the inconsistency inside a single function body is the tell.

Remediation: implement `Gamer::validate` for sushi-go-2 asserting
`(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)`,
`all_players == if players == 2 { 3 } else { players }`,
`hands.len() == playing.len() == played.len() == player_points.len() == all_players`,
`controller < players` and `(1..=TOTAL_ROUNDS).contains(&round)` - lost-cities-2's
`validate()` (`rust/game/lost-cities-2/src/lib.rs:550-577`) is the right shape to copy.
Then convert the five render-path raw indexes to `.get(..)` with a defined fallback, and
either drop `draw_count`'s dead `2` arm or make the function total.

### F-62 (Medium) lost-cities-1 and lost-cities-2 share the same file, but only -2 got a `validate()`

`rust/game/lost-cities-1/src/lib.rs` (no `fn validate`) vs
`rust/game/lost-cities-2/src/lib.rs:550-577`

lost-cities-2 is a near-verbatim generalisation of lost-cities-1 over `self.players`:
identical `Game` field list, identical `PubState`/`PlayerState`, identical
`draw`/`take`/`discard`/`play`/`draw_hand_full`/`player_stats`, and - the tell - the
*same* five-line comment in `player_state` declaring the bounds fix to be WP-09's. -2
received a full `validate()` covering `players` range and the `hands` / `scores` /
`expeditions` / `stats` lengths plus `current_player`, and a `validate_works` test.
-1 received nothing.

There is no crate-specific reason for the split: every invariant -2 validates exists
verbatim in -1 (with `PLAYERS` fixed at 2 instead of `self.players`). This is WP-09b's
sweep stopping short, and it is the mechanism behind F-60.

Remediation: port lost-cities-2's `validate()` to lost-cities-1 with `self.players`
replaced by the `PLAYERS` constant, and port `validate_works` with it.

### F-63 (Low) lost-cities-2: `unreachable!()` in three player-count lookups on the command path

`rust/game/lost-cities-2/src/lib.rs:730-752`

`expedition_cost`, `hand_size` and `expedition_bonus_size` all `unreachable!()` on any
`players` value other than 2 or 3. `hand_size(self.players)` is called from
`draw_hand_full`, which is on the command path, and `score(self.players, ..)` is a `pub fn`
reachable from `end_round`. The `validate()` at line 550 does gate `players` into
`MIN_PLAYERS..=MAX_PLAYERS`, so this is defence-in-depth rather than a live panic - but it
relies entirely on `validate()` being invoked at every deserialization boundary, and
`Game::default()` (used by `..Game::default()` in `start`) has `players == 0`.

Remediation: return `Result` or fall back to the 3-player values with an internal error
log rather than `unreachable!()`. Low because `validate()` covers the realistic path.

### F-64 (Low) lost-cities-2: the 3-player scoring constants have no test

`rust/game/lost-cities-2/src/lib.rs:32-35, 754-782`; test at `877-929`

`EXP_COST_3P = 15` and `EXP_BONUS_SIZE_3P = 7` are the only genuinely new *scoring* rules
in lost-cities-2 relative to lost-cities-1, and `score_works` calls `score(2, ..)` for all
six of its assertions - the 3-player branch of `expedition_cost` /
`expedition_bonus_size` is never exercised. Note also that line 780 uses `exp_cost` as the
completion-bonus *value* (`if cards >= exp_bonus_size { exp_cost }`), coupling two
unrelated quantities that happen to be equal at 2p (20/20); at 3p this silently makes the
bonus 15. Whether that is the intended 3p rule is untested and undocumented.

Remediation: add `score(3, ..)` assertions covering the 15-point expedition cost, the
7-card completion bonus threshold, and the bonus value; and either introduce an explicit
`EXP_BONUS_3P` constant or comment the deliberate `exp_cost` reuse.

### F-65 (Medium) WP-24 satisfied `d F28` literally and missed the entire point of the row; a stranger's backlog item closed it two days later

`rust/game/sushi-go-2/src/lib.rs:140-147`; commits `66053159` (WP-24) then `ae04843c`

T3-B4's row reads:

| Finding | Fix (one line) | Test? |
|---|---|---|
| `d F28` | Replace the table lookup + `.unwrap_or(9)` with an exhaustive `match players` **so no caller can silently fall back**, keeping 2/3 => 9, 4 => 8, 5 => 7 | y |

What `66053159` shipped:

```rust
fn draw_count(players: usize) -> usize {
    match players {
        2 | 3 => 9,
        4 => 8,
        5 => 7,
        _ => 9,        // <-- the silent fallback, preserved verbatim
    }
}
```

The table lookup became a `match` (the row's literal instruction) while `_ => 9`
reproduced `.unwrap_or(9)`'s behaviour bit for bit - the exact defect the row existed to
remove. The row is not ambiguous; it states the purpose inline. WP-24 was then marked
done, T3-B4 recorded "d F26-F32 via 6605315" as landed (see `3174b3fc`'s commit message),
and nothing in the programme noticed. The fallback survived until `ae04843c` on 2026-07-30
- a *separate*, untagged commit closing BACKLOG #59, i.e. it was rediscovered from the
outside rather than caught by the remediation programme.

The row also carries `Test? y`. Neither `66053159` nor `ae04843c` added any test for
`draw_count`; `66053159` is net test-*negative* for this crate (it deletes
`test_hand_passing_left` and adds nothing).

Secondary problem with the eventual fix: `ae04843c`'s justification is factually wrong.

> The `_ => 9` fallback was domain-dead: start() rejects player counts outside 2..=5
> before draw_count is called, so it could never fire

`draw_count` is called as `draw_count(self.all_players)` (line 289), not with `players`,
and `all_players`/`players` arrive from *deserialized* state on every request, not only
from `start()`. sushi-go-2 has no `validate()` override (F-61), so `start()` is emphatically
not the only entry point. The change therefore converted a harmless silent fallback into a
command-reachable `unreachable!()` panic - a small regression justified by an incorrect
premise. Note also that `draw_count(2)` is itself unreachable (2 players means
`all_players == 3`), so the `2` arm is dead - which the commit's own dead-arm analysis
missed.

Remediation: make `draw_count` total by returning `Result` or by keying it on a validated
newtype; add the test `d F28` asked for; and land sushi-go-2's `validate()` (F-61) so the
`unreachable!()` premise becomes true.

## Verified good

### Hidden-information audit (the priority check) - every `Log::public` / `Log::private` site read by hand

**love-letter-2** (`src/lib.rs`) - the highest-risk crate in the unit, and it is correct.
Every private-information reveal is scoped to exactly the players the rules entitle:

- `play_king` (400-424): public log says only *that* hands were swapped and with whom; two
  separate `Log::private` entries, `vec![player]` and `vec![target]`, each phrased from that
  recipient's side. Both recipients legitimately learn the other's card - that is the King's
  effect. Recipient lists are not transposed (checked individually).
- `play_baron` (497-528): same shape, `vec![player]` and `vec![target]`; the public log
  states only that a comparison is happening. The loser's card becomes public only via
  `eliminate` -> `discard_card_log`, which is the correct rules behaviour.
- `play_priest` (571-581): public "played Priest"; the peeked card goes to
  `Log::private(private_log, vec![player])` - the peeker alone. Correct.
- `draw_card` (249-267): public log carries only "drew a card from the draw pile, N
  remaining" / "drew a card from the removed cards" and the deck count; the card identity is
  private to the drawer.
- `eliminate` (156-173) and `play_prince` (449-450): publicly reveal the discarded hand,
  which is exactly what the rules require of an eliminated or Prince-targeted player.
- `end_round` (175-231): reveals surviving hands publicly, but the round is over and
  `start_round` reshuffles, so nothing exploitable persists.
- `play_countess` (367-375): the "they might have been forced to" wording deliberately
  leaks *ambiguity*, not information - it does not disclose whether the King/Prince was
  actually held.

`PubState`/`PlayerState` redaction is additionally covered by
`pub_state_does_not_leak_hidden_info` (sweep 3), so love-letter-2 is the one crate in this
unit tested at both layers.

**sushi-go-2** - correct. `start_hand` (313-321) sends the card drawn from the dummy's
hand as `Log::private(.., vec![self.controller])`; only the controller may see it, and only
the controller gets `dummy_playing` in `player_state`. `end_hand` (335-339) logs all plays
publicly *after* every seat has committed (`play_cards` returns `Ok(vec![])` early until
`playing[p].is_some()` for all `p`), so the simultaneous-reveal invariant holds and no
early player's choice leaks. `PubState` carries `played` and `player_points` only - no
hands, no deck.

**age-of-war-2** - correct, and worth flagging for the record: **age-of-war-2 is not a
hidden-hand game.** The unit brief asserts all five crates are hidden-hand; that is right
for the other four but not this one. Its `PubState` is deliberately total (the sweep-3 test
is named `pub_state_carries_full_public_info`), it has no `Log::private` site anywhere, and
its five `Log::public` sites (190, 199, 230, 234, 317, 365) carry only dice results, castle
conquests and clan completions - all public by the rules. Nothing to redact.

**lost-cities-1 / lost-cities-2** - correct at the log layer. `draw_hand_full`
(-1:397-414, -2:441-458) splits the draw into a public count-only log and a
`Log::private(.., vec![player])` carrying the card identities; `take`, `discard` and `play`
log publicly, which is right because all three move cards to or from public zones. No
`Log::public` site in either crate carries a hand card that is not already public.

### WP-24 (sushi-go-2, `66053159`) - per-row verification

- `d F27` **legitimate deletion.** `test_hand_passing_left` was genuinely vacuous: its body
  contained zero assertions, ended in three comments conceding "we can't easily check
  passing here", and its only executed check was `g.command(MICK, "play 1", &n).unwrap_err()`
  - asserting an *error*, unrelated to passing. The checklist explicitly permitted deletion
  ("real coverage lives in test fn `test_passing_direction`"), and `test_passing_direction`
  does exist, immediately following the deleted test. Not a test removed to make something
  pass.
- `d F29` ✓ Pudding explanation now reads "end: most 6, least -6 (no penalty in 2p)"
  (line 123).
- `d F30` ✓ the `second_players.len() <= 3` condition is gone (line 437); integer division
  already yields 0 for a 4-way second-place tie, so only the suppressed log changed.
- `d F31` ✓ one implementation only - `Game::render_name` (248-250) delegates to
  `render::render_name`, and the dummy test became `player >= players`. This also removed a
  latent `players - 1` underflow panic on a `players == 0` state, which the row did not
  ask for and is a genuine improvement.
- `d F32` ✓ guard reordered so `self.players == 2` short-circuits before
  `self.playing[DUMMY]` (line 672-676), matching `can_dummy`.
- `d F26` ✓ doc-only; the pudding tiebreak paragraph was added to `RULES.md` and
  `placings()` was correctly left alone.
- `d F28` ✗ - see F-65.

### WP-27 (`eb49ceca`) + T3-B4 (`3174b3fc`) - per-row verification

- `e F5` ✓ `command_parser` returns `None` on `check_finished()`
  (`love-letter-2/src/command.rs:27-29`), so a finished game offers no commands.
- `e F6` ✓ / `e F7` ✓ PORTING_NOTES gained the Guard self-target ordering entry recording
  the deliberate Go-mirroring quirk.
- `e F8` ✓, but late: WP-27's own commit did **not** implement it despite owning the row.
  It landed two days later in `3174b3fc`. **This is the routing/verification machinery
  working correctly** - the T3-B4 pass re-checked all 15 rows, found the one that had not
  landed, fixed it, and said so explicitly in the commit message ("The other 14 T3-B4 rows
  verified already landed with zero change"). Recorded as a positive, not a finding.
- `e F11` ✓ `HashSet<usize>` -> `BTreeSet<usize>` for `completed_lines`, and the
  now-redundant `completed.sort_unstable()` in `pub_state` was correctly removed rather
  than left as dead belt-and-braces. Deterministic serialization achieved.
- `e F12` ✓ `GameError::NotYourTurn` replaces `invalid_input("not your turn")`.
- `e F15` ✓ single shared `clan_conquered_data(&[bool], &[Option<usize>], Clan)` helper,
  called from both `Game::clan_conquered` and `render::clan_conquered`; the stale-player-on-
  `false` quirk is preserved verbatim, including the two distinct false returns
  (`(false, None)` vs `(false, player)`).
- `e F16` ✓ "discard one die and reroll the rest".

`eb49ceca` also narrowed eight `play_*` methods from `pub fn` to `fn`, which is not on any
checklist row. Out of declared scope, but it shrinks the crate's public surface with no
behavioural effect and is the right call.

### WP-08 epilogue dedup (`f13450a1`) - sushi-go-2

Correct, and better than the row required. The `was_finished` snapshot taken *before*
dispatch (line 835) plus a single `if !was_finished && self.is_finished()` guard (849)
replaces two copy-pasted blocks and is strictly safer than the originals, which fired on
`is_finished()` alone and would have re-emitted a placings log for a command issued on an
already-finished game. `test_finish_epilogue_single_placings_log` exercises all three
distinct finishing paths (3p play arm, 2p dummy arm, 2p non-controller play arm) and
asserts *exactly one* placings log, that it is last, and that `can_undo` is false. This is
the strongest test in the unit.

### WP-28 (`ed88fab9`) - lost-cities

All 13 rows verified present in the end state: `e F17` (`stats: (0..self.players)...`,
-2:583), `e F19`/`e F37` (`logs.extend(self.end_round()?); Ok(logs)`, -1:418-421 /
-2:462-465), `e F20`/`e F38` (`hand.sort()`, -1:553-557 / -2:623-627), `e F24`
(generalised `game_over_log` via `leaders()`, sorted for determinism with an explicit
comment saying why, -2:196-230), `e F26`/`e F41` (`saturating_sub`, -1:385 / -2:429),
`e F43` (`let Some(&cards) = exp_cards.get(&e) else { return acc; }`, -1:673 / -2:776),
`e F42` (`PLAYERS` const used throughout -1).

Two things the spec got right and the implementation honoured:

- `e F24`'s generalisation used `leaders()` rather than copying -1's two-player-hardwired
  `game_over_log`, and explicitly sorts the `HashSet` before formatting ("Sorted for
  determinism: leaders() returns a HashSet and this text is written into the permanent log
  stream", -2:197-198). Getting determinism right in a log that is persisted forever is
  easy to miss.
- `e F23`'s finding recommendation (`% self.players`) was correctly **overturned** in the
  spec - it would have introduced a divide-by-zero on a `PubState` with `players == 0` -
  in favour of the clamp form already used elsewhere in the same file. The implementation
  followed the spec, not the finding.

### WP-81 (`63f4aa91`) - dead stats machinery

Deletion only (-16 / -11 lines, no insertions), removing `Stats.investments` and
`Stats.expeditions` from both lost-cities crates. `player_stats()` in both crates now
surfaces exactly the four counters it increments (`Plays`, `Discards`, `Draws`, `Takes`),
with `turns` as the denominator. No dead field remains.

### F-35 occurrences (recorded only, per instruction - not re-raised)

`Status::Finished { stats: vec![] }` at `rust/game/sushi-go-2/src/lib.rs:770` and
`rust/game/age-of-war-2/src/lib.rs:468`. The other three crates in this unit populate
`stats` meaningfully: `lost-cities-1/src/lib.rs:514`
(`vec![self.player_stats(0), self.player_stats(1)]`), `lost-cities-2/src/lib.rs:583`
(`(0..self.players).map(|p| self.player_stats(p)).collect()`), and
`love-letter-2/src/lib.rs:669` (`vec![Default::default(); self.players]` - correct length
but empty maps, so functionally an F-35 instance with the right shape).

## Coverage gaps

1. **No log-layer test in any of the five crates.** This unit's hidden-information audit
   was done entirely by reading; nothing in these crates would fail if a `Log::private`
   recipient list were changed to `Log::public`, or if `play_king`'s two private logs were
   transposed. love-letter-2 is the crate where this matters most (five distinct
   private-reveal paths, each with a different correct audience) and it has zero
   assertions over `Log.public` or `Log.to` for any of them. Suggested minimal shape: for
   each `play_*` with a private reveal, assert the log's `public` flag is false and its
   recipient vector equals the expected single-player vector. This is the same structural
   gap the programme's own retrospective identified ("no game crate tests the log layer")
   and it is still open in all five crates.

2. **WP-10 3a redaction-shape compliance, per crate:**
   - `sushi-go-2` - **compliant.** `test_pub_state_redacts_hands` exists (lib.rs:1445) and
     `pub_state()` constructs its fields rather than cloning `hands`/`deck` through.
   - `love-letter-2` - **compliant.** `pub_state_does_not_leak_hidden_info` (lib.rs:1152).
   - `age-of-war-2` - **n/a**, no hidden information (see Verified good).
   - `lost-cities-1` - **non-compliant on testing.** `pub_state()` itself is correctly
     constructive (it reduces `deck` to `deck_remaining` and `discards` to a top-card map,
     per shape rules 1/2/5), but there is no test asserting `hands` stays out of
     `PubState`. Sweep 3 confirms no `pub_state()` call appears in any `#[test]` fn.
   - `lost-cities-2` - **non-compliant on testing**, identically. Note `player_state` *is*
     tested (`player_state_hand_is_sorted_as_documented`), which makes the absence of the
     redaction assertion easier to miss: the test suite touches `player_state(0).hand` but
     never checks that the same data is absent from `pub_state()`.

   Both lost-cities crates therefore confirm the brief's premise: WP-10's shape was
   declared "for every game crate" and neither of these two was ever swept. The
   remediation is one test per crate, ~10 lines, copying
   `love-letter-2::pub_state_does_not_leak_hidden_info`.

3. **F-06 per-crate status** (requested confirmation):
   - `sushi-go-2` - **missing `validate()`, and exploitable from render alone.** See F-61.
     `status()` -> `is_finished()` -> `self.playing[0]`, and `status()` ->
     `whose_turn_inner()` -> `can_dummy()` -> `self.playing[DUMMY]`, and `status()` ->
     `placings()` -> `pudding_cards()` -> `self.played[player]`. Three independent
     render-path panics behind a fail-open trust boundary.
   - `lost-cities-1` - **missing `validate()`, and exploitable from render alone.** See
     F-60. `renders()` -> `player_state(p)` -> `self.hands[player]`.
   - `lost-cities-2` - **has `validate()`** (lib.rs:550-577), covering `players` range and
     all four parallel-vector lengths plus `current_player`. Compliant.
   - `love-letter-2` - **has `validate()`** (lib.rs:809). Compliant.
   - `age-of-war-2` - **has `validate()`** (lib.rs:437). Compliant.

   In both non-compliant crates the exploitable pattern is exactly the one the brief
   predicted: parallel per-player vectors indexed raw inside `status()` / `player_state()`,
   which the web layer calls on every render. `check_player` at
   `rust/lib/cmd/src/requester/gamer.rs:24-36` does not help, because it bounds `player`
   against `player_count()` - which is a constant in lost-cities-1 and a stored field in
   sushi-go-2, neither derived from the vectors' actual lengths.

4. **`draw_count` has no test** despite T3-B4 marking `d F28` as `Test? y`; neither
   `66053159` nor `ae04843c` added one. The per-player-count deal sizes (9/9/8/7) are
   asserted only indirectly, via `test_deck`'s total and the 2p/3p flows in
   `test_finish_epilogue_single_placings_log`. A 4- or 5-player game's deal size is never
   checked.

5. **lost-cities-2's 3-player scoring is untested** - see F-64. `EXP_COST_3P` /
   `EXP_BONUS_SIZE_3P` are never exercised by `score_works`.

6. **Not reviewed (out of sub-unit 04a):** red7-1, zombie-dice-2, battleship-2, for-sale-2,
   category-5-2, and the WP-83 parity fixes - these are 04b. `650e924e` (WP-83) touches
   none of this unit's five crates, confirmed by the filtered diffstat.
