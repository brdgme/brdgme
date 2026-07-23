# Command parser spec duplication and suggest semantics

**DECIDED 2026-07-23, IMPLEMENTED:** the autocomplete trailing-fragment fix
is committed together with its regression tests and the typed-vs-spec parser
parity tests.

## Context

Each game defines its command grammar once as typed combinators
(`rust/lib/game/src/command/parser/mod.rs`). That single grammar drives
three engines: (a) the typed-combinator `Parser::parse` used server-side to
execute commands, (b) `impl Parser for CommandSpec`
(`parser/mod.rs:813-1040`), which parses via the serializable `Spec`
projection into a `serde_json::Value`, and (c) `suggest_spec`
(`rust/lib/game/src/command/suggest.rs`), which runs client-side in WASM
per keystroke against the same `Spec`. All games use this shared infra;
none hand-roll parsing.

## Decision 1 - suggest treats the trailing word as a fragment

The last whitespace-delimited word of the input is always treated as a
still-being-typed fragment for suggestion filtering, even when it parses
completely: `take dia` suggests only `Diamond` rather than advancing to the
next position with an empty fragment (which matches everything). When the
input ends in whitespace, suggestions are for the next position.

In the `Many` arm, if the item parse succeeded but the delimiter parse
failed, filter by the whole first whitespace-delimited word - never by the
unconsumed leftover of a partial parse. `Enum::partial`'s `shared_prefix`
can consume a strict prefix of a word (it parses `em` out of `emsa`), so a
partial-parse leftover is not a trustworthy suggestion filter.

## Decision 2 - the word-boundary guard lives only in the `Many` arm

`AfterSpace::to_spec()` produces `Chain([Space, inner])`, which makes
"item parse stopped mid-word" indistinguishable from "consumed the leading
delimiter" inside the `Chain` arm. Applying the mid-word-stop guard in
`Chain` therefore breaks 14 tests. The guard belongs only in the `Many`
arm, where the item and delimiter parses are separate steps and the two
cases can be told apart.

## Decision 3 - `impl Parser for CommandSpec` is retained

Deleting the spec-parse impl as "dead duplication" was attempted and
rejected. It is the suggest engine's advancement mechanism: `Spec::parse`
is called from `suggest.rs:64` (Chain arm) and `suggest.rs:99`/`:108`
(Many arm), and suggest consumes `out.remaining` to walk the input. Those
calls are reachable from real game specs on edge/garbage input, so the impl
is load-bearing.

The duality between the typed parser and the spec parser is a deliberate
consequence of the architecture: the `Spec` must be serializable to ship
to the browser for client-side WASM suggestion, while typed parsers carry
non-serializable `Map` closures. The two engines are guarded against drift
by the typed-vs-spec parity tests (`parser/mod.rs:1307-1488`), which
assert both agree on consumption (`remaining` plus success/failure) across
real game specs.

A consumption-only advance interface (dropping the `Value` production)
remains a future option ONLY if real drift bugs appear; it would shrink
but not eliminate the second matching implementation.

## Alternatives considered

- **Full unification into one engine.** Rejected: typed parsers carry
  non-serializable `Map` closures, and the `Spec` must be serializable for
  WASM. The `Spec` is also consumed by bots, fuzz, rand_bot, and doc
  rendering, so it cannot be folded into the typed-parser path either.
- **Deleting the spec-parse impl.** Rejected: it is load-bearing as
  suggest's advancement mechanism (Decision 3).
