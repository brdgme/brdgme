# WP-03: lib-game parser mechanical fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Fix eleven mechanical defects in `brdgme_game`'s command layer: make `Enum` match ranking independent of value declaration order (lg F5), add a zero-progress termination guard to all three `Many` loops (lg F6), remove the typed `Many` early return that bypasses the `min` check and diverges from the spec impl (lg F8), stop the suggest engine offering items past a bounded `Many`'s `max` (lg F9 - which DISCHARGES c F31, sushizock-2's `roll` suggestion overrun), stop the `Int` suggestion range overflowing (lg F10), fix `doc_int`'s fake `0` minimum (lg F11) and `doc_many`'s dropped bounded max (lg F12), dedupe `Enum` suggestions (lg F18), stop `Token("")` shadowing later chain elements (lg F20), and drop the unused `combine` dependency (lg F15).

**Architecture - how `rust/lib/game`'s command layer works (read this before editing):**

- Crate `rust/lib/game`, package name **`brdgme_game`** (underscore; confirmed `rust/lib/game/Cargo.toml:2`). It is a LIBRARY consumed by all 27 game crates, `rust/web` (server + WASM frontend), `rust/lib/cmd`, `rust/lib/game_client` and `rust/lib/rand_bot`. Every behavior change here ripples; see **Caller audit** below.
- `src/command/mod.rs`: the serializable `Spec` enum (`Int`/`Token`/`Enum`/`OneOf`/`Chain`/`Many`/`Opt`/`Doc`/`Player`/`Space`, mod.rs:13-40) and `Suggestion { value, desc }` (mod.rs:7-11). Both are `Serialize + Deserialize` and cross the wire: a game service returns `command_spec` in its render response, the web server forwards it, and the Leptos WASM client calls `spec.suggest(...)` on every keystroke (`rust/web/src/components/game.rs:580`). **No task in this package changes any serialized shape** - `Spec`, `Suggestion` and `Output` keep their exact fields and variants.
- `src/command/parser/mod.rs` (1488 lines): the hand-rolled combinator library (`Token`, `Int`, `Map`, `Opt`, `Many`, `Space`, `OneOf`, `Enum`, `Doc`, `Player`, `AfterSpace`) plus a SECOND `Parser` implementation for `CommandSpec` (parser/mod.rs:813-1040). The dual implementation is deliberate (handover §6 D5); the `assert_typed_spec_parity` helper (parser/mod.rs:1319-1350) is the drift guard - it compares success/failure and `remaining`, never values.
  - Unit convention: every offset is a BYTE length (`consumed.len()` accumulated in typed `Many` at 358/370, spec `Chain` at 909, spec `Many` at 940/948, `chain_2` at `chain.rs:19`).
  - `GameError::Parse { message, expected, offset }` - `offset` is always `0` at every construction site (dead ranking machinery, lg F7, owned by WP-04). **Do not touch any `offset` value.**
- `src/command/suggest.rs` (1218 lines): `suggest_spec` walks the `Spec` recursively and returns `Vec<Suggestion>`. Two documented invariants in the module header (suggest.rs:1-12): the trailing whitespace-delimited word is always the still-being-typed fragment, and the mid-word-stop guard lives only in the `Many` arm. Runs on the WASM main thread, so a hang freezes the browser tab.
- `src/command/doc.rs` (187 lines): renders a `Spec` to `Vec<(Vec<Node>, Option<String>)>` markup for help output. Consumers: `rust/lib/cmd/src/repl.rs:95` (REPL help) and `rust/web/src/email/notify.rs:94` ("you can" lines in turn-notification emails). Purely presentational; no serialized shape.
- Tests: inline `#[cfg(test)] mod tests` only, no `tests/` dir (repo convention, `specs/notes-conventions.md`). Current counts: `parser/mod.rs` 11 tests, `suggest.rs` 85 tests, `parser/chain.rs` 1, `game.rs` 1, `rng.rs` 3 = **101 tests in `brdgme_game`**. `doc.rs` has NO test module today - Task 7 creates one.

**Tech Stack:** Rust 1.97.0, edition 2024 (let-chains available and already used in this file). Workspace root `/home/beefsack/Development/brdgme/rust`. Only crate touched: `brdgme_game` (`rust/lib/game`), plus `rust/Cargo.lock` in Task 8.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p brdgme_game`. NEVER workspace-wide builds/tests (AGENTS.md "Resource constraints" - a workspace build links ~30 binaries).
- Each task ends with `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- All 101 existing tests MUST keep passing **unmodified**. No task in this package may edit an existing test's assertions. If an existing test fails, the implementation is wrong - stop and escalate, do not adjust the test.
- **No serialized shape changes.** `Spec`, `Suggestion`, `Output`, `GameError` keep their exact variants/fields. No public function signature changes (the only signature touched anywhere is the private `shared_prefix`, and that is WP-01's change, not this package's).
- ASCII behavior of the existing in-tree grammars must be preserved except where a task explicitly documents a behavior change (Task 3 only).
- Line numbers below are LIVE-file numbers (drift-checked, see below). Tasks 1 and 2 shift `parser/mod.rs` numbering below line ~336 and Tasks 4/5/6 shift `suggest.rs` numbering; every task therefore locates its edit site by symbol/arm name as well as by line.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (it provides the Postgres/NATS containers; DB-test failures without them are pre-existing, backlog #40).

**Non-Goals (explicit fences):**

WP-04 (`lib-game parser design items`, BLOCKED-ON-DECISION D-38) owns five findings that sit in the same two files. Each is adjacent to something in this package and must NOT be absorbed:

- **lg F7 - OneOf "furthest error wins" is dead code** (parser/mod.rs:473-520 typed, 854-901 spec). Adjacent to Task 1/2 because the same `Many`/`Chain` code paths would have to propagate offsets. Do NOT implement offset propagation, do NOT change any `offset: 0`, do NOT delete the `e_consumed.cmp(&error_consumed)` ranking.
- **lg F13 - `Doc::expected` diverges typed vs spec** (parser/mod.rs:718-720 vs 1031). Adjacent to Task 7 because both concern how a `Doc` name is surfaced to users; Task 7 only touches `doc.rs` rendering, never `expected()`.
- **lg F14 - `Many::expected` diverges typed vs spec** (parser/mod.rs:402-413 vs 1025). Adjacent to Tasks 1/2 (same `impl Parser for Many` block, lines immediately below `parse`) and to Task 7 (both describe cardinality). Tasks 1/2 edit `Many::parse` ONLY; leave `Many::expected` and the spec `CommandSpec::Many` arm of `expected()` byte-identical.
- **lg F17 - case-folding differs between suggest (`to_lowercase`) and `Token::parse` (`UniCase`)** (suggest.rs:26, 36-39, 52-55, 98-101). Directly overlaps Task 6's lines: Task 6 edits the `Spec::Token` and `Spec::Enum` arms of `suggest_spec` but KEEPS `to_lowercase()` exactly as it is. Do not introduce `UniCase` into `suggest.rs`.
- **lg F19 - unbounded recursion over spec nesting, no depth guard** (suggest.rs:23, parser/mod.rs:907/945). Easily confused with Task 2: Task 2 guards ITERATION progress inside one `Many` loop (an infinite loop on a fixed spec); F19 concerns RECURSION DEPTH over nested specs (a stack overflow on a pathological spec). Do NOT add any depth counter, recursion limit, or deserialization guard.

Also out of scope:

- **WP-01 (`char/byte panic elimination`) owns lg F1, F2, F3, F4, F16 and the `shared_prefix` signature.** Do not re-fix any char/byte slicing. See "Coordination with WP-01".
- The `Spec`-is-`Deserialize`-from-untrusted-input question (WP-09 / D-36 territory).
- `Enum::parse`'s dedupe key being the lowercased value (so `"Red"` and `"red"` collapse) - observed while re-deriving, not a filed finding; see "Cross-package / newly discovered".
- Any edit to a game crate. In particular **do not touch `rust/game/sushizock-2/`**: c F31 is discharged by the lib fix in Task 4 and pinned by a lib-side test that constructs sushizock's spec locally.

**Snapshot drift:** **None.** `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/lib/game /home/beefsack/Development/brdgme/rust/lib/game` produced no output (exit 0), and so did the same diff for `rust/game/sushizock-2` (verified 2026-07-25 against snapshot commit `f8763a5`). Every finding line number is valid against the live files, and the live line numbers cited in this spec are identical to the findings' snapshot numbers.

**Coordination with WP-01 (`specs/WP-01-char-byte-panic-elimination.md`):**

| Site | WP-01 | WP-03 | Disjoint? |
|---|---|---|---|
| `Space::parse` (parser/mod.rs:430-444) | Task 1 rewrites line 431 | untouched | yes |
| `Token::parse` (parser/mod.rs:48-64) | Task 2 rewrites the whole body | untouched | yes |
| `shared_prefix` (parser/mod.rs:578-593) | Task 3 changes the return type to `(usize, usize)` | untouched | yes |
| `Enum::parse` loop (parser/mod.rs:600-637) | Task 3 rewrites the whole method; explicitly leaves the `matching > match_len` replace-vs-push asymmetry to WP-03 | **Task 3 rewrites ONLY the final ranking `if` block** | overlapping lines, disjoint intent - **ORDER MATTERS** |
| `Int::parse` (parser/mod.rs:123-172) | Task 4 swaps `consumed_count` for `consumed_len` | untouched (WP-03 Task 5 touches `Spec::Int` in **suggest.rs**, a different file/function) | yes |
| `Many::parse` typed + spec (parser/mod.rs:336-400, 918-973) | untouched | Tasks 1, 2 | yes |
| `suggest.rs` all arms | untouched | Tasks 2, 4, 5, 6 | yes |
| `doc.rs`, `Cargo.toml` | untouched | Tasks 7, 8 | yes |
| `lib/markup`, `red7-1`, `docs/CODING.md` | WP-01 Tasks 5-7 | untouched | yes |

**Landing order: WP-01 lands FIRST, then WP-03.** Only WP-03 Task 3 depends on it (it edits lines WP-01 rewrites); Tasks 1, 2, 4, 5, 6, 7, 8 are textually disjoint from WP-01 and can land in any order. Task 3 starts with a hard precondition check on `shared_prefix`'s signature and carries a documented contingency if WP-01 has not landed.

**Also note (report to the Lead, do not "fix" here):** WP-01's Non-Goals paragraph (WP-01 spec line 35) assigns lg F13/F14 to WP-03 and lg F18/F20 to WP-04. That contradicts `planning/work-packages.md` (WP-03 = F5, F6, F8, F9, F10, F11, F12, F15, F18, F20, c F31; WP-04 = F7, F13, F14, F17, F19). **`work-packages.md` is authoritative** and this spec follows it: F18/F20 are Task 6 here, F13/F14 are fenced out to WP-04.

**Re-derivation notes (every recommendation re-validated by reading live source):**

- **lg F5** (parser/mod.rs:626): re-traced both orderings by hand. `values = ["abc","ab"]`, input `"ab"`: `"abc"` matches 2 (partial) and sets `match_len = 2`; `"ab"` then matches 2 fully but `matching > match_len` is false so it is PUSHED, giving `matched = ["abc","ab"]` -> spurious *"matched ab and abc, more input is required"*. With `["ab","abc"]` it works. Confirmed. **But the finding's framing is incomplete:** the code comment at 608-609 claims "a shorter full match will happen over a longer partial match", and the current code delivers that only when the short full value is declared first (`["ab","abcd"]` + `"abcx"` -> `"ab"`, remaining `"cx"`; reversed -> `"abcd"`, remaining `"x"`). Making THAT rule order-independent would entrench a wart rather than a regression - it is today's behavior in one of the two orderings - but a bad one: with player names `["Bo","Bobby"]` and input `"bobb"` it would select `"Bo"`, consume 2 bytes and leave `"bb"` for the rest of the command to choke on. Task 3 therefore adopts the order-independent rule **(longest match wins; a full match breaks ties against equal-length partials)** and updates the stale comment.

  **LEAD RULING on this adjustment (binding):** adopting longest-wins is APPROVED, but be clear about what it is: a *deliberate, user-visible policy change* to how prefix-overlapping `Enum`/`Player` values resolve, going beyond the finding's own recommendation ("track full matches separately and prefer them"), which would have preserved the documented full-match-first policy. It is approved because (a) it is order-independent, which is all lg F5 requires; (b) the only in-tree proper-prefix value list is cathedral-2's loc enum, whose behavior is provably unchanged (traced in the Task 3 edge cases); (c) the only behavioral delta is at runtime for player names that prefix each other, where consuming the longer name is the answer the player meant. The implementer must NOT silently reinterpret this as a mechanical refactor: the rewritten comment IS the record of the new policy. If the policy is ever contested, it is a parser-convention question and belongs to **WP-04 / D-38** (which already owns case-folding and `expected()` conventions) - reopen it there, not by re-editing this task. Verified against every existing assertion in `test_enum_works` (see Task 3).
- **lg F6** (parser/mod.rs:353, 918-954, suggest.rs:111-144): all three loops verified guard-free. Zero-width success is constructible three ways today: `Opt` always succeeds (parser/mod.rs:259-270), `Token::new("")` always succeeds (`input.len() < 0` is false, `UniCase("") == UniCase("")`), and `CommandSpec::Chain(vec![])` succeeds consuming nothing (902-917). With `delim: None` (or a zero-width delim) the offset never advances. In the typed and spec loops this grows the value `Vec` without bound when `max` is `None`; in suggest it spins forever with no allocation. Confirmed.
- **lg F8** (parser/mod.rs:342-350): confirmed - the early return skips the `min` check at 382-394, so typed `Many { min: Some(2), max: Some(1) }` returns `Ok(vec![])` while the spec impl returns `Err`. **The finding's recommendation ("drop the early return; the loop plus min check already handle these configs identically") is WRONG as written:** the typed loop checks `max` only AFTER pushing (`parsed.len() == max`, line 371-375), so with `max = Some(0)` it would never break and would parse items unboundedly. The fix must MOVE the `max` check to the top of the loop (`parsed.len() >= max`), mirroring the spec impl's loop head at 929-933. Adjusted.
- **lg F9 + c F31** (suggest.rs:109): confirmed - `Spec::Many { spec, delim, .. }` discards `min`/`max`. Read `rust/game/sushizock-2/src/command.rs:38-50`: `roll_parser(max)` = `Map(Chain2(Doc("roll",…,Token("roll")), AfterSpace(Doc("dice",…, Many::bounded_spaced(Int::bounded(1, max as i32), 1, max)))))` with `max = self.rolled_dice.len()` (command.rs:24). `Many::to_spec` (parser/mod.rs:415-422) propagates `min: Some(1), max: Some(max)` faithfully. Note precisely what is and is not wrong: the die-number VALUES are already bounded (`Spec::Int` suggest is capped by `max`), what is unbounded is the ITEM COUNT - after all 5 dice have been typed, suggest still offers a 6th number that `Many::parse` will reject. Fixing the item count in lib/game therefore fully discharges c F31 with no crate-side change.
- **lg F10** (suggest.rs:87): confirmed - `start + 4` with `min: Some(i32::MAX - 3..=i32::MAX)` overflows: debug panic, release wrap to a negative `end` (empty range). `start.saturating_add(4)` is correct and cannot change any in-range behavior.
- **lg F11** (doc.rs:51): confirmed - the `(min, Some(max))` arm renders `min.unwrap_or(0)`, so `Int { min: None, max: Some(5) }` documents as `0-5` while `Int::parse` accepts negatives and `Int::expected_output` (parser/mod.rs:113) says "number 5 or lower". Reachability re-verified: `rust/game/for-sale-2/src/command.rs:41-46` builds exactly `Int { min: None, max: Some(max) }`, and `Spec::doc()` feeds `repl.rs:95` and `notify.rs:94`.
- **lg F12** (doc.rs:134, 139): confirmed, and verification's stronger reachability claim re-checked: `Many::bounded_spaced` is used by `sushi-go-2/src/command.rs:42` (`1, 2`), `sushizock-2/src/command.rs:47` (`1, max`) and `roll-through-the-ages-2/src/command.rs:317` (`1, max`), all with `min = Some(1)`, so the `(Some(1), _)` arm renders `thing+` and the bounded max is silently dropped in REPL help and notification emails today.
- **lg F15** (Cargo.toml:12): confirmed - `grep -rn combine rust/lib/game/` matches ONLY `Cargo.toml:12`. `rust/Cargo.lock` lists `combine` under `brdgme_game` (lock line 839) and `brdgme_markup` (line 872); markup is the real consumer, so the crate stays in the lock, only the `brdgme_game` edge is removed. `lib/game` has no `bin/`, `benches/`, `examples/` or `tests/` directories, so `Cargo.toml` is the only place to check.
- **lg F18** (suggest.rs:37): confirmed for the `Enum` arm - `Enum::parse` dedupes via a `HashSet` keyed on the lowercased value (parser/mod.rs:612-619) while the suggest `Enum` arm maps every value verbatim. **The finding's optional "and optionally after OneOf concatenation" is declined:** `OneOf` branches attach independent `Doc` descriptions (suggest.rs:50-69), so two branches yielding the same value are NOT interchangeable suggestions, and `OneOf::parse` has no dedupe either - there is no parser asymmetry to mirror. Task 6 dedupes the `Enum` arm only.
- **lg F20** (suggest.rs:26): confirmed - `"".starts_with("")` is true, so `Spec::Token("")` on empty input returns `Suggestion { value: "" }`, and the `Chain` arm returns the first non-empty result (suggest.rs:74-76), shadowing every later element. Guarding the arm is correct; the `Token("")` PARSE behavior stays as-is (changing it is not a finding).

**Caller audit (library crate - who breaks):**

- **No public API signature changes anywhere in this package.** `grep -rn` across `rust/` confirms the only cross-crate entry points touched behaviorally are `Parser::parse` for `Many`/`Enum` (Tasks 1-3), `Spec::suggest` (Tasks 2, 4, 5, 6) and `Spec::doc` (Task 7) - all keep their signatures and return types. **Zero call sites need editing. No game crate, `web`, `bot`, `lib/cmd`, `lib/game_client` or `lib/rand_bot` file changes as part of this package.**
- Behavioral blast radius, enumerated:
  - Tasks 1, 2 (`Many`): every game using `Many::any_spaced`/`some_spaced`/`bounded_spaced` (splendor-2, jaipur-2, sushi-go-2, sushizock-2, roll-through-the-ages-2, …). All in-tree specs use a `Space` delimiter and a non-zero-width item, so neither the progress guard nor the moved `max` check can fire for them; the only observable changes are for degenerate configs no in-tree crate constructs (`max == 0`, `max < min`, zero-width items).
  - Task 3 (`Enum` ranking): every `Enum::partial`/`Enum::exact` and every `Player`. `exact` enums are unaffected (only full matches ever enter the candidate set). For `partial` enums the change is observable ONLY when one value is a proper prefix of another in the same enum. Audited all in-tree partial-`Enum` value lists (`grep -rn "Enum::partial" rust/game`, then read each value source): acquire-1 `CORPS` (corp.rs:24-32), jaipur-2 `Good::all_goods` (lib.rs:49-59), starship-catan-1 `Module::ALL`/`Resource::GOODS`/`BUILDABLES` (card.rs:26-39, 93-100), roll-through-the-ages-2 goods/developments/monuments (good.rs:42-50, development.rs:59+, monument.rs:38-83), age-of-war-2 castle names (castle.rs:264-385), battleship-2 `Ship::all`/`Direction::all` (lib.rs:64-72, 118-125), splendor-2 `GEMS`+Gold (command.rs:157-165), sushizock-2 `TileType` (command.rs:70, 93), lords-of-vegas-1 `CASINOS` (command.rs:161), cathedral-2 loc/dir names (command.rs:106, 118). **The only proper-prefix pair in-tree is cathedral-2's loc enum (`"A1"` ⊂ `"A10"`, from `Loc::to_key`, loc.rs:113-115)**, and `all_locs()` (loc.rs:149-157) declares `A1` before `A10`, which is already the post-fix outcome for every input - traced in Task 3. So no in-tree grammar changes behavior; the remaining exposure is `Player`, whose values are runtime player names, where the new rule is strictly better (see Task 3's `"Bo"`/`"Bobby"` trace).
  - Task 7 (`doc`): output text seen in `lib/cmd/src/repl.rs:95` help and `web/src/email/notify.rs:94` email lines. No test in the workspace asserts doc output (`grep -rn "\.doc()" --include=*.rs rust/` returns exactly those two call sites), so nothing breaks; the rendered text simply becomes correct (`#-5` instead of `0-5`, `thing(1-2)` instead of `thing+`).
  - Task 8 (`combine`): compile-only; nothing references it.
- Serialized-shape proof: `Spec` and `Suggestion` (command/mod.rs:7-40) are untouched. The consumers that deserialize them are `rust/web` (game-service render responses -> `command_spec`, then `spec.suggest` at `rust/web/src/components/game.rs:580`) and `rust/lib/cmd`/`lib/game_client` (service request/response types). There is no TypeScript mirror of `Spec` - `grep -rln "command_spec" --include=*.ts --include=*.tsx rust/web` returns nothing; the frontend is Leptos/WASM Rust using the same crate.

---

### Task 1: typed `Many` - move the `max` check into the loop, delete the min-bypassing early return (lg F8, minor)

**Problem (restated):** `parser/mod.rs:342-350`:

```rust
        if let Some(max) = self.max
            && (max == 0 || max < self.min.unwrap_or(0))
        {
            return Ok(Output {
                value: parsed,
                consumed: &input[..0],
                remaining: input,
            });
        }
```

returns `Ok(vec![])` for `max == 0` and for `max < min`, never reaching the `min` check at 382-394. The spec impl (`CommandSpec::Many`, 918-973) has no such early return: its loop head breaks on `values.len() >= max` and then the `min` check fails. So typed `Many { min: Some(2), max: Some(1) }` SUCCEEDS while the same spec FAILS - exactly the drift `assert_typed_spec_parity` exists to catch, uncovered today.

**Fix (re-derived, ADJUSTED from the finding):** deleting the early return alone is wrong - the typed loop checks `max` only after pushing (`parsed.len() == max`, lines 371-375), so `max == 0` would parse unboundedly. Delete the early return AND move the `max` test to the top of the loop as `parsed.len() >= max`, mirroring the spec impl's head (929-933). The post-push check then becomes redundant and is removed. For every non-degenerate config the consumption is byte-identical: the break happens before the next delimiter parse either way.

**Edge cases:**
- `max = Some(0)`, `min = None` -> loop breaks immediately, `Ok(vec![])`, `consumed = ""`, `remaining = input` (same as today, now via the loop).
- `max = Some(0)`, `min = Some(1)` -> `Ok` today, `Err` after (matches spec impl).
- `max = Some(1)`, `min = Some(2)` -> `Ok(vec![])` today; after: one item is parsed, the loop breaks at the top, the min check fails -> `Err` (matches spec impl, which also parses one item first).
- `max = Some(n)` reached exactly: the trailing delimiter is NOT consumed, before and after (`many_parser_works` pins this: `min=5, max=5`, input `"3, 4, 5, 6, 7, 8, 9, 10"` -> `remaining = ", 8, 9, 10"`).
- `max = None` -> the new guard never fires; unchanged.
- `first`/delimiter handling is untouched; with `max = Some(0)` the loop breaks before `first` is ever read.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`impl Parser for Many`'s `parse`, lines 336-400)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/lib/game/src/command/parser/mod.rs` (place it directly after `many_parser_works`, which ends at line 1211):

```rust
    #[test]
    fn many_degenerate_bounds_match_the_spec_impl() {
        // lg F8: the typed impl used to return Ok(empty) for `max == 0` or
        // `max < min` via an early return that skipped the min check, while
        // the spec impl broke out of its loop and failed the min check. Same
        // grammar, different success/failure - the exact drift the parity
        // helper guards against.
        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: Some(2),
            max: Some(1),
            delim: Some(Space {}),
        };
        parser
            .parse("1 2", &[])
            .expect_err("min 2 with max 1 must fail the min check");
        assert_typed_spec_parity(&parser, &["1 2", "1", ""]);

        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: Some(1),
            max: Some(0),
            delim: Some(Space {}),
        };
        parser
            .parse("1 2", &[])
            .expect_err("min 1 with max 0 must fail the min check");
        assert_typed_spec_parity(&parser, &["1 2", "1", ""]);

        // max == 0 without a min still succeeds consuming nothing.
        let parser: Many<Int, Space> = Many {
            parser: Int::any(),
            min: None,
            max: Some(0),
            delim: Some(Space {}),
        };
        let out = parser
            .parse("1 2", &[])
            .expect("max 0 with no min must succeed with an empty value");
        assert!(out.value.is_empty());
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "1 2");
        assert_typed_spec_parity(&parser, &["1 2", ""]);
    }
```

- [ ] Run: `cargo test -p brdgme_game many_degenerate_bounds_match_the_spec_impl`. Expected: **FAIL** on the first assertion - `min 2 with max 1 must fail the min check` (the typed parser returns `Ok`).
- [ ] Implement. In `rust/lib/game/src/command/parser/mod.rs`, replace the whole body of `Many::parse` (lines 336-400) with:

```rust
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let mut parsed: Self::T = vec![];
        let mut first = true;
        let mut offset = 0;
        'outer: loop {
            // Checked at the top of the loop exactly like the spec impl
            // (`CommandSpec::Many`), so degenerate configs (`max == 0`, or
            // `max < min`) fall through to the min check below instead of
            // returning early with an empty Ok (lg F8).
            if let Some(max) = self.max
                && parsed.len() >= max
            {
                break 'outer;
            }
            let mut inner_offset = offset;
            if !first {
                if let Some(d) = self.delim.as_ref() {
                    match d.parse(&input[offset..], names) {
                        Ok(Output { consumed, .. }) => inner_offset += consumed.len(),
                        Err(_) => break 'outer,
                    };
                }
            } else {
                first = false;
            }
            match self.parser.parse(&input[inner_offset..], names) {
                Ok(Output {
                    value, consumed, ..
                }) => {
                    parsed.push(value);
                    offset = inner_offset + consumed.len();
                }
                Err(_) => {
                    break 'outer;
                }
            };
        }
        if let Some(min) = self.min
            && parsed.len() < min
        {
            return Err(GameError::Parse {
                message: Some(format!(
                    "expected at least {} items but could only parse {}",
                    min,
                    parsed.len()
                )),
                expected: vec![],
                offset: 0,
            });
        }
        Ok(Output {
            value: parsed,
            consumed: &input[..offset],
            remaining: &input[offset..],
        })
    }
```

- [ ] Run: `cargo test -p brdgme_game many_degenerate_bounds_match_the_spec_impl` - PASS. Then `cargo test -p brdgme_game` - all 102 tests PASS (`many_parser_works` pins the `max`-reached consumption, the three parity tests pin real grammars).
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `fix(parser): typed Many honours min for degenerate max configs (lg F8, WP-03)`

NOTE: this task changes `parser/mod.rs` line numbering below ~line 336 by about -8. Task 2 locates its edits by symbol.

---

### Task 2: zero-progress guards in all three `Many` loops (lg F6, major)

**Problem (restated):** none of the three `Many` loops enforces that an iteration consumes input:

1. typed `Many::parse` (`parser/mod.rs`, post-Task-1 body above): `offset = inner_offset + consumed.len()`; if the item consumes 0 bytes and there is no delimiter (or a zero-width one), `offset` never advances and the loop pushes values forever - unbounded `Vec` growth when `max` is `None`.
2. spec `CommandSpec::Many` (`parser/mod.rs:918-973`, pre-Task-1 numbering): same shape via `consumed_len`/`remaining`.
3. `suggest_spec`'s `Spec::Many` arm (`suggest.rs:109-145`): the two `continue` paths (123-124 and 136-137) re-enter the loop with `rem` unchanged - a hang on the WASM main thread, freezing the browser tab.

Zero-width success is constructible today: `Opt` always succeeds, `Token::new("")` always succeeds, `CommandSpec::Chain(vec![])` succeeds consuming nothing. No in-tree game builds such a spec (all use `*_spaced` helpers), but `Spec`'s fields are public and it derives `Deserialize`, so one buggy spec hangs a game-service thread or the browser.

**Fix (re-derived, as the finding recommends):** in each loop, compare the input position before and after an iteration and break/stop when nothing was consumed. The value produced by that final zero-width iteration IS kept (the item parser did succeed), which keeps the typed and spec impls agreeing and lets a `min: Some(1)` zero-width `Many` succeed. In suggest, "stop" means falling back to the same "filter by the current fragment" behavior the `Err` arm already uses. Also document the invariant on the `Many` struct, per the finding.

**Edge cases:**
- Non-zero-width items (every in-tree grammar): `progressed` is always true, no behavior change.
- Zero-width item + `Space` delimiter: the delimiter consumes, so the iteration progresses; the loop still terminates because the delimiter eventually fails at end of input. Unchanged.
- Zero-width DELIMITER with a zero-width item: `step == 0` -> break. Covered by the same guard (the guard measures delimiter + item together).
- `max = Some(n)` with zero-width items: the Task-1 loop head breaks at `n` items; the progress guard breaks earlier (after 1). The guard is checked after the push, so exactly one item is produced.
- Typed vs spec parity for the degenerate shape: both produce exactly one item and consume nothing. Pinned by an `assert_typed_spec_parity` call plus explicit value assertions.
- suggest with a zero-width item: returns the fragment-filtered suggestions of the item spec (for `Chain(vec![])` that is `vec![]`), never loops.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`pub struct Many` doc comment; `Many::parse`; the `CommandSpec::Many` arm of `impl Parser for CommandSpec`)
- Modify: `rust/lib/game/src/command/suggest.rs` (the `Spec::Many` arm of `suggest_spec`)
- Test: inline `mod tests` in both files

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/lib/game/src/command/parser/mod.rs`, after `many_degenerate_bounds_match_the_spec_impl`:

```rust
    #[test]
    fn many_zero_width_item_terminates() {
        // lg F6: `Opt` always succeeds consuming nothing, so with no
        // delimiter every iteration made zero progress and the loop pushed
        // values forever (unbounded Vec growth with max = None).
        let parser: Many<Opt<Token>, Space> = Many {
            parser: Opt::new(Token::new("x")),
            min: None,
            max: None,
            delim: None,
        };
        let out = parser
            .parse("y", &[])
            .expect("a zero-width Many must terminate and succeed");
        assert_eq!(out.value, vec![None]);
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "y");
        assert_typed_spec_parity(&parser, &["y", ""]);
    }

    #[test]
    fn spec_many_zero_width_item_terminates() {
        // lg F6: `Chain(vec![])` succeeds consuming nothing, so the spec
        // Many loop had the same unbounded-growth defect as the typed one.
        let spec = CommandSpec::Many {
            spec: Box::new(CommandSpec::Chain(vec![])),
            min: None,
            max: None,
            delim: None,
        };
        let out = spec
            .parse("y", &[])
            .expect("a zero-width spec Many must terminate and succeed");
        assert_eq!(out.consumed, "");
        assert_eq!(out.remaining, "y");
        assert_eq!(
            out.value,
            serde_json::Value::Array(vec![serde_json::Value::Array(vec![])])
        );
    }
```

  and add to `mod tests` in `rust/lib/game/src/command/suggest.rs`, at the end of the `// --- Many ---` section (after `many_with_space_delimiter_suggests_after_consumption`, which ends at line 567):

```rust
    #[test]
    fn many_zero_width_item_suggest_terminates() {
        // lg F6: an item spec that succeeds consuming nothing (an empty
        // Chain) spun this loop forever - on the WASM main thread, which
        // freezes the browser tab.
        let spec = Spec::Many {
            spec: Box::new(Spec::Chain(vec![])),
            min: None,
            max: None,
            delim: None,
        };
        assert!(spec.suggest("y", &[]).is_empty());
    }
```

- [ ] Run the red check. These tests HANG pre-fix (the parser ones also allocate; peak RSS grows by roughly 100-500 MB over a few seconds, which is why the run is time-boxed). Compile first, then run with a timeout:

```
cargo test -p brdgme_game --no-run
timeout 5 cargo test -p brdgme_game zero_width
```

  Expected: the command is killed by `timeout` (exit status **124**), with no test result lines printed for the three `zero_width` tests. That hang IS the red signal. Do NOT run these tests without `timeout` before the fix.
- [ ] Implement (1/4) - document the invariant. In `rust/lib/game/src/command/parser/mod.rs`, above `pub struct Many` (line 286), add:

```rust
/// Repetition combinator.
///
/// Progress invariant: every iteration of the parse loop must consume at
/// least one byte of input (via the delimiter or the item). A parser that
/// succeeds consuming nothing (`Opt`, `Token::new("")`, an empty `Chain`)
/// would otherwise loop forever, so both this impl and the `Spec::Many`
/// impl stop as soon as an iteration makes no progress.
```

- [ ] Implement (2/4) - typed loop. In `Many::parse`, replace the success arm of the item match (the `Ok(Output { value, consumed, .. })` arm installed in Task 1) with:

```rust
                Ok(Output {
                    value, consumed, ..
                }) => {
                    parsed.push(value);
                    let new_offset = inner_offset + consumed.len();
                    // Progress invariant (see the struct doc comment): stop
                    // when neither the delimiter nor the item consumed
                    // anything, otherwise this loop never ends (lg F6).
                    let progressed = new_offset > offset;
                    offset = new_offset;
                    if !progressed {
                        break 'outer;
                    }
                }
```

- [ ] Implement (3/4) - spec loop. In the `CommandSpec::Many` arm of `impl Parser for CommandSpec` (search for `CommandSpec::Many {`), replace the inner item match:

```rust
                    match spec.parse(inner_remaining, names) {
                        Ok(out) => {
                            values.push(out.value);
                            consumed_len += delim_len + out.consumed.len();
                            remaining = out.remaining;
                            first = false;
                        }
                        Err(_) => break,
                    }
```

  with:

```rust
                    match spec.parse(inner_remaining, names) {
                        Ok(out) => {
                            let step = delim_len + out.consumed.len();
                            values.push(out.value);
                            consumed_len += step;
                            remaining = out.remaining;
                            first = false;
                            // Progress invariant, see the typed `Many` impl:
                            // a zero-width iteration would loop forever.
                            if step == 0 {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
```

- [ ] Implement (4/4) - suggest loop. In `rust/lib/game/src/command/suggest.rs`, replace the entire `Spec::Many { spec, delim, .. } => { ... }` arm (lines 109-145) with:

```rust
        Spec::Many { spec, delim, .. } => {
            let mut rem = remaining;
            loop {
                match spec.parse(rem, names) {
                    Ok(out) => {
                        let after_item = out.remaining;
                        if after_item.is_empty() {
                            // A fully-parsed trailing word is still the fragment
                            // being typed, so filter by it rather than advancing.
                            return suggest_spec(spec, rem, names);
                        }
                        if let Some(d) = delim {
                            match d.parse(after_item, names) {
                                Ok(d_out) => {
                                    if d_out.remaining.len() == rem.len() {
                                        // Progress invariant (lg F6): neither the
                                        // item nor the delimiter consumed
                                        // anything, so looping would hang the
                                        // WASM main thread.
                                        return suggest_spec(spec, rem, names);
                                    }
                                    rem = d_out.remaining;
                                    continue;
                                }
                                Err(_) => {
                                    // The item parse may have stopped mid-word
                                    // (e.g. an Enum prefix inside a longer word);
                                    // the whole first word is the fragment, not
                                    // the unconsumed leftover.
                                    let fragment = rem.split_whitespace().next().unwrap_or("");
                                    return suggest_spec(spec, fragment, names);
                                }
                            }
                        } else {
                            if after_item.len() == rem.len() {
                                // Progress invariant (lg F6), delimiter-free case.
                                return suggest_spec(spec, rem, names);
                            }
                            rem = after_item;
                            continue;
                        }
                    }
                    Err(_) => {
                        return suggest_spec(spec, rem, names);
                    }
                }
            }
        }
```

  (`rem`, `after_item` and `d_out.remaining` are all suffixes of the same input, so comparing `len()` is a valid position comparison.)
- [ ] Run: `timeout 60 cargo test -p brdgme_game zero_width` - all three tests PASS in milliseconds. Then `cargo test -p brdgme_game` - all 105 tests PASS.
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs rust/lib/game/src/command/suggest.rs` ; message: `fix(parser): zero-progress guards in all three Many loops (lg F6, WP-03)`

---

### Task 3: `Enum` match ranking independent of declaration order (lg F5, major) - REQUIRES WP-01 Task 3

**Problem (restated):** `parser/mod.rs:626-636`:

```rust
            if matching > 0 && matching >= match_len && (!full_match || matching == v_len) {
                if matching == v_len {
                    full_match = true
                }
                if matching > match_len {
                    matched = vec![v];
                    match_len = matching;
                } else {
                    matched.push(v);
                }
            }
```

A candidate that ties the current `match_len` is APPENDED, so with `values = ["abc","ab"]` and input `"ab"`, `"abc"` (partial, 2) is recorded first and `"ab"` (full, 2) is appended -> `matched.len() == 2` -> the spurious error *"matched ab and abc, more input is required to uniquely match one"*. Reverse the declaration order and it works. A value that is a prefix of an earlier-declared value cannot be selected at its own exact length.

**Fix (re-derived, ADJUSTED):** rank candidates by the ordered key **(matched length, then full-match)**, always replacing on a strictly better key, appending only on an exact tie, and ignoring anything worse. That is order-independent and it is what the finding asks for ("a full match replaces same-length partials").

Deliberately NOT adopted: the stale comment at lines 608-609 ("a shorter full match will happen over a longer partial match"). Making THAT rule order-independent would be a regression - with player names `["Bo","Bobby"]` and input `"bobb"`, `"Bo"` (full, 2) would displace `"Bobby"` (partial, 4), consume 2 bytes and leave `"bb"` for the rest of the command to choke on. Today that misbehavior only occurs in one of the two declaration orders; under the new rule it never occurs, because a longer match always wins and the full-match flag only breaks ties. The comment is rewritten to state the implemented rule.

**Precondition - WP-01 Task 3 must already be merged.** WP-01 rewrites this exact method (`shared_prefix` returns `(input_bytes, value_bytes)` and the loop computes `let full = v_matching == v_str.len();`). The snippets below are written against the POST-WP-01 source.

**Edge cases (each traced by hand against the existing tests):**
- `test_enum_works` (`["fart","cheese","dog","bacon","farty"]`): input `"fart"` -> `fart` full 4 beats `farty` partial 4 on the tie-break -> `"fart"` ✔; `"farty"` -> `farty` full 5 beats `fart` full 4 on length ✔; `"far"` -> two partials of 3, exact tie -> ambiguity `Err` ✔; `"c"` -> single partial ✔; `"DoGlog"` -> `dog` full 3, `consumed = "DoG"`, `remaining = "log"` ✔.
- `matching == 0`: skipped before ranking (same as the old `matching > 0`).
- Empty value `""`: `matching` is 0 -> skipped; an empty value can never be selected (unchanged).
- `exact` enums: only full matches enter the candidate set, so ranking degenerates to "longest full match wins"; two distinct full matches of equal length are impossible after the lowercase dedupe. Unchanged behavior.
- cathedral-2 locs (`A1` before `A10`, the only in-tree proper-prefix pair): input `"a1"` -> `A1` full 2 ties `A10` partial 2, full wins -> `A1` (same as today); input `"a1 down"` -> identical, the space stops both at 2 -> `A1`; input `"a10"` -> `A10` full 3 wins on length -> `A10` (same as today). No behavior change.
- `Player` with `["Bo","Bobby"]`: `"bobb"` -> `Bobby` partial 4 wins on length (today: order-dependent); `"bobby"` -> `Bobby` full 5 wins; `"bo"` -> `Bo` full 2 ties `Bobby` partial 2, full wins -> `Bo`. All three are the desirable answers.
- Duplicate-value dedupe (`searched`) is untouched, and so are the three `matched.len()` result arms and both `GameError::Parse` payloads.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`Enum::parse` - the comment at 608-609 and the ranking `if` block; locate by symbol, Tasks 1-2 shifted the numbering)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Precondition check: read `shared_prefix` in `rust/lib/game/src/command/parser/mod.rs`. It MUST have the signature `fn shared_prefix(input: &str, value: &str) -> (usize, usize)` and `Enum::parse` MUST contain `let full = v_matching == v_str.len();`. If instead `shared_prefix` returns a bare `usize`, WP-01 Task 3 has not landed: **stop and report to the coordinator.** (Contingency, only if the coordinator directs you to proceed regardless: the pre-WP-01 loop has no `full` binding - add `let full = matching == v_len;` immediately after the `let matching = shared_prefix(&input_lower, &v_str);` line and apply the rest of this task verbatim. Everything else is identical.)
- [ ] Write the failing tests. Add to `mod tests` in `rust/lib/game/src/command/parser/mod.rs`, after `test_enum_works` (ends at line 1287 pre-Task-1):

```rust
    #[test]
    fn enum_full_match_wins_ties_in_either_declaration_order() {
        // lg F5: a full match that ties the current best length used to be
        // appended instead of replacing the partial, so ["abc", "ab"] with
        // input "ab" produced a spurious "matched ab and abc" ambiguity
        // error while ["ab", "abc"] parsed fine.
        for values in [vec!["abc", "ab"], vec!["ab", "abc"]] {
            let parser = Enum::partial(values.clone());
            assert_eq!(
                Output {
                    value: "ab",
                    consumed: "ab",
                    remaining: "",
                },
                parser
                    .parse("ab", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
            assert_eq!(
                Output {
                    value: "abc",
                    consumed: "abc",
                    remaining: "",
                },
                parser
                    .parse("abc", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
        }
    }

    #[test]
    fn enum_longest_match_wins_in_either_declaration_order() {
        // lg F5, second half: the ranking key is (matched length, then full
        // match), so the longer partial match wins regardless of which value
        // was declared first. Pre-fix ["ab", "abcd"] consumed only "ab" while
        // ["abcd", "ab"] consumed "abc" for the same input.
        for values in [vec!["ab", "abcd"], vec!["abcd", "ab"]] {
            let parser = Enum::partial(values.clone());
            assert_eq!(
                Output {
                    value: "abcd",
                    consumed: "abc",
                    remaining: "x",
                },
                parser
                    .parse("abcx", &[])
                    .unwrap_or_else(|e| panic!("values {:?}: {}", values, e)),
            );
        }
    }

    #[test]
    fn player_name_prefix_of_another_name_parses_longest() {
        // lg F5 reachability: Player builds Enum::partial from player names,
        // which are user-chosen, so prefix pairs are ordinary. Both orderings
        // must resolve the same way.
        for names in [
            vec!["Bo".to_string(), "Bobby".to_string()],
            vec!["Bobby".to_string(), "Bo".to_string()],
        ] {
            let parser = Player {};
            let bobby = names.iter().position(|n| n == "Bobby").unwrap();
            let bo = names.iter().position(|n| n == "Bo").unwrap();
            assert_eq!(
                bobby,
                parser
                    .parse("bobb", &names)
                    .unwrap_or_else(|e| panic!("names {:?}: {}", names, e))
                    .value,
                "a longer partial name match must win: {:?}",
                names
            );
            assert_eq!(
                bo,
                parser
                    .parse("bo", &names)
                    .unwrap_or_else(|e| panic!("names {:?}: {}", names, e))
                    .value,
                "an exact full name match must win ties: {:?}",
                names
            );
        }
    }
```

- [ ] Run: `cargo test -p brdgme_game enum_full_match_wins_ties_in_either_declaration_order enum_longest_match_wins_in_either_declaration_order player_name_prefix_of_another_name_parses_longest` (or one filter at a time, e.g. `cargo test -p brdgme_game either_declaration_order`). Expected:
  - `enum_full_match_wins_ties_in_either_declaration_order` FAILS with the panic `values ["abc", "ab"]: matched ab and abc, more input is required to uniquely match one`.
  - `enum_longest_match_wins_in_either_declaration_order` FAILS on the `["ab", "abcd"]` iteration (left/right mismatch: `consumed "ab"`, `remaining "cx"`).
  - `player_name_prefix_of_another_name_parses_longest` FAILS on the `["Bo", "Bobby"]` iteration of the `"bobb"` assertion.
- [ ] Implement. In `Enum::parse`, replace the two comment lines above `let mut full_match = false;`:

```rust
        // Exact matches are prioritised, a shorter full match will happen over a longer partial
        // match.
```

  with:

```rust
        // Candidates are ranked by (bytes of input matched, then whether the
        // whole value was matched). Longest wins; a full match only breaks a
        // tie against an equal-length partial match. Replacing on a strictly
        // better key - rather than appending on ties - is what makes the
        // outcome independent of value declaration order (lg F5).
```

  and replace the ranking block:

```rust
            if matching > 0 && matching >= match_len && (!full_match || full) {
                if full {
                    full_match = true
                }
                if matching > match_len {
                    matched = vec![v];
                    match_len = matching;
                } else {
                    matched.push(v);
                }
            }
```

  with:

```rust
            if matching == 0 {
                continue;
            }
            match (matching.cmp(&match_len), full.cmp(&full_match)) {
                // Strictly longer match: it becomes the sole candidate.
                (Ordering::Greater, _) => {
                    matched = vec![v];
                    match_len = matching;
                    full_match = full;
                }
                // Same length, but a full match beats a partial one.
                (Ordering::Equal, Ordering::Greater) => {
                    matched = vec![v];
                    full_match = full;
                }
                // Genuinely ambiguous: same length, same match kind.
                (Ordering::Equal, Ordering::Equal) => matched.push(v),
                // Shorter, or an equal-length partial against a full match.
                _ => {}
            }
```

  (`Ordering` is already imported at parser/mod.rs:1. `bool`'s `Ord` orders `false < true`, so `full.cmp(&full_match)` is `Greater` exactly when this candidate is full and the incumbent was not.)
- [ ] Run: the three new tests - PASS. Then `cargo test -p brdgme_game` - all 108 tests PASS, `test_enum_works` unmodified.
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `fix(parser): order-independent Enum match ranking (lg F5, WP-03)`

---

### Task 4: suggest honours a bounded `Many`'s `max` (lg F9, minor) - DISCHARGES c F31 (minor)

**Problem (restated):** `suggest.rs:109` destructures `Spec::Many { spec, delim, .. }`, throwing away `min`/`max`. The parse side enforces `max` (the `CommandSpec::Many` loop head), so once `max` items have been typed the suggest engine keeps offering another item that the parser will reject. Live today: `sushizock-2/src/command.rs:47` `Many::bounded_spaced(Int::bounded(1, max as i32), 1, max)` with `max = rolled_dice.len()` (c F31) and `sushi-go-2/src/command.rs:42` `Many::bounded_spaced(Int::bounded(1, max as i32), 1, 2)`. In sushizock, after a player has typed all their dice numbers the autocomplete still offers one more - in the game's most common interaction.

Precisely what is wrong (c F31's wording is looser than the code): the die-number VALUES are already bounded, because `Spec::Int`'s suggest arm caps at `max`. What is unbounded is the ITEM COUNT.

**Fix (re-derived, as the finding recommends):** count items that were fully consumed (item followed by a delimiter) and return `vec![]` at the top of the loop once the count reaches `max`. `min` is deliberately ignored - suggesting an item while below the minimum is correct and desirable.

**Edge cases:**
- `max: None` -> guard never fires; all 85 existing suggest tests keep their behavior (none constructs a `Many` with `max: Some(..)`; verified by `grep -n "max: Some" suggest.rs`, whose only hits are `Spec::Int` specs and the acquire buy-phase `Int`).
- Item currently being typed at the cap: input `"roll 1 2 3 4 5"` (no trailing space, `max = 5`) - only four items are followed by a delimiter, so the count is 4 and the fifth word is still suggested (`["5"]`). Correct: the user is mid-word.
- Cap reached with a trailing delimiter: `"roll 1 2 3 4 5 "` -> count 5 -> `vec![]`.
- Cap exceeded in the input (`max = 2`, `"1 2 3 "`) -> count reaches 2 -> `vec![]`; the parse side will reject the third item anyway.
- `max: Some(0)` -> immediately `vec![]`. Consistent with the parse side accepting zero items.
- The `Doc` wrapper around the `Many` (sushizock/sushi-go both wrap it) passes an empty result straight through: `at_current_pos` is false for an empty vec, so the arm returns `suggs` unchanged (suggest.rs:52-59).
- Task 2's progress guards are preserved verbatim in the replacement arm below.

**Files:**
- Modify: `rust/lib/game/src/command/suggest.rs` (the `Spec::Many` arm of `suggest_spec`)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/lib/game/src/command/suggest.rs`, at the end of the file (after the last test, inside `mod tests`):

```rust
    // --- Bounded Many (lg F9) and the sushizock-2 roll command (c F31) ---

    fn bounded_many_spec(max: usize) -> Spec {
        Spec::Many {
            spec: Box::new(Spec::Enum {
                values: vec!["1".into(), "2".into(), "3".into()],
                exact: true,
            }),
            min: Some(1),
            max: Some(max),
            delim: Some(Box::new(Spec::Space)),
        }
    }

    #[test]
    fn many_stops_suggesting_at_max_items() {
        // lg F9: the suggest loop discarded min/max, so it offered a third
        // item for a Many the parser caps at two.
        let spec = bounded_many_spec(2);
        assert_eq!(vals(&spec.suggest("", &[])), vec!["1", "2", "3"]);
        assert_eq!(vals(&spec.suggest("1 ", &[])), vec!["1", "2", "3"]);
        assert!(
            spec.suggest("1 2 ", &[]).is_empty(),
            "no third item may be suggested once max is reached"
        );
        assert!(
            spec.suggest("1 2 3 ", &[]).is_empty(),
            "already past max: still nothing to suggest"
        );
        // A word still being typed at the cap is not a new item.
        assert_eq!(vals(&spec.suggest("1 2", &[])), vec!["2"]);
    }

    // Mirrors the exact `to_spec()` output of sushizock-2's `roll_parser`
    // (rust/game/sushizock-2/src/command.rs:38-50):
    //   Map(Chain2(
    //     Doc("roll", "roll dice", Token("roll")),
    //     AfterSpace(Doc("dice", "list of dice numbers to roll, separated by
    //       spaces", Many::bounded_spaced(Int::bounded(1, dice), 1, dice))),
    //   ))
    // where Map::to_spec() delegates to the inner parser and
    // AfterSpace::to_spec() = Chain([Space, inner]).
    fn sushizock_roll_spec(dice: usize) -> Spec {
        Spec::Chain(vec![
            Spec::Doc {
                name: "roll".into(),
                desc: Some("roll dice".into()),
                spec: Box::new(Spec::Token("roll".into())),
            },
            Spec::Chain(vec![
                Spec::Space,
                Spec::Doc {
                    name: "dice".into(),
                    desc: Some("list of dice numbers to roll, separated by spaces".into()),
                    spec: Box::new(Spec::Many {
                        spec: Box::new(Spec::Int {
                            min: Some(1),
                            max: Some(dice as i32),
                        }),
                        min: Some(1),
                        max: Some(dice),
                        delim: Some(Box::new(Spec::Space)),
                    }),
                },
            ]),
        ])
    }

    #[test]
    fn sushizock_roll_suggestions_stop_at_the_dice_count() {
        // c F31: with five dice rolled, `roll` must stop suggesting numbers
        // once five have been entered - the parser accepts at most five.
        let spec = sushizock_roll_spec(5);
        assert_eq!(vals(&spec.suggest("", &[])), vec!["roll"]);
        assert_eq!(
            vals(&spec.suggest("roll ", &[])),
            vec!["1", "2", "3", "4", "5"]
        );
        assert_eq!(
            vals(&spec.suggest("roll 1 2 ", &[])),
            vec!["1", "2", "3", "4", "5"],
            "two of five dice entered: more are still legal"
        );
        // Mid-word at the cap is still the fragment being typed.
        assert_eq!(vals(&spec.suggest("roll 1 2 3 4 5", &[])), vec!["5"]);
        assert!(
            spec.suggest("roll 1 2 3 4 5 ", &[]).is_empty(),
            "all five dice entered: nothing more may be suggested"
        );
        // The two-dice case (also sushi-go-2's shape).
        let spec = sushizock_roll_spec(2);
        assert_eq!(vals(&spec.suggest("roll ", &[])), vec!["1", "2"]);
        assert!(spec.suggest("roll 1 2 ", &[]).is_empty());
    }
```

- [ ] Run: `cargo test -p brdgme_game many_stops_suggesting_at_max_items sushizock_roll_suggestions_stop_at_the_dice_count` (or `cargo test -p brdgme_game at_max_items` then `cargo test -p brdgme_game sushizock_roll`). Expected: BOTH FAIL on their first `is_empty()` assertion - pre-fix, `"1 2 "` suggests `["1","2","3"]` and `"roll 1 2 3 4 5 "` suggests `["1","2","3","4","5"]`.
- [ ] Implement. In `rust/lib/game/src/command/suggest.rs`, replace the whole `Spec::Many` arm (as installed by Task 2) with:

```rust
        Spec::Many {
            spec,
            max,
            delim,
            ..
        } => {
            let mut rem = remaining;
            // Items already fully consumed (item plus delimiter). The parse
            // side refuses more than `max` of them, so suggesting a further
            // item would offer input the parser rejects (lg F9, c F31).
            // `min` is deliberately ignored: suggesting an item while below
            // the minimum is correct.
            let mut consumed_items = 0usize;
            loop {
                if let Some(max) = max
                    && consumed_items >= *max
                {
                    return vec![];
                }
                match spec.parse(rem, names) {
                    Ok(out) => {
                        let after_item = out.remaining;
                        if after_item.is_empty() {
                            // A fully-parsed trailing word is still the fragment
                            // being typed, so filter by it rather than advancing.
                            return suggest_spec(spec, rem, names);
                        }
                        if let Some(d) = delim {
                            match d.parse(after_item, names) {
                                Ok(d_out) => {
                                    if d_out.remaining.len() == rem.len() {
                                        // Progress invariant (lg F6): neither the
                                        // item nor the delimiter consumed
                                        // anything, so looping would hang the
                                        // WASM main thread.
                                        return suggest_spec(spec, rem, names);
                                    }
                                    consumed_items += 1;
                                    rem = d_out.remaining;
                                    continue;
                                }
                                Err(_) => {
                                    // The item parse may have stopped mid-word
                                    // (e.g. an Enum prefix inside a longer word);
                                    // the whole first word is the fragment, not
                                    // the unconsumed leftover.
                                    let fragment = rem.split_whitespace().next().unwrap_or("");
                                    return suggest_spec(spec, fragment, names);
                                }
                            }
                        } else {
                            if after_item.len() == rem.len() {
                                // Progress invariant (lg F6), delimiter-free case.
                                return suggest_spec(spec, rem, names);
                            }
                            consumed_items += 1;
                            rem = after_item;
                            continue;
                        }
                    }
                    Err(_) => {
                        return suggest_spec(spec, rem, names);
                    }
                }
            }
        }
```

- [ ] Run: the two new tests - PASS. Then `cargo test -p brdgme_game` - all 110 tests PASS (in particular the acquire/splendor/jaipur suggest scenarios, which all use `max: None`).
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/suggest.rs` ; message: `fix(suggest): stop suggesting items past a bounded Many max (lg F9, c F31, WP-03)`

---

### Task 5: `Int` suggestion range cannot overflow (lg F10, minor)

**Problem (restated):** `suggest.rs:85-96`:

```rust
        Spec::Int { min, max } => {
            let start = min.unwrap_or(1);
            let end = max.map(|m| m.min(start + 4)).unwrap_or(start + 4);
```

`start + 4` overflows for a spec with `min` in `i32::MAX - 3 ..= i32::MAX`: a panic in debug builds (and `cargo test` builds in debug), a wrap to a negative `end` - hence an empty suggestion list - in release. Spec-supplied rather than user-supplied, hence minor, but the fix is one word.

**Fix:** `start.saturating_add(4)`. Saturating at `i32::MAX` is exactly the wanted clamp, and for every non-overflowing `start` the value is identical, so no in-range behavior can change.

**Edge cases:** `min: Some(i32::MAX - 1), max: None` -> `["2147483646", "2147483647"]`; `min: Some(i32::MAX), max: Some(i32::MAX)` -> `["2147483647"]`; `min: None` -> `start = 1`, unchanged; negative `min` -> unchanged (no overflow possible on the positive side). `RangeInclusive<i32>` iteration up to `i32::MAX` terminates correctly.

**Files:**
- Modify: `rust/lib/game/src/command/suggest.rs` (the `Spec::Int` arm)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/lib/game/src/command/suggest.rs`, at the end of the `// --- Int ---` section (after `int_no_prefix_match_returns_empty`, which ends at line 463):

```rust
    #[test]
    fn int_near_i32_max_does_not_overflow() {
        // lg F10: `start + 4` overflowed - a panic in debug builds, a wrap to
        // a negative end (empty range) in release.
        let spec = Spec::Int {
            min: Some(i32::MAX - 1),
            max: None,
        };
        assert_eq!(vals(&spec.suggest("", &[])), vec!["2147483646", "2147483647"]);
        let spec = Spec::Int {
            min: Some(i32::MAX),
            max: Some(i32::MAX),
        };
        assert_eq!(vals(&spec.suggest("", &[])), vec!["2147483647"]);
    }
```

- [ ] Run: `cargo test -p brdgme_game int_near_i32_max_does_not_overflow`. Expected: FAIL with the panic `attempt to add with overflow` at the `Spec::Int` arm.
- [ ] Implement. In the `Spec::Int` arm, replace:

```rust
            let start = min.unwrap_or(1);
            let end = max.map(|m| m.min(start + 4)).unwrap_or(start + 4);
```

  with:

```rust
            let start = min.unwrap_or(1);
            // Saturating: a spec may set `min` within 4 of i32::MAX, where
            // `start + 4` panics in debug and wraps to a negative (empty)
            // range in release (lg F10).
            let capped = start.saturating_add(4);
            let end = max.map(|m| m.min(capped)).unwrap_or(capped);
```

- [ ] Run: `cargo test -p brdgme_game int_near_i32_max_does_not_overflow` - PASS. Then `cargo test -p brdgme_game` - all 111 tests PASS (the five existing `int_*` suggest tests pin the ordinary ranges).
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/suggest.rs` ; message: `fix(suggest): saturating Int suggestion range (lg F10, WP-03)`

---

### Task 6: dedupe `Enum` suggestions and stop `Token("")` shadowing a chain (lg F18 + lg F20, nits)

**Problem (restated):**

- lg F18 (`suggest.rs:35-45`): the `Enum` arm maps every value verbatim, so a spec with duplicate values yields duplicate suggestions - while `Enum::parse` explicitly dedupes with a `HashSet` keyed on the lowercased value (parser/mod.rs:612-619). Undocumented asymmetry between what is suggested and what is selectable.
- lg F20 (`suggest.rs:25-34`): `"".starts_with("")` is true, so `Spec::Token("")` on an empty fragment returns `Suggestion { value: "" }`. Inside a `Chain`, any non-empty result short-circuits advancement (suggest.rs:74-76), so a zero-width token suppresses suggestions for every later element of the chain.

**Fix (re-derived):** dedupe the `Enum` arm on the lowercased value, preserving the first occurrence and the declaration order (same key as `Enum::parse`, so suggestions and selectability agree). Guard the `Token` arm on an empty token. **`OneOf` is deliberately NOT deduped** (the finding's optional half): `OneOf` branches attach independent `Doc` descriptions, so equal values from different branches are not interchangeable, and `OneOf::parse` performs no dedupe either - there is no parser asymmetry to mirror there.

**Case-folding stays exactly as it is** (`to_lowercase`, not `UniCase`) - lg F17 is WP-04's.

**Edge cases:**
- No duplicates (every real spec): output identical, including ordering. All 85 existing suggest tests unaffected.
- Values differing only in case (`"Red"`, `"red"`): collapse to the first, which mirrors `Enum::parse`'s dedupe (only the first is selectable anyway).
- `Spec::Token("")` standalone -> `vec![]` (was `[Suggestion { value: "" }]`).
- `Spec::Token("")` inside a `Chain` -> the chain now advances past it (`Token("")::parse` succeeds consuming nothing) and suggests the next element.
- `Token("")` PARSE behavior is unchanged - only the suggestion is suppressed.
- A `Many` whose item is `Token("")`: the empty suggestion is gone and Task 2's progress guard already prevents the loop from hanging.

**Files:**
- Modify: `rust/lib/game/src/command/suggest.rs` (imports; the `Spec::Token` and `Spec::Enum` arms)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/lib/game/src/command/suggest.rs`: the first at the end of the `// --- Enum ---` section (after `enum_exact_same_prefix_behavior`, which ends at line 256) and the second at the end of the `// --- Token ---` section (after `token_input_longer_than_token_no_match`, which ends at line 203):

```rust
    #[test]
    fn enum_duplicate_values_suggested_once() {
        // lg F18: Enum::parse dedupes values with a HashSet keyed on the
        // lowercased value, so suggestions must not show duplicates that
        // cannot be selected independently anyway.
        let spec = Spec::Enum {
            values: vec!["buy".into(), "buy".into(), "BUY".into(), "sell".into()],
            exact: false,
        };
        assert_eq!(vals(&spec.suggest("", &[])), vec!["buy", "sell"]);
        assert_eq!(vals(&spec.suggest("b", &[])), vec!["buy"]);
    }
```

```rust
    #[test]
    fn empty_token_suggests_nothing_and_does_not_shadow_a_chain() {
        // lg F20: `"".starts_with("")` produced a Suggestion { value: "" },
        // and the Chain arm treats any non-empty result as final - so a
        // zero-width token hid every later element's suggestions.
        assert!(Spec::Token("".into()).suggest("", &[]).is_empty());
        let spec = Spec::Chain(vec![Spec::Token("".into()), Spec::Token("play".into())]);
        assert_eq!(vals(&spec.suggest("", &[])), vec!["play"]);
    }
```

- [ ] Run: `cargo test -p brdgme_game enum_duplicate_values_suggested_once empty_token_suggests_nothing_and_does_not_shadow_a_chain` (or one filter at a time). Expected: `enum_duplicate_values_suggested_once` FAILS (`left: ["buy", "buy", "BUY", "sell"]`); `empty_token_suggests_nothing_and_does_not_shadow_a_chain` FAILS on the first assertion (the suggestion list is `[""]`).
- [ ] Implement (1/3) - import. At the top of `rust/lib/game/src/command/suggest.rs`, above `use crate::command::parser::Parser;` (line 14), add:

```rust
use std::collections::HashSet;
```

- [ ] Implement (2/3) - `Token` arm. Replace lines 25-34:

```rust
        Spec::Token(token) => {
            if token.to_lowercase().starts_with(&remaining.to_lowercase()) {
                vec![Suggestion {
                    value: token.clone(),
                    desc: None,
                }]
            } else {
                vec![]
            }
        }
```

  with:

```rust
        Spec::Token(token) => {
            if token.is_empty() {
                // A zero-width token has nothing to offer, and a
                // `Suggestion { value: "" }` would short-circuit the `Chain`
                // arm (any non-empty result is final there), hiding the
                // suggestions of every later chain element (lg F20).
                return vec![];
            }
            if token.to_lowercase().starts_with(&remaining.to_lowercase()) {
                vec![Suggestion {
                    value: token.clone(),
                    desc: None,
                }]
            } else {
                vec![]
            }
        }
```

- [ ] Implement (3/3) - `Enum` arm. Replace lines 35-45:

```rust
        Spec::Enum { values, .. } => {
            let lower = remaining.to_lowercase();
            values
                .iter()
                .filter(|v| v.to_lowercase().starts_with(&lower))
                .map(|v| Suggestion {
                    value: v.clone(),
                    desc: None,
                })
                .collect()
        }
```

  with:

```rust
        Spec::Enum { values, .. } => {
            let lower = remaining.to_lowercase();
            // Deduped on the lowercased value, the same key `Enum::parse`
            // uses, so the suggestion list matches what is selectable
            // (lg F18). Declaration order and first-occurrence casing are
            // preserved.
            let mut seen: HashSet<String> = HashSet::new();
            let mut suggestions: Vec<Suggestion> = vec![];
            for v in values {
                let v_lower = v.to_lowercase();
                if !v_lower.starts_with(&lower) {
                    continue;
                }
                if !seen.insert(v_lower) {
                    continue;
                }
                suggestions.push(Suggestion {
                    value: v.clone(),
                    desc: None,
                });
            }
            suggestions
        }
```

- [ ] Run: the two new tests - PASS. Then `cargo test -p brdgme_game` - all 113 tests PASS.
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/suggest.rs` ; message: `fix(suggest): dedupe Enum suggestions, ignore empty tokens (lg F18, lg F20, WP-03)`

---

### Task 7: doc rendering - honest open minimum and bounded maximum (lg F11 + lg F12, minors)

**Problem (restated):**

- lg F11 (`doc.rs:51`): the `(min, Some(max))` arm renders `format!("{}-{}", min.unwrap_or(0), max)`, so `Int { min: None, max: Some(5) }` documents as `0-5` even though the parser accepts negatives (parser/mod.rs:151-159 only rejects below `min` when `min` is `Some`) and `Int::expected_output` (parser/mod.rs:113) says "number 5 or lower". Live shape: `rust/game/for-sale-2/src/command.rs:41-46`.
- lg F12 (`doc.rs:134, 139`): the `(None, _) | (Some(0), _)` arm renders `*` and the `(Some(1), _)` arm renders `+` even when `max` is `Some(n)`, so a bounded `Many` loses its cap. Live shapes: `sushi-go-2/src/command.rs:42` (`1, 2`), `sushizock-2/src/command.rs:47` (`1, max`), `roll-through-the-ages-2/src/command.rs:317` (`1, max`) - all render `thing+` in REPL help and in turn-notification emails today.

**Fix (re-derived):**
- `doc_int`: split the `(min, Some(max))` arm into `(None, Some(max))` -> `#-{max}` and `(Some(min), Some(max))` -> `{min}-{max}`. `#` is already this function's marker for an open end (the `(None, None)` arm renders `#`), so `#-5` reads as "any number up to 5" and matches `expected_output`'s semantics without inventing a `0` floor. The `unwrap_or(0)` disappears.
- `doc_many`: restrict the `*` and `+` shorthands to `max: None` and let every bounded case fall through to the existing range arm. The range arm's `min.unwrap_or(0)` is CORRECT here (unlike `doc_int`): a `Many` with no minimum requires zero items, so `0` is the true floor - a comment records the distinction.

**Edge cases (all pinned by the new tests):**
- `doc_int`: `(None, None)` -> `#`; `(Some(3), Some(3))` -> bold `3` (the `min == max` arm still wins, it is listed first); `(Some(1), Some(5))` -> `1-5`; `(Some(2), None)` -> `2+`; `(None, Some(5))` -> `#-5` (changed).
- `doc_many` unchanged shapes: `(_, Some(0))` -> `None`; `min > max` -> `None`; `(Some(0)|None, Some(1))` -> `thing?`; `(Some(1), Some(1))` -> `thing`; `(None|Some(0), None)` -> `thing*`; `(Some(1), None)` -> `thing+`; `(Some(2), None)` -> `(2+)thing`.
- `doc_many` changed shapes: `(Some(1), Some(2))` -> `thing(1-2)` (was `thing+`); `(Some(0), Some(3))` and `(None, Some(3))` -> `thing(0-3)` (was `thing*`).
- Arm order matters: `(Some(1), None)` MUST come before the general `(Some(min), None)` arm, otherwise `+` becomes `(1+)`.
- Exhaustiveness: after the seven specific arms the catch-all `(min, Some(max))` receives only `Some`/`None` minima with `max >= 2`, so the match still compiles without a wildcard.
- `doc.rs` has no test module today; Task 7 adds one. `doc_int`/`doc_many` are private, so the tests must live in-module (`use super::*;`), which also matches the crate convention.

**Files:**
- Modify: `rust/lib/game/src/command/doc.rs` (`doc_int` lines 45-54, `doc_many` lines 115-155, new `mod tests` at the end of the file)
- Test: same file, NEW inline `#[cfg(test)] mod tests`

**Steps:**

- [ ] Write the failing tests. Append to `rust/lib/game/src/command/doc.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_int_open_minimum_is_not_rendered_as_zero() {
        // lg F11: `min.unwrap_or(0)` documented Int { min: None, max: Some(5) }
        // as "0-5" while the parser accepts negatives and
        // `Int::expected_output` says "number 5 or lower".
        assert_eq!(doc_int(None, Some(5)), vec![Node::text("#-5")]);
        // Unchanged shapes.
        assert_eq!(doc_int(None, None), vec![Node::text("#")]);
        assert_eq!(doc_int(Some(1), Some(5)), vec![Node::text("1-5")]);
        assert_eq!(doc_int(Some(2), None), vec![Node::text("2+")]);
        assert_eq!(
            doc_int(Some(3), Some(3)),
            vec![Node::Bold(vec![Node::text("3")])]
        );
    }

    fn thing() -> Node {
        Node::Bold(vec![Node::text("thing")])
    }

    fn many_doc(min: Option<usize>, max: Option<usize>) -> Option<Vec<Node>> {
        doc_many(
            &Spec::Token("thing".into()),
            min,
            max,
            &None,
            &Opts::default(),
        )
        .map(|(doc, _)| doc)
    }

    #[test]
    fn doc_many_keeps_a_bounded_max() {
        // lg F12: the `*` and `+` arms shadowed the range arm, so
        // Many { min: Some(1), max: Some(2) } (sushi-go-2, sushizock-2,
        // roll-through-the-ages-2) documented as "thing+" instead of
        // "thing(1-2)".
        assert_eq!(
            many_doc(Some(1), Some(2)),
            Some(vec![thing(), Node::text("(1-2)")])
        );
        assert_eq!(
            many_doc(Some(0), Some(3)),
            Some(vec![thing(), Node::text("(0-3)")])
        );
        assert_eq!(
            many_doc(None, Some(3)),
            Some(vec![thing(), Node::text("(0-3)")])
        );
        // Unbounded shapes keep their shorthand.
        assert_eq!(many_doc(None, None), Some(vec![thing(), Node::text("*")]));
        assert_eq!(many_doc(Some(0), None), Some(vec![thing(), Node::text("*")]));
        assert_eq!(many_doc(Some(1), None), Some(vec![thing(), Node::text("+")]));
        assert_eq!(
            many_doc(Some(2), None),
            Some(vec![Node::text("(2+)"), thing()])
        );
        // Optional-like and exactly-one shapes are unchanged.
        assert_eq!(
            many_doc(Some(0), Some(1)),
            Some(vec![thing(), Node::text("?")])
        );
        assert_eq!(many_doc(None, Some(1)), Some(vec![thing(), Node::text("?")]));
        assert_eq!(many_doc(Some(1), Some(1)), Some(vec![thing()]));
        // Empty ranges still document as nothing.
        assert_eq!(many_doc(Some(0), Some(0)), None);
        assert_eq!(many_doc(Some(2), Some(1)), None);
    }
}
```

- [ ] Run: `cargo test -p brdgme_game doc_int_open_minimum doc_many_keeps_a_bounded_max` (or one filter at a time). Expected: `doc_int_open_minimum_is_not_rendered_as_zero` FAILS on the first assertion (`left: [Text("0-5")]`); `doc_many_keeps_a_bounded_max` FAILS on the first assertion (`left: Some([Bold([Text("thing")]), Text("+")])`).
- [ ] Implement (1/2). In `rust/lib/game/src/command/doc.rs`, replace `doc_int` (lines 45-54) with:

```rust
fn doc_int(min: Option<i32>, max: Option<i32>) -> Vec<Node> {
    match (min, max) {
        (None, None) => vec![Node::text("#")],
        (Some(min), Some(max)) if min == max => {
            vec![Node::Bold(vec![Node::text(format!("{}", min))])]
        }
        // `#` marks the open end. Substituting `0` would contradict both the
        // parser (which accepts negatives when `min` is None) and
        // `Int::expected_output` ("number N or lower") - lg F11.
        (None, Some(max)) => vec![Node::text(format!("#-{}", max))],
        (Some(min), Some(max)) => vec![Node::text(format!("{}-{}", min, max))],
        (Some(min), None) => vec![Node::text(format!("{}+", min))],
    }
}
```

- [ ] Implement (2/2). In the same file, replace the match inside `doc_many` (lines 122-154) with:

```rust
    join_docs(&spec.doc_opts(opts)).and_then(|(mut doc, desc)| match (min, max) {
        // Some combinations expect nothing.
        (_, Some(0)) => None,
        (Some(min), Some(max)) if min > max => None,
        // Like optional
        (Some(0), Some(1)) | (None, Some(1)) => {
            doc.push(Node::text("?"));
            Some((doc, desc))
        }
        // Exactly 1
        (Some(1), Some(1)) => Some((doc, desc)),
        // 0 or more. The `*` and `+` shorthands only apply when `max` is
        // None - otherwise they would hide a bounded maximum (lg F12).
        (None, None) | (Some(0), None) => {
            doc.push(Node::text("*"));
            Some((doc, desc))
        }
        // 1 or more
        (Some(1), None) => {
            doc.push(Node::text("+"));
            Some((doc, desc))
        }
        // Other "or more" prepended with min
        (Some(min), None) => {
            let mut prepended = vec![Node::text(format!("({}+)", min))];
            prepended.extend(doc);
            Some((prepended, desc))
        }
        // All others displayed as range. `min.unwrap_or(0)` is correct here,
        // unlike in doc_int: a Many with no minimum requires zero items.
        (min, Some(max)) => {
            doc.push(Node::text(format!("({}-{})", min.unwrap_or(0), max)));
            Some((doc, desc))
        }
    })
```

- [ ] Run: `cargo test -p brdgme_game doc_int_open_minimum doc_many_keeps_a_bounded_max` - PASS. Then `cargo test -p brdgme_game` - all 115 tests PASS.
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/doc.rs` ; message: `fix(doc): honest open Int minimum and bounded Many max (lg F11, lg F12, WP-03)`

---

### Task 8: drop the unused `combine` dependency (lg F15, minor) + final gate

**Problem (restated):** `rust/lib/game/Cargo.toml:12` declares `combine = "4.6.7"`, but `grep -rn combine rust/lib/game/` matches only that line - the command parser is hand-rolled (deliberately), and `lib/markup` is the real `combine` consumer. Every game crate compiles this dead dependency. (`unicase`, `log`, `serde_json`, `thiserror` all have genuine uses and stay: `unicase` at parser/mod.rs:5/51, `log` in `src/bot.rs`, `serde_json` in the `CommandSpec` `Parser` impl, `thiserror` in `src/errors.rs`.)

**Fix:** delete the line. `combine` remains in `rust/Cargo.lock` as a dependency of `brdgme_markup` (lock line 872) and of `jni` (line 2952); only the `brdgme_game` edge (lock line 839) disappears.

**Edge cases:** `lib/game` has no `bin/`, `benches/`, `examples/` or `tests/` directory, so `Cargo.toml` is the only manifest surface. Building the crate proves there is no non-test use; testing proves there is no test use.

**Files:**
- Modify: `rust/lib/game/Cargo.toml` (line 12), `rust/Cargo.lock`

**Steps:**

- [ ] Delete the line `combine = "4.6.7"` from `rust/lib/game/Cargo.toml` (line 12, in `[dependencies]`).
- [ ] Run: `cargo build -p brdgme_game` - compiles (proves no non-test use), then `cargo test -p brdgme_game` - all 115 tests PASS (proves no test use). `rust/Cargo.lock` loses `"combine",` from the `brdgme_game` package block; include the lock in the commit.
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` - must pass end to end (it provides the Postgres/NATS containers; per AGENTS.md this is required before any Rust change lands).
- [ ] Commit: `git add rust/lib/game/Cargo.toml rust/Cargo.lock` ; message: `refactor(game): drop the unused combine dependency (lg F15, WP-03)`

---

### Final verification

- [ ] `cargo test -p brdgme_game` - 115 tests PASS (101 pre-existing, 14 added: 6 in `parser/mod.rs`, 6 in `suggest.rs`, 2 in `doc.rs`).
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean.
- [ ] `/home/beefsack/Development/brdgme/scripts/rust-test.sh` passes.
- [ ] Spot-check the discharge of c F31 by reading `rust/game/sushizock-2/src/command.rs:38-50` and confirming `sushizock_roll_spec` in `suggest.rs`'s tests still mirrors it exactly. **No file under `rust/game/` is modified by this package.**
- [ ] Confirm nothing outside `rust/lib/game/` and `rust/Cargo.lock` was touched: `git status --short` should list only `rust/lib/game/src/command/parser/mod.rs`, `rust/lib/game/src/command/suggest.rs`, `rust/lib/game/src/command/doc.rs`, `rust/lib/game/Cargo.toml`, `rust/Cargo.lock`.

---

## Findings disposition

| Finding | Severity | Original recommendation | Verdict | Reason |
|---|---|---|---|---|
| lg F5 Enum full-match priority is declaration-order dependent | major | Track full matches separately and prefer them so same-length partials are replaced, not appended | **ADJUSTED** | Order-dependence confirmed by re-tracing both orderings. Implemented as an explicit ranking key `(matched length, then full-match)`; the code comment's stronger rule ("shorter full match beats longer partial") is deliberately DROPPED and the comment rewritten, because making that rule order-independent would regress `Player` parsing (`["Bo","Bobby"]` + `"bobb"` would select `Bo` and strand `"bb"`). Task 3. |
| lg F6 Many loops lack a zero-progress guard | major | Track remaining-input length across iterations, break on no progress, in all three loops; document the invariant | **CONFIRMED** | All three loops re-read and verified guard-free; zero-width success constructible via `Opt`, `Token::new("")`, `Chain(vec![])`. Implemented exactly as recommended, plus the invariant doc comment on `Many`. Task 2. |
| lg F8 typed Many early-return bypasses the min check | minor | Drop the early return - "the loop plus min check already handle these configs identically" | **ADJUSTED** | Divergence confirmed, but the recommendation is incorrect as written: the typed loop checks `max` only AFTER pushing (`parsed.len() == max`), so with `max == 0` dropping the early return alone would parse unboundedly. The fix moves the check to the loop head as `parsed.len() >= max`, mirroring the spec impl. Task 1. |
| lg F9 suggest's Many ignores min/max | minor | Count consumed items and return `vec![]` once `max` is reached; ignoring `min` is fine | **CONFIRMED** | Implemented as recommended. One precision added: the die-number VALUES were already bounded by `Spec::Int`; only the item COUNT was unbounded. Task 4. |
| c F31 sushizock-2 roll suggests past the legal dice count | minor | No crate-local fix; resolves when the lib/game Many-ignores-max bug is fixed | **CONFIRMED (discharged by lg F9)** | Re-read `roll_parser` (`sushizock-2/src/command.rs:38-50`) and `Many::to_spec`; the spec really does carry `max = rolled_dice.len()`. Discharge is pinned by `sushizock_roll_suggestions_stop_at_the_dice_count`, a lib-side test that reconstructs sushizock's spec locally. No game-crate file touched. Task 4. |
| lg F10 Int suggestion range can overflow | minor | `start.saturating_add(4)` | **CONFIRMED** | Implemented verbatim; cannot change any in-range behavior. Task 5. |
| lg F11 doc_int renders an open minimum as 0 | minor | Match `expected_output` semantics, e.g. `#-5` rather than substituting 0 | **CONFIRMED** | Implemented as `#-{max}` (`#` is already this function's open-end marker); the `(min, Some(max))` arm is split so `unwrap_or(0)` disappears. Task 7. |
| lg F12 doc_many drops a bounded max | minor | Only take the `*`/`+` shortcuts when `max` is None; otherwise fall through to the range arm | **CONFIRMED** | Implemented verbatim; arm ordering fixed so `(Some(1), None)` still renders `+` rather than `(1+)`. The range arm's `min.unwrap_or(0)` is kept and justified (a Many with no minimum genuinely requires zero items). Task 7. |
| lg F15 `combine` declared but unused | minor | Remove `combine` from `lib/game/Cargo.toml` | **CONFIRMED** | Re-verified: the only match in the whole crate is the manifest line; no bins/benches/examples/tests dir exists. Task 8. |
| lg F18 suggestions are not deduplicated | nit | Dedupe by value in the `Enum` arm (and optionally after `OneOf` concatenation) | **ADJUSTED** | `Enum` arm deduped on the lowercased value (the key `Enum::parse` uses). The optional `OneOf` half is DECLINED: `OneOf` branches carry independent `Doc` descriptions so equal values are not interchangeable, and `OneOf::parse` has no dedupe either - there is no parser asymmetry to mirror. Task 6. |
| lg F20 `Token("")` yields an empty suggestion and shadows later chain elements | nit | Guard `if token.is_empty() { return vec![] }` or document the invariant | **CONFIRMED** | Guard implemented (the documentation-only alternative leaves a live suggestion bug). Parse-side `Token("")` behavior deliberately unchanged. Task 6. |
| lg F7 OneOf furthest-error ranking is dead code | major | Implement offset propagation or delete the ranking | **FENCED OUT** | WP-04 (D-38). Adjacent to Tasks 1/2 (`Many`/`Chain` would carry the offsets); no `offset` value is touched here. |
| lg F13 Doc::expected diverges typed vs spec | minor | Align or add a WHY comment | **FENCED OUT** | WP-04 per `work-packages.md` (note: WP-01's spec text mis-assigns this to WP-03). Task 7 touches only `doc.rs` rendering, never `expected()`. |
| lg F14 Many::expected diverges typed vs spec | minor | Align the spec impl with the typed one, or document | **FENCED OUT** | WP-04 per `work-packages.md`. Tasks 1/2 edit `Many::parse` only; `Many::expected` (immediately below it) is left byte-identical. |
| lg F17 case-folding differs suggest vs Token::parse | nit | Use `UniCase` in suggest, or document the difference | **FENCED OUT** | WP-04. Overlaps Task 6's exact lines - Task 6 preserves `to_lowercase()` and introduces no `UniCase`. |
| lg F19 unbounded recursion over spec nesting | nit | No action for current callers; depth limit only if specs become untrusted | **FENCED OUT** | WP-04. Distinct from Task 2: Task 2 guards iteration progress, F19 concerns recursion depth. No depth counter added. |
| lg F1, F2, F3, F4, F16 char/byte panics | critical/major/nit | see WP-01 | **OUT OF SCOPE** | WP-01 owns them and lands first; Task 3 builds on WP-01's rewritten `Enum::parse`. |

## Cross-package / newly discovered

Recorded, NOT fixed and NOT baked into any test as if intended:

1. **`Spec::Int` suggest starts at 1 when `min` is `None`** (`suggest.rs:86`, `let start = min.unwrap_or(1);`). The parser accepts `0` and negative values when `min` is `None` (parser/mod.rs:151-159), and after Task 7 the doc renders `#-{max}` / `#` for that shape - so suggest is the last of the three views still implying a floor of 1. Same defect class as lg F11 but on the suggest side, and not a filed finding. **Proposed routing: WP-04** (the suggest/spec design package, D-38), since choosing what to suggest for an unbounded-below `Int` is a design call (offer negatives? offer `0`?). Task 5 deliberately leaves `unwrap_or(1)` untouched and its test asserts only the overflow behavior.
2. **`Spec::Int` suggest filters with the raw fragment** (`suggest.rs:90`, `s.starts_with(remaining)`): typing `-` yields no suggestions at all even for an `Int` that accepts negatives. Sub-case of (1); same routing.
3. **`Enum::parse`'s dedupe key is the lowercased value** (parser/mod.rs:612-619), so two values differing only in case (`"Red"` / `"red"`) silently collapse and only the first is ever selectable. Probably intentional (the whole matcher is case-insensitive) but undocumented; Task 6 mirrors the same key in suggest, which makes the two views consistent rather than fixing the underlying question. **Proposed routing: WP-04** as a one-line WHY comment alongside lg F17's folding-convention decision.
4. **Spec-ownership inconsistency in a sibling spec (documentation defect, no code impact):** `specs/WP-01-char-byte-panic-elimination.md` line 35 lists lg F13/F14 as WP-03's and lg F18/F20 as WP-04's, which contradicts `planning/work-packages.md` (WP-03 = F5, F6, F8, F9, F10, F11, F12, F15, F18, F20, c F31; WP-04 = F7, F13, F14, F17, F19). This spec follows `work-packages.md`. **Proposed routing: a one-line correction to WP-01's Non-Goals paragraph** by whoever owns that spec - flagged to the Lead rather than edited here.
