# cathedral-2 stray edits - analysis notes (pre-existing material for WP-21)

Scope: findings F22, F25, F26, F28 in `findings/games-batch-c.md`
(work package **WP-21 cathedral-2 + sushizock-2**).

## Provenance

A stray subagent modified three cathedral-2 files during the read-only review
phase. The edits were captured to `raw/cathedral-stray-edits.diff`
(2562 bytes, 3 hunks) and then **REVERTED** from the working tree.

The diff is **reference material for the fix phase, not applied code**. The
working tree contains none of it. Each edit below was independently assessed
against the live source; adopt/reject per the verdicts.

---

## Edit A - `rust/game/cathedral-2/src/command.rs`

Finding **F22** "Per-request memory leak in `loc_name` (`Box::leak` per parser
construction)" (major, command.rs:26).

**Change:** removes the `LocChoice` struct + its `Display` impl + the
`Box::leak`-based `loc_name()`; `loc_parser()` becomes
`Enum::partial(loc::all_locs())`.

**Verdict: CORRECT + COMPLETE. Recommend adopting as-is.**

Evidence:

- `Loc` implements `Display` at `loc.rs:118-122`, forwarding verbatim to
  `to_key()` - which is exactly what `LocChoice.name` held. So the accepted
  command grammar and the emitted command spec are byte-identical.
- `Enum` requires only `T: ToString + Clone`
  (`rust/lib/game/src/command/parser/mod.rs:551-576`); `Loc` is
  `Copy + Display`, so `Vec<Loc>` satisfies `partial`.
- `Enum::parse` / `expected()` / `to_spec()` all go through `v.to_string()`
  (mod.rs:614, 665-673, 675-681), so dropping the `Map` wrapper is
  transparent.
- No compile risk: `command.rs:4` is a glob import, and `Map` is still used at
  command.rs:41, :73, :90. There is no `deny(warnings)` and no `[lints]` table
  in `rust/Cargo.toml` or the crate manifest.

**Bonus:** this also resolves **F28** "Dead code: `impl Display for Loc` is
never used" (loc.rs:118) - it is F28's second recommended option. Close **F28
as resolved-by-A** rather than fixing it separately.

---

## Edit B - `rust/game/cathedral-2/src/lib.rs`

Finding **F26** "`Loc::to_key` arithmetic overflow on out-of-range
coordinates" (nit, loc.rs:114).

**Change:** `Game::tile_at()` gains an early
`if !loc.valid() { return empty_tile(); }` guard.

**Verdict: CORRECT + COMPLETE. Adopt, but do NOT repeat the finding's
"mirrors Go" justification.**

Evidence:

- It is literally F26's recommendation, and it mirrors the existing guard in
  `render.rs:37-49` (`Tiler for HashMap<String, Tile>`), unifying both
  `tile_at` contracts.
- No caller is masked today - every live path validates upstream:
  - `can_play_piece` checks `!l.valid()` at lib.rs:146 before `tile_at` at
    :149;
  - `check_captures` (:255) receives locs already validated by
    `can_play_piece` (every piece's `positions[0]` is `(0,0)`,
    piece.rs:60-101);
  - the walk calls (:270, :276, :286, :291, :307) get locs from `loc::walk`,
    which only enqueues `next_loc.valid()` neighbours (loc.rs:210-215);
  - `loc_filter_matches` (:96) is only driven from `all_locs()` (:359).
- So the guard removes a latent overflow / garbage-key hazard without changing
  legality behaviour.

**Caveat to record.** F26's rationale "mirroring Go's missing-map-key
behaviour" is **factually WRONG**. Go's `Tile` zero value is
`{Player:0, Owner:0}` (`brdgme-go/cathedral_1/tile.go`), whereas
`empty_tile()` is `{-1, -1}`. Go off-board reads yield "player 0" tiles, not
empty ones (moot in Go because game.go:85 pre-fills every `AllLocs` key). The
Rust guard is the **permissive** direction: off-board reads as empty +
unowned, so a future caller that forgets `valid()` would see a *placeable*
square and be stopped only by the separate `!l.valid()` check at lib.rs:146.
That check is the real safety net and **must stay**. Do not carry the
"mirrors Go" claim into the fix commit message.

---

## Edit C - `rust/game/cathedral-2/src/piece.rs`

Finding **F25** "`pieces()` panics on out-of-range player index
(request-adjacent)" (minor, piece.rs:110).

**Change:** `pieces(player)` fallback `panic!("invalid player: {}", player)`
-> `vec![]`.

**Verdict: PARTIAL. It closes the reachable panic and satisfies the letter of
F25, but leaves the invariant unenforced. Prefer boundary validation in the
spec.**

Evidence the panic is real: verification
(`findings/verification/games-batch-c.md:51`) - "Panic reachable from
`Gamer::command` with player>=2 ... harness (requester/gamer.rs:130) forwards
player unvalidated". F25's own recommendation sanctions the empty-vec form:
"return an empty `Vec` (or make it `Option`/`Result`) for out-of-range
players".

Downstream trace with `player >= 2` and the empty vec - no secondary panic,
and the request degrades to a clean user error:

- `can_play_something` (lib.rs:358-382): empty list -> `for i in (0..0).rev()`
  never runs -> never indexes `played_pieces[player]` -> `false`.
- `can_play` (lib.rs:106-112): `false` on both branches.
- `command_parser` (command.rs:31-37) -> `None` -> `Gamer::command` returns
  `GameError::invalid_input("not expecting any commands at the moment")`
  (lib.rs:470-477); `command_spec` -> `None` (lib.rs:502-504); harness emits a
  clean `Response::UserError` (`rust/lib/cmd/src/requester/gamer.rs:130-135`).
- `piece_parser`'s otherwise-degenerate `Int::bounded(1, 0)`
  (command.rs:71-74) is unreachable because `can_play` gates it.
- `remaining_piece_size` (lib.rs:346-355) returns `0`; loop body never indexes
  `played_pieces[player]`.
- `render_player_remaining_tiles` (render.rs:358-390) via `player_render`
  (render.rs:432): `has_tiles == false` -> renders "None"
  (render.rs:381-386), never indexes `played_pieces[p_num]`; `opponent(2) == 1`
  (lib.rs:71-73) so the opponent panel still renders. **This path previously
  panicked.**

Where it falls short: `pieces()` becomes total-but-lying - it cannot
distinguish "player 2 does not exist" from "this player has no pieces left".
That silently reshapes `remaining_piece_size(2) == 0` (a scoring function) and
renders a bogus player panel headed "None" instead of erroring.

**Ideal fix:** validate `player >= self.players` at the boundary
(`Gamer::command` / `command_spec` / `player_state`, or in the harness at
`rust/lib/cmd/src/requester/gamer.rs`) and make `pieces()` return
`Option<&[Piece]>` / `Result` so the impossible case is typed rather than
flattened.

**Not addressed by the edit** (F25 also names these, and rated them
acceptable): the `ortho_dir_name` panic (`loc.rs:41`) - still
parser-constrained to `ORTHO_DIRS`, reached from `play`'s log at lib.rs:195 -
and `wall_char` (`render.rs:85`).
