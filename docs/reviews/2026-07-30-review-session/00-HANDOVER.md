# Handover prompt for a fresh Orchestrator session

Paste the block below as the first message of a new session. Everything the new
Orchestrator needs is in this file plus `00-STATE.md`; this conversation is not
required.

**THE SESSION IS COMPLETE.** All 22 review units closed; all three findings
extractions done; `99-UNIFIED-REPORT.md` (1,945 lines, 11 sections, 219 rows) and
`98-REMEDIATION-PLAN.md` (55 work packages R-01..R-55) both delivered. Everything
below is history. **No further review work is outstanding** - the next step belongs
to the owner: the three decisions in the plan's section 6, and whether to commit
this directory.

**Resolutions already made - do not re-litigate.** The unified report resolved
every extraction discrepancy: 04b's stray `F-78` is **void** (F-78 is 04c's Low
WP-33 row); the routing-leak conflict went the reports' way, so the live set is
**one High (F-60) + one Medium (F-55)**, not two Highs, with F-57 excluded under
the WIP ruling; F-104 is pattern **4f** with citations `bot/src/config.rs:28`
**and** `:67`; `deny.toml`'s skip list is **29**, not 24; report 07 has **23**
findings (its header's "13" is a miscount); F-161a-d are now formally declared and
roll up into F-161; F-06 is **15 of 28 overriding, 13 not** (the "15 of 27" heading
is wrong, and report 02's `seven-wonders-1` warning was not reproducible against
`00-sweeps.md`). `00-breakdown.md`'s premises were wrong **four** times and every
error inflated apparent work - never size from it.

**One question remains genuinely open** (unified report 8.12): whether Unit 07b
covered the whole WP-51/WP-53 surface after its quota-death re-dispatch. Resolving
it means re-walking `dcd8844c` and `3610b957`. It is proposed as a small
verification work package in the remediation plan.

---

We are resuming an orchestrated code review. All review units are finished; what
remains is the final compilation. Invoke `/sop` and act as the Orchestrator. **Do
no work in this parent session** - no investigation, no code reading, no tests.
Delegate everything to Leads. A lean parent session is what lets a long session
finish.

**Read `/home/beefsack/Development/brdgme/docs/reviews/2026-07-30-review-session/00-STATE.md`
before doing anything else.** It is the authoritative record: ground rules, the
unit progress table, confirmed systemic patterns, owner rulings, and a large set of
carry-forwards organised per unit. Do not re-derive anything it already contains.

## What this session is

A review of a large multi-week remediation effort - 127 commits, 2026-07-25 to
2026-07-30 - that implemented fixes for the 570-finding corpus in
`docs/reviews/2026-07-23-rust-review/SUMMARY.md`. We are reviewing **the
remediation work itself** for quality and correctness: real bugs, hacks,
shortcuts, and whether each fix actually closed the finding it claimed to.

Repo: `/home/beefsack/Development/brdgme`, branch `master`.

## Hard constraints - put in every brief, verbatim

- **Never run tests, benchmarks or lints. Running Rust tests crashes the machine.**
  `git`, `rg`, `wc`, `ls` and file reads only.
- Do not commit, stage or push. Do not modify source. Read-and-report only; the
  only files anyone writes are the reports in the session directory.
- Reports stay **uncommitted** until the owner says otherwise.

## Role/model ruling

**Opus for every role - Orchestrator, Leads and Workers alike.** Never pass a
`sonnet` model override to any subagent; pass no `model` override at all so
everyone inherits opus. One subagent at a time, serially, at every tier - never two
dispatches in one response.

## Where we are

**All 22 review units done. 211 findings (F-01..F-211).** Every unit report is on
disk in the session directory. The web half produced denser and more severe
findings than the game half.

**Unit 11 (the last review unit) closed** with F-208..F-211 - 1 High, 2 Medium,
1 Low, plus two refutations. Its results, which are NOT yet folded into
`00-STATE.md`'s pattern list:

- **F-208 (High): `hanamikoji-1` is unshippable.** No `rust/Dockerfile` stage, no
  `docker-bake.hcl` target, and no k8s Deployment. It is compiled by
  `rust/Dockerfile:36`'s `--workspace` build on every image build and then never
  copied out. `rg 'hanamikoji'` finds zero hits outside the crate, `rust/Cargo.toml:13`
  and `Cargo.lock`. A complete, tested, documented new game with no build- or
  deploy-time signal that it does not ship.
- **F-210 (Medium):** `ae04843c` turned sushi-go-2's `_ => 9` into
  `_ => unreachable!()` on the premise "start() rejects counts outside 2..=5".
  sushi-go-2 has **no `validate` override at all**, `all_players` is never bounded,
  `Game` derives `Default` with all-`pub` fields, and `draw_count(self.all_players)`
  is called from `command()` at `:289` - past the D-36 boundary. `all_players: 0`
  now panics the game service where it previously dealt 9 cards. **Remediate as one
  item with F-06's sushi-go-2 row.** Second WP-72-class self-certifying commit, and
  the first where the self-certified premise is demonstrably false.
- **F-209 (Medium):** hanamikoji-1's `validate` bounds every parallel vector and
  seat index but never relates `phase` to `pending`. `OpponentChoose` + `pending:
  None` passes validate, reports `Status::Active` naming a player to move, and
  returns `Err` for both seats forever. Textbook pattern 2b.
- **REFUTED - do not re-derive.** (1) The carry-forward that hanamikoji-1 has an
  unguarded epilogue is **wrong**: `:796` `let was_finished = self.is_finished();` /
  `:830` `if !was_finished && self.is_finished()`, identical to jaipur-2; `:833` is
  inside the guard, and there is a dedicated regression test
  `test_finish_emits_epilogue_once`. It does **not** join the F-18/F-71 list. Also:
  `finish_epilogue` is a per-crate inherent method (12 crates), not a
  `rust/lib/game` helper; only `placings_log` is shared. (2) Unit 10b's "43 k8s
  Deployments vs 26 image stages" premise is **refuted** - 43 = 26 Rust + 17 legacy
  Go games with stages in `brdgme-go/Dockerfile`. Bake is identical to Dockerfile
  stages; zero stages lack a Deployment. The only real gap is F-208's.
- **Carry into the unified report:** hanamikoji-1 is the **first crate in the
  session with a `validate` test** (`:1079`) - pattern 2b's "no crate has one" is
  now broken; use it as the model and F-209 as proof that having one is not
  sufficient. Its `Status::Finished` populates `stats` (`:734-737`), a **negative**
  for the F-35 tally. Notable inversion worth stating plainly: the crate written
  *after* the review internalised the review's lessons better than most crates the
  review remediated, and both its gaps are in areas no checklist covered. New
  process-fix item: three (four, cross-repo) hand-maintained delivery lists with no
  cross-check - needs a CI guard, not just a hanamikoji-1 fix.

## Remaining work - the final compilation, split into five sequential Leads

11k lines of unit reports do not fit one Lead. The compilation was therefore split
into three extraction Leads (findings normalized into tables, by unit range), then
a composition Lead, then a remediation-breakdown Lead.

**Steps 1-4 are DONE**: all three extraction tables (`90-findings-part1/2/3.md`)
and the unified report (`99-UNIFIED-REPORT.md`, 1,945 lines). The composition Lead
was killed by quota after section 5 and a successor completed sections 6-11 - the
incremental-flush rule saved the work for the seventh time this session.

**Only step 5 remains: `98-REMEDIATION-PLAN.md`.** Its brief: ordered work packages
(`R-01`...) with objective, F-numbers closed, files, S/M/L sizing, dependencies, and
**acceptance criteria written to defeat the decoys this review found**; F-161 first;
the pre-grouped items below kept as single work packages; a deployment-checklist
section (F-96, `TURNSTILE_SITE_KEY`, `public_base_url()`, F-207, F-211 - not code
findings); process fixes centred on the four-tooth sign-off rule; coverage work
carrying all 18 unowned items U1-U18 from unified report section 7; owner decisions;
an explicit out-of-scope list; and a one-screen summary table. Sources are
`99-UNIFIED-REPORT.md` (delegate it by section - it is 1,945 lines), `00-STATE.md`
and the three `90-` tables. **Not** the 22 unit reports, and no `rust/` source.

**Pre-grouped - each is ONE work package, never split:** F-104+F-138+F-183+F-189
plus re-fixturing the decoy F-185 (one bot-name case-sensitivity defect spanning
four units; fix = canonicalize inside `validate_bot_slots` and return the canonical
name); F-199+F-206+10b's Coverage gap 3; F-128+F-173+the `CanonicalEmail` newtype
(closes the F-124/F-127 class); F-145+F-136; F-162+F-169; F-06's sushi-go-2 row +
F-210; F-113+F-116+F-117 (one `left_at` schema change).

### Historical: extraction part 3 (DONE)

Write `90-findings-part3.md`, same table format as parts 1 and 2 (read part 1's
header and a few rows to match precisely - `| ID | Severity | Unit | WP |
file:line | One-line summary | Remediation-pairing / status notes |`).

Range **F-158..F-211**, from `09-web-frontend-email-sse.md` (1503 lines,
F-158..F-185 - **split this across Workers**), `10-bot-operator-tools.md`
(F-186..F-196), `10b-dependency-workspace-hygiene.md` (F-197..F-207) and
`11-hanamikoji-unassociated.md` (F-208..F-211).

Same rules as parts 1/2: one row per ID including sub-letters (F-161d exists);
severity verbatim or `unstated`, never invented; summary states the defect, not the
fix; status column captures REFUTED / DOWNGRADED / owner rulings / remediation
pairings, with `00-STATE.md` winning any conflict and the conflict noted. End with
`## Severity tally` and `## Discrepancies`. **`## Discrepancies` must also capture
substantive items carrying no F-number** - part 1 found 2 such orphans, part 2
found **43**, so this is the highest-value part of the deliverable. Extraction
only: do not open any `rust/` source file, do not re-verify, do not add findings.

### Historical: composition Lead - the unified report (DONE)

Built the narrative from `90-findings-part1/2/3.md` plus `00-STATE.md` without
re-reading the unit reports. Delivered `99-UNIFIED-REPORT.md`: executive summary;
severity distribution with a stated normalization rule; the most severe findings;
renumbered systemic patterns; a counting-integrity section; verified good and 26
refutations plus 11 discharged obligations; coverage gaps consolidating 86
unnumbered items and ending in the U1-U18 unowned table; discrepancies and
corrections; owner decision items; the sign-off rule; and the full 219-row table.

## Extraction results so far - the composition Lead needs all of this

**Part 1 (F-01..F-084, 85 rows):** Critical 0, High 9 (F-05, F-06, F-09, F-17,
F-22, F-36, F-60, F-61, F-72a), Medium 30, Low 43, plus 1 informational (F-58),
1 Nit (F-77), 1 unstated (F-27, withdrawn). Net remediable rows: 78.

Discrepancies to resolve:
1. **F-78 ID collision.** 04c's F-78 is Low/live (WP-33 `saturating_add`
   half-landed); 04b also files an `F-78`, WITHDRAWN, outside its allotted
   F-66..F-77 range. The table carries 04c's. Nothing was renumbered.
2. **Unresolved severity conflict.** `00-STATE.md`'s routing-leak pattern says two
   of F-55/F-57/F-60 are High; the reports rate F-60 High, F-55 Medium, F-57 Low.
   Pick one and say which.
3. `00-STATE.md` overrides five statuses: F-81 not a finding; F-50/F-57 WIP
   excluded; F-35/F-41/F-58 parked in WP-20; F-15 DISCHARGED/LATENT; F-18's crate
   list is **five**, not four (+`battleship-2` via F-71).
4. **F-06's own denominators contradict** ("15 of 27" heading vs "15 of 28 / 13
   without" body), and report 02 warns its crate list omits `seven-wonders-1`.
   Re-derive mechanically from `00-sweeps.md`.
5. **Two in-range items have no F-number and will be lost:** `Gamer::points()`
   ordering contract + cathedral-2's inverted sign (03b coverage gaps only; Unit 08
   handed it back); and 04b's escalation of F-01 (`for-sale-2` `pass()` panics on
   `open_cards.remove(0)`).
6. Minor: F-82's pattern-2b membership disputed; F-59's WIP-exclusion undecided;
   F-61's WP inferred from prose; several citations are crate-level, not `file:line`.
7. **Answered in range:** `for-sale-2`'s half-bid rounding **is** inside the WP-11
   park (`f F14`, D-30+D-35) - not a remediation gap. This closes an open Unit 04c
   question in `00-STATE.md`.

**Part 2 (F-085..F-157, 75 rows** - 73 numbered, no gaps or duplicates, plus 2
`(no F-number assigned)` rows from the F-96 report**):** High 14, Medium 32, Low 17,
`Low/Medium` 2, `Low, note` 2, `Low, informational` 2, REFUTED 1, unstated 2, plus
F-96 `High/DOWNGRADED` and F-129/F-130 `Medium/ESCALATED`. **Five qualified-severity
buckets need normalizing by the composition Lead.**

Discrepancies (7 report-vs-`00-STATE` conflicts, `00-STATE` applied): F-96
downgraded; F-104 labelled 4b in-report but 4f in `00-STATE`, and its citations
differ (`00-STATE` adds the uncited `config.rs:67`); F-109's remediation is
bookkeeping, not a revert of `efad81f9`; F-128 not closed and ownerless;
F-129/F-130 escalated to account takeover (the condition fired via F-161); F-142
framing.

ID anomalies: report 07 orders its findings non-monotonically and its own header
miscounts 23 findings as 13. F-123 is REFUTED/ARCHIVED (re-issued as F-133, remnant
F-132); F-110 is explicitly "not a defect". Both kept as rows. WP attribution is
inferred for F-104/105/107/108/133.

**43 unnumbered items** are captured in part 2's `## Discrepancies`. The ones that
most change the deliverable:
- **`left_at` conflates two meanings** - F-113/F-116/F-117 are **one** schema-change
  item, not three.
- **The no-spec WP list is longer than `00-STATE.md` records**: add WP-36, WP-43,
  WP-44, WP-52, WP-53. **WP-36 is the crypto package**, so Unit 05's highest-stakes
  verdict rests on a commit message alone.
- **A second decoy test with no F-number**: `rating_before_aggregates_exclude_nulls`
  never calls `game_history` - this is the actual source of `00-STATE.md`'s F-109
  sign-off sharpening (ii).
- **`00-breakdown.md`'s Unit 08 sizing is wrong** (91 of 95 files are sqlx cache
  JSON; deletion-risk surface near zero). This is the **fourth** time a breakdown
  premise has been wrong.
- Three email bearer tokens with three different lifecycle disciplines - one
  combined remediation item.
- 11 coverage gaps with no owner, including no request-parts test harness and
  `require_admin`'s true path untested for 13 of 16 server fns.

## The brief template that works

Refined across 22 units. Include, in this order:

1. "You are a Lead under the orchestrate skill; invoke it and assume the Lead role.
   Delegate all shell commands to Workers, one at a time, serially - never two
   dispatches in one response. You may read files yourself but must not run shell
   commands."
2. The model ruling above.
3. The hard constraints, verbatim.
4. "Read `00-STATE.md` first", then the specific unit section. Point at the state
   file; do not re-explain accumulated context in the brief.
5. The unit's exact scope, and explicitly what belongs to a *neighbouring* unit so
   two Leads don't overlap.
6. Prior finding range and the number to continue from.
7. **The method** (below) plus the subsystem-specific things that matter most.
8. Named carry-forwards from `00-STATE.md` that this unit must act on, as
   first-class deliverables rather than extras.
9. Budget warning: "hard 150k; if it won't fit, stop and report a split -
   splitting always beats overrunning."
10. Report path and structure, and **"write incrementally - flush each finding as
    you confirm it; unflushed context is lost work."**
11. "Do not stop to wait for worker notifications - read worker output files and
    keep driving until the unit is complete."
12. "Reply with: findings count by severity, the 3 most serious in one line each,
    coverage, and anything to carry into later units. Under 40 lines."

## The method - this is what produces the real findings

Recover the work package's spec from git history first, then check the **end
state** against each acceptance criterion individually, reading both the diff and
the final code.

- Specs and the full finding corpus were compacted out of the tree (`d89fa345`).
  Recover: `git show 868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/<file>`
  Listing cached at `03a-specs.md`. **Have each Lead verify spec existence itself** -
  several WPs have none, and an Orchestrator brief was wrong about which once.
- Every high-value finding in this session came from a commit that satisfied its
  checklist row **literally** while breaking or missing what the row was for.
  Commit messages and checklists read clean in all of those cases.
- Reasoning from commit messages or spec narrative alone has produced **false**
  findings. Always verify against final code. **Instruct every Lead that if it
  cannot demonstrate a failing path concretely it must mark the item REFUTED with
  the evidence rather than ship it** - eleven Leads did this and several refutations
  were among the most valuable output of their units.
- Treat large mechanical diffs as guilty until read - but **verify file counts
  yourself**. `00-breakdown.md`'s premise has now been wrong **four** times (Unit
  06's gotcha was a false premise; Unit 08's 95 files were really 9 - and part 2's
  extraction re-confirmed the sizing error independently; Unit 10b found WP-66's 101
  files were really 12). Build-artifact noise (`.sqlx/`, `Cargo.lock`) inflates
  every count.

## Session mechanics that proved essential

- **Incremental flushing is why nothing was lost.** Three Leads were killed by API
  529s and three by quota limits. Every one that flushed as it went let its
  successor resume with zero rework; the one that died before flushing lost only
  spec recovery.
- **Split before dispatching.** Most Leads overran the 150k budget (155k-214k), and
  the overruns were source reading, not extraction. Units 01, 03, 04, 05, 07, 09
  and 10 all had to be split mid-flight. When unsure, split smaller.
- **Interrupt a Lead that overruns** rather than letting it grind: tell it to stop
  investigating, flush everything confirmed, mark `## Progress` honestly, and write
  a handover section for its successor.

## Things to surface to the owner, not to a Lead

- **The vendoring policy question is open and is the owner's** - see the dedicated
  section in `00-STATE.md`. The owner considers vendoring something that should be
  forbidden except when genuinely blocked. WP-66's spec did gate it correctly and
  the port was faithful, yet it still produced F-200 and an unmachine-checked
  licence obligation. **A repo-wide sweep for other vendored code has never been
  done** and is a remediation-plan item.
- **F-96 is resolved** - see its section in `00-STATE.md` and
  `F-96-turnstile-key.md`. It downgrades to a pre-rollout deployment blocker, not a
  code defect. Two deployment items must land together with it:
  `TURNSTILE_SITE_KEY` has no startup check (setting only the secret key is a total
  login outage), and `config::public_base_url()` defaults to `http://localhost:3000`,
  which would make WP-58's `List-Unsubscribe` RFC 8058-invalid in prod.
- **F-81 was ruled not a finding** (reconstructing hidden information from public
  logs is acceptable by design). The ruling is general, not `no-thanks-2`-specific,
  and its boundary matters: direct leaks into `Log::public` are still findings.
- **`lords-of-vegas-1` is work in progress** by owner ruling - no findings about
  missing or incomplete functionality there.

## Housekeeping for the unified report

- **The most severe finding is F-161** (WP-56's inbound auth gate is fail-open
  three ways), because it escalates F-129 + F-130 to **account takeover** under a
  condition Unit 07 set explicitly and Unit 09a confirmed. It belongs at the top of
  the remediation order.
- `00-STATE.md`'s systemic-pattern list grew organically and its numbering is out
  of order (4b, 4c, 4e, 4f, 4d interleaved). **Renumber and consolidate.** Patterns
  that earned promotion: the "Test? y with no test" gap (nine falsified rows) and
  pattern 2, inconsistent hardening within one file, which was the single most
  productive pattern in the session.
- **Do not conflate "untested by design" with falsified rows.** WP-72/76/77/79/80
  and every Unit 10 commit have no checklist row at all, or rows marked `Test? = n`.
  Counting them toward the nine-strong tally would inflate the process-failure
  narrative with an artifact of counting. `00-STATE.md` records which is which.
- **The sign-off rule to recommend has three teeth**, each learned from a real
  decoy: a closed finding's citation must still exist (F-109), must be
  **reachable** (F-147 - the citation exists and has never had a caller), and its
  regression test must **actually call the function under test** (F-151, F-161d,
  and the unnumbered `rating_before_aggregates_exclude_nulls`).
- Newly named patterns worth carrying: the documentation-only constant (F-153,
  F-170); a vendoring WP inheriting an upstream defect that "minimal port"
  guarantees comes along (F-200); a spec's stop-work trigger answered with a
  comment rather than a stop (F-206); a disproved finding closed rather than
  amended (F-205); a self-certifying WP that exists only as a commit, with no spec
  and no checklist row (WP-72, and now `ae04843c`/F-210 where the self-certified
  premise is demonstrably false).
- 01c's checkmarks are **epilogue-shape only** and must not be read as crate-level
  coverage.
- `roll-through-the-ages-2` has never had a crate-level review (3,290 lines, no
  `validate` override, no redaction test, and the one function anyone read
  contained F-83). Out of scope for a review-of-the-remediation; recommend a
  dedicated pass in the remediation plan.
- Several findings **must be remediated as single items** rather than separately -
  the pairings are recorded per unit in `00-STATE.md`'s carry-forwards and in the
  status column of `90-findings-part1/2/3.md`. The largest is
  F-104 + F-138 + F-183 + F-189, one bot-name case-sensitivity defect spanning four
  units. Also F-113 + F-116 + F-117 (one `left_at` schema change) and F-210 + F-06's
  sushi-go-2 row.
