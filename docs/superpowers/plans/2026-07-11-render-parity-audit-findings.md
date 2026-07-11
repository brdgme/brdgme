# Render Parity Audit Findings (2026-07-11)

Audit of all Rust game ports with in-repo Go counterparts, using the
procedure in `docs/porting/RENDER_PARITY.md` (Rust `_cli` binary vs Go
`<game>_1/cmd` binary, both rendered to plain text via
`rust/tools/render_plain`). One subagent per game; plan at
`2026-07-11-render-parity-audit.md`.

## Verdicts

| Game | Verdict | Issues |
|---|---|---|
| category-5-2 | ISSUES | glued table columns (all 3 tables); missing space after "Legend:"; end-round log loses per-number bold (ANSI only) |
| battleship-2 | ISSUES | board grid glued to row/column labels (Go bakes 1-space margins); error body casing lowercased vs Go |
| farkle-2 | ISSUES | glued columns (both tables) |
| for-sale-2 | ISSUES | missing Oxford "and" in remaining-players list; 2 missing blank lines (buying + selling phases); finish-log table has extra header row + glued columns; finished-state renders diverge structurally (Rust shows persistent score table and hides player info; Go shows empty pub render and keeps player info) |
| greed-2 | ISSUES | glued columns (both tables); parser error body differs ("expected D or G" vs "expected D or done") |
| liars-dice-2 | ISSUES | glued columns (players table + dice-reveal log table) |
| no-thanks-2 | ISSUES | glued columns (2- and 3-column tables). Bonus: Go itself panics rendering a finished game (PeekTopCard on empty deck) - pre-existing Go bug, finished-state comparison impossible |
| sushi-go-2 | ISSUES | score headers drop the word "points"; glued columns (hand table, colSpacing=2; played-cards table, colSpacing=3); some bold-scope diffs (ANSI only) |
| sushizock-2 | ISSUES | glued columns (players table); tiles rows not rendered as a table (no cross-row column alignment); missing blank line before players table; extra "Your tiles:" section absent from Go; finished-game pub render is a different structure ("The game is finished!" + score table vs Go's normal layout); finished log table has extra header row + glued columns |
| zombie-dice-2 | ISSUES | glued columns (status table + scores table, Go colSpacing=2) |

10/10 games audited have the glued-column defect. Log wording, command
specs, and section ordering were otherwise verbatim-clean in almost every
game (exceptions listed above).

## Root causes, by layer

1. **Missing column spacer cells (every game).** Go's
   `render.Table(cells, rowSpacing, colSpacing)` inserts literal blank
   spacer cells between columns; Rust `Node::Table` has no spacing
   parameter and no equivalent helper, so every port silently dropped the
   gaps. Fix options:
   - (a) add a shared Rust table helper (or a spacing parameter /
     `table_with_spacing` in `brdgme_markup` or `brdgme_game`) mirroring
     Go's, then use it in all 10 games - fixes the class;
   - (b) insert spacer cells manually per game - 10 similar small edits.
   Option (a) recommended: the 10/10 hit rate shows the API shape itself
   is the trap. Now documented in `GAME_PORTING.md` step 6 either way.
2. **Shared error prefix.** Go CLI wraps command errors as
   `"Command failed, <msg>"` (`brdgme-go/cmd/cli.go:268`); Rust uses
   `"invalid input, <msg>"` (`rust/lib/game/src/errors.rs:13`).
   Cross-cutting, one-line decision: which wording is canonical?
3. **Unapproved behavioural deviations in ports** (violates the porting
   correctness rule - should have been raised during the port):
   - for-sale-2 and sushizock-2 finished-state renders restructured;
   - sushizock-2 added a "Your tiles:" player section;
   - for-sale-2 and sushizock-2 finish-log tables gained header rows;
   - sushi-go-2 dropped the "points" literal;
   - category-5-2 dropped the space after "Legend:";
   - battleship-2 lowercased error message bodies.
   Each needs a keep-or-revert decision (default per the rule: revert to
   Go behaviour).
4. **ANSI-only bold-scope differences** (category-5-2, sushi-go-2,
   sushizock-2, for-sale-2 double-bold nit): invisible in plain text;
   only matters if ANSI/rich parity is ever audited.
5. **Pre-existing Go bug found in passing:** `no_thanks_1` panics when
   rendering any finished game (`PeekTopCard` on empty deck,
   `brdgme-go/no_thanks_1/no_thanks.go:276`); its Score column is dead
   code in practice. The Rust port handles this state correctly.

## Old-project audits (second round, via old-repo harness)

Harness: `~/Development/brdg.me/cmd/renderparity` (+ minimal `go.mod`),
uncommitted in the old repo. Prints `render.RenderPlain(RenderForPlayer(p))`
per player with new/play/render subcommands and a persistent state file.

| Game | Verdict | Notes |
|---|---|---|
| tic-tac-toe-2 | CLEAN | byte-identical through a full deterministic game |
| jaipur-2 | ISSUES (9) | fixed this session - see below |
| acquire-1 | ISSUES (8) | EXEMPT per user decision - see below |
| lost-cities-1 | ISSUES (8) | EXEMPT per user decision - see below |

**User decision (2026-07-11): acquire-1 and lost-cities-1 (and
lost-cities-2, which derives from lost-cities-1) are exempt from render
parity vs Go.** They are the original Rust implementations, in production
use in legacy brdg.me for years and battle-hardened; their differences
from the old Go renders are established behaviour, not regressions. No
fixes to be applied. (For the record, the acquire audit flagged one
non-render item: un-founded corp values are non-zero in Rust vs Go's
explicit $0, which also lets the buy phase trigger after an isolated tile
play where Go skips it - covered by the same exemption since the Rust
behaviour is the long-deployed one.)

jaipur-2 issues (all unapproved - the port plan itself contained them):
missing outer centered layout, two missing newlines (rounds-remaining
and bonuses headings glued to next line), missing Rare/Common header
row, sale-price table missing real column gaps, bonus line 2-space vs
Go 4-space gaps, deck-count line rendered mid-output instead of last,
hardcoded plurals ("1 point tokens"), malformed comma lists in draw
logs ("Drew gold,  , leather,  and camel").

## Ports without an in-repo Go counterpart (feasibility check)

- acquire-1, jaipur-2, lost-cities-1, tic-tac-toe-2: Go sources exist in
  `~/Development/brdg.me/game/<name>`; the game+render dependency chain is
  stdlib-only, so a small per-game harness (~30-100 lines, calling
  `RenderForPlayer` + the old repo's `render.RenderPlain`) is cheap.
  Caveats: old markup dialect is incompatible with `render_plain` (use
  the old repo's own plain renderer), no public/spectator render exists
  in the old interface, and old `render.PlayerName` truncates/prefixes
  names (use short names without "@").
- lost-cities-2: was ported from lost-cities-1's Rust, not Go - its
  parity baseline is lost-cities-1's renders.
- lords-of-vegas-1: no Go source anywhere; render parity vs Go is
  impossible (manual review vs RULES.md only).
- Decision pending: whether to build the old-project harness.

## Deliverables created this session (uncommitted)

- `rust/tools/render_plain/` (workspace crate `brdgme_render_plain`)
- `scripts/render-compare/render.sh`
- `docs/porting/RENDER_PARITY.md`
- `docs/porting/GAME_PORTING.md` - render-parity verification now REQUIRED
  in step 6 + deployment checklist item 9 (spacer-cell trap documented)
- `rust/Cargo.toml` / `Cargo.lock` (new workspace member)
- Prebuilt audit binaries in `/tmp/render-parity/` and
  `rust/target/debug/` (ephemeral)

## User decisions (2026-07-11)

1. Build a shared table-spacing helper in `brdgme_markup`; the caller
   provides the spacer as markup nodes (e.g. `N::text(" ")`, or
   `N::text("|")` with color). If node-spacers feel cumbersome, add a
   thin convenience wrapper for the common plain-gap case.
2. Add the node-based comma-list helpers (approved).
3. Revert behavioural deviations UNLESS they fixed provable bugs. Some
   ports asked for and received bug-fix approval during porting - check
   plan docs / history for evidence; must be certain it is a real bug.
4. Error prefix: keep the current Rust library wording ("invalid
   input,"). No change.
5. Old-project harness: build it if trivial and high value (feasibility
   says yes) and audit acquire-1, jaipur-2, lost-cities-1,
   tic-tac-toe-2.
6. Commit the docs; commit the render-compare harness too if clean,
   well implemented, and documented.

## Resolution (2026-07-11, same session)

All decided follow-ups were executed:

- `brdgme_markup` gained `table_with_spacer(rows, spacer)`,
  `table_with_gap(rows, gap)`, `comma_list_and(items)`,
  `comma_list_or(items)` (node-based), with unit tests.
- All 11 affected games fixed and re-verified against their Go binaries
  (side-by-side plain renders, package tests, fmt, clippy):
  category-5-2 (gaps in 3 tables, Legend space, end-round bold scope,
  plus Taken/Pts center alignment found during verify), farkle-2,
  greed-2, liars-dice-2, zombie-dice-2, no-thanks-2 (gaps),
  sushi-go-2 ("points" literal, 2 tables, bold scopes),
  battleship-2 (board margins byte-matched via full playthrough),
  for-sale-2 (all 5 deviations reverted, final_scores field removed),
  sushizock-2 (all 6 deviations reverted),
  jaipur-2 (all 9 defects; Rare/Common header is a documented
  approximation because Go's own CellSpan centering is buggy).
- Deviation evidence audit: no deviation had documented approval; all
  reverted. Two deliberate keeps: battleship-2's "ship" wording (fixes a
  provable Go typo "shift") and its lowercase error bodies (coherent
  with the kept Rust "invalid input," prefix).
- tic-tac-toe-2 audited CLEAN; acquire-1 / lost-cities-1/2 exempted.

Remaining open items (not blocking):

1. Parser error-offset propagation: Rust `chain_2`
   (rust/lib/game/src/command/parser/chain.rs) does not add consumed
   length to inner error offsets like Go does, so OneOf merges
   sibling expectations ("expected D or done" vs Go "expected D or G").
   Library-wide, cosmetic; fix in brdgme_game if desired.
2. Backlog: no_thanks_1 finished-render panic (Go legacy).
3. for-sale finish-log scores show cheque totals only while placings
   use cheques+chips - faithful port of a Go quirk, noted for awareness.
4. Old-repo harness (`~/Development/brdg.me/cmd/renderparity` + minimal
   go.mod) left uncommitted in the legacy repo.
