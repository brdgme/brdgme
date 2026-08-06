# Historical Sign-off Manifest (D1)

Compact canonical sign-off input for the 4.x units, extracted exclusively from
the deleted 2026-07-23 review corpus. Extraction only; no sign-off checks
implemented, no dependent unit executed.

## Source

- Corpus source commit (the parent of the corpus deletion):
  `23f8ab78e015c127f4e809d4901467e494e21bb3`
- Corpus deletion commit (the deletion this manifest replaces):
  `d89fa345019ec9d52d5e56e7c6c2affa98cd7b8d`
- Corpus root: `docs/reviews/2026-07-23-rust-review/`
- Sign-off-relevant corpus sub-roots (all paths below are relative to this
  root unless absolute):
  - `planning/work-packages.md` - WP registry (85 headings), status tags,
    per-WP finding counts (coverage table).
  - `planning/specs/` (47 active WP specs) and `planning/specs/archive/`
    (13 archived WP specs) - provenance and STOP/deferral fields.
  - `planning/checklists/` (T3-B1..B8) - per-WP finding rows incl. `Test?`.
  - `planning/EXECUTION-STATE.md` - closure/landing register, parked items,
    decisions, rulings, surfaced/dropped items.
  - `planning/EXECUTION-README.md` - deliberate-gap notes (section 8),
    archive-spec status (section 9), citation guidance (section 7).
  - `findings/*.md` + `findings/verification/*.md` + `findings/raw/*.md` -
    finding text, `location:` citations, verification status.
  - `planning/specs-CLASSIFICATION.md`, `PROGRESS.md`, `handover.md` - status
    corroboration. NOT ingested: unrelated review corpora and raw logs not
    needed for the fields below.

## Extraction method

All values below were derived with exact git commands against the source
commit; nothing was read from the live worktree or restored to it.

- Corpus file listing: `git ls-tree -r --name-only 23f8ab78... -- docs/reviews/2026-07-23-rust-review/`
- WP registry headings: `git grep -oE '^### WP-[0-9]+[ab]?' 23f8ab78... -- .../work-packages.md`
- Spec enumeration: `git ls-tree --name-only 23f8ab78... -- .../planning/specs/ .../planning/specs/archive/`
- Checklist rows: `git grep -n -E '^\| `' 23f8ab78... -- .../checklists/T3-B*.md`
  (132 rows match; 3 are T3-B8 non-finding gate rows, so 129 finding rows).
- Closure register: `git show 23f8ab78...:.../planning/EXECUTION-STATE.md`
- STOP/deferral fields: `git grep -n -i -E '...' 23f8ab78... -- .../planning/specs/**`
- Every cited path:line reference below was re-verified against the source
  commit after extraction (see Verification).

## Counts (at source commit)

| Item | Count |
|---|---|
| WP headings in `planning/work-packages.md` | 85 (WP-01..WP-85; WP-09 is a registry-only split entry, WP-09a/09b carry the work) |
| Active spec files | 47 |
| Archived spec files | 13 (= the 13 pre-landed WPs) |
| Checklist files | 8 (T3-B1..T3-B8) |
| Checklist finding rows | 129 (52 `Test? y`, 77 `Test? n`) |
| Findings covered by triage (coverage table) | 570 (10 crit / 78 major / 257 minor / 225 nit) |
| Pre-landed WPs (before tree start `37118d3`) | 13: WP-01,03,06,13,14,15,21,25,36,37,39,41,44 |
| Landed-with-commit WPs in closure register | ~49 |
| Staged-patch WPs (done per tracker, no commit SHA in corpus) | 13: WP-07,09a,09b,22,34,35,42,47,49,57,81,83,84 |
| WPs with neither spec nor checklist row | 12: WP-11,12,16,20,26,30,72,76,77,78,79,80 |

## WP provenance and closure inventory

Legend - Spec: `A` active spec, `X` archive spec, `C` checklist row(s), `N`
neither, `(in WP-69)` content embedded in another spec. Closure: commit SHA
(8-char prefix as recorded in the corpus), `stg` = staged patch per tracker
(no SHA recorded in corpus), `pre` = pre-landed, `park` = parked, `skip` =
skipped by ruling, `SUP` = superseded, `DEF` = deferred, `UNC` = uncommitted,
`fold` = folded into WP-09a/09b.

| WP | Spec | Closure | Notes (as recorded in corpus) |
|---|---|---|---|
| 01 | X | pre | |
| 02 | A | 91f2682 | |
| 03 | X | pre | |
| 04 | A | 8215754 | |
| 05 | A | 4a978cb | Two STOP-AND-REPORT triggers resolved by orchestrator 2026-07-29 |
| 06 | X | pre | |
| 07 | A | stg WP-07.patch | |
| 08 | A | f13450a | + c14bc65 (WP-08b riders) |
| 09 | N | - | registry-only; split into 09a/09b at spec time |
| 09a | A | stg WP-09a.patch | receiving package for routed items (WP-19/WP-21) |
| 09b | A | stg WP-09b.patch | receiving package for routed items (WP-19/WP-21) |
| 10 | A | 90dae6d | f F1, f F13, starship sensor; 3-crate scope |
| 11 | N | park | BLOCKED-ON-USER-RULES-REVIEW |
| 12 | N | park | BLOCKED-ON-USER-RULES-REVIEW; `a F1` released |
| 13 | X | pre | |
| 14 | X | pre | |
| 15 | X | pre | |
| 16 | N | park | BLOCKED-ON-USER-RULES-REVIEW |
| 17 | A,C | 614cf4f | T3-B3 rows b F31/ls F39/dp F27 |
| 18 | C | 84b68b9 | |
| 19 | A | 07ad476 | two panic sites ROUTED to WP-09 (not folded) |
| 20 | N | park | BLOCKED-ON-USER-RULES-REVIEW |
| 21 | X | pre | two items ROUTED to WP-09 (not folded) |
| 22 | A | stg WP-22.patch | |
| 23 | A | a692b63 | |
| 24 | C | 6605315 | d F28 `_ => 9` arm SURFACED, accepted |
| 25 | X | pre | |
| 26 | N | park | BLOCKED-ON-USER-RULES-REVIEW |
| 27 | C | eb49cec | |
| 28 | A | ed88fab | |
| 29 | A | 071ace6 | PARTIAL; Task 4/e F31 PARKED (P1) |
| 30 | N | park | BLOCKED-ON-USER-RULES-REVIEW |
| 31 | C | f16cb02 | |
| 32 | C | 807ab4e | f F24 RULED MOOT at T3-B1 (keep 8) |
| 33 | C | abffb7a | 17 findings |
| 34 | A | stg WP-34.patch | |
| 35 | A | stg WP-35.patch | |
| 36 | X | pre | Task 5 = ws F55 WebSocket close frames + `websocket_hygiene.rs` |
| 37 | X | pre | |
| 38 | A | 914aa0c | |
| 39 | X | pre | |
| 40 | A | 9ba3736 | |
| 41 | X | pre | routed items to WP-44/46/52/40/54 |
| 42 | A,C | stg WP-42.patch | rescoped to visibility predicates (D-44) |
| 43 | C | a9609e5 | ws F63-F67 |
| 44 | X | pre | |
| 45 | A | c1c1d20 | |
| 46 | A | 69bcd1e | tx-aware send_reminder; handover `planning/wp46-handover.md` |
| 47 | A | stg WP-47.patch | |
| 48 | A | 7092294 | + 5e9bae2 de-flake |
| 49 | A | stg WP-49.patch | |
| 50 | A | 33f22f1 | migration 026 |
| 51 | A | dcd8844 | Task 3 full dedup not done (would regress WP-46) |
| 52 | C | f374434 | wd F50 ELIGIBILITY_PREDICATE doc-const |
| 53 | C | 3610b95 | |
| 54 | A | fddc42d | inherited-from-WP-41 routed note |
| 55 | A | f0a468b | |
| 56 | A | 4ca73ec | + da1ea24; Task 2 classify_inbound_auth |
| 57 | A | stg WP-57.patch | |
| 58 | A | 390dd3b | migration 025 |
| 59 | A | f56ff37 | Task 14 dropped (WP-85 deferred) |
| 60 | C | e5513ec | wfe F44-F51+F63 |
| 61 | C | 4f5f6d4 | bo F4-F16 + dp F7 |
| 62 | A | e682f6b | |
| 63 | A | d2decf8 | |
| 64 | A | 4fb252d | no checklist rows by design |
| 65 | C | 2c28ae8 | all 9 rows `Test? n` |
| 66 | A | 667c8f4 | step-0 gate honored, then branch B vendoring |
| 67 | A | 634c72d | dp F12; finding text never amended |
| 68 | A | 618156a | |
| 69 | A | UNC | deny.toml hardening; ~30-entry skip list; §5 negative checks parked |
| 70 | A | 8304baf | dp F14 backend half OPEN |
| 71 | A | dcec1ad | |
| 72 | N (in WP-69 §3d) | a5d6f10 | no spec file, no checklist row |
| 73 | A | 22d00b8 | |
| 74 | C | skip (P4) | queued behind parked WP-30 |
| 75 | C | skip (P4) | escalated; live-render capture needed |
| 76 | N | bc05116 | + ca7925b (T-b); deliberate no-spec gap (EXECUTION-README:408) |
| 77 | N | 33150af | deliberate no-spec gap |
| 78 | N | SUP | superseded by WP-82 |
| 79 | N | 91c723d | deliberate no-spec gap |
| 80 | N | fold | folded into WP-09a/09b; deliberate no-spec gap |
| 81 | A | stg WP-81.patch | |
| 82 | A | 4d31f6e | |
| 83 | A | stg WP-83.patch | |
| 84 | A | stg WP-84.patch | SSE migration (deleted websocket code) |
| 85 | A | DEF | deferred, blocked on Michael |

Tier-3 follow-up commits (cross-cutting, not WP closures): T3-B3 `0688e03`,
T3-B4 `3174b3f`, T3-B5 `46847d4`, T3-B6 `2b116b2`. T3-B1/B2/B7/B8: zero code
changes, tracker-only.

## Fields per dependent unit

### 4.1 four-tooth-core

Fields needed per closed finding (source in brackets):
- Finding ID, WP ownership: checklists (ID column, WP section headers) and
  coverage table `planning/work-packages.md`.
- Closure status: closure register above (`planning/EXECUTION-STATE.md`).
- Cited symbol + file + snapshot line: `location:` fields in
  `findings/<unit>.md` (and `findings/raw/*.md`). Corpus citation guidance:
  `planning/EXECUTION-README.md` section 7 (navigate by symbol, not line).
- `Test?` value and named test: checklist rows (register in 4.2).

Named audit targets from 98-REMEDIATION-PLAN 4.1, crosswalked to the corpus:
- Tooth 1 (F-109): WP-36 Task 5 "F55 WebSocket close frames on graceful
  shutdown" + test file `rust/web/tests/websocket_hygiene.rs` at
  `planning/specs/archive/WP-36-crypto-deploy-hardening.md:290-307`. Deletion
  via WP-84 SSE migration (plan cites `efad81f`; corpus records WP-84 as a
  staged patch, no SHA).
- Tooth 2 (F-147): symbol `send_turn_reminder`. Corpus context is WP-46
  (`send_reminder`, commit 69bcd1e; `planning/wp46-handover.md`); the symbol
  itself and its "no caller" state are current-codebase facts the unit checks
  live, not corpus fields.
- Tooth 3 (F-151, F-161d): decoy tests. F-151 context WP-52 (wd F50/wd F51
  stats, commit f374434); F-161d context WP-56 Task 2
  (`classify_inbound_auth`, commit 4ca73ec). Named tests are current-codebase
  symbols.
- Tooth 4 (F-205): dp F12 (WP-67, commit 634c72d) never true; the unamended
  finding text lives in `findings/dependencies.md` (finding text unchanged),
  not in the corpus closure notes.

Retirement note (user decision 2026-08-06): the permanent CI sign-off checker
(`scripts/check-four-tooth.sh`), its committed contract harness
(`scripts/check-four-tooth.test.sh`), all sign-off fixtures, and the CI job
that ran them were retired/rejected by user decision and deleted. No checker
input contract or automated enforcement remains. The 4.1 material above is
retained purely as historical reference data extracted from the deleted corpus;
this manifest's role as canonical historical sign-off input is unchanged.

### 4.2 test-row-sweep

The 52 `Test? y` rows are the sweep universe. Per row the unit must read the
row's Fix column for the prescribed change and the named test, then verify a
test exists and its body references the function under test (tooth 3).

`Test? y` rows by checklist (finding - line in the checklist file):

- T3-B1 (WP-31): `f F3` 24, `f F6` 26, `f F8` 28. (WP-32): `f F17` 40,
  `f F18` 41, `f F24` 46, `f F25` 47, `f F27` 51.
- T3-B2 (WP-33): `f F32` 25, `f F39` 33, `f F40` 34, `f F38` 36, `f F45` 44,
  `f F48` 45, `f F49` 54, `f F51` 55, `f F58` 62, `f F56` 63, `f F57` 64.
- T3-B3 (WP-17): `b F30` 40, `b F32` 43, `b F31` 77, `ls F39` 78,
  `dp F27` 79. (WP-18): `c F1` 55.
- T3-B4 (WP-24): `d F27` 30, `d F28` 31. (WP-27): `e F5` 52, `e F8` 58,
  `e F12` 67, `e F15` 73.
- T3-B5 (WP-52): `wd F50` 28, `wd F51` 29, `wd F55` 31, `wd F48` 38,
  `wd F52` 40, `wd F46` 46, `wd F21` 52. (WP-53): `wd F6` 76, `wd F25` 87,
  `wd F61` 99.
- T3-B6 (WP-60): `wfe F44` 42, `wfe F45` 43, `wfe F46` 44, `wfe F63` 65.
- T3-B7 (WP-61): `bo F4` 38, `bo F6` 45, `bo F9` 51, `bo F10` 57, `bo F11` 63,
  `bo F13` 70. (WP-43): `ws F66` 98.
- T3-B8: none (all `Test? n`).

Not-to-count exclusions (98-REMEDIATION-PLAN 4.2): WP-76/77/79/80 (no
checklist, deliberate gap, EXECUTION-README.md:408); WP-65 all rows `n`;
WP-64/66/67/69/70/73 no checklist rows (deferred); WP-72 no checklist and no
spec. T3-B8 also carries 3 non-finding decision-blocked/gate rows (dp F9
CLEARED, WP-74, WP-75) at source lines 137-139 - these are NOT finding rows.
The WP-75 open question at line 140 is not a gate row either.

### 4.3 wp-provenance

WP IDs come from the 85 registry headings (list in the inventory). Provenance
gate (spec file or checklist row) per WP is the Spec column of the inventory.
The 12 WPs failing both: WP-11,12,16,20,26,30 (parked rules-review, never
executed), WP-72 (content inside WP-69 spec section 3d), WP-76,77,79,80
(deliberate no-spec gap, EXECUTION-README.md:408-410), WP-78 (superseded by
WP-82, no spec). WP-09 is a registry alias, not a work package.

### 4.4 deferral-routing

Routing records in the corpus (sender spec -> receiver):
- WP-19 spec -> WP-09: two `panic!("must be Phase::SellOrTrade")` sites,
  acquire-1 to be added to WP-09 crate list
  (`planning/specs/WP-19-acquire-fixes.md:850`).
- WP-21 (archive) spec -> WP-09: sushizock-2 player-index guard, plus the
  structural `Gamer::player_state` item (`planning/specs/archive/WP-21-cathedral-sushizock-fixes.md:1079,1081`).
- WP-41 (archive) spec -> WP-44, WP-46, WP-52, WP-40, WP-54(note-only)
  (`planning/specs/archive/WP-41-db-quality-pass.md:1980-1986`).
- WP-54 spec: "inherited from WP-41, routed INTO this package" note
  (`planning/specs/WP-54-frontend-ux-error-handling.md:2016`).
- WP-80 -> WP-09a/09b fold (EXECUTION-README.md:419).
- WP-85 deferred, blocked on Michael (`planning/specs/WP-85-email-parser-first-dispatch.md:3,22`).

Receiving packages with active specs that should list inherited findings:
WP-09a, WP-09b, WP-40, WP-44, WP-46, WP-52, WP-54. The plan's three
named closed-as-routed cases (F-55/F-57/F-60) are unified numbers NOT present
in the corpus; the corpus-level records above are the extractable set.

### 4.6 stop-escalation

Substantive STOP/HALT triggers in active/archive specs (file:line), beyond the
boilerplate "STOP and report rather than improvising" header most specs carry:
- WP-69: intro file-mismatch STOP (line 15); 3a "if the live file has a
  different count... STOP and report" (line 76); 3b "if the residual list
  runs past roughly a dozen entries... STOP and report" (line 91); 3c cargo
  deny complaints STOP (line 104).
- WP-04:157 (3b alignment incomplete), WP-40:43, WP-49:57, WP-56:46 and
  WP-56:224 (no verdict field), WP-59:1103, WP-62:80, WP-54:1055/1963.

Recorded owner responses (corpus):
- WP-69 §3b: corpus records ~30-entry skip list shipped in UNCOMMITTED work
  with a "not papered-over sibling work" claim; plan F-206 asserts the claim
  is falsified and cites commit `e2ee5342` (not in corpus). §5 negative
  checks ("the flip must actually bite") recorded as NOT run, parked
  (`planning/EXECUTION-STATE.md` WP-69 row).
- WP-05: both STOP-AND-REPORT triggers resolved by orchestrator 2026-07-29
  (`planning/EXECUTION-STATE.md`, "ORCHESTRATOR decision" entries).
- WP-56:224: resolved - Resend docs confirmed no verdict field; WP-56 Task 2
  landed 4ca73ec.
- WP-66 step 0 binding gate: honored (upgrade-first attempted, failed on
  tower-sessions-sqlx-store, branch B vendored) - `planning/EXECUTION-STATE.md`
  WP-66 row.

### 4.9 pattern-sweeps

Named pattern anchors in the corpus (plan 4.9 fields):
- Pattern 2 (sibling hardening): WP-09 (guarded one function); WP-40 `WHERE id
  = $1 AND NOT is_finished`/`undo_game` sibling - landing evidence is WP-53
  `wd F6` is_eliminated CASE guard (commit 3610b95).
- Pattern 5 (`_ => default`): `d F28` draw_count unreachable `_ => 9` arm,
  WP-24, SURFACED not fixed (T3-B4 tracker note).
- Doc-only constant: `wd F50` ELIGIBILITY_PREDICATE const (WP-52, commit
  f374434; decision "documented-const").
- Log::public content: WP-10 redaction scope (f F1, f F13, starship sensor);
  no game crate test of the log layer (plan F-22/F-28 unified refs).
- "For every game crate" scope claim: WP-10 spec intro "This WP decides the
  redaction shape once for every game crate" applied to 3 crates
  (`planning/specs/WP-10-pub-state-hidden-info-redaction.md`).
- `allow(dead_code)` sweep universe: the commit range in the closure register
  (all landed/staged commits above).

## Unavailable / ambiguous data

- Staged WPs (07,09a,09b,22,34,35,42,47,49,57,81,83,84): closure recorded as
  done via `WP-NN.patch`; no commit SHA exists in the corpus. Their actual
  landing commits (e.g. plan-cited `efad81f`) are outside the corpus.
- WP-69: work done but UNCOMMITTED at source; skip-list size recorded as
  "~30" in EXECUTION-STATE and "29" in the plan (F-206) - count ambiguity
  recorded, not resolved.
- Plan unified finding numbers (F-55, F-57, F-60, F-109, F-142, F-147, F-148,
  F-149, F-150, F-151, F-161d, F-171, F-200, F-205, F-206, F-208, F-61,
  F-116, F-65, F-136, F-153, F-22, F-28, F-104, F-120) are NOT present in the
  deleted corpus; crosswalks to batch IDs are given above where determinable,
  and marked "current-codebase" where they are not corpus facts.
- `ws F55` and `dp F12` batch IDs resolve in the corpus only via
  WP-36-spec:290-307 and the EXECUTION-STATE WP-67 row respectively; the
  findings files use descriptive headings, not batch IDs.
- WP-09's coverage-table row (19 findings) spans WP-09a + WP-09b; the split
  is not itemised there.

## Verification

- Both commits resolve: `git cat-file -t 23f8ab78e015c127f4e809d4901467e494e21bb3`
  = commit; `git cat-file -t d89fa345019ec9d52d5e56e7c6c2affa98cd7b8d` = commit.
- Every corpus path cited above was listed by `git ls-tree` at the source
  commit; every `file:line` cited above was grep-verified at the source
  commit.
- `git diff --check` clean (run after writing).
- Counts reconciled: 47 + 13 = 60 WP spec files (matches EXECUTION-README);
   129 checklist finding rows = 52 y + 77 n (the raw 132-row grep includes 3
   T3-B8 non-finding gate rows).
- No worktree restoration performed; only the manifest file was created.
