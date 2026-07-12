# Porting notes: love_letter_1 (Go) -> love-letter-2 (Rust)

## Preserved source quirks (not fixed, per the porting correctness rule)

1. **`AssertTarget` double `Eliminated` check.** In Go (`game.go`), after the
   `Eliminated[target]` check there is a second, identical
   `if g.Eliminated[target] { return errors.New("... protected by the
   Handmaid") }` - clearly meant to be `Protected[target]`. This means
   targeting a protected player is **not** rejected by the action method
   itself; protected players are only excluded from `AvailableTargets`,
   which is what forces the "must target yourself" fallback when no
   unprotected targets remain. Ported verbatim in `Game::assert_target` in
   `src/lib.rs`, with a comment pointing at the bug.

2. **Priest parser doc typo.** `command.go`'s `PriestParser` has
   `Desc: "play the Baron card to peek at another player's hand"` - it says
   Baron, means Priest. Preserved verbatim in `priest_parser()` in
   `src/command.rs`.

3. **`EndRound` winner-selection logic.** Ported the `highestCard`/
   `discardTotal` tiebreak and default-to-player-0 behaviour exactly
   (`Game::end_round` in `src/lib.rs`): iterate players in index order,
   track the highest card value seen so far and (only among players tying
   that highest value) the most total discarded; `highest_player` starts at
   `0` and is only updated when a strictly-higher discard total is found for
   the current highest card, matching Go's uninitialized-int-defaults-to-0
   behaviour.

## Other decisions

- **`PubState`/`PlayerState` are hand-built, not literal ports of Go's
  `PubState()`/`PlayerState()`.** Those Go methods just `return nil` - the
  real spec is what `PlayerRender`/`PubRender` read from `Game` directly.
  `PubState` therefore carries: player count, deck-remaining count,
  per-player discard piles (public - discards are visible on the table),
  points, current player, eliminated/protected flags, the win threshold, and
  the leader's point total (derived, but included since the render needs
  it and it doesn't leak anything - it's the max of already-public points).
  Hands, deck contents and the removed card are simply absent fields, so
  serialization can't leak them - verified by
  `test::pub_state_does_not_leak_hidden_info`.
- **Log wording**: every log message is a line-for-line port of the Go
  `fmt.Sprintf` templates, using `Node`/`N::Player`/`N::Bold` markup instead
  of the `{{b}}...{{/b}}` string markup and `render.Player(n)`.
- **`EndRound` multi-line public log**: Go joins per-player result lines with
  `strings.Join(output, "\n")` into a single log message. Ported as a single
  `Log::public` whose node list embeds `N::text("\n")` between sections,
  preserving the single-log, multi-line structure.
- **Command availability**: `command_parser` only returns commands for the
  current player, matching Go's `CanPlay(player) == (CurrentPlayer ==
  player)`; unlike Go there's no separate `CanPlay` check duplicated in each
  action method beyond `assert_can_play`, which every `play_*` method calls
  first (defence in depth, per the porting guide's "Command availability is
  checked twice" gotcha).
- **Countess-forced rule**: `assert_must_not_play_countess` is called from
  `play_king`/`play_prince` exactly as Go's inlined
  `brdgme.IntFind(Countess, g.Hands[player])` checks in `PlayKing`/
  `PlayPrince`; parser availability is unaffected (Countess and King/Prince
  parsers can all appear simultaneously in the `OneOf`, same as Go).
- **Draw semantics**: `draw_card` draws from `removed[0]` without removing it
  from `removed` when the deck is empty, matching Go's
  `card = g.Removed[0]` (no removal). This can only trigger the game's final
  draw of a round in practice (deck empties mid-round need one more draw to
  fill the deck-out slot), same as the Go implementation.
- **Guard's card enum ordering** (`princess_to_guard()` in `src/card.rs`)
  matches Go's `CardParserValues` iteration (`for c := Princess; c >=
  Guard; c--`) - Princess first, Guard last. This only affects `command_spec`
  presentation order, not parsing correctness (the parser is `Enum::exact`,
  which matches on name regardless of list order).
- **Render layout**: ported the rendered output (`PlayerRender`/`PubRender`
  in `render.go`), not the (empty) Go state structs, per the porting guide.
  `render.Table(cells, 0, 2)`'s `colSpacing = 2` is reproduced with
  `brdgme_markup::table_with_gap(&rows, 2)`, which inserts two-space spacer
  cells between adjacent columns - equivalent to Go's literal spacer cells.
- **Colors**: Princess=Yellow, Countess=Red, King=Blue, Prince=Purple,
  Handmaid=Black, Baron=Green, Priest=Cyan, Guard=Grey, taken directly from
  `brdgme-go/render/color.go`'s named constants via the equivalent
  `brdgme_color` constants.
- **Tie-break in `gen_placings`**: `love_letter_1`'s `Placings()`/`Winners()`
  only ever has a single metric (`PlayerPoints`), so the Go
  compact-ordinal-vs-Rust standard-competition placings divergence noted in
  the porting guide doesn't come up in this game's tests, but the same
  `gen_placings` semantics apply if ties occur in play.

## Render parity fix

- `render.rs`'s top-level `render` function originally emitted every row of
  the outer layout with `A::Left`. Go's `PlayerRender`/`PubRender` return
  their rows through `render.Layout(rows)`, which (`render.go`'s `Layout`)
  wraps each row in a single-column table cell with `Align: Center` - i.e.
  every top-level block (leader line, "Your card(s)" header, hand, player
  table, "Cards remaining" line, help table) is centered on the page, not
  left-aligned. Fixed by changing the outer `rows.push(...)` alignments in
  `render()` from `A::Left` to `A::Center` to match. The nested
  `player_table`/`help_table` column alignments were already correct (they
  mirror Go's explicit per-column `render.Center`/left default) and were
  left unchanged. Verified via the render-parity procedure in
  `docs/porting/RENDER_PARITY.md`.

## `Status::Active.eliminated` intentionally empty

`PlayResult::status`'s `Status::Active { whose_turn, eliminated }` always
sets `eliminated: []` for this game. This matches Go: `love_letter_1`'s
`Status()` method only ever sets `WhoseTurn` and never populates a
per-round-eliminated list at that layer - round elimination is exposed
through `PubState`/`PlayerState` (the `eliminated: Vec<bool>` field) and the
rendered player table's "eliminated" status column instead. Left empty here
to match the Go implementation's behaviour, not an oversight.

## Tests not ported literally

All Go test cases (`TestGame_IsFinished`, all four `TestCharBaron_Play_*`,
`TestCharPrince_Play_end`) are ported 1:1 with snake_cased names. Additional
tests were added per the task brief and the porting guide's "thin suite"
guidance: happy-path tests for all eight card commands, the Guard-vs-Guard
rejection, both must-play-Countess errors, the Handmaid-protection
self-target fallback, an explicit eliminated-target rejection case, the
per-player-count end-score finish check, and a `PubState` hidden-info leak
test (structural - inspects the serialized JSON object's keys rather than
asserting on rendered output, per `GAME_DEVELOPMENT.md`'s test guidance).
