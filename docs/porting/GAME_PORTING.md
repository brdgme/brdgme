# Game Porting Guide (Go -> Rust)

How to port a game to a Rust game crate in this project. Two source kinds:

- **Old project** (`~/Development/brdg.me/game/<name>`) -> `rust/game/<name>-1`.
  The bulk of this guide covers these.
- **In-repo Go games** (`brdgme-go/<name>_1`) -> `rust/game/<name>-2` (new
  edition number, like lost-cities-2). These skip most of the restructuring
  below: the Go source already matches the platform architecture (int players,
  returned logs, parser combinators, placings, JSON state), so the work is
  language translation - int-const enums become Rust enums, `interface{}`
  states become typed `PubState`/`PlayerState`, render strings become `Node`
  trees. Tests are ported 1:1 (see step 8) - they are the proof that the
  conversion preserves behaviour. Deployment-wise the `-2` gets its own
  manifests and the `-1` `GameVersion` is marked `isDeprecated: true`.

Decision to target Rust rather than Go: `docs/decisions/GO_VS_RUST_PORTING.md`.
Reference ports: `rust/game/lost-cities-1` (small, clean, recent idioms -
primary template) and `rust/game/acquire-1` (large).

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
rust/game/<name>-1/
  Cargo.toml            # deps: brdgme_cmd, brdgme_fuzz, brdgme_color,
                        #       brdgme_game, brdgme_markup, rand, serde, tokio
  RULES.md              # player-facing rules text
  src/
    lib.rs              # Game struct, Gamer impl, core logic
    card.rs             # (or board.rs etc.) domain types as serde enums/structs
    command.rs          # Command enum + parser combinators
    render.rs           # Renderer impls for PubState/PlayerState
    bin/
      <name>_1_cli.rs   # 4 tiny stubs calling brdgme_cmd entry points
      <name>_1_http.rs  #   (http is what runs in production)
      <name>_1_repl.rs
      <name>_1_fuzz.rs
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
   Decks/hands are `Vec<Card>`; shuffle with `rand::seq::SliceRandom`.
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
   - Where the old suite is thin or absent (e.g. zombie_dice has zero tests),
     1:1 preserves nothing: add at least happy-path tests for every command
     plus end-of-game scoring before calling the port done.
   - `tests/contract.rs` with `assert_gamer_contract::<Game>()` (needs the
     `test-support` dev-dependency feature).
   - The fuzz binary gives free crash-hunting: `cargo run --bin <name>_1_fuzz`.
     Run it for a while before shipping; it catches panics unit tests miss.
9. **Binaries**: copy the four ~12-line stubs from lost-cities-1, rename the
   crate references.

## Registration / deployment checklist (per game)

1. `rust/Cargo.toml`: add `game/<name>-1` to workspace `members`.
2. `rust/Dockerfile`: add final stage (`FROM debian:bookworm-slim AS <name>-1`,
   copy `target/release/<name>_1_http`, `CMD`). The workspace build stage
   picks the crate up automatically.
3. `.github/workflows/ci.yml`: add an `image`/`target` entry to the
   `build-rust-games` job matrix. Without this the image is never built or
   pushed to GHCR, so the prod override in step 6 points at an image that
   doesn't exist (this was missed for liars-dice-2/no-thanks-2 and only
   caught in review).
4. `Tiltfile`: add `"<name>-1"` to the Rust games list.
5. `k8s/base/game/<name>-1/`: `deployment.yaml`, `service.yaml` (port 80),
   `game-version.yaml` (`kind: GameVersion`, `spec.typeName` display name,
   `weight`), `kustomization.yaml`; add the dir to
   `k8s/base/game/kustomization.yaml`.
6. `k8s/prod/app/kustomization.yaml`: add the `ghcr.io/brdgme/brdgme/<name>-1`
   image override.
7. Verify: `cargo build && cargo test` in `rust/`, then a Tilt/Kind run.

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
