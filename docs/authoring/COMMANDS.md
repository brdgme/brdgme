# Commands and autocomplete

How command grammars, parsing, and autocomplete work, and what game
authors need to know. See `docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`
for the design decisions behind the shared engine.

## One grammar, automatic autocomplete

- Define the command grammar once as typed combinators in `src/command.rs`
  via `command_parser(player)`. Execution is `parser.parse(input, players)`
  server-side; autocomplete is automatic - the parser is projected through
  `to_spec()` into a serializable `Spec` shipped to the browser and
  suggested client-side in WASM (`rust/lib/game/src/command/suggest.rs`).
- Never hand-roll command parsing or autocomplete. All games use the
  shared combinators.

## Suggest semantics

- `Token` suggests case-insensitive prefixes.
- `Enum` and `Player` suggest `starts_with` (prefix) matches.
- The trailing word being typed always filters candidates as a prefix,
  even when it would parse completely (`take dia` suggests only
  `Diamond`).
- Input ending in a space suggests the next position.

## `Enum::partial` in full parse

`Enum::partial` accepts unique prefixes in full parse too, so `take dia`
executes as Diamond. It can also consume a strict prefix of a word (it
parses `em` out of `emsa`), so trailing garbage is not a parse error at
the game level - see the `remaining_input` convention below.

## The `remaining_input` convention

Games' `command` implementations return `Ok` with a non-empty
`remaining_input` pass-through (mandated by
`docs/porting/GAME_PORTING.md`); downstream consumers (web, repl, fuzz)
reject non-empty remaining input. Any NEW in-process caller must check
`remaining_input` itself - see `rust/web/src/game/mod.rs:138-143` for the
web-side rejection pattern.

## Key files

- `rust/lib/game/src/command/parser/mod.rs` - the parser combinators and
  `to_spec()` projection.
- `rust/lib/game/src/command/suggest.rs` - the suggest engine.
- `rust/lib/game/src/command/doc.rs` - help-text rendering from the same
  grammar.
