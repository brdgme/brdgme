# Orchestrator handover - brdgme Rust review planning

Written 2026-07-26 at the end of the **third** planning session.
Resume from this file. It is the single entry point for a successor Orchestrator.

## What this effort is

A 570-finding Rust code review (10 critical / 78 major / 257 minor / 225 nit)
lives in `docs/reviews/2026-07-23-rust-review/`. **The review itself is
COMPLETE** - no more reviewing is needed. This planning effort turns the
findings into material a small cheap model can execute.

A **separate agent** is concurrently executing fixes under `rust/` on its own
branch. Never write under `rust/` from a planning session. Expect drift.

## Session rules that must carry forward

Run `/sop` (loads `orchestrate`, act as Orchestrator). Delegate everything to
Leads; the Orchestrator does no investigation and reads almost nothing. Leads
and Workers run on **opus** (Michael's instruction, overrides the skill's
default). Spawn serially, one at a time, never in parallel, at every tier.

### Hard read-only constraints - put verbatim in EVERY Lead and Worker brief

- Write only inside `docs/reviews/2026-07-23-rust-review/planning/`.
- NEVER modify any file under `rust/`. NEVER run cargo/build/check/test/clippy/fmt.
  NEVER run git mutations. Validation is by READING source only.
- Another agent is concurrently editing files under `rust/`. Expect drift.

A subagent violated this early on and modified game source. It is not theoretical.
The third session's one-unit grant to write `docs/CODING.md` and
`docs/BACKLOG.md` is **SPENT**: writes are confined to `planning/` again.

### Hard-won lessons - also verbatim in every brief

1. **NO exhaustive line-number citations.** 33-46% of citations in earlier specs
   were WRONG, including two delete ranges that would have destroyed live code.
   Line numbers only as hints marked "approximate, verify". Every spec must tell
   the implementer: read the named function; if it does not match, STOP and
   report rather than improvising.
2. **Never assert anything about code or config you have not read.** If you did
   not read it, mark it UNKNOWN. Prove-before-deleting is a real rule here.
3. **Proportionate documents.** 1000+ line specs for three-line fixes is the
   failure mode being corrected. Tier 2 cap is ~120 lines.
4. **NO adversarial double-verification pass** outside Tier 1. One Worker
   drafts, the Lead sanity-checks and lands it.
5. **Findings' own fix recommendations are unreliable** - 4+ proven invalid. But
   three times a spec's "re-derivation" was wrong and the finding was right.
   Never reject a finding without reading the code that proves it wrong.
6. Review used snapshot `f8763a5`; the repo has drifted. **Specs describe LIVE
   code.**
7. `findings/verification/` **supersedes** raw findings. Numbering hazard: raw
   `findings/lib-support.md` has 46, verification 45; `work-packages.md` uses
   verification numbering.
8. `specs-LOG.md` commit SHAs are **not branch-stable** (the agent rebases), and
   a dead predecessor's claimed WRITES must be verified, not just its LOG claims.

### Durability

Append to `planning/specs-LOG.md` after **every** Worker return and **every**
acceptance. Never hold unwritten state. Several Leads have died from session
limits; the LOG is what makes that survivable.

## Where to start reading

1. `planning/README.md` - directory index and file map
2. `planning/decisions-ANSWERED.md` - D-01..D-34, closed, plus standing constraints
3. `planning/decisions-session3.md` - **D-41..D-53.** Fold into `decisions-ANSWERED.md`.
4. `planning/work-packages.md` - canonical per-package definition
5. `planning/landing-order.md` - inter-package ordering (critical)
6. `planning/specs-LOG.md` - 6100+ line crash log; grep it, never read it whole

## State: what is DONE

- **All decisions answered.** D-01..D-34 in `decisions-ANSWERED.md`, D-41..D-53
  in `decisions-session3.md`. `BLOCKED-ON-DECISION` is extinct.
- **59 specs** in `planning/specs/` (plus `notes-conventions.md`) and **8 Tier 3
  checklists** in `planning/checklists/` (T3-B1..B8).

Landed in the third session specifically:

- **WP-73 game binary consolidation**, specced and then **amended twice** (D-41,
  then D-43 reversing the fuzz half). Net: `brdgme_game_bin` has **three** entry
  points - `cli_main`, `http_main`, `fuzz_main`. The 27 `_fuzz` bins are **KEPT**
  as thin wrappers and `fuzz_gamer` is kept (fuzzing is selected on raw
  throughput). Only the **27 `_repl` bins are deleted**.
  - **Terminology correction, on the record:** there is **no "game-bin macro"**.
    D-20 chose a **generic crate parameterised over the `Gamer` trait**, not a
    macro, approved partly *because* it avoids macros. Keep it macro-free.
- **WP-81** (dead per-game stats machinery removal), **WP-17** (`lib/cost`
  consolidation) and **WP-83** (the three parity fixes released from the park:
  `a F1`, `b F7`, `e F30`) - specs written and landed.
- **WP-84 SSE migration spec finalised** on the settled two-stream multi-topic
  design, with all conditionality removed.
- **WP-42 reworked.** It is **no longer a WebSocket auth package**. The
  pre-upgrade auth dance is SUPERSEDED by WP-84; Task B (`sub`/`unsub`) is
  ELIMINATED; what survives is the transport-independent predicate work
  (`is_proposal_visible_to_user`, WP-47's predicate, the bounded per-connection
  TTL cache). Filename kept so cross-references resolve.
- **The SSE evaluation and topology decision** - `ws-to-sse-evaluation.md`
  (superseded in its recommendation) and `sse-topology-decision.md`. D-46 was
  resolved by D-48 on Michael's measured `HTTP/2 200` against the live edge.
- **The fuzz throughput evaluation** - `planning/fuzz-throughput-evaluation.md`.
- **Doc applications (the one-unit wider permission):**
  - `docs/CODING.md` gained `## Request-Path Invariants` - six rules, applied
    verbatim, insertion anchors re-verified live.
  - `docs/BACKLOG.md` item **#53** (the parity park) was found **ALREADY PRESENT**
    in the working tree (provenance UNKNOWN) and was not duplicated. One stale
    clause was corrected in place: only `a F1` / `b F7` / `e F30` are fix-now,
    `b F4` was re-parked and `d F37` rejected.
  - `docs/BACKLOG.md` item **#54** was ADDED for the maximum-performance fuzzer
    per D-51, carrying all seven must-survive points and linking the evaluation.
  - Both proposal files under `planning/` are now **APPLIED and historical**.
    README.md still describes them as unapplied - stale.

Several packages have also been **implemented** under `rust/` by the executing
agent (WP-06, 13, 14, 15, 21, 25, 36, 39, 41 appear in the LOG and in `git log`).
Not authoritative - that agent rebases. Check `git log` before assuming.

## State: what REMAINS

Spec/checklist coverage is now **effectively complete**. Established by listing
`planning/specs/`, grepping `planning/checklists/`, and reading `work-packages.md`.

- **Not specced and not on a checklist:** **WP-76, WP-77, WP-79, WP-80** - the
  "Unowned / newly discovered" cluster, all small. WP-80 is meant to fold into
  WP-09a/WP-09b. This is the only genuine spec-writing gap left.
- **WP-72** has no file of its own by design (it sits inside
  `specs/WP-69-deny-toml-hardening.md`); **WP-78** is SUPERSEDED by WP-82.
- **WP-74 / WP-75** (red7-1 rules docs) are covered by `checklists/T3-B8`.
  **WP-75 is not spec-writable from source reading alone** - its "Reading the
  Display" section needs a render captured from a live game.
- **Parked** (see below): WP-11, WP-12, WP-16, WP-20, WP-26, WP-30.
- Everything else has either a spec in `planning/specs/` or a row in a Tier 3
  checklist. **The remaining work is mostly EXECUTION**, which is the other
  agent's job.

**Package totals: RECOUNTED 2026-07-26** in `work-packages.md`'s Coverage-check
section, which is the source of truth: **84 packages = 77 READY + 6
BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED (WP-78, by WP-82)**. Method was a
grep over `^### WP-` headings; the three status buckets sum exactly to the
heading count, and `BLOCKED-ON-DECISION` matches 0 headings (extinct). The
570-finding sum and the one-package-per-finding invariant are unaffected -
WP-83 is a **carve-out**, not a re-assignment, so it adds 0 findings and the
three released findings stay counted under WP-12 / WP-16 / WP-30.

**Drift CLEARED 2026-07-26** (planning-corpus cleanup unit): `work-packages.md`
now has a full WP-83 entry under `## Released from the rules park 2026-07-26
(D-35)`, and WP-84's entry was corrected to gate on **WP-42's predicate work**
rather than all of WP-42. `README.md`'s banner, file map, tiering corrections
and implementer rules are all current.

**Open items recorded but deliberately NOT fixed** (scope discipline; a
successor may pick them up):

- `work-packages.md`: the prose above the coverage table still names only
  WP-74/WP-75 as the zero-finding exceptions, but that set has grown to
  WP-74..WP-80, WP-82, WP-83, WP-84.
- `work-packages.md` section headings are chronological, so numeric ordering
  only holds while each new package takes the next free number.
- `specs/WP-50-email-canonicalization.md` still contains two literal `023`
  strings - inside the `DO $$` block's `RAISE EXCEPTION` message and in a
  regression-test description. Its section 3e tells the implementer to update
  them to the real number; they were left rather than guessed.
- `README.md`'s File map row for `decisions-needed.md` says "D-1..D-41", but
  `decisions-session3.md` covers D-41..D-53 - the range may be stale.
- **UNKNOWN, nobody has checked:** whether D-41..D-53 have actually been folded
  into `decisions-ANSWERED.md`. `README.md` still says they are "still to be
  folded in". Verify before quoting `decisions-ANSWERED.md` as complete.
- `tier2-tier3-plan.md` is still stale at source. Only `README.md`'s
  description of it has been corrected.

## Landing order

Full detail is in `planning/landing-order.md` - read it before sequencing
anything; it overrides `BACKLOG.md` phase order where they disagree.

**The WP-82-first rule.** WP-82 (the `db.rs` module split) is a **hard
predecessor for the whole remaining web cluster**: WP-35, 40, 42, 45, 47, 49,
50, 52, 53, 59 and transitively WP-84. It is a pure move - `pub use` re-exports
keep all 293 external `crate::db::foo(...)` call sites compiling - so landing it
first costs those packages nothing, while landing it last makes it a merge
against ten sets of edits to the file it is moving. WP-41 is the one exception:
it has landed, and WP-82 is specced against that shape. See section 7.

**The realtime chain** (`landing-order.md` section 10):
`WP-82 -> WP-47 -> WP-42 (predicates only) -> WP-84`.

Other verified chains: WP-41 -> WP-36 -> WP-34 -> WP-35; WP-51 -> WP-46;
WP-59 -> WP-58; WP-56 -> WP-58; WP-06 -> WP-71; WP-81 before WP-19 (drop WP-19
Task 5); WP-69 last among the dependency packages. Non-`db.rs` work (game
crates, most Tier 3 checklists, WP-51, 54-58) is not gated on WP-82.

**Migration-numbering collision - FOUR packages, not three.**
`landing-order.md` 6.4/6.5 records WP-50, WP-56 and WP-58 as each adding a
migration and each assuming `022` is the highest. **Found this session and not
yet in `landing-order.md`: WP-34 also adds one**, and its spec names
`023_login_email_sends.sql` while WP-50's names `023_canonical_emails.sql` - a
direct filename clash between two specs. The set is **WP-34, WP-50, WP-56,
WP-58**; the second, third and fourth to land must renumber. Fold this into
`landing-order.md`.

**Stale ordering in `specs/WP-50-email-canonicalization.md`.** Its header still
says *"WP-78 (`db.rs` split) is deferred until this lands."* `landing-order.md`
7.3 **withdraws that and reverses the direction** - the item is WP-82, and it is
**WP-82 -> WP-50**. An implementer reading only the WP-50 spec would sequence it
backwards. The spec was not edited (outside the last unit's scope); fix it.

## Parked - do not touch

`BLOCKED-ON-USER-RULES-REVIEW`: **WP-11, WP-12, WP-16, WP-20, WP-26, WP-30**.
Game rules parity is parked pending Michael's own review of `RULES.md` content,
some of which was AI-generated and may be wrong, and because edition choices are
his. D-35: keep the park; review **per game**, prioritising acquire-1,
seven-wonders-1/splendor-2, modern-art-2, red7-1. An implementing agent must not
change gameplay or "correct" a `RULES.md` under these.

Two parity items are resolved and **must NOT be reopened**:

- `b F4` seven-wonders same-turn trade - **re-parked**. Michael's binding
  correction: 7 Wonders resources are **not depleted by trade**; they are printed
  on cards and both players use them, so the "asymmetric advantage" framing was
  wrong. The residual **narrower simultaneity question** (seat order lets p+1
  trade for a card p built that turn) is recorded and parked.
- `d F37` modern-art zero-card artists - **REJECTED, not a bug.** If only one
  artist has cards, 2nd and 3rd go to artists in order from the top; `suits()`
  already returns the canonical order. No follow-up.

## Deferred to a separate session

**Architectural review** - oversized functions/types/files, crate splitting,
module-tree flattening. Deferred until **after remediation**, so it measures the
real post-fix shape rather than a codebase mid-demolition. The review never asked
these questions, so absence of findings is not evidence of absence.
`planning/architecture-observations.md` is the accumulating seed file; Leads
append to it instead of widening a package. **WP-82 was carved out and done now**
because the web cluster would otherwise make it far more expensive.

## Michael's standing preferences (also in agent memory)

- **Dependencies at latest.** For any dependency problem the first step is
  "upgrade everything to latest and see where we stand" before designing a
  workaround.
- **Wary of macros** - maintenance and cognitive cost. Prefer plain code; keep
  any macro small and obvious; pause and discuss if one grows complex.
- **`BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md` are deliberately separate** so
  bot difficulty can be tiered (all bots get BASIC, only hard bots get ADVANCED).
  **Never fold into `RULES.md`.**
- **No Sentry functionality may be lost** in the WP-67 feature trim.
- **The parser must stay straightforward and obvious** (WP-04) - prefer the
  plainer implementation at every choice point.
- Prod mutations need Michael to name the action; hand blocked commands to him
  via the `!` prefix.

## OPEN ITEMS STILL NEEDING MICHAEL

Raise these next session. None blocks a spec. **Two of the four are now
resolved** (items 1 and 4) - kept below as history, do NOT re-raise them.

1. ~~**The WP-84 topic cap.**~~ **RESOLVED 2026-07-26 - D-52** (in
   `decisions-session3.md`). Michael confirmed the Worker's proposed number:
   the cap is **16**, over the cap is a **400**, never a truncation.
   `specs/WP-84-sse-migration.md` section 3c now states it as a settled ruling
   citing D-52, not a proposal the implementer may vary.
2. **Two prove-before-deleting UNKNOWNs in WP-84.** Neither may be assumed:
   whether the `ws_tasks: TaskTracker` can be deleted (spec section 3g requires a
   real-listener graceful-shutdown proof first), and Cloudflare's edge idle
   behaviour for `text/event-stream` (spec section 6).
3. **The accepted window where `/ws` stays unfiltered**, between WP-42 and
   WP-84. WP-42 makes **no** edit to `rust/web/src/websocket.rs` because the
   filter cannot be wired there without the superseded auth dance. Accepted risk:
   skinny UUID payloads, existence and timing only. **Flag it if WP-84 slips.**
4. ~~**`docs/BACKLOG.md`'s "Priority order" block** does not list **#54**.~~
   **RESOLVED 2026-07-26 - D-53** (in `decisions-session3.md`). #54 belongs in
   the **"Then"** tier, **after #31**, alongside #52, #50 and #15 - a faster
   fuzzer makes every subsequent game port and remediation package cheaper to
   validate, and #31 reworks the workspace layout the fuzz bins sit on. Applied
   to `docs/BACKLOG.md`: the priority block now lists #54 in "Then", the block
   is dated 2026-07-26, and the #54 Status row no longer says "unscheduled".

## NEXT SESSION: cleanup and consolidation pass

Michael ended the third planning session here. **The next session's entire job is
a cleanup and consolidation pass.** No new specs beyond item 6, no new
investigation, no widening. His words:

> "The planning directory is a bit of a mess with heaps of files ... I'm not sure
> I want to commit and push all these files in the current state."

The output should be a `planning/` directory Michael is willing to commit and
hand to an executing agent. Six things, in this order.

### 1. Disambiguate `specs/` - HIGHEST RISK ITEM IN THE CORPUS

`planning/specs/` holds roughly **26 compact Tier 2 specs mixed with roughly 25
older bloated pre-tiering specs**, and **nothing in the filenames marks which is
which**. Both sets use the same `WP-NN-name.md` convention.

Why this outranks everything else: the executor is a small cheap model. If it
opens a superseded bloated spec it will act on ~1000 lines whose line-number
citations were measured at **33-46% WRONG**, including **two delete ranges that
would have destroyed live code** (lesson 1 above). The compact spec for the same
package may sit right beside it.

Move the superseded ones into an `archive/` subdirectory, or delete them, so
`specs/` contains **only what an executor should read**. Determine which is which
**BY READING the specs** - not by filename, not by mtime, not by file size. Size
and date correlate but are not proof, and a wrong call here is the expensive
kind.

### 2. Consolidate the decision record into ONE file

Four overlapping decision documents exist:

- `decisions-needed.md`
- `open-decisions-for-user.md`
- `decisions-ANSWERED.md` - D-01..D-34
- `decisions-session3.md` - D-41..D-53

Three of these are historical forms of the same table. `decisions-ANSWERED.md`
currently bills itself as the file that wins over everything else **while being
incomplete**; a banner was added pointing at `decisions-session3.md`, but that is
a stopgap, not a fix.

Merge them into a single sorted `DECISIONS.md` and retire the rest. Note
`decisions-session3.md` is **NOT in numeric order** - D-52/D-53 sit between D-50
and D-51, and D-42 is last. **Sort on merge.**

### 3. Retire the process exhaust

- `specs-LOG.md` - 234KB+ of crash log, written so a Lead could die without
  losing state. That job is done. Retire it.
- `ORCHESTRATOR-HANDOVER.md` - this file. Retire it too (see item 4).
- `tier2-tier3-plan.md` - stale at source; only `README.md`'s description of it
  was ever corrected. Retire it.

### 4. Replace the handover with a short `EXECUTION-README.md`

What execution actually needs and nothing else:

- the landing order,
- the **WP-82-first rule**,
- the **migration renumbering rule - FOUR packages: WP-34 / WP-50 / WP-56 /
  WP-58**,
- the parked list, plus the two resolved parity items that must **NOT** be
  reopened (`b F4` re-parked, `d F37` rejected),
- the hard-won lessons about **line-number citations** and **proportionate
  specs**.

Not the session history. Not the orchestration rules. Not the state narrative.

### 5. Refresh `README.md` LAST

Once everything else has moved. `README.md` went stale **three separate times**
in this session because it was touched before the things it describes settled.
Make it the final step, not the first.

### 6. Decide WP-76 / WP-77 / WP-79 / WP-80

These four are unowned with no specs - the only real coverage gap left. Establish
whether they are on the critical path by **READING `landing-order.md` and
`work-packages.md`**; do not guess. Then either write the specs, or record them
as a **deliberate, documented gap** with the reasoning.

### Ground rules for the cleanup pass

The cleanup pass is itself a planning session under the same rules: **read-only
with respect to `rust/`, opus for all roles, serial dispatch at every tier, no
cargo, no git mutations.**

**Do the destructive moves LAST and in ONE pass**, after the consolidation writes
have landed. A Lead dying mid-session must leave the corpus **readable rather
than half-dismantled**. This is not hypothetical: a half-applied `README.md` edit
caused exactly this problem in this session.
