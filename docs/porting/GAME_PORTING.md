# Game Porting Guide (Go -> Rust)

How to port a game to a Rust game crate in this project. Two source kinds,
regardless of which version number the crate ends up with (see "Versioning"
below):

- **Old project** (`~/Development/brdg.me/game/<name>`). The bulk of this
  guide covers these - the Go source follows the old monolithic-server
  architecture and needs restructuring (see "Big picture").
- **In-repo Go games** (`brdgme-go/<name>_1`). These skip most of the
  restructuring below: the Go source already matches the platform architecture
  (int players, returned logs, parser combinators, placings, JSON state), so
  the work is language translation - int-const enums become Rust enums,
  `interface{}` states become typed `PubState`/`PlayerState`, render strings
  become `Node` trees. Tests are ported 1:1 (see step 8) - they are the proof
  that the conversion preserves behaviour.

Decision to target Rust rather than Go: `docs/decisions/GO_VS_RUST_PORTING.md`.
Reference ports: `rust/game/lost-cities-1` (small, clean, recent idioms -
primary template) and `rust/game/acquire-1` (large).

General best practices: `docs/authoring/GAME_DEVELOPMENT.md`.

## Porting correctness rule

Preserve source behaviour by default. Every suspected source defect (inverted
conditions, off-by-one counts, missing validations) must be raised with the
user and approved before correction during implementation.

## Versioning

Every new port increments the highest existing version number for that game
across all implementations, not merely Rust crates or deployments. The
original Go implementation counts as version 1, so the first Rust port is
`<name>-2` even when no `<name>-1` Rust crate exists. Never reuse an old
version number. Authoring a truly new game (no prior implementation in any
language) is outside this porting guide.

- `lost-cities` was first ported from the old project as `lost-cities-1`
  (predates this versioning rule); a later replacement port from the in-repo
  Go code became `lost-cities-2`.
- `jaipur` has only the old-project Go implementation (version 1). Its first
  Rust port is `jaipur-2`.

When replacing a previously deployed GameVersion, the new version gets its own
manifests and the old GameVersion is marked `isDeprecated: true`. If no prior
version was deployed in this repository, no deprecation manifest is needed.

## Big picture

The old project was a monolithic Go server: games implemented `game.Playable`,
identified players by name strings, gob-encoded state, and pushed log messages
into a `*log.Log` on the game struct. Here, each game is a standalone crate
compiled to binaries speaking JSON over HTTP/stdin (via `brdgme_cmd`),
deployed as its own container. Games implement the `brdgme_game::Gamer` trait,
identify players by `usize` index, serialize state with serde, and return log
entries from each call.

## Crate layout (mirror lost-cities-1)

```
rust/game/<name>-N/
  Cargo.toml            # deps: brdgme_cmd, brdgme_fuzz, brdgme_color,
                        #       brdgme_game, brdgme_markup, rand, serde, tokio
  RULES.md              # player-facing rules text
  src/
    lib.rs              # Game struct, Gamer impl, core logic
    card.rs             # (or board.rs etc.) domain types as serde enums/structs
    command.rs          # Command enum + parser combinators
    render.rs           # Renderer impls for PubState/PlayerState
    bin/
      <name>_N_cli.rs   # 4 tiny stubs calling brdgme_cmd entry points
      <name>_N_http.rs  #   (http is what runs in production)
      <name>_N_repl.rs
      <name>_N_fuzz.rs
  tests/
    contract.rs         # assert_gamer_contract::<Game>();
```

## Interface mapping: `game.Playable` (old Go) -> `Gamer` (Rust)

| Old Go | Rust |
|---|---|
| `Start(players []string) error` | `fn start(players: usize) -> Result<(Self, Vec<Log>), GameError>` |
| `Name()/Identifier()` | gone (display name lives in the k8s `GameVersion` manifest) |
| `Encode()/Decode()` (gob) | gone; `#[derive(Serialize, Deserialize)]` on `Game` |
| `Commands(player)` + `command.Command` structs | `fn command_spec(&self, player) -> Option<Spec>` + `fn command(&mut self, player, input, players) -> Result<CommandResponse, GameError>` |
| `RenderForPlayer(string) (string, error)` | `type PubState` / `type PlayerState` (serde types) implementing `brdgme_game::Renderer` (`fn render(&self) -> Vec<Node>`) |
| `PlayerList()` | gone; games only know the player count |
| `IsFinished()/Winners()` | `fn status(&self) -> Status` (`Active{whose_turn, eliminated}` / `Finished{placings, stats}`); `is_finished`/`whose_turn`/`placings` come free from the trait |
| `GameLog() *log.Log` | every mutating call returns `Vec<Log>` (`Log::public(nodes)` / `Log::private(nodes, to)`) |
| `Eliminator` | `Status::Active.eliminated` |
| n/a | `fn player_count(&self)`, `fn player_counts() -> Vec<usize>`, `fn points(&self) -> Vec<f32>`, `Stat` map in `Status::Finished` |

## Porting steps

1. **Model the domain as enums.** Old Go int-constant "enums" (`TILE_QUEEN_BEE
   = iota`, suit bitmasks, card kind ints) become real Rust enums with
   `#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, ...)]`. Old
   interface-based card hierarchies (alhambra's `Card`/`ScoringCard`,
   seven_wonders' ~18 gob-registered types, starship_catan's behaviour
   interfaces) become a single `enum Card { ... }` with variant data - serde
   handles tagged serialization natively. Static card data (costs, names,
   effect parameters) goes in `impl` methods or const tables keyed by variant.
   Decks/hands are `Vec<Card>`; shuffle with the game's `rng` field - see
   `docs/authoring/GAME_DEVELOPMENT.md` (Randomness).

   **Logs and information boundaries are observable behaviour.** Porting means
   reproducing what players and spectators see, not just final state. Log
   messages, their order, and public/private visibility are part of the spec.
   Write tests that assert on `Vec<Log>` as well as game state.
2. **Game state.** Translate the old `Game` struct: `Players []string` ->
   player count only; `map[int]X` keyed by player -> `Vec<X>`; phases ->
   `enum Phase`. Everything `#[derive(Default, Clone, Serialize, Deserialize)]`.
3. **start.** Old `Start` validation and setup; return `(game, logs)` instead
   of writing to `g.Log`.
4. **Log plumbing.** Functions that did `g.Log.Add(log.NewPublicMessage(...))`
   return `Vec<Log>`; callers accumulate and bubble up to `command`. Messages
   are markup `Node` trees (`N::text`, `N::Bold`, `N::Player(p)`,
   `N::Fg(color, ...)`) rather than `{{b}}...{{_b}}` format strings.
5. **Commands.** Define `enum Command { Play(Card), ... }`. Build
   `command_parser(&self, player) -> Option<Box<dyn Parser<T = Command>>>`
   returning `OneOf::new(...)` of the commands currently available to that
   player (old `CanX` guards move here AND stay as validation inside the
   action methods). Combinators mirror the old-Go-port ones: `Token`, `Int`,
   `Enum`, `Chain2/3/...`, `AfterSpace`, `Map`, `Opt`, `Many`, `Doc::name_desc`,
   `Player`. Old `Usage()` strings become `Doc` descriptions.
   `command()` parses, matches on the `Command` variant, dispatches, and
   returns `CommandResponse { logs, can_undo, remaining }`:
   - `can_undo: true` only for deterministic moves revealing no hidden info
     (dice rolls, draws, reveals => `false`).
   - always pass through the parser's `remaining` - it enables chained
     commands in one input line.

   **Validate parser combinator semantics against source behaviour.** Parser
   combinators (`OneOf`, `Chain2`, `Token`, `Enum`, etc.) have subtle semantics
   that may not exactly match source text parsing. After building the parser,
   run every command form from the source test suite through it and verify:
   the correct variant is parsed, error messages match intent, and edge cases
   (leading/trailing spaces, partial matches, and ambiguous matches) match
   source behaviour. Any deviation that appears preferable must be raised with
   and approved by the user under the porting correctness rule. Add regression
   tests at the parser level, not only at the command-dispatch level.
6. **Pub/player state + render.** Define `PubState` (spectator view: hidden
   info reduced to counts/summaries) and `PlayerState` (usually
   `{ public: PubState, player: usize, hand: ... }`). Implement `Renderer`
   for both in `render.rs` using `brdgme_markup` (`Node`, `Row`, `Align`,
   table helpers) - this replaces the old `render.Table`/`Centred` string
   layout. Colors come from `brdgme_color` constants.

   **Port the rendered output, not the Go state struct field-for-field.** In
   `brdgme-go` the `Render(player *int)` method typically reads `Game`
   fields directly, while `PubState()` and `PlayerState()` are
   serialization-only and may be incomplete or buggy relative to what is
   actually rendered. In Rust `PubState`/`PlayerState` *drive* the
   `Renderer`, so they must capture what the user sees on screen. If a Go
   state struct omits or mis-computes a field that `Render` sources from
   `Game`, port the rendered behaviour, not the buggy/missing field.
   liars_dice-1 is the example: its `PlayerState.Dice` is always empty due
   to an inverted bounds guard (`len(g.PlayerDice) < player`), yet render
   still shows dice correctly because it reads `g.PlayerDice` directly.
   Copying that state struct literally would have hidden every player's
   dice in the Rust port.

   **REQUIRED: verify render parity against the Go CLI before calling this
   step done.** Build the Rust `<name>_N_cli` binary and the Go
   `<game>_1/cmd` binary, render both to plain text, and compare wording,
   whitespace/column spacing, and alignment for the pub render **and every
   player render**, at game start and after representative mid-game
   commands. Full procedure: `docs/porting/RENDER_PARITY.md`. Known trap:
   Go `render.Table(cells, rowSpacing, colSpacing)` inserts literal spacer
   cells between columns; `Node::Table` in Rust has no spacing parameter, so
   the port must insert those spacer cells by hand or columns glue together
   (this is exactly how `category-5-2` shipped broken - see the "Known bug"
   section of that doc).
7. **status/points.** `Status::Finished { placings: gen_placings(&metrics),
   stats }` - build per-player metric vectors (score, then tiebreakers)
   exactly like the old `Winners()` logic implied. `points()` returns the
   running score.

   **Placings tie semantics differ across versions.** Go
   `brdgme.GenPlacings` ranks ties compact-ordinal (two tied at top ->
   `[1, 1, 2]`; `curPlace++` per unique group). Rust
   `brdgme_game::game::gen_placings` ranks standard-competition (same tie ->
   `[1, 1, 3]`; `cur_place += group_size`). The Rust `gen_placings_works`
   tests don't cover ties, so neither semantic is pinned by the suite - the
   divergence appears inadvertent. **Decision (2026-07): keep Rust
   standard-competition; when porting Go games whose placings tests assert
   compact-ordinal results, adapt the expected assertion to Rust output and
   note the deviation in the port's PR description (per step 8).** Affects
   both tracks; for example `no_thanks-1` `TestWinners` asserts `[]int{1, 1,
   2}` and becomes `vec![1, 1, 3]` in `no-thanks-2`. Audit each Go suite's
   `Placings`/`Winners` test for tie cases before porting it verbatim.
8. **Tests - 1:1 porting is required.** The Go tests are the executable spec
   of each game's rules; they are how we know functionality survived the port.
   - Port **every** existing Go test case, keeping the original test names
     (snake_cased) and assertions: `helper.Cmd(g, helper.Mick, "play x")` /
     `g.Command(0, "play x", ...)` -> `game.command(0, "play x", &players)`;
     testify asserts -> `assert!`/`assert_eq!`. Do not drop cases because they
     look redundant; if a case cannot be ported (tests old-framework
     behaviour, not game rules), note it in the port's PR description.
   - Where old tests fixed game state directly (e.g. stacking a deck before a
     command), keep doing that - construct the `Game` struct explicitly.
   - When mechanically constructing identical source state is impossible,
     translate the test's intent - the rule it exercises - and construct a
     deterministic Rust scenario that exercises the same rule. Do not contrive
     brittle preconstructed state solely to match the source line-for-line
     when a simpler setup tests the same behaviour.
   - Where the old suite is thin or absent (e.g. zombie_dice has zero tests),
     1:1 preserves nothing: add at least happy-path tests for every command
     plus end-of-game scoring before calling the port done.
   - `tests/contract.rs` with `assert_gamer_contract::<Game>()` (needs the
     `test-support` dev-dependency feature).
   - The fuzz binary gives free crash-hunting: `cargo run --bin <name>_N_fuzz`.
     Run it for a while before shipping; it catches panics unit tests miss.
9. **Binaries**: copy the four ~12-line stubs from lost-cities-1, rename the
   crate references.
10. **Update tracking documents.** After all CI/registration steps pass and
    the port is deployed, update `docs/BACKLOG.md` to move the port from planned
    to done. If any game-specific tracking documents reference the port, update
    those too. This prevents stale backlog entries that make it unclear whether
    a game still needs porting.

## Registration / deployment checklist (per game)

1. `rust/Cargo.toml`: add `game/<name>-N` to workspace `members`.
2. `rust/Dockerfile`: add final stage (`FROM debian:bookworm-slim AS <name>-N`,
   copy `target/release/<name>_N_http`, `CMD`). The workspace build stage
   picks the crate up automatically.
3. `docker-bake.hcl`: add the crate name (e.g. `<name>-N`) to the `tgt` array
   inside `target "image"`. This makes the image matrix build and push that
   game image to GHCR. Separately, CI runs `cargo test --workspace --exclude web`,
   which covers this crate because item 1 registers it in the Cargo workspace.
4. Report actual test counts, not hand-aggregated estimates. Run:
   ```
   cargo test --package <name>-N -- --list --format terse | rg ': test$' | wc -l
   ```
   Filtering `: test` excludes per-target summary lines. Use the integer from
   that command in the PR description. If the ported count differs from the
   source count, briefly explain why.
5. `Tiltfile`: add `"<name>-N"` to the Rust games list.
6. `k8s/base/game/<name>-N/`: `deployment.yaml`, `service.yaml` (port 80),
   `game-version.yaml` (`kind: GameVersion`, `spec.typeName` display name,
   `weight`), `kustomization.yaml`; add the dir to
   `k8s/base/game/kustomization.yaml`.
7. `k8s/prod/app/kustomization.yaml`: add the `ghcr.io/brdgme/brdgme/<name>-N`
   image override.
8. Verify: `cargo build --package <name>-N` and `cargo test --package <name>-N`
   in `rust/`, then a Tilt/Kind run.
9. Render-parity comparison (step 6) has been performed and passed for the
   pub render and every player render, at game start and after
   representative mid-game commands - per `docs/porting/RENDER_PARITY.md`.
10. Run `cargo fmt --all -- --check` and `cargo clippy --workspace --exclude web
    --all-targets -- -D warnings` after completing Rust changes. These are the
    same checks CI runs (`.github/workflows/ci.yml`) and must pass clean before
    merging.

## Gotchas

- Command availability is checked twice: parser construction (unavailable
  commands don't parse) and inside the action method (defence in depth).
- Redaction is structural here, not best-effort: `PubState` simply does not
  contain hidden fields. Get this right when defining the types and the
  render layer can't leak.
- Simultaneous-turn games (seven_wonders): `whose_turn` returns all
  unresolved players; `command_spec` must be per-player.
- Old Go code full of `map[int]bool` sets -> `HashSet`/`Vec<bool>`; keep
  serialized shapes simple (Vecs) for state.
- Ordering of returned logs matters (action log, then consequences, then
  next-turn logs) - preserve the old message order; tests often assert on it.
- **Borrow order in `command()`.** `command_parser(&self, player)` returns
  `Option<Box<dyn Parser<T = Command> + '_>>` borrowing `&self`. If you
  bind the returned parser to a `let`, that immutable borrow lives until
  end-of-scope and blocks the `&mut self` call to the action method
  (`self.pass(player)`, `self.take(player)`, ...) with E0502. Inline the
  `parse` call in the same expression so the borrow ends before the
  mutation:

  ```rust
  let output = match self.command_parser(player) {
      Some(cp) => cp,
      None => return Err(GameError::invalid_input("not expecting any commands at the moment")),
  }
  .parse(input, players);
  match output {
      Ok(ParseOutput { remaining, value: Command::Pass, .. }) => {
          let logs = self.pass(player)?;
          Ok(CommandResponse { logs, can_undo: false, remaining_input: remaining.to_string() })
      }
      // ...
  }
  ```

  The `liars-dice-2` and `no-thanks-2` crates both follow this pattern.
