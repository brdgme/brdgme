# T3-B8: workspace hygiene + red7-1 RULES.md docs

- **Batch**: T3-B8 = WP-65 (workspace hygiene, 9 findings) + WP-74 and WP-75
  (red7-1 `RULES.md` documentation, **zero finding ids each**)
- **Scope**: the workspace root manifest, `rust/web`, `rust/lib/cmd`,
  `rust/lib/color`, `rust/game/{lost-cities-2,acquire-1,lords-of-vegas-1}`
  crate-root cruft, the workspace-wide test-module naming convention, the CI
  config (`.github/workflows/ci.yml`), and `rust/game/red7-1/RULES.md`
- **Sources**: `findings/dependencies.md` (`dp Fnn`) - **no verification file
  exists**, the raw file is authoritative; `findings/games-batch-e.md`
  (`e Fnn`) - **superseded by `findings/verification/games-batch-e.md`**, whose
  corrected details are honoured below (`e F9` and `e F28` are both ADJUSTED,
  severity nit stands in each case)
- **Numbering**: neither findings file carries inline ids. `Fnn` = the nth
  `###` heading in that file. `dependencies.md` claims 26 findings but has 27
  `###` headings; sequential numbering is nonetheless sound and was re-anchored
  here on `dp F6` (sqlx split), `dp F12` (sentry default features) and
  `dp F20` (num_cpus). Every `dp` id used below was checked against WP-65's
  declared paths and **all seven matched** - no mismatches to report.
- **WP-74 / WP-75 carry no finding ids by construction.** They were filed at
  spec time from `specs/WP-29-red7-cleanup.md` "Cross-package / newly
  discovered" items 1 and 2. Their `Finding` column reads `WP-74` / `WP-75`.
- **Rows**: 11 (5 minor / 6 nit). Minor: `dp F4`, `dp F9`, `dp F17`, `WP-74`,
  `WP-75`. Nit: `dp F5`, `dp F21`, `dp F22`, `dp F23`, `e F9`, `e F28`. This
  matches `tier2-tier3-plan.md` section 2.1's 5m/6n for T3-B8.
- **3 of the 11 rows are gated** (`dp F9`, `WP-74`, `WP-75`) - each appears
  once in its file section below and again in `## Decision-blocked rows` with
  the gate spelled out. `WP-75` additionally **escalates** (see `## Escalate`).
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong, including
  two delete ranges that would have destroyed live code.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by source file so one session sweeps a file at a time.

## WP-65 - root manifest (`rust/Cargo.toml`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F5` | `rust/Cargo.toml` `[workspace] members` array + the `[profile.android-dev]` / `[profile.server-dev]` tables | Sort `members` (or replace the per-game entries with a `game/*` glob) and delete the two empty `inherits = "dev"` profiles, which nothing in the repo's toml/json/nix references | n |

Sequencing: WP-64 (D-19) rewrites this same manifest. Land after it if it
lands, or expect a trivial rebase - the two edits do not overlap textually.

## WP-65 - web manifest (`rust/web/Cargo.toml`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F4` | `rust/web/Cargo.toml` `[profile.wasm-release]` table | Delete the whole block - cargo ignores non-root profiles and warns on every build; the root `rust/Cargo.toml` copy is the one that applies (leptos' `lib-profile-release = "wasm-release"` resolves against the root) | n |
| `dp F9` | `rust/web/Cargo.toml` `[dependencies]` keys `tower-http`, `gloo-net`, `gloo-timers` | **WP-64 recorded (2026-07-27):** tower-http resolved (0.6.11 only), gloo-timers resolved (0.4.0 only), gloo-net STILL DUPLICATED (0.6.0 via leptos_router/server_fn + 0.7.0 via web). Pin gloo-net back to 0.6 to dedupe, unless WASM bundle size loses to latest-first (escalate to Michael) | n |

## WP-65 - brdgme_cmd manifest + logger init (`rust/lib/cmd`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F17` | `rust/lib/cmd/Cargo.toml` the `env_logger` dependency key, plus its initialisation site in `rust/lib/cmd/src` | Make `env_logger` optional behind the `http-server` feature (or move logger init out of the library into the binaries), so a library crate stops installing a log implementation while the deployables run tracing | n |

Sequencing note: WP-64 (D-19) would restate this dependency key. Not
decision-blocked - the change here is feature-gating and init placement, not a
version choice - but land it after WP-64 if WP-64 lands, to avoid a manifest
conflict.

## WP-65 - lazy_static removal (`rust/lib/color`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F21` | `rust/lib/color/src/lib.rs` fn `Color::from_hex` (the `lazy_static!` `RE` block) + the `lazy_static` key in `rust/lib/color/Cargo.toml` | Replace the `lazy_static!` static with `static RE: LazyLock<Regex> = LazyLock::new(...)`, swap the `use` line to `std::sync::LazyLock`, and drop the `lazy_static` dependency | n |

**Do not touch `rust/game/lords-of-vegas-1` here** - `specs/WP-22-lords-of-vegas-fixes.md`
Task 5 owns that crate's `lazy_static` removal (see
`## Not in this checklist (owned elsewhere)`). Read-only spot-check on live
source: `lazy_static` is still declared in `rust/lib/color/Cargo.toml` **and**
`rust/game/lords-of-vegas-1/Cargo.toml`, i.e. WP-22 Task 5 had not landed at
the time of writing; if it still has not landed when this row is worked, leave
lords-of-vegas-1 alone rather than absorbing it.

## WP-65 - lockfile monitor item (no manifest edit)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F22` | `rust/Cargo.lock` the three `convert_case` entries (0.6.0 / 0.10.0 / 0.11.0) | No first-party change is possible: re-confirm all three copies are still purely transitive (config/leptos_config, derive_more-impl, leptos macros) and record it as a monitor-only duplicate that collapses when leptos and config converge | n |

## WP-65 - CI config (`.github/workflows/ci.yml`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `dp F23` | `.github/workflows/ci.yml` - a new job alongside the existing `cargo-deny` job | Add a scheduled dependency-currency check (`cargo outdated --workspace`, or a `schedule:`-triggered `cargo deny check advisories`) so version drift is caught mechanically instead of by review | n |

Read-only spot-check: a `cargo-deny` job already exists but is gated on
`needs.changes.outputs.rust == 'true'`, so it never runs on a quiet week - the
new job is the scheduled/currency half, not a duplicate of it.

## WP-65 - test-module naming convention (workspace-wide)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F9` | every crate's inline `#[cfg(test)] mod test` declaration (chiefly `rust/game/*/src/lib.rs`) | Adopt `mod tests` as the workspace convention and rename the remaining `mod test` modules - a pure identifier rename with no behaviour change | n |

Verification recast this from "love-letter-2 crate defect" to a workspace-wide
inconsistency: the game crates are majority `mod test`, only `lib/` is uniformly
`mod tests`. Two live-source facts, read-only: 21 `mod test` sites vs 78
`mod tests` sites across `rust/`. The target convention is `tests` (implied by
`specs/WP-29-red7-cleanup.md`, which keeps red7-1's module named `tests`);
**red7-1 needs no change.** Mechanically broad but one decision and one rename -
if the implementer prefers, split it into its own commit.

## WP-65 - stale crate-root template files (three game crates)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F28` | `rust/game/lost-cities-2/{build-release,.rls.toml,.gitignore}`, `rust/game/acquire-1/{build-release,.rls.toml,.gitignore}`, `rust/game/lords-of-vegas-1/{.rls.toml,.gitignore}` | Delete the obsolete `build-release` packaging scripts and the dead-RLS `.rls.toml` files, and trim the copied-template `.gitignore` entries (`lambda`, `.vscode`, `.idea`) | n |

Verification correction to honour: `.rls.toml` is **not** malformed - it is
exactly `build_lib = true` with no trailing newline; the reported
`build_lib = truetarget` was a `cat`-concatenation artifact with `.gitignore`.
Read-only spot-check: `lords-of-vegas-1` has `.rls.toml` but **no**
`build-release`, so that crate is a two-file delete, not three.

## WP-74 + WP-75 - red7-1 rules documentation (`rust/game/red7-1/RULES.md`)

Both rows are **hard-sequenced behind `specs/WP-29-red7-cleanup.md` Task 5 and
behind WP-30**, and WP-75 additionally behind WP-74. See
`## Decision-blocked rows`. Do not pick up WP-30's own scope, do not change
gameplay, and do not "correct" any other crate's `RULES.md`.

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `WP-74` | `rust/game/red7-1/RULES.md` `## Turn` section (behaviour source: `src/lib.rs` fn `start_turn`, which eliminates the current player "for not having any cards left", and fn `end_turn`, which can end the round off that elimination) | Doc-only: add a sentence stating that a player whose hand is empty at the start of their turn is eliminated, and that this can immediately end the round | n |
| `WP-75` | `rust/game/red7-1/RULES.md` - whole document, against `docs/authoring/RULES_AUTHORING.md` `## Required Sections` | **ESCALATED - not a one-line fix. See `## Escalate`.** Bringing the ~55-line file to authoring compliance is a whole-document rewrite that additionally needs a live render capture and a Strategy-Tips ruling | n |

## Decision-blocked rows

| Row | Gate | What clears it |
|---|---|---|
| `dp F9` (web tower-http / gloo-net / gloo-timers pins) | **CLEARED (2026-07-27)** - D-19 answered, WP-64 landed. `cargo tree -d` result: tower-http and gloo-timers no longer duplicate; gloo-net still 0.6.0 + 0.7.0. Residual action: pin gloo-net or escalate WASM-size-vs-latest to Michael. |
| `WP-74` (empty-hand elimination sentence) | **Sequencing gate**: must land after `specs/WP-29-red7-cleanup.md` Task 5 **and** after WP-30. WP-30 is **BLOCKED-ON-USER-RULES-REVIEW** (parked - clears only on the user's per-game rules sign-off, which is stronger than a decision block) and is additionally BLOCKED-ON-DECISION(D-29, D-40) | WP-29 Task 5 lands, the user signs off WP-30's rules review, and D-29/D-40 are answered. D-29's outcome (the zero-rule-fulfilling-player question) may change how elimination is described, which is exactly why this cannot go first. WP-29 Task 5 must **not** absorb this row - it is explicitly outside `e F32`'s scope. |
| `WP-75` (RULES_AUTHORING compliance) | Same sequencing gate as WP-74, **plus** it must land after WP-74 (it rewrites the whole document and would otherwise churn WP-74's diff). **Plus two non-decision blockers**: (a) the "Reading the Display" section needs a live render captured from a real game state - the extraction recipe is in `docs/authoring/RULES_AUTHORING.md` under the `### Reading the Display` heading and needs a database and a built binary, so it is **not writable from source alone; do not attempt the capture**; (b) an unresolved ruling, below | Everything WP-74 needs, plus WP-74 landed, plus a live render capture, plus the ruling below |
| **OPEN QUESTION for the user (WP-75)** | Do the shipped `rust/game/red7-1/BASIC_STRATEGY.md` and `ADVANCED_STRATEGY.md` - surfaced through `Gamer::basic_strategy` / `advanced_strategy` - satisfy `RULES_AUTHORING.md`'s mandatory "Strategy Tips" section, given that document says "Always include this section"? | A user/Lead ruling. **This question is not in `planning/decisions-needed.md` today** (its highest-numbered relevant entries are D-19, D-20 for this batch; no strategy-docs item exists) - it needs to be filed there or answered inline before a WP-75 spec can be written. |

## Not in this checklist (owned elsewhere)

- **`rust/game/lords-of-vegas-1`'s `lazy_static` -> `LazyLock` migration
  (`d F6`)** - owned by `specs/WP-22-lords-of-vegas-fixes.md` Task 5, which
  removes both the `tile.rs` `lazy_static!` `TILES` block and the `Cargo.toml`
  dependency. `dp F21` above therefore owns only the **remaining** site,
  `rust/lib/color`. WP-22's own spec anticipates this overlap and expects
  WP-65's sweep to find nothing left in that crate.
- **`dp F1`, `dp F2`, `dp F3`** (`[workspace.dependencies]`,
  `[workspace.package]`, `[workspace.lints]`) - WP-64, BLOCKED-ON-DECISION
  D-19. Not in this batch even though they touch the same manifests.
- **`dp F24`, `dp F25`** (deny.toml `multiple-versions = "warn"`; four stale
  advisory ignores) - WP-69 deny.toml hardening, BLOCKED-ON-DECISION D-23 and
  sequenced last among the dependency packages. `dp F23`'s CI row above adds a
  job and does **not** edit `deny.toml`.
- **`dp F13`** (term_size unmaintained) - owned by
  `specs/WP-68-term-size-replacement.md`.
- **`e F45` / `e F46` and the 108 boilerplate binaries** (`dp F26`) - WP-73,
  BLOCKED-ON-DECISION D-20. WP-65 does not touch `src/bin/`.
- **red7-1's `e F31`-`e F35` and the `RULES.md` turn/scoring text (`e F32`)** -
  owned by `specs/WP-29-red7-cleanup.md` Tasks 1-5. WP-74's sentence and
  WP-75's rewrite are explicitly outside Task 5's scope; do not fold them in
  unless the WP-29 implementer is landing last and chooses to (work-packages
  permits that only for WP-74).
- **red7-1 `e F30` / zero-rule-fulfilling-player behaviour** - WP-30, parked on
  the user's rules review. Never touch gameplay from this checklist.
- **`e F27`** (lost-cities-2 deployed blurb) - `specs/WP-28-lost-cities-shared-fixes.md`.
  WP-28 explicitly leaves `e F28`'s three crate-root files to this package.

## Escalate

- **`WP-75` - red7-1 `RULES.md` RULES_AUTHORING compliance. Not Tier 3.**
  Reasons, in order of severity:
  1. **Not a one-line fix.** Five required sections are missing outright
     (Cards / Components; Rounds / Game End; Winning; Reading the Display;
     Strategy Tips) and Scoring lacks the mandated worked example - that is a
     whole-document rewrite of a ~55-line file whose current headings are
     Overview, Setup, Commands, Turn, Rules (by colour), Scoring.
  2. **Not writable from source alone.** "Reading the Display", which
     RULES_AUTHORING calls critical for the bot, requires a render captured
     from a live game state (recipe under that document's
     `### Reading the Display` heading); it needs a database and a built
     binary. Do not attempt the capture from a read-only session.
  3. **Blocked on an unfiled ruling** - the BASIC/ADVANCED_STRATEGY question in
     the decision-blocked table above.
  4. **Blocked on a parked package** - WP-30's user rules review, plus WP-29
     Task 5 and WP-74.
  Recommended route: a Tier 2-style spec written *after* WP-30 clears, with the
  render capture as an explicit implementer step. The pointer row in the
  WP-74/WP-75 section above exists so this is not lost.
  `red7-1 is unlikely to be the only offender` - work-packages suggests
  auditing the other 26 game crates' `RULES.md` against the same checklist;
  that audit is out of scope here and unfiled.
- All nine WP-65 rows compress to one line. `e F9` is mechanically broad (21
  rename sites) but is a single decision plus a rename, so it stays Tier 3.
