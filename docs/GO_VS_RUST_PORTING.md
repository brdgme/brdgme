# Go vs Rust for the Remaining Game Ports

Question: should the remaining old-project games be ported Go -> Go (per
`docs/GO_GAME_PORTING.md`) or rewritten in Rust like acquire-1 and
lost-cities-1? Evidence from comparing the two existing Rust ports against
their old Go sources, and the maturity of `rust/lib` vs `brdgme-go`.

## Evidence

**Code size is at parity.** The Rust ports are not bigger than the Go originals:

| Game | Old Go | Rust port (incl. 4 bin stubs) |
|---|---|---|
| lost_cities | 1,266 lines | 1,290 lines |
| acquire | 3,881 lines | 3,704 lines |

**The two target APIs are near-isomorphic.** `brdgme_game::Gamer` (Rust trait)
and `brdgme.Gamer` (Go interface) mirror each other: `start/New`,
`command` returning `CommandResponse{logs, can_undo, remaining}`, `status`
Active/Finished, `command_spec`, `gen_placings`, the same parser combinator
set (Token/Int/Enum/OneOf/Chain/Map/Opt/Many/Doc/Player). The hard part of
any port - restructuring for int players, returned logs, parser-based
commands, placings, serializable state - is identical work in either
language. Go -> Go only saves the language translation itself.

**The Rust side is the better-supported target.**
- Typed `PubState`/`PlayerState` with a `Renderer` trait vs Go's untyped
  `interface{}` + render-to-string.
- Serde gives tagged-enum serialization natively - the exact thing the Go
  side cannot do (docs/DECK_DESIGN.md exists mostly to work around Go's
  interface/JSON limitation; in Rust `enum Card { ... }` + `Vec<Card>` is
  the idiomatic solution, no design needed).
- Free per-game infrastructure: cli/http/repl/fuzz binaries,
  `assert_gamer_contract` contract test, `rand_bot` fuzzing.
- Markup is a typed `Node` AST rather than format strings.

**Where Go -> Go is genuinely cheaper**: mechanical ports of small/medium
games (tic_tac_toe, jaipur, red7) where the old Go logic transfers
line-by-line. For the big three (alhambra, starship_catan, seven_wonders)
the interface-deck restructure already forces a semi-rewrite in Go, so the
saving over a Rust rewrite shrinks substantially - and the restructure's
target design (concrete structs + kind enums + switch dispatch) is just a
worse-tooling imitation of Rust enums with data.

**Rust library gaps** (small): `rust/lib` has cmd/color/game/markup/rand_bot
but no ports of `libcost` (needed by seven_wonders, ~330 lines of
cost/permutation logic), `libdie`, or `libpoker` (neither needed by the
remaining games). Card/deck handling needs no library at all in Rust.

## View / recommendation

**Port the remaining games to Rust.**

- For the three large games (~10.5k of the ~13k remaining lines), effort is
  near parity with Go and the result is strictly better: enum-based cards
  instead of the switch-dispatch workaround, contract tests, fuzzing, typed
  states, and no growth of the legacy Go surface.
- For the three small/medium games, Go -> Go would be somewhat faster, but
  they are small in absolute terms; consistency and preference for Rust
  outweigh the saving. tic_tac_toe first as the Rust-port warm-up, mirroring
  lost-cities-1's structure.
- hive/chess (deferred, WIP-only): if ever built, build them in Rust
  directly; there is no working Go implementation to preserve.

## Status: adopted (2026-07-04)

Decision adopted, and extended to converting the 17 in-repo `brdgme-go` games
to Rust as `-2` editions. Consequences applied:

- Porting guide rewritten for Rust: `docs/GAME_PORTING.md` (replaced
  `GO_GAME_PORTING.md`).
- Per-game plan re-targeted at Rust with two tracks (old-project ports +
  `-2` conversions): `docs/GAME_PORTING_PLAN.md` (replaced
  `GO_GAME_PORTING_PLAN.md`).
- The Go generic-deck design (`DECK_DESIGN.md`, required a Go 1.22 toolchain
  bump) was dropped as unnecessary - Rust enums + serde cover it.
- `docs/superpowers/specs/2026-07-04-23-rust-game-ports-design.md` re-pointed at Rust ports.
- Rust library prerequisites: cost/permutation module (seven_wonders,
  splendor-2), poker hand evaluation (texas-holdem-2).
