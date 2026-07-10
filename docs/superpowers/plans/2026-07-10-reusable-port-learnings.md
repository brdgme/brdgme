# Reusable Port Learnings - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 10 broadly-reusable guidance items (discovered during the jaipur-2 port) to `docs/porting/GAME_PORTING.md` and `docs/authoring/GAME_DEVELOPMENT.md` without Jaipur-specific examples or document rewrites.

**Architecture:** Two documentation-only tasks targeting existing sections. All additions are new paragraphs/bullets/sections within the current document structure. No file reorganisation, no code changes, no new files.

**Tech Stack:** Markdown-only edits. Verification via `git diff` and `rg` searches; no code build or test steps.

## Global Constraints

**Source:** `docs/superpowers/specs/2026-07-10-reusable-port-learnings-design.md` (status: approved design, implementation pending).

- No Jaipur-specific examples, mechanics, test counts, command details, or Go-source file names.
- All additions are within existing sections of the two target documents only - no new files, no changes to `docs/decisions/` or any other file.
- Do not duplicate existing guidance already in the target documents.
- Use `<name>-N` (not `<name>-1`, `<name>-2`, or `<name>-<version>`) for generic paths and examples.
- Keep `lost-cities-1` only where cited as a reference port (the "primary template" sentence and the "Crate layout (mirror lost-cities-1)" heading).
- No commits, staging, or pushes - preserve all dirty work.
- No code tests are needed; verification is markdown-formatting and diff review only.
- The spec is the approved design; implement what it says, not a reinterpretation.

---

### Task 1: GAME_PORTING.md - Six guidance additions

**Files:**
- Modify: `docs/porting/GAME_PORTING.md` (6 insertions, 14 version-neutral replacements)

**Interfaces:**
- Consumes: current `docs/porting/GAME_PORTING.md` (247 lines) at HEAD.
- Produces: updated porting guide with items 1-6 from the spec.

#### Spec item 1: Log output is observable behaviour (Step 1)

Target: after line 106 (`docs/authoring/GAME_DEVELOPMENT.md` (Randomness).`), before line 107 (`2. **Game state.**`).

- [ ] Insert new paragraph at line 107 (pushing Step 2+ down):

```markdown

   **Log output is observable behaviour.** Porting means reproducing what
   players and spectators *see*, not just final state. Log messages and
   public/private information boundaries are part of the spec. If the Go
   source logs `{{b}}Market:{{_b}} ...` before a turn prompt, the Rust
   port must emit equivalent `N::Bold(...)` logs in the same order. Write
   tests that assert on `Vec<Log>` as well as game state.

```

#### Spec item 2: Translate test intent with deterministic setup (Step 8)

Target: between the existing bullet at line 176 (`command), keep doing that - construct the `Game` struct explicitly.`) and the next bullet at line 177 (`- Where the old suite is thin or absent`).

- [ ] Insert new bullet between lines 176-177:

```markdown
    - When old-state construction is mechanically impossible (the Rust
      `start` method distributes state differently and cannot produce the
      identical hand without special-casing `start` for test purposes),
      translate the *intent* of the test -- what rule is it exercising? --
      and construct a deterministic Rust scenario that exercises the same
      rule. Do not contrive a brittle preconstructed state solely to match
      Go line-for-line when a simpler setup tests the same behaviour.
```

#### Spec item 3: Validate parser combinator semantics (Step 5)

Target: after line 128 (`commands in one input line.`), before line 129 (`6. **Pub/player state + render.**`).

- [ ] Insert new paragraph at line 129 (pushing Step 6+ down):

```markdown

    **Validate parser combinator semantics against source behaviour.**
    Parser combinators (`OneOf`, `Chain2`, `Token`, `Enum`, etc.) have
    subtle semantics that may not exactly match Go text parsing. After
    building the parser, run every command form from the Go test suite
    through it and verify: correct variant is parsed, error messages
    match intent, and edge cases (leading/trailing spaces, partial match
    vs ambiguous match) match source behaviour. Any deviation that appears
    preferable must be raised with and approved by the user under the
    porting correctness rule. Add regression tests at the parser level,
    not only at the command-dispatch level.

```

#### Spec item 4: Version-neutral `<name>-N` in generic paths/examples

Replace all generic `<name>-1` and `<name>_1_*` references with `<name>-N` and `<name>_N_*`. Do NOT change reference-port citations (`lost-cities-1`/`lost-cities-2` in the versioning intro, the primary-template sentence, the crate-layout heading, and the borrow-order example's `liars-dice-2`/`no-thanks-2`) or the conceptual versioning explanation at line 35.

- [ ] Replace line 62: `rust/game/<name>-1/` -> `rust/game/<name>-N/`
- [ ] Replace line 72: `<name>_1_cli.rs` -> `<name>_N_cli.rs`
- [ ] Replace line 73: `<name>_1_http.rs` -> `<name>_N_http.rs`
- [ ] Replace line 74: `<name>_1_repl.rs` -> `<name>_N_repl.rs`
- [ ] Replace line 75: `<name>_1_fuzz.rs` -> `<name>_N_fuzz.rs`
- [ ] Replace line 182: `<name>_1_fuzz` -> `<name>_N_fuzz`
- [ ] Replace line 189: `game/<name>-1` -> `game/<name>-N`
- [ ] Replace line 190: `AS <name>-1` -> `AS <name>-N`
- [ ] Replace line 191: `<name>_1_http` -> `<name>_N_http`
- [ ] Replace line 198: `"<name>-1"` -> `"<name>-N"`
- [ ] Replace line 199: `k8s/base/game/<name>-1/` -> `k8s/base/game/<name>-N/`
- [ ] Replace line 203: `ghcr.io/brdgme/brdgme/<name>-1` -> `ghcr.io/brdgme/brdgme/<name>-N`

**Verify (post-edit):** `rg '<name>_1_' docs/porting/GAME_PORTING.md` must return zero results.
`rg '<name>-1' docs/porting/GAME_PORTING.md` must match only line 35 (the versioning-rule explanation: "even when no `<name>-1` Rust crate exists").

#### Spec item 5: Update tracking/backlog on completion (new Step 10)

Target: after line 185 (`crate references.`), before line 187 (`## Registration / deployment checklist`).

- [ ] Insert new Step 10 between lines 185-187:

```markdown
10. **Update tracking documents.** After all CI/registration steps pass
    and the port is deployed, update `docs/BACKLOG.md` to move the port
    from planned to done. If any game-specific tracking documents
    reference the port, update those too. This prevents stale backlog
    entries that make it unclear whether a game still needs porting.

```

#### Spec item 6: Report actual Cargo per-target test counts (Registration checklist)

Target: replace current item 3 (line 193, `.github/workflows/ci.yml` matrix)
with docker-bake.hcl guidance, then insert test-count item as new item 4
between lines 197 and 198. Renumber items 4-8 to 5-9.

- [ ] Replace current item 3 (lines 193-197) with:

```markdown
3. `docker-bake.hcl`: add the crate name (e.g. `<name>-N`) to the `tgt`
   array inside `target "image"`. The workspace Docker Bake configuration
   builds and tests all game crates through this matrix; without it the
   image is never built or pushed to GHCR, so later steps will reference
   an image that doesn't exist.
```

- [ ] Insert new item 4 after line 197, renumbering subsequent items:

```markdown
4. Report actual test counts, not hand-aggregated estimates. Run:
   ```
   cargo test --package <name>-N -- --list --format terse | rg ': test$' | wc -l
   ```
   Filtering `: test` excludes per-target summary lines. Use the integer
   from that command in the PR description, with a note if the ported count
   differs from the Go count and a brief reason (e.g. "Go has 14 tests;
   Rust has 15: one parser-edge-case test added per step 5 validation" or
   "Go has 12 tests; Rust has 12: 10 original + 1 assert_gamer_contract +
   1 parser regression").
```

- [ ] Renumber old item 4 (Tiltfile) -> 5
- [ ] Renumber old item 5 (k8s manifests) -> 6
- [ ] Renumber old item 6 (prod image override) -> 7
- [ ] Renumber old item 7 (Verify) -> 8
- [ ] Renumber old item 8 (fmt + clippy) -> 9

---

### Task 2: GAME_DEVELOPMENT.md - Four guidance additions

**Files:**
- Modify: `docs/authoring/GAME_DEVELOPMENT.md` (4 section insertions)

**Interfaces:**
- Consumes: current `docs/authoring/GAME_DEVELOPMENT.md` (44 lines) at HEAD.
- Produces: updated game development guide with items 7-10 from the spec.

#### Spec item 9: Neutralise unrelated randomness in focused tests (Randomness section)

Target: after line 30 (`identical game, so exact assertions are safe.`), before line 31 (blank line before `## Module boundaries`).

- [ ] Insert new paragraph after line 30:

```markdown

When a test targets one mechanic but unrelated randomness (starting-player
roll, initial shuffle, mid-game draw) complicates the scenario, neutralise
it by re-seeding the RNG field mid-test to produce a known outcome for the
unrelated part, then exercise the target mechanic. Document the seed choice
with a comment stating the desired outcome (e.g. "// seed 99 gives player 0
the first turn"). This keeps tests short and focused without scripting an
entire game to reach the scenario.
```

#### Spec item 7: `can_undo: false` for hidden/random reveals (new Commands section)

Target: between item 9's new paragraph (now ending the Randomness section) and line 32 (`## Module boundaries`).

- [ ] Insert new section after the Randomness section:

```markdown

## Commands

- `can_undo: false` is the default for commands that draw cards, roll dice,
  reveal tiles, or disclose opponent information. Only purely-deterministic
  operations with no information gain qualify for `true`. (See
  `docs/porting/GAME_PORTING.md` step 5 for context; this is repeated here
  because new-game authors may not read the porting guide.)
```

#### Spec item 10: Infallible helpers return plain values (new Error handling section)

Target: after the new Commands section, before `## Module boundaries`.

- [ ] Insert new section after Commands:

```markdown

## Error handling

Helper methods that are never expected to fail (e.g. `next_player()`,
`replenish_market()`, `end_round()`) should return the result type directly
(`Self`, `()`, `Phase`) rather than wrapping in `Result`. This avoids
`.unwrap()` noise at every call site and prevents `GameError` variants
that exist only to satisfy the type checker. Reserve `Result` for
operations where callers need to distinguish success from rejection
(command validation, `start` preconditions).
```

#### Spec item 8: Hidden-information tests structurally inspect PubState (new Tests section)

Target: after line 38 (`extract additional modules only when `lib.rs` grows unwieldy.`) (Module boundaries section end), before line 40 (`## CI verification`).

- [ ] Insert new section between Module boundaries and CI verification:

```markdown

## Tests

Games with hidden information (cards in hand, secret objectives) must have
tests that confirm `PubState` does not leak it. The pattern: call
`pub_state()` after a command that grants hidden info, then assert by
structural field inspection (not render-output grep) that the serialized
`PubState` has no `hand` field or private card field (structural field
absence), while public `hand_sizes` or count fields are present and
correct. Never assert on individual hidden cards -- such assertions only
compile when `PubState` already leaks.
```

---

### Verification (run after applying all edits)

- [ ] `git diff --stat -- docs/` shows only `docs/porting/GAME_PORTING.md` and `docs/authoring/GAME_DEVELOPMENT.md` modified, nothing else.
- [ ] `git diff --cached --stat` is empty (no staged changes).
- [ ] `rg -i '\bjaipur\b' docs/porting/GAME_PORTING.md docs/authoring/GAME_DEVELOPMENT.md` returns zero matches in the sections added/edited (existing `jaipur` references in versioning examples at `docs/porting/GAME_PORTING.md:42-43` are pre-existing and untouched).
- [ ] `rg '<name>-1' docs/porting/GAME_PORTING.md` matches only line 35 (the versioning-rule explanation: "even when no `<name>-1` Rust crate exists").
- [ ] `rg '<name>_1_' docs/porting/GAME_PORTING.md` returns zero results.
- [ ] `rg '<name>-N' docs/porting/GAME_PORTING.md` returns all generic-path lines (directory tree, checklist items, cargo command) and the `--package <name>-N` line.
- [ ] `rg 'lost-cities-1' docs/porting/GAME_PORTING.md` returns only pre-existing reference-port citations: the primary-template sentence (line 19), the crate-layout heading (line 59), and the binaries step (line 184). No new generic `lost-cities-1` was introduced.
- [ ] `rg N::Bold docs/porting/GAME_PORTING.md` returns the new item 1 paragraph (contains `N::Bold(...)`).
- [ ] `rg can_undo docs/authoring/GAME_DEVELOPMENT.md` returns the new Commands section.
- [ ] `rg PubState docs/authoring/GAME_DEVELOPMENT.md` returns the new Tests section (structural inspection pattern).
- [ ] `rg 're-seed' docs/authoring/GAME_DEVELOPMENT.md` returns the new Randomness paragraph (neutralising unrelated randomness).
- [ ] `rg 'Error handling' docs/authoring/GAME_DEVELOPMENT.md` returns the new Error handling section.
- [ ] `rg 'Vec<Log>' docs/porting/GAME_PORTING.md` returns the new item 1 paragraph.
- [ ] Markdown lint: no stray backticks, no unclosed code blocks, no mismatched headings. Run `rg '```' docs/porting/GAME_PORTING.md docs/authoring/GAME_DEVELOPMENT.md` and verify even count (each open has a matching close).
- [ ] After item 4 renumbering, the Registration checklist entries numbered 4-9 are sequential and match the original 4-8 content plus the new item 4.
- [ ] Manual visual review of `git diff -- docs/` confirms all edits match the approved spec text above.
