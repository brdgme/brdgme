# Reusable Port Learnings - Design

Date: 2026-07-10
Status: Approved design, implementation pending
Parent: #23 Rust Game Ports
Source: jaipur-2 port experience; learnings generalised to reusable guidance.

## Problem

The `GAME_PORTING.md` and `GAME_DEVELOPMENT.md` guides lack several patterns
discovered during the jaipur-2 port that apply broadly to future ports and new
game development. Rather than write a port-specific postmortem, capture the
reusable subset as targeted additions to the existing guides.

## Design

### GAME_PORTING.md additions

**1. Observable behaviour includes logs and information visibility**
(Step 1 sidebar, or new bullet under step 1)

Porting means reproducing what every player/spectator *sees*, not just final
state. Log output and public/private information boundaries are part of the
spec. If the Go source logs `{{b}}Market:{{_b}} ...` before a turn prompt, the
Rust port must emit equivalent `N::Bold(...)` logs in the same order. Write
tests that assert on `Vec<Log>` as well as state.

**2. Translate original test intent with deterministic setup**
(Step 8, under "Where old tests fixed game state directly")

Direct state construction is the preferred approach, but some Go setups are
mechanically impossible to replicate (e.g. the Go test stacks a deck to force
a specific market draw sequence, but the Rust struct's `start` method
distributes cards differently and cannot produce the identical hand without
special-casing `start` for test purposes). In those
cases, translate the *intent* of the test - what rule is it exercising? - and
construct a deterministic Rust scenario that exercises the same rule. Do not
contrive a brittle preconstructed state solely to match Go line-for-line when
a simpler setup tests the same behaviour.

**3. Validate parser-combinator semantics against source behaviour**
(Step 5 sidebar)

Parser combinators (`OneOf`, `Chain2`, `Token`, `Enum`, etc.) have subtle
semantics that may not exactly match Go text parsing. After building the
parser, run every command form from the Go test suite through it and verify:
correct variant is parsed, error messages match intent, and interesting edge
cases (leading/trailing spaces, partial match vs ambiguous match) match
source behaviour. Any deviation that appears preferable must be raised with
and approved by the user under the porting correctness rule. Add regression
tests at the parser level, not only at the command-dispatch level.

**4. Use version-neutral `<name>-N` in generic paths/examples**
(Replace all examples that use a specific versioned crate name)

Current examples use `lost-cities-1` everywhere. Replace with `<name>-N` or
`<name>-<version>` so readers don't copy-paste a real crate name into a new
port. Keep `lost-cities-1` only where it is cited as a reference port (the
"primary template" sentence and the "Crate layout (mirror lost-cities-1)"
heading are fine).

**5. Update backlog/tracking docs when port completes**
(New bullet under step 9, or a step 10)

After all CI/registration steps pass, update `docs/BACKLOG.md` to move the
port from planned to done (or remove it, depending on convention in that
file). If any game-specific tracking documents reference the port, update
those too. This prevents stale backlog entries that make it unclear whether a
game still needs porting.

**6. Report actual Cargo per-target/verified test counts**
(Step 9 sidebar or new CI checklist item)

When adding/verifying the target in `docker-bake.hcl`, do not hand-aggregate
test counts. Run the actual count:

```
cargo test --package <name>-N -- --list --format terse | rg ': test$' | wc -l
```

Filtering `: test` excludes per-target summary lines. Use the integer from
that command. If the ported test count differs from the Go test count, note
the delta in the PR description with a brief reason (e.g. "Go has 14 tests;
Rust has 15: one parser-edge-case test added per step 5 validation" or "Go
has 12 tests; Rust has 12: 10 original + 1 assert_gamer_contract + 1 parser
regression").

### GAME_DEVELOPMENT.md additions

**7. `can_undo: false` when commands reveal hidden or random information**
(New bullet or sentence under Commands guidance, if a Commands section exists;
otherwise add to a new "Commands" subsection and cross-reference step 5 of the
porting guide)

The porting guide step 5 already says `can_undo: true` only for deterministic
moves revealing no hidden info. Explicitly restate the positive case here:
`can_undo: false` is the default for commands that draw cards, roll dice,
reveal tiles, or disclose opponent information. Only purely-deterministic
operations with no information gain qualify for `true`. This is worth
repeating in the permanent guide because new-game authors (not porters) may
not read the porting guide.

**8. Hidden-information tests structurally inspect public serialized state**
(New subsection under Tests)

Games with hidden information (cards in hand, secret objectives) must have
tests that confirm `PubState` does not leak it. The test pattern: call
`pub_state()` after a command that grants hidden info, then assert by
structural field inspection (not render-output grep) that the serialized
`PubState` contains only counts, summaries, or `Unknown` markers. Example:
after dealing cards, assert that the serialized `PubState` has no `hand`
field or private card field (structural field absence), while public
`hand_sizes` or count fields are present and correct. Never assert on
individual hidden cards -- such assertions only compile when `PubState`
already leaks.

**9. Neutralise unrelated randomness in focused tests**
(Add to the Randomness section, after "Tests seed explicitly")

When a test targets one mechanic but unrelated randomness (starting-player
roll, initial shuffle, mid-game draw) changes the scenario, neutralise it by
re-seeding the RNG field mid-test to produce a known outcome for the
unrelated part, then exercise the target mechanic. Document the seed choice
with a comment stating the desired outcome (e.g. "// seed 99 gives player 0
the first turn"). This keeps tests short and focused without needing to script
an entire game to reach the scenario.

**10. Use infallible return types for game helpers that cannot fail**
(New "Error handling" subsection)

Helper methods that are never expected to fail (e.g. `next_player()`,
`replenish_market()`, `end_round()`) should return the result type directly
(`Self`, `()`, `Phase`) rather than wrapping in `Result`. This avoids
`:?`/`.unwrap()` noise at every call site and prevents `GameError` variants
that exist only to satisfy the type checker. Reserve `Result` for operations
where callers need to distinguish success from rejection (command validation,
start preconditions). The `command()` method itself always returns `Result`
because its callers (`brdgme_cmd`) expect it, but internal helpers need not
propagate that convention.

## Scope boundaries

- **Include**: broadly reusable patterns that apply to future ports and new
  game development.
- **Exclude**: Jaipur-specific mechanics (goods, camels, market, round/match
  lifecycle, camel-bonus tie-breakers), Jaipur-specific Go-source file names,
  Jaipur-specific test counts, any Jaipur command or colour details.
- **Exclude**: postmortem format, new top-level learnings document, or
  dedicated "Gotchas from Jaipur" section. Add to existing sections.
- **Exclude**: any edits to `docs/BACKLOG.md` itself (that is part of
  implementation, not this spec).
- **Do not duplicate** existing guidance already in the target documents.

## Implementation notes

- All changes are additions within existing sections, not document rewrites.
- Version-neutral examples use `<name>-N` not `<name>-2` or `jaipur-2`.
- No changes to `docs/decisions/` or any file outside `docs/porting/` and
  `docs/authoring/`.
- Reference implementation (e.g. which crate demonstrates the hidden-info test
  pattern) is determined at implementation time; not specified here.
- Follow `docs/CODING.md` style and the conventions of the target documents.
