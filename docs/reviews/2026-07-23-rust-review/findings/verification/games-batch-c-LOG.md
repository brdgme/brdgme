# Verification LOG: games-batch-c (2026-07-24)

Independent verification of `findings/games-batch-c.md` (unit 5, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

34 findings total in games-batch-c.md, numbered F1-F34 in document order.
Four serial Workers (model fable per user override), split by crate so each
reads a coherent source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/texas-holdem-2 | F1 raise parser min bound diverges from Go + wrong comment (minor), F2 MAX_PLAYERS 8 vs Go 9 (minor), F3 bet_up_to .expect() (nit), F4 documented Go-mirroring panics next_player_in_set/pop_n (nit), F5 HandResult.category Option redundant with Category::None (nit), F6 placings-log block x5 (nit) | raw/games-batch-c-th.md |
| W2 | game/acquire-1 | F7 player_counts() excludes 6 (major), F8 dummy die roll 1..=5 not 1..=6 (major), F9 panic! in pay_bonuses (minor), F10 expect() cluster on HashMap keys (minor), F11 "Trades" stat reports merges (minor), F12 stats tracked never surfaced (minor), F13 random start player vs tile draw (minor), F14 full-hand redraw discards temp-unplayable tiles (minor), F15 bag exhaustion ends game immediately (minor), F16 unused thiserror dep (minor), F17 can_undo tautology (nit), F18 unwrap() neighbouring_corps (nit), F19 unwrap() render row-run (nit), F20 full-game clone for can_end (nit), F21 nondeterministic corp ordering in found parser (nit) | raw/games-batch-c-acquire.md |
| W3 | game/cathedral-2 | F22 Box::leak per parser construction (major), F23 cathedral traversable by capture flood-fill (minor), F24 dead parse_loc (minor), F25 pieces() panics on bad player index (minor), F26 Loc::to_key overflow on out-of-range coords (nit), F27 unused rand dep (nit), F28 dead Display for Loc (nit) | raw/games-batch-c-cathedral.md |
| W4 | game/sushizock-2 | F29 steal n = i32::MIN overflow (major), F30 Roll-arm game end missing placings log (minor), F31 roll Many suggest cross-reference (minor), F32 .unwrap() in roll_dice (nit), F33 take_worst hand-rolled min loops (nit), F34 take/steal near-verbatim duplicates (nit) | raw/games-batch-c-sushizock.md |

Game-rule correctness claims are judged against each crate's rules docs /
in-code comments and, where a Go source exists (texas_holdem_1, cathedral_1,
sushizock_1 in brdgme-go/), against the Go original; acquire-1 has no Go
source, so official-rulebook claims are checked for internal consistency and
flagged if they rest solely on the original reviewer's rules knowledge.
Lead spot-checks all REJECTED/ADJUSTED verdicts; if a Worker confirms
everything, Lead re-verifies its 1-2 hardest confirmations. Curated report:
verification/games-batch-c.md.

### W1 dispatched — texas-holdem-2 (F1-F6)

### W1 returned

All 6 CONFIRMED at stated severities. Dump: raw/games-batch-c-th.md.
- F1: Go command.go:174 `min := g.MinRaise()` vs Rust command.rs:51
  `self.largest_raise`; comment misattribution confirmed; pre-flop 5..9
  parse-then-reject window confirmed.
- F4: all next_player_in_set call sites verified guarded; deck math
  16+5=21 <= 52.

### Lead spot-checks (W1)

Since W1 confirmed everything, Lead directly re-verified its two hardest
confirmations:
- F1 upheld — read command.rs:41-52 (comment claims "Go quirk preserved:
  the Int bound's min is g.LargestRaise" and code uses
  `self.largest_raise`), lib.rs:280-310, and Go command.go:174
  (`min := g.MinRaise()`), texas_holdem.go:302-304
  (`MinRaise = max(MinimumBet, LargestRaise)`) and :326-332 (CanRaise uses
  LargestRaise). The comment is factually wrong about the parser; the real
  quirk lives in CanRaise and is correctly preserved at lib.rs:298-310.
  Minor/correctness stands.
- F6 upheld via W1's five block spans (lib.rs:720-730/738-748/756-766/
  774-784/792-802) and can_undo true only in the Raise arm (line 800).

### W2 dispatched — acquire-1 (F7-F21)

### W2 returned

All 15 CONFIRMED at original severities. Dump: raw/games-batch-c-acquire.md.
- F8 strengthened: RULES.md:153-154 also says "result (1-6)", so the 1..=5
  roll contradicts two in-repo sources, not just the start log.
- F10: unwrap_or(0)-style inconsistency sites extended to lib.rs:616/813/
  909/965 and render.rs:138.
- Extra observation: F14 and F15 compound (mass redraw discard can drain
  the bag and trigger the premature end).

### Lead spot-checks (W2)

Since W2 confirmed everything, Lead directly re-verified its two hardest
confirmations:
- F14 upheld — read lib.rs:693-735, lib.rs:375-408, board.rs:130-142.
  start_turn's no-playable test uses assert_loc_playable, which rejects
  both multiple-safe-corp mergers (permanent) and founds-with-no-available
  -corp (temporary); redraw_hand then `set_discarded`s the ENTIRE hand
  (lib.rs:730-731). draw_replacement_tiles partitions only on
  loc_neighbours_multiple_safe_corps (lib.rs:377-380). Asymmetry real;
  minor stands.
- F8 upheld — lib.rs:902 `dummy_shares = self.rng.random_range(1..=5);`
  in bonus_players for 2-player games. Major stands.

### W3 dispatched — cathedral-2 (F22-F28)

### W3 returned

6 CONFIRMED, 1 ADJUSTED (F23). Dump: raw/games-batch-c-cathedral.md.
- F22 major upheld: loc_parser rebuilds and leaks 100 strings per
  command()/command_spec(); Enum only needs ToString + Clone so 'static
  is self-imposed.
- F23 ADJUSTED: all code facts verify (walk condition lib.rs:283, Go
  parity play_command.go:218, PLAYER_CATHEDRAL=2, no code comment), but
  the "undocumented" premise is wrong — RULES.md documents the cathedral
  as a neutral capturable identity inside enclosures. Lead spot-check
  pending.
- F25 strengthened: harness (requester/gamer.rs:130) forwards player
  unvalidated, so panic is reachable from Gamer::command with player>=2
  when no_open_tiles.

### Lead spot-checks (W3)

- F23 ADJUSTED upheld, severity minor -> nit — read RULES.md ("Captures
  resolve automatically": "The Cathedral counts as a piece identity for
  this limit exactly like an opponent piece - a region containing the
  Cathedral plus one opponent piece has two distinct identities and is
  NOT captured"; capture flips cathedral tile ownership) and
  lib.rs:262-306. The implemented walk matches the crate's own rules doc
  exactly, so this is documented intended behavior, not an undocumented
  preserved defect. The residual claim (official rules let the cathedral
  act as an enclosure wall) rests solely on the original reviewer's
  external rules knowledge. Downgraded to nit (add a code comment
  cross-referencing RULES.md at the walk site).

### W4 dispatched — sushizock-2 (F29-F34)

### W4 returned

All 6 CONFIRMED at stated severities. Dump: raw/games-batch-c-sushizock.md.
- F29 refined: no Cargo profile sets overflow-checks, so default dev/test
  profiles panic; release wraps and is caught by the index<0 guard.
- F31: suggest.rs:109 destructures `Spec::Many { spec, delim, .. }`,
  discarding min/max — parse honors max, suggest ignores it.

### Lead spot-checks (W4)

Since W4 confirmed everything, Lead directly re-verified its hardest
confirmation:
- F29 upheld — read command.rs:54-75 (steal_parser's tile-index component
  is `Int::any()`) and lib.rs:433-473 with lib.rs:338-348.
  can_steal_blue_n checks only turn/target-has-tiles/chopsticks>=4 and
  never touches n; the empty-stack guard passes for len>=1; then
  `let index = len as i32 - n;` (lib.rs:460) computes len - i32::MIN,
  which overflows i32. Identical at lib.rs:502. Major stands.

## Curation complete (2026-07-24)

33/34 CONFIRMED, 1 ADJUSTED (F23 cathedral flood-fill: code facts upheld
but behavior is documented as intended in the crate's own RULES.md;
severity minor -> nit), 0 REJECTED, 0 UNVERIFIABLE. Corrected unit tally:
0 critical / 4 major / 14 minor / 16 nit (was 0/4/15/15). Report:
verification/games-batch-c.md. LOG closed.
