# EXECUTION-README

The reference document for the agent that **executes** the 2026-07-23 Rust review
remediation. Everything here is what execution needs. No session history, no
orchestration narrative.

Assembled 2026-07-26 by reading `landing-order.md` (596 lines), `work-packages.md`
(1455), `specs-CLASSIFICATION.md` (655), `DECISIONS.md` (1274), `BACKLOG.md` (173)
and `README.md` in full.

---

## 0. Authorities, in precedence order

| Rank | File | Holds |
|---|---|---|
| 1 | `planning/DECISIONS.md` | **All rulings.** D-01..D-56, no gaps. Where a ruling contradicts a spec, a checklist, `work-packages.md` or an older recommendation, **THE RULING WINS.** |
| 2 | `planning/landing-order.md` | **Pairwise ordering constraints.** Overrides `planning/BACKLOG.md` order where they disagree. |
| 3 | `planning/BACKLOG.md` | **Global phase order**, Phase 0-7. See the two warnings in section 2. |
| 4 | `planning/work-packages.md` | Canonical WP list, 85 packages, per-package status and scope. |
| 5 | `planning/specs/WP-*.md` | The task detail. 47 spec files (plus `notes-conventions.md`); the 13 landed specs sit in `specs/archive/` - provenance only. |

`planning/checklists/` holds the Tier 3 per-crate checklists (T3-B1..B8), worked
after the WP suite.

### Files that are NOT authorities - do not obey them

- **`planning/decisions-ANSWERED.md`** - **DELETED, does not exist.** Ignore any
  citation of it anywhere in the corpus. It was superseded, and its banner **"this file covers
  D-01..D-34 ONLY"** and its **"All 34 open decisions are CLOSED"** are **FALSE as a
  coverage claim**: `34` is the number of **table rows**, not a range. Its table
  includes D-35/37/38/39/40, six `N-` items and six finding ids, and **excludes**
  D-1..D-6, D-12, D-13, D-26..D-34 and D-36. Never use "D-01..D-34" as a range. This
  false banner is the reason a D-35..D-40 "gap" once looked real; `DECISIONS.md`
  states plainly: *"D-35..D-40 were searched for and FOUND - there is no numbering
  gap. Do not re-open this as a gap."*
- **`planning/archive/decisions-needed.md`**,
  **`planning/archive/open-decisions-for-user.md`**,
  **`planning/archive/decisions-session3.md`** - all moved to `planning/archive/` and
  all superseded by `DECISIONS.md`.
- **`planning/critical-path.md`** - its own banner says **"STALE - HISTORICAL
  SNAPSHOT"**. Citations are against worktree `f8763a5`. Ignore it.

### Same-basename trap

There are **two** files called `BACKLOG.md`:

- **`docs/reviews/2026-07-23-rust-review/planning/BACKLOG.md`** - the remediation
  phase order. This is the one the execution order means.
- **`docs/BACKLOG.md`** - the product backlog. Different file, different purpose.
  Only D-51/N-5 touch it.

`planning/README.md` and `planning/archive/ORCHESTRATOR-HANDOVER.md` both write the bare
`BACKLOG.md` when they mean the planning one. Always resolve the full path.

---

## 1. WP-82 FIRST. This is a hard rule.

**Land `WP-82` (the `db.rs` module split) before anything else in the web cluster.**

`landing-order.md` section 7: *"WP-82 lands FIRST, before every remaining package
that writes into `rust/web/src/db.rs`"* - namely **WP-35, WP-40, WP-42, WP-45,
WP-47, WP-49, WP-50, WP-52, WP-53, WP-59**, and transitively **WP-84**.

Why first costs nothing: the split is *"a **pure move**: same functions, same SQL,
same signatures, and `pub use` re-exports in `db/mod.rs` keep all 293 external
`crate::db::foo(...)` call sites compiling unchanged."* (The WP-82 spec phrases the
same count as "293 `db::` references outside `db.rs`". Treat the number as
indicative, not verified.) Landing it **last** instead
makes it a merge against ten sets of edits.

**WP-41 is the single exception** - it has already landed, and WP-82 is specced
against the post-WP-41 shape.

Not gated on WP-82: the game crates, the Tier 3 checklists, WP-51, WP-54, WP-55,
WP-56, WP-57, WP-58.

---

## 2. The order to work in

### Two warnings about `planning/BACKLOG.md`

1. **Its `D-nn` blocker tags are stale.** It was written when decisions were open;
   Phase 0 is "get answers from Michael" and every package carries a D-tag. All
   decisions are now ruled in `DECISIONS.md`. **Use its ordering; ignore its blocker
   tags.** Phase 0 is finished - skip it.
2. **It has no knowledge of WP-76 through WP-85, nor of the WP-09 a/b split.** It
   covers WP-01..WP-75 only. Those twelve packages are sequenced in section 2.3
   below, not by BACKLOG.md.

### 2.1 Phase order (from `planning/BACKLOG.md`, blocker tags stripped)

**Phase 0** - nothing to do. Decisions are answered.

**Phase 1 - security and data-corruption criticals**
WP-44, WP-56, WP-01, WP-40, WP-14, WP-25, WP-36, WP-45

**Phase 2 - platform correctness majors + early unblockers**
WP-64, WP-68, WP-39, WP-38, WP-46, WP-57, WP-47, WP-42, WP-34, WP-35, WP-49,
WP-06, WP-07

**Phase 3 - game correctness majors**
WP-13, WP-15, WP-22, WP-23, WP-28, WP-19, WP-21, WP-09 (now WP-09a then WP-09b),
WP-10, WP-03, WP-02, WP-41, WP-37, WP-59, WP-58

**Phase 4 - rules adjudication**
WP-26, WP-16, WP-30, WP-20, WP-11, WP-12, WP-17, WP-29
**Six of these eight are PARKED** - see section 4. Only **WP-17** and **WP-29** are
workable, and WP-29 sequences after WP-30, which is parked, so treat WP-29's
ordering note as satisfied only if its own spec says so.

**Phase 5 - quality, consistency, cleanup**
WP-54, WP-55, WP-51, WP-60, WP-52, WP-53, WP-50, WP-48, WP-61, WP-62, WP-63,
WP-08, WP-27, WP-24, WP-18, WP-31, WP-32, WP-33, WP-04, WP-05, WP-43

**Phase 6 - dependency structure**
WP-66, WP-67, WP-70, WP-71, WP-73, WP-72, WP-65, **WP-69 LAST**
**WP-64 is this phase's precondition and is scheduled in Phase 2 - it is `READY`,
not landed. Do not skip it.** WP-73 and WP-65 both require it.

**Phase 7 - documentation follow-ups**
WP-74, then WP-75. Both touch `rust/game/red7-1/RULES.md`; both must land after
WP-29 Task 5 and after WP-30. WP-30 is parked, so **Phase 7 is effectively blocked**
- do not force it.

**Then** the Tier 3 checklists in `planning/checklists/`: T3-B1..B4 game crates,
T3-B5..B7 web/domain/email/bot, T3-B8 workspace hygiene and red7-1 docs.

### 2.2 Verified ordering constraints - these override the phase order

Realtime chain (`landing-order.md` s10.1):
```
WP-82  ->  WP-47  ->  WP-42 (PREDICATE WORK ONLY)  ->  WP-84
```
WP-42 contributes only `is_proposal_visible_to_user` plus the TTL cache; its
WebSocket work is dead - see D-44 in section 6.

Other verified chains:

| Constraint | Reason (from `landing-order.md`) |
|---|---|
| WP-41 -> WP-36 -> WP-34 -> WP-35 | WP-36 changes `crypto::load_key`'s return type; WP-35 rewrites the same function |
| WP-41 -> WP-40 | VERIFIED. WP-40's spec: *"If WP-41 has NOT landed: stop and say so."* |
| WP-51 -> WP-46 | whichever lands second **rebases on, not forks,** the other's shape |
| WP-59 -> WP-57 | both own `email/inbound.rs`; WP-57 widens WP-59's `fetch_inbound_text` return |
| WP-59 -> WP-58 | WP-59 Task 5 defers every unsubscribe concern to WP-58 |
| WP-56 -> WP-58 | avoids a merge in the same file |
| WP-37 -> WP-38 | WP-37 Task 1 reshapes every `#[server]` fn in `web/src/admin.rs` |
| WP-37 -> WP-38 -> WP-55 | same file; land WP-55 last, keep WP-37's rewritten `"/"` bounce |
| WP-54 -> WP-55 | both rewrite the same `Effect`; **WP-55 rebases onto WP-54's arm, must not fork it** |
| WP-06 -> WP-71 | both touch `lib/cmd/src/http.rs`; WP-71 ports the fixed surface to axum |
| WP-09a -> WP-09b | WP-09a is a hard prerequisite |
| WP-09a -> WP-21 Task 10 | Task 10 must **carry the guard forward into the helper**, not drop it |
| WP-81 before WP-19 | then **drop `c F11` / Task 5 from WP-19**. Whichever lands second must NOT resurrect `stats.rs` |
| WP-64 first, WP-69 **last** | among the dependency packages (D-19, D-23) |
| WP-38 / WP-46 | either order, both own `email/sweep.rs`; second one rebases |
| WP-56 / WP-59 | either order; whichever goes second drops the dead items |
| WP-10 / WP-13 | independent, either order |

Do-not-collide notes: **WP-28 Task 3 deliberately leaves `self.hands[player]`
panicking** so WP-09a's red test stays reproducible - do not "fix" it in WP-28.
**WP-06 must not be retro-edited** to carry the `gamer.rs` bounds check.

### 2.3 The twelve packages `BACKLOG.md` does not know about

| WP | Status | Where it goes |
|---|---|---|
| **WP-82** | READY | **FIRST, before everything.** See section 1. |
| **WP-09a / WP-09b** | READY | Replaces BACKLOG's single `WP-09` slot in Phase 3. WP-09a then WP-09b. D-36: land the boundary fix **before the bulk of Phase 3** per-crate work; coordinate with WP-28. |
| **WP-81** | READY | Phase 3, **before WP-19**. Deletes the dead stats machinery (D-40). |
| **WP-83** | READY | The three released parity carve-outs: `a F1` (from WP-12), `b F7` (WP-16), `e F30` seat-order half (WP-30). Independent; land any time. `b F4` and `d F37` are **NOT** in its scope. |
| **WP-84** | READY | End of the realtime chain: after WP-42. |
| **WP-76** | READY, no spec | After **WP-51 Task 1** returns the pre-command snapshot. Five-line change. Must **NOT** fold into WP-59 or WP-40. |
| **WP-77** | READY, no spec | No sequencing constraint. |
| **WP-79** | READY, no spec | After **both WP-40 and WP-45** - collides with both in `restart_core`. |
| **WP-80** | READY, no spec | Instructed to **fold into WP-09a/WP-09b** (they own the requester-trust pattern). Its own heading is still a standalone READY. |
| **WP-78** | **SUPERSEDED by WP-82** | Skip. Never had a spec file. Its entry is retained only so `landing-order.md` 6.4's reference resolves. |
| **WP-85** | **DEFERRED-BLOCKED-ON-MICHAEL** | Skip. See section 5. |

`work-packages.md` recount, verified in-file: **85 headings = 77 READY + 6
BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED + 1 DEFERRED = 85.** The status
`BLOCKED-ON-DECISION` is **extinct** - zero packages carry it.

---

## 3. Migration numbering: FOUR packages collide on `023`

`ls rust/web/migrations/` confirmed: the highest migration actually on disk is
**`022_concede_bot_replacement.sql`** (22 files, `001`..`022`).

Four packages each add a migration and each assumes `022` is the highest:

| WP | Filename its spec hard-codes | Content |
|---|---|---|
| WP-34 | `023_login_email_sends.sql` | login-email send-rate table |
| WP-50 | `023_canonical_emails.sql` | email canonicalization |
| WP-56 | `0NN_settings_email_token.sql` ("next free") | settings email token |
| WP-58 | `0NN_unsubscribe_token.sql` ("next free") | unsubscribe token |

**WP-34 and WP-50 name the same number - a direct filename clash.**

The rule, from `landing-order.md` s6.4: *"only the package that lands FIRST may use
`023`. The second, third and fourth must each renumber to the then-next free number
(`024`, `025`, `026` in landing order) and must not collide with each other either -
**re-`ls` `rust/web/migrations/` immediately before writing the file, do not trust
the number written in the spec.** Migrations are immutable once applied: renumber
before landing, never edit an applied file."*

In the current plan WP-34 is the likeliest `023`; WP-50, WP-56 and WP-58 should all
expect to renumber.

---

## 4. Parked: do not touch

`BLOCKED-ON-USER-RULES-REVIEW` = **WP-11, WP-12, WP-16, WP-20, WP-26, WP-30.**
Confirmed as exactly these six, in three independent places in `work-packages.md`.

Game rules parity is parked pending **Michael's own review**. Under these packages
an executor must **not change gameplay** and must **not "correct" a `RULES.md`**.
Per D-35: official rules win, but **no gameplay change without per-game sign-off**;
the park lifts **per game**, not wholesale.

Three carve-outs are RELEASED from the park and are owned by **WP-83**, not by their
parent packages: `a F1` (WP-12), `b F7` (WP-16), `e F30`'s seat-order half (WP-30).

### Two resolved parity items - do NOT reopen

- **`b F4`** - seven-wonders-1 same-turn trade. **RE-PARKED.** Michael's correction
  is binding: **7 Wonders resources are NOT depleted by trade.** There is no
  competition for a resource; the "asymmetric advantage by seat" framing recorded in
  the older tables was **WRONG**. The residual - seat-order resolution letting p+1
  trade for a card p built the same turn - is a *simultaneity* question, parked and
  unscheduled.
- **`d F37`** - modern-art-2 zero-card artist placings. **REJECTED - NOT A BUG. Do
  not "fix" this later. No fix, no follow-up.** `suits()` returns the canonical
  top-to-bottom order and `end_round` scans it in declared order; current behaviour
  is the accepted way to play.

`archive/decisions-needed.md`'s old "egregious candidates" table disagrees with both.
`DECISIONS.md` rules on it: **THE RULINGS WIN.**

---

## 5. WP-85: deferred, and it blocks nothing

**WP-85** (email dispatch: game parser first, platform commands as fallback) is
`DEFERRED-BLOCKED-ON-MICHAEL` (that is the exact grep-able label in
`work-packages.md`). It is a **separate bucket from the rules park** -
`work-packages.md` states WP-85 is not a member of the six.

It is blocked on the escape-hatch verb set, which is *"undecided by choice"*.
**Its membership must NOT be invented.** The spec on disk is a sketch of a future
change, not an execution script: *"do not implement it, do not do 'just the easy
half', do not invent the missing input."*

**It blocks nothing.** The actionable-now count is 77 either way, no package waits
on it, and `landing-order.md` does not mention it at all. **An executor must not
pick it up.**

The only forward coupling runs the other way: *when* WP-85 eventually lands, WP-59's
"Known collisions" list and its "the reservation is absolute" text become false.

---

## 6. Rulings that constrain an executor

Full text in `DECISIONS.md`. These are the ones that will bite during execution.

**Do-not / scope-limiting**
- **D-03** - forbid undo once finished. **No rating rewind at all** - no
  delta-reversal, no recompute. Out of scope for WP-40.
- **D-07** - OVERRULED: build **no** redacted user-facing export. No
  `--redact-private`. Full bundle, admin-only.
- **D-14** - **sessions must NOT expire**; no session-expiry GC.
  `revoke-all-sessions` is in scope. Link-vs-code is a **non-goal** - keep the
  6-digit code; nothing changes in WP-35/WP-56.
- **D-16** - do **not** call Turnstile `render()` from an effect; `/login` is an
  unrouted full-page load. Three `use_navigate` paths need hard navigation.
- **D-20** - option B, **explicitly not a macro**. Name it `rust/lib/game_bin` /
  `brdgme_game_bin`, **not** `game-bin`. Do not "simplify" back to a macro.
- **D-24** - **accept** `combine` 4.6 as a recorded risk in `deny.toml`. Do not
  migrate markup now. (This is all of WP-72.)
- **D-37** - the escape is **`{{lbrace}}`**, not a bare `{{`. Assess stored-content
  risk **by reading code and migrations only - NEVER by querying a database.**
- **D-38** - **skip** the depth guard. Keep the parser plain over clever.
- **D-44** - **SSE now. D-13's WebSocket design is historical - do not execute it.**
  Do not re-argue "axum supports both".
- **D-45** - do **not** emit `id:`. No `Last-Event-ID` replay.
- **D-48** - **two** SSE streams. `/events/public` needs **no auth and no visibility
  predicate**. Hard cap: **never three held streams**.
- **D-49** - build **only** the two streams. No topic machinery, no multiplexing, no
  channel registry.
- **D-51** - the maximum-performance fuzzer is **out of scope** (persisted as
  `docs/BACKLOG.md` #54).
- **D-56** - WP-54 ships the error message only. The residual `/friends` desync is
  **EXPECTED**, recorded in its manual checklist, not a regression.

**Delete-something rulings (verify before deleting)**
- **D-39** - delete the dead color parse API (drops `regex` + `lazy_static`).
- **D-40** - delete the dead stats machinery (acquire-1, lost-cities-1/-2). Split
  into **WP-81**.
- **D-41** - delete the 27 `_repl` bins, **but verify first**: *"If anything does
  depend on them, **STOP and report - do not delete.**"*
- **D-43** - `_fuzz` deletion is **REVERSED**. **Keep** the 27 `_fuzz` bins as
  3-line wrappers. WP-73 ships **three** entry points.

**Process / ordering**
- **D-17** - STANDING PROCESS CHANGE binding all of WP-64..WP-73: for **any**
  dependency problem, **upgrade everything to latest FIRST**, re-assess, and only
  then apply the recorded workaround.
- **D-19** - all three workspace tables in **one** migration, early.
- **D-22** - port warp -> axum in the same window as WP-06's `http.rs` fixes.
- **D-23** - clear the 4 stale advisory ignores now; flip `multiple-versions` to
  deny **only after WP-66/67/68 land**. WP-69 lands **last**.
- **D-36** - land WP-09's boundary fix before the bulk of Phase 3; coordinate with
  WP-28.

**Verify-at-fix-time**
- **D-47** - rate-limit `/events` **connection establishment only**, never duration
  or bytes. Require a server-side heartbeat. **Verify the real Cloudflare config and
  proxy idle timeout - do not assume.**
- **D-50** - repeatable `?topic=game:<a>&topic=game:<b>`, no `[]` suffix; reject
  unknown/malformed topics with an error. **Verify axum 0.8.9's duplicate-query-key
  behaviour against crate source.**
- **D-52** - public topic cap **16**; over-cap is a **400**, not truncation.
- **`bo F25`** - pin `k8s-openapi` `v1_36`, but **confirm the feature flag exists at
  fix time**; if not, take the highest flag <= v1.36 and record it in the WP-62 spec.

**Small fixed answers**
- **D-02** - at-least-once; write the dedupe marker **only after** success; 5xx to
  force retry; do not mark `sent` on skip paths.
- **D-05** - bots stay referenced **by name**, no bot-id migration. Dangling bot
  names are a **supported** no-op plus admin warning. "All bots disabled" must not
  trip alerts.
- **D-08** - restart resolves a deprecated bot to the **latest non-deprecated**
  version. The no-op fallback is **not** the answer for restart.
- **D-11** - the reminder sweep must **NOT** consult `turn_emails_enabled`.
- **D-18** - **no Sentry functionality may be lost**; verify behaviour preservation,
  preserve native-tls.
- **D-25** - gates only 3 of WP-17's 8 findings. `lib/cost` **must** gain its own
  automated tests as part of the port.
- **N-1** - WP-38 ships a 15-minute sweep threshold and 60s `AckKind::Progress`.
- **N-2** - WP-10 uses `PubState::cup_counts: Vec<(Colour, usize)>`.
- **N-3** - `game_types.player_counts` is newest-non-deprecated-version-wins, not a
  union.
- **N-4** - an amendment must **state its rationale**; `BASIC_STRATEGY.md` and
  `ADVANCED_STRATEGY.md` **must not be folded into `RULES.md`**.
- **N-5** - **re-read the live `docs/BACKLOG.md`** before applying item #53; do not
  fold it into #37.
- **N-6** - apply the 6-rule `## Request-Path Invariants` section as drafted,
  between `## Rust: Error Handling` and `## Leptos: SSR and Hydration`.

### Eight decisions have NO ruling. Do not invent one.

**D-26, D-27, D-28, D-29, D-30, D-31, D-32, D-34** are all
`PARKED-PENDING-USER-RULES-REVIEW` - they carry a park plus recommendations written
under a **superseded** policy. The recommendations are not rulings.

Three of the eight are **partial** - a single finding was released while the rest
stays parked:

- **D-27** - `b F7` is FIX NOW; `b F4` is PARKED. F5/F6/F8 stay parked.
- **D-29** - `e F30`'s seat-order half is FIX NOW. The "can an empty winning set win
  at all" half stays parked, no ruling.
- **D-34** - `a F1` is FIX NOW. F7/F9 and the quirk-preservation policy stay parked,
  no ruling.

Fully unruled: **D-26, D-28, D-30, D-31, D-32.**

Also unruled in-section but resolved elsewhere: **D-46**, superseded by **D-48**
(two streams) - use D-48. Ruled but with an open sub-part: **D-43** (the
maximum-throughput fuzzer, 4(d), needs Michael) and **D-49** (whether public topics
eventually share one connection - deliberately not decided).

---

## 7. Citation risk: navigate by symbol, never by line number

**Line numbers across this entire corpus are approximate and were measured 33-46%
WRONG. Two delete ranges, had they been followed, would have destroyed live code.**

`DECISIONS.md` says of its own citations: *"approximate, verify."*

Rules:

1. **Navigate by named function, type or symbol.** Never trust a line number, a
   line range, or a "delete lines N-M" instruction.
2. **If the code does not match the spec's description: STOP AND REPORT.** Do not
   improvise a fix. Do not guess the intended target. Do not "fix it while you're in
   there".
3. Spec length is not a measure of the work. **Many specs are pre-tiering and
   bloated - a 1000-line spec may describe a three-line fix.** Read for the change,
   not for the volume.

Six specs carry a citation-risk banner and are the **highest risk for a cheap
executor**: **WP-51, WP-59, WP-28, WP-19, WP-23, WP-54.**

Eight specs are KEEP-with-a-known-defect: **WP-19, WP-29, WP-40, WP-45, WP-51,
WP-59, WP-62, WP-68.**

---

## 8. Coverage gaps - deliberate and documented

**WP-76, WP-77, WP-79, WP-80 have no spec file.** This is a **deliberate,
documented gap**: Michael chose not to spec them now. They are described in
`work-packages.md` under `## Unowned / newly discovered`, each with enough detail to
execute or to defer.

**None of the four is on the critical path.** Verified: none appears in any ordering
diagram in `landing-order.md`, and none is named as a predecessor of anything. WP-76
appears there only in a negative note (*"WP-57 and WP-76: checked, no collision"*);
WP-77, WP-79 and WP-80 are not mentioned at all. Their own dependencies are in
section 2.3.

**WP-80** was intended to fold into WP-09a/WP-09b, which own the requester-trust
pattern. Its `work-packages.md` heading nonetheless still reads READY as a
standalone entry - treat the fold as the intent.

Two further absences are by design, not gaps:

- **WP-72** has no spec file of its own. Its content is a section of
  `specs/WP-69-deny-toml-hardening.md`. **Do not look for a `WP-72-*.md`.** Do not
  delete or archive WP-69 - that would silently delete WP-72 too.
- **WP-78** is **SUPERSEDED by WP-82** and never had a spec file.

`specs/` holds 48 files: 47 `WP-*.md` plus `notes-conventions.md` (not a spec). The
13 landed specs are no longer among them - they are in `specs/archive/` (section 9),
which brings the corpus total to 60 WP specs.
`specs-CLASSIFICATION.md`'s 60 verdict rows cover 59 WP specs plus
`notes-conventions.md`. The one WP spec it does **not** cover is
`WP-85-email-parser-first-dispatch.md`, written after the classification was built.
WP-72 has no row because it has no file. **No spec file is missing.**

---

## 9. The 13 ARCHIVE specs are archived and verified - do not act on them

`specs-CLASSIFICATION.md` marked 13 specs **ARCHIVE**, meaning "already landed":

**WP-01, WP-03, WP-06, WP-13, WP-14, WP-15, WP-21, WP-25, WP-36, WP-37, WP-39,
WP-41, WP-44.**

Count verified: exactly 13, two independent ways. Buckets are **47 KEEP / 13 ARCHIVE
/ 0 UNCERTAIN** over 60 verdict rows.

All 13 have been **moved to `planning/specs/archive/`**. They are there for
**provenance only. The executor must NOT act on them** - they are not in the working
set and no package waits on them.

**The verification is COMPLETE - do not redo it.** The original verdicts were read
off a worktree that still held uncommitted work, so "landed" could have meant
"landed but uncommitted, possibly partial". Every one of the 13 was therefore
re-checked task-by-task against clean committed `master`: the result was **13
CONFIRMED-LANDED, 0 NOT-LANDED**. Per-spec evidence and landing commits are in
`specs-CLASSIFICATION.md` under the section
`## ARCHIVE re-verification against clean committed master (2026-07-27)`.

Two things bloat was **never** grounds for: archiving a spec for being long *"would
have left its package with no spec at all"*, and no spec was archived for
supersession - every supersession found is section-internal and is fixed by amending
the section (WP-42 s3a by WP-84, WP-19's `c F11` by WP-81, WP-59 Task 14 by D-15,
WP-40's rewind note by D-3).

---

## 10. The uncommitted-work rule

**Survey the working tree yourself before you start. Do not trust any snapshot in
this document or any other.**

At the time this file was written, `git status --short -- rust/` and
`git diff --stat -- rust/` were both **empty** - the `rust/` tree was clean on
`master`, and the recent history was dense with landed `WP-*` fix commits. All
uncommitted change was inside `planning/`.

That will have drifted. Concretely:

1. Run `git status --short` and `git log --oneline -30` first.
2. If there is uncommitted work under `rust/`, **read it before writing anything**.
   Work out which package it belongs to and whether that package is complete.
3. Find your continuation point from what the tree and the log actually show - not
   from a status line in a planning document. `work-packages.md` status lines and
   the retired `archive/specs-LOG.md` entries have both proved repeatedly stale.
4. **Do not revert, stash, or rewrite someone else's in-progress work.** If you
   cannot tell whether a package is finished, **stop and report**.

`landing-order.md`'s own claim that *"`git log --oneline -40` contains zero `WP-*`
commits"* is already **stale** - do not rely on it.

---

## 11. Working discipline

- **One package at a time**, in the order from section 2.
- Each package gets **its own verification** (its spec names it) and **its own
  commit**. **Defer all pushes** to a final pass.
- Read the spec, then read the live code it names, **then** decide. If they
  disagree - **stop and report**.
- Never change gameplay and never "correct" a `RULES.md` under a parked package.
- Migrations: re-`ls` `rust/web/migrations/` immediately before writing one.
- Stopping to report is the **expected and correct** outcome when reality does not
  match a spec. It is not a failure.
