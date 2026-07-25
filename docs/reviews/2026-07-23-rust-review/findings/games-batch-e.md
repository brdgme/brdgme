# Findings: games-batch-e

Crates: `love-letter-2` (1,699 LOC), `age-of-war-2` (1,656), `lost-cities-2`
(1,494), `red7-1` (1,404), `lost-cities-1` (1,342). Reviewed against the
snapshot worktree (HEAD f8763a5). Raw worker findings (with per-crate
verified-clean notes) are in `findings/raw/games-batch-e-*.md`; Lead
spot-checked the headline findings against snapshot line numbers.

Cross-cutting notes:

- Go originals exist only for love-letter (`brdgme-go/love_letter_1`) and
  age-of-war (`brdgme-go/age_of_war_1`); both ports were verified line-by-line
  against them with no rules divergence. lost-cities-1/-2 and red7-1 have no
  Go original, so their rules correctness was judged against the official
  rulebooks (noted per finding).
- lost-cities-1 vs lost-cities-2 relationship: **lost-cities-1 is the legacy,
  superseded version; the duplication is deliberate.** lost-cities-1's
  GameVersion manifest carries `isDeprecated: true`
  (`k8s/base/game/lost-cities-1/game-version.yaml:10`); both stay deployed per
  the documented deprecation lifecycle in `docs/porting/GAME_PORTING.md` so
  in-flight games can finish. lost-cities-2 is a faithful generalization
  adding a RULES.md-documented (non-official) 3-player variant; all 2-player
  logic, scoring, and tests are identical where they overlap. The shared
  defects below were inherited from -1, except the stats hardcode, which
  became a defect only through -2's 3-player support. One doc wrinkle:
  GAME_PORTING.md claims -2 was ported from in-repo Go code, but no
  lost-cities Go exists in `brdgme-go/`.
- Boilerplate-binary systemic issues (identical across ~27 game crates) are
  captured once at the end of this file; all five crates' binaries were
  checked for deviations and none were found.

## love-letter-2

Rules logic verified line-by-line against `brdgme-go/love_letter_1` — no
divergence (card effects, burn-card counts, end scores, tiebreaks all match).
`card.rs` and `render.rs` are clean; serde views have no information leaks
(guarded by the `pub_state_does_not_leak_hidden_info` test). Documented
preserved Go quirks in PORTING_NOTES.md are cross-referenced, not flagged.

### `command()` match arms duplicate the same ~20-line wrap-up 8 times
- severity: major
- category: simplicity
- location: game/love-letter-2/src/lib.rs:698
- finding: Each of the 8 `Ok(ParseOutput { ... })` arms in `Gamer::command`
  (lib.rs:713-857) repeats the identical block: call `play_*`, then
  `if self.is_finished()` build `scores` and push `placings_log`, then
  construct the `CommandResponse`. ~140 lines of copy-paste where only the
  `play_*` call differs; any change to finish/scoring behaviour must be
  edited in 8 places and can drift silently.
- recommendation: Collapse to a single `Ok(ParseOutput { value, remaining, .. })`
  arm that `match`es `value` to get `logs`, then runs the shared
  finish/response wrap-up once.

### `end_score` uses `unreachable!()` in a runtime path
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:29
- finding: `end_score()` ends in `_ => unreachable!()` and is called from
  `check_finished()` (lib.rs:273), `pub_state()` (lib.rs:685) and indirectly
  `status()`. `Game::default()` has `players = 0`, so `pub_state()`/`status()`
  on a default or corruptly-hydrated `Game` panics and kills the request. Not
  reachable through normal play (player count validated at lib.rs:645), but
  violates CODING.md's no-panicking-code-in-runtime-paths rule.
- recommendation: Return a safe fallback for out-of-range counts (e.g.
  `usize::MAX` so the game never reports finished), or make the invariant
  unrepresentable. At minimum `debug_assert!` + fallback.

### `end_round` indexes `hands[p][0]` without checking emptiness
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:184
- finding: `let c = self.hands[p][0];` panics if any non-eliminated player has
  an empty hand at round end. The invariant holds for all current play paths
  (verified against Go), so this is latent, but it is an unchecked indexing
  path in a runtime request handler.
- recommendation: Use `.first()` and skip on `None`, or document the invariant
  with a comment.

### `assert_target` and play methods index state with unvalidated `target`
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:305
- finding: `assert_target()` indexes `self.eliminated[target]` (lines 305,
  311) with no `target < self.players` check, and the play methods then index
  `self.hands[target][0]` (play_king lib.rs:405-407, play_prince lib.rs:444,
  play_baron lib.rs:500-501). `target` comes from the parser's `Player {}`
  indices over the `players: &[String]` slice passed into `command()` —
  nothing clamps it to the game's actual player count. If a caller ever passes
  a names slice longer than `self.players`, a crafted command naming the extra
  "player" panics the service. Go has the identical defect (parity), but the
  Rust-side fix is one cheap bounds check.
- recommendation: Add `if target >= self.players { return Err(GameError::invalid_input("invalid target player")) }`
  at the top of `assert_target()`.

### Commands are still accepted after the game is finished
- severity: minor
- category: correctness
- location: game/love-letter-2/src/command.rs:23
- finding: `command_parser()` only checks `self.current_player != player`;
  there is no finished-game guard. After the game ends the current player
  still holds a card, so the parser offers commands and `command()` executes
  plays (discards, eliminations, `end_round` re-runs awarding more points) on
  a finished game. Go has the same shape and the web layer presumably blocks
  finished games, but the crate doesn't enforce it — defence in depth is
  otherwise applied (`assert_can_play` per PORTING_NOTES).
- recommendation: Return `None` from `command_parser()` (or an error from
  `command()`) when `self.check_finished()`.

### Redundant no-op hand assignments in `play_baron`
- severity: nit
- category: quality
- location: game/love-letter-2/src/lib.rs:529
- finding: `self.hands[player] = vec![player_card];` (and the mirror at line
  532) are no-ops: `discard_card(player, Card::Baron)` at line 479 already
  left exactly `[player_card]`. A verbatim port of the same dead assignments
  in Go `PlayBaron`, but unlike the other preserved quirks it is not
  documented in PORTING_NOTES.
- recommendation: Drop the two assignments, or keep them and add a
  PORTING_NOTES entry.

### Guard self-target fallback silently ignores an invalid Guard guess
- severity: nit
- category: consistency
- location: game/love-letter-2/src/lib.rs:595
- finding: When all other players are protected/eliminated and the player
  targets themselves, the `target == player` early-return (lines 595-605)
  runs before the `card == Card::Guard` validation (line 607), so
  `guard mick guard` succeeds as a plain discard instead of returning "you
  can't use Guard against other Guards". Verbatim port of Go `PlayGuard`
  ordering, but not documented in PORTING_NOTES.
- recommendation: No behaviour change; add a one-line PORTING_NOTES entry
  recording the preserved ordering.

### `discard_card` records discards for cards the player does not hold
- severity: nit
- category: correctness
- location: game/love-letter-2/src/lib.rs:144
- finding: `discard_card()` removes the card "if present" but unconditionally
  pushes it to `discards[player]`, so a `play_*` call for a card not in hand
  corrupts the public discard record. Guarded by the parser today; a fragile
  implicit contract for the `pub play_*` API. Matches Go `DiscardCard`.
- recommendation: Return an error (or no-op) when the card is absent, or make
  the play_* methods crate-private and document the parser-gating contract.

### Test module named `test` instead of conventional `tests`
- severity: nit
- category: consistency
- location: game/love-letter-2/src/lib.rs:916
- finding: `mod test` — Rust convention and the rest of the workspace use
  `mod tests`.
- recommendation: Rename to `mod tests`.

## age-of-war-2

A very faithful port: all 14 castles, clan groupings, point values, battle
lines, die faces, turn logic, scoring, and placings verified line-by-line
against `brdgme-go/age_of_war_1` — no rules divergence. Command-parser
validation is solid (attackable-castle enum, exact line enum, re-validation
in `line_action`); the game is fully public so serde views leak nothing.
Documented preserved Go quirks (post-finish commands accepted, stale clan
player value, standard-competition placings) are test-covered and not
flagged. Binaries/Cargo.toml are byte-identical to the boilerplate.

### Panicking unwrap/expect cluster in game-service runtime paths
- severity: minor
- category: consistency
- location: game/age-of-war-2/src/lib.rs:132
- finding: Six `.unwrap()`/`.expect()` sites sit on paths executed by the Play
  endpoint: `scores()` unwraps `ALL_CLANS.iter().position(...)` (lib.rs:132);
  `check_end_of_turn` expects a conquered castle to have an owner
  (lib.rs:219); `line_action` (lib.rs:334) and `line_parser` (command.rs:89)
  expect `currently_attacking`; `render_castle` (render.rs:48) and
  `render_castles` (render.rs:110) expect owners for conquered castles/clans.
  Each is guarded by an invariant the command flow maintains (verified), so
  none is reachable from crafted player input — but CODING.md's no-panic rule
  makes no invariant exception, and a panic kills the game-service request.
- recommendation: Convert to error propagation where a `Result` is available
  (`line_action`: `ok_or_else(|| GameError::internal(...))?`); for
  render/state-invariant sites use a non-panicking fallback or restructure so
  the invariant lives in the type (e.g. store conquered castles as
  `(owner, ...)` pairs instead of parallel vectors). The `scores()`
  position-unwrap is provably total and can become `if let` + `continue`.

### `completed_lines: HashSet<usize>` in persisted state serializes nondeterministically
- severity: minor
- category: quality
- location: game/age-of-war-2/src/lib.rs:35
- finding: `Game` is the serde-persisted state blob, and `completed_lines` is
  a `HashSet<usize>`, which serializes in per-process randomized iteration
  order — two logically identical states can persist to different JSON bytes,
  breaking any diff/hash/dedup of serialized states. The pub view already
  works around this by sorting (lib.rs:434-435), evidence the nondeterminism
  was noticed but only fixed on the output side.
- recommendation: Use `BTreeSet<usize>` (same API surface used here) or a
  sorted `Vec<usize>`.

### "Not your turn" returned as unstructured `invalid_input` instead of `GameError::NotYourTurn`
- severity: nit
- category: consistency
- location: game/age-of-war-2/src/lib.rs:461
- finding: `command()` maps a missing parser (which happens exactly when it is
  not the player's turn) to `GameError::invalid_input("not your turn")`. The
  `GameError` type has a dedicated `NotYourTurn` variant, so callers matching
  on the structured variant misclassify this rejection.
- recommendation: Return `GameError::NotYourTurn` (or call
  `self.assert_player_turn(player)?` before parsing).

### Placings-log tail triplicated across all three command arms
- severity: nit
- category: simplicity
- location: game/age-of-war-2/src/lib.rs:473
- finding: The identical 10-line block (build scores vec, push `placings_log`
  when `is_finished()`) is copy-pasted into the Attack, Line, and Roll arms
  (lib.rs:473-482, 491-500, 509-519).
- recommendation: Extract a helper that runs the command then appends the
  placings log once.

### Finished games keep emitting duplicate placings logs (side effect of preserved Go quirk)
- severity: nit
- category: correctness
- location: game/age-of-war-2/src/lib.rs:473
- finding: Because `can_roll` deliberately does not check finished status
  (preserved, test-covered Go quirk), the current player can keep issuing
  `roll` after the game ends; each accepted command appends another
  `placings_log` and advances `current_player` in a finished game. Cosmetic
  log spam only, and the placings-log amplification is new relative to Go
  (Go's Command never appends placings logs).
- recommendation: Gate the placings-log append on the false→true transition
  of `is_finished()` rather than on "game is finished".

### `clan_conquered` logic duplicated between `Game` and the renderer
- severity: nit
- category: quality
- location: game/age-of-war-2/src/lib.rs:93
- finding: The clan-conquest scan (including the subtle preserved Go quirk of
  returning a possibly-stale player on `false`) is implemented twice: once on
  `Game` (lib.rs:93-113), once as a free function over `PubState`
  (render.rs:10-30). The two copies must be kept in lockstep forever, quirk
  included.
- recommendation: Extract one shared helper taking `&[bool]` +
  `&[Option<usize>]` and call it from both sites.

### Player-facing help text: "discard one dice"
- severity: nit
- category: quality
- location: game/age-of-war-2/src/command.rs:117
- finding: The `roll` command description reads "discard one dice and roll
  the rest" — "dice" should be "die". Carried verbatim from Go, but the Rust
  port is the player-facing surface now.
- recommendation: Change to "discard one die and reroll the rest" (check
  suggest/help snapshot tests for the spec text).

## lost-cities-2

Rules verified correct against the official Lost Cities rules (no Go port
exists) for 2-player mode: investments multiply, cost -20, ascending play,
investment-after-number rejected, round ends on deck exhaustion, +20 bonus at
8+ cards counting wagers. The 3-player mode is a documented house variant
(RULES.md:23,61-64), not a finding. `command.rs` is clean; serde views leak
nothing (deck order and opponent hands not exposed).

### Status::Finished stats hardcoded to players 0 and 1 — breaks 3-player games
- severity: major
- category: correctness
- location: game/lost-cities-2/src/lib.rs:534
- finding: `status()` builds `stats: vec![self.player_stats(0), self.player_stats(1)]`,
  but lost-cities-2 supports 3 players (lib.rs:26-27), so a finished 3-player
  game silently omits player 2's stats. The surrounding code was generalized
  (`placings()` at lib.rs:491-497 iterates `0..self.players`) but this line
  was carried over verbatim from lost-cities-1, where the hardcode is correct
  because -1 is 2-player-only. (Lead-verified against the snapshot.)
- recommendation: `stats: (0..self.players).map(|p| self.player_stats(p)).collect()`.

### `player_state()` panics on out-of-range player index (request-reachable)
- severity: major
- category: correctness
- location: game/lost-cities-2/src/lib.rs:570
- finding: `player_state()` does `hand: self.hands[player].clone()` —
  unchecked indexing. `handle_player_render` in `lib/cmd/src/requester/gamer.rs:174`
  passes the `player` field of a `Request::PlayerRender` straight into
  `game.player_state(player)`, so a crafted request with
  `player >= hands.len()` panics the handler. Every other accessor in this
  crate uses `.get()`/`.get_mut()` with `GameError`; `player_state` is the
  exception. Same defect in lost-cities-1 (lib.rs:566). (Lead-verified.)
- recommendation: `self.hands.get(player).cloned().unwrap_or_default()`
  (PlayerState is not fallible), or add a bounds check in
  `handle_player_render` returning a UserError.

### Draw logs silently dropped when the draw empties the deck
- severity: minor
- category: correctness
- location: game/lost-cities-2/src/lib.rs:441
- finding: `draw_hand_full` accumulates the public "P drew a card" and private
  "You drew …" logs into a local `logs`, but when the draw empties the deck
  it returns `self.end_round()` directly (lib.rs:441-445), discarding them.
  The final draw of every round is never logged. Inherited from lost-cities-1;
  looks accidental — `end_round` itself merges `start_round` logs
  (lib.rs:188-191).
- recommendation: `if self.deck.is_empty() { let mut l = logs; l.extend(self.end_round()?); Ok(l) } else { Ok(logs) }`.

### `PlayerState.hand` documented as sorted but serialized unsorted
- severity: minor
- category: consistency
- location: game/lost-cities-2/src/lib.rs:102
- finding: The doc comment on `PlayerState.hand` (lib.rs:102-103) and
  DATA_DOCS.md:19 both state "sorted by expedition then value", but
  `player_state()` returns the hand in acquisition order — only the per-draw
  batch is sorted before being appended (lib.rs:418) and `render_hand` sorts
  a copy for display. API consumers (bots, operator, LLM tooling) reading
  DATA_DOCS assume sorted input that isn't delivered.
- recommendation: Sort the hand in `player_state()` (`hand.sort()` —
  `Card: Ord`), or fix the docs.

### `Stats.investments` never incremented; `Stats.expeditions` write-only
- severity: minor
- category: quality
- location: game/lost-cities-2/src/lib.rs:51
- finding: `Stats.investments` (lib.rs:51-52) is declared and serialized but
  never incremented — dead state in every saved game. `Stats.expeditions` is
  incremented in `play()` (lib.rs:383) but `player_stats()`
  (lib.rs:455-489) never surfaces it. Same in lost-cities-1.
- recommendation: Increment `investments` for `Value::Investment` plays and
  surface both fields in `player_stats()`, or remove the dead fields.

### `unreachable!()` arms panic on any player count outside 2..=3
- severity: minor
- category: correctness
- location: game/lost-cities-2/src/lib.rs:673
- finding: `expedition_cost`, `hand_size`, `expedition_bonus_size`
  (lib.rs:673-695) and `render_tableau` (render.rs:187) `unreachable!()` on
  players ∉ {2,3}. `self.players` is validated at `Game::start`, but `Game`
  is `Deserialize` with no validation on load — a malformed state blob (the
  Play/Status requests carry the full game JSON) with `players: 4` panics
  these paths, as does the public `score(players, …)` (lib.rs:697) for any
  other count.
- recommendation: Return `GameError::internal` from a checked path or
  clamp/default; at minimum document the invariant on `Game.players`.

### Perspective index uses `% MAX_PLAYERS` instead of `% self.players`
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/render.rs:130
- finding: `let p = player.unwrap_or(0) % MAX_PLAYERS;` — the modulus is the
  crate-wide maximum (3), not the actual player count. In a 2-player game an
  index of 2 survives the modulo and mis-renders. Latent today
  (`player_state` panics earlier); the lost-cities-1 equivalent clamped to
  the real count. Semantically wrong and misleading.
- recommendation: `% self.players`.

### Game-over log regressed vs lost-cities-1 (no winner announcement)
- severity: nit
- category: quality
- location: game/lost-cities-2/src/lib.rs:198
- finding: lost-cities-1's `game_over_log` announced the winner and margin
  ("P2 won by 34 points"); lost-cities-2 reduced it to a bare "The game is
  over." (lib.rs:198-200). Looks like accidental drift during the 3-player
  generalization — the `leaders()` helper (lib.rs:120-134) already computes
  what's needed.
- recommendation: Generalize -1's winner log, or confirm the regression is
  accepted.

### Discard piles expose only the top card; official rules have face-up, inspectable piles
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/lib.rs:86
- finding: Judged against official rules (no Go port): physical Lost Cities
  discard piles are face-up and inspectable (only the top card may be taken),
  but `PubState.discards` exposes only the top value per pile
  (lib.rs:551-559), hiding information the physical game makes public
  (card-counting is part of the game). Same in lost-cities-1. A deliberate
  simplification is plausible — worth a conscious decision.
- recommendation: Expose the full ordered discard lists in `PubState`, or
  document the deviation in RULES.md.

### Potential usize underflow in draw-count computation
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/lib.rs:408
- finding: `let mut num = hand_size(self.players) - hand.len();` underflows
  (debug panic; release wrap) if a hand ever exceeds the hand size.
  Unreachable through normal play, but a saturating form is free.
- recommendation: `hand_size(self.players).saturating_sub(hand.len())`.

### Deployed blurb still advertises a strictly two-player game
- severity: minor
- category: consistency
- location: k8s/base/game/lost-cities-2/game-version.yaml:1
- finding: The GameVersion blurb for lost-cities-2 says "A tense two-player
  card game of investment and restraint" (copied verbatim from -1's
  manifest), but -2 advertises `player_counts() = [2, 3]` (lib.rs:644-646)
  and RULES.md documents the 3-player variant. Players browsing the new-game
  page are told it's 2-player-only.
- recommendation: Update the blurb in
  `k8s/base/game/lost-cities-2/game-version.yaml` to mention 2-3 players.

### Stale legacy files in crate root
- severity: nit
- category: quality
- location: game/lost-cities-2/build-release:1
- finding: `build-release` is a pre-workspace lambda/muslrust packaging
  script, obsolete under the current `rust/Dockerfile` + docker-bake
  pipeline; `.rls.toml` configures the long-dead RLS and is malformed
  (`build_lib = truetarget` — missing newline). Same cruft exists in
  acquire-1 and lords-of-vegas-1 — looks copied from an old template.
- recommendation: Delete `build-release` and `.rls.toml`; trim `.gitignore`.

## red7-1

No Go original exists — rules judged against the official Red7 rulebook
(Asmadi). The crate implements Advanced Red7 (canvas-draw rule + round
scoring with 40/35/30 targets). All 7 color-rule evaluations and tie-breaks
(value then color, red highest), turn/elimination flow, setup, first-player
rule, advanced draw rule, scoring, and deck-exhaustion game end were verified
correct except as noted below. Serde views leak nothing (guard test present);
render.rs is clean; binaries/Cargo.toml are pure boilerplate.

### CardParser panics on non-ASCII input via byte-index string slicing
- severity: critical
- category: correctness
- location: game/red7-1/src/command.rs:31
- finding: `CardParser::parse` checks `chars.len() >= 2` (command.rs:23-24)
  but then slices by BYTES: `Card::parse(&input[..2])`, `consumed: &input[..2]`,
  `remaining: &input[2..]` (command.rs:31,34-35). Any input whose second byte
  is not a char boundary panics ("byte index 2 is not a char boundary") —
  reachable from the Play endpoint with e.g. `play r€` or `play €5`. A panic
  in the game service kills the request. This is a crate-LOCAL panic path,
  distinct from the already-captured core-parser non-ASCII panics in
  lib/game. (Lead-verified against the snapshot.)
- recommendation: Slice on char boundaries — take the first two chars with
  `input.chars().take(2).collect::<String>()` and use its byte length for
  `consumed`/`remaining`, or derive the boundary from `char_indices()`.

### Player with zero rule-fulfilling cards is treated as winning (official: cannot win)
- severity: major
- category: correctness
- location: game/red7-1/src/card.rs:297
- finding: Judged against official rules (no Go port): "If you don't have a
  card fulfilling the rule (can happen for green or purple), you cannot win."
  In `leader()` (card.rs:297-316), when ALL palettes have an empty winning
  set (possible under Green = most even cards, Violet = most cards below 4),
  the count comparison ties at 0 and the rank_key maxes tie at `(0, 0)`, so
  the strict `>` at card.rs:311 keeps the FIRST non-eliminated player as
  "leader" with zero qualifying cards. Consequences: (1) that player survives
  `done` (lib.rs:154-164) under a rule where officially they must be
  eliminated; (2) the `discard` pre-check (lib.rs:325-330) lets the
  lowest-index player discard INTO a Green/Violet rule where no one has
  qualifying cards — officially an illegal discard; (3) at round end such a
  "winner" scores 0 points with an empty card list. Example: Green rule,
  palettes p0=[b5], p1=[r7] — crate calls p0 the leader; official rules say
  neither can be winning.
- recommendation: In `leader()`/`leader_with_suit`, treat empty winning sets
  as non-winning: skip players whose rule set is empty, and define explicit
  behaviour when all sets are empty (return `Option`; `end_turn` eliminates
  the current player; the discard pre-check rejects). If the deviation is
  deliberate, document it in RULES.md and DATA_DOCS.md.

### DATA_DOCS.md tie-break description contradicts both the code and official rules
- severity: minor
- category: consistency
- location: game/red7-1/DATA_DOCS.md:36
- finding: "Ties within a rule are broken by the highest card in the winning
  set, then by the highest card overall in the palette." The second clause is
  not implemented anywhere (`leader()` only compares the winning set's max
  rank_key; on fully-empty ties it keeps the first player by index) and does
  not exist in the official rules. Bots consuming DATA_DOCS.md get a wrong
  model of tie-breaking.
- recommendation: Rewrite to describe actual behaviour (after fixing the
  empty-set finding): ties broken by highest card among rule-fulfilling
  cards, value then color.

### RULES.md undersells the turn structure and misdescribes scoring
- severity: minor
- category: consistency
- location: game/red7-1/RULES.md:21
- finding: (1) The Turn section (RULES.md:21-29) lists Play/Discard/Done as
  alternatives and never states the officially-sanctioned combo — play to
  palette AND THEN discard to canvas in the same turn — which the code
  permits (`can_discard`, lib.rs:287-289, ignores `has_played`) and which is
  the strongest move in the game. (2) The Scoring section (RULES.md:46-50)
  says the winner "scores their palette cards"; the code scores only palette
  cards MEETING the current rule (lib.rs:176-179), per official rules.
- recommendation: Document the play-then-discard combo explicitly, and change
  the scoring line to "scores the cards in their palette that meet the
  current rule".

### Aliased re-export `PubCard`/`PubSuit` deviates from sibling crates and is unused
- severity: nit
- category: consistency
- location: game/red7-1/src/lib.rs:16
- finding: `pub use card::{Card as PubCard, Suit as PubSuit};` — sibling card
  crates use plain `pub use card::*;`, and a workspace-wide grep shows
  `PubCard`/`PubSuit` are referenced nowhere. Dead, non-conventional API
  surface.
- recommendation: Switch to `pub use card::*;` (or drop the re-export).

### `leader_with_suit` indexes `player_map[l_index]` — panics if all players eliminated
- severity: nit
- category: quality
- location: game/red7-1/src/lib.rs:251
- finding: `card::leader()` returns `(0, vec![])` for an empty palette list,
  so `player_map[l_index]` panics when every player is eliminated. All
  current call sites guarantee at least one non-eliminated player, so
  unreachable today — but the invariant is implicit and fragile.
- recommendation: Return `Option<(usize, Vec<Card>)>`, or add a short comment
  documenting the non-empty precondition.

### `end_points` arithmetic underflows for player counts above 10
- severity: nit
- category: quality
- location: game/red7-1/src/lib.rs:22
- finding: `(50 - players * 5) as u32` (lib.rs:22-24) panics on usize
  underflow in debug builds for `players > 10`. All in-crate callers pass
  validated 2..=4, so unreachable in practice, but it is a `pub fn` with no
  guard or documented precondition.
- recommendation: Document the 2..=4 precondition, or clamp / take a
  validated type.

## lost-cities-1

Deprecated-but-deployed (superseded by lost-cities-2); defects still matter
because it serves live games. Rules verified correct against the official
Lost Cities rules (no Go port exists): deck composition, ascending play with
investment lockout, can't-take-just-discarded, round end on deck exhaustion,
round-start order, scoring formula, 3 rounds. Command parser is clean; serde
views leak nothing; card.rs and render.rs are clean modulo the nits below.
Binaries/Cargo.toml: no deviation from the boilerplate.

### `player_state()` unchecked `hands[player]` index — panic via crafted PlayerRender request
- severity: major
- category: correctness
- location: game/lost-cities-1/src/lib.rs:566
- finding: `player_state()` does `hand: self.hands[player].clone()` with no
  bounds check. It is invoked from `Request::PlayerRender` via
  `handle_player_render` (`lib/cmd/src/requester/gamer.rs:44-46,174`) with
  `player` taken straight from the deserialized request JSON — a crafted
  request with `player >= 2` panics the handler (hands always has exactly 2
  entries). All other crate entry points guard the player index; this is the
  one reachable panic from crafted input. Same defect as lost-cities-2.
- recommendation: `self.hands.get(player).cloned().unwrap_or_default()`, or
  have the requester layer reject out-of-range player indices before calling
  `player_state`.

### `draw_hand_full` drops the draw's public+private logs when the draw empties the deck
- severity: major
- category: correctness
- location: game/lost-cities-1/src/lib.rs:434
- finding: When the draw takes the last card, `end_round()` is called and the
  accumulated draw logs (public "drew a card, N remaining" + private "You
  drew …") are silently dropped (lib.rs:434-438) — the final draw of every
  round is absent from the log stream. State is correct (round ends after the
  last card is drawn); only the logs are lost. Same defect as lost-cities-2,
  inherited by it.
- recommendation: Extend the draw logs with `end_round()`'s logs and return
  the combined vec.

### `PlayerState.hand` documented as "sorted" but is never sorted
- severity: minor
- category: consistency
- location: game/lost-cities-1/src/lib.rs:92
- finding: The rustdoc on `PlayerState.hand` (lib.rs:92) and DATA_DOCS.md:18
  both claim "sorted by expedition then value", but `player_state()`
  (lib.rs:562-568) returns the hand in raw acquisition order. API consumers
  relying on the documented ordering get arbitrary order. Same defect as
  lost-cities-2.
- recommendation: Sort the hand in `player_state()`, or fix the two doc
  strings.

### `Stats.investments` never written; `Stats.expeditions` write-only
- severity: minor
- category: quality
- location: game/lost-cities-1/src/lib.rs:44
- finding: `Stats.investments` (lib.rs:44-45) is declared, serialized, and
  incremented nowhere — always 0. `Stats.expeditions` is incremented once
  (lib.rs:376) but `player_stats()` (lib.rs:448-482) never surfaces it. Dead
  weight in every serialized game state. Same in lost-cities-2.
- recommendation: Remove both fields (`#[serde(default)]` for back-compat) or
  actually track and surface them.

### `stats.expeditions` increment condition does not match the field name
- severity: minor
- category: correctness
- location: game/lost-cities-1/src/lib.rs:370
- finding: In `play()`, `stats[player].expeditions += 1` fires only when the
  player's entire expedition tableau is empty — i.e. only on the very first
  card played in a round, counting "rounds with at least one play", not
  expeditions started (lib.rs:370-377). Harmless today only because the stat
  is never displayed.
- recommendation: If kept, test per-expedition emptiness before the push, or
  rename the field.

### Hand-size arithmetic `HAND_SIZE - hand.len()` can underflow
- severity: nit
- category: correctness
- location: game/lost-cities-1/src/lib.rs:401
- finding: `let mut num = HAND_SIZE - hand.len();` panics on usize underflow
  (debug builds) if a hand ever exceeds 8 cards. Latent — unreachable through
  the normal turn cycle — but a saturating form is panic-free.
- recommendation: `HAND_SIZE.saturating_sub(hand.len())`.

### Hardcoded literal player count `2` instead of the `PLAYERS` const
- severity: nit
- category: consistency
- location: game/lost-cities-1/src/lib.rs:144
- finding: The crate defines `const PLAYERS: usize = 2` (lib.rs:25) and uses
  it in places, but four spots use bare `2` (lib.rs:144, 230, 501, 616).
  Harmless while 2-player-only; inconsistent and easy to miss in refactors.
- recommendation: Replace the literals with `PLAYERS`.

### `score()` uses `unwrap()` guarded by an `is_none()` check
- severity: nit
- category: quality
- location: game/lost-cities-1/src/lib.rs:680
- finding: `let cards = exp_cards.get(&e); if cards.is_none() { return acc; } … cards.unwrap()`
  (lib.rs:680-687) — safe only because of the early return. Scoring math
  itself verified correct against official rules.
- recommendation: Restructure with `if let Some(&cards) = exp_cards.get(&e)`.

### `render.rs` builds temporary empty Vecs for map lookups
- severity: nit
- category: quality
- location: game/lost-cities-1/src/render.rs:185
- finding: `by_exp.get(&e).unwrap_or(&vec![])` (render.rs:185,196) allocates a
  throwaway empty `Vec` on every lookup. Tiny render-only cost, but wasteful
  and noisy.
- recommendation: `.map(Vec::as_slice).unwrap_or(&[])`.

## Shared / systemic (boilerplate binaries, all ~27 game crates)

Captured once here (line numbers from love-letter-2's copies; the other four
crates in this batch were diffed against the boilerplate and have zero
deviations).

### Binary-only dependencies declared as library `[dependencies]`
- severity: minor
- category: dependencies
- location: game/love-letter-2/Cargo.toml:9
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio = { features = ["full"] }`
  (Cargo.toml:9-16) are only used by the `src/bin/` binaries, yet are
  declared as library dependencies — every consumer of a game library
  transitively builds the fuzz harness, the cmd/requester stack, and all of
  tokio's "full" feature set. Binaries in the same package can use
  `[dev-dependencies]`, so there is no need for these to be lib deps.
  `tokio "full"` is additionally over-broad for a single `#[tokio::main]` +
  one async call.
- recommendation: Move `brdgme_cmd`, `brdgme_fuzz`, `tokio` to
  `[dev-dependencies]` (merge with the existing `brdgme_cmd` dev-dep with
  `test-support`), and reduce tokio features to `["rt-multi-thread", "macros"]`.

### HTTP binary defaults to privileged port 80
- severity: nit
- category: quality
- location: game/love-letter-2/src/bin/love_letter_2_http.rs:9
- finding: `env::var("ADDR").unwrap_or("0.0.0.0:80".to_string())` — binding
  port 80 requires root / CAP_NET_BIND_SERVICE, so the binary fails out of
  the box when run unprivileged without `ADDR` set (e.g. local dev).
- recommendation: Default to an unprivileged port (e.g. `0.0.0.0:8080`) or
  require `ADDR` explicitly.
