# Game Development Guidelines

Best practices for building game crates. The porting guide
(`docs/porting/GAME_PORTING.md`) covers Go-to-Rust conversion mechanics and
retires when porting finishes; this document is permanent.

## Randomness

All game randomness must be deterministic given the seed passed to
`Gamer::start`. See the `brdgme_game::rng` module docs for design rationale.

- Never call `rand::rng()`, `rand::random()`, or `rand::random_range()` in
  game code.
- Store one `rng: brdgme_game::rng::GameRng` field in your `Game` struct
  (it serializes with the rest of the state) and seed it first thing in
  `start()` with `GameRng::seed_from_u64(seed)`.
- Draw everything from that field: `deck.shuffle(&mut self.rng)`,
  `self.rng.random_range(1..=6)`, `faces.choose(&mut self.rng)`. Helper
  functions take `rng: &mut GameRng` as a parameter.
- Games with no randomness (e.g. Battleship) name the parameter `_seed`
  and add no field.
- Randomness only ever happens while processing `start()` or `command()`.
  Never draw from the RNG in `pub_state`, `player_state`, `status`,
  `points`, or render code - those must stay pure.
- Reference implementation: `rust/game/zombie-dice-2`.

Tests seed explicitly (`Game::start(3, 42)`) and may re-seed mid-test
(`g.rng = GameRng::seed_from_u64(n)`) to make a specific outcome happen;
comment what property the seed was chosen for. Same seed + same commands =
identical game, so exact assertions are safe.

When a focused test is complicated by unrelated randomness, re-seed the RNG
mid-test to produce a known outcome for that unrelated part, then exercise the
behavior under test. Comment the intended outcome of the seed choice. This
keeps tests deterministic without constructing unnecessary state.

## Commands

- Set `can_undo: false` for commands that reveal hidden information or create
  random information gain. Only fully deterministic operations that expose no
  new information qualify for `true`.

## Error handling

Helpers that are genuinely infallible should return plain values rather than a
`Result` whose errors callers discard. Reserve `Result` for operations where
callers must distinguish success from rejection or failure.

## Module boundaries

Extract modules only for cohesive independently-understandable subsets where
doing so materially improves `lib.rs` readability. Avoid tiny single-type
modules. The `command.rs` (Command enum + parser combinators) and `render.rs`
(PubState/PlayerState + Renderer impls) split is the established pattern for
Rust game crates; extract additional modules only when `lib.rs` grows unwieldy.

## Tests

Games with hidden information must test that serialized `PubState` does not
leak it. After an action grants hidden information, inspect its fields
structurally: assert private fields are absent, while public count fields are
present and correct. Do not rely on rendered output or assertions about hidden
values for this boundary.

## CI verification

Run `cargo fmt --all -- --check` and `cargo clippy --workspace --exclude web
--all-targets -- -D warnings` after all Rust changes. These are the same checks
CI runs (`.github/workflows/ci.yml`) and must pass clean before merging.
