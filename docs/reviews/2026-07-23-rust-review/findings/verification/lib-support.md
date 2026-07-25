# Verification: lib-support (unit 2)

Independent verification of `findings/lib-support.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Raw verdict dumps: `raw/lib-support-markup.md`,
`raw/lib-support-color-cost.md`, `raw/lib-support-cmd.md`,
`raw/lib-support-client-randbot.md`. Process log: `lib-support-LOG.md`.

## Per-finding verdicts

### lib/markup

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | slice() byte-indexes char-count offsets | critical | CONFIRMED | TNode::len counts chars (ast.rs:201), slice byte-indexes (transform.rs:274); canvas path reachable; `<` vs `<=` skip-check also correct |
| F2 | parse_u8/parse_usize unwrap on overflow | major | CONFIRMED | unwrap at parser.rs:54 and 79; reachable via markup typos; parse_pct shows the correct in-file pattern |
| F3 | Malformed markup silently truncates | major | CONFIRMED | many(choice) succeeds with tail in rest; all three cited web callers discard rest; no `{` escape |
| F4 | to_string emits Node::Text raw | minor | CONFIRMED | Text emitted verbatim at lib.rs:45; tags in text re-parse on round-trip |
| F5 | eprintln + silent rgb fallback | minor | CONFIRMED | rgb_reverse_map warns to stderr and substitutes Foreground; col_type_named fails the parse instead |
| F6 | Duplicated escape/wrappers/player() | minor | CONFIRMED | escape byte-identical html.rs:20-25 vs html_class.rs:63-68; other duplications as cited |
| F7 | PLAYER_COUNT = 8 hardcoded | minor | CONFIRMED | Matches `[Color; 8]` player_colors by comment convention only |
| F8 | word_wrap bytes + space collapsing | minor | ADJUSTED | Byte measure (wrap.rs:16) and leading-space drop correct; but mid-line space runs ARE preserved — collapsing occurs only at line starts/wrap points. Severity unchanged |
| F9 | Error discards parse diagnostics | minor | CONFIRMED | `map_err(\|_\| MarkupError::Parse)` at lib.rs:38 drops combine position/expected info |
| F10 | panic!/unwrap in transform+align parse | nit | CONFIRMED | Both unreachable by construction; no-panic rule verified at docs/CODING.md:46 |
| F11 | Stale TNode::len doc | nit | CONFIRMED | TNode::len cannot panic; doc sentence stale |

### lib/color

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F12 | regex+lazy_static serve a dead API | major | CONFIRMED | Only regex dependent workspace-wide; from_hex/from_str callers all under `#[cfg(test)]`; four unwraps at lib.rs:54,58-60 |
| F13 | mono lossy per-channel division | minor | CONFIRMED | rgb(128,128,128) -> 42*3=126 -> black though true mean 128 -> white |
| F14 | Three divergent alias tables | minor | CONFIRMED | named() vs NamedColor::from_str vs markup resolve_named, all at cited locations with divergent alias sets |
| F15 | Palette data ~4x verbose | minor | ADJUSTED | Substance holds (379 five-line literals, protanopia field clones, const fn fix applies) but counts overstated: ~2,000 literal lines not ~3,000; post-fix file ~2,300 lines not ~400. Severity unchanged |
| F16 | themes() doc describes absent mechanism | nit | CONFIRMED | Categories hardcoded in THEMES array; nothing computes background lightness |
| F17 | sRGB thresholds inconsistent, triplicated | nit | CONFIRMED | Runtime 0.03928 (palette.rs:3266) vs two test copies at 0.04045 (3438, 3655) |
| F18 | hex()/Display duplicate format | nit | CONFIRMED | Identical format string at lib.rs:48 and 121 |

### lib/cmd

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F19 | Panic on malformed request in HTTP path | major | CONFIRMED | `.unwrap()` at http.rs:54 in warp handler; parse Err reachable via bad game JSON; `impl Reject` at :17 unused; lead re-traced |
| F20 | REPL undo/load leave stale renders | minor | CONFIRMED | :undo/:load replace game without re-requesting renders; display diverges |
| F21 | bot_cli::cli/Response dead | minor | CONFIRMED | Workspace grep: rand_bot uses only Request; cli has four unwraps (not three — see corrections) |
| F22 | REPL spins forever on stdin EOF | minor | CONFIRMED | read_line byte count ignored; EOF yields "" in a hot loop; errors unwrapped |
| F23 | Panic-heavy runtime paths | minor | CONFIRMED | All cited unwrap inconsistencies and "wrong reponse" typo present |
| F24 | term_size unmaintained | minor | CONFIRMED | 0.3.2 at Cargo.toml:16; sole call site repl.rs:186 |
| F25 | warp vs axum stack drift | minor | CONFIRMED | warp 0.4.3 here, axum 0.8.9 in web and game_client dev-deps |
| F26 | comparison_to_empty | nit | CONFIRMED | `trim() != ""` at repl.rs:147 |
| F27 | Redundant #[serde(default)] | nit | CONFIRMED | serde defaults Option to None regardless |
| F28 | No content-length limit | nit | CONFIRMED | No content_length_limit in the filter chain |
| F29 | Child exit status unchecked | nit | CONFIRMED | Empty stdout surfaces as JSON parse error |
| F30 | First :undo silent no-op | nit | CONFIRMED | undo_stack seeded with initial game |

### lib/game_client

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F31 | No crate-enforced timeout; operator has none | major | CONFIRMED | send_with_retry sets no timeout; operator uses bare Client::new() (controller.rs:230); web 10s / bot 60s verified; lead re-traced |
| F32 | anyhow in a library crate | minor | CONFIRMED | Sole anyhow lib; game/cmd/color/markup all thiserror; error kinds string-flattened |
| F33 | Retry predicate too narrow | minor | CONFIRMED | `is_connect() \|\| is_timeout()` only at lib.rs:80; mid-request resets not retried |
| F34 | serde_yaml deprecated | minor | CONFIRMED | 0.9 present; only bot (Cargo.toml:29) shares it |
| F35 | version_name into Host header unvalidated | nit | CONFIRMED | Interpolated at lib.rs:54; callers pass DB/k8s names; reqwest rejects rather than injects |
| F36 | fetch_game_data 5 sequential round trips | nit | CONFIRMED | Five sequential awaits; four post-Status independent; bot calls per turn |
| F37 | Timing-sensitive retry test | nit | CONFIRMED | 15ms spawn vs 20-40ms jittered first backoff race exists |

### lib/cost

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F38 | Spurious K: Clone bound on new() | nit | CONFIRMED | new() in Clone-bounded impl block; Default needs only Hash+Eq |
| F39 | splendor-2 re-implements lib/cost | minor | CONFIRMED | Sole-consumer claim verified workspace-wide; all semantic equivalences (from_resources/add/inv/sub/sum/can_afford) traced and hold |

### lib/rand_bot

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F40 | chrono unused dependency | minor | CONFIRMED | Declared, zero source references; cmd uses time 0.3 |
| F41 | Join separator inconsistent with tools/fuzz | minor | CONFIRMED | commands() joins " ", fuzz joins ""; Space tokens double-space |
| F42 | Pulls http-server stack unused | minor | CONFIRMED | Default features pull warp/tokio/sentry into a stdio bot |
| F43 | Panics on degenerate specs | minor | CONFIRMED | Unwraps at lib.rs:50, 84, 107; Enum arm handles empties gracefully |
| F44 | extern crate leftover | nit | CONFIRMED | Edition 2024 verified; statement redundant |
| F45 | Mangled comment referencing dead API | nit | CONFIRMED | `// /` bad wrap; no bot uses bot_cli's CLI |

## Summary

- Findings verified: 45
- CONFIRMED: 43, ADJUSTED: 2, REJECTED: 0, UNVERIFIABLE: 0
- Corrected tallies for the unit (unchanged from original): 1 critical /
  5 major / 23 minor / 16 nit — neither ADJUSTED verdict changed a
  severity.
- Lead spot-checked both ADJUSTED verdicts (F8 via full read of wrap.rs,
  F15 via the Worker's quantified dump) and directly re-verified the
  hardest confirmations of the two all-CONFIRMED batches (F19 http.rs
  unwrap, F31 operator no-timeout); all reproduced.

## Notable corrections

Neither changed a verdict's substance or severity:

- F8: "collapses runs of spaces" is overbroad. `split(' ')` yields empty
  words for mid-line runs and the join branch re-adds one space per
  empty word, so mid-line space runs are preserved; only leading spaces
  and runs at line starts / wrap boundaries collapse. The byte-length
  width measure and the docstring gap stand as claimed.
- F15: line-count estimates overstated. palette.rs has 379 `Color {`
  literals at 5 lines each (~1,900-2,000 literal lines, not ~3,000), and
  a const-fn rewrite would land the file near ~2,300 lines, not ~400
  (doc comments, functions, and the extensive test suite dominate the
  remainder). The protanopia field-by-field clones and the const fn
  recommendation check out.
- F21 (minor factual refinement, verdict CONFIRMED): bot_cli::cli
  contains four `.unwrap()`s (bot_cli.rs:29,30,41,43), not three.

Overall assessment: the original lib-support review is highly accurate —
all locations, reachability claims, and cross-crate traces (dead parse
API, sole-consumer claims, caller timeout configs, semantic equivalence
of the duplicated cost module) checked out. The only corrections are
quantitative overstatements in two minor findings.
