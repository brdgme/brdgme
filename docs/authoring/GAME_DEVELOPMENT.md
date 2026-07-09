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
