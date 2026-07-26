# Tier 2 / Tier 3 execution plan - remaining brdgme Rust review work

Written 2026-07-25 by the SURVEY Lead. Read-only unit: derived entirely from
`REVIEW.md`, `findings/`, `findings/verification/`, `planning/*.md` and
`planning/specs/*.md`. No file under `rust/` was read, run or modified.

This file is the dispatch plan for everything that is NOT on the critical path.
The 10 criticals are specced and are being executed by a separate agent.

> **STATUS UPDATE 2026-07-26 - TIER 2 AND TIER 3 PLANNING ARE COMPLETE.**
> Every Tier 2 package in roster 1.1 now has a compact spec in
> `planning/specs/`, and every Tier 3 batch in roster 2.1 now has a checklist in
> `planning/checklists/`. The per-row `Covered by` columns added below name the
> file. **Do not re-run any T2-B* or T3-B* batch** - they are done. The
> remaining open work in this file is the decision-blocked rosters (1.2 and 2.2),
> which stay blocked. `planning/README.md` is the current entry point for the
> whole planning set.

---

## 0. Scope arithmetic and one plan decision

### 0.1 Specs that already exist (25, not 24)

`planning/specs/` holds WP-01, 03, 06, 07, 13, 14, 15, 19, 21, 22, 23, 25, 28,
29, 36, 37, 39, 40, 41, 44, 51, 54, 56, 59 **and WP-68** (term_size
replacement), plus `notes-conventions.md`. The Orchestrator brief omitted
WP-68. Treat all 25 as done.

### 0.2 The "~480 Tier 3 findings" figure is wrong - it is ~248

| Bucket | minor | nit |
|---|---|---|
| Whole review (corrected tally) | 257 | 225 |
| less: already inside the 25 finalized specs | -99 | -96 |
| less: inside the 6 BLOCKED-ON-USER-RULES-REVIEW packages | -30 | -11 |
| **Remaining, unspecced, unparked** | **128** | **118** |

### 0.3 PLAN DECISION - Tier 2 specs cover their whole package

The brief's split (Tier 2 = majors, Tier 3 = all minors + nits) would cut 20 of
the 21 Tier 2 packages in half and put two different sessions into the same
file. Every existing spec is whole-package, and `work-packages.md` sized every
package as one implementation session.

**Therefore:** a Tier 2 spec covers its package end to end - the majors in full
detail, the package's own minor/nit riders as a short checklist appendix inside
the same spec. Tier 3 covers only the packages that contain **zero** majors.

Resulting split: Tier 2 = 21 packages (13 dispatchable, 8 decision-blocked)
carrying 21 majors + 57 riders-minor + 38 riders-nit. Tier 3 = 23 packages
(16 dispatchable, 7 decision-blocked) carrying 73 minors + 80 nits.

If the user disagrees, the only change needed is to move each Tier 2 package's
rider appendix into its crate's Tier 3 checklist; the batching below is
unaffected.

### 0.4 Excluded by instruction

- **BLOCKED-ON-USER-RULES-REVIEW - not in this plan at all:** WP-11, WP-12,
  WP-16, WP-20, WP-26, WP-30. Game rules parity is parked pending the user's
  per-game sign-off. Do not schedule, do not spec, do not "correct" a
  `RULES.md` under them.
- **Splittable exceptions worth surfacing to the user (listed only, NOT
  scheduled):** WP-20's D-40 acquire-stats item (c F12, write-only
  `to_brdgme_stats`) and WP-30's lost-cities D-40 items (e F39, e F40) are
  **stats** questions, not rules questions. Either could be carved into its own
  package and moved before the rules review. Flagged, not done.
- **Still-unanswered decisions blocking otherwise-ready work:** **D-11** (blocks
  WP-46) and **D-30** (blocks WP-11, itself parked anyway).

---

## 1. Tier 2 roster

Criteria: >= 1 MAJOR finding, no finalized spec, not
BLOCKED-ON-USER-RULES-REVIEW. Major IDs are **post-verification** (units 1-9
verified per finding; units 10-13 lead-verified).

### 1.1 Dispatchable now (13) - **ALL SPECCED, ROSTER COMPLETE 2026-07-26**

Every row below has a finalized compact spec. Nothing here is dispatchable as
planning work any more; the specs are implementation inputs.

| WP | Title | Crate(s) | Major finding IDs | Decision status | Size | Covered by |
|---|---|---|---|---|---|---|
| WP-02 | markup robustness and dedup | `lib/markup` (+`lib/color`) | ls F2, ls F3 | READY (D-37 = option A) | M | `specs/WP-02-markup-robustness-dedup.md` |
| WP-08 | finish/placings epilogue dedup sweep | ~11 game crates + maybe `lib/game` | e F1 | READY | M | `specs/WP-08-finish-placings-epilogue-dedup.md` |
| WP-09 | deserialized-state trust hardening | `lib/cmd/requester/gamer.rs` + ~17 game crates | e F18, e F36 | READY (D-36 = option A) | **L** | **SPLIT as predicted**: `specs/WP-09a-deserialized-state-boundary.md` + `specs/WP-09b-game-crate-state-trust-sweep.md`. Land 09a first |
| WP-10 | pub_state hidden-info redaction | zombie-dice-2, for-sale-2 (+starship-catan-1) | f F1, f F13 | READY (D-33 = option A) | S | `specs/WP-10-pub-state-hidden-info-redaction.md` |
| WP-34 | auth races and session mechanical | `web/src/auth/{server,session}.rs` | ws F1 (ADJUSTED critical->major) | READY, never blocked | M | `specs/WP-34-auth-races-session-mechanical.md` |
| WP-35 | auth edge semantics and fail-open | `web/src/auth/server.rs`, `crypto.rs`, `main.rs`, `db.rs` | ws F2, ws F16 | READY (D-12+D-14 = A, MODIFIED) | M | `specs/WP-35-auth-edge-semantics-fail-open.md` |
| WP-38 | bot-turn wedge recovery | `web/src/{game/mod,admin,nats}.rs`, `bot/src/main.rs` | ws F27 (ADJUSTED minor->major), wd F1, wd F2, wd F3, bo F2 | READY (D-5 = C-lite, MODIFIED) | **L** | `specs/WP-38-bot-turn-wedge-recovery.md` |
| WP-45 | bot-slot validation choke point | `web/src/proposals.rs`, `game/server_fns.rs`, `db.rs`, `email/commands.rs` | wd F27, wfe F18 | READY (D-8 = option C) | S | `specs/WP-45-bot-slot-validation.md` |
| WP-47 | game_visibility gates | `web/src/game/server_fns.rs`, `stats/`, `db.rs` | wd F17, wd F45 | READY (D-6+D-13 = option A) | S | `specs/WP-47-game-visibility-gates.md` |
| WP-49 | rules and game-info pages | `web/src/{rules,game_info/*}.rs`, `db.rs` | wd F67 | READY (D-6 answered) | M | `specs/WP-49-rules-and-game-info-pages.md` |
| WP-57 | inbound webhook delivery semantics | `web/src/email/inbound.rs` | wfe F2 | READY (D-2 = option A) | M | `specs/WP-57-inbound-webhook-delivery-semantics.md` |
| WP-62 | operator | `operator/src/{controller,crd}.rs` | bo F18 | READY | M | `specs/WP-62-operator.md` |
| WP-63 | fuzz tool | `tools/fuzz/` | bo F26 | READY | S | `specs/WP-63-fuzz-tool.md` |

### 1.2 Blocked on an unanswered decision - SKIP until answered (8)

| WP | Title | Crate(s) | Major finding IDs | Blocked on | Size |
|---|---|---|---|---|---|
| WP-04 | lib-game parser design items | `lib/game/src/command/` | lg F7 | D-38 | M |
| WP-05 | lib color | `lib/color` | ls F12 | D-39 | S |
| WP-46 | sweep delivery semantics | `web/src/email/sweep.rs`, `proposals.rs`, `outbound.rs` | wd F28, wfe F30, wfe F31 | **D-11** (D-2 already answered) | **L** |
| WP-55 | Turnstile SPA rendering | `web/src/app.rs` | wfe F53 | D-16 | S |
| WP-58 | unsubscribe RFC 8058 | `web/src/email/{inbound,render,commands}.rs` | wfe F3 | D-10 | S |
| WP-64 | workspace-deps migration | root `Cargo.toml` + 40 manifests | dp F1 | D-19 | M |
| WP-66 | sqlx unification | web/bot/operator manifests | dp F6 | D-17 | S |
| WP-67 | sentry feature trim | bot/web/lib manifests | dp F12 | D-18 | S |

**WP-46 is the highest-value blocked package** (3 majors, 12 findings). D-11 is
its only remaining blocker. Worth pushing the user for D-11 before this plan's
Tier 2 run finishes.

### 1.3 Partial coverage - exactly what remains

Audited across all 25 finalized specs. Most specs *fence out* the unspecced
packages rather than doing their work; only three real partials exist.

| WP | Already done by | Remains |
|---|---|---|
| **WP-38** | WP-39 shipped the **visibility half** of ws F56 (MAX_DELIVERIES advisory listener + `bot_stream_max_deliveries_total`), the supervised consumer restart loop (ws F53/wd F4), F57 config-drift warning, F58 ack_wait doc, wd F9 stale-conflict re-publish filter, bo F1/F3/F5 | ws F27, wd F1, wd F2, wd F3, wd F5, bo F2. Boundary: **"what gets acked, when, with which AckKind"**. Do NOT re-do WP-39's work. `specs/WP-36-*.md` (~:18, :36, :292, :422) is **STALE** - it attributes consumer supervision to WP-38; WP-39:28 records the correction |
| **WP-65** (Tier 3) | WP-22 Task 5 already removed lords-of-vegas-1's `lazy_static` use and dependency | remaining `lazy_static`->`LazyLock` sites, e F9 test-module naming sweep (red7-1 keeps `tests`), stale Cargo.toml template files, deps CI job |
| **WP-72** (Tier 3, blocked) | WP-03 Task 8 deletes `combine` from `lib/game/Cargo.toml` (inferred, not stated in-spec) | only the `lib/markup` half of dp F15. Confirm whether dp F15 is now a one-file package |
| WP-02 | WP-01 fixed ls F1 only (the char/byte panic in `transform.rs`) | all of ls F2-F11. Rebase on WP-01's edits; WP-01 and WP-06 both forbid touching `parser.rs`/`wrap.rs`/`lib.rs` before WP-02 |
| everything else | nothing of substance | full declared scope |

### 1.4 Scope ROUTED IN by LEAD RULING in finalized specs

These widen packages beyond `work-packages.md`. A Tier 2 spec that misses them
will leave a known defect unowned.

- **WP-09** gains: acquire-1's two `panic!("must be Phase::SellOrTrade")`
  (`specs/WP-19-*.md`:838) and sushizock-2's unbounded `Player{}` `target` index
  (`specs/WP-21-*.md`:1079) - **both crates must be added to WP-09's crate
  list**; the workspace-wide `Gamer::player_state` totality gap
  (`specs/WP-21-*.md`:1081); red7-1's `num_players` trust (already folded into
  `work-packages.md`). Note WP-28 Task 3 **deliberately keeps**
  `self.hands[player]` panicking so WP-09's red test stays reproducible - that
  is correct, do not "fix" it early. WP-06 must NOT be retro-edited to carry
  the `gamer.rs` bounds check.
- **WP-10** gains starship-catan-1's `peeking` JSON exposure in `player_state()`
  (`specs/WP-13-*.md`) - a new instance of the class, not in WP-10's 2-finding
  scope. WP-13 Task 5 already render-guards it; the JSON level is WP-10's.
- **WP-35** gains: the **web** email-change + confirmation-link + re-verification
  flow (WP-56 deleted the email path, so the web path is now the only one);
  the duplicated `DELETE FROM login_confirmations` at `auth/server.rs` (use
  `db::delete_login_confirmation`); the `cap_digest`/`find_active_turn_games`
  off-by-one disclosure.
- **WP-62** gains a second major by LEAD RULING (`specs/WP-28-*.md`:727):
  `upsert_game_type_and_version` is last-writer-wins on
  `game_types.player_counts`/`blurb`/`weight` across versions of one type, so
  Lost Cities may advertise 2 players though `-2` supports 3. Design call
  (union counts vs non-deprecated-only).
- **WP-08** gains acquire-1, starship-catan-1 and the lost-cities double
  placings-log site; red7-1 is explicitly **not** WP-08's.
- **WP-04** gains two items routed by `specs/WP-03-*.md`:1315/1317 (`Spec::Int`
  suggest `unwrap_or(1)` floor; `Enum::parse` lowercased dedupe key collapsing
  "Red"/"red").
- **WP-57** needs the missing `AppState`/webhook test fixture - none exists for
  the email handlers, and this will block coverage.
- Tier 3 riders routed in: WP-43, WP-50, WP-52, WP-53, WP-58, WP-60, WP-64,
  WP-69, WP-70, WP-71 (see `specs-LOG.md`'s Worker 2 entry for the item list).

### 1.5 Unowned items the Orchestrator must file

1. **Email-originated game moves never call `notify_game_emails`** - the other
   players receive no turn email. Major functional gap, discovered while
   writing WP-51. WP-51 explicitly forbids folding it into WP-59 or WP-40 and
   proposes a new spec-time package. **No owner today.**
2. `get_available_bots` does not guarantee the default `bot_name: "medium"`
   (`specs/WP-54-*.md`:2007) - "if no package owns it, no owner - Lead to file".
3. **db.rs module split (ws F42)** - a deferred future package, must land after
   WP-35/40/45/47/49/50/52/53/59. Do not fold it into any decision-blocked
   package.
4. **D-15 is reopened** by WP-59: the email `end` verb collides with acquire-1's
   and starship-catan-1's top-level `end` move. User decision required.

### 1.6 Tier 2 batches - one batch = one future Worker

| Batch | Packages | Shared concern | Notes for the Worker brief |
|---|---|---|---|
| **T2-B1** | WP-34, WP-35 | `web/src/auth/` + `crypto.rs` | WP-41 must land before WP-35's `db.rs` edits. WP-35 absorbs the WP-56-orphaned web email-change flow (1.4) |
| **T2-B2** | WP-47, WP-45, WP-49 | web read-path privacy + write-path validation | **WP-41 must land first for all three.** WP-41 Task 8 added a *second in-file copy* of `is_game_visible_to_user` with a mandatory cross-reference comment - WP-47 must reconcile, not fork it. WP-42 (Tier 3) will consume the same predicate. WP-49 also owns `rules.rs:46`'s error surfacing |
| **T2-B3** | WP-38, WP-57 | at-least-once delivery / ack semantics | **WP-37 must land before WP-38** (shared `admin.rs`; if WP-38 lands first, re-derive WP-37 Tasks 6-7). **WP-59 must land before WP-57.** WP-38's residue only (1.3). WP-57 has no test fixture (1.4) |
| **T2-B4** | WP-09, WP-10 | game-crate state boundary: trust in, redaction out | Largest batch. WP-09 may split into a requester-boundary package + a per-crate sweep at spec time - that is expected. If it does, land the boundary half before the bulk of the per-crate game packages. Both gain routed-in scope (1.4) |
| **T2-B5** | WP-02, WP-08 | core-lib robustness + cross-crate dedup | WP-02 rebases on WP-01. WP-08 must first pick a refactor shape (shared `lib/game` helper vs identical per-crate extract) - that is the spec's first decision, and it also fixes e F14's duplicate-log amplification |
| **T2-B6** | WP-62, WP-63 | non-web Rust services and tooling | WP-62 carries the routed-in `game_types` major (1.4) and bo F25's k8s-openapi pin needs the **deployed cluster version - confirm with Michael at spec time**. WP-63 may dedupe against `brdgme_rand_bot::commands()` after WP-07 |

Blocked packages (1.2) get no batch. When a decision lands, slot the package
into the nearest batch by concern: WP-04/WP-05 -> T2-B5, WP-46 -> T2-B3,
WP-55/WP-58 -> a new web batch, WP-64/WP-66/WP-67 -> T3-B7's manifest batch
upgraded to Tier 2.

---

## 2. Tier 3 roster

Zero-major packages. Deliverable per batch: **one crate checklist file** in
`planning/checklists/`, a table with one row per finding: `finding id | file |
one-line fix | test needed (Y/N)`. **No specs.**

Verification rule: where a verification file exists it **supersedes** the raw
finding - corrected severities and REJECTED verdicts must be honoured.

### 2.1 Dispatchable now (16 packages, 8 batches) - **ALL WRITTEN, ROSTER COMPLETE 2026-07-26**

Every batch below has a finalized checklist. Do not re-run any of these.

| Batch | Packages | Findings file(s) | Verification file? | m | n | Covered by |
|---|---|---|---|---|---|---|
| **T3-B1** | WP-31 (zombie-dice-2 + battleship-2), WP-32 (for-sale-2 + category-5-2) | `findings/games-batch-f.md` | **yes** `verification/games-batch-f.md` | 7 | 12 | `checklists/T3-B1-zombie-battleship-forsale-category5.md` |
| **T3-B2** | WP-33 (greed-2, farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2) | `findings/games-batch-f.md` | **yes** | 4 | 13 | `checklists/T3-B2-small-game-crates.md` |
| **T3-B3** | WP-17 (splendor-2 + `lib/cost`), WP-18 (texas-holdem-2) | `findings/games-batch-b.md`, `games-batch-c.md`, `lib-support.md`, `dependencies.md` | b/c/ls **yes**; dp **no** | 5 | 7 | `checklists/T3-B3-splendor-libcost-holdem.md` - **WP-17 is PARTIALLY blocked**, see 2.1a |
| **T3-B4** | WP-24 (sushi-go-2), WP-27 (love-letter-2 + age-of-war-2) | `findings/games-batch-d.md`, `games-batch-e.md` | **yes** both | 4 | 11 | `checklists/T3-B4-sushigo-loveletter-ageofwar.md` |
| **T3-B5** | WP-52 (stats/query perf), WP-53 (domain misc server fns) | `findings/web-domain.md` | **no** (lead-verified) | 14 | 13 | `checklists/T3-B5-web-domain-stats-misc.md` (was not split) |
| **T3-B6** | WP-60 (outbound tokens/metrics/render), WP-42 (websocket pass) | `findings/web-frontend-email.md`, `web-server.md` | wfe **no**; ws **yes** | 7 | 6 | `checklists/T3-B6-outbound-email-websocket.md` |
| **T3-B7** | WP-61 (bot service quality), WP-43 (web cargo deps) | `findings/bot-operator-tools.md`, `dependencies.md`, `web-server.md` | bo/dp **no**; ws **yes** | 10 | 7 | `checklists/T3-B7-bot-service-web-deps.md` |
| **T3-B8** | WP-65 (workspace hygiene), WP-74 + WP-75 (red7-1 RULES.md docs) | `findings/dependencies.md`, `games-batch-e.md`; WP-74/75 have **zero finding IDs** | dp **no**; e **yes** | 5 | 6 | `checklists/T3-B8-workspace-hygiene-red7-docs.md` |

Batch-specific notes:

- **T3-B2** is deliberately WP-33 alone (17 findings across 5 crates) - splitting
  it out keeps T3-B1 inside one pass.
- **T3-B5** is the biggest single-file read (27 findings in a 646-line findings
  file with no verification file). If the Worker reports budget pressure, split
  WP-52 and WP-53 into T3-B5a/T3-B5b.
- **T3-B6 / WP-42 is not purely a checklist.** Its 4 findings are minor/nit, but
  D-13 assigned it real design work: Task A authenticates the `/ws` upgrade and
  filters per socket using **WP-47's** `is_game_visible_to_user` (do not fork
  it); Task B is a NEW `sub`/`unsub` protocol for public-game pages. **Land
  WP-47 first.** Anonymous upgrades must keep returning 101 - never 401
  (`tests/websocket_hygiene.rs` asserts this). Flag to the Orchestrator: WP-42
  may deserve a Tier 2-style compact spec despite having no majors.
  **RESOLVED 2026-07-26: it got one.**
  `specs/WP-42-websocket-auth-and-filtering.md` is the authoritative document
  for WP-42; the T3-B6 checklist covers WP-60 and defers to that spec.
- **T3-B8 / WP-75 is NOT writable from source alone.** It needs a live render
  captured from a real game state (recipe at `docs/authoring/RULES_AUTHORING.md`
  ~:56-64, requires a DB and a built binary) and a ruling on whether the shipped
  `BASIC_STRATEGY.md`/`ADVANCED_STRATEGY.md` satisfy the mandatory "Strategy
  Tips" section. The checklist row must say so; do not attempt the capture.
  Both WP-74 and WP-75 must land after WP-29 Task 5 and after WP-30 (parked).

### 2.1a WP-17 is PARTIALLY blocked, not fully blocked (resolved 2026-07-26)

`work-packages.md` labelled WP-17 `BLOCKED-ON-DECISION(D-25)` at package level.
That overstated the gate. The authoritative reading is
`checklists/T3-B3-splendor-libcost-holdem.md`:

- **D-25 gates exactly 3 of WP-17's 8 findings: `b F31`, `ls F39`, `dp F27`** -
  the `lib/cost` keep-or-fold consolidation, which is one change seen from three
  findings units and must be implemented as one change.
- The other 5 (`b F30`, `b F32`, `b F34`, `b F35`, `ls F38`) are splendor-2 and
  `lib/cost` hardening/cleanups with no dependency on D-25 and are
  **implementable now**.

So WP-17 is **PARTIALLY-BLOCKED-ON-DECISION(D-25)**. Both this file's 2.1 row
and `work-packages.md`'s WP-17 heading now say so.

### 2.2 Blocked on an unanswered decision - SKIP (7 packages)

| WP | Title | Blocked on | m | n |
|---|---|---|---|---|
| WP-48 | export/import | D-7 (privacy posture) | 1 | 4 |
| WP-50 | email canonicalization | D-9 | 4 | 0 |
| WP-69 | deny.toml hardening | D-23 - land LAST among dep packages, after WP-66/67/68 shrink the duplicate set | 3 | 0 |
| WP-70 | serde_yaml migration | D-21 - both consumers move together; re-point WP-07's new `StateYaml(#[from] serde_yaml::Error)` variant | 3 | 0 |
| WP-71 | warp -> axum | D-22 - must re-apply WP-06 Task 1's SystemError mapping + body cap and port its route tests | 2 | 0 |
| WP-72 | combine posture | D-24 - now likely `lib/markup` only (1.3) | 1 | 0 |
| WP-73 | game-bins consolidation | D-20 - sequence after WP-64. e F45's `[dev-dependencies]` recommendation is **INVALID** | 3 | 1 |

---

## 3. Prevention package inputs

**DONE 2026-07-26** - delivered as `planning/CODING-md-amendment-proposal.md`
(6 rules). The filename planned below (`planning/CODING-amendment-proposed.md`)
was never used; do not look for it.

One consolidated `docs/CODING.md` amendment **proposal**, written as
`planning/CODING-md-amendment-proposal.md` (writes outside `planning/` are
forbidden; follow the precedent of `planning/BACKLOG-note-proposed.md`). Draw
one rule per root cause, each from the named source: **(1) char-vs-byte
indexing** - never derive a byte index from a char count; source
`specs/WP-01-char-byte-panic-elimination.md` and findings lg F1-F4, ls F1, e F29
(5 of the 10 criticals), plus the workspace non-ASCII input test convention
WP-01 introduces (NBSP, multi-byte names - the core libs had zero non-ASCII
coverage). **(2) No panics on request-reachable paths** - no `unwrap`/`expect`/
unchecked indexing/unguarded arithmetic between the HTTP boundary and the game
crates; sources `specs/WP-06-lib-cmd-tools-http.md` (ls F19, the production warp
handler unwrap), `specs/WP-21-cathedral-sushizock-fixes.md` (c F29 `i32::MIN`),
`specs/WP-03-lib-game-parser-mechanical.md` (lg F6 zero-progress infinite loop),
and WP-09's e F18/e F36 for the deserialize boundary. **(3) A privacy gate on
every query** - `is_game_visible_to_user` (or its successor) called by every
read path, one predicate not two; sources findings wd F17, wd F45 (WP-47),
`specs/WP-44-proposals-integrity-email-token-leak.md` (wd F26, credential
serialized to every viewer), WP-10's f F1/f F13 for the `pub_state` half, and
D-13/ws F59 for the socket. **(4) TOCTOU guards on state mutation** - every
mutating statement carries its own `is_finished` + `updated_at` predicate in the
`WHERE` clause and asserts rows-affected, never a snapshot check followed by an
unguarded write; sources
`specs/WP-40-undo-concede-toctou-ratings-integrity.md` (wd F14 critical, wd F15,
wd F16, ws F34) and ws F1 (WP-34's atomic-UPDATE race; note the original
recommendation's off-by-one - compare with `>`, not `>=`). **(5) At-least-once
delivery semantics** - mark after success, never before; return 5xx so the
sender retries; claim-then-act inside a real transaction; heartbeat long work
so it is not redelivered; sources D-2's answer (option A) in
`decisions-needed.md`, WP-57 (wfe F2), WP-46 (wfe F30, wfe F31 - `FOR UPDATE
SKIP LOCKED` is a no-op outside a transaction), and `specs/WP-39-*.md` +
WP-38 (bo F1, bo F2, wd F1-F3) for the NATS ack side. **Two optional extra
rules to put to the user, not assumed:** deterministic iteration (no `HashMap`/
`HashSet` in serialized or RNG-consuming game state - b F18, d F2, e F16), and
no `#[serde(default)]` on persisted state enums (f F18 - defaulting `Phase` to
`Buying` corrupts live Selling games; use `Option<T>` with an explicit
fallback).

---

## 4. Execution order for the Orchestrator

Dispatch Leads **serially**, one at a time.

1. **T2-B1** - WP-34, WP-35 (auth). WP-34 has never been blocked and is the
   oldest unspecced critical-path-adjacent package.
2. **T2-B2** - WP-47, WP-45, WP-49 (web privacy + validation). Unblocks WP-42
   in T3-B6.
3. **T2-B3** - WP-38, WP-57 (delivery/ack semantics).
4. **Prevention package** - by this point all five root-cause families have at
   least one finalized spec to cite. It is one file and one short session; it
   may be pulled earlier or later without cost.
5. **T2-B4** - WP-09, WP-10 (game state boundary). Largest; expect WP-09 to
   split.
6. **T2-B5** - WP-02, WP-08 (core libs).
7. **T2-B6** - WP-62, WP-63 (operator, fuzz).
8. **T3-B5**, then **T3-B6**, then **T3-B7** (web + services checklists - the
   highest-density minor clusters and the ones whose files the Tier 2 work has
   just churned).
9. **T3-B1**, **T3-B2**, **T3-B3**, **T3-B4** (game-crate checklists - fully
   independent, freely reorderable).
10. **T3-B8** (workspace hygiene + red7 docs) last: WP-65 wants WP-64 (D-19)
    answered first, and WP-74/WP-75 sit behind WP-29 and the parked WP-30.

Out-of-band, any time: push the user for **D-11** (unblocks WP-46, 3 majors),
**D-15** (reopened, blocks nothing but will bite WP-59's landing), and the
1.5 unowned items.

---

## 5. Gotchas carried forward

Every future Lead and Worker brief must reproduce this section. These are
hard-won; they cost budget to learn.

1. **NO exhaustive line-number citations.** Verification found **33-46% of the
   citations in the web-side specs were WRONG**, including two delete ranges
   that would have destroyed live code. Line numbers are permitted only as
   hints, marked "approximate, verify". Every spec must instruct the
   implementer to **locate code by file path + function name**, and to **stop
   and report** if what they find does not match the spec - never to adapt the
   edit.
2. **NO adversarial double-verification pass outside Tier 1.** That is what
   burned the budget. The implementer reads the code anyway. One pass, then
   ship.
3. **The findings' own fix recommendations are unreliable, and so are
   re-derivations.** At least four recommendations were proven invalid: one
   would have introduced a 14-camel bug (d F13, REJECTED); one relied on
   dev-dependencies applying to `src/bin` targets (e F45); one used a
   nonexistent SQL column (ws F30 - `bot_providers` has no `updated_at`,
   REJECTED); one was an unsound serde default (f F18). But **three times a spec
   draft's "re-derivation" was wrong and the original finding was right.** So:
   when a finding is correct, **say so explicitly in the spec** so nobody
   reverts it later.
4. **Keep specs proportionate.** 1000+ line specs for three-line fixes is the
   failure mode to avoid. Tier 2 target is ~1 page: problem, why it is wrong,
   required end state, explicit non-goals, regression test cases. Tier 3 is a
   table row, not prose.
5. **The review was conducted against a snapshot worktree at `f8763a5`; the live
   repo has drifted**, and another agent is landing critical fixes right now.
   Specs must describe **LIVE** code. Re-read before asserting anything about
   current state.
6. **HARD READ-ONLY.** Planning agents write only inside
   `docs/reviews/2026-07-23-rust-review/planning/`. Never modify anything under
   `rust/`. Never run cargo/build/check/test/clippy/fmt. Never run a git
   mutation. Validation is by READING source only. A previous subagent violated
   this and modified game source - do not repeat it.
7. **`lib-support` finding numbering diverges between the raw and verification
   files.** `findings/lib-support.md` has 46 findings;
   `findings/verification/lib-support.md` has 45 - raw F10 (ANSI/plain renderer
   escaping) is absent from verification, so **every raw `ls` number >= 10 is +1
   against verification**. `work-packages.md` uses **verification numbering**.
   Resolving an `ls F10`-or-higher ID against the raw file reads the WRONG
   finding. Affects WP-02, WP-05, WP-06, WP-07, WP-17, WP-65, WP-68, WP-70,
   WP-71.
8. **Two unit tally lines are off by one against their own content**:
   `findings/dependencies.md` says 26 findings but has 27 headings;
   `findings/bot-operator-tools.md` says 30 but has 31. Sequential numbering
   itself is sound (anchored by dp F6/F12/F20 and bo F18/F25/F26/F28).
9. **`ws F67` (WP-43) is UNVERIFIABLE, not rejected** - it needs network access
   to check dependency currency. Mark it as such in the checklist; do not
   silently drop it.
10. **Only two findings were REJECTED review-wide** (games-batch-d F13,
    web-server F30) and both are already excluded from package scope. No
    package shrinks. Do not re-litigate them, and do not implement them.
11. **Landing-order constraints are real and already documented** in
    `planning/landing-order.md` and `critical-path.md` section 4. The ones that
    bind this plan: WP-41 -> WP-40/WP-47/WP-50/WP-52/WP-53/WP-35; WP-37 ->
    WP-38; WP-59 -> WP-57/WP-58/WP-40; WP-54 -> WP-55; WP-68 -> WP-69; WP-64 ->
    WP-65/WP-73; WP-47 -> WP-42; db.rs module split after everything touching
    `db.rs`.
12. **`specs/WP-36-*.md` is stale** where it attributes bot-consumer supervision
    and poison-message stranding to WP-38 (~:18, :36, :292, :422). WP-39 shipped
    that work; WP-39:28 records the correction. WP-36 was never retro-edited.
    Similarly `specs/WP-03-*.md`:1318 still lists an already-applied WP-01
    routing-label correction as open.
13. **Parity is parked.** WP-11, WP-12, WP-16, WP-20, WP-26, WP-30 are
    BLOCKED-ON-USER-RULES-REVIEW, which is stronger than
    BLOCKED-ON-DECISION: it does not clear when a decision is answered, only on
    the user's per-game sign-off. Do not pick them up, do not change gameplay,
    and do not "correct" any `RULES.md` under them - the docs are themselves
    under user review and some content was AI-generated.
