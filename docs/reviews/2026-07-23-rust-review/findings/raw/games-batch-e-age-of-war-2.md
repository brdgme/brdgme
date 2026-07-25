# Raw findings: game/age-of-war-2 (Worker, games batch E)

Snapshot reviewed: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/age-of-war-2/`
Go cross-reference: `brdgme-go/age_of_war_1/` (game.go, command.go, attack/line/roll_command.go, castles.go, dice.go, render.go all read in full).
No `PORTING_NOTES.md` exists in this crate; documented quirks live in code comments/tests.

## Verified-clean areas (no findings)

- **Castle/clan/dice data tables** (`src/castle.rs:259-395`): all 14 castles, clan
  groupings, point values, battle lines, clan set points, die faces, and colours
  match `brdgme-go/age_of_war_1/castles.go` and `dice.go` exactly, line for line.
- **Core turn logic** (`src/lib.rs` `check_end_of_turn`, `attack`, `line_action`,
  `roll_action`): semantics match Go `CheckEndOfTurn`/`Attack`/`Line`/`RollForPlayer`
  exactly, including steal-adds-Daimyo-line, min-dice fail condition, exact-min-dice
  no-affordable-line fail condition, reroll-after-line (`roll(len - using)` is
  provably non-negative since `can_afford` guarantees `using <= roll.len()`), and
  turn advancement. `roll_action`'s `saturating_sub(1)` (lib.rs:383) vs Go's
  `len-1` is a safe deviation: the empty-roll state is unreachable (with 0 dice
  `check_end_of_turn` always advances the turn, since every castle line needs >= 1
  die); Go's `RollN` also returns empty for n <= 0.
- **Scoring** (`scores`, `clan_conquered`, `calc_placings`): matches Go `Scores`/
  `ClanConquered`/`Placings` semantics, including the stale-player-on-false Go
  quirk (documented at lib.rs:90-92) and the deliberate standard-competition
  placing divergence from Go's compact ordinals (documented in test at
  lib.rs:787-799).
- **Command-after-finish quirk**: `can_attack`/`can_line`/`can_roll` not checking
  finished status is a documented, test-preserved Go quirk (lib.rs:811-826). Not
  flagged as a defect (cross-reference only).
- **Command parser validation** (`src/command.rs`): castle enum built from
  attackable castles only, `Enum::partial` for names, `Enum::exact` over
  uncompleted 1-based line numbers, and `line_action` re-validates range and
  affordability. No input-validation gap found; no negative/overflow path
  (`line0 < 0` checked before `as usize` cast at lib.rs:337-341).
- **Serde views / information leaks**: fully public game; `PubState` and
  `PlayerState` (lib.rs:44-72) contain only public information. No hidden state
  exists to leak. `completed_lines` is sorted before exposure (lib.rs:434-435).
- **`rules()` convention**: returns `include_str!("../RULES.md")` (lib.rs:541-543)
  per CODING.md. V2 docs (`DATA_DOCS.md`, `BASIC_STRATEGY.md`,
  `ADVANCED_STRATEGY.md`) all present and wired (lib.rs:545-555).
- **Binaries + Cargo.toml deviation check**: the 4 bins are byte-identical to the
  standard boilerplate (compared against battleship-2/cathedral-2); `Cargo.toml`
  is identical to `battleship-2/Cargo.toml` modulo crate name (diff exit 0).
  Zero deviations — systemic issues (binary-only deps in `[dependencies]`,
  `0.0.0.0:80`, tokio "full") intentionally not re-flagged.
- **Go dice modulo bias not ported**: Go `Roll()` is `rnd.Int() % 6`; Rust uses
  `GameRng::random_range(0..6)` (lib.rs:173) — an improvement, not a finding.

## Findings

### Panicking unwrap/expect cluster in game-service runtime paths
- severity: minor
- category: consistency
- location: game/age-of-war-2/src/lib.rs:132, game/age-of-war-2/src/lib.rs:219, game/age-of-war-2/src/lib.rs:334, game/age-of-war-2/src/command.rs:89, game/age-of-war-2/src/render.rs:48, game/age-of-war-2/src/render.rs:110
- finding: Six `.unwrap()`/`.expect()` sites sit on paths executed by the Play
  endpoint: `scores()` unwraps `ALL_CLANS.iter().position(...)` (lib.rs:132,
  statically impossible to fail since `Clan` is a closed 6-variant enum and
  `ALL_CLANS` covers it); `check_end_of_turn` expects a conquered castle to have
  an owner (lib.rs:219); `line_action` expects `currently_attacking` (lib.rs:334);
  `line_parser` expects `currently_attacking` (command.rs:89); `render_castle`
  expects an owner for conquered castles (render.rs:48) and `render_castles`
  expects an owner for conquered clans (render.rs:110). I verified each is guarded
  by an invariant the command flow maintains (conquest always sets `conquered` +
  `castle_owners` together at lib.rs:223-224; `can_line` gates both
  `currently_attacking` expects), so none is reachable by crafted player input —
  but CODING.md's "no panicking code in runtime paths" rule makes no
  invariant-guarded exception for game services, and a panic here kills the game
  service request.
- recommendation: Convert to error propagation where a `Result` is available
  (`line_action`: `ok_or_else(|| GameError::internal(...))?` or reuse the
  existing invalid-input error); for render/state-invariant sites, either use a
  non-panicking fallback (skip the owner node) or restructure so the invariant is
  expressed in the type (e.g. store conquered castles as `(owner, ...)` pairs
  instead of parallel `conquered`/`castle_owners` vectors). At minimum, the
  `scores()` position-unwrap can become a `match`/`if let` with `continue` since
  it is provably total.

### `completed_lines: HashSet<usize>` in persisted Game state serializes nondeterministically
- severity: minor
- category: quality
- location: game/age-of-war-2/src/lib.rs:35
- finding: `Game` is the serde-persisted state blob, and `completed_lines` is a
  `HashSet<usize>`. serde serializes a HashSet by iteration order, which is
  randomized per process/state, so two logically identical states can persist to
  different JSON byte strings. Any infrastructure that diffs, hashes, or
  deduplicates serialized game states (or replays/fixtures comparing blobs) gets
  spurious mismatches. The pub view already works around this by sorting
  (lib.rs:434-435), which is evidence the nondeterminism was noticed but only
  fixed on the output side.
- recommendation: Use `BTreeSet<usize>` (deterministic order, same API surface
  used here: `contains`/`insert`/`clear`/`iter`) or a sorted `Vec<usize>`.

### "Not your turn" returned as unstructured `invalid_input` instead of `GameError::NotYourTurn`
- severity: nit
- category: consistency
- location: game/age-of-war-2/src/lib.rs:461-464
- finding: `command()` maps a missing parser (which happens exactly when it is
  not the player's turn, since `can_roll` is otherwise always true for the
  current player) to `GameError::invalid_input("not your turn")`. The
  `brdgme_game::errors::GameError` type has a dedicated `NotYourTurn` variant
  (used via `assert_player_turn` in the trait), so callers/framework code that
  match on the structured variant will misclassify this rejection as a generic
  input error.
- recommendation: Return `GameError::NotYourTurn` (or call
  `self.assert_player_turn(player)?` before parsing).

### Placings-log tail triplicated across all three command arms
- severity: nit
- category: simplicity
- location: game/age-of-war-2/src/lib.rs:473-482, game/age-of-war-2/src/lib.rs:491-500, game/age-of-war-2/src/lib.rs:509-519
- finding: The identical 10-line block (build scores vec, push `placings_log`
  when `is_finished()`) is copy-pasted into the Attack, Line, and Roll arms of
  `command()`. A future change to end-of-game logging must be made in three
  places.
- recommendation: Extract a small helper, e.g.
  `fn finish_response(&self, resp: &mut CommandResponse)` or a wrapper that runs
  the command closure then appends the placings log once.

### Finished games keep emitting duplicate placings logs (side effect of preserved Go quirk)
- severity: nit
- category: correctness
- location: game/age-of-war-2/src/lib.rs:473, game/age-of-war-2/src/lib.rs:491, game/age-of-war-2/src/lib.rs:509
- finding: Because `can_roll` deliberately does not check finished status
  (preserved Go quirk, test `command_after_finished_still_accepted_go_quirk` at
  lib.rs:816), the current player can keep issuing `roll` after the game ends;
  each accepted command appends another `placings_log` (and `roll_action` also
  advances `current_player` and emits failed-attack/roll logs in a finished
  game). Cosmetic log spam only — `status()` stays `Finished` and placings are
  unaffected — and the underlying quirk is intentional, but the Rust-added
  placings-log amplification is new relative to Go (Go's Command never appends
  placings logs).
- recommendation: Gate the placings-log append on "this command finished the
  game" rather than "game is finished" (e.g. check `is_finished()` before the
  command and only append on a false→true transition), or no-op the response
  logs when the game was already finished.

### `clan_conquered` logic duplicated between `Game` and the renderer
- severity: nit
- category: quality
- location: game/age-of-war-2/src/lib.rs:93-113, game/age-of-war-2/src/render.rs:10-30
- finding: The clan-conquest scan (including the subtle preserved Go quirk of
  returning a possibly-stale player on a `false` result) is implemented twice:
  once on `Game`, once as a free function over `PubState`. The duplication exists
  because the renderer only has the pub view, but the two copies must be kept in
  lockstep forever, quirk included.
- recommendation: Extract one shared helper taking `&[bool]` +
  `&[Option<usize>]` (the two slices both copies read) and call it from both
  sites, keeping the quirk comment in one place.

### Player-facing help text: "discard one dice"
- severity: nit
- category: quality
- location: game/age-of-war-2/src/command.rs:117
- finding: The `roll` command description reads "discard one dice and roll the
  rest" — "dice" should be "die". The text is carried over verbatim from Go
  (`rollParser` in command.go), so this is port fidelity rather than a new typo,
  but the Rust port is the player-facing surface now. (`line_action`'s plural
  handling at lib.rs:351 is correct, so the log message itself is fine.)
- recommendation: Change to "discard one die and reroll the rest" (verify against
  the suggest/help snapshot tests, if any assert on the spec text).
