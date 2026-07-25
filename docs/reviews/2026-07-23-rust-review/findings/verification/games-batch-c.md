# Verification: games-batch-c (unit 5)

Independent verification of `findings/games-batch-c.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/texas-holdem-2`, `game/acquire-1`, `game/cathedral-2`,
`game/sushizock-2`. Raw verdict dumps: `raw/games-batch-c-th.md`,
`raw/games-batch-c-acquire.md`, `raw/games-batch-c-cathedral.md`,
`raw/games-batch-c-sushizock.md`. Process log: `games-batch-c-LOG.md`.

## Per-finding verdicts

### texas-holdem-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | Raise parser min diverges from Go, comment wrong | minor | CONFIRMED | Go command.go:174 uses `min := g.MinRaise()`; Rust command.rs:51 uses `self.largest_raise`; comment at command.rs:43-48 misattributes the quirk (real one is CanRaise, texas_holdem.go:326-332, preserved at lib.rs:298-310); pre-flop 5..9 parse-then-reject window checks out |
| F2 | MAX_PLAYERS 8 vs Go 9 | minor | CONFIRMED | lib.rs:33 `MAX_PLAYERS = 8` vs texas_holdem.go:58 (2-9); no documenting comment |
| F3 | bet_up_to `.expect()` in runtime path | nit | CONFIRMED | lib.rs:158; invariant locally provable; Go panics at texas_holdem.go:207-210 |
| F4 | Documented Go-mirroring panics | nit | CONFIRMED | All next_player_in_set call sites guarded; pop_n deck math 16+5=21 <= 52 |
| F5 | `Option<Category>` redundant with `Category::None` | nit | CONFIRMED | poker.rs:31; `unwrap_or(Category::None)` at poker.rs:39 and 54 |
| F6 | Placings-log block x5 | nit | CONFIRMED | Identical blocks at lib.rs:720-730/738-748/756-766/774-784/792-802; can_undo true only in Raise arm (line 800) |

### acquire-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F7 | player_counts() excludes 6 players | major | CONFIRMED | `(2..6).collect()` = [2,3,4,5] at lib.rs:313 vs MAX_PLAYERS=6 (lib.rs:25) and start() accepting 6 (lib.rs:186); trait method feeds the platform's advertised counts, so 6p games can never be offered |
| F8 | Dummy die roll 1..=5, never 6 | major | CONFIRMED | lib.rs:902 `random_range(1..=5)`; contradicts BOTH the start log ("A dice (D6)") and RULES.md:153-154 ("result (1-6)") — two in-repo sources, not just external rules |
| F9 | panic! in pay_bonuses | minor | CONFIRMED | lib.rs:841; reachable from command(); unreachability holds under game invariants but not for deserialized/foreign states |
| F10 | expect() cluster on HashMap keys | minor | CONFIRMED | All 10 cited expect sites verified incl. "could not et player shares" typo (command.rs:163); unwrap_or(0) inconsistency at lib.rs:616/813/909/965 and render.rs:138 |
| F11 | "Trades" stat reports merges | minor | CONFIRMED | stats.rs:46 inserts `self.merges`; `stats.trades` maintained at lib.rs:1095 |
| F12 | Stats tracked, never surfaced | minor | CONFIRMED | `stats: vec![]` at lib.rs:238; `to_brdgme_stats` has zero callers workspace-wide |
| F13 | Random start player vs tile draw | minor | CONFIRMED | lib.rs:213 `random_range(0..players)`; initial tiles not player-associated; official-setup premise is external (crate RULES.md silent) |
| F14 | Full-hand redraw discards temp-unplayable tiles | minor | CONFIRMED | start_turn (lib.rs:693-708) tests via assert_loc_playable (board.rs:130-142, rejects both permanent and temporary cases); redraw_hand set_discards the ENTIRE hand (lib.rs:730-731); end-of-turn path partitions only on the permanent condition (lib.rs:377-380) |
| F15 | Bag exhaustion ends game mid-turn | minor | CONFIRMED | lib.rs:403-408 `self.end()` when bag < refill need; RULES.md end conditions don't include tile exhaustion; edition premise external. Compounds with F14: a mass redraw can drain the bag and trigger the premature end |
| F16 | Unused thiserror dep | minor | CONFIRMED | Declared in Cargo.toml:14, zero grep hits in crate |
| F17 | can_undo tautology | nit | CONFIRMED | buy_phase unconditionally sets Phase::Buy before the matches! at lib.rs:586 |
| F18 | unwrap() on 1-element set | nit | CONFIRMED | lib.rs:466, guarded by `1 =>` arm |
| F19 | unwrap() in render row-run | nit | CONFIRMED | render.rs:268-270, safe by construction |
| F20 | Full-game clone for can_end | nit | CONFIRMED | pub_state() is `self.to_owned().into()`; runs per command_parser build; PubState::can_end needs only board + 2 flags |
| F21 | Nondeterministic found-parser corp order | nit | CONFIRMED | available_corps() returns HashSet<Corp> feeding Enum::partial |

### cathedral-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F22 | Box::leak per parser construction | major | CONFIRMED | loc_parser rebuilds and leaks 100 strings on every command()/command_spec(); "one-time" comment false; Enum only needs ToString + Clone, so the 'static is self-imposed by LocChoice.name |
| F23 | Cathedral traversable by capture flood-fill | minor | ADJUSTED (minor -> nit) | All code facts verify (lib.rs:283 walk condition, Go parity play_command.go:218, PLAYER_CATHEDRAL=2, no code comment) — but the "undocumented" premise is wrong: the crate's own RULES.md documents the cathedral as a capturable piece identity inside enclosures ("counts as a piece identity ... exactly like an opponent piece"), matching the implementation. Documented intended behavior; the official-rules-wall claim rests solely on the reviewer's external rules knowledge. Residual: add a code comment at the walk site cross-referencing RULES.md |
| F24 | Dead parse_loc | minor | CONFIRMED | loc.rs:167, zero callers repo-wide; pub suppresses the lint |
| F25 | pieces() panics on bad player index | minor | CONFIRMED | Panic reachable from Gamer::command with player>=2 (can_play -> can_play_something -> pieces); harness (requester/gamer.rs:130) forwards player unvalidated |
| F26 | Loc::to_key overflow | nit | CONFIRMED | render.rs:45 guards with loc.valid(); Game::tile_at (lib.rs:85-90) does not; no live panic path today |
| F27 | Unused rand dep | nit | CONFIRMED | Unused in src/, bins, tests; seed ignored; fuzz bin uses brdgme_fuzz |
| F28 | Dead Display for Loc | nit | CONFIRMED | loc.rs:118 never invoked; no `"{}"` formatting of Loc anywhere |

### sushizock-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F29 | Steal n = i32::MIN overflow | major | CONFIRMED | steal parser tile index is `Int::any()` (command.rs:75); can_steal_*_n never touches n; `len as i32 - n` at lib.rs:460/502 overflows for n = i32::MIN with len >= 1; default dev/test profiles panic (no profile overrides overflow-checks); release wraps and is luckily caught by the index<0 guard |
| F30 | Roll-arm finish misses placings log | minor | CONFIRMED | Take (lib.rs:732-737) and Steal (lib.rs:753-758) arms append placings_log on finish; Roll arm (lib.rs:711-722) does not; roll_dice_cmd can finish via take_worst (lib.rs:612) |
| F31 | roll Many suggest cross-reference | minor | CONFIRMED | command.rs:47 as described; suggest.rs:109 destructures `Spec::Many { spec, delim, .. }`, discarding min/max (parse honors max, suggest ignores it) |
| F32 | .unwrap() in roll_dice | nit | CONFIRMED | lib.rs:151, choose over 6-element DIE_FACES const; infallible |
| F33 | take_worst hand-rolled min loops | nit | CONFIRMED | Both branches hand-roll find-min; `blue_tiles[0]` at lib.rs:549 safe only via non-local not-finished invariant |
| F34 | take/steal near-verbatim duplicates | nit | CONFIRMED | Pairs at lib.rs:399-431 and 433-515 differ only in gates/piles/dice fields; Go original has the same duplication |

## Summary

- Findings verified: 34
- CONFIRMED: 33, ADJUSTED: 1 (F23), REJECTED: 0, UNVERIFIABLE: 0
- Corrected tallies for the unit: 0 critical / 4 major / 14 minor /
  16 nit (original: 0 critical / 4 major / 15 minor / 15 nit; F23
  downgraded minor -> nit)
- Lead spot-checked the sole adjustment (F23 via cathedral-2 RULES.md
  capture section + lib.rs:262-306) and, since W1/W2/W4 confirmed
  everything, directly re-verified their hardest confirmations: F1
  (command.rs:41-52, lib.rs:280-310, Go command.go:174 and
  texas_holdem.go:302-332), F6 (five block spans + can_undo), F14
  (lib.rs:693-735, 375-408, board.rs:130-142), F8 (lib.rs:902), and F29
  (command.rs:54-75, lib.rs:433-473, 338-348).

## Notable corrections

One severity change:

- F23 (cathedral-2 flood-fill): every code-level claim is accurate — the
  capture walk at lib.rs:283 does not block on cathedral tiles, this is
  verbatim Go parity, and no code comment flags it. But the finding's
  framing as "undocumented, reads as intentional-correct rather than
  preserved-quirk" is refuted by the crate's own RULES.md, which
  explicitly documents the cathedral as a capturable piece identity
  inside enclosed regions (a region with cathedral + one opponent piece
  has two identities and is NOT captured) — exactly what the code does.
  Under the review's own rule (judge game-rule claims against the game's
  own rules docs where available), this is documented intended behavior;
  only the claim that official Cathedral rules treat the cathedral as an
  enclosure wall rests on the original reviewer's external knowledge.
  Downgraded minor -> nit (add a walk-site comment referencing RULES.md).

Evidence strengthenings recorded in the raw dumps: F8 contradicts
RULES.md:153-154 ("result (1-6)") in addition to the start log; F10's
unwrap_or(0)-style inconsistency extends to lib.rs:616/813/909/965 and
render.rs:138; F25's panic is concretely reachable because the request
harness (requester/gamer.rs:130) forwards the player index unvalidated;
F29's build-profile analysis pinned down (no profile sets
overflow-checks, so all default dev/test builds panic); F14+F15 compound
(a mass redraw discard can drain the bag and trigger the premature game
end).

Overall assessment: the original games-batch-c review is highly accurate —
all 34 locations and code traces checked out, including both acquire
majors, the cathedral Box::leak traffic-driven leak, and the sushizock
i32::MIN overflow. The only verdict-level correction is F23, where the
reviewer missed that the crate's RULES.md already documents the
flood-fill behavior as intended, flipping an "undocumented preserved
defect" minor into a documentation-polish nit.
