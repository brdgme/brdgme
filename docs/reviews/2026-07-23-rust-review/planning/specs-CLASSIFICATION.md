# specs/ classification - KEEP / ARCHIVE / UNCERTAIN

Covers **every one of the 60 files** in `docs/reviews/2026-07-23-rust-review/planning/specs/`.
Produced 2026-07-26 by a read-only classification unit: nothing was moved, renamed
or deleted. The destructive pass is a separate, later unit.

**Counts: 47 KEEP, 13 ARCHIVE, 0 UNCERTAIN.**

## How the verdicts were reached

Each file was **read**. Filename, mtime and file size were explicitly not used as
evidence - they correlate with tier but prove nothing. For every spec a Worker
also checked, read-only: `git log --grep 'WP-NN'` and the log for the target
paths, the presence or absence in **live source** of the symbols the spec says to
add or delete, the package's status line in `work-packages.md`, and whether any
decision the spec depends on was later changed in `decisions-ANSWERED.md` /
`decisions-session3.md`. `work-packages.md` status lines proved repeatedly stale;
live source was treated as the authority.

## What ARCHIVE turned out to mean

The premise this unit started from - that `specs/` holds ~26 compact Tier 2 specs
mixed with ~25 superseded bloated pre-tiering specs - is **wrong**, and acting on
it would have destroyed work.

- There is **exactly one spec file per WP number**. No two files cover the same
  work package. There is no compact-vs-bloated duplicate pair anywhere in the
  directory. The ~25 "bloated" Tier 1 specs are specs for *different packages*,
  not replacements of anything.
- Bloat and citation rot are therefore recorded in the `risk-to-executor` field,
  never in the verdict. Archiving a spec for being long would have left its
  package with no spec at all.
- Every one of the 13 ARCHIVE verdicts is a spec whose work has **already fully
  landed**, verified in live source and not merely from a commit message:
  **WP-01, WP-03, WP-06, WP-13, WP-14, WP-15, WP-21, WP-25, WP-36, WP-37, WP-39,
  WP-41, WP-44.**
- No spec was archived for supersession. Every supersession found in the corpus is
  **section-internal** (WP-42 s3a by WP-84 s3c, WP-19 `c F11` by WP-81, WP-59
  Task 14 by D-15, WP-40's rewind note by D-3) and is handled by amending that
  section, not by discarding the file. Package WP-78 is superseded by WP-82, but
  WP-78 never had a spec file.

## KEEP specs carrying a known defect (fix in the destructive/amendment pass)

These are live and must be kept, but a literal executor would go wrong in one
named section:

- **WP-19** - drop Task 5; `c F11` is superseded by WP-81, which deletes the file
  Task 5 edits. Land WP-81 first.
- **WP-29** - Task 4 is invalidated by the `e F30` ruling.
- **WP-45** - s3b and s3c encode the pre-refinement D-8 behaviour; D-8 was REFINED
  to re-resolve a deprecated bot to the latest non-deprecated version on restart.
- **WP-59** - Task 14 must be rewritten for the redesigned D-15 (parser-first
  dispatch, platform commands as fallback). Tasks 1-13 are unaffected. The spec's
  own stale "D-15 IS STILL OPEN" gate fails safe.
- **WP-62** - s4's `bo F25` "BLOCKED" is stale; the decision was answered (pin
  `v1_36`).
- **WP-40** - the WP-82 landing order stated in the spec is now inverted.
- **WP-51** - its drift table is stale now that WP-44 has landed.
- **WP-68** - cites `repl.rs:186` three times; the call site is now ~`:219` after
  WP-06 landed. Line hints only - the quoted text still locates it.

## Highest-risk files for a small cheap executor

`risk-to-executor: high` was recorded for the heavily line-addressed Tier 1 specs.
Among the KEEP set the sharpest are **WP-51**, **WP-59**, **WP-28**, **WP-19**,
**WP-23** and **WP-54** - hundreds of `file.rs:NNN` citations each, several as
"replace lines A-B" ranges. The corpus-wide measured error rate on such citations
was 33-46%. Any executor pointed at these must be told to locate code by file plus
symbol name and treat every line number as a sanity check only.

## Duplicate coverage

**None.** Checked across all 60 files: one spec file per WP number, no package
covered twice. `WP-72` intentionally has no file - its content is section 3d of
`WP-69-deny-toml-hardening.md`. `WP-09` is deliberately split into `WP-09a` and
`WP-09b`, which cover different scopes and do not supersede each other.

## Non-`WP-` files

`notes-conventions.md` is the only one - KEEP, not a spec, it is the shared
conventions reference (repo layout, build/test commands, lint, deploy) that the
specs' Global Constraints sections restate. Every path it asserts was verified
present.

## Delete in the destructive pass

These scratch files were the working notes for this classification and are fully
superseded by this document:

- `.classify-survey.md`
- `.classify-batch-1.md`
- `.classify-batch-2.md`
- `.classify-batch-3.md`
- `.classify-batch-4.md`
- `.classify-batch-5.md`
- `.classify-batch-6.md`
- `.classify-batch-7.md`
- `.classify-batch-8.md`
- `.classify-batch-9.md`
- `.classify-batch-10.md`
- `.classify-batch-11.md`
- `.classify-batch-12.md`

## Verdict table

| file | verdict | lines |
|---|---|---:|
| notes-conventions.md | KEEP | 109 |
| WP-01-char-byte-panic-elimination.md | ARCHIVE | 805 |
| WP-02-markup-robustness-dedup.md | KEEP | 139 |
| WP-03-lib-game-parser-mechanical.md | ARCHIVE | 1318 |
| WP-04-game-parser-design.md | KEEP | 179 |
| WP-05-color-dead-parse-api.md | KEEP | 128 |
| WP-06-lib-cmd-tools-http.md | ARCHIVE | 837 |
| WP-07-game-client-rand-bot.md | KEEP | 845 |
| WP-08-finish-placings-epilogue-dedup.md | KEEP | 150 |
| WP-09a-deserialized-state-boundary.md | KEEP | 166 |
| WP-09b-game-crate-state-trust-sweep.md | KEEP | 138 |
| WP-10-pub-state-hidden-info-redaction.md | KEEP | 129 |
| WP-13-starship-catan-fixes.md | ARCHIVE | 725 |
| WP-14-alhambra-core-fixes.md | ARCHIVE | 715 |
| WP-15-seven-wonders-mechanical.md | ARCHIVE | 875 |
| WP-17-lib-cost.md | KEEP | 178 |
| WP-19-acquire-fixes.md | KEEP | 841 |
| WP-21-cathedral-sushizock-fixes.md | ARCHIVE | 1081 |
| WP-22-lords-of-vegas-fixes.md | KEEP | 736 |
| WP-23-jaipur-fixes.md | KEEP | 637 |
| WP-25-modern-art-liveness.md | ARCHIVE | 525 |
| WP-28-lost-cities-shared-fixes.md | KEEP | 730 |
| WP-29-red7-cleanup.md | KEEP | 467 |
| WP-34-auth-races-session-mechanical.md | KEEP | 170 |
| WP-35-auth-edge-semantics-fail-open.md | KEEP | 215 |
| WP-36-crypto-deploy-hardening.md | ARCHIVE | 423 |
| WP-37-admin-pass.md | ARCHIVE | 2355 |
| WP-38-bot-turn-wedge-recovery.md | KEEP | 175 |
| WP-39-bot-consumer-supervision.md | ARCHIVE | 965 |
| WP-40-undo-concede-toctou-ratings-integrity.md | KEEP | 477 |
| WP-41-db-quality-pass.md | ARCHIVE | 1990 |
| WP-42-websocket-auth-and-filtering.md | KEEP | 247 |
| WP-44-proposals-integrity-email-token-leak.md | ARCHIVE | 795 |
| WP-45-bot-slot-validation.md | KEEP | 119 |
| WP-46-sweep-delivery-semantics.md | KEEP | 229 |
| WP-47-game-visibility-gates.md | KEEP | 120 |
| WP-48-export-import.md | KEEP | 129 |
| WP-49-rules-and-game-info-pages.md | KEEP | 121 |
| WP-50-email-canonicalization.md | KEEP | 179 |
| WP-51-invite-mailer-notify-dedup.md | KEEP | 1310 |
| WP-54-frontend-ux-error-handling.md | KEEP | 2051 |
| WP-55-turnstile-spa-rendering.md | KEEP | 148 |
| WP-56-email-from-auth-redesign.md | KEEP | 548 |
| WP-57-inbound-webhook-delivery-semantics.md | KEEP | 140 |
| WP-58-unsubscribe-rfc8058.md | KEEP | 214 |
| WP-59-inbound-processing-quality.md | KEEP | 2795 |
| WP-62-operator.md | KEEP | 166 |
| WP-63-fuzz-tool.md | KEEP | 150 |
| WP-64-workspace-tables.md | KEEP | 120 |
| WP-66-sqlx-unification.md | KEEP | 134 |
| WP-67-sentry-feature-trim.md | KEEP | 135 |
| WP-68-term-size-replacement.md | KEEP | 117 |
| WP-69-deny-toml-hardening.md | KEEP | 151 |
| WP-70-serde-yaml-ng.md | KEEP | 98 |
| WP-71-warp-to-axum.md | KEEP | 152 |
| WP-73-game-binary-consolidation.md | KEEP | 253 |
| WP-81-stats-deletions.md | KEEP | 122 |
| WP-82-db-module-split.md | KEEP | 290 |
| WP-83-parity-fixes-released.md | KEEP | 155 |
| WP-84-sse-migration.md | KEEP | 408 |

## Per-file evidence

### notes-conventions.md
- verdict: KEEP
- lines: 109
- tier: not a spec (shared reference: repo layout, build/test commands, test placement, lint, k8s/deploy)
- risk-to-executor: low - no code edits prescribed, no line-number citations, only file paths.
- evidence: Every path it asserts still exists (`AGENTS.md`, `docs/CODING.md`, `scripts/rust-test.sh`, `scripts/rust-ci-commands.sh`, `rust/rust-toolchain.toml`, `rust/deny.toml`, `k8s/base/web/deployment.yaml` all verified present). No SUPERSEDED/obsolete/replaced-by marker anywhere in the file; it is the shared conventions reference that the Tier 1 and Tier 2 specs' Global Constraints sections restate.

### WP-01-char-byte-panic-elimination.md
- verdict: ARCHIVE
- lines: 805
- tier: Tier 1 pre-tiering - same `REQUIRED SUB-SKILL` banner, Tech Stack / Global Constraints / Non-Goals preamble, a "Snapshot drift: None ... All finding line numbers cited below are valid against the live files" claim, and dense line citations (`ast.rs:198-207`, `transform.rs:274`, `gamer.rs:31-43`). Listed in `README.md`'s Tier 1 roster.
- risk-to-executor: high - the spec asserts its line numbers are drift-verified against a 2026-07-25 snapshot; that claim is now false and the executor would trust it.
- evidence: Landed in commit `9abe8b4` "fix(lib): eliminate char/byte panics in parsers and markup slice (WP-01)", listing exactly the spec's seven findings (lg F1, F2, F3, F4, F16, ls F1, e F29) and touching exactly its declared paths plus `docs/CODING.md` (the non-ASCII test convention the spec asked for). Verified in live source: `lib/markup/src/transform.rs::slice`'s `Text` arm now slices by chars with the comment "start/end are char offsets (TNode::len counts chars), so slice by chars; byte indexing panics on multi-byte glyphs", and `game/red7-1/src/command.rs` now builds `let chars: Vec<char> = input.chars().collect();`. `specs/WP-02-markup-robustness-dedup.md` corroborates: "WP-01 already landed `ls F1` in `rust/lib/markup/src/transform.rs` (`slice`). Do not revisit it."


### WP-02-markup-robustness-dedup.md
- verdict: KEEP
- lines: 139
- tier: Tier 2 compact - carries the standard Tier 2 header block (Findings / Decision / Rebase), the "no line numbers are cited on purpose" banner, and the Problem / Why it's wrong / Required end state / Non-goals / Regression tests / Riders skeleton. Listed in `README.md`'s Tier 2 roster.
- risk-to-executor: low - no line numbers, explicit STOP-and-report instruction, and an explicitly flagged open question about the escape token.
- evidence: Not landed - grepping `rust/lib/markup/src` for `lbrace` returns nothing, so neither the `text()` escape (3b) nor `to_string`'s `{` escaping (3d) exists. The spec is written against the corrected decision: `decisions-ANSWERED.md` records D-37 as "**CORRECTED.** `{{lbrace}}`, not a bare `{{`" with the same nested-closing-tag rationale the spec gives in section 2, so the spec matches the post-correction ruling rather than being invalidated by it. It also correctly rebases on WP-01's landed `slice` change and fences off WP-05 and WP-06 territory.


### WP-03-lib-game-parser-mechanical.md
- verdict: ARCHIVE
- lines: 1318
- tier: Tier 1 pre-tiering - carries the `**For agentic workers:** REQUIRED SUB-SKILL` banner, a Tech Stack / Global Constraints / Non-Goals preamble, and exhaustive live line-number citations (`parser/mod.rs:473-520`, `suggest.rs:26, 36-39`, `chain.rs:19`, etc.). `README.md` lists WP-03 in the Tier 1 roster.
- risk-to-executor: high - ~1300 lines of stale line-number citations for work that is already committed; an executor opening it would re-apply landed changes.
- evidence: The work has fully landed in commit `c39786f` "fix(lib): parser/suggest/doc mechanical fixes (WP-03)", whose message enumerates exactly the spec's eleven findings (lg F5, F6, F8, F9, F10, F11, F12, F15, F18, F20, c F31) and whose diff touches exactly the spec's declared paths (`command/doc.rs`, `command/parser/mod.rs`, `command/suggest.rs`, `lib/game/Cargo.toml`, `Cargo.lock`). Verified in live source: `combine` is absent from `rust/lib/game/Cargo.toml` (Task 8), and `parser/mod.rs` contains the zero-progress guard (`let progressed = new_offset > offset;` plus a doc comment "impl stop as soon as an iteration makes no progress"). `specs/WP-04-game-parser-design.md` independently states "WP-03 first (already applied in live code - the progress guards, the `Many` max-at-top-of-loop check and the suggest dedup are present)". No decision this spec depends on was reversed. `work-packages.md` still says READY, but live source wins.


### WP-04-game-parser-design.md
- verdict: KEEP
- lines: 179
- tier: Tier 2 compact - Findings / Decision / Landing order header, "no line numbers are cited on purpose" banner, standard six-section skeleton with a Riders table. `README.md` names WP-04 as one of the eight formerly decision-blocked Tier 2 packages specced on 2026-07-26.
- risk-to-executor: low - locates code by symbol, states the standing constraint from D-38 ("keep the parser obvious"), and records one Lead ruling (the `CommandSpec::Chain` expected() divergence) explicitly.
- evidence: Not landed - grepping `rust/lib/game/src` for `add_offset`, `many_expected` and `to_folded_case` returns nothing, so none of 3a, 3b or 3c exists in live source. Its stated predecessor is satisfied: WP-03 is committed (`c39786f`) and the spec already says so. `decisions-ANSWERED.md` records D-38 "ACCEPTED as recommended, all four sub-items", matching this spec's header verbatim including the (iv) skip; nothing was reversed. The Non-goals correctly fence WP-03's landed items back out.


### WP-05-color-dead-parse-api.md
- verdict: KEEP
- lines: 128
- tier: Tier 2 compact (FINDHDR + no-line-numbers blockquote; deletions specified "by item name only, never by line range")
- risk-to-executor: low - every deletion is name-addressed with an explicit keep-list, and the one line hint it gives is marked "approximate, verify".
- evidence: Not landed, decision intact. `git log --grep 'WP-05'` is empty and `work-packages.md:141` reads "### WP-05 lib color - READY (D-39 answered 2026-07-26: option A, delete the dead parse API)", matching `decisions-ANSWERED.md` D-39 verbatim ("ACCEPTED - option A: delete the dead color parse API (`from_hex` / `from_str`)"); D-39 is not among the five rulings that changed a previously recorded position. All targets are live and unmodified: `lib/color/src/lib.rs` still has `pub fn from_hex` (:51), `impl FromStr for Color` (:69), the private `fn named` (:127) and the test `color_from_hex_works` (:165), and `lib/color/Cargo.toml` still declares `lazy_static = "1.5.0"` (:9) and `regex = "1.12.4"` (:10). The spec has already done the corrective work a Tier 1 spec would not have - it explicitly restates ls F15's numbers as wrong ("Live: 379 `Color {` literals, ~2,000 literal lines (not ~3,000) ... lands the file near ~2,300 lines, not ~400. Use these numbers").


### WP-06-lib-cmd-tools-http.md
- verdict: ARCHIVE
- lines: 837
- tier: Tier 1 pre-tiering - "REQUIRED SUB-SKILL" banner, Architecture/Tech Stack/Global Constraints/Non-Goals preamble, numbered `### Task 1..5` sections, and dense live line-number citations (http.rs:36-57, gamer.rs:28,37,41,45, bot_cli.rs:29,30,41,43). README's Tier 1 roster names WP-06.
- risk-to-executor: high - 837 lines of task instructions against code that has already been rewritten, so every citation is now stale and re-executing Tasks 1-5 would churn or regress landed fixes.
- evidence: The spec's five tasks map exactly onto findings ls F19, F23, F20/F22/F26/F30, F21/F44/F45, F29/F27, and commit `a543120` ("fix(cmd): ... (WP-06)", 14 files, +475/-114) lists every one of those finding IDs in its body. Verified in live source: `lib/cmd/src/http.rs` now has `content_length_limit(MAX_CONTENT_LENGTH)` and `unwrap_or_else(|e| Response::SystemError {...})` with SystemError tests (Task 1); `requester/gamer.rs::renders` returns `Result<(PubRender, Vec<PlayerRender>), GameResponseError>` (Task 2's only public-signature change); `repl.rs` has `Ok(0) | Err(_) => None` on EOF and a "No undos available" empty-stack path (Task 3); `bot_cli.rs` contains only `Request`, the dead `cli`/`Response` are gone (Task 4); `requester/local.rs` has `RequestError::ChildExit { status }` plus a `failing_child_reports_exit_status_not_json_error` test and `api.rs` has no `serde(default)` (Task 5). `specs-LOG.md` independently records WP-06 among landed packages and repeatedly notes "WP-06 Task 1 has ALREADY LANDED". No open decision reverses it; D-22's warp->axum port is WP-71's spec, not this one.


### WP-07-game-client-rand-bot.md
- verdict: KEEP
- lines: 845
- tier: Tier 1 pre-tiering - same "REQUIRED SUB-SKILL" banner and Architecture/Tech Stack/Global Constraints/Non-Goals preamble, with exhaustive line-number citations (lib.rs:17-33, 47-89, 113, 212; operator/src/controller.rs:230; bot/src/main.rs:786-790). README's Tier 1 roster names WP-07.
- risk-to-executor: high - the package is genuinely unlanded, but the executor will be reading ~845 lines whose line numbers are pre-`a543120` and pre-concurrent-edit, so it must locate code by symbol and ignore every numeric citation.
- evidence: Nothing in this package has landed. `lib/game_client/src/lib.rs` still uses `anyhow::{Context, Result, anyhow}` throughout (ls F32 untouched), `lib/game_client/Cargo.toml` still lists `anyhow = "1.0.103"` with no `thiserror`, the retry predicate is still the narrow `e.is_connect() || e.is_timeout()` (ls F33), and there is no crate-level timeout ceiling (ls F31, the major). `lib/rand_bot/Cargo.toml` still carries `chrono = { version = "0.4.45", features = ["serde"] }` (ls F40) with no `default-features = false` on `brdgme_cmd` (ls F42). The only overlap with the landed WP-06 is rand_bot's `main.rs` extern line and the lib.rs comment block, which WP-07 already declares a non-goal owned by WP-06 Task 4 - so the overlap is already handled by the spec text and does not make it stale. No decision in `decisions-ANSWERED.md` or `decisions-session3.md` touches WP-07.


### WP-08-finish-placings-epilogue-dedup.md
- verdict: KEEP
- lines: 150
- tier: Tier 2 compact - Findings header, "no line numbers are cited on purpose" banner, six-section skeleton with a per-crate riders work list. Not in `README.md`'s original Tier 2 bullet list by number but sized and shaped as Tier 2; the one arm-count column is explicitly marked "approximate, verify before editing".
- risk-to-executor: low - the only numbers in it are self-labelled approximate; the refactor shape is decided up front (per-crate private helper, no `lib/game` API) so the executor has no design latitude.
- evidence: Not landed - grepping all of `rust/` for `finish_epilogue` returns zero hits, so section 3a's helper does not exist in any of the thirteen crates. No commit matches `WP-08`. The spec's own Non-goals reference still-live sibling packages (WP-28 Task 4, WP-25) as reasons not to widen, and it names the routed-in acquire-1 and starship-catan-1 items that `work-packages.md`'s WP-08 scope line does not - i.e. it is a superset of, not superseded by, the package definition.


### WP-09a-deserialized-state-boundary.md
- verdict: KEEP
- lines: 166
- tier: Tier 2 compact - Findings / Routed-in / Decision / Crate list / Landing order header, "line numbers are deliberately not cited" banner, six-section skeleton. `README.md` lists WP-09a in the Tier 2 roster (WP-09 split into 09a + 09b).
- risk-to-executor: low - symbol-located, and it carries the explicit red-test-first instruction plus a warning not to "fix" the deliberately-panicking `self.hands[player]` that WP-28 Task 3 depends on.
- evidence: Not landed - `rust/lib/game/src/game.rs` has no `fn validate` and `rust/lib/cmd/src` has no `check_player`, so neither 3b nor 3c exists. `decisions-needed.md` records D-36 as answered "A - bounds-check player index at requester boundary + per-game validate hook", exactly the shape 3b/3c implement; nothing reversed it. `work-packages.md` documents the WP-09 -> WP-09a/WP-09b split and names this file as the first-landing half, so it is the live spec for that half.


### WP-09b-game-crate-state-trust-sweep.md
- verdict: KEEP
- lines: 138
- tier: Tier 2 compact - README's Tier 2 roster lists WP-09b (the WP-09 split); has the Tier 2 skeleton (1 Problem / 2 Why it's wrong / 3 Required end state / 4 Non-goals / 5 Regression test cases / 6 Riders) and the explicit "no line numbers are cited on purpose" banner.
- risk-to-executor: low - compact, symbol-addressed, and it states its own hard predecessor (WP-09a) up front.
- evidence: The whole spec is one table of 18 rows implementing `Gamer::validate` per crate. Verified unlanded: `grep 'fn validate'` in `lib/game/src/lib.rs`, `game/modern-art-2/src/lib.rs` and `game/love-letter-2/src/lib.rs` returns nothing, so neither WP-09a's hook nor any of this package's impls exist. `work-packages.md` records WP-09 as READY with D-36 answered option A and confirms the WP-09a/WP-09b split at spec time. The one already-landed item it references (WP-29 Task 2's red7-1 saturating `end_points`) is explicitly fenced off as "do not widen or revisit", so partial-landing elsewhere does not contaminate it.


### WP-10-pub-state-hidden-info-redaction.md
- verdict: KEEP
- lines: 129
- tier: Tier 2 compact - Findings / Decision header, "No line numbers are cited on purpose; the few offered elsewhere are approximate, verify" banner, six-section skeleton. Listed in `README.md`'s Tier 2 roster.
- risk-to-executor: low - symbol-located and it is the canonical source for section 3a's redaction shape that later crates copy, so archiving it would lose a cross-package convention.
- evidence: Not landed in any of the three crates. Verified live: `rust/game/zombie-dice-2/src/lib.rs` still declares `pub cup: Vec<Dice>` on both `Game` and `PubState` and still does `cup: self.cup.clone()` in `pub_state` (`cup_counts` exists only as the pre-existing private render helper the spec asks to delete); `rust/game/for-sale-2/src/lib.rs` still has an unredacted `pub bids: Vec<i32>` on both structs with no `bid` field on `PlayerState`; `rust/game/starship-catan-1/src/lib.rs` still does an unconditional `peeking: self.peeking.clone()` in `player_state`. `decisions-needed.md` records D-33 answered option A (counts/aggregates public, secrets in `player_state`) and explicitly notes "D-33 is unaffected" by the D-35 parity park, matching the spec's own READY claim. The recent commit `4e0abe6` (WP-13, starship-catan-1 "sensor peek render") touched the render half only, not `player_state`, exactly as the spec's Non-goals anticipate.
# Classify batch 5


### WP-13-starship-catan-fixes.md
- verdict: ARCHIVE
- lines: 725
- tier: Tier 1 pre-tiering - "REQUIRED SUB-SKILL: superpowers:subagent-driven-development" banner, Tech Stack / Global Constraints / Non-Goals / Snapshot drift sections, 9 numbered Task blocks, dense live-line-number citations ("lib.rs:1064-1066", "render.rs:125").
- risk-to-executor: high - 725 lines of pre-landing line-number citations against a file the fix commit already rewrote by ~259 lines, so an executor would chase stale offsets and re-apply landed work.
- evidence: The spec's full roster is Tasks 1-9 covering a F11 (cannon surcharge), a F12 (can_lose_module), a F14 (amount cap), a F13 (astro check), a F15 (Sensor peek render), a F16 (turn row), a F18 (direction error), a F19 (last_sectors cap), a F17 + a F20-comment (dead code); a F20 itself is an explicit non-goal. Commit `4e0abe6` enumerates all nine in its subject and touches card.rs/command.rs/lib.rs/render.rs. Verified live in `rust/game/starship-catan-1`: `command.rs` now uses `Int::bounded(1, MAX_TRADE_AMOUNT)` (Task 3), `render.rs::render` takes `peeking: Option<&[SectorCard]>` and emits peek rows gated on the viewer (Task 5), `can_lose_module` is now `current_player == player && self.losing_module` with the `||` gone (Task 2), tests `cannon_surcharge_keys_off_cannons_not_boosters`, `trade_and_build_buy_requires_astro`, `trade_and_build_buy_allows_exact_astro`, `last_sectors_capped_on_flight_end` exist (Tasks 1, 4, 8), and `start_card` no longer appears anywhere in lib.rs (Task 9). No task remains outstanding. Note: `specs-LOG.md` line ~2889 says "WP-13 has not landed" - that entry predates commit `4e0abe6`; live source wins.


### WP-14-alhambra-core-fixes.md
- verdict: ARCHIVE
- lines: 715
- tier: Tier 1 pre-tiering - same subagent-driven-development banner, Architecture / Tech Stack / Global Constraints / Snapshot-drift preamble, 10 numbered Task blocks, live line-number citations ("card.rs:516", "lib.rs:1028").
- risk-to-executor: high - the fix commit rewrote ~450 lines across card.rs/lib.rs/render.rs, invalidating essentially every cited offset in a 715-line document whose work is already done.
- evidence: Roster is Tasks 1-10 = b F16, F17, F18, F21, F23, F24, F25, F26, F27, F28. Commit `c52f1a5` lists all ten findings individually in its body. Verified live in `rust/game/alhambra-1/src/card.rs`: `grid_tile_counts` exists as the single shared helper (Task 8, b F26), the flood walks now use `VecDeque` + `HashSet` (Task 10, b F28), and `card.rs` imports `{HashMap, HashSet, VecDeque}`. `lib.rs` now has 33 `#[test]`s, consistent with the F21 coverage inventory (Task 4). Nothing in the spec is deferred to a later package except explicit non-goals (b F19/F20 -> WP-16, b F22 -> WP-08), which are other packages' work.


### WP-15-seven-wonders-mechanical.md
- verdict: ARCHIVE
- lines: 875
- tier: Tier 1 pre-tiering - subagent-driven-development banner, long Architecture / re-derivation-notes preamble, 9 Task blocks plus a Findings-disposition table, pervasive line-number citations ("lib.rs:701-725", "card.rs:1269").
- risk-to-executor: high - largest spec in the batch, fully landed, and the Task 9 module split moved the very functions its line numbers point at out of lib.rs.
- evidence: Roster is Tasks 1-9 = b F1, F3, F2, F9, F12, F13, F10, F14, F15. Commit `52680e5` names exactly `b F1 F2 F3 F9 F10 F12 F13 F14 F15` - that is the complete roster, not a subset, so the "lists specific ids suggests partial" hypothesis does not hold here. Verified live in `rust/game/seven-wonders-1`: `src/scoring.rs` and `src/trade.rs` now exist carrying `player_vp`/`science_vp`/`mimic_guild_vp` and `can_afford_cost`/`resolve_deal`/`pay_cost` (Task 9, b F15), `lib.rs` has the `#[serde(default)] deal_coins: Option<HashMap<i32,i32>>` field alongside legacy `deal` and `resolve_deal(player, cost, deal, deal_coins)` (Task 4, b F9), and `check_hand_complete` calls `prune_resolvers()` (Task 3, b F2). `lib.rs` now has 37 `#[test]`s (Task 8, b F14). Remaining b F4-F8/F19/F20 are WP-16's, listed as non-goals here.


### WP-17-lib-cost.md
- verdict: KEEP
- lines: 178
- tier: Tier 2 compact - README's corrections section names `specs/WP-17-lib-cost.md` as a newly written spec; carries the Tier 2 skeleton, the "no line numbers are cited on purpose" banner, and an explicit self-declared, Lead-accepted overrun of the ~120-line cap.
- risk-to-executor: low - scope is fenced to 3 of WP-17's 8 findings, the other 5 are routed to `checklists/T3-B3`, and it ships a read-only verification section.
- evidence: Unlanded and unambiguous. `game/splendor-2/src/cost.rs` still declares `pub struct Cost(pub HashMap<Resource, i32>)` and `pub fn from_resources`, and `lib/cost/src/lib.rs` has no `pub fn get` / `pub fn set` - i.e. neither section 2 nor section 3 has been done. D-25 in `decisions-ANSWERED.md` is ANSWERED as option A (port splendor-2 onto `lib/cost`) with the automated-testing constraint the spec's section 4 encodes, and it explicitly notes D-25 gates only 3 of WP-17's 8 findings, matching the spec's scope split. Option B (delete `lib/cost`) is closed, so nothing here relies on a reversed decision.


### WP-19-acquire-fixes.md
- verdict: KEEP
- lines: 841
- tier: Tier 1 pre-tiering - carries the `> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development` banner, an "Architecture" dump, a "Snapshot drift" section, and dense live-file line-number citations throughout (the exact pattern the tiering retired). `specs-LOG.md:485` records it as written at 841 lines in the pre-Tier-2 backlog wave.
- risk-to-executor: high - ~841 lines of exhaustive `lib.rs:NNN` / `command.rs:NNN` citations against a crate under concurrent edit; every one must be re-verified by symbol name before touching anything.
- evidence: Nothing in this package has landed. Verified live in `rust/game/acquire-1`: `player_counts()` is still `(2..6).collect()` (Task 1 unlanded), `bonus_players` still rolls `self.rng.random_range(1..=5)` (Task 2), `panic!("expected some major bonus players")` still present in `pay_bonuses` (Task 3), the whole `.expect("could not get player shares")` cluster is still in `lib.rs`, `command.rs` and `render.rs` including the `"could not et player shares"` typo (Task 4), `stats.rs` still has `s.insert("Trades".to_string(), Stat::Int(self.merges as i32))` (Task 5), and `thiserror = "2.0.18"` is still in `Cargo.toml` (Task 6). `git log --all --grep 'WP-19'` returns nothing. The WP-81 supersession is **section-internal, not whole-file**: `work-packages.md` (WP-19 entry) says only that `c F11` is superseded and "Land WP-81 first and DROP Task 5"; Tasks 1-4 and 6-9 are untouched by it, and WP-81 is a separate READY package that deletes `acquire-1/src/stats.rs`. KEEP with an execution note: DROP Task 5 (and its test) if WP-81 has landed; also honour the spec's own non-goals fencing WP-20 (c F13/F14/F15) and c F12.


### WP-21-cathedral-sushizock-fixes.md
- verdict: ARCHIVE
- lines: 1081
- tier: Tier 1 pre-tiering - same `REQUIRED SUB-SKILL` banner, Architecture/Tech-Stack/Snapshot-drift preamble, exhaustive live line numbers.
- risk-to-executor: high if kept - 1081 lines of line-number citations against two crates that have since been rewritten by the landing commit, so essentially every citation is now stale.
- evidence: The spec's 10 tasks map 1:1 onto findings c F22, c F23, c F24, c F25, c F26, c F27, c F28 (cathedral-2) and c F29, c F30, c F32, c F33, c F34 (sushizock-2) - the full 12-finding scope listed for WP-21 in `work-packages.md`. Commit `f547238` names every one of those 12 findings in its body and touches exactly the spec's file set (cathedral-2 `command.rs`/`lib.rs`/`loc.rs`/`piece.rs`/`render.rs`/`Cargo.toml`, sushizock-2 `lib.rs`, plus `Cargo.lock`). Verified in live source rather than trusting the message: `grep` finds no `Box::leak`, no `LocChoice` and no `parse_loc` anywhere in `cathedral-2/src`; `piece.rs` now reads `pub fn pieces(player: i32) -> Option<Vec<Piece>>`; `rand` is gone from `cathedral-2/Cargo.toml`; `sushizock-2/src/lib.rs` carries an explicit `c F29` `i32::MIN` guard comment plus a matching test, and `log_game_end` is called from the roll path. No task remains outstanding - full landing, ARCHIVE.


### WP-22-lords-of-vegas-fixes.md
- verdict: KEEP
- lines: 736
- tier: Tier 1 pre-tiering - README's Tier 1 roster names WP-22; markers: "For agentic workers / REQUIRED SUB-SKILL" header, `**Goal:**`/`**Architecture**`/`**Tech Stack:**`/`**Global Constraints:**`/`**Non-Goals:**` preamble, per-task `- [ ]` step lists, and dense live line-number citations throughout ("Line numbers cited are LIVE-file numbers").
- risk-to-executor: high - ~736 lines saturated with exact line numbers taken against a 2026-07-25 snapshot, so citation rot is likely even though the fixes are real.
- evidence: Nothing has landed. `git log --oneline --all --grep 'WP-22'` is empty, and `git log --all -- rust/game/lords-of-vegas-1` shows no review-era commit (newest is `4ca42fc` V2 upgrade). Live source still has all five `unimplemented!()` arms at `rust/game/lords-of-vegas-1/src/lib.rs` (Command::Remodel/Reorg/Sprawl/Gamble/Raise), `lazy_static` still in use in `src/tile.rs`, and `src/render.rs` has no `saturating_sub` - so d F1, d F6 and d F4 are all outstanding. `work-packages.md` line 362 says "WP-22 lords-of-vegas-1 - READY". The spec's Non-Goals defer d F5 to WP-09 and d F12 to WP-26 (parity park), both still parked, so nothing it depends on was reversed.


### WP-23-jaipur-fixes.md
- verdict: KEEP
- lines: 637
- tier: Tier 1 pre-tiering - in README's Tier 1 roster; same markers as WP-22 (sub-skill header, Architecture/Tech Stack/Global Constraints/Non-Goals preamble, checkbox step lists, exhaustive live line numbers).
- risk-to-executor: high - 637 lines of line-number-heavy prose for what is one major plus four nits/minors; the citations are the failure mode, not the fixes.
- evidence: Nothing has landed. `git log --all --grep 'WP-23'` is empty and `git log --all -- rust/game/jaipur-2` shows no review-era commit. Live checks confirm every task is outstanding: `bonus_sizes()` in `rust/game/jaipur-2/src/lib.rs` still returns `3..=5` with tests asserting `bonus_sizes_are_3_to_5` (d F14 unfixed), `RULES.md` is still a 1-line stub (d F17), and `parsers.is_empty()` is still present in `rust/game/jaipur-2/src/command.rs` (d F19). `work-packages.md` line 369: "WP-23 jaipur-2 - READY". No decision reversal touches it - work-packages.md line 58 explicitly lists WP-23 as unaffected by the D-35 parity park.


### WP-25-modern-art-liveness.md
- verdict: ARCHIVE
- lines: 525
- tier: Tier 1 pre-tiering - in README's Tier 1 roster; five `### Task N` sections with checkbox steps plus a closing "Findings disposition" table.
- risk-to-executor: high if opened - it is fully superseded by landed code, and re-executing Task 1 would rewrite the already-corrected round-boundary logic.
- evidence: The spec's Goal covers exactly d F34, F35, F41, F42, F39, F40, F44, F45, F46 across Tasks 1-5, and five commits land exactly that set, each naming WP-25: `7821938` (d F34 d F35), `af2c014` (d F41), `b0babb8` (d F42), `e560a75` (d F39 d F40), `6c0c19c` (d F44 d F45 d F46). Verified in live source: `advance_past_empty_hands` exists in `rust/game/modern-art-2/src/lib.rs` and is called from both round boundaries, and the Task 5 nits are present as `is_some_and(|h| !h.is_empty())` and `is_none_or(|&b| b > 0)`. No task remains. `work-packages.md` line 382 still says READY, but live source wins.


### WP-28-lost-cities-shared-fixes.md
- verdict: KEEP
- lines: 730
- tier: Tier 1 pre-tiering - "REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development" header, 10 tasks of `- [ ]` steps, byte-exact "replace line NNN" instructions and pervasive `lib.rs:NNN` / `render.rs:NNN` citations.
- risk-to-executor: high - 730 lines, every fix addressed by line number across two crates plus k8s manifests; citation rot is the main hazard, not wrongness.
- evidence: Nothing has landed. Verified in live source: `rust/game/lost-cities-2/src/lib.rs` `status()` still emits `stats: vec![self.player_stats(0), self.player_stats(1)]` (Task 1); `rust/game/lost-cities-2/src/render.rs` still has `let p = player.unwrap_or(0) % MAX_PLAYERS;` (Task 5); neither crate sorts `hand` in `player_state()` - the only `sort()` in either lib.rs is `drawn.sort()` on the private draw log (Task 3); both `k8s/base/game/lost-cities-{1,2}/game-version.yaml` blurbs still say "A tense two-player card game" (Task 10). No commit mentions WP-28 and the newest commit touching either crate predates the review. `work-packages.md` lists WP-28 as READY with all 13 findings. Decision drift is cosmetic only: the spec routes e F21/e F39/e F40 to "WP-30 (D-29/D-40)", and D-40 has since been answered (option B) with e F39/e F40 re-homed to the new WP-81 - that changes the owner name, not any instruction in this spec, and WP-28 touches no `Stats` field either way.

### WP-29-red7-cleanup.md
- verdict: KEEP
- lines: 467
- tier: Tier 1 pre-tiering - "REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development" header, `- [ ]` step checklists, byte-exact replace-line-NN instructions, hundreds of `lib.rs:NN` / `card.rs:NN` citations.
- risk-to-executor: high - the whole spec is line-addressed ("Replace `lib.rs:22-24`", "Replace `DATA_DOCS.md:36`") and Task 4 is now actively wrong; see evidence.
- evidence: Nothing has landed. Live `rust/game/red7-1/src/lib.rs:16` still reads `pub use card::{Card as PubCard, Suit as PubSuit};` (Task 1), `end_points` has no `saturating_sub` (Task 2), there is no PRECONDITION comment on `leader_with_suit` (Task 3), and `DATA_DOCS.md` still carries both "highest even card" (line 31) and "highest card overall in the palette" (line 36) (Task 4); no commit references WP-29 and the last red7-1 commit is WP-01's parser work. Tasks 1, 2, 3 and 5 are all still the live actionable work for this package, so it must not be archived - it is red7-1's only spec. **Task 4 is INVALIDATED by a later decision and must not be executed as written:** `decisions-ANSWERED.md` (the `e F30` row) rules that DATA_DOCS.md's second tie-break clause "then by the highest card overall in the palette" is CORRECT and officially supported and that the CODE must be changed to implement it (fall through to the full palette's `rank_key` max), whereas Task 4 declares that sentence fictional and deletes it, replacing it with "There is no further tie-break". The spec's SEQUENCING section is also stale: it treats D-29/D-40 as open, but D-40 is ACCEPTED (option B, split to WP-81, red7 unaffected) and the `e F30` half of D-29 is released from the park while only the empty-winning-set half stays parked.


### WP-34-auth-races-session-mechanical.md
- verdict: KEEP
- lines: 170
- tier: Tier 2 compact - named in README's Tier 2 roster; Tier 2 skeleton plus a Riders table, and an explicit "no line numbers are given deliberately; earlier specs had a 33-46% citation error rate" note.
- risk-to-executor: low - symbol-addressed, and it already warns that migration number `023` is not guaranteed and to `ls rust/web/migrations/` at write time.
- evidence: Nothing landed. `grep 'cycle_id\|login_email_sends\|random_range'` over `web/src/auth/server.rs` returns nothing, so F3's session rotation, F6's windowed-cap table and F12's unbiased code generation are all absent; `web/migrations/` ends at `022_concede_bot_replacement.sql`, so the new `login_email_sends` migration has not been written. `work-packages.md` lists "WP-34 auth races and session mechanical - READY". The spec already carries the two binding corrections (F1's `>` not `>=` off-by-one, F6's rejection of the finding's option 1) and its predecessor claim (WP-34 before WP-35) matches WP-35's own landing-order block.


### WP-35-auth-edge-semantics-fail-open.md
- verdict: KEEP
- lines: 215
- tier: Tier 2 compact - README's Tier 2 roster names WP-35; Tier 2 skeleton plus Riders table and the same deliberate-no-line-numbers note.
- risk-to-executor: medium - not for content quality but for sequencing: it declares a hard chain WP-41 -> WP-36 -> WP-34 -> WP-35, and it instructs edits to `rust/web/src/db.rs` while assuming the WP-82 `db.rs` split has NOT happened.
- evidence: Nothing landed - `grep 'logout_everywhere\|invalidate_all_auth_tokens\|ALLOW_INSECURE_DEFAULT_KEY\|MissingKey'` across `web/src` returns zero hits, so F11's revoke-all and F16's fail-fast key loading are both absent. Two of its three predecessors are in: WP-41 (`baa5fc6`) and WP-36 (`13a1e69`); WP-34 is not, consistent with the KEEP above. `work-packages.md` shows "WP-35 ... READY (D-12 + D-14 answered: A, MODIFIED - no session expiry; email change requires re-verification)", exactly the decision state the spec cites, and the spec already encodes the two rejections (F4's uniform-rejection recommendation, F11's expiry half) rather than depending on a later reversal. `web/src/db.rs` is still a single file, so its db.rs instructions are still addressable.


### WP-36-crypto-deploy-hardening.md
- verdict: ARCHIVE
- lines: 423
- tier: Tier 1 pre-tiering - "REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development" header, per-task `- [ ]` step checklists, exhaustive `file.rs:NN-NN` citations throughout.
- risk-to-executor: high - ~423 lines of dense line-cited instructions for work that is already fully in the tree; re-applying would double-add deps, tests and select arms.
- evidence: Commit `13a1e69 fix(web): crypto and deploy hardening (WP-36)` exists. All five tasks verified landed in live source: `secure_cookie(env_value: Option<&str>)` plus the three named unit tests in `rust/web/src/auth/session.rs`; `k8s/dev/web-patch.yaml` present and `SECURE_COOKIE=false` in the Tiltfile local web `serve_cmd` and `rust/web/.env.template`; the `rustls::crypto::aws_lc_rs::default_provider().install_default()` block at the top of `rust/web/src/main.rs`; `Zeroizing<[u8; 32]>` returns from `default_key`/`load_key` with `bytes.zeroize()` in `rust/web/src/crypto.rs`; `CancellationToken`/`TaskTracker` fields on `GameBroadcaster` with `begin_shutdown` in `rust/web/src/websocket.rs` and the `broadcaster.begin_shutdown()` call in `main.rs`. Nothing in the spec remains unlanded.


### WP-37-admin-pass.md
- verdict: ARCHIVE
- lines: 2355
- tier: Tier 1 pre-tiering (SUBSKILL banner line 3; bare `:NNNN` citation form throughout)
- risk-to-executor: high - it is a 13-task plan whose every task is already implemented; a cheap model handed it would re-apply edits against code that no longer matches any quoted "before" block, and several steps are phrased as replace-this-region.
- evidence: The work has landed in full. Commit `b49df61` "fix(web): admin.rs quality pass - dedup gate, atomic reorder, validation, capping (WP-37)" says "Implements all 14 confirmed findings", lists exactly the spec's 14 ws findings, and records the same two exclusions the spec's Non-Goals state (ws F27 fenced to WP-38, ws F30 rejected - no `updated_at` column). Verified in live `rust/web/src/admin.rs`, not just from the message: `ADMIN_REQUIRED` const + `async fn require_admin` (Tasks 1/2), `BotProviderTestRow` (3), `fn mask_api_key` (4), `BOT_DISPLAY_ORDER_LOCK` + `FROM unnest($1::uuid[]) WITH ORDINALITY` (6), `rows_affected()` checks on the updates (7), `pub enum ApiKeyUpdate` (8), `require_text`/`validate_temperature` + the 8192 extra-body cap (9), `SELECT model FROM bot_providers` replacing the hardcoded literal (10), `MAX_TEST_BODY_BYTES`/`read_capped_body`/`allowlisted_headers` (11), and `grep -c "value().get().unwrap()"` = **0** (12/13). Caveat for the destructive pass: `work-packages.md` still reads "### WP-37 admin.rs pass - READY" and `git log --grep 'WP-37'` returns only that one commit, so the status line is stale rather than corroborating - the live source is the proof. The spec's "Cross-package / newly discovered" section (items 1-5, incl. the `AdminPage` raw-`Display` routing and the empty-string API key) is the only content not represented in code; it is routing advice for the Lead, already mirrored in `WP-54`'s cross-package list, so archiving does not lose an actionable instruction.


### WP-38-bot-turn-wedge-recovery.md
- verdict: KEEP
- lines: 175
- tier: Tier 2 compact - README's Tier 2 roster names WP-38; Tier 2 skeleton with lettered subsections 3a-3d, a Riders table, and the "no line numbers are cited on purpose" banner.
- risk-to-executor: low - it names its predecessor (WP-37, landed) and fences off everything WP-39 shipped, which is the main confusion risk here.
- evidence: Unlanded. `grep 'bot_turn_wedge_total\|AckKind::Progress\|MAX_DELIVER\b\|bot_turn_sweep_republished_total\|dangling'` over `web/src` and `bot/src` matches only WP-39's already-shipped `MAX_DELIVERIES_ADVISORY_SUBJECT` machinery in `web/src/nats.rs` - none of WP-38's own counters, sweep, `pub const MAX_DELIVER`, or ack heartbeat exist. Both predecessors it names have landed (WP-37 `b49df61`, WP-39 `347970a`), and its Non-goals section already enumerates the WP-39 items so the executor will not redo them. `decisions-ANSWERED.md` confirms D-5 C-lite MODIFIED plus N-1 (15-minute sweep threshold, 60s Progress cadence), which are exactly the values sections 3a and 3c use - no reversal.


### WP-39-bot-consumer-supervision.md
- verdict: ARCHIVE
- lines: 965
- tier: Tier 1 pre-tiering - the bloated shape (agentic sub-skill banner, "Architecture - how the pieces fit today", Tech Stack / Global Constraints / Non-Goals / Snapshot drift preamble) and dense exhaustive line citations (`main.rs:55-74`, `game/mod.rs:391-398`, `bot/src/main.rs:242`, ...); WP-39 is on README.md's Tier 1 roster.
- risk-to-executor: high - ~1000 lines of pre-drift line citations for work that is already done; an executor opening it would re-apply landed changes against stale line numbers.
- evidence: Fully landed by commit 347970a ("fix(web,bot): supervise bot.command consumer, advisory listener, config drift, conflict targeting, bot retry/shutdown (WP-39)"). Verified every task in live source: Task 1 `web::nats::supervise_consumer` called from `rust/web/src/main.rs` with `nats_consumer_restarts_total`; Task 2 `MAX_DELIVERIES_ADVISORY_SUBJECT` + `bot_stream_max_deliveries_total` in `rust/web/src/nats.rs`; Task 3 `stream_config_drift`/`consumer_config_drift` there too; Task 4 the conflict re-publish in `rust/web/src/game/mod.rs` now `.filter(|t| t.position == event.player_position)`; Task 5 zero `unreachable!` left in `rust/bot/src/main.rs`; Task 6 `Semaphore` + `MAX_CONCURRENT_TURNS` + SIGTERM/ctrl-c handler; Task 7 the declined-DB-check rationale as a doc comment on `healthz`. The commit message lists exactly the spec's finding set (ws F53/F56/F57/F58, wd F4/F9, bo F1/F3/F5/F8).


### WP-40-undo-concede-toctou-ratings-integrity.md
- verdict: KEEP
- lines: 477
- tier: Tier 2 compact - written in the newer lean format despite sitting on the Tier 1 roster. Marker evidence: an explicit "How to use this spec" section stating every reference is by file path + function name and that line numbers "appear only as navigational hints and are marked *approximate, verify*"; no per-line architecture map; no "REQUIRED SUB-SKILL" banner.
- risk-to-executor: low - locate-by-symbol discipline plus an explicit STOP-and-report rule if the code does not match; the only live hazards are two stale ordering claims, noted below.
- evidence: Nothing in the package has landed. Verified in live `rust/web/src/`: no `claim_unfinished_game_tx`, `GameAlreadyFinished`, `undo_core`, `concede_core`, `ActingPlayer` or `conflict_or_internal` anywhere; `db::concede_game`, `db::concede_game_replace` and `db::undo_game` still carry their pre-WP-40 signatures with no `expected_updated_at` parameter; `email/commands.rs` still calls `crate::db::concede_game_replace`, `crate::db::concede_game` and `crate::db::undo_game` directly (the exact grep the spec's Task 4 says must return nothing). Its binding decisions are intact, not reversed: `decisions-needed.md` records "D-3 (+ D-4) | A - forbid undo once finished; yes to shared `undo_core`/`concede_core`". The specs-LOG "rewind via stored deltas is SUPERSEDED" note is **section-internal, not whole-file** - the spec itself already carries a "SUPERSEDED NOTE" block near its top voiding that `work-packages.md` phrase and forbidding any rewind, so the spec is the corrected artifact, not the victim.
- caveats for the executor (do not archive over these): (1) its stated predecessor **WP-41 has now landed** (commit `baa5fc6`), so the spec's "if WP-41 has NOT landed, stop" branch is discharged - `specs-LOG.md` ~:2299 still asserts the opposite and is stale. (2) **Its db.rs-split ordering is INVERTED by a later decision.** The spec says "the db.rs module split (ws F42) is a separate future package that must land **after** your db.rs edits" (~:82-83, and again in Non-Goals ~:450), but `landing-order.md` section 7.1 and the `work-packages.md` WP-82 entry now make **WP-82 a HARD PREDECESSOR of WP-40**, with db.rs becoming `web/src/db/`. Following the spec's ordering as written would do harm. (3) Predecessor **WP-59 has not landed**: `classify_server_fn_error` does not exist in live `rust/web/src`, and Task 4 consumes it.


### WP-41-db-quality-pass.md
- verdict: ARCHIVE
- lines: 1990
- tier: Tier 1 pre-tiering - carries the full pre-tiering apparatus (per-finding disposition table, exhaustive line-number architecture map, "REQUIRED SUB-SKILL" banner, Snapshot drift section, 11 numbered tasks). Citations were later audited and repaired in place, but the format is Tier 1.
- risk-to-executor: high - 1990 lines of dense line-cited instructions for work that is already in the tree; re-running Task 1's `updated_at = NOW()` sweep or Task 3's sticky-finish edit against the post-landing file would be destructive churn.
- evidence: Commit `baa5fc6` "fix(web): db.rs quality pass ... (WP-41)" enumerates every one of the package's 16 findings (ws F35-F51) in its body, including the `.sqlx` regeneration the spec's Global Constraints require. Verified in live `rust/web/src/db.rs`: the `# Module map` header comment (:27, F36/F42 doc half), the `game_proposals has NO update_updated_at trigger` exclusion note (:1578) matching the spec's must-exclude rule, `pg_advisory_xact_lock` in `send_friend_request` (:2187, F39), `make_interval(secs => $1::double precision)` (:3303, F47), sticky finish `is_finished = ($2 OR is_finished)` (:1991, F37), `is_user_admin` now on the file-wide `Result` alias (:639, F45), the F49 clone removal with an explanatory comment (:1060-1062), and an `is_user_admin_true_false_and_unknown_user` `#[sqlx::test]` (:7194, F35 coverage). `work-packages.md` still labels WP-41 READY (stale), but its own WP-82 entry at ~:1270 states "Measured 2026-07-26 against the live post-WP-41 tree (WP-41 landed, +1397/-125)" - and the live file is 8149 lines vs the spec's stated 6877, consistent with that. The one non-landing item, F42's module split, was already FENCED out of this spec by its own disposition table and is now owned by WP-82, so nothing actionable remains here.


### WP-42-websocket-auth-and-filtering.md
- verdict: KEEP
- lines: 247
- tier: Tier 2 compact - README's corrections note WP-42 was promoted from Tier 3 into a compact spec; Tier 2 skeleton, "no line numbers are cited on purpose" banner, and a supersession table at the head.
- risk-to-executor: low - the supersession is already applied in-place, and the filename-is-historical banner plus the DO NOT BUILD markers are unmissable at the top of the file.
- evidence: The SSE supersession is section-internal, not whole-file, and the spec has already been reworked to reflect it: a table at lines 19-24 marks §3a SUPERSEDED and §3d ELIMINATED while §3b (per-connection TTL cache) and §3c (`db.rs` predicates) SURVIVE, and new §3e decides WP-42 makes no edit to `websocket.rs`. `specs-LOG.md` records Worker 3 reworking this file in place (143 -> 244 lines) under D-44 and a later pass fixing its §3d cross-refs. `work-packages.md` line 562 states "READY, RESCOPED 2026-07-26 by D-44 ... filename kept so cross-references resolve". The surviving work is unlanded: `grep 'is_proposal_visible_to_user\|is_game_visible_to_viewer'` across `rust/` returns zero hits, and `web/src/db.rs` is still one file (WP-82's split, the spec's stated predecessor, has not landed - the spec already covers that case: "if the split has not landed, add it to `db.rs` as-is and say so").
# Classification batch 6


### WP-44-proposals-integrity-email-token-leak.md
- verdict: ARCHIVE
- lines: 795
- tier: Tier 1 pre-tiering - carries the `REQUIRED SUB-SKILL: superpowers:subagent-driven-development` banner, a "Snapshot drift" section asserting byte-identity with the review snapshot, and exhaustive absolute line-number citations throughout (`proposals.rs:70-81`, `:513`, `:1717-1765`, ...).
- risk-to-executor: high - the spec pins ~50 absolute line numbers against a file that has since been rewritten by the landing commit (+277/-144), so every citation is now wrong and re-executing would duplicate or corrupt already-fixed code.
- evidence: The whole package landed in commit `f4e7640` "fix(web): proposals integrity, email_token leak, and cleanup (WP-44)", whose 10 commit bullets map one-to-one onto the spec's 10 tasks (wd F26, F29, F30, F31+F35, F36, F40, F41, F42, F43, F44). Verified in live `rust/web/src/proposals.rs`: `ProposalPlayerView` (approximate line 63, verify) no longer has an `email_token` field and the roster SELECT no longer selects it; `RespondOutcome` is gone from the entire `web/src` tree; only `count_pending_human_invitees_tx` survives (the pool variant is deleted); sweep queries bind `($1 * interval '1 second')` rather than text concat; owner-decline guard exists ("The owner can't respond to their own proposal."); `transfer_target_error` enforces an accepted target. The spec's own Task 1 test `roster_view_never_exposes_email_token` is present in the inline test module. Nothing in the task list remains undone.


### WP-45-bot-slot-validation.md
- verdict: KEEP
- lines: 119
- tier: Tier 2 compact (FINDHDR + the no-line-numbers blockquote; zero `file.rs:NNN` citations)
- risk-to-executor: medium - symbol-addressed and safe to locate, but **one section is contradicted by a later ruling** (below) and a literal executor would implement the superseded behaviour.
- evidence: Not landed. `git log --grep 'WP-45'` is empty; `grep -rn validate_bot_slots rust/web/src/` returns nothing while `pub async fn find_enabled_bots` is live at `db.rs:616`, so section 3a's "add one function next to the existing `find_enabled_bots`" is still the work to do. `work-packages.md:629` reads "### WP-45 bot-slot validation choke point - READY (D-8 answered: option C ... validate on write, tolerate on read)". **Section-internal conflict, not whole-file supersession:** `decisions-ANSWERED.md` lists D-8 among the five rulings that CHANGED a recorded position - "**REFINED** - on restart, resolve a deprecated bot to the LATEST NON-DEPRECATED version of that bot ... The restart path now actively re-resolves rather than rejecting or no-opping", and that file states "where a ruling contradicts an older recommendation in ... a spec ... **this file wins**". WP-45 §3b's last bullet still says of `restart_core` "rejecting a now-disabled one is intended feedback", and §3c still routes the restart case to "D-5's dangling-name no-op" - both are the superseded text. D-8's core (validate on write, tolerate on read) is explicitly unchanged, so the other four call sites and the whole of §3a stand. The spec is the only spec its package has; it needs a one-paragraph amendment to §3b/§3c during the destructive pass, not archiving.


### WP-46-sweep-delivery-semantics.md
- verdict: KEEP
- lines: 229
- tier: Tier 2 compact - carries the Tier 2 banner "Read the named functions before editing... no line numbers are cited on purpose", Problem / Why it's wrong / Required end state / Non-goals / Regression tests / Riders structure, zero line-number citations.
- risk-to-executor: low - compact, no citation rot, but it declares a landing-order dependency (WP-51 preferably first, WP-38/WP-57/WP-76 adjacent) that the executor must respect.
- evidence: Nothing has landed. Verified live in `rust/web/src/email/sweep.rs`: `fetch_candidates` still ends with `FOR UPDATE SKIP LOCKED` (F31 unfixed), `is_reminder_candidate` still exists with its five unit tests (F37 unfixed), and there is no `ReminderOutcome` enum (F30 unfixed). `rust/web/src/db.rs` has no `delete_old_processed_webhook_events` (F11 unfixed). `git log --all --grep 'WP-46'` returns nothing. Both governing decisions are ANSWERED and match the spec: `decisions-ANSWERED.md` records D-11 as "ACCEPTED - option A: reminder_emails_enabled alone governs reminders" and D-2 option A; `work-packages.md` line 636 marks WP-46 READY with both decisions answered 2026-07-26. No reversal found.


### WP-47-game-visibility-gates.md
- verdict: KEEP
- lines: 120
- tier: Tier 2 compact (FINDHDR + the no-line-numbers blockquote; zero `file.rs:NNN` citations)
- risk-to-executor: low - purely symbol-addressed, carries the STOP-and-report instruction, and its "do not add a third copy of the predicate" guard is stated as a hard invariant.
- evidence: Not landed, and its stated preconditions verify true. `git log --grep 'WP-47'` is empty; `work-packages.md:649` reads "### WP-47 game_visibility gates - READY (D-6 + D-13 answered: option A, anonymize private users in stats)". In live `rust/web/src/db.rs`, `pub async fn is_game_visible_to_user` (:2547), `is_game_publicly_visible` (:2525) and `friend_recent_visible_game` (:2646) all exist with the existing `is_game_visible_to_user_*` tests the spec's §5 says to sit beside, while the two functions §3a adds - `is_game_visible_to_viewer` and `visible_user_ids` - return no hits anywhere. `stats/queries.rs` still has `opponents_by_game` (:206) and `head_to_head` (:476) with no viewer parameter. No decision reversal: neither D-6 nor D-13 appears among `decisions-ANSWERED.md`'s five changed rulings, and it remains the anchor of a live sequence - `landing-order.md` ~:151-155 "**re-ordered 2026-07-26 by the SSE pivot (D-44)** ... **WP-47 first**, so WP-42's per-connection [predicate] ... then **WP-84**". *(Bookkeeping gap worth noting, not a verdict factor: `decisions-ANSWERED.md` claims to cover D-01..D-34 but contains no `D-6` or `D-13` row at all - the ruling survives only in `work-packages.md` and this spec's header.)*


### WP-48-export-import.md
- verdict: KEEP
- lines: 129
- tier: Tier 2 compact - same "no line numbers are cited on purpose" banner, Problem / Why it's wrong / Required end state / Non-goals / Regression tests / Riders structure, records the D-7 scope shrink inline.
- risk-to-executor: low - short, cites only function names, and it pre-empts the main hazard by stating explicitly that the admin gate already exists and must not be rebuilt.
- evidence: Nothing has landed. Verified live: the `rust/web/src/game/export.rs` module doc still reads "Never includes email addresses - the bundle may get pasted into issues", the exact text section 3b says to rewrite (wd F7 outstanding). `rust/web/src/game/import.rs` still binds `is_turn_at`/`last_turn_at` to `NOW(), NOW()` in the `game_players` INSERT (wd F13 outstanding) and its `games`/`game_logs` inserts do not carry `created_at`/`updated_at` (wd F12 outstanding). `git log --all --grep 'WP-48'` returns nothing. D-7 is recorded twice in `decisions-ANSWERED.md` as OVERRULED with the same shrunk scope the spec encodes (admin-only full bundle, no redaction, no user-facing path), so the spec reflects the current, not a reversed, decision; `work-packages.md` line 663 agrees (READY, SCOPE SHRANK).
# Classification batch 7


### WP-49-rules-and-game-info-pages.md
- verdict: KEEP
- lines: 121
- tier: Tier 2 compact - 121 lines, the Tier 2 "Read the named function before editing... line numbers are deliberately omitted" banner, and the standard Problem / Why it's wrong / Required end state / Non-goals / Regression tests / Riders skeleton. `specs-LOG.md:2691-2724` records it written at 121 lines and ACCEPTED in Tier 2 batch T2-B2.
- risk-to-executor: low - compact, zero line numbers, locates everything by symbol name.
- evidence: Entirely unlanded. Verified live: `rust/web/src/game_info/queries.rs` still has `ORDER BY name LIMIT 1`; `rust/web/src/rules.rs` still uses `LocalResource::new`, still calls `get_current_user()` with the auth gate, still renders `{e.to_string()}` in the error arm, and has no `UnterminatedFence` variant; `rust/web/src/game_info/mod.rs` still has `pub use queries::*;`; `find_game_version_rules` in `db.rs` is still a bare `SELECT rules FROM game_versions WHERE id = $1` with no visibility predicate. No `WP-49` commit exists. Decision D-6 is answered option A ("rules public") in `decisions-needed.md:30,448` and `work-packages.md:681` - the direction the spec assumes, not reversed. The stated landing order (WP-41 before WP-49) and the do-not-do fences (no `db.rs` split, no `RULES.md`) are still current.


### WP-50-email-canonicalization.md
- verdict: KEEP
- lines: 179
- tier: Tier 2 compact - 179 lines (slightly over the ~120 cap, as `specs-LOG.md:4358` itself notes), Tier 2 banner, no line numbers, standard section skeleton.
- risk-to-executor: low - symbol-located, and it already self-corrects its own stale header (see below).
- evidence: Entirely unlanded. Verified live: no `canonicalize_email` anywhere under `rust/web/src`, no `rust/web/src/auth/email_addr.rs` (the `auth/` dir holds only `blocked_domains.rs`, `mod.rs`, `server.rs`, `session.rs`), and `rust/web/migrations/` stops at `022_concede_bot_replacement.sql` with no canonical-emails migration. No `WP-50` commit exists. D-9 is ANSWERED option B in `decisions-ANSWERED.md:48` - exactly what the spec implements, not reversed. The one reversal in play is already absorbed *inside* the spec: its header withdraws the old "WP-78 first" ordering and states WP-82 -> WP-50, which matches `work-packages.md:1262` where WP-82 is READY and explicitly supersedes WP-78. Its migration-number guidance is deliberately defensive (`ls migrations/` and take the next free number), so the `023` reference is not a rot hazard.

### WP-51-invite-mailer-notify-dedup.md
- verdict: KEEP
- lines: 1310
- tier: Tier 1 pre-tiering - `REQUIRED SUB-SKILL` banner, a "Snapshot drift" table claiming `diff -ru` byte-identity with snapshot `f8763a5`, an architecture section that pins ~60 absolute line ranges, and the statement "Line numbers below are live-file numbers verified 2026-07-25 at HEAD `0243472`". Only Tasks 5-7 opt out ("locate every edit by symbol name").
- risk-to-executor: high - 1310 lines of dense prose with heavy absolute-line citation, and its snapshot-drift table is now provably stale (see evidence), so a small model following numbers rather than symbols in Tasks 5-7 will edit the wrong place in `proposals.rs`.
- evidence: NOT landed - zero of the seven tasks. Verified in live source: `game::execute_command` (`rust/web/src/game/mod.rs`, approximate :79, verify) still returns `Ok(())` with no snapshot in its signature; `enum NotifyKind` in `web/src/email/notify.rs` still has exactly `Turn, Eliminated, Finished` with no `Reminder` (Task 3); `send_reminder` still exists in `web/src/email/sweep.rs` as its own body; all five `spawn_*` interval wrappers (`spawn_turn_reminder_sweep`, `spawn_unverified_email_sweep`, `spawn_invite_nudge_sweep`, `spawn_invite_expiry_sweep`, `spawn_invite_auto_decline_sweep`) are still separate (Task 4); `game_log_count` is unchanged and still called per-recipient (Task 2); all six `RealInviteMailer` methods including `notify_owner_decline` are intact and ungated (Tasks 5-7). `notify.rs` is 679 lines and `sweep.rs` 1046 - the exact counts the spec states, confirming those files have not moved at all. `work-packages.md:699` lists WP-51 READY with all 10 findings unresolved; no commit anywhere mentions WP-51.
- staleness note (does not change the verdict): the spec's drift table asserts `rust/web/src/proposals.rs` diffs empty against the snapshot and is 2961 lines. That is now FALSE - WP-44 landed (`f4e7640`) and proposals.rs is 3094 lines, with `trait InviteMailer` at approximate :102 (spec says :110-122) and `notify_owner_decline` at approximate :278 (spec says :286-328), roughly an 8-line shift. The spec anticipated exactly this ("proposals.rs line numbers shift if WP-44 lands first; Tasks 5-7 locate every edit by symbol name"), so the guidance is still correct - only the drift table needs a correction. Tasks 1-4's citations still resolve (`broadcast_and_trigger` :51-59 and `execute_command` :79 are exact in live `game/mod.rs`).
- also load-bearing elsewhere: `work-packages.md` (approximate :1187-1192, verify) records a new defect scoped by direct citation to `specs/WP-51-invite-mailer-notify-dedup.md:43`, stating WP-51 "explicitly refuses to absorb it; must NOT fold into WP-59 or WP-40" and that it becomes a five-line change "once WP-51 Task 1 returns the pre-command snapshot". Deleting this spec would orphan that reference.

### WP-54-frontend-ux-error-handling.md
- verdict: KEEP
- lines: 2051
- tier: Tier 1 pre-tiering (SUBSKILL banner line 3; ~280 `file.rs:NNN` citations)
- risk-to-executor: medium - enormous and heavily line-addressed with several "replace lines :A-:B inclusive" delete ranges, but its citations verify against live source today and the spec itself repeatedly warns that anchors shift as its own tasks land ("locate each edit by the quoted anchor text plus the enclosing symbol name, and treat the line number as a sanity check only").
- evidence: Nothing has landed and the tree has not drifted since the spec's 2026-07-25 repair pass. `git log --grep 'WP-54'` is empty; `grep -rn "action_error_message\|clamp_player_count\|FriendRequestCount\|browser_locale" rust/web/src/` returns **zero** hits (Task 1's helper, Task 9's pure fn, Task 7's newtype and Task 11's fn are all absent); all three `style="cursor:pointer"` sites Task 10 removes are still live at exactly the cited `app.rs:603`, `app.rs:623`, `layout.rs:170`. Every one of the eight target files still has exactly the line count the spec's "The eight files" table records (game.rs 681, app.rs 924, friends.rs 581, settings.rs 572, layout.rs 316, opponent_slot.rs 352, mod.rs 15, new_game.rs 660, error.rs 16), and `git log f8763a5..HEAD` over all nine paths shows only `1f665b0` - the single #47 commit the spec's own "Snapshot drift" section already accounts for. Spot-checked anchors match byte-for-byte: the five `ServerAction`s at :49-53, `<h3>"Actions"</h3>` at :110, `window_key` :305-308 / `format_log_time` starting :312. The spec's five in-file SUPERSEDED-style notes are all **section-internal repairs of its own earlier draft** (the adversarial-review banner's four "do not revert" corrections, plus wd F57's overturned recommendation) - none is a whole-file supersession. No decision reversal touches it: `decisions-ANSWERED.md`'s five changed rulings are D-7/D-8/D-15/D-16/D-37, and the spec's Non-Goals already fence D-15 (WP-59) and D-16 (WP-55) out explicitly. `landing-order.md` treats it as live and load-bearing (~:104-146: "WP-40's new conflict errors are INVISIBLE in the UI until WP-54 lands", "**WP-54** ... as soon as practical after WP-40"). `work-packages.md:720` reads "### WP-54 frontend UX error handling - READY". One forward-compat note for the Lead, not grounds for archiving: Task 1 writes `crate::websocket_client::bump_game_update` (module still live at `rust/web/src/websocket_client.rs`), which WP-84's SSE migration (D-44) will later rename - WP-54 lands well before that cluster per `landing-order.md` §10.


### WP-55-turnstile-spa-rendering.md
- verdict: KEEP
- lines: 148
- tier: Tier 2 compact - numbered `## 1. Problem` / `## 2. Why it's wrong` / `## 3. Required end state` / `## 4. Non-goals` / `## 5. Regression test cases` / `## 6. Riders` skeleton, explicit "no line numbers are cited on purpose".
- risk-to-executor: low - compact, symbol-located, and it already encodes the post-reversal design.
- evidence: The spec is written for the D-16 reversal, not against it: it says D-16 was OVERRULED and the `turnstile.render()` approach is CANCELLED, matching `README.md` and `work-packages.md` line 728 ("D-16 answered 2026-07-26: option B, OVERRULING the recommendation - SCOPE GREW"). It also already carries the grown scope (the three `use_navigate` redirects). Nothing landed: `grep -rn 'hard_navigate|rel="external"' rust/web/src/` returns nothing, and all five `/login` sites are still SPA - `settings.rs`, `components/layout.rs` (both the effect and the `<A href="/login">`), `app.rs`'s `index-cta` `<A>`, and `admin.rs`.


### WP-56-email-from-auth-redesign.md
- verdict: KEEP
- lines: 548
- tier: Tier 2 compact (SUBSKILL banner is a FALSE POSITIVE - CONFIRMED). It carries the `REQUIRED SUB-SKILL` line, but the body is written in the newer lean format: an explicit "How to use this spec" rule that code is identified by file + function name and that "Line numbers are navigational hints only, always marked 'approximate, verify'", and every one of its ~11 numeric citations is in fact so marked. No exhaustive per-task line ranges, no snapshot-byte-identity claim.
- risk-to-executor: medium - the content is sound and its hints still resolve, but Task 2 (SPF/DKIM) deliberately has no answer in-repo and instructs a STOP-and-report, and the migration number must be renegotiated (see evidence), so a small model could stall or collide.
- evidence: Nothing has landed. Verified in live source that every "approximate, verify" hint still points at the described code: `web/src/email/inbound.rs` still has `resolve_user_by_verified_from` (hint :393 area), the default-allow arm `Some(InboundRoute::Settings(_)) | None =>` (hint :484, exact), `format!("s-{user_id}@brdg.me")` in `send_settings_response` (hint :1191, exact), and the tests `resolve_user_by_verified_from_truth_table` (:1891, exact). `web/src/email/commands.rs` still defines `run_emails_add`/`run_emails_confirm`/`run_emails_active`/`run_emails_remove` and their `"add"`/`"confirm"` match arms. `settings_email_token` has zero hits across `web/src`, and `web/migrations/` still tops out at `022_concede_bot_replacement.sql` exactly as the spec states. Decision status intact, not reversed: `decisions-needed.md` records D-1 = option B plus the same 2026-07-25 narrowing the spec encodes (only `emails add`/`confirm`/`active`|`use`/`remove` leave email; `name`/`theme`/`colors`/notification prefs KEPT; cold start via web-UI opt-in reveal, never an email footer), and `work-packages.md:762` marks WP-56 READY on that basis.
- ordering note (WP-50 interaction, verified in `landing-order.md`): section 6.4 states "**WP-50 is independent of WP-56 and WP-59**" - there is NO logical ordering dependency. The only interaction is a migration-number collision: `landing-order.md` names WP-34, WP-50, WP-56 and WP-58 as all claiming `023`, with a renumbering rule to apply. WP-56's own spec says only "next free number; 022 is the highest today", so whoever executes it must consult `landing-order.md` rather than trusting that line. Also live: the WP-59 overlap on `handle_settings_reply`/`handle_settings_reply_route`, which the spec handles explicitly in both orders.


### WP-57-inbound-webhook-delivery-semantics.md
- verdict: KEEP
- lines: 140
- tier: Tier 2 compact - README's Tier 2 roster names WP-57; standard numbered 1-6 skeleton, "line numbers are omitted on purpose (the one marked approximate is a hint - verify it)".
- risk-to-executor: low - short, symbol-located, and it flags its own single approximate citation.
- evidence: Built on D-2 option A (at-least-once, dedupe after processing), which `work-packages.md` line 768 confirms as the standing answer with no later reversal. Nothing landed: in `rust/web/src/email/inbound.rs` there is no `RouteOutcome`, no `event_already_processed`, no `InvalidHeaderValue`, and `mark_event_processed` is still called early in the handler at roughly line 456 - exactly the wfe F2 shape the spec describes. `fetch_inbound_text` and `extract_addr_spec` also do not exist yet, confirming its predecessor WP-59 has not landed either, so the stated landing order (WP-59 first) is still live guidance rather than stale.


### WP-58-unsubscribe-rfc8058.md
- verdict: KEEP
- lines: 214
- tier: Tier 2 compact - README lists WP-58 among the eight formerly decision-blocked Tier 2 packages specced 2026-07-26; standard numbered 1-6 skeleton, "no line numbers are cited on purpose".
- risk-to-executor: low - longest of the Tier 2 batch but proportionate (new module, new migration, six call sites); no line numbers to rot.
- evidence: Written to D-10 option A plus the addition (one-click HTTPS endpoint plus two visible links), matching `work-packages.md` line 774 "D-10 answered 2026-07-26: option A plus an addition - SCOPE GREW"; the spec already contains the grown visible-links half, so it postdates the ruling. Nothing landed: `rust/web/src/email/` has no `unsubscribe.rs`, `EmailKind` and `unsubscribe_token` appear nowhere in `rust/web/src/`, the mailto `List-Unsubscribe` header is still emitted from both sites (`email/render.rs` and the hand-built `BTreeMap` in `email/inbound.rs`), and the highest migration is still `022_concede_bot_replacement.sql` so the 3e migration is unwritten.


### WP-59-inbound-processing-quality.md
- verdict: KEEP
- lines: 2795
- tier: Tier 1 pre-tiering (SUBSKILL banner line 3; ~150 `file.rs:NNN` citations)
- risk-to-executor: high - dense live line citations throughout (Tasks 1-13 give exact insert/replace ranges), plus Task 14 as written contradicts the now-settled D-15, so a cheap model following it literally would write the wrong documentation and could mis-target deletions.
- evidence: `work-packages.md` still lists "### WP-59 inbound processing quality - READY"; `git log --oneline --all --grep 'WP-59'` returns nothing and no rust/ commit references it, so none of the 14 tasks has landed. It is the only spec for the package. The one supersession is **section-internal, not whole-file**: `decisions-ANSWERED.md` records D-15 as "REDESIGNED ... game parser FIRST; platform commands are the FALLBACK", and `work-packages.md` says "**do not execute Task 14 as specced.** Rewrite the COMMANDS.md section to describe parser-first dispatch plus the escape-hatch set". Tasks 1-13 are untouched by that ruling. The spec's own text ("D-15 IS STILL OPEN ... DO NOT EXECUTE THIS TASK UNTIL THE LEAD CONFIRMS D-15") is stale but fails safe - it gates rather than misleads on the whole file.


### WP-62-operator.md
- verdict: KEEP
- lines: 166
- tier: Tier 2 compact - README's Tier 2 roster names WP-62; standard numbered 1-6 skeleton with a riders table, "no line numbers are cited on purpose".
- risk-to-executor: low - one stale paragraph, called out below, otherwise accurate and symbol-located.
- evidence: Nothing landed. In `rust/operator/src/controller.rs` the two hand-built `Patch::Merge(json!({"metadata": {"finalizers": ...}}))` blocks are still there (bo F18), status is still a hand-written `json!({"status": {"ready": true, ...}})` (bo F20/F21), `.bind(weight as f64)` is still present (bo F22), and `interceptor_uri()` still takes no argument and reads the env (bo F24); `src/crd.rs` still declares the `.spec.playerCounts` printcolumn (bo F19); `Cargo.toml` still has `k8s-openapi = { version = "0.28", features = ["latest"] }` (bo F25). One staleness to flag, not grounds for archive: section 4 says bo F25 is BLOCKED with an OPEN QUESTION for Michael, but `decisions-ANSWERED.md` line 44 answers it (cluster is k8s server v1.36.0, pin the `v1_36` feature, or the highest flag at or below v1.36 if it does not exist, recording the choice here). Per README, decisions-ANSWERED wins and the spec must be amended.
# Classification batch 9


### WP-63-fuzz-tool.md
- verdict: KEEP
- lines: 150
- tier: Tier 2 compact - explicit "no line numbers are cited on purpose" banner, "Read the named functions before editing... STOP and report", Problem/Why-it's-wrong/Required-end-state/Non-goals/Riders shape.
- risk-to-executor: low - symbol-addressed, no line citations, small scope.
- evidence: Nothing landed. Live `rust/tools/fuzz/src/lib.rs` still has `use std::time::{Duration, SystemTime}`, `for _ in 0..num_cpus::get()`, `let mut last_output_at = SystemTime::now()`, `tx.send(()).unwrap()`, and `player_render.clone().command_spec.unwrap()`; `num_cpus = "1.17.0"` is still in `rust/tools/fuzz/Cargo.toml`; there is no `mod tests` and no `drop(step_tx)`. No commit mentions WP-63 and the last commit touching `rust/tools/fuzz` is a dependency/edition sweep. `planning/fuzz-throughput-evaluation.md` does not supersede this spec: it reverses the fuzz half of D-41 (keep fuzzing in-process, reject `LocalRequester` for fuzz) but the `fuzz()` driver and `Fuzzer::command` it fixes are shared by the in-process `fuzz_gamer` path, and the evaluation's recommendation 6 explicitly endorses two of this spec's fixes (the `PlayerRender` clone). The bigger throughput rework (option d) is deferred to `docs/BACKLOG.md` #54 per the decisions header, so it does not block these mechanical fixes.


### WP-64-workspace-tables.md
- verdict: KEEP
- lines: 120
- tier: Tier 2 compact (FINDHDR + the "read every named file/table" blockquote variant; zero `file.rs:NNN` citations)
- risk-to-executor: medium - not from citation rot (there is none) but from scope: its binding Step 0 is "bump every direct dep to latest (root + all 40 members)", and §3d contains a genuine open question the spec orders the executor to escalate rather than decide.
- evidence: Not landed, decision intact. `git log --grep 'WP-64'` is empty; `grep -n "workspace.dependencies\|workspace.package\|workspace.lints" rust/Cargo.toml` returns **zero hits**, so all three tables dp F1/F2/F3 call for are still absent exactly as §2 describes. `work-packages.md:866` reads "### WP-64 workspace-deps migration - READY (D-19 answered 2026-07-26: option A, all three tables)", matching `decisions-ANSWERED.md` D-19 verbatim ("ACCEPTED - option A: `[workspace.dependencies]` **and** `[workspace.package]` **and** `[workspace.lints]` in one migration, early"); D-19 is not among the five changed rulings, and D-17's standing "upgrade to latest first" process change explicitly names WP-64 as bound by it, which is what §0 encodes. The spec has already corrected its own findings' stale counts ("dp F1 is correct; counts slightly stale ... serde 36, tokio 32, rand 32 ... findings said tokio/rand 33"). It is also a stated predecessor for other live packages (WP-70's landing-order note, `work-packages.md:882` "Best after WP-64", :963 "Sequence after WP-64").


### WP-66-sqlx-unification.md
- verdict: KEEP
- lines: 134
- tier: Tier 2 compact - explicit banner "No line numbers are cited on purpose; the tree is under concurrent edit ... STOP and report rather than improvising", numbered sections 0-6 with a riders table, zero line citations in the body.
- risk-to-executor: low - no line numbers to rot, a hard STOP rule, and Step 0 is a measurement gate before any structural change.
- evidence: Nothing has landed. Verified live: `rust/web/Cargo.toml` still declares `sqlx = { version = "0.8", features = ["runtime-tokio-rustls", ...] }` plus `tower-sessions = "0.14.0"` and `tower-sessions-sqlx-store = { version = "0.15.0", ... }`, while `rust/bot/Cargo.toml` and `rust/operator/Cargo.toml` declare `sqlx = "0.9"` - exactly the split dp F6 describes; `rust/Cargo.lock` still carries two `name = "sqlx"` entries; there is no `rust/lib/session_store/` member and no `[workspace.dependencies]` table in `rust/Cargo.toml`. D-17 is ANSWERED and not reversed: `decisions-ANSWERED.md` records it ACCEPTED with the standing "upgrade to latest first, vendor only if needed" process change, which the spec's Step 0 already encodes as binding. Landing-order caveat for the executor: the spec's stated predecessor **WP-64** (`[workspace.dependencies]`) has NOT landed - no such table exists in `rust/Cargo.toml` - so section 3a's "one root entry" instruction has nothing to hoist into yet. Its section 2 "Currency check - re-verify, may have moved" is self-flagging and remains the correct instruction; I did not re-check crates.io versions (read-only source inspection only).


### WP-67-sentry-feature-trim.md
- verdict: KEEP
- lines: 135
- tier: Tier 2 compact - same "No line numbers are cited on purpose ... STOP and report" banner as WP-66, sections 0-6 plus a riders table, call sites named by file+function rather than by line.
- risk-to-executor: low - measurement-first ("First implementation action is measurement, not editing"), and it explicitly distrusts its own finding's mechanism rather than asserting it.
- evidence: Nothing has landed. Verified live: all four declarations are still bare defaults - `bot/Cargo.toml` `sentry = "0.48"`, and `web/Cargo.toml`, `lib/cmd/Cargo.toml`, `lib/game_client/Cargo.toml` each `{ version = "0.48", optional = true }` - with no `default-features = false` anywhere, matching the spec's own "No crate sets `default-features = false` today". `rust/Cargo.lock` still contains `sentry-actix` and `ureq`, so the reconciliation question section 2 raises is still open and unanswered. D-18 is ANSWERED and not reversed: `decisions-ANSWERED.md` records it ACCEPTED with the standing constraint that no Sentry functionality may be lost, which the spec carries through into section 3a's justification for retaining `debug-images` and `release-health` and into the six-point end-to-end check in section 5. Same predecessor caveat as WP-66: **WP-64 has not landed**, so rider 5's "single hoisted `sentry` entry" has no `[workspace.dependencies]` table to live in yet.
# Classification batch 11


### WP-68-term-size-replacement.md
- verdict: KEEP
- lines: 117
- tier: Tier 1 pre-tiering by marker, but a survey-confirmed false positive - SUBSKILL banner on a 117-line single-task spec
- risk-to-executor: medium - it instructs "replace line 186" and "delete lines 31-34 exactly", and repl.rs has already drifted (see below); the quoted line contents make recovery easy, but a literal line-addressed executor would edit the wrong place.
- evidence: Nothing has landed - `git log --oneline --all --grep 'WP-68'` is empty and `work-packages.md` reads "### WP-68 term_size replacement - READY". All three targets are live and unmodified in substance: `rust/lib/cmd/Cargo.toml:16` still `term_size = "0.3.2"`, `rust/deny.toml:31-34` still carries the RUSTSEC-2020-0163 comment + ignore (line numbers still exact), and the sole call site `let (term_w, _) = term_size::dimensions().unwrap_or_default();` still exists. **Citation drift found:** that call site is now `repl.rs:219`, not `:186` as the spec states three times (WP-06's repl fixes, commit `a543120`, shifted it) - the Non-Goals line "do not touch anything in `repl.rs` beyond line 186" is likewise off by ~33 lines. No SUPERSEDED note anywhere; D-23/WP-69 depends on this landing first, so it is still live and load-bearing.


### WP-69-deny-toml-hardening.md
- verdict: KEEP
- lines: 151
- tier: Tier 2 compact - no SUBSKILL banner, and an explicit standing instruction "No line numbers are cited on purpose; the tree is under concurrent edit ... STOP and report rather than improvising". Zero absolute line-number citations in the file.
- risk-to-executor: low - short, gated, config-only, and it tells the executor to stop on any mismatch; the only hazard is landing 3b before the sibling dependency packages, which the spec itself forbids.
- evidence: Nothing has landed. Live `rust/deny.toml` still matches the spec's section 1 problem statement exactly: `[advisories].ignore` has 7 entries including the 4 stale ones (RUSTSEC-2024-0365, -2026-0136, -2026-0137, -2021-0153) with their diesel/legacy-`rust/api` and `encoding` comment blocks; `[bans] multiple-versions = "warn"`, `wildcards = "allow"`, empty `skip`/`skip-tree`; `[sources] unknown-registry`/`unknown-git` both `"warn"`. `git log -- rust/deny.toml` shows only two pre-review commits (`6c43ff3`, `cb74632`) - no WP-69 work. Its governing decisions are intact, not reversed: `decisions-ANSWERED.md` D-23 ACCEPTED (flip to deny only after WP-66/67/68, clear the 4 stale ignores now) and D-24 ACCEPTED (option A, record `combine` as a risk), which is exactly what the spec encodes. Confirmed load-bearing beyond its own package: section 3d IS the whole of WP-72 ("combine accepted-risk comment"), stated in the spec's own header banner and corroborated by `planning/README.md` ("WP-72 has no file of its own by design: it is section 3d of specs/WP-69-deny-toml-hardening.md") - archiving WP-69 would silently delete WP-72 too.


### WP-70-serde-yaml-ng.md
- verdict: KEEP
- lines: 98
- tier: Tier 2 compact (FINDHDR + the no-line-numbers blockquote variant; zero `file.rs:NNN` citations)
- risk-to-executor: low - no line numbers, three named edits, explicit STOP-and-report instruction if a third `serde_yaml` site or any deserialise call turns up.
- evidence: Not landed and not reversed. `git log --grep 'WP-70'` is empty; `work-packages.md` reads "### WP-70 serde_yaml migration - READY (D-21 answered 2026-07-26: option A, `serde_yaml_ng`)", and `decisions-ANSWERED.md` D-21 confirms the same option A with the same reasoning the spec encodes (JSON rejected because it would change a file format ops depend on). Both consumers are live and still on the old crate: `rust/bot/Cargo.toml:29` and `rust/lib/game_client/Cargo.toml:15` each declare `serde_yaml = "0.9"`, and `bot/src/prompt.rs` still calls `serde_yaml::to_string` with the native-tags comment the spec says to preserve. Its only external dependency is a landing-order preference on WP-64 (workspace hoist), which the spec handles explicitly with a fallback.


### WP-71-warp-to-axum.md
- verdict: KEEP
- lines: 152
- tier: Tier 2 compact (FINDHDR + the no-line-numbers blockquote variant; zero `file.rs:NNN` citations)
- risk-to-executor: low - symbol-addressed throughout, and its hard gate is self-checking ("Re-read and confirm ... If WP-06 has been reverted or the file differs, STOP and report").
- evidence: Not landed, and its stated preconditions verify true against live source. `git log --grep 'WP-71'` is empty; `work-packages.md` reads "### WP-71 warp -> axum consolidation - READY (D-22 answered 2026-07-26: port now)", matching `decisions-ANSWERED.md` D-22 verbatim ("port warp -> axum now, in the same window as WP-06's http.rs fixes"). `rust/lib/cmd/Cargo.toml` still has `warp = { version = "0.4.3", features = ["server"], optional = true }` and `http-server = ["warp", "tokio", "sentry"]`. Section 3's "WP-06 Task 1 has ALREADY LANDED" claim checks out: live `lib/cmd/src/http.rs` has `const MAX_CONTENT_LENGTH: u64 = 16 * 1024 * 1024`, a private `fn route<G: ...>`, `content_length_limit(MAX_CONTENT_LENGTH)`, no `impl Reject`, and exactly the three named tests. No decision reversal touches it - D-20 (generic `brdgme_game_bin`, WP-73) is intact and `WP-73-game-binary-consolidation.md` itself says "if both are pending land WP-71 first".

### WP-73-game-binary-consolidation.md
- verdict: KEEP
- lines: 253
- tier: Tier 2 compact - explicit self-marker "Over the ~120-line Tier 2 cap (Lead-accepted)", no line-number citations by design ("no line numbers are cited on purpose"), one approximate-and-marked hint for GAME_PORTING.md.
- risk-to-executor: low - no exhaustive line citations, symbol-based, and it states its own verification greps.
- evidence: Nothing has landed. `rust/lib/` contains cmd, color, cost, game, game_client, markup, rand_bot - there is no `game_bin` crate. `ls rust/game/*/src/bin/*_repl.rs` returns 27 files and the bin total is still 108, exactly the spec's "before" state. The only commit mentioning WP-73 is 43bcf72, the planning-docs commit that created the spec. The D-41 fuzz reversal by D-43 is already folded into the spec text (3d keeps `_fuzz`, deletes `_repl` only), so it is not written against a reversed decision.


### WP-81-stats-deletions.md
- verdict: KEEP
- lines: 122
- tier: Tier 3 package written as a compact spec (README's Tier 3 roster: "Plus WP-17, WP-81, WP-83"); carries FINDHDR + the no-line-numbers blockquote and zero `file.rs:NNN` citations
- risk-to-executor: low - every deletion is named by function, the collateral removals carry an explicit "Confirm the second lookup is present before removing the first; if it is not, STOP", and §5 gives grep-based over-deletion guards in both directions.
- evidence: Not landed. The one `git log --all --grep 'WP-81'` hit is `43bcf72 docs(review): planning session 3` - a planning-doc commit, not an implementation. All three deletion targets are live: `rust/game/acquire-1/src/stats.rs` still exists, and `pub investments: usize` plus `self.stats[player].expeditions += 1;` are present in **both** `lost-cities-1/src/lib.rs` (:44, :376) and `lost-cities-2/src/lib.rs` (:51, :383). `work-packages.md:1236` reads "### WP-81 dead per-game stats machinery removal - READY (D-40 answered 2026-07-26: option B)", matching `decisions-ANSWERED.md` D-40 verbatim including the clean-slate rationale the spec's own "This is NOT a design statement about stats" paragraph reproduces; D-40 is not among the five changed rulings. This spec is the *beneficiary* of the corpus's only relevant supersession rather than a victim of it - `work-packages.md:322` records "**`c F11` is SUPERSEDED by WP-81** ... **WP-81 deletes that file entirely** ... Land WP-81 first and DROP Task 5" from WP-19, which §4's landing-order collision note states from the other side.
# Classification batch 4


### WP-82-db-module-split.md
- verdict: KEEP
- lines: 290
- tier: Tier 2 compact - explicit "no line numbers are cited on purpose" banner and symbol-table structure; over the ~120-line cap only because the module table enumerates 13 modules of symbols. No line citations at all.
- risk-to-executor: low - zero line numbers, entirely symbol-driven, and it tells the executor to re-verify the symbol inventory in `raw/db-split-inventory.md` before starting.
- evidence: Nothing has landed. `rust/web/src/db.rs` still exists as a single 293KB file and `rust/web/src/db/` does not exist, which is exactly the spec's "before" state; no commit anywhere mentions WP-82. The supersession direction is the right way round: this spec supersedes the DEFERRED WP-78 entry (README.md's STATUS banner records "WP-78 is SUPERSEDED by WP-82" and counts WP-78 as the single SUPERSEDED package), so WP-82 is the live one. Its stated predecessor WP-41 has already landed, and README.md calls WP-82 the hard predecessor that "lands first" for the whole remaining web cluster.


### WP-83-parity-fixes-released.md
- verdict: KEEP
- lines: 155
- tier: Tier 2 compact - self-marked "Slightly over the ~120-line Tier 2 cap (Lead-accepted)", symbol-based, no line numbers.
- risk-to-executor: low - three self-contained surgical fixes, each with a live-code confirmation note and a test.
- evidence: None of the three fixes has landed. `grep phase_before rust/game/roll-through-the-ages-2/src/lib.rs` -> no hits (fix 1 absent). `rust/game/seven-wonders-1/src/lib.rs` still does `let assigned_cities: Vec<City> = all_cities[..players].to_vec();` with no `by_board` grouping (fix 2 absent). `rust/game/red7-1/src/card.rs` still declares `pub fn leader(palettes: &[Vec<Card>]) -> (usize, Vec<Card>)`, the single-argument signature the spec replaces with pairs (fix 3 absent). The only WP-83 commit is 43bcf72, the planning-docs commit. D-35's release of `a F1`/`b F7`/`e F30` is affirmed by planning/README.md's STATUS banner, so the enabling decision was not reversed.


### WP-84-sse-migration.md
- verdict: KEEP
- lines: 408
- tier: Tier 2 compact - carries the standard Tier 2 banner ("no line numbers are cited on purpose") and a self-aware length justification ("the Tier 2 cap is ~120 lines. This spec is ~300 because it is a transport migration ... Reviewed and accepted at this length by the Lead"). Zero line-number citations; every reference is by file plus symbol.
- risk-to-executor: medium - long and dense, but the density is verified fact with explicit UNKNOWN markers (Cloudflare edge idle behaviour; hyper graceful-shutdown, where it forbids deletion without a proving test), so the failure mode is length, not wrong citations.
- evidence: Nothing has landed. `rust/web/src/events.rs` does not exist, `rust/web/src/websocket.rs` and `websocket_client.rs` are both still present, and `use_websocket` still appears in the client. The only WP-84 commit is 43bcf72, the planning-docs commit that created it. The claimed internal supersession is NOT this file's: `specs-LOG.md` line ~5870 reads "§3a pre-upgrade auth SUPERSEDED (do not build, replaced by WP-84 §3c)" inside the Worker-3 entry describing edits to `specs/WP-42-websocket-auth-and-filtering.md` - it is WP-42's §3a that is superseded BY WP-84, which makes WP-84 the superseding spec, not the superseded one. WP-84 §3c states the same thing from the other side ("WP-42 §3a's pre-upgrade dance is deleted, not ported"). The spec also already absorbs the D-48 two-stream resolution and deletes the single-stream fallback, so it is written on the post-pivot decisions, not reversed ones.


## ARCHIVE re-verification against clean committed master (2026-07-27)

### Why

The 13 ARCHIVE verdicts above were all reached on the ground that the spec's work had
**already fully landed** - but they were reached while the worktree still held uncommitted
remediation work. "Already landed" could therefore have been reading the executor's own
dirty tree rather than history. `rust/` is now clean and fully committed on `master`
(`git status --porcelain rust/` returns nothing), so every ARCHIVE verdict was
re-confirmed from scratch before anything is moved.

### Method

Two verification Workers split the 13. For each spec: the spec's **own Task roster** was
enumerated from the spec file, then **every task was checked individually against live
committed source** - not against this document's own evidence summary, which was treated
as a claim to be tested rather than a source. Each cited commit was confirmed present on
`master` via `git log`. Evidence was recorded as `file:line` plus symbol.

### Result

**All 13 CONFIRMED-LANDED. 0 NOT-LANDED.**

| spec | verdict | landing commit(s) | tasks confirmed |
|---|---|---|---:|
| WP-01-char-byte-panic-elimination.md | CONFIRMED-LANDED | `9abe8b4` | 7/7 |
| WP-03-lib-game-parser-mechanical.md | CONFIRMED-LANDED | `c39786f` | 8/8 |
| WP-06-lib-cmd-tools-http.md | CONFIRMED-LANDED | `a543120` | 5/5 |
| WP-13-starship-catan-fixes.md | CONFIRMED-LANDED | `4e0abe6` | 9/9 |
| WP-14-alhambra-core-fixes.md | CONFIRMED-LANDED | `c52f1a5` | 10/10 |
| WP-15-seven-wonders-mechanical.md | CONFIRMED-LANDED | `52680e5` | 9/9 |
| WP-21-cathedral-sushizock-fixes.md | CONFIRMED-LANDED | `f547238` | 10/10 |
| WP-25-modern-art-liveness.md | CONFIRMED-LANDED | `6c0c19c`, `e560a75`, `b0babb8`, `af2c014`, `7821938` | 5/5 |
| WP-36-crypto-deploy-hardening.md | CONFIRMED-LANDED | `13a1e69` | 6/6 (T6 = cargo gate) |
| WP-37-admin-pass.md | CONFIRMED-LANDED | `b49df61` | 13/13 |
| WP-39-bot-consumer-supervision.md | CONFIRMED-LANDED | `347970a` | 8/8 (T8 = cargo gate) |
| WP-41-db-quality-pass.md | CONFIRMED-LANDED | `baa5fc6` | 11/11 |
| WP-44-proposals-integrity-email-token-leak.md | CONFIRMED-LANDED | `f4e7640` | 10/10 |

### Per-spec evidence

#### WP-01-char-byte-panic-elimination.md - 7/7, `9abe8b4`
- T1 `Space::parse`: `rust/lib/game/src/command/parser/mod.rs:450` `input.len() - input.trim_start().len()` (byte-based, boundary-safe); test `space_parser_handles_multibyte_whitespace` at `:1707`.
- T2/T3 `Token::parse` / `Enum::parse` + `shared_prefix`: `shared_prefix` at `parser/mod.rs:603` returns an `(input_bytes, value_bytes)` pair built from `chars()`; `full = v_matching == v_str.len()` at `:652`. Tests `token_parser_handles_multibyte_input` (`:1739`), `enum_parser_handles_multibyte_values` (`:1786`), `enum_parser_multibyte_player_name` (`:1820`), `exact_enum_matches_multibyte_values` (`:1837`).
- T4 `Int::parse`: `parser/mod.rs:126-141` uses `char_indices().take_while(..).last().map(|(i,c)| i + c.len_utf8())`; test `int_parser_stops_cleanly_at_multibyte_chars` (`:1763`).
- T5 markup `slice()`: `rust/lib/markup/src/transform.rs:275-282` `text.chars().skip(start).take(end.saturating_sub(start))` with the byte-indexing comment; test `slice_multibyte_works` (`:602`).
- T6 red7 `CardParser`: `rust/game/red7-1/src/command.rs:23` `let chars: Vec<char> = input.chars().collect();`; test `card_parser_handles_multibyte_input` (`:117`).
- T7 convention: `docs/CODING.md:723-744` "Non-ASCII input coverage for string slicing" section exists.

#### WP-03-lib-game-parser-mechanical.md - 8/8, `c39786f`
- T1 typed `Many` max-at-top-of-loop: `rust/lib/game/src/command/parser/mod.rs:357-362` `if let Some(max) = self.max && parsed.len() >= max { break 'outer; }` with the `lg F8` comment; no min-bypassing early return remains.
- T2 zero-progress guards in all three loops: typed `Many` at `parser/mod.rs:384-389`, `CommandSpec::Many` parse at `:992-996`, suggest-side loop at `rust/lib/game/src/command/suggest.rs:163-169`. Struct doc comment at `parser/mod.rs:290-297` states the invariant.
- T3 order-independent `Enum` ranking: `parser/mod.rs:660-676` `match (matching.cmp(&match_len), full.cmp(&full_match))` replacing-on-strictly-better, plus `searched: HashSet<String>` dedupe at `:639`.
- T4/T6 suggest: `suggest.rs:145-149` returns `vec![]` past a bounded `Many` max (`lg F9` / `c F31`); `seen: HashSet<String>` at `:50` and `if token.is_empty()` at `:28`.
- T5 `Int` suggestion overflow: `suggest.rs:106-112` `max.map(|m| m.min(capped))`; test `int_near_i32_max_does_not_overflow` (`:540`).
- T7 doc: `rust/lib/game/src/command/doc.rs:53` and `:203` carry the `lg F11` "number N or lower" alignment.
- T8 `combine` dep: absent from `rust/lib/game/Cargo.toml` (grep returns nothing).

#### WP-06-lib-cmd-tools-http.md - 5/5, `a543120` (14 files, +475/-114)
- T1: `rust/lib/cmd/src/http.rs:15` `const MAX_CONTENT_LENGTH: u64 = 16 * 1024 * 1024;`, `:21` `warp::body::content_length_limit(MAX_CONTENT_LENGTH)`, `:38` `unwrap_or_else(|e| Response::SystemError {..})`; SystemError test at `:96`, oversize-body test at `:127`.
- T2: `rust/lib/cmd/src/requester/gamer.rs:64-66` `pub fn renders<..>(game: &G) -> Result<(PubRender, Vec<PlayerRender>), GameResponseError>`; commit also touches `cli.rs` (+35) and `requester/error.rs`.
- T3: `rust/lib/cmd/src/repl.rs:267` `Ok(0) | Err(_) => None` (EOF), `:131` `vec![Node::text("No undos available")]`.
- T4: `rust/lib/cmd/src/bot_cli.rs` now contains only `pub struct Request` (-32 lines); no `cli` fn, no `Response`. `rust/lib/rand_bot/src/main.rs` and `lib.rs` touched for the F44/F45 nits.
- T5: `rust/lib/cmd/src/requester/local.rs:42` `RequestError::ChildExit { .. }` plus test `failing_child_reports_exit_status_not_json_error` (`:56`) asserting `status.code() == Some(3)`; `serde(default)` absent from `rust/lib/cmd/src/api.rs`.

#### WP-13-starship-catan-fixes.md - 9/9, `4e0abe6`
- T1: test `cannon_surcharge_keys_off_cannons_not_boosters` at `rust/game/starship-catan-1/src/lib.rs:2522`.
- T2: `lib.rs:1270-1272` `can_lose_module` = `self.current_player == player && self.losing_module` (no `||`).
- T3: `rust/game/starship-catan-1/src/command.rs:63` `const MAX_TRADE_AMOUNT: i32 = 99;` used via `Int::bounded(1, MAX_TRADE_AMOUNT)` at `:128` and `:143`.
- T4/T8: tests `trade_and_build_buy_requires_astro` (`lib.rs:2619`), `trade_and_build_buy_allows_exact_astro` (`:2637`), `last_sectors_capped_on_flight_end` (`:2709`).
- T5: `render.rs:108` `fn render(pub_state, player: Option<usize>, peeking: Option<&[SectorCard]>)`, peek rows gated at `:183-192`; `PlayerRender.peeking` at `:55`.
- T6: `render.rs:122-124` "Current turn:" row. T7: `lib.rs:911` `format!("you can only {} with this trade card", direction.string())`.
- T9: `start_card` absent from the whole crate (grep empty); `flight_actions` invariant comment at `lib.rs:498-500` citing a F20.

#### WP-14-alhambra-core-fixes.md - 10/10, `c52f1a5`
- T1: `rust/game/alhambra-1/src/lib.rs:564-585` `take()` clones the market and `remove(pos)`s each request before committing; test `take_cannot_mint_duplicate_cards` (`:1620`).
- T2: `lib.rs:101` helper doc "Raw index of the `n`th non-Empty tile, matching the 1-based numbering"; regression test comment at `:1670` citing b F17.
- T3: `rust/game/alhambra-1/src/card.rs:488-530` `grid_longest_ext_wall` sorts entries by `(v.x, v.y)` and scans `for rot_num in 0..3i32` skipping Empty candidates; test comment at `lib.rs:1394` cites b F18.
- T6: `card.rs:458-461` "Inclusive on both axes for symmetry" with `for x in min.x..=max.x`.
- T8: single `pub fn grid_tile_counts` at `card.rs:334`, called from `render.rs:230` and `card.rs:618`. T9: `render.rs:152` clamp comment "columns past 'z' are unaddressable". T10: `card.rs:1` imports `{HashMap, HashSet, VecDeque}`; flood walks at `:389` and `:430` use `VecDeque::from` + `HashSet`.
- T4/T5/T7: `lib.rs` test block grew +358 lines in the commit; only 2 `expect(` remain in `lib.rs`; remaining `{:?}` occurrences are test assertion messages only (`lib.rs:1579`, `:1600`, `:1614`).

#### WP-15-seven-wonders-mechanical.md - 9/9, `52680e5`
- T9: `rust/game/seven-wonders-1/src/scoring.rs` (`science_vp`, `score_science`, `player_vp`, `mimic_guild_vp`) and `src/trade.rs` (`can_afford_cost`, `resolve_deal`, `pay_cost`, `player_goods_options`, `trade_cost_per_good`) both exist.
- T1: `scoring.rs:78` `CardEffect::DrawDiscard { vp: stage_vp } => vp += stage_vp`; test comment `lib.rs:1088` cites b F1.
- T2: test `auto_discarded_last_card_pays_no_coins` at `lib.rs:1109` asserting 3 starting + 3 chosen-discard + 0 auto coins.
- T3: `fn prune_resolvers` at `lib.rs:701`, called at `:258` and `:742`.
- T4: `deal_coins: Option<HashMap<i32,i32>>` field at `lib.rs:44` with legacy-`deal` fallback comment; `resolve_deal(player, cost, deal, deal_coins)` calls at `:331` and `:358`.
- T5: test `military_log_uses_player_node` at `lib.rs:1475` asserting `{{player 0}} defeated {{player 1}} ...`. T6: `start_hand` absent from the crate. T7: `player_state` uses `.get(player).cloned().unwrap_or_default()` (`lib.rs:794`) and test `command_parser(99).is_none()` at `:1495`. T8: 37 `#[test]`s in `lib.rs`.

#### WP-21-cathedral-sushizock-fixes.md - 10/10, `f547238`
- T1/T4: `Box::leak`, `LocChoice` and `parse_loc` all absent from `rust/game/cathedral-2/src/` (grep empty).
- T2: `cathedral-2/src/lib.rs:93-101` `tile_at` returns `empty_tile()` when `!loc.valid()`; test `tile_at_returns_empty_for_off_board_locations` (`:1348`) citing c F26.
- T3: `cathedral-2/src/piece.rs:110` `pub fn pieces(player: i32) -> Option<Vec<Piece>>`. T5: capture-walk comment at `cathedral-2/src/lib.rs:324` citing c F23. T6: `rand` absent from `cathedral-2/Cargo.toml`.
- T7: `rust/game/sushizock-2/src/lib.rs:497-498` "Accepts exactly 1..=len (c F29)"; tests at `:1766`, `:1797` (the duplicated `steal_red` branch) and `:1825`.
- T8: `log_game_end` defined at `sushizock-2/src/lib.rs:380` and returned from the roll path at `:374`. T9: `roll_dice` at `:150-159` uses `DIE_FACES[rng.random_range(..)]` with the c F32 comment, no `.unwrap()`. T10: shared private `fn take` (`:406`) and `fn steal` (`:454`) with `take_blue`/`take_red`/`steal_blue`/`steal_red` delegating; `take_worst` at `:551`.

#### WP-25-modern-art-liveness.md - 5/5
- T1: `fn advance_past_empty_hands` at `rust/game/modern-art-2/src/lib.rs:457`, called at both boundaries (`:371` settle path, `:453`).
- T2: `self.state = State::PlayCard;` at `lib.rs:309`, immediately after the `currently_auctioning = vec![]` clear at the top of `end_round`; test `game_end_via_fifth_card_leaves_no_stale_auction` at `:1259`.
- T3: `rust/game/modern-art-2/src/render.rs:63` guard chain includes `&& bid > 0`; test `no_current_bid_line_before_any_bid` at `lib.rs:1281`.
- T4: `rust/game/modern-art-2/RULES.md:63` reads "Open, Fixed Price, Sealed, or Once Around"; `:76` reads "turn then passes to the player on the auctioneer's left".
- T5: `is_some_and(|h| !h.is_empty())` at `lib.rs:259`, `is_none_or(|&b| b > 0)` at `lib.rs:151`, and `use std::default::Default` returns zero hits in `lib.rs`.

#### WP-36-crypto-deploy-hardening.md - 6/6 (T6 = cargo gate), `13a1e69`
- T1: `fn secure_cookie(env_value: Option<&str>)` at `rust/web/src/auth/session.rs:33` plus all three named tests at `:111`, `:116`, `:121`.
- T2: `k8s/dev/web-patch.yaml` exists with `SECURE_COOKIE: "false"` and is wired via `k8s/dev/kustomization.yaml:11`; `Tiltfile:131` `serve_cmd` sets `SECURE_COOKIE=false`; `rust/web/.env.template:27` same.
- T3: `rustls::crypto::aws_lc_rs::default_provider()` at `rust/web/src/main.rs:18`.
- T4: `rust/web/src/crypto.rs` - `pub fn default_key() -> Zeroizing<[u8; 32]>` (`:43`), `load_key() -> Result<Zeroizing<[u8; 32]>, _>` (`:54`), `bytes.zeroize()` at `:61` and `:66`.
- T5: `CancellationToken`/`TaskTracker` fields at `rust/web/src/websocket.rs:34-35`, `pub fn begin_shutdown` at `:92`, called from `rust/web/src/main.rs:126`.

#### WP-37-admin-pass.md - 13/13, `b49df61` (all in `rust/web/src/admin.rs`, 3235 lines)
- T1/T2: `pub const ADMIN_REQUIRED` `:38`, `async fn require_admin` `:46`, error-variant match at `:1204-1209`.
- T3: `type BotProviderTestRow` local alias at `:871`, used at `:878`.
- T4/T5: `fn mask_api_key` `:416`; `"(undecryptable)"` per-row degrade at `:452` with `#[sqlx::test] test_admin_list_providers_degrades_one_undecryptable_row` `:2793`.
- T6/T7: `BOT_DISPLAY_ORDER_LOCK` `:176` (bound `:264`, `:357`), `FROM unnest($1::uuid[]) WITH ORDINALITY` `:366`, `rows_affected()` guards `:327`, `:378`, `:696`.
- T8/T9/T10: `pub enum ApiKeyUpdate` `:84`; `fn require_text` `:183`, `fn validate_temperature` `:197`; `SELECT model FROM bot_providers` `:801`.
- T11/T12/T13: `MAX_TEST_BODY_BYTES` `:720`, `async fn read_capped_body` `:740`, `fn allowlisted_headers` `:770`; `grep -c "value().get().unwrap()"` = 0; both `test_action`s now carry the id in the value (`Action::new(|(id, model)...` `:1606`, destructured `Some((provider_id, result))` `:1653`; `:1960`/`:2006` for bots) - no completion Effect.

#### WP-39-bot-consumer-supervision.md - 8/8 (T8 = cargo gate), `347970a`
- T1: `pub async fn supervise_consumer` at `rust/web/src/nats.rs:253` with `nats_consumer_restarts_total` counter `:284`; called twice from `rust/web/src/main.rs:71` and `:84`.
- T2: `pub const MAX_DELIVERIES_ADVISORY_SUBJECT` `rust/web/src/nats.rs:181`, subscribed `:210`, `bot_stream_max_deliveries_total` `:220`.
- T3: `pub fn stream_config_drift` `:53` and `pub fn consumer_config_drift` `:74`, called at startup `:126`/`:163`; ack_wait invariant documented `:136-144`; unit tests `:320`, `:348`.
- T4: conflict re-publish filter `.filter(|t| t.position == event.player_position)` at `rust/web/src/game/mod.rs:395`.
- T5: zero `unreachable!` in `rust/bot/src/main.rs`; replacement error "Bot turn gave up after {} attempts..." at `:457`.
- T6: `use tokio::sync::Semaphore` `:31`, `DEFAULT_MAX_CONCURRENT_TURNS`/`MAX_CONCURRENT_TURNS` env read `:844-850`, `Semaphore::new` `:850`, SIGTERM+ctrl_c shutdown signal `:720-732`.
- T7: `async fn healthz` `:696` carries the declined-DB-check rationale doc comment `:690-695` naming "review bo F8".

#### WP-41-db-quality-pass.md - 11/11, `baa5fc6` (all in `rust/web/src/db.rs`, 8149 lines)
- T1: `//!` header documents the `update_updated_at` trigger convention (`:5`) and `# Module map` (`:27`). Only 2 production `updated_at = NOW()` writes remain, both on trigger-less `game_proposals` in `delete_game` (`:1582`, `:1588`) behind the explanatory NOTE at `:1578` - exactly the must-exclude set. Remaining hits are test backdating helpers (`:4155`, `:4160`, `:4400`). Trigger test `update_updated_at_trigger_maintains_games_and_game_players` `:7125`.
- T2/T3: `pub async fn is_user_admin(...) -> Result<bool>` on the file-wide alias `:639` with test `is_user_admin_true_false_and_unknown_user` `:7194`; sticky finish `is_finished = ($2 OR is_finished)` `:1991`.
- T4/T5: `// is_turn_at is LAST TURN ACTIVITY` comment `:2021`; F49 `for (pos, pref) in &rem_prefs` `:1062` with rationale `:1061`; F50 comment `:1898`; F43 sentinel doc `:127` + assertion `:4852`; F46 race note `:934`.
- T6/T7: `make_interval(secs => $1::double precision)` `:3303` with rationale `:3297`; `send_friend_request` `:2175` has the `source == target` no-op (ws F48) and `pg_advisory_xact_lock(hashtext(LEAST..), hashtext(GREATEST..))` `:2187`.
- T8/T9: inlined predicate in `friend_recent_visible_game` `:2646` using `gp2`/`u` aliases `:2659-2661`, cross-ref comments `:2541`/`:2639`, drift-guard test `friend_recent_visible_game_matches_is_game_visible_to_user` `:7286`; `insert_game_logs_tx` doc "Deliberately row-at-a-time" `:1311`.
- T10/T11: `is_game_visible_to_user_friends_tier_requires_every_friends_player` `:4011`, `count_rows` note `:6769`. 113 `#[sqlx::test]`s in the file; all 27 previously-untested public fns from the T11 roster were checked individually (`count_incoming_friend_requests`, `find_active_turn_games`, `find_enabled_bots`, `find_game_version_render_meta`, `find_game_version_rules`, `find_latest_non_deprecated_game_version`, `find_open_restart_proposal_tx`, `find_user_id_by_name`, `generate_unique_username`, `get_pending_request_source`, `get_user_by_email`, `get_user_pref_colors`, `has_block_conn`, `insert_game_logs_tx`, `mark_game_read`, `replacement_bot_available`, `set_user_name`, `set_user_pref_colors`, `should_hide_add_friend`, ...) and now have >=2 references inside the test region.
- F42 module split was fenced out of this spec by its own disposition table and is owned by WP-82 - not an outstanding task here.

#### WP-44-proposals-integrity-email-token-leak.md - 10/10, `f4e7640` (all in `rust/web/src/proposals.rs`)
- T1: `pub struct ProposalPlayerView` `:63-72` has NO `email_token` field; `find_proposal_roster`'s SELECT `:504` selects only `pp.id, pp."position", pp.user_id, pp.bot_name, pp.bot_difficulty, pp.response, ...` - no token. Test `roster_view_never_exposes_email_token` `:2314`.
- T2/T3: owner-decline guard "The owner can't respond to their own proposal. Cancel the invite instead." `:1010`; `fn transfer_target_error` `:1026` called at `:1649`, unit-asserted `:2400-2403`.
- T4: `cancel_proposal` `:1501` now does `begin` -> `lock_proposal_for_update` -> owner/open checks -> `find_proposal_players_tx` inside the tx; no pre-transaction fetch or duplicated authz. `fn accepted_invitee_ids` `:1039`, test `:2407`.
- T5/T8: `RespondOutcome` returns zero hits across `rust/web/src`; only `count_pending_human_invitees_tx` `:846` survives (pool variant deleted; sole caller `email/inbound.rs:735`).
- T6/T7: typed interval binds `($1 * interval '1 second')` at `:698`, `:728`, `:792` - no `|| ' seconds'` text concat remains; single-statement roster reset via `replace(gen_random_uuid()::text, '-', '')` `:658`.
- T9/T10: `.ok_or_else(|| ServerFnError::new("Game type not found"))?` at `:1270` (respond) and `:1346` (start), matching `create_proposal` `:1083`; neutral labels `resolve invite email: {lookup,gen username,insert user,insert email}` `:869`/`:877`/`:884`/`:892`; `tracing::instrument` added at `:1500` (`cancel_proposal`), `:1551` (`remove_proposal_slot`), `:1722` (`get_pending_invites`).

### Caveats for future verifiers

- **WP-44 - `email_token` still greps positive in `proposals.rs`.** It appears at `:52` and `:491`, but only on the **server-side** `ProposalPlayer` model and that model's own SELECT, neither of which crosses the wire. The wire type `ProposalPlayerView` (`:63-72`) and `find_proposal_roster`'s SELECT (`:504`) are clean. A future verifier grepping only for the bare string could misread this as an unfixed leak.
- **WP-41 - 2 production `updated_at = NOW()` writes survive** at `db.rs:1582` and `:1588`. These are the trigger-less `game_proposals` rows that the spec's Task 1 **explicitly designates as must-keep**, guarded by the NOTE at `:1578`. Task 1's sweep is complete, not partial.

### Correction to existing content in this document

The WP-03 evidence above (and in the per-file section) claims zero-progress guards "in all
three Many loops". That is correct **in effect**, but the three guards use three different
shapes, so a symbol-level grep for one form will not find the other two:
`let progressed = new_offset > offset;` (typed `Many`), `if step == 0 { break; }` at
`rust/lib/game/src/command/parser/mod.rs:992`, and
`if d_out.remaining.len() == rem.len()` at `rust/lib/game/src/command/suggest.rs:163`.

### Conclusion

Because all 13 re-verified as CONFIRMED-LANDED, **all 13 are cleared for archiving.** The
`Counts: 47 KEEP, 13 ARCHIVE, 0 UNCERTAIN.` line at the top of this document stands
unchanged.
