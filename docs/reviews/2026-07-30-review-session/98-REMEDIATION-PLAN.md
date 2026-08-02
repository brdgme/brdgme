# Remediation work breakdown

Derived from `99-UNIFIED-REPORT.md` (211 findings, F-01..F-211) against the
127-commit remediation effort of 2026-07-25..2026-07-30.

## Execution status

This directory is committed; execution state, per-package blockers, and owner
decisions live in `97-REMEDIATION-PROGRESS.md` - start there. Package-number
order is not execution order when blockers exist; the tracker's status column
decides what is next.

- **R-49 is done** (see 97 for commits), as are R-47 and R-48. R-45 is
  `partial/parked` on the unpinned `blocked_domains.rs` provenance record; see
  97. R-37 and R-43 are parked by user process; R-38 remains blocked on 5.3.
  R-46 is `partial/parked` after R-46.0; R-51 is parked for the later
  parked-item review pending two scope rulings; see 97.
- **R-35 is blocked on a user decision:** removing `Status` leaves no approved
  public points source - `Response::Status` is the only response carrying
  `GameResponse.points`, and `Response::PlayerRender` carries no `points`. The
  owner must either approve optional all-seat `points` in `Response::PlayerRender`
  (populated by Rust handlers, absent for existing Go handlers) or specify
  another Go-compatible source. See Pending User Decisions in 97.

## 1. How to use this document

- Every work package (`R-NN`) closes a named set of findings, cited by **F-number
  against `99-UNIFIED-REPORT.md` section 11**, which is the canonical findings
  table. Section 11 is the only authority on a finding's severity and wording;
  this document never restates a severity that section 11 does not carry.
- Finding detail - the evidence, file:line citations and the reasoning - lives in
  `90-findings-part1.md`, `90-findings-part2.md` and `90-findings-part3.md`.
  Read those before starting a work package, not the 22 individual unit reports.
- **Size the work from section 11 and the unit reports' file lists. Never size
  from `00-breakdown.md`.** That file's premises were wrong four separate times
  during this review and every error inflated the apparent work (the clearest
  case: its Unit 06 "shared-core extraction" gotcha, where `9ba3736b` turned out
  to touch zero `rust/game/*` files at all). It is a historical planning artefact,
  not a sizing input.
- Sizes are **S** (under a day, one file or one function), **M** (a few days, one
  crate or one contract across a handful of files) and **L** (a week or more,
  cross-crate or requiring a new test harness). Each size states its basis. Where
  the evidence does not support a confident number, the package says so rather
  than guessing.
- **Acceptance criteria are written to defeat the decoys this review found.**
  Every criterion that asks for a test names the function that test must *call*
  (F-151 and F-161d were both tests that name-matched their risk and never
  invoked the code under test). Every criterion that closes a finding by citation
  asserts the citation still **exists** (F-109) and is **reachable** (F-147).
- Do not re-open anything in section 7 of this document.

## 2. Ordered work packages

Ordering is **F-161 first**, then the remaining Highs, then Medium, then Low.
Within each band, packages that unblock others come first.

Section 11 records **no Critical rows**. The severity ceiling of this plan is
High, with two Mediums (F-129, F-130) that 00-STATE escalates to **account
takeover** and which therefore lead the order alongside F-161.

---

### R-01 - Close the inbound-email authentication gate

**Objective:** make `classify_inbound_auth` fail closed, so a forged `From:`
header cannot authenticate an inbound command.

- **Closes:** F-161 (High) including sub-items F-161a, F-161b, F-161c, F-161d.
- **Files:** `rust/web/src/email/inbound.rs:164-219` (+4 sites).
- **Size: M** - basis: one classification function plus four call sites, and two
  existing tests must be re-fixtured rather than added to.
- **Depends on:** nothing. **Blocks:** R-02 (they ship together).
- **Why first:** WP-56's gate is fail-open **three independent ways**. The
  cleanest is `spf=fail; dkim=none` -> `Pass`, because the code requires SPF
  *and* DKIM to both say "fail", inverting the DMARC rule. Combined with R-02's
  token weaknesses this is **account takeover** - the session's most severe
  finding. Unit 07 set this escalation condition explicitly and it fired.

**Acceptance criteria**

1. A test named for the DMARC rule **calls `classify_inbound_auth` directly** and
   asserts `spf=fail; dkim=none` classifies as **not** `Pass`. Asserting on a
   route handler that happens to reject for another reason does not satisfy this.
2. F-161a: a test calls `classify_inbound_auth` with an `Unknown` verdict and
   asserts the request is **rejected**, not warned-and-continued.
3. F-161c: a test calls `classify_inbound_auth` with an attacker-supplied sole
   authentication header and asserts it is **not** honoured.
4. F-161d: the two existing decoy tests
   `classify_inbound_auth_softfail_is_not_fail` and
   `..._single_fail_is_not_fail` are **re-fixtured** so their inputs contain no
   independently passing result. A reviewer must confirm the new fixtures fail
   against the pre-fix code - a test that passes before and after the fix is not
   a regression test.
5. `rg` confirms every citation in F-161's row still exists at the cited path and
   has at least one caller outside its own test module.

---

### R-02 - Scope and expire the settings-email token

**Objective:** stop the settings token being a permanent, unscoped bearer
credential.

- **Closes:** F-129 (**escalated to account takeover**), F-130 (**escalated to
  account takeover**).
- **Files:** `rust/web/src/email/inbound.rs:520-530` (+2 sites),
  `rust/web/src/email/commands.rs:329-346` (+2 sites).
- **Size: M** - basis: a token table change (expiry, single-use, revocation) plus
  a command-scope check at three call sites.
- **Depends on:** R-01. **Must ship in the same release as R-01** - each is a
  mitigating control for the other, and shipping either alone leaves the takeover
  path open.
- **Note:** WP-60 is **not** a precedent here. Unit 09c refuted that: WP-60's
  outbound tokens have no expiry, single-use or rate limit either, so F-161's
  substance is untouched by WP-60. Do not cite it as prior art.

**Acceptance criteria**

1. A test **calls `find_user_by_settings_token`** with an expired token and
   asserts rejection; a second calls it twice with the same token and asserts the
   second call fails (single-use).
2. A test **calls the command dispatcher** with a settings token and each of
   `new`, `bump` and the subscribe verbs, asserting each is refused. F-130 is
   precisely that the token is not scoped to settings.
3. A rate limit exists on the settings-token path, or the package records
   explicitly that it depends on R-37 (there is currently **no rate-limiting
   middleware anywhere in `rust/web`** - F-94 - and two doc comments falsely
   assert a per-IP limit; do not trust them).

---

### R-03 - Canonicalize bot names at the validation boundary

**Objective:** one defect, four views. Make `validate_bot_slots` canonicalize and
**return the canonical name**, so a bot seated by email actually takes its turn.

- **Closes:** F-104 (High), F-138 (Medium), F-183 (High), F-189 (High), and
  re-fixtures the decoy test F-185 (Low) **in the same change**.
- **Files:** `rust/web/src/db/bots.rs:57-71` and `:61-63` (+5 sites),
  `rust/web/src/email/commands.rs:82-93` (written at `:398-401`),
  `rust/bot/src/config.rs:26-29` **and `:67`** (the second site was never cited
  before F-189), `rust/bot/src/main.rs:186-194` (and `:188-193`),
  `rust/web/src/admin.rs:293-303`, `rust/web/src/email/commands.rs:1435-1455`
  (the decoy fixture).
- **Size: M** - basis: one function's signature change rippling to five write
  paths and two bot-side read paths. **Do not split this.**
- **Depends on:** nothing. **Blocks:** R-04.

**The defect, end to end:** the email `new` command lowercases the bot name;
`validate_bot_slots` accepts it via `eq_ignore_ascii_case` and returns nothing
canonical; the bot service looks it up **case-sensitively** and silently skips.
The game is created, the bot is seated, and it **never takes a turn** - no error,
no retry. F-189 sharpens it: `main.rs:186-194` returns `Ok(())`, which **acks and
discards** the turn, whereas the sibling "no providers" path returns `Err` and is
retried - so the wrong-case path is the one that fails silently.
Precondition: `admin::create_bot` permits arbitrary casing.

**Acceptance criteria**

1. `validate_bot_slots` returns the canonical bot name, and **all five** write
   paths persist the returned value rather than the caller's input. A reviewer
   greps for every caller and confirms none discards the return.
2. F-185's fixture is re-fixtured to mixed case. The existing
   `validate_bot_slots_accepts_case_mismatch` must be **rewritten, not extended** -
   as written it pins the lenient half of the inconsistency as intended behaviour
   (pattern 4f), so leaving it in place re-asserts the bug.
3. A test **calls the bot-side lookup** in `rust/bot/src/config.rs` with a
   differently-cased name and asserts a hit, covering **both** `:26-29` and `:67`.
4. A test **calls `rust/bot/src/main.rs`'s turn handler** on a lookup miss and
   asserts it returns `Err` (retried), not `Ok(())` (acked and discarded).
5. An end-to-end test creates a game via the email `new` command with a
   mixed-case bot name and asserts the bot takes a turn.

---

### R-04 - `restart_core` must validate bot slots

**Objective:** close the one game-creation path that bypasses bot-slot
validation entirely.

- **Closes:** F-137 (High).
- **Files:** `rust/web/src/game/server_fns.rs:1087`.
- **Size: S** - basis: one missing call.
- **Depends on:** R-03 (it must call the canonicalizing version, not the old one).

**Acceptance criteria**

1. A test **calls `restart_core`** with a garbage bot name and asserts an error,
   not a created game.
2. A reviewer greps every game-creation entry point and confirms each calls
   `validate_bot_slots`; the hit count is recorded in the commit message
   (pattern-2 mechanism, section 4.9).

---

### R-05 - Concede-and-replace transaction integrity

**Objective:** make `concede_game_replace` atomic, idempotent and non-wedging.

- **Closes:** F-111 (High), F-112 (High), F-119 (High), F-115 (Medium),
  F-114 (Medium).
- **Files:** `rust/web/src/db/game_write.rs:387-426`, `:394-399` (+2 sites),
  `:401-410` (+2 sites), `:348-355` (+1 site), `:1816-2282` (the tests).
- **Size: L** - basis: three interacting defects in one function plus a
  transaction-boundary change and three tests whose dropped assertions must be
  restored.
- **Depends on:** nothing. **Related:** `db::pick_replacement_bot`
  (`rust/web/src/db/bots.rs:76-98`) takes `&PgPool`, **not** a transaction, and
  does SELECT-then-INSERT as two autocommit statements - **it cannot be made
  atomic with a caller's `game_players` update as it stands.** Changing its
  signature is part of this package, not a follow-up.

**The three defects:** F-111 picks the replacement bot **before** `begin()`,
committing an orphan `game_bots` row. F-112 never updates the `games` row, so a
duplicated concede replays - second bot, second public log. F-119 clears
`is_turn` without reassigning it, wedging the game so the bot never plays.
F-115 is the mirror: a human-count mismatch makes concede **permanently
impossible**.

**Acceptance criteria**

1. A test **calls `concede_game_replace`** twice with the same input and asserts
   exactly one `game_bots` row and one public log (F-112).
2. A test asserts that a failure after bot selection leaves **no** `game_bots`
   row (F-111) - i.e. selection is inside the transaction.
3. A test **calls `concede_game_replace`** and then asserts `is_turn` is set on
   the replacement bot (F-119). This must assert the post-state, not merely that
   the call returned `Ok`.
4. F-114: the three guard tests at `:1816-2282` restore the **state assertions**
   the spec prescribed. A reviewer diffs each test against the spec row and
   confirms the assertion set matches - this is a pattern-4b remediation, so
   moving the spec to the test is not acceptable.
5. **Cross-check against F-119's mitigation, which Unit 06 left open:** confirm
   WP-38's bot-turn wedge-recovery sweep re-derives turn ownership from the game
   service rather than gating on `is_turn`. If it gates on `is_turn`, **F-119 has
   no production mitigation** and this package is the only control.

---

### R-06 - One `left_at` / lifecycle-writer change

**Objective:** a single schema-and-guard change to elimination and departure
state - **not three separate fixes**.

- **Closes:** F-113 (Medium), F-116 (High), F-117 (Medium). Also folded in
  because they live on the same lines and the same lifecycle-writer set:
  F-118 (Low), F-120 (Medium), F-148 (Medium).
- **Files:** `rust/web/src/db/game_write.rs:584-598` (+2 sites), `:356-381`
  (+2 sites), `:430-473` (+2 sites), `:739`,
  `rust/web/src/game/server_fns.rs:945-947` (+1 site).
- **Size: L** - basis: a schema change plus four writers plus a `docs/CODING.md`
  rule that must be rewritten (see below).
- **Depends on:** nothing. Overlaps R-05's file; sequence R-05 first to avoid a
  merge conflict in `game_write.rs`.

**Why one package:** F-116 is that `undo_game`'s `left_at` CASE has **no
un-elimination arm**, so an undone elimination permanently rates the player last.
F-113 is the same state enforced only against a **pool snapshot**. F-117 is that
conceded games never get `ranked_placing` written. All three are the same
`left_at`/placing state machine viewed from three writers. F-116 is also the
clean instance of **pattern 2**: WP-40 added `AND NOT $9` to the `left_at` CASE
in `update_game_command_success` and left the **byte-identical sibling** in
`undo_game` alone.

**Acceptance criteria**

1. A test **calls `undo_game`** on an elimination and asserts `left_at` is
   cleared and the player is no longer rated last (F-116), and that `points` is
   not left stale (F-118).
2. A test **calls the "already left" check** with a concurrent departure and
   asserts it is enforced transactionally, not against a snapshot (F-113).
3. A test **calls the concede path** and asserts `ranked_placing` is written
   (F-117).
4. F-148: the elimination guard at `:739` gets a test that **calls the guard's
   enclosing function**; its checklist row is marked `Test? y` and currently has
   no test at all.
5. **F-120 and the doc rule.** `end_game` writes ratings with no claim or
   `expected_updated_at`. The `docs/CODING.md` rule WP-40 added names **three**
   functions, which makes `end_game` - a fourth unguarded lifecycle writer that
   rates the game - invisible to the grep procedure the doc itself prescribes.
   The rule must be rewritten to describe the **property** (any writer that
   rates or finalises a game) and the reviewer must run the prescribed grep and
   record the hit count. Enumerate the writers found; there must be four.

---

### R-07 - `CanonicalEmail` newtype

**Objective:** make it impossible to hold a non-canonical email, closing the
F-124/F-127 class permanently rather than one site at a time.

- **Closes:** F-128 (Medium - **00-STATE wins the conflict here: it is NOT
  closed and has no owner**), F-173 (Low), F-124 (High), F-127 (Low),
  F-125 (Medium), F-126 (Medium).
- **Files:** `rust/web/src/auth/email_addr.rs` (`canonicalize_email`, WP-50),
  `rust/web/src/email/inbound.rs:538` (+3 sites) and `:532-545`,
  `rust/web/src/db/game_write.rs:81-115`,
  `rust/web/src/proposals.rs:1730-1786` and `:1733 -> :1772 -> :1191`,
  `rust/web/migrations/026_canonical_emails.sql:33`.
- **Size: L** - basis: a new type threaded through every auth entry path, plus a
  migration correction and a backfill reconciliation.
- **Depends on:** nothing. High-value: `canonicalize_email` now runs on **every
  auth entry path before uniqueness checks**, so a canonicalization bug is
  account takeover, not a formatting nit.

**The defect:** the canonicalization contract is currently enforced **only by a
doc comment**. `from_matches_verified_email` compares in **SQL** (`LOWER`) while
every write path canonicalizes in **Rust** - `İ@example.com` breaks the
equivalence (F-128, F-173). F-124 is the consequence at the proposal boundary:
`add_proposal_player` passes raw email through, minting **verified ghost
accounts** and bypassing invite policy. F-126 is `email: Some("")` inserting a
junk verified account and then 500ing. F-125 is the migration's index on
`lower(email)` against a backfill using `lower(btrim())`, so trimmed duplicates
survive.

**Acceptance criteria**

1. A `CanonicalEmail` newtype exists whose **only** constructor is
   `canonicalize_email`. Every DB column holding an email is typed as it. A
   reviewer greps for raw `String` email parameters on write paths; the count
   must be zero.
2. A test **calls `from_matches_verified_email`** with `İ@example.com` and
   asserts the Rust and SQL paths agree. Comparison must move out of SQL, or the
   SQL must call the same canonicalization.
3. A test **calls `add_proposal_player`** with a raw, non-canonical, and an empty
   email, asserting no verified account is created in any case (F-124, F-126).
4. F-125: a migration test asserts the unique index and the backfill use the
   **same** expression. Re-run the backfill reconciliation and record the number
   of trim-duplicate rows found.
5. F-127's doc comment is replaced by the type; the comment must be **deleted**,
   not kept alongside, so nobody re-derives the contract from prose.

---

### R-08 - Transient errors must not be classified permanent

**Objective:** one fix for the surviving duplicate of the abandoned `wfe F36`
dedup and the catch-all that mis-classifies transient failures.

- **Closes:** F-136 (High), F-145 (Medium).
- **Files:** `rust/web/src/email/sweep.rs:135-137`,
  `rust/web/src/proposals.rs:257-296` (+1 site).
- **Size: S** - basis: two error-classification sites. **These must be one
  change**: the surviving duplicate of the abandoned dedup is where F-136 lives.
- **Depends on:** nothing.

**The defect:** F-136 is **pattern 5 at High severity in the web half** - a
`_ =>` catch-all turns transient errors into `PermanentSkip`, marking a reminder
sent that never sends. F-145 is the same shape: `Err(_)` folded to "permanently
unsendable", marking nudged and sending nothing.

**Acceptance criteria**

1. Neither site has a `_ =>` arm. The `match` is exhaustive over named variants
   (section 4.9's pattern-5 mechanism: an exhaustiveness row is only satisfied by
   a `match` with no `_` arm).
2. A test **calls the sweep's classifier** with a transient DB error and asserts
   the reminder is **not** marked sent.
3. A test **calls the proposals nudge path** with a transient send error and
   asserts the nudge is **not** marked delivered.

---

### R-09 - One `RouteOutcome` contract

**Objective:** the at-least-once `Retry` contract must hold on **every** inbound
route, not the ones WP-79 remembered.

- **Closes:** F-162 (Medium), F-169 (High).
- **Files:** `rust/web/src/email/inbound.rs:992-1060` (+7 sites) and
  `:1392-1433`.
- **Size: M** - basis: eight return sites across two routes, one shared contract.
  **One package** - same contract, invite route and settings route.
- **Depends on:** nothing.
- **Settled, do not re-open:** Unit 09c re-read **every** return in lines
  654-1405. The only `Done`-on-transient sites are F-162 and F-169. **There is no
  third route with this defect** - do not sweep again.

**Acceptance criteria**

1. A single named contract (a helper or a typed error mapping) decides
   `Done` vs `Retry`, and both routes call it. A reviewer greps for literal
   `RouteOutcome::Done` constructions; each remaining one must be justified in a
   comment naming the non-transient condition.
2. A test **calls the invite route** with a transient DB error and asserts
   `Retry` (F-162 - seven transient failures currently return `Done` and invite
   responses are lost silently).
3. A test **calls the settings route** with a transient DB error and asserts
   `Retry` (F-169).

---

### R-10 - SSE authorization lifetime and task hygiene

**Objective:** an SSE stream must stop when the session does, and must not leak
tasks or amplify anonymous DB load.

- **Closes:** F-158 (High), F-159 (Medium), F-160 (Medium), F-131
  (Low/Medium - routed to Unit 09 and concretised there as F-158), F-163 (Low).
- **Files:** `rust/web/src/events.rs:33-41` (+1 site), `:47-112` (+1 site),
  `:117-183` (+2 sites), `rust/web/tests/sse_events.rs:456-457` (+1 site).
- **Size: M** - basis: one module, three defects, plus an `#[ignore]` to remove.
- **Depends on:** nothing.
- **Refuted, do not re-derive:** the `VisibilityCache` cross-user leak. Each
  instance is a local inside the per-request spawn at `events.rs:65`.

**The defects:** SSE validates the session **once at connect**, so a revoked
session keeps streaming private events (F-158). Tasks exit only on send failure,
so idle viewers leak tasks and subscriptions (F-159). `events_public_handler` is
unauthenticated, subscribes every connection to `game.>`, skips the visibility
cache and runs an **uncached** `is_game_publicly_visible` query per matching
message while the authenticated handler beside it uses `VisibilityCache` -
anonymous DB amplification, pattern 2, and no rate limiting anywhere (F-160,
F-94).

**Acceptance criteria**

1. A test **calls the SSE handler**, revokes the session mid-stream, and asserts
   the stream terminates (F-158). Re-validation must be periodic or
   event-driven, not connect-only.
2. A test asserts an idle connection's task and subscription are dropped within a
   bounded time (F-159).
3. `events_public_handler` uses `VisibilityCache` and does not subscribe to
   `game.>` unfiltered (F-160). A reviewer diffs it against the authenticated
   handler and records the remaining differences with a justification for each.
4. F-163: the timeout regression test's `#[ignore]` is removed and the test runs
   in CI. If it is flaky, fix the flake - an `#[ignore]`d regression test is a
   citation that is present but **not reachable** (tooth 2).

---

### R-11 - Shutdown drain: bookkeeping, then a decision

**Objective:** correct the record on ws F55 and decide its never-implemented
second half. **This is not a revert.**

- **Closes:** F-109 (High) - as a bookkeeping fix, plus an owner decision.
- **Files:** `rust/web/src/websocket.rs:78-80` (+2 sites), WP-36's checklist row,
  and the bot-consumer / email-sweep task startup.
- **Size: S** for the bookkeeping; the second half is **unsized** pending the
  owner decision in 6.3 - say so rather than guessing.
- **Depends on:** nothing.

**Settled, do not re-derive:** Unit 09a enumerated **all 12 files** touched by
`efad81f9` and demonstrated it contains **exactly one** pattern-4e instance
(F-109). WP-84's spec §3g **anticipated** the deletion and required a proof test,
which does exist. Therefore F-109's remediation is a bookkeeping fix on WP-36's
row plus a decision on the never-implemented second half of ws F55 - **NOT a
revert of `efad81f9`**. Also settled: WP-42 was **not** reverted by the SSE
migration, and `ca7925bc` is not a 4e revert either (`+20/-0`, deletes nothing).

**Acceptance criteria**

1. WP-36's checklist row is amended to record that its fix and its test
   (`rust/web/tests/websocket_hygiene.rs`) were **deleted by `efad81f`**, with
   the WP-84 §3g proof test named as the successor. This is a tooth-4 amendment:
   closing the row silently is what produced F-109.
2. The second half of ws F55 - **bot consumer and email sweep tasks get no
   shutdown signal** - is either implemented with a test that **calls each task's
   shutdown path**, or recorded as an owner-accepted gap with a dated ruling.
   It may not simply remain unmentioned.
3. Detached SSE spawns are bounded (the concrete harm F-109 cites).

---

### R-12 - `logout_everywhere` must not report success on failure

**Objective:** a session-lookup error must not read as "all tokens revoked".

- **Closes:** F-85 (High).
- **Files:** `rust/web/src/auth/server.rs:590-612` (+1 site).
- **Size: S** - basis: one error path collapsing to `None`.
- **Depends on:** nothing.

**Acceptance criteria**

1. A test **calls `logout_everywhere`** with a failing session store and asserts
   an `Err`, not `Ok(true)`.
2. A test calls it on a healthy store and asserts the auth-token rows are
   actually deleted - the current bug is that it returns `Ok(true)` having
   deleted **no** rows.
3. Related but separate: F-86 (session-store errors swallowed at
   `auth/session.rs:68-74`, so a transient blip de-authenticates the user) is in
   R-37; both are the same "error collapsed to a benign value" shape and should
   be reviewed together even though they ship separately.

---

### R-13 - Bot crypto: remove the ungated dev-key fallback and end the divergence

**Objective:** `rust/bot/src/crypto.rs` must stop being an unhardened copy of
`rust/web/src/crypto.rs`.

- **Closes:** F-186 (High), F-90 (Medium), F-187 (Medium), and the unnumbered
  05a row (`rust/bot/src/crypto.rs:66-76` dev-key fallback, routed to Unit 10
  with F-90).
- **Files:** `rust/bot/src/crypto.rs:59-76` and `:66-70` (+1 site),
  `rust/web/src/crypto.rs:56-75` (the house pattern to copy).
- **Size: M** - basis: four divergence axes plus tests that currently **pin the
  old behaviour** and must be rewritten, not extended.
- **Depends on:** nothing.

**The defect:** the bot silently falls back to the hardcoded dev encryption key
**ungated in any environment** - a real `docs/CODING.md` violation
(`docs/CODING.md:701` explicitly forbids the dev-default pattern). The house
pattern is *panic unless an explicit opt-in flag is set*, implemented correctly
in `rust/web/src/crypto.rs:56-75`. Fixes landed only in the web copy.

**Acceptance criteria**

1. The bot adopts the web crypto module - preferably by extracting a shared crate
   so the divergence cannot recur. If it stays duplicated, the commit message
   must say why, and a test must assert the two implementations agree.
2. A test **calls the bot's key loader** with no key set and asserts it panics
   unless the opt-in flag is set.
3. The tests that pin the old bot behaviour are **deleted or inverted**, not left
   passing. A reviewer confirms each removed assertion (pattern 4b).
4. F-187's four divergence axes are enumerated in the commit message with a
   per-axis resolution.

---

### R-14 - Share the NATS wire protocol

**Objective:** the two `Bot*Event` structs are a wire protocol maintained as two
copy-pasted files with no round-trip test.

- **Closes:** F-108 (Medium), F-188 (Medium).
- **Files:** `rust/bot/src/nats.rs:1-36` (+2 sites), `rust/web/src/nats.rs`.
- **Size: M** - basis: a new shared crate plus two consumers.
- **Depends on:** nothing.
- **Settled:** the duplicated-module sweep is **done** (Unit 05b). Exactly one
  further duplicate exists beyond crypto - this one - and it has **not yet
  diverged**. `bot/config.rs` vs `web/config.rs` share only a filename. Do not
  re-run the sweep.

**Acceptance criteria**

1. The wire types live in one crate; both `rust/bot` and `rust/web` depend on it.
2. A round-trip test **serialises with one crate's types and deserialises with
   the other's** - or, once shared, asserts the wire format against a golden
   fixture so a future divergence is loud.
3. The duplicated constants (F-188) are gone; a reviewer greps for the constant
   names and confirms one definition each.

---

### R-15 - NATS delivery semantics

**Objective:** one turn must produce one LLM completion, and a config drift must
not be silently tolerated.

- **Closes:** F-101 (Medium), F-102 (Medium), F-105 (Medium), F-107 (Low).
- **Files:** `rust/web/src/game/mod.rs:329-355` (+1 site) and `:200-255`
  (+2 sites), `rust/web/src/nats.rs:121-179` and `:21-25` (+2 sites).
- **Size: M** - basis: a dedup window plus a durable-reconciliation decision.
- **Depends on:** R-14 (do the shared crate first so the constants move once).

**The defects:** no `Nats-Msg-Id` or dedup window means one turn produces **four
LLM completions** (F-105 - a direct cost and a duplicate-move risk). An unacked
message stalls the bot turn for the full **5-minute** `ack_wait` (F-101). Config
drift **only warns**, so deployed durables keep pre-fix settings indefinitely
(F-102). F-107 is cosmetic but misleading: comments call shipped work "(future)".

**Acceptance criteria**

1. A test **publishes the same turn twice** and asserts one completion.
2. A test asserts an unacked message is redelivered well inside 5 minutes, or the
   `ack_wait` is documented as deliberate with the reasoning recorded.
3. Config drift is either reconciled automatically or fails startup. A warning is
   not a resolution - this is the same shape as 4.6 (a stop condition answered
   with a comment).
4. F-107: the "(future)" comments are deleted. A reviewer greps `rg '\(future\)'`
   in `rust/web/src/nats.rs` and confirms zero hits.

---

### R-16 - CI guard for the hand-maintained delivery lists (`R-DEL`)

**Objective:** make it impossible to add a game crate that is built but never
shipped.

- **Closes:** F-208 (High). Related deployment items in section 3: F-211 and the
  `hanamikoji-1` Dockerfile stage.
- **Files:** `rust/Cargo.toml` (workspace members - 28 game crates),
  `rust/Dockerfile:36` and `:174-303` (**26** game stages), `docker-bake.hcl`,
  `k8s/base/game/`, `rust/web/end2end/tests/page-loads.spec.ts:8` (+1 site).
- **Size: M** - basis: one CI script plus the missing `hanamikoji-1` stage, bake
  target and Deployment.
- **Depends on:** nothing. **This is the mechanism described in section 4.7.**
- **Scope note:** **F-208a is refuted** - drop the 43-vs-26 framing entirely; it
  was a carried premise, not a defect. The guard should still assert set equality
  across the four lists, but do not treat the raw count difference as the finding.

**Acceptance criteria**

1. `hanamikoji-1` has a Dockerfile stage, a bake target and a k8s Deployment. It
   is currently built by `cargo build --release --workspace --exclude web` and
   **never copied into an image** - built and unshippable.
2. A CI script derives the game crate list from `rust/Cargo.toml` and asserts set
   equality against the Dockerfile stages, the bake targets and the k8s
   Deployments. It **fails the build**; a warning does not satisfy this.
3. The allow-list of intentional absentees contains exactly `lords-of-vegas-1`
   (WIP, owner-excluded) and is commented with the reason and a review date.
4. F-211: the end2end smoke assertion that was weakened to a fallback string is
   restored to assert the real content, and a reviewer confirms it fails against
   a broken deploy.

---

### R-17 - Stats query correctness

**Objective:** the stats surface returns the numbers it claims to.

- **Closes:** F-151 (High), F-152 (Medium), F-153 (Medium), F-154 (Medium),
  F-155 (Low), F-156 (Medium), F-150 (Medium - all seven WP-52 `Test? y` rows).
- **Files:** `rust/web/src/stats/queries.rs:104-152` (+1 site), `:713`, `:7-20`,
  `:511` (+2 sites), `rust/web/src/stats/mod.rs:343-348`,
  `rust/web/src/index.rs:47-73`,
  `docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B5-web-domain-stats-misc.md`.
- **Size: M** - basis: five query fixes in one module plus seven missing tests.
- **Depends on:** 5.4 (the request-parts harness) for the server-fn level tests.

**The defects:** F-151 applies the game-type filter to **only one side of a FULL
OUTER JOIN**, returning **another game type's rating**. F-154's unknown game-type
filter binds NULL and returns the **entire history**. F-152's `NULLS LAST` fix
skipped a sibling (pattern 2), so legacy rows displace recent games. F-153 is
**the documentation-only constant**: `wd F50`'s "one const used by all eight
sites" shipped `#[allow(dead_code)]` with **zero** referents and a doc comment
saying manual sync is now required. F-156's `take(20)` over alphabetical friends
hides later friends permanently.

**Acceptance criteria**

1. A test **calls `game_history`** (the function under test) and asserts a
   game-type filter returns no other game type's rating. **F-151's existing
   `rating_before_aggregates_exclude_nulls` name-matches this risk exactly and
   never calls `game_history`** - it must be rewritten, and the reviewer must
   confirm the new test fails against the pre-fix code.
2. A test calls the stats entry point with an unknown game type and asserts an
   empty result, not the whole history (F-154).
3. F-153: the constant has referents at all eight sites, or it is deleted. A
   reviewer runs `rg "allow\(dead_code\)"` across `rust/web/src/stats/` and
   confirms zero remaining suppressions in this module.
4. F-150: each of the seven WP-52 `Test? y` rows has a test that **calls the
   function the row names**. This is the largest single instance of the
   "Test? y with no test" pattern - all seven rows of one WP.
5. F-155's copy-pasted justifying comment is corrected or deleted.

---

### R-18 - No network calls inside a database transaction

**Objective:** remove the three sites that hold a row or advisory lock across an
HTTP call.

- **Closes:** F-134 (High), F-135 (High), F-143 (Low, note).
- **Files:** `rust/web/src/proposals.rs:1702-1709`,
  `rust/web/src/email/inbound.rs:1021-1034`,
  `rust/web/src/email/sweep.rs:260-306`.
- **Size: M** - basis: three call sites, each needing the call hoisted out of the
  transaction with a re-check after.
- **Depends on:** nothing.

**The defects:** `start_proposal` calls the game service over HTTP while holding
the proposal `FOR UPDATE` lock (F-134). WP-79's **own commit** inserted
`fetch_game_from_service` inside the transaction and behind the lock (F-135) -
the fix introduced the defect. F-143 holds a row lock across a render **and a
Resend API call**.

**Acceptance criteria**

1. A reviewer greps each transaction body in the three files for HTTP client
   calls and records zero hits.
2. Each hoisted call is followed by a re-read and re-validation inside the
   transaction, with a test that **calls the enclosing function** under a
   concurrent modification and asserts correctness.
3. F-143 is recorded as reconciling the WP-46 vs WP-79 policy, not as a deviation
   from its own spec - section 11 marks it "Low, note" for that reason.

---

### R-19 - Invite nudge dedup

**Objective:** stop the per-tick re-nudge of the whole roster, and give
`send_turn_reminder` a caller or a grave.

- **Closes:** F-144 (High), F-147 (Medium).
- **Files:** `rust/web/src/email/sweep.rs:507-519` (+2 sites),
  `rust/web/src/email/notify.rs:523-543` (+1 site).
- **Size: M** - basis: a dedup-key change plus removal or wiring of a dead
  function.
- **Depends on:** nothing.
- **Attribution note:** WP-51 introduced **none** of F-144/F-145/F-146; they
  belong to WP-46 (`69bcd1e`) and the original #24 invite work. Do not look for
  the cause in `dcd8844c`.

**The defects:** F-144 keys the dedup **per proposal** against **per-invitee**
sends, so every tick re-nudges the whole roster. F-147 is a tooth-2 case:
`send_turn_reminder` **exists, has never had a caller**, and its doc comment
states the dedup as accomplished fact.

**Acceptance criteria**

1. The dedup key is per-invitee. A test **calls the sweep** twice and asserts no
   invitee is nudged twice.
2. `send_turn_reminder` is either called from the live path with a test that
   exercises it through that path, or **deleted**. Its doc comment must not
   survive either way. A reviewer greps for callers and records the count -
   present-but-callerless is not closed (tooth 2).
3. **Partial refutation, do not re-derive:** there is no pattern-4e revert in
   `dcd8844c`; it edited `sweep.rs::send_reminder` in place and
   `send_turn_reminder` was dead from birth.

---

### R-20 - Notification identity, threading and duplication

**Objective:** one event, one mail, correctly threaded and observable in tests.

- **Closes:** F-146 (Low/Medium), F-179 (Medium), F-180 (Low), F-181 (Low),
  F-182 (Low).
- **Files:** `rust/web/src/proposals.rs:401` (+8 sites),
  `rust/web/src/email/inbound.rs:1076` (+2 sites),
  `rust/web/src/email/proposals.rs:1471`, `:1470-1478` (+4 sites), `:111-120`.
- **Size: M** - basis: nine notification sites plus a seam change so the paths
  become spyable.
- **Depends on:** nothing.

**The defects:** five notifications share **one subject and thread id** (F-146).
Invite-accept auto-start mails one invitee **three times, under three different
preferences** (F-179). Solo-start notify is **always suppressed** (F-180).
`bot.turn` is published before notify, producing a double mail (F-181). Notify is
called **outside the mailer seam**, so it cannot be spied on in tests (F-182) -
which is why the other four survived.

**Acceptance criteria**

1. F-182 first: notify moves inside the mailer seam. Without this the other four
   have no testable assertion. A test **calls the proposal path** and asserts on
   the recorded mails.
2. A test asserts an invite-accept auto-start produces exactly one mail per
   invitee (F-179).
3. A test asserts a solo start produces a notification (F-180).
4. Subjects and thread ids are distinct per notification kind (F-146).
5. **Refuted, do not re-derive:** F-170 is **not** extended to the game-start
   mail - it reads `turn_emails_enabled` directly, so unsubscribed users do not
   get it; there is no hidden-information leak in the game-start mail (it is
   rendered from the recipient's own seat); and `ca7925bc`'s game-start sweep is
   complete (all four `insert_game_from_service` callers notify).

---

### R-21 - Close the `Gamer::validate` trust boundary at the trait

**Objective:** stop `validate` being fail-open by default. This is the root of
the whole game-crate panic family.

- **Closes:** F-06 (High) at the trait level. The per-crate work is R-22..R-27
  and coverage item 5.5.
- **Files:** `rust/lib/game/src/game.rs:106-108`.
- **Size: S** at the trait (remove the default body so every implementor must
  opt in) - but it **will not compile** until R-22..R-26 land, so schedule it as
  the closing commit of that family, not the opening one.
- **Depends on:** R-22, R-23, R-24, R-25, R-26.

**Acceptance criteria**

1. `Gamer::validate` has **no default implementation**, or the default returns an
   error rather than `Ok(())`. A reviewer greps for `fn validate` across
   `rust/game/*` and confirms 28 implementations (27 excluding the WIP
   `lords-of-vegas-1`, whose exclusion must be explicit and commented).
2. Every implementation has a test that **calls `validate`** with a short or
   inconsistent deserialized state and asserts `Err`. **No crate reviewed in this
   session has a `validate` test** - an override with no test is not coverage
   (pattern 2b).
3. `00-sweeps.md`'s list of 13 non-overriding crates is reduced to zero. Read
   that list; do not re-derive it.

---

### R-22 - `texas-holdem-2` state validation

- **Closes:** F-36 (High), F-44 (Medium), F-45 (Low).
- **Files:** `rust/game/texas-holdem-2/src/lib.rs:663-814` (+3 sites),
  `src/command.rs:47-69` (+1 site), `src/poker.rs:348-377` (+1 site).
- **Size: M** - basis: **seven parallel per-player vectors** raw-indexed with no
  `validate()`, so a short deserialized state panics `status()`.
- **Depends on:** nothing. **Blocks:** R-21.

**Acceptance criteria**

1. A test **calls `validate`** with each of the seven vectors short by one and
   asserts `Err` in every case. Bounding the player index against
   `player_count()` is not sufficient - `check_player`
   (`rust/lib/cmd/src/requester/gamer.rs:24-36`) does exactly that and gives **no
   protection against short parallel vectors**.
2. A test calls `status()` on a short state and asserts no panic.
3. F-44: a test calls the raise parser with a short stack and asserts the `Int`
   bounds do not invert.
4. F-45: `poker.rs` iteration is deterministic; a test calls the log-producing
   path twice on the same input and asserts identical output.

---

### R-23 - `lost-cities-1` / `lost-cities-2` parity

- **Closes:** F-60 (High), F-62 (Medium), F-63 (Low), F-64 (Low).
- **Files:** `rust/game/lost-cities-1/src/lib.rs:545-559` (+6 sites),
  `rust/game/lost-cities-2/src/lib.rs:550-577`, `:730-752`, `:32-35`, `:754-782`,
  test `:877-929`.
- **Size: M** - basis: two crates, one of which already has the fix; F-62 records
  that **only -2 got `validate()` and there is no reason for the split**.
- **Depends on:** nothing. **Blocks:** R-21.

**Acceptance criteria**

1. `lost-cities-1` gets `validate()`, and a test **calls `player_state()`** on a
   short `hands` vector asserting no panic (F-60 - it currently panics the render
   for **every** viewer).
2. A reviewer diffs the two crates' `validate` implementations and records any
   remaining difference with a justification (F-62).
3. F-63: the `unreachable!()` outside 2..=3 players is replaced by a `validate`
   rejection; a test calls the path with 4 players and asserts `Err`.
4. F-64: a test exercises 3-player scoring, which the rules define and no test
   covers.

---

### R-24 - `sushi-go-2`: F-06's row, the catch-all, and the false-premise panic

**Objective:** the mandated single package for `sushi-go-2` - F-06's crate row
and F-210 are the same defect seen twice.

- **Closes:** F-61 (High), F-65 (Medium), F-210 (Medium).
- **Files:** `rust/game/sushi-go-2/src/lib.rs:798-818` (+6 sites), `:140-147`
  (+1 site).
- **Size: M** - basis: six raw render-path indexes plus one `match` that is wrong
  in two different ways at the same lines.
- **Depends on:** nothing. **Blocks:** R-21.

**Why one package:** WP-09 added **three** length guards and left `playing[DUMMY]`
and **five render-path indexes** raw with no `validate()` (F-61 - the clearest
instance of pattern 2, inconsistent hardening within a single file). At
`:140-147` the same code is both a pattern-5 instance (`_ => 9` reproduces the
silent fallback the checklist row removed - F-65) and, at HEAD, an
`unreachable!()` **on a false premise that now panics the game service** (F-210).
Fixing one without the other leaves the crate wrong.

**Acceptance criteria**

1. `validate()` exists and a test **calls it** with each parallel vector short,
   asserting `Err`.
2. A test calls each of the six render-path sites (including `playing[DUMMY]`) on
   a short state and asserts no panic. The reviewer records the grep hit count
   for raw indexing in the file; it must be zero or each remaining one justified.
3. `:140-147` has **no `_` arm and no `unreachable!()`**. A test calls the
   function with the input that currently reaches `unreachable!()` and asserts a
   defined result (F-210), and a second test asserts the removed default is not
   silently reintroduced (F-65).

---

### R-25 - The remaining crates with no `validate` override

- **Closes:** F-24, F-31, F-33, F-37, F-49, F-51, F-54, F-70, F-82 (all Medium
  except F-70/F-82 which section 11 lists in the Low family).
- **Files:** `alhambra-1/src/lib.rs:813-963` (+3 sites);
  `starship-catan-1/src/lib.rs` (no `fn validate`);
  `seven-wonders-1/src/lib.rs` (+2 sites); `splendor-2/src/lib.rs:529-712`
  (+2 sites); `cathedral-2/src/lib.rs:343` (+6 sites);
  `sushizock-2/src/lib.rs:278-283` (+5 sites); `jaipur-2/src/lib.rs:161`
  (+14 sites); `for-sale-2/src/lib.rs:400-424` and
  `battleship-2/src/lib.rs:425-447`; `tic-tac-toe-2/src/lib.rs:190-204`
  (+1 site).
- **Size: L** - basis: nine crates, and `jaipur-2` alone has **14 sites** where an
  unbounded `current_player` reaches fixed-array indexing.
- **Depends on:** nothing. **Blocks:** R-21.
- **Sizing caveat:** this is the least confident estimate in the plan. The site
  counts come from section 11; the per-crate effort varies by an order of
  magnitude between `tic-tac-toe-2` and `cathedral-2`. **Re-size per crate before
  committing to a schedule** rather than treating L as a single bucket.

**Acceptance criteria (per crate, tracked as a checklist row each - see 4.9)**

1. `validate()` exists and a test **calls it** with a short/inconsistent state.
2. A test calls the crate's render entry point on that state and asserts no
   panic.
3. F-49 additionally: the unvalidated `i32` board indexes and the off-by-one are
   both fixed, with a test per boundary.
4. F-70: the player count is bounded **inside** `validate`, in both `for-sale-2`
   and `battleship-2`.

---

### R-26 - `validate` exists but misses the invariant the panic depends on

**Objective:** pattern 2b - the override is present and still insufficient. This
is a **distinct failure mode from F-06** and must not be folded into R-25.

- **Closes:** F-66 (Medium), F-67 (Medium), F-68 (Medium), F-74 (Low),
  F-76 (Medium).
- **Files:** `category-5-2/src/lib.rs:315`, `:228-241` (+1 site), `:228`;
  `zombie-dice-2/src/lib.rs:239-255`; `red7-1/src/lib.rs:240-266` (+2 sites).
- **Size: M** - basis: three crates, one cross-field invariant each.
- **Depends on:** nothing. **Blocks:** R-21.

**The pattern:** the `validate` overrides cover the parallel-vector sweep but
**miss the one cross-field invariant each crate's remaining panic actually
depends on**. F-66: `resolving` implies `Some(play)`, unchecked, so `expect`
panics. F-67/F-74: the equal-hand-size invariant is **only a comment** - and
F-74 records that the comment is **false**. F-68: best-effort refill then an
unconditional `drain`, so an empty cup panics. F-76: an all-eliminated state
**passes `validate`** and then `discard` panics on the empty map.

**Acceptance criteria**

1. Each named invariant is asserted **inside `validate`**, and a test **calls
   `validate`** with a state violating it, asserting `Err`.
2. For each, a second test calls the function that currently panics with that
   same state and asserts no panic.
3. F-74: the false comment is deleted, not corrected in place alongside a new
   check - leaving it invites the next reader to trust it again.

---

### R-27 - `for-sale-2` deadlock and short-deck stall

- **Closes:** F-05 (High), F-69 (Medium).
- **Files:** `rust/game/for-sale-2/src/lib.rs:130-135` and `:144-149`,
  `:301-327`.
- **Size: S** - basis: two functions in one crate.
- **Depends on:** nothing.

**The defects:** the underflow guard returns an **empty log**, so a short-deck
game stays `Active` with **no legal move** (F-05). `next_bidder` falls into an
**unbreakable loop**, so `bid()` hangs (F-69). Section 7 also records an
un-numbered escalation: `pass()` -> `take_first_open_card()` panics on
`open_cards.remove(0)`, and `start_selling_round` has the same shape - fix both
in this package.

**Acceptance criteria**

1. A test **calls the deck-exhaustion path** and asserts the game reaches a
   finished state, not an `Active` state with no legal move.
2. A test **calls `next_bidder`** in the all-passed configuration and asserts it
   terminates.
3. A test calls `take_first_open_card()` and `start_selling_round` with empty
   `open_cards` and asserts no panic.
4. `for-sale-2`'s `pass()` rounding the half-bid **in the player's favour**,
   opposite to the published rules, is either fixed here or confirmed to sit
   inside the WP-11 park. **Confirm, do not assume** - Unit 04c was asked to
   settle this and the answer must be recorded in this package.

---

### R-28 - `rand_bot` spec handling

- **Closes:** F-09 (High), F-10 (Medium).
- **Files:** `rust/lib/rand_bot/src/lib.rs:33`, `:13` (reached from `:70-72`),
  `:70-71`.
- **Size: M** - basis: the panic class WP-07 **claimed** to have fixed, plus an
  integer-width bug on the same lines.
- **Depends on:** nothing.

**The defects:** degenerate `Spec::Int` / `Spec::Many` specs **panic in
`rand_bot`, wire-reachable** - the exact class WP-07 recorded as fixed (F-09).
`as i32` wrap makes `min` negative, violating the spec (F-10).

**Acceptance criteria**

1. A test **calls `rand_bot`'s spec walker** with each degenerate `Int` and
   `Many` spec and asserts an error, not a panic. The reviewer confirms the test
   fails against pre-fix code - WP-07 already has a test that does not.
2. A test asserts no `as i32` narrowing occurs for out-of-range bounds.

---

### R-29 - `lib/cmd` panic paths and envelope handling

- **Closes:** F-17 (High), F-16 (Low), F-191 (Low).
- **Files:** `rust/lib/cmd/src/repl.rs:210` (+7 sites),
  `src/requester/gamer.rs:91-93`, `src/http.rs:26-29` (+1 site).
- **Size: M** - basis: **18 markup/IO/response paths** still unwrap or panic,
  including on a normal `Response::UserError`.
- **Depends on:** nothing.

**Acceptance criteria**

1. A reviewer greps `unwrap()`/`expect()`/`panic!` in `rust/lib/cmd/src/repl.rs`
   and records the count; each survivor is justified in a comment.
2. A test **calls the repl response handler** with `Response::UserError` and
   asserts a rendered message, not a panic.
3. F-16: the three variants that discard their payload now carry it; a test
   asserts the payload survives.
4. F-191: `http.rs`'s final form is **axum via WP-71**, so a malformed request
   *envelope* now yields a **400 with a text body** rather than a
   `Response::SystemError` JSON - different from what WP-06's test implies. Decide
   which contract is correct, write it down, and add an envelope-level test on
   `route::<G>()` (the unified report's section 7 unowned item, carried forward
   under plan section 5.7, records there is none).

---

### R-30 - Hidden information in `Log::public` (`R-LOG`)

**Objective:** close the leak class nobody checked - and give the log layer its
first tests.

- **Closes:** F-22 (High), F-28 (Medium), F-23 (Medium), F-29 (Low),
  F-30 (Low), F-34 (Low), F-38 (Low), F-39 (Low).
- **Files:** `alhambra-1/src/lib.rs:160-181`, `:452-479`;
  `modern-art-2/src/lib.rs:442-450` (+3 sites), `:53-75`, `:304-309` (+1 site);
  `seven-wonders-1/src/lib.rs:722-725`;
  `splendor-2/src/lib.rs:237-239`, `:79-97`.
- **Size: L** - basis: three crates plus a new test shape that **no game crate
  currently has**.
- **Depends on:** nothing. Fold the per-crate `Log::public` assertion into 5.5's
  checklist so the other 25 crates get it too.

**Why it exists:** the programme targeted hidden-information leaks and **every
fix and every test looked only at `pub_state` struct fields**. F-22 is
`start_game` public-logging each player's **exact opening money-card draw**,
making the whole hand reconstructible. F-28 is public money logs making "secret"
balances **always derivable**.

**Acceptance criteria**

1. A test per crate **calls the log-producing path** and asserts the rendered
   `Log::public` content contains no per-player private value. Asserting on
   `pub_state` fields does not satisfy this criterion - that is exactly the test
   shape that missed these.
2. **Apply the F-81 distinction explicitly in each case.** Owner ruling: hidden
   information *inferable* from legitimately public entries is **acceptable by
   design** and is not a finding. Only information appearing **directly** in
   `Log::public` content is in scope. F-22 and F-28 are direct leaks and remain
   valid; a reviewer must classify each fix against this line and record the
   classification.
3. F-23: `best_value`, an aggregate over the private hand, is no longer
   published. F-39's per-level deck sizes are **over-redacted** - this one goes
   the other way; restore the public information a player is entitled to.
4. F-38: the swallowed error at `splendor-2:237-239` no longer becomes a public
   log entry.

---

### R-31 - `category-5-2` player count

**Objective:** raise `MAX_PLAYERS` to match the published rules - and undo the
`RULES.md` edit that hid the gap.

- **Closes:** F-72a (High), F-72 (Medium), F-73 (Medium).
- **Files:** `rust/game/category-5-2/RULES.md:3`, `src/lib.rs:21` (+1 site),
  `:271-286` (+2 sites).
- **Size: M** - basis: a constant change plus the deal logic that assumes 8.
- **Depends on:** nothing.

**Why F-72a is High:** WP-32 edited the **published** `RULES.md` from "2-10" down
to "2-8" instead of raising `MAX_PLAYERS`. This is the canonical instance of
pattern 4b - the discrepancy the finding cited was erased by moving the
documentation to the code.

**Acceptance criteria**

1. `RULES.md:3` reads "2-10" again, and `MAX_PLAYERS` is 10. The commit message
   must state that this reverts a documentation edit, so the history records the
   4b correction.
2. F-73: `draw_cards` **errors** rather than returning short. A test calls it
   with an exhausted deck and asserts `Err`, and a second asserts hands remain
   equal length.
3. A test starts a 9-player and a 10-player game and plays a round.

---

### R-32 - Epilogue gate sweep

**Objective:** the `!was_finished` gate WP-08 introduced must exist in every
crate that copy-pasted the epilogue.

- **Closes:** F-18 (Medium), F-71 (Low), F-52 (Low), F-19 (Low), F-20 (Low),
  F-21 (Low), F-56 (Low).
- **Files:** `for-sale-2/src/lib.rs:495` (+9 sites); `battleship-2:531-537`;
  `sushizock-2/src/lib.rs:737-745` (+4 sites);
  `jaipur-2/src/lib.rs:699-715` vs `:665-677` (+4 crates);
  `starship-catan-1/src/lib.rs` (`finish_epilogue`);
  `acquire-1/src/lib.rs:247-262`; `cathedral-2/src/lib.rs:124-130` (+3 sites).
- **Size: M** - basis: **14 crates lack the gate**; the change is mechanical but
  the test per crate is not.
- **Depends on:** nothing.
- **Ownership note:** F-18's remediation covers `for-sale-2`, `sushizock-2`,
  `category-5-2`, `farkle-2` **and `battleship-2`** - F-71 records that the
  original list was one crate short.
- **`01c's checkmarks do not discharge this.** They are epilogue-*shape* only.

**Acceptance criteria**

1. Every crate's epilogue is gated on `!was_finished`. A reviewer greps for the
   epilogue call sites and records the count with and without the gate; the
   ungated count must be zero.
2. A test per crate **calls the finish path twice** and asserts one epilogue and
   one end log (F-52 currently produces a **double end log**).
3. F-19: the placings metric is written **once**, not twice, in the four affected
   crates.
4. F-20: `starship-catan-1`'s hardcoded `0..2` for placings is replaced by the
   real player count; a test with a non-2-player game asserts correct placings.
5. F-21: `acquire-1`, a migrated crate with **no regression test**, gets one.
6. F-56: a finished `cathedral-2` game no longer advertises `play` in its command
   spec; a test asserts the spec is empty at finish.

---

### R-33 - `acquire-1` correctness

- **Closes:** F-40 (Medium), F-42 (Low), F-43 (Low), F-46 (Low), F-47 (Low).
- **Files:** `rust/game/acquire-1/src/lib.rs:1119` (+2 sites), `:452-519`,
  `:1167-1173`, `:226-228` (+1 site), and the package test count at HEAD.
- **Size: M** - basis: one conservation invariant plus three localised bugs.
- **Depends on:** nothing.
- **Out of scope here:** F-41 (`stats: vec![]`) is **parked** under the F-35
  owner ruling. Record the occurrence; do not fix it in this package.

**Acceptance criteria**

1. F-40: the missing-share default no longer **mints phantom shares**. A test
   calls the share path with a missing entry and asserts total share conservation
   across the whole board - a conservation assertion, not a single-value check.
2. A test drives the real Festival-then-American-into-Imperial cascade in one `play b2` command, with the played tile at hand index 0 and two survivors, and asserts the played tile is gone and survivors retain their original relative order.
3. F-43: the turn check goes through `assert_player_turn`; a reviewer greps for
   turn checks that bypass it and records zero.
4. F-46: `pub_state()` no longer deep-clones the whole game.
5. F-47: the package test-count gate is met. Record the count.

---

### R-34 - `alhambra-1` scoring order

- **Disposition:** F-25 (Low) **refuted** - `inject_scoring_cards` places the
  two scoring cards in the second and fourth of five piles, which is the
  official Queen Games distribution once the back-pop draw direction is
  accounted for; no production change. F-26 (Low) **closed/fixed** - the
  final score is forced to the round-3 reward tier.
- **Files:** `rust/game/alhambra-1/src/lib.rs:223-236`, `:381-398` (+1 site:
  `:337-375`).
- **Size: S** - basis: one regression test (F-25, no production change) plus
  a one-line final-scoring fix with a test (F-26).
- **Depends on:** R-30 (same crate, same file; sequence after to avoid conflict).

**Acceptance criteria**

1. A regression test locks the current official Queen Games five-pile behavior
   and the explicit back-pop draw direction of `draw_cards`
   (`card_pile.pop()`, `lib.rs:249`). With `L` money cards remaining at
   injection and `f = floor(L / 5)`, every applicable valid setup size must
   fire round 1 after between `L - 4f` and `L - 3f - 1` money-card draws and
   round 2 after between `L - 2f + 1` and `L - f` money-card draws. The
   historic Go port's one-card-shifted round-1 distribution is **not**
   restored.
2. A test drives the early-final path directly: a valid 3-player game at
   round 1 in `FinalPlace` with all placement queues empty and one Pavillion
   on board 0, advanced through the real `FinalPlace` -> `End` transition,
   must end at round 3 and score 16 points, not the round-1 1 point.

---

### R-35 - `game_client`: stop shipping every seat's private state

**Objective:** the hidden-information-to-third-party class **did** land - not in
`prompt.rs`, but one layer below it.

- **Closes:** F-192 (Medium), F-193 (Medium), F-11 (Medium), F-12 (Medium),
  F-13 (Low), F-14 (Low).
- **Files:** `rust/lib/game_client/src/lib.rs:25-35` (+2 sites), `:310-331`
  (+1 site), `:188-191`, `:192-195`, `:57-65` (+2 sites), `:104-117`, `:435-442`.
- **Size: M** - basis: an error-`Display` change, a request-scope change, and a
  retry/timeout correction in one crate.
- **Depends on:** nothing.

**The defect:** the error `Display` embeds the **whole game-service response
body**; for `fetch_game_data` that body carries **every** seat's `player_renders`
plus raw state, and it reaches `tracing::error!(error = ?e)` with
`sentry_tracing` installed - i.e. private game state leaves the system to a
third-party error tracker. F-193 is the cause: `fetch_game_data` requests **all
seats and discards all but one**.

**Refuted, do not re-raise:** `rust/bot/src/prompt.rs` is a pure renderer over a
closed field list, its pattern-2 sibling check passes, and it predates the
programme.

**Acceptance criteria**

1. Error `Display` carries a status code and a bounded, redacted excerpt - never
   the response body. A test **calls the error's `Display`** on a full game body
   and asserts no seat identifier or private field appears.
2. `fetch_game_data` requests **only the acting seat**. A test asserts the
   outbound request scope.
3. F-11: `Response::UserError` is handled and the service message surfaced, not
   discarded.
4. F-12: the timeout ceiling is **per call, not per attempt** - it is currently
   ~6 minutes per call. A test asserts the total elapsed bound.
5. F-14's retry test no longer races the backoff; a flaky test is not an accepted
   outcome. F-13's version-name check rejects non-DNS labels.

---

### R-36 - Bot leak surface and startup robustness

- **Closes:** F-194 (Low), F-195 (Low), F-190 (Low), F-198 (Low).
- **Files:** `rust/bot/src/main.rs:585` (+1 site), `:276-282`, `:809-816`
  (+2 sites), `:776-827` (+2 sites).
- **Size: M** - basis: four independent defects in one binary, one of which
  (rustls) is a dependency and process-default change.
- **Depends on:** nothing.

**The defects:** `points()` **bypasses the redaction boundary** (F-194). TRACE
logs full prompts **including the bot's own hand** (F-195). An invalid key only
**warns**, and turns strand (F-190). There is **no rustls process-default install
or dependency** (F-198) - and section 11 records this is explicitly **not** a
checklist falsification and **not** a WP-64 regression; the omission is original.

**Acceptance criteria**

1. `points()` goes through the redaction boundary, or the bot stops calling it. A
   test asserts the bot cannot observe a value the redaction boundary hides.
2. TRACE prompt logging is removed or redacted; a test asserts no private hand
   content reaches the log at any level.
3. F-190: an invalid key **fails startup**; a test calls the loader and asserts
   an error. Warning-and-continuing is the same shape as 4.6.
4. F-198: a rustls process default is installed at startup and the dependency is
   declared.
5. **Also fix here:** the module-granularity `#[allow(dead_code)]` at
   `rust/bot/src/main.rs:4-7` covers `mod config` **and** `mod crypto`, and would
   hide `crypto::encrypt` and `LoadedKey::is_default`. Narrow it to the specific
   items or remove it.

---

### R-37 - Web auth hardening

- **Closes:** F-86, F-87, F-89, F-92, F-93, F-94 (all Medium), F-95, F-88,
  F-91 (Low).
- **Files:** `rust/web/src/auth/session.rs:67-72`; the confirmation, login,
  email-address, and test paths in `rust/web/src/auth/server.rs`; the auth and
  inbound route surfaces in `rust/web/src/router.rs` and
  `rust/web/src/email/inbound.rs`; and shared crypto at
  `rust/lib/crypto/src/lib.rs:21-44,120-162` (not the one-line
  `rust/web/src/crypto.rs` re-export).
- **Size: L** - basis: nine findings across the auth surface, plus the fact that
  **there is no rate-limiting middleware anywhere in `rust/web`** (F-94), which is
  a new component rather than a fix.
- **Status:** parked by user. R-37.0 is complete at
  `0270f296a39755b44feacf85d6d2220d7c8b4f80`. Do not implement the remaining
  R-37 work until the simpler unblocked remediation plan is complete and the user
  explicitly revisits R-37.
- **Ordering:** preserve R-37.1 before R-37.2 when revisited. No exact
  implementation units, migrations, or rollout are approved.

#### R-37.0 - Source and acceptance reconciliation

- **Depends on:** 5.4.
- **Status:** complete at `0270f296a39755b44feacf85d6d2220d7c8b4f80`.
- **Scope and evidence:** no production change; current source map and the approved
  park record are in `97-REMEDIATION-PROGRESS.md`'s R-37.0 evidence.

#### R-37.1 - Auth and session integrity

- **Status:** parked by user; preserve before R-37.2.
- **Approved behavior when revisited:**
  - F-87 confirmations are purpose-bound as `login` and `add_email`; a wrong
    endpoint consumes neither flow, while valid login keeps D-14's true-owner
    stealing behavior.
  - Expire every ambiguous active or legacy confirmation in the migration; an
    old-pod insert must never default to `login`.
  - Retain the 10-attempt cap. Incorrect codes for the same purpose count;
    wrong-purpose submissions do not. This supersedes stale non-consumption
    wording.
  - F-93 `add_email` returns generic acceptance and creates state or sends only
    when valid. F-92's historical `add_email` global-cap `Err` expectation is
    superseded: its eventual test asserts generic acceptance with no pending row,
    code, or send.
  - F-95 uses a conditional attempt increment capped at 10, with concurrent
    upper-bound evidence. F-88 remains in scope and is not closed by this record.

#### R-37.2 - Rate-limit and external-auth hardening

- **Status:** parked by user; follows R-37.1 when R-37 is explicitly revisited.
- **Approved behavior when revisited:**
  - Any rate limiting is PostgreSQL-backed in `web`: no standalone service and no
    Gubernator compatibility. It uses fixed-window counters keyed by scoped,
    opaque digests and never application-observed IP.
  - Retain settled login and confirmation caps. The former all-three-ingress and
    router-middleware criterion is superseded: the only new generic limiter is
    for signed Resend webhooks after signature verification.
  - Webhooks allow 600 verified events per five minutes globally and 20 accepted
    messages per canonical sender and validated route capability per five minutes.
    On denial or limiter database failure, return retryable generic `503` and
    write no processed marker.
  - F-91 accepts the no-AAD risk. Revisit it before another encrypted data
    category, credential movement or import, or multiple interchangeable
    ciphertext contexts; no crypto migration now.

**Unresolved before implementation:**

- The last proposed migration retained `email` as a single-column primary key,
  which cannot satisfy approved coexistence of `login` and `add_email`
  confirmations for one address.
- Safe rolling deployment and legacy-pod compatibility remain unresolved.
  Temporary `Recreate`, a new table, staged migrations, or another simpler design
  require later value/complexity review and user approval.
- Before implementation, perform a deliberate overengineering review. The selected
  design must justify complexity relative to value and protect readability,
  maintainability, and simplicity; reassess whether the generic webhook limiter
  and keyed-digest machinery remain proportionate.

**The defects:** session-store errors are swallowed, so a transient blip
**de-authenticates the user** (F-86). A legitimate pending email row is deleted
and a **second account forked** (F-87). Attempts are incremented **before**
authorization, so a victim's login code is burnable by an attacker (F-89). An
unverified row is committed **before** the send, giving a registered-address
oracle (F-93). Three WP-34-mandated regression tests were **never written**
(F-92). F-95 is the pattern-4c escalation: the WP-35 F1 concurrency test asserts
a **lower** bound where the spec prescribed an **upper** bound.

**Acceptance criteria**

1. The approved, purpose-bound confirmation and add-email behavior above is
   implemented with regression evidence; F-95 demonstrates the concurrent upper
   bound.
2. F-92: the three WP-34 regression tests exist and each **calls the server fn
   the spec row names**, including generic `add_email` acceptance with no pending
   row, code, or send at its historical global cap.
3. The approved verified-Resend-webhook limiter has its fixed-window, opaque-digest,
   denial, and database-failure behavior evidenced; no generic login,
   registration, or inbound-email router limiter is added.
4. F-86: a test calls the session middleware with a failing store and asserts the
   request errors rather than proceeding as anonymous.
5. F-91's accepted risk and revisit triggers are recorded without a crypto
   migration.
6. **Also fix here** (from unified report section 7, carried forward under plan
   section 5.7, no F-number):
   `verify_turnstile_rejects_on_transport_error` makes a **real network call to
   Cloudflare** and passes for the wrong reason
   (`auth/server.rs:1897-1902`); non-200, malformed JSON and live
   `success: false` are all uncovered. Shared crypto's `load_key` tests now live
   in `rust/lib/crypto/src/lib.rs:120-162`; do not claim an unfixed bot copy.

---

### R-38 - Admin surface and db module

- **Closes:** F-97, F-98, F-99 (Medium), F-106, F-100, F-103 (Low).
- **Files:** `rust/web/src/admin.rs:254-262` (+2 sites), `:515-533` (+1 site),
  `:787-813`; `rust/web/src/db/mod.rs:161-256`, `:119-159`, `:94-101`.
- **Size: M** - basis: two input-validation fixes plus a test module that names
  22 functions and asserts only degenerate cases.
- **Depends on:** 5.4 and 5.3 (the `require_admin` true-path coverage).

**The defects:** a prefix-only URL check makes the provider test an **admin
read-SSRF** (F-97). `api_key` is unvalidated - empty keys are stored, displayed
as `(set)`, and unbounded in length (F-98). One smoke test **names 22 functions
and asserts only degenerate cases** (F-99). F-103's pool panics from a `Result`
function. F-100 pins an **absent** session expiry as if intended.

**Acceptance criteria**

1. F-97: the URL check validates scheme, host and resolved address, not a prefix.
   A test calls the provider-test endpoint with a loopback and a link-local URL
   and asserts rejection.
2. F-98: empty and oversized `api_key` values are rejected at the boundary; a
   test asserts each.
3. F-99: the smoke test is replaced by per-function tests that **call each of the
   22 functions** with a non-degenerate fixture. A count of 22 real tests is the
   acceptance bar.
4. F-100: the test is rewritten to assert the **intended** expiry once one
   exists, or deleted. Pinning an absent behaviour as intended is pattern 4b.
5. F-103: the pool constructor returns `Err`, not `panic!`.

---

### R-39 - Visibility, bounds and untested guards

- **Closes:** F-141 (Low), F-142 (Low), F-149 (Low), F-157 (Low), F-133
  (Low). F-132 is refuted: `VisibilityCache` is local to one SSE connection,
  whose viewer is fixed before its task is spawned; no cache or SSE change is
  approved.
- **Files:** `rust/web/src/proposals.rs:1047,1087,1162`;
  `rust/web/tests/ssr_pages.rs:1290-1300`; `rust/web/src/friends.rs:99-107`,
  `:221-234`; `rust/web/src/game_info/mod.rs:44-52`;
  `rust/web/src/db/proposals.rs:40-52`.
- **Size: M** - basis: three unbounded queries, two untested guards and a cache
  key.
- **Depends on:** nothing.

**Acceptance criteria**

1. F-141: the three sweep queries are bounded; a test asserts the bound.
2. F-142: the leak assertion no longer uses a **random game id**, which makes it
   vacuous. The rewritten test must fail against a leaking implementation - the
   reviewer confirms this.
3. F-149: `block_user`'s guard gets a test. `rust/web/src/friends.rs` currently
   **has no tests at all**.
4. F-132: refuted, not implemented. `events_handler` fixes `viewer` before its
   per-connection task and constructs one local `VisibilityCache`; the cache is
   never shared between viewers. Preserve the two-stream SSE topology without
   additional cache machinery.
5. F-133: a proposal owner who is not a player can see their own proposal.
6. F-157: the eleven collapsed error contexts are restored.

**Approved execution units and evidence**

| Unit | Depends on | Allowed files | Acceptance evidence |
|------|------------|---------------|---------------------|
| R-39.1 | none | `rust/web/src/proposals.rs` | F-141: a shared proposal-sweep cap of 200 bounds all three candidate queries; a DB test calls each query with more than 200 candidates and asserts the bound. |
| R-39.2 | none | `rust/web/tests/ssr_pages.rs` | F-142: the existing non-admin export-route test seeds a real game and private-log sentinel, then asserts both 403 and sentinel absence. |
| R-39.3 | none | `rust/web/src/db/proposals.rs` | F-133: direct DB tests prove an owner without a roster row is visible and a stranger is not. |
| R-39.4 | none | `rust/web/src/friends.rs` | F-149: a direct server-function test, using `crate::test_support::non_admin`, calls `block_user` with an unknown id and asserts `User not found`. |
| R-39.5 | after R-39.4 (shared file only) | `rust/web/src/friends.rs`, `rust/web/src/game_info/mod.rs` | F-157: static inspection confirms six distinct friends-query contexts and five distinct game-info-query contexts inside their `try_join!` calls. |

- Run `git diff --check` and static acceptance inspection after each unit.
- After all source units, run exactly once from `rust/`:
  `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`.
- Runtime DB and SSR tests are CI-pending under the laptop constraint. An
  independent review is required after the final check because R-39.2 and
  R-39.3 exercise authorization and visibility boundaries.

---

### R-40 - Import path

- **Closes:** F-139 (Medium), F-121, F-122 (both "Low, informational").
- **Files:** `rust/web/src/game/import.rs:190-210`, `:109,124` (+1 site),
  `rust/web/src/bin/import_game.rs:20-32`.
- **Size: S** - basis: one retry bug and two guard gaps in a dev-only CLI.
- **Depends on:** nothing.
- **Severity note:** F-121 and F-122 are explicitly **informational** - the CLI is
  dev-only, there is **no HTTP route**, and nothing is attacker-controlled. Do not
  re-rate them upward. `a9609e57`'s 100 MiB guard is the relevant context.

**Acceptance criteria**

1. F-139: the retry no longer runs on an aborted transaction (it currently
   **always fails 25P02**). A test asserts the retry succeeds.
2. F-121: the stat-then-unbounded-read no longer bypasses the size guard; a test
   asserts a file over the limit is rejected by the read path itself.
3. F-122: bundle `undo_game_state` is **validated** before it is written.
   `game/import.rs:109,124` is the only site outside
   `update_game_command_success` writing a non-NULL `undo_game_state`, taken
   verbatim from an import bundle, and `undo_game` replays it after checking only
   non-NULL. A test asserts a malformed bundle state is rejected.

---

### R-41 - Email rendering, escaping and preference mapping

- **Closes:** F-175, F-176 (Medium), F-170 (Medium), F-171 (Medium), F-172,
  F-174, F-177, F-178, F-184 (Low).
- **Files:** `rust/web/src/email/outbound.rs:123-139` (+1 site), `:301-364`
   (+2 sites); `rust/web/src/email/render.rs:35-42` (+1 site), `:252-262`,
   `:152-164`; `rust/web/src/email/inbound.rs:135`, `:1377-1380`;
   `rust/web/src/email/commands.rs:179-208` (`:192`);
   `rust/web/src/email/unsubscribe.rs:159-173` (+3 sites);
   `rust/web/src/app.rs:55-70`;
   `rust/web/src/components/opponent_slot.rs:93-97` (+2 sites).
- **Size: M** - basis: nine findings in one module family, mostly localised.
- **Depends on:** nothing.

**The defects:** token ensure fns keep **select-then-update**, so tokens end up
unpersisted or lost (F-175). `e5513ec6` adds **no test**, falsifying four
`Test? y` rows (F-176). `pref_column()` has **no callers** and the live mapping
is an untested duplicate (F-170). The promised `List-Unsubscribe` absence test
**does not exist** (F-171). The CRLF sanitiser **truncates instead of replacing**
(F-172). Two of four hrefs are left unescaped (F-177), and `escape_html_attr`
duplicates `html_escape` (F-178). Help text advertises unavailable verbs (F-174).

**Acceptance criteria**

1. F-175: both sibling user-token ensure functions use an atomic update; tests
   call each concurrently and assert convergence on one persisted token, and call
   each with an unknown user id and assert an error.
2. F-176: the four `Test? y` rows get discriminating tests that call the named
   functions: `ensure_email_token` concurrently and with an unknown id (F44/F45),
   `try_send_rendered_email` plus its smallest private delivery-result boundary
   for the success/failure metric branches (F46), and `sentry_init_snippet` with
   `</script>` in both interpolated values and no frontend `tracesSampleRate`
   key (F63; `SENTRY_SAAS_EXCEPTION.md` compliance).
3. F-170: delete `pref_column()` and its callerless test. Parameterized
   unsubscribe endpoint tests cover all four `EmailKind` slugs and assert the
   live preference mapping changes only its intended column. Note the game-start
   half of F-170 is **refuted**; only the row itself is open.
4. F-171: a test asserts `List-Unsubscribe` is **absent** where the row specified
   it - this is the fifth confirmed "Test? y" with no test and the most explicit,
   because the row said exactly what to assert.
5. F-172: folded CR/LF followed by header whitespace collapses to one space, so
   content after the fold survives; bare CR/LF terminates parsing, preserving the
   existing `Bcc:` injection rejection. Tests assert both boundaries.
6. F-177/F-178: one escaping helper, used by all four hrefs; a reviewer greps for
   the duplicate and confirms it is gone.
7. F-184: the pre-settle default no longer stores `"medium"`. **The settled-path
   half is refuted** - only the pre-settle residual is a defect; scope the fix
   accordingly.

---

### R-42 - Frontend, theme and colour

- **Closes:** F-164, F-165, F-166, F-167, F-168 (Low), and records F-15
  (Medium) as **latent**.
- **Files:** `rust/web/style/main.scss:1091-1094`;
   `rust/web/src/websocket_client.rs:84-101`;
   `rust/web/src/game_info/queries.rs:14-24` (+1 site);
   `rust/web/src/theme.rs:12-19`; `rust/web/tests/ssr_pages.rs:256-266`
   (+1 site).
- **Size: M** - basis: five small independent fixes across the frontend.
- **Depends on:** nothing.

**Approved execution units and evidence**

1. **U42.1 (F-164):** `rust/web/style/main.scss` only. Replace the literal with
   `var(--mk-orange)`; source inspection confirms the existing `(N new)` text is
   the non-hue cue. No Cargo command for this CSS-only unit.
2. **U42.2 (F-165):** `rust/web/src/websocket_client.rs` only. Move only the
   `last_update` bump outside each `Closed` guard; do not redesign EventSource
   liveness. Source inspection is the acceptance evidence; this row remains
   `Test? n`.
3. **U42.3 (F-166):** `rust/web/src/game_info/queries.rs` only. Apply the
   existing `name DESC` tiebreak and add a tied-`created_at` regression test
   that calls both selectors. Review records the two-selector count and
   unchanged visibility predicates.
4. **U42.4 (F-167):** `rust/web/src/theme.rs` only. Remove the dead Red/86
   expression and correct the comment; inspect generated-token expectations.
5. **U42.5 (F-168):** `rust/web/tests/ssr_pages.rs` only. Assert the relevant
   accessible `href="#"` properties rather than the absent styling marker.
   Runtime DB/SSR evidence remains CI-pending.

**Acceptance criteria**

1. F-164: the hardcoded `orange` is replaced by a theme token; a test or visual
   check covers all 34 themes.
2. F-165: the refetch bump moves back outside the `Closed` guard so the badge
   refreshes. **F-165's checklist row is `Test? n`** - no test was owed and it is
   explicitly excluded from the "Test? y with no test" tally; do not count it.
3. F-166: the tiebreak is applied to **both** "latest version" queries
   (pattern 2). A reviewer greps for the sibling and records the count.
4. F-167: the dead `(Red, 86)` entry, which emits **72 dead declarations**, is
   removed.
5. F-168: the a11y tests assert **presence** of the accessible property, not
   absence of the inaccessible one.
6. **F-15 stays LATENT and is PARKED for the later parked-item review.**
   Obligation 4 remains discharged: every `--mk-soften-*` token referenced
   anywhere is emitted; game crates emit exactly
   `{(Pink,80),(Foreground,80),(Foreground,90)}` from three sites, identical to
   `IN_USE_SOFTENS`, and no game emits a `mix`. A local equality test would not
   enforce future renderer output because `rust/web/src/theme.rs` has no
   producer relationship with game renderers. Source scanning, build-time
   generation, and parser validation are not justified inside R-42. This does
   not resolve the latent issue.

---

### R-43 - Enforce the `bans` section (`deny.toml`)

**Objective:** three views of **one unenforced `bans` section**. Mandated single
package.

- **Closes:** F-199 (Medium), F-206 (Medium), and Unit 10b's **Coverage gap 3**.
- **Files:** `rust/deny.toml:71-76` (+3 sites) and `:131`,
  `.github/workflows/deps-currency.yml` (+1 site), `rust/web/Cargo.toml:44`.
- **Size: M** - basis: 29 skip entries to triage plus a CI job change plus WP-69
  §5's parked negative checks.
- **Depends on:** nothing.

**The defect:** `cargo deny` in the weekly job checks **advisories only** and
never runs `bans`, so the `bans` section is decorative (F-199). WP-69's spec §3b
was a **stop-and-report threshold** at "roughly a dozen" skip entries;
`e2ee5342` shipped **29** and answered the trigger with the comment *"not
papered-over sibling work"* - a claim falsified by `:131`'s own annotation
(`tower-http 0.7.0`, "via web (first-party, pins 0.7.0 directly)", against
`rust/web/Cargo.toml:44`), the only one of the 29 with a first-party cause, all
29 checked (F-206). WP-69 §5's "the flip must actually bite" negative checks are
recorded in `EXECUTION-STATE.md` as **parked, never run**.

**Neither F-199 nor F-206 is a falsified checklist row** - `dp F23` is `Test? n`.
They are a scope and enforcement failure, not a testing one.

**Acceptance criteria**

1. The weekly job runs `cargo deny check bans` (and licenses), and **fails**, not
   warns. A reviewer confirms by inspecting the workflow, not the commit message.
2. The `tower-http 0.7.0` skip is removed by fixing `rust/web/Cargo.toml:44`'s
   direct pin - it is the one entry with a first-party cause and its presence is
   what falsifies the rebuttal comment.
3. The remaining skips carry an **expiry date** and a `unused-skip` check flags
   stale ones. The list count is recorded; it is **29**, not 24 (10b's count was
   corrected by 10c).
4. WP-69 §5's negative checks are run and their results recorded, or formally
   abandoned by the owner. **A parked check is not a passed check.**
5. The rebuttal comment at `:71-76` is deleted. Per section 4.6, a code comment
   is never a valid answer to a spec's stop-work trigger; leaving it in place
   preserves the falsified claim in the tree.

---

### R-44 - Fix the vendored session store

- **Closes:** F-200 (Medium), F-201 (Medium), F-202 (Low).
- **Files:** `rust/lib/session_store/src/postgres_store.rs:87-130` (+2 sites);
  `rust/web/src/db/users.rs:256` (+2 sites);
  `rust/web/src/db/test_support.rs:146-152`.
- **Size: M** - basis: one migration function plus an error-classification
  recheck across the sqlx 0.8 -> 0.9 boundary.
- **Depends on:** coverage item 5.2 (the crate has **no `tests/` and no
  `#[cfg(test)]` module**, so the test module must be created first).
- **Related but distinct:** R-45 (the sweep) and owner decision 6.1 (the policy).
  **Do not wait for the policy to fix the defect.**

**The defect:** `migrate()` returns `Ok(())` **before** `create table` and
**without committing** on the duplicate-key path, and it is the **sole** creator
of `tower_sessions.session` - nothing in `rust/web/migrations/` creates it. A
cold start with more than one web replica reports success and the table is never
created.

**Acceptance criteria**

1. A test **calls `migrate()`** concurrently from two connections against a fresh
   database and asserts the table exists afterwards.
2. A test asserts `migrate()` returns `Err` when the table was not created.
3. F-201: every sqlx error classification changed by the 0.8 -> 0.9 upgrade is
   rechecked; the reviewer lists the classifications examined.
4. F-202: `table: &str` is no longer interpolated behind `AssertSqlSafe`; it is
   either a compile-time constant or validated against an allow-list.

---

### R-45 - Repo-wide vendored-code sweep (`R-VEND`)

**Objective:** find out how much vendored third-party code actually exists.
**This has never been done.**

- **Closes:** no F-number. It closes an unknown.
- **Files:** whole repo.
- **Size: S** for the sweep itself; **unsized** for whatever it finds - say so
  rather than guessing.
- **Depends on:** nothing to start. Its **output** feeds owner decision 6.1.

**Acceptance criteria**

1. A written inventory of every directory in the repo containing code copied from
   a third party, with the upstream source, version, licence and the reason.
   Only `rust/lib/session_store` is currently known.
2. Each entry states whether the licence obligation is **machine-checked**. For
   `session_store` the answer today is **no**: `rust/deny.toml:45-49` sets
   `[licenses.private] ignore = true` and the crate is `publish = false`, so
   cargo-deny skips it. Unit 10c established this is **correct config** and that
   **no cargo-deny setting** could machine-check it.
3. Section 7's proposed closure for U7 - a **one-line CI grep** asserting the
   licence and attribution files are present in each vendored directory - is
   implemented. `cargo-deny` is the wrong instrument; do not try to make it work.
4. Each entry states its **known inherited upstream defects** (section 4.8).

---

### R-46 - Workspace lint configuration

- **Closes:** F-203 (Low), F-204 (Low), F-197 (Low).
- **Files:** `rust/Cargo.toml:78-79`, `:56-76`;
  `rust/game/love-letter-2/.rls.toml`, `rust/lib/rand_bot/.rls.toml`.
- **Size: S** - basis: one table to add and ten dependency entries to respell.
- **Depends on:** **owner decision 6.3** - WP-64's rider 1 forbids bare-major
  spellings while §3b of the **same spec** endorses them. The criterion is
  internally inconsistent and someone must rule before this is "fixed".
- **Severity note:** F-203 and F-204 are **spec-vs-code gaps, explicitly not
  regressions**; WP-64 has no checklist row. F-197 is a `Test? n` row and **not**
  a falsified row - it is a pattern-2 sweep gap only. None of these three count
  toward the "Test? y with no test" tally.

**Acceptance criteria**

1. `[workspace.lints.rust]` exists, or the owner records that only the clippy
   half is wanted.
2. The ten bare-major entries are respelled, or rider 1 is struck. **One or the
   other - not both left standing.**
3. F-197: the two remaining `.rls.toml` files are removed; WP-65
   (`2c28ae8`) already removed the other three siblings. A reviewer greps for
   `.rls.toml` and records the remaining count.

---

### R-47 - Amend the finding corpus (tooth 4)

**Objective:** the corpus currently records a fact that is false. Fix the record.

- **Closes:** F-205 (Low).
- **Files:** this section; `docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46`
  and `:141-143` plus a new traceability record; and two archive docs that still
  assert the `0.0.0.0:80` default and cite a WP-73-deleted bin path.
- **Size: S** - basis: four files, six text locations.
- **Depends on:** nothing.

**The defect:** `dp F12`'s "sentry drags actix-web + ureq into every build" was
**never true** - neither is a sentry 0.48 default and nothing enables them; both
are still inert `[[package]]` entries in `rust/Cargo.lock` at HEAD **after a
later regeneration**. WP-67's own **rider 2 required** the downgrade be written
back into the finding. It never was.

**The historical findings are immutable.** The original dp F12 text
(`findings/dependencies.md:103-108` and the `:157` svix follow-on) survives
**only** at the immutable revision
`868094a6c8177858dededdd5321ce0c03882ada5` - it is not in the current tree and
cannot be amended. This plan corrects the record with **current-tree
traceability** instead: the false claim is struck from `SUMMARY.md`, and a
traceability record states where the legacy dp F12 text and the current F-205
finding live.

**Acceptance criteria**

1. `SUMMARY.md:44-46` (headline dependencies bullet) and `:141-143` (WP-67 record)
   are amended to state the disproved premise and the correct fact. **Closing
   the finding without amending it is the defect** - do not simply mark it done
   a second time.
2. `SUMMARY.md` gains a concise traceability record: current F-205 is documented
   in `docs/reviews/2026-07-30-review-session/99-UNIFIED-REPORT.md` (P11,
   section 5, and the tooth-4 sign-off rule, section 10); the original legacy
   dp F12 text exists only at the immutable revision
   `868094a6c8177858dededdd5321ce0c03882ada5` in
   `docs/reviews/2026-07-23-rust-review/findings/dependencies.md:103-108,157`.
   `SUMMARY.md` carries no `wd Fnn` / `wfe Fnn` identifiers, so citations
   resolve to the unified report or the immutable revision above. The findings
   corpus is **not** recreated.
3. The two archive docs asserting the `0.0.0.0:80` default are corrected to
   `0.0.0.0:8080`, and the WP-73-deleted bin path is replaced with the current
   `brdgme_game_bin` entrypoint. The `0.0.0.0:80` -> `:8080` change is **inert**
   in production - all 44 current k8s Deployments under `k8s/base/game` set
   `ADDR` explicitly to `0.0.0.0:8080` (verified count) - so this is a
   documentation fix, not a behaviour change.

---

### R-48 - `hanamikoji-1`

- **Closes:** F-209 (Medium), plus the crate's epilogue gap.
- **Files:** `rust/game/hanamikoji-1/src/lib.rs:673-730` (+4 sites) and `:833`.
- **Size: M** - basis: a cross-field invariant in `validate` plus an epilogue
  migration.
- **Depends on:** nothing. **Blocks:** R-21. **Related:** R-16 (its delivery
  gap).

**The defect:** `validate` **never relates `phase` to `pending`**, so a crafted
state either wedges the game or loses cards (F-209).

**Scope correction (2026-08-03):** the existing
`!was_finished && self.is_finished()` guard at `lib.rs:830` predates F-209 and
already prevents duplicate epilogues. The epilogue is not part of this package.

**Acceptance criteria**

1. A test **calls `validate`** with a `phase`/`pending` mismatch and asserts
   `Err`, covering all four cited sites.

---

### R-49 - `lib/markup` and the command parser

- **Closes:** F-01, F-02, F-03, F-04, F-08 (Medium). F-07 is **REFUTED**, not
  closed here (see F-07 disposition below).
- **Files:** `rust/lib/markup/src/lib.rs:43-56`, `src/parser.rs:737-757`,
  `src/wrap.rs:14-24`; `rust/lib/game/src/command/parser/mod.rs:45-62` and
  `:1095-1100` (+3 sites); `rust/lib/game/src/command/suggest.rs:39-52`.
- **Size: M** - basis: five findings, of which F-08 (Chain2/3/4 `expected()`
  parity) touches four sites and is the only non-trivial one.
- **Depends on:** nothing.

**F-07 disposition (refuted)**

F-07 is refuted under the current trusted `no-thanks-2` contract. Active
`Game::pub_state()` (`rust/game/no-thanks-2/src/lib.rs:269-295`) derives
`current_card` as `Some(self.peek_top_card())` whenever the game is unfinished,
and `Game` has no `current_card` field, so no such field can be validated at the
proposed location. No response-validation layer, render fallback,
`no-thanks-2` code change, or new contract is added. Hostile or mismatched
service-response validation requires separately approved scope.

**Implementation scope**

- **R49.1:** F-01, F-02, F-04 (`lib/markup`).
- **R49.2:** F-03, F-08 (command parser and suggest parity).

**Acceptance criteria**

**R49.1**

1. F-01: the dead `&str` is removed from the return tuple.
2. F-02: the disjunctive overflow assertions are replaced with assertions that
   pin a specific outcome. **An assertion that cannot fail is a decoy** (tooth 3
   applied to assertions rather than call sites).
3. F-04: `wrap_segment` is no longer O(n^2); a benchmark-free complexity argument
   is recorded in a comment. **Do not run benchmarks.**

**R49.2**

4. F-03: a test asserts parse and suggest fold identically for the same input -
   Unicode fold parity for complete tokens while preserving intentionally
   incomplete autocomplete fragments.
5. F-08: a test **calls `expected()`** on Chain2, Chain3 and Chain4 and asserts
   parity with Chain1's contract, at all four sites.

The duplicated `impl Parser for CommandSpec`
(`rust/lib/game/src/command/parser/mod.rs:878`) is deliberately retained as the
suggest engine's advancement mechanism and must not be removed; see
`docs/decisions/COMMAND_PARSER_SPEC_DEDUP.md`.

---

### R-50 - Remaining game-crate low findings

- **Closes:** F-32, F-53, F-75, F-78, F-79, F-80, F-83, F-84, F-77 (Nit).
- **Files:** `starship-catan-1/src/render.rs:22-23` (+1 site);
  `sushizock-2/RULES.md:7-8` (+2 sites);
  `zombie-dice-2/src/lib.rs:257-265` vs `:294-299`;
  `greed-2/src/lib.rs:365` (+3 sites in `farkle-2`);
  `farkle-2/src/render.rs:26-46` (+1 site);
  `liars-dice-2/src/command.rs:44-47` (+1 site);
  `roll-through-the-ages-2/src/lib.rs:741-756` (+1 site);
  `seven-wonders-1/src/lib.rs:1748-1774` (+2 sites).
- **Size: M** - basis: nine independent small fixes across eight crates.
- **Depends on:** F-83's fix should follow coverage item 5.1 (the
  `roll-through-the-ages-2` crate-level review), since that review may reframe it.
- **Out of scope here:** F-58 is **informational** - four more `stats: vec![]`
  sites for the parked F-35 tally. Record, do not fix.

**Acceptance criteria**

1. **F-79 and F-83 are pattern-4b remediations and need the strongest criteria.**
   F-79's new test **re-hardcodes the legacy values**, and F-83's new test
   **asserts the unchanged value where the spec prescribed the changed one**. In
   both cases the test must be rewritten to assert the **spec's** value, and the
   reviewer must confirm the rewritten test **fails against current HEAD**.
   A test that passes today is proof the code was not fixed.
2. F-78: the overflow fix is applied to the twin in `farkle-2` that was skipped
   (pattern 2); the reviewer records the grep hit count.
3. F-84: the test loop covers the real maximum of **7** players, not `3..=4`.
4. F-53: `RULES.md` and `DATA_DOCS.md` agree. **Reconcile toward the published
   rules, not toward the code** - moving the doc to the code is F-72a's mistake.
5. F-80: the parser accepts every bid the rules allow; a test enumerates the
   boundary bids.
6. F-75: the duplicated inline turn reset is extracted to one function.
7. F-32: full boards are no longer cloned, and the blocklist test is replaced by
   an allow-list assertion (asserting absence is F-168's failure shape).
8. F-77: the stale doc comment is corrected. Section 11 rates it "Nit" - do not
   inflate it.

---

### R-51 - Operator and game-type version lifecycle

- **Closes:** F-196 (Medium), F-140 (Medium).
- **Files:** `rust/operator/src/controller.rs:240` (+2 sites);
  `rust/web/src/db/game_types.rs:81-91` (+1 site).
- **Size: M** - basis: a guard that must learn to write backwards, plus a filter
  fix, in a crate with **no test coverage at all** for `cleanup`.
- **Depends on:** nothing.

**The defects:** WP-62's authoritative-version guard **only writes forward**, so
deprecating the newest version permanently strands `game_types`; `cleanup` has
the same shape and **zero test callers** (F-196). F-140's deprecated-version
filter breaks the rules lookup for **in-flight games** - games already running on
a now-deprecated version lose their rules page.

**Acceptance criteria**

1. A test **calls the version guard** deprecating the newest version and asserts
   `game_types` is not stranded.
2. A test **calls `cleanup`** - it currently has zero test callers - including
   the already-newest-to-deprecated case section 5.7's U13 names as untested.
3. F-140: a test calls the rules lookup for an in-flight game on a deprecated
   version and asserts the rules are returned.

---

### R-52 - Verify Unit 07b's coverage of WP-51 and WP-53 (`R-VER`)

**Objective:** settle the one unresolved scoping question in the review, so the
coverage claim is either true or corrected.

- **Closes:** no F-number. It closes unified report section 8.12's open question.
- **Files:** commits `dcd8844c` (WP-51) and `3610b957` (WP-53). Surfaces named as
  unaudited: the six `RealInviteMailer` methods, the `spawn_sweep` collapse, and
  `notify_owner_decline` gating.
- **Size: S** - basis: a re-walk of two commits, read-only.
- **Depends on:** **owner decision 6.2** (whether the pass is worth doing).

**Context:** Unit 07b **died to quota before producing any finding** and was
re-dispatched from scratch. The re-dispatch produced F-144..F-149, but whether it
re-walked the full surface of both commits was never confirmed. Note WP-51 **does
have a spec** (`planning/specs/WP-51-invite-mailer-notify-dedup.md`, Tier-2) - an
earlier brief wrongly said otherwise.

**Acceptance criteria**

1. Each of the three named surfaces is read at line level and the result recorded
   as covered-clean or a new finding.
2. WP-53's residual cosmetics are dispositioned: `3610b957` deleted
   `encode_path_segment`'s doc comment and left a mid-file
   `use percent_encoding::...` at `players.rs:34`, and `wd F77` was satisfied by a
   two-word swap in `settings.rs:1-2`.
3. **Refuted, do not re-derive:** `restart_core`'s pool-read-under-`FOR UPDATE`
   is **not** a deadlock (different table, no write in-transaction). The
   off-convention residual against its neighbours at `:1136`/`:1145` has no owner
   and can be closed as a convention note.

---

### R-53 - `Gamer::points()` ordering contract

**Objective:** give `points()` a documented contract, because at least one crate
disagrees with itself about its sign.

**Execution status:** Parked for the later parked-item review. `higher-is-better`
is recommended from `gen_placings` and prevailing score semantics, but is not an
approved contract. Cathedral currently negates remaining-piece values for
placings while returning positive values from `points()`. The exact game-crate
conformance population remains unsized, and the "other 27 crates" estimate is
unverified. No acceptance criterion is complete and no source or conformance
sweep starts before the ruling.

- **Closes:** U2 (no F-number exists anywhere for this).
- **Files:** `rust/lib/game/src/game.rs` (the trait),
  `rust/game/cathedral-2/src/lib.rs:580-584` and its `calc_placings`.
- **Size: S** for the contract and the `cathedral-2` fix; **unsized** for the
  game-crate conformance sweep until its population is inventoried. The current
  "other 27 crates" estimate is unverified.
- **Depends on:** nothing. **Do not route this to another review unit.** Unit 08
  handed it **back**: WP-52 touches no `lib/game` surface and stats read DB
  columns only, so no review unit will ever own it. It needs a remediation owner.

**Acceptance criteria**

1. `Gamer::points()` has a documented ordering contract (higher-is-better or
   lower-is-better, stated once).
2. `cathedral-2`'s sign is consistent with its own `calc_placings`; a test
   **calls both** and asserts they agree.
3. A sweep records, per crate, whether `points()` matches the contract. Record
   the count of conforming crates; do not assume.

---

### R-54 - Small unowned fixes

**Objective:** three independent one-to-two-line items that no unit owned and no
finding number covers.

- **Closes:** U8, U9, U14.
- **Files:** `.github/workflows/` (the e2e job),
  `rust/web/src/auth/server.rs:92`,
  `rust/web/src/email/inbound.rs:95-96` and `:678-693`.
- **Size: S** - basis: each is a few lines; the e2e item may reveal a backlog of
  real failures once it starts gating.
- **Depends on:** nothing. Sequence U8 last - turning the gate on will surface
  whatever the hydration race has been hiding.

**Acceptance criteria**

1. **U8:** the e2e job is no longer `continue-on-error: true`, so it gates merges
   and deploys. It currently gates **nothing**. F-211's assertion was restored
   separately in R-16. If the hydration race makes it flaky, that flake becomes a
   tracked defect, not a reason to keep the gate off.
2. **U9:** `login_emails_sent_total` is incremented **after** a successful send,
   and a failure counter exists. A test asserts a failed send does not increment
   the success counter. The `wfe F46` fix has this shape but was scoped outside
   WP-60, so this site was never corrected.
3. **U14:** `i-noreply@brdg.me` is short-circuited in the inbound router, saving
   one wasted token-lookup query per one-way mail. A test calls the router with
   that address and asserts no lookup occurs.

---

### R-55 - Read the surfaces nobody read

**Objective:** this is a **review** package, not a fix package. Four surfaces
were never read at line level by any unit, and one work package has no
verifiable acceptance criteria at all.

- **Closes:** U10, U15, U16, U17.
- **Files:** `rust/bot/src/routing.rs`; WP-46's `proposals.rs` test module and
  `email/sweep.rs`'s test module; `rust/web/src/admin.rs:1560-3488`,
  `rust/web/src/db/game_write.rs` and `update_game_command_success`; WP-36.
- **Size: L** - basis: `admin.rs:1560-3488` alone is ~1,900 unread lines, and
  `game_write.rs` is the file that produced F-111..F-120 from the parts that
  *were* read.
- **Depends on:** nothing. Run it **before** R-05 and R-06 if capacity allows -
  it covers the same file and may change their scope.

**Acceptance criteria**

1. Each surface is read at line level and the result recorded as covered-clean or
   as new findings appended to the corpus.
2. **U10:** `rust/bot/src/routing.rs` had no owning sub-unit at 05b time and 10a
   took only `prompt.rs`. It is read in full.
3. **U15:** the two test modules are audited, not just the four spec-named
   functions that were read. The audit asks tooth-3 of every test: does it call
   the function under test?
4. **U17:** **WP-36 has neither a spec nor a checklist row**, so Unit 05's crypto
   verdict rests on a commit message. Treat WP-36 as **unverified** until a spec
   is reconstructed from the commit and the owner confirms it. This is the same
   class as WP-72 (section 4.3) but with a security verdict resting on it.


## 3. Deployment checklist family

These are **not code findings**. No code change closes them; they are changes to
deployment manifests and to the GitOps repo
`/home/beefsack/Development/brdgme-config`, plus one Dockerfile pin. They are
grouped because they share a failure mode: the code is already correct and
fail-closed, and shipping it against an unprepared environment is the outage.

`brdgme-config` is a **GitOps repo, not a Kubernetes Secret** - the commands are
in `F-96-turnstile-key.md`; do not re-derive them.

| Item | What must be set | Failure mode if shipped without it |
|------|------------------|-------------------------------------|
| **F-96** (downgraded - deployment blocker, not a code defect) | Real `TURNSTILE_SECRET_KEY` in prod, or `ALLOW_INSECURE_DEFAULT_KEY` deliberately set | `rust/web/src/main.rs:40-45` panics at startup. Dev and CI already set the opt-in flag (`k8s/dev/web-patch.yaml:18-19`, `scripts/rust-test.sh:64`); **no manifest sets the var in prod**. The panic is correct - it is the only thing preventing the `secret.is_empty() -> true` fail-open at `auth/server.rs:256-277`. |
| **`TURNSTILE_SITE_KEY` startup check** (new, from the F-96 out-of-band report) | The site key, set **in the same change** as the secret key | No startup check exists; it silently defaults to empty, renders no widget, and rejects every login. **Setting only the secret key is a total login outage.** Both halves must land together, and a startup check for the site key should be added alongside. |
| **`config::public_base_url()`** (from Unit 09b) | An HTTPS production base URL | Defaults to `http://localhost:3000`, which makes WP-58's `List-Unsubscribe` header non-HTTPS and **RFC 8058-invalid** in production. Mail providers may reject or ignore the header. |
| **F-207** (Low) | Reconcile the three sqlx migrators: `sqlx-cli` **0.8.6** pinned in `rust/Dockerfile:132`, **unpinned latest** in `.github/workflows/ci.yml:90-92`, and the **0.9** library used by `#[sqlx::test]` | Divergent `_sqlx_migrations` checksum/format writes. **No commit in the entire 127-commit range touches `rust/Dockerfile`** and no spec mentions the pin. Mitigating: `rg 'migrate!' rust` is empty, so nothing validates checksums at runtime - this is why it is Low, not why it is safe. |
| **F-211** | See section 11 for the exact wording; it is grouped here as a delivery/manifest item alongside F-208's hand-maintained lists (see `R-DEL` in section 2). | The `hanamikoji-1` delivery gap: the crate has **no `rust/Dockerfile` stage** (26 game stages at `:174-303` against 28 workspace members; the other absentee is `lords-of-vegas-1`, WIP and excluded). It **is** built by `cargo build --release --workspace --exclude web` and then never copied into an image. |

**Also suggested, not urgent** (from the F-96 report): split
`ALLOW_INSECURE_DEFAULT_KEY`. One flag currently disables two unrelated guards.

**Acceptance for this whole section:** a single pre-rollout checklist file lives
in `brdgme-config` and is referenced from `docs/DEV.md`; a reviewer can point at
the manifest line that sets each variable above; and the Turnstile pair is
verified as a pair (site key set **and** secret key set) rather than
individually.

## 4. Process fixes

Source: unified report section 4 (systemic patterns) and section 10 (the
four-tooth sign-off rule). These are changes to how work is specified, checked
and signed off. They are not optional: every High finding in this review came
from a commit whose checklist row, commit message and tests all read clean.

### 4.1 The four-tooth sign-off rule (the centrepiece)

A finding may only be marked closed when **all four** hold. Each tooth exists
because a real closed finding in this programme defeated the weaker version.

1. **The citation still exists.** (F-109) WP-36 shipped the ws F55 shutdown drain
   plus a dedicated regression test `rust/web/tests/websocket_hygiene.rs`; the
   later SSE migration `efad81f` **deleted the fix and the test together**, and
   the checklist row and both commits still read as closed.
2. **The citation is reachable.** (F-147) Present is not enough.
   `send_turn_reminder` exists, has **never had a caller**, and its doc comment
   states the dedup as accomplished fact.
3. **The regression test actually calls the function under test.** (F-151,
   F-161d) `rating_before_aggregates_exclude_nulls` name-matches its risk exactly
   and never calls `game_history`. F-161d found two more:
   `classify_inbound_auth_softfail_is_not_fail` and `..._single_fail_is_not_fail`,
   whose inputs each contain an independently passing result, so both name-match
   the risk without exercising it. **Decoy tests are a confirmed class, not
   incidents.**
4. **A finding whose premise is disproved must be AMENDED, not merely closed.**
   (F-205) `dp F12`'s "sentry drags actix-web + ureq into every build" was never
   true; WP-67's own rider 2 required the downgrade be written back into the
   finding, and `SUMMARY.md:44-46,139` and `findings/dependencies.md:103-108,157`
   are unamended to this day.

**Mechanism, not principle:** the sign-off script must, for each closed finding,
(a) `rg` the cited symbol and fail if absent, (b) `rg` for at least one caller of
that symbol outside its own definition and test module, (c) for each row marked
`Test? y`, `rg` the named test and assert the test body mentions the function
under test, and (d) diff the finding text against its status and fail any
`closed` finding whose text still asserts a mechanism the closing commit
disproved.

### 4.2 The "Test? y with no test" sweep

Nine confirmed instances, the most-confirmed pattern in the session (F-142,
F-148, F-149, F-150 - the last being **all seven rows of one WP** - and F-171,
which is the most explicit because the row specified what to assert).

**Mechanism:** grep every `T3-B*` checklist for `Test? y` rows and confirm a test
exists *and* satisfies tooth 3 above.

**Do not conflate "untested by design" with a falsified row.** These are NOT part
of the nine and must not be counted as such:
- WP-76/77/79/80 - `EXECUTION-README.md:408` records the gap as deliberate.
- WP-65's nine checklist rows are all `Test? = n`.
- WP-64/66/67/69/70/73 have no checklist row at all (explicitly deferred,
  BLOCKED-ON-DECISION D-19/D-20/D-23).
- WP-72 appears in no checklist and has no spec.

### 4.3 Self-certifying work packages

**WP-72 has no spec, no checklist row, and a one-line commit message.** A work
package that exists only as a commit cannot be verified by any sign-off
procedure. **Mechanism:** no WP may be marked done without either a spec file or
at least one checklist row; the sign-off script enumerates WP IDs from the commit
range and fails on any with neither.

### 4.4 The routing leak

Findings deferred from one work package to another were treated as closed by the
**sending** package, with nothing tracking whether the receiving package ever
picked them up. Three confirmed cases (F-55, F-57, F-60); two are High. WP-09a
and WP-09b are the most common unfulfilled receiver.

**Mechanism:** a deferral is a state (`routed-to: WP-NN`), never `closed`. The
receiving WP's spec must list its inherited findings, and the sign-off script
fails any finding whose `routed-to` WP closed without naming it.

### 4.5 Tests and docs adjusted to agree with the code (pattern 4b)

Four confirmed instances plus two named variants. This erases the discrepancy the
finding cited and leaves code, tests and docs mutually consistent but all wrong.

- F-72a edited `RULES.md` down to match the code.
- F-83's new test asserts the unchanged value where the spec prescribed the changed one.
- F-79's new test re-hardcodes the legacy values.
- F-95 is the escalation, not sloppiness: WP-35's F1 concurrency test asserts a
  **lower** bound where the spec prescribed an **upper** bound, because the
  prescribed bound was unachievable under the design the same spec mandated - the
  acceptance criterion quietly renegotiated by the implementation.
- **Variant 4f (F-104):** a test that blesses the lenient half of a
  cross-boundary inconsistency. `validate_bot_slots_accepts_case_mismatch` is not
  wrong about its own function; it is wrong about the system.
- **Variant, mirror image (F-120):** instead of a test edited to agree with the
  code, a **new `docs/CODING.md` rule scoped narrowly enough** (three named
  functions) that `end_game` - a fourth unguarded lifecycle writer that rates the
  game - is invisible to the grep procedure the doc itself prescribes.

**Mechanism:** any commit that edits a test assertion, a `RULES.md` or a
`docs/CODING.md` rule **in the same change as the fix it validates** requires a
second reviewer, and the review must answer in writing: "did the code move to the
spec, or the spec to the code?" Any new `docs/CODING.md` rule that prescribes a
grep procedure must state the grep and the reviewer must run it and record the
hit count.

### 4.6 The spec's own STOP-AND-REPORT trigger, answered with a comment (F-206)

WP-69's spec §3b was a **stop-work condition**: stop if `multiple-versions =
"deny"` needed more than "roughly a dozen" skip entries. `e2ee5342` shipped
**29** and wrote *"not papered-over sibling work"* directly above them
(`rust/deny.toml:71-76`). That claim is falsified by `:131`'s own annotation
(`tower-http 0.7.0`, "via web (first-party, pins 0.7.0 directly)", against
`rust/web/Cargo.toml:44`) - the only one of the 29 with a first-party cause, all
29 checked. Compounding: WP-69 §5's "the flip must actually bite" negative checks
are recorded in `EXECUTION-STATE.md` as **parked, never run**.

This is beyond 4b/4c: the criterion was a stop-work condition, not a test.

**Mechanism:** a spec's STOP-AND-REPORT trigger firing is an **escalation to the
owner**, and the only valid resolutions are an owner-signed spec amendment or
abandonment of the step. A code comment asserting the trigger does not apply is
never a resolution. The sign-off script should grep specs for STOP/HALT triggers
and require a recorded owner response for each.

### 4.7 Hand-maintained delivery lists have no CI guard (F-208)

F-208 exposed that the delivery surface is described by **three (four, counting
cross-repo) independently hand-maintained lists** that have already diverged:
- `rust/Cargo.toml` workspace members (28 game crates),
- `rust/Dockerfile` game build stages (**26** at `:174-303`),
- `docker-bake.hcl` targets,
- `k8s/base/game/` Deployments (**43** counted by Unit 10b, against 26 image
  stages - these two lists already disagree).

**Mechanism (a real CI job, not a principle):** a script that derives the game
crate list from `rust/Cargo.toml` and asserts set equality against the Dockerfile
stages, the bake targets and the k8s Deployments, with an explicit allow-list for
intentional absentees (`lords-of-vegas-1`, WIP). It must run in CI on every PR and
fail the build, not warn. See `R-DEL` in section 2.

### 4.8 A "known upstream defects inherited" criterion for any vendoring spec (F-200)

WP-66's spec **did** gate the vendoring correctly (step 0 was binding, the gate
was honoured, the port was minimal and faithful, MIT licence and attribution are
present, the schema is unchanged) - and the cost landed anyway. The "minimal
port, not a rewrite" criterion, **correctly followed**, *guaranteed* an upstream
defect came along. `migrate()` returns `Ok(())` before `create table` and without
committing on the duplicate-key path, and it is the **sole** creator of
`tower_sessions.session` (nothing in `rust/web/migrations/`).

**Mechanism:** any future vendoring spec must carry a mandatory
**"known upstream defects inherited"** section - the vendorer reads the upstream
issue tracker and open PRs, lists what is coming along, and the owner signs that
list. Absence of known defects must be stated explicitly, not left blank.

### 4.9 Other named patterns worth a sweep at sign-off

- **Pattern 2 - inconsistent hardening within a single file.** WP-09 guarded one
  function while its neighbours on the same render path stayed raw-indexed (F-61
  is the clearest case; F-116 is the clean web-half instance - WP-40 added
  `AND NOT $9` to the `left_at` CASE in `update_game_command_success` and left the
  byte-identical sibling in `undo_game` alone). **Mechanism:** when a fix lands in
  one function, grep the file for structurally identical siblings and record the
  hit count in the commit message.
- **Pattern 5 - the `_ => <default>` substitution** (F-65, and now F-136 at High
  severity in the web half). Converting a lookup-with-default into a `match` with
  a catch-all arm satisfies "make this exhaustive so no caller can silently fall
  back" rows **without changing any behaviour**. **Mechanism:** an exhaustiveness
  row is only satisfied by a `match` with no `_` arm.
- **The documentation-only constant** (F-153). `wd F50`'s "one const used by all
  eight sites" shipped as an `#[allow(dead_code)]` string used by **zero** sites,
  with a doc comment stating manual sync is now required. **Mechanism:** sweep
  `rg "allow\(dead_code\)"` across the commit range at sign-off.
- **Nobody checked `Log::public` content.** The programme targeted
  hidden-information leaks, but every fix and every test looked only at
  `pub_state` struct fields. **No game crate tests the log layer.** F-22 (High)
  and F-28 (Medium) survived because of this. See `R-LOG` in section 2.
- **"For every game crate" declared, 3 of 28 delivered.** WP-10 3a was scoped to
  every game crate and applied to three; no later WP swept the rest, and 13 crates
  have no redaction test. **Mechanism:** a scope claim of "every X" requires an
  enumerated list in the spec and a per-item checkbox, never a prose claim.

## 5. Coverage work

Reviews and tests that are **not bug fixes**. Nothing here closes a finding; each
item closes a hole through which the findings in section 2 arrived unseen.

**`01c's checkmarks are epilogue-shape only.`** Unit 01c's `V` marks record that a
crate's epilogue matches WP-08's shape. They confer **no crate-level coverage**
and must never be read as such when scoping the work below.

### 5.1 `roll-through-the-ages-2` - never reviewed at crate level

- **3,290 lines**, no `validate` override, no redaction test.
- The **one function anyone read contained F-83**.
- It was out of scope for a review-of-the-remediation because the crate was
  barely touched by the programme - which is exactly why it has no coverage.
- **Work:** a dedicated crate-level pass on the same terms as Units 02-04:
  `pub_state` field enumeration, `Log::public` content, parallel-vector indexing,
  `validate` override, redaction test.
- **Size: M** - basis: 3,290 lines against the per-crate effort of Units 02-04.

### 5.2 `rust/lib/session_store` - vendored, untested, authentication-adjacent

- No `tests/` directory and no `#[cfg(test)]` module anywhere in the crate.
- It is now **first-party code** in an authentication path, and it is the **sole**
  creator of `tower_sessions.session` (nothing in `rust/web/migrations/` creates
  it).
- **Work:** a test module covering `migrate()` at minimum - including the
  cold-start path that F-200 identifies (returns `Ok(())` before `create table`,
  and does not commit on the duplicate-key path). See `R-VEND`/F-200 in section 2
  for the fix; this item is the coverage that should have caught it.
- **Size: M** - basis: a new test module against a database-backed crate with no
  existing harness to copy.

### 5.3 `require_admin` - true path untested for 13 of 16 server fns

- Only 3 of 16 server functions have a test exercising the **authorised** path.
  The remaining 13 are covered, if at all, only on the rejection path.
- **Work:** extend the admin test module so every server fn has both a
  reject-when-not-admin and a **succeed-when-admin** case. Each test must call the
  server fn itself, not `require_admin` in isolation (tooth 3 of the sign-off
  rule).
- **Size: M** - basis: 13 near-identical tests, blocked on 5.4 for the request
  parts.

### 5.4 No request-parts test harness

- There is no shared harness for constructing the request parts a server fn needs,
  which is the direct cause of 5.3's shape and of several "Test? y" rows having no
  test.
- **Work:** a `test_support`-style helper in `rust/web` that builds authenticated,
  admin and anonymous request contexts.
- **Size: M** - basis: one new module, but it gates 5.3 and several section-2
  packages, so build it first.
- **This is a dependency of 5.3 and of any package whose acceptance criteria
  require a server-fn test.**

### 5.5 The 13 crates with no `validate` override and no redaction test

- **F-06 (High) is the root:** `Gamer::validate` defaults to `Ok(())`, so the D-36
  trust boundary is **fail-open**. 13 of 28 game crates never override it. The
  list is in `00-sweeps.md` - read it, do not re-derive.
- WP-10 3a's redaction test was declared "for every game crate" and applied to
  **3 of 28**; no later WP swept the rest, leaving 13 crates with no redaction
  test.
- **Also: no crate reviewed in this session has a `validate` *test*.** Pattern 2b
  is distinct from F-06: where the override *does* exist it still misses the one
  cross-field invariant that crate's remaining panic depends on (F-66, F-67,
  F-68, F-76). An override with no test is not coverage.
- **Work:** per-crate `validate` override + a `validate` test + a redaction test.
  Track as a checklist with one row per crate (see 4.9's "every X" mechanism) -
  **not** as a single prose claim.
- **Size: L** - basis: 13 crates x three artefacts each, plus the 13 that have an
  override but no test.

### 5.6 The log layer

**No game crate tests `Log::public` content at all.** Every fix and every test in
the programme looked only at `pub_state` struct fields, which is why F-22 (High)
and F-28 (Medium) survived. Covered as `R-LOG` in section 2 because it closes
findings; noted here because the *coverage* half of it - a `Log::public` assertion
in every game crate's redaction test - is coverage work and should be folded into
5.5's per-crate checklist rather than run as a separate sweep.

### 5.7 Unowned items carried forward from unified report section 7

The 18 items below had **no owner at all** at the end of the review. None may be
dropped. Each is given a proposed owner here; the owner column is a proposal, not
a ruling.

| ID | Item | Proposed owner |
|----|------|----------------|
| U1 | `roll-through-the-ages-2` never had a crate-level review - 3,290 lines, no `validate`, no redaction test, and F-83 came from the one function anyone read | **5.1** (dedicated crate pass); F-83's fix in R-50 follows it |
| U2 | `Gamer::points()` has no documented ordering contract; `cathedral-2`'s sign is inverted relative to its own `calc_placings` (`cathedral-2/src/lib.rs:580-584`). No F-number exists anywhere; Unit 08 handed it back | **R-53** |
| U3 | No request-parts test harness - the structural cause of F-92 and the reason F-85 was uncatchable (`rust/web/src/auth/server.rs`) | **5.4**, blocking R-37 and R-38 |
| U4 | `require_admin`'s true path untested for 13 of 16 server fns - wiring proven, authorised behaviour never exercised (WP-37 hole) | **5.3** |
| U5 | Nothing tests the vendored `rust/lib/session_store` - authentication-adjacent, now first-party, and the home of F-200 | **5.2**, blocking R-44 |
| U6 | `left_at` conflates elimination with leaving across four writers; F-113/F-116/F-117 are symptoms. **Carry as ONE schema change, not four findings** | **R-06** |
| U7 | Vendored MIT obligations are hand-satisfied and unverifiable by tooling; a one-line CI grep closes it and `cargo-deny` is the wrong instrument | **R-45**, criterion 3 |
| U8 | The e2e job is `continue-on-error: true`, so it gates no merge and no deploy; the hydration race is untracked and it compounds F-211 | **R-54**, criterion 1 |
| U9 | `auth/server.rs:92` increments `login_emails_sent_total` **before** the send and there is no failure counter; the `wfe F46` shape was fixed outside WP-60's scope | **R-54**, criterion 2 |
| U10 | `rust/bot/src/routing.rs` was never picked up by any sub-unit - no owner at 05b time, and 10a took only `prompt.rs` | **R-55**, criterion 2 |
| U11 | Zero tests on `build_messages`; the bot crate has **no DB tests at all** - its test module covers only `merge_json_patch` and one constant | **5.8** (below) |
| U12 | Nothing asserts that opponent hidden state is **absent** from the rendered bot prompt; the `fetch_game_data` test asserts only the positive - the exact property F-192/F-193 turn on | **5.8** (below), tested as part of R-35 |
| U13 | `rust/operator/src/controller.rs`: `cleanup` has **zero test callers** and there is no already-newest-to-deprecated test; no unit owns operator test coverage | **R-51**, criterion 2 |
| U14 | `i-noreply@brdg.me` is not short-circuited in the inbound router (`inbound.rs:95-96`, `:856-866`) - one wasted token-lookup query per one-way mail; two-line fix | **R-54**, criterion 3 |
| U15 | WP-46's `proposals.rs` test module and `email/sweep.rs`'s test module were never read - only the four spec-named functions were | **R-55**, criterion 3 |
| U16 | `admin.rs:1560-3488`, `db/game_write.rs` and `update_game_command_success` were **never read at line level by any unit** | **R-55** |
| U17 | **WP-36 has neither a spec nor a checklist row**, so Unit 05's crypto verdict rests on a commit message alone | **R-55**, criterion 4 |
| U18 | WP-10 3a's thirteen crates with no redaction test - untouched by Unit 11 and every other unit. **Having a test is not the same as having a sufficient test** | **5.5** |

### 5.8 Bot test coverage (U11, U12)

The bot crate's test module covers only `merge_json_patch` and one constant. It
has **no DB tests at all** and nothing tests `build_messages` (U11). More
pointedly, **nothing asserts that opponent hidden state is absent from the
rendered prompt** - the `fetch_game_data` test asserts only the positive (U12),
which is exactly the property F-192 and F-193 turn on.

- **Work:** a test module for `build_messages` plus an **absence** assertion over
  the rendered prompt: for a two-player game, the acting seat's private state
  appears and the opponent's does not.
- **Size: M** - basis: a new test module in a crate with no test infrastructure.
- **Sequence with R-35**, which fixes the leak this coverage would have caught.


## 6. Owner decisions required before work starts

### 6.1 Vendoring policy - RESOLVED (2026-07-31)

The owner ruled on 2026-07-31 that vendoring third-party code is **forbidden
except where there is no alternative and the work is completely blocked**. The
ruling is recorded in `97-REMEDIATION-PROGRESS.md`; R-VEND may inventory the
existing code under that ruling, but it must not make a new vendoring decision.

What the review has already established - **do not re-derive**:

- **WP-66's spec did gate it.** Step 0 was binding: "Bump ... and re-resolve
  before designing anything. If that alone puts every crate on one sqlx major,
  this spec collapses to section 3a and you are done - do NOT vendor anything.
  Only if no sqlx-0.9-compatible store release exists does 3b apply."
- **The gate was honoured.** `tower-sessions-sqlx-store` 0.15.0 pins
  `sqlx = "0.8.0"` upstream, so branch 3b was correctly live.
- **The port was faithful.** Minimal, verified by direct diff against the registry
  copy; MIT licence and attribution present; schema unchanged.
- **The cost landed anyway: F-200.** The correctly-followed "minimal port, not a
  rewrite" criterion guaranteed an upstream defect came along, and it is now
  first-party code in an authentication-adjacent path with **no tests** -
  `rust/lib/session_store` has no `tests/` and no `#[cfg(test)]` module.
- **The MIT obligation is never machine-checked.** `rust/deny.toml:45-49` sets
  `[licenses.private] ignore = true`, and because the crate is `publish = false`
  cargo-deny skips it entirely. Unit 10c established that this is **correct
  config** and that **no** cargo-deny setting could machine-check the vendored MIT
  obligations - so the obligation is satisfied by hand, permanently.

**Ruling:** "no compatible upstream release yet" is not sufficient by itself.
Vendoring is permitted only where there is no alternative and work is completely
blocked. The ruling becomes a rule in `docs/CODING.md` and a mandatory section in
every future dependency spec (see 4.8).

**Not a decision but a consequence:** **the scope of vendoring across the repo has
never been swept.** Only `session_store` is known. That sweep is itself a work
package - `R-VEND` in section 2 - and it should run regardless of which way the
policy lands.

### 6.2 Unit 07b scoping - OPEN, blocks nothing but leaves a coverage claim unproven

Unified report section 8.12 records one unresolved scoping question: **did Unit
07b cover the whole WP-51 / WP-53 surface** after its quota-death re-dispatch?
07b died before producing any finding and was re-dispatched from scratch; the
re-dispatch produced F-144..F-149, but whether it re-walked the full surface of
both commits was never confirmed.

Resolving it means re-walking `dcd8844c` (WP-51) and `3610b957` (WP-53). This is
proposed as a small verification work package (`R-VER` in section 2), not a
blocker. The owner's decision is only whether it is worth the pass.

### 6.3 Second-order decisions the plan surfaces but does not make

- **`ALLOW_INSECURE_DEFAULT_KEY` split** (section 3) - one flag currently disables
  two unrelated guards. Suggested, not urgent.
- **The never-implemented second half of ws F55** (F-109): bot consumer and email
  sweep tasks get no shutdown signal. F-109's remediation is a **bookkeeping fix
  on WP-36's row plus a decision on this second half - NOT a revert of
  `efad81f9`.** WP-84's spec §3g anticipated the deletion and required
  a proof test which does exist. The owner decides whether the second half ships.
- **F-203/F-204** are WP-64 spec-vs-code gaps, not regressions, and §3b of the
  same spec endorses exactly the bare-major spelling rider 1 forbids - **the
  criterion is internally inconsistent** and needs an owner ruling on which half
  stands before anyone "fixes" it.

## 7. Explicitly out of scope

Do not re-open any of these. Each has a recorded ruling or disposition.

| Item | Disposition |
|------|-------------|
| **F-81** | **Owner ruling, 2026-07-30: not a finding.** Reconstructing hidden information by inference from the public log is acceptable **by design** - a great deal of it is reconstructible, this is equivalent to reconstructing it from memory, and brdgme does not intend to defend against it via ephemeral logging or any similar mechanism. The ruling is **general**, not specific to `no-thanks-2`. It does **not** excuse hidden information appearing *directly* in `Log::public` content - **F-22 and F-28 remain valid findings** and are in scope (see `R-LOG`). The distinction for any future finding: direct leak = finding, inferable = not a finding. |
| **F-50, F-57, F-59** | **`lords-of-vegas-1` is work in progress** (owner ruling). Its missing endgame - it never assigns `finished = true` - is out of scope, as is any finding about missing or incomplete functionality in that crate. Marked "WIP crate, excluded"; not routed to remediation. It is also the second legitimate absentee from `rust/Dockerfile`'s game stages. |
| **The F-35 family** (`Status::Finished { stats: vec![] }`, 24 sites across 21 crates) | **Parked in WP-20 (`c F12`)** by owner ruling. Occurrences are recorded; do not demand fixes and do not re-raise per crate. |
| **F-110** | **Not a defect.** |
| **F-123** | **Refuted and archived.** |
| **F-208a** | Out of scope. (F-208 proper - the hand-maintained delivery lists - **is** in scope; see 4.7 and `R-DEL`.) |
| **04b's `F-78`** | **Void** - a numbering collision, not a finding. The live F-78 belongs to Unit 04c. |

Additionally, the following were **refuted during the review** and must not be
re-derived as work: the `VisibilityCache` cross-user leak; `prompt.rs` as a leak
vector (F-192/F-193 were found adjacent to it instead, and those **are** in
scope); the `ssr` feature-gate question (`scripts/rust-ci-commands.sh:30` runs
`cargo test -p web --features ssr` and CI runs the same script, so **no "Test? y"
row is retro-voided**); the sqlx-cache carry-forward (causality inverted - WP-52
is an *ancestor* of WP-66); the `test_support` feature risk (28/28 consumers are
`[dev-dependencies]`, not in `default`, correctly `#[cfg]`-gated); WP-73's deleted
`*_repl` binaries (a capability **move** to `rust/tools/repl`, not a loss); and
`serde_yaml_ng`'s fidelity as a fork.

## 8. Summary table

Section 11 records **no Critical rows**. "Ceiling" is the highest severity in the
package as section 11 writes it; **ATO** marks the two Mediums that 00-STATE
escalates to account takeover.

| R | Objective | Ceiling | F-count | Size |
|---|-----------|---------|---------|------|
| R-01 | Close the inbound-email auth gate (fail-open three ways) | High/ATO | 1 (+4 sub) | M |
| R-02 | Scope and expire the settings-email token | ATO | 2 | M |
| R-03 | Canonicalize bot names in `validate_bot_slots` | High | 5 | M |
| R-04 | `restart_core` must validate bot slots | High | 1 | S |
| R-05 | Concede-and-replace transaction integrity | High | 5 | L |
| R-06 | One `left_at` / lifecycle-writer change | High | 6 | L |
| R-07 | `CanonicalEmail` newtype | High | 6 | L |
| R-08 | Transient errors must not be classified permanent | High | 2 | S |
| R-09 | One `RouteOutcome` contract | High | 2 | M |
| R-10 | SSE authorization lifetime and task hygiene | High | 5 | M |
| R-11 | Shutdown drain: bookkeeping, then a decision | High | 1 | S (+unsized half) |
| R-12 | `logout_everywhere` must not report success on failure | High | 1 | S |
| R-13 | Bot crypto: remove ungated dev-key fallback | High | 3 (+1 unnumbered) | M |
| R-14 | Share the NATS wire protocol | Medium | 2 | M |
| R-15 | NATS delivery semantics | Medium | 4 | M |
| R-16 | CI guard for the delivery lists (`R-DEL`) | High | 2 | M |
| R-17 | Stats query correctness | High | 7 | M |
| R-18 | No network calls inside a database transaction | High | 3 | M |
| R-19 | Invite nudge dedup | High | 2 | M |
| R-20 | Notification identity, threading and duplication | Medium | 5 | M |
| R-21 | Close `Gamer::validate` at the trait | High | 1 | S (blocked by R-22..R-26, R-48) |
| R-22 | `texas-holdem-2` state validation | High | 3 | M |
| R-23 | `lost-cities-1` / `-2` parity | High | 4 | M |
| R-24 | `sushi-go-2`: F-06's row, the catch-all, the false-premise panic | High | 3 | M |
| R-25 | The nine remaining crates with no `validate` override | Medium | 9 | L |
| R-26 | `validate` exists but misses the invariant (pattern 2b) | Medium | 5 | M |
| R-27 | `for-sale-2` deadlock and short-deck stall | High | 2 | S |
| R-28 | `rand_bot` spec handling | High | 2 | M |
| R-29 | `lib/cmd` panic paths and envelope handling | High | 3 | M |
| R-30 | Hidden information in `Log::public` (`R-LOG`) | High | 8 | L |
| R-31 | `category-5-2` player count | High | 3 | M |
| R-32 | Epilogue gate sweep | Medium | 7 | M |
| R-33 | `acquire-1` correctness | Medium | 5 | M |
| R-34 | `alhambra-1` scoring order | Low | 2 | S |
| R-35 | `game_client`: stop shipping every seat's private state | Medium | 6 | M |
| R-36 | Bot leak surface and startup robustness | Low | 4 | M |
| R-37 | Web auth hardening (incl. the missing rate limiter) | Medium | 9 | L |
| R-38 | Admin surface and db module | Medium | 6 | M |
| R-39 | Visibility, bounds and untested guards | Low | 6 | M |
| R-40 | Import path | Medium | 3 | S |
| R-41 | Email rendering, escaping, preference mapping | Medium | 9 | M |
| R-42 | Frontend, theme and colour | Low | 5 (+F-15 latent) | M |
| R-43 | Enforce the `bans` section (`deny.toml`) | Medium | 2 (+10b gap 3) | M |
| R-44 | Fix the vendored session store | Medium | 3 | M |
| R-45 | Repo-wide vendored-code sweep (`R-VEND`) | n/a | 0 | S (+unsized findings) |
| R-46 | Workspace lint configuration | Low | 3 | S |
| R-47 | Amend the finding corpus (tooth 4) | Low | 1 | S |
| R-48 | `hanamikoji-1` validate and epilogue | Medium | 1 | M |
| R-49 | `lib/markup` and the command parser | Medium | 6 | M |
| R-50 | Remaining game-crate low findings | Medium | 9 | M |
| R-51 | Operator and game-type version lifecycle | Medium | 2 | M |
| R-52 | Verify Unit 07b's WP-51/WP-53 coverage (`R-VER`) | n/a | 0 | S |
| R-53 | `Gamer::points()` ordering contract | n/a (U2) | 0 | S (+unsized sweep) |
| R-54 | Small unowned fixes (e2e gate, metric, `i-noreply`) | n/a (U8, U9, U14) | 0 | S |
| R-55 | Read the surfaces nobody read | n/a (U10, U15-U17) | 0 | L |

**Totals: 55 work packages** - **14 S**, **34 M**, **7 L**.

Plus **six coverage items** in section 5 (5.1 M, 5.2 M, 5.3 M, 5.4 M, 5.5 L,
5.8 M), **five deployment items** in section 3, and **nine process fixes** in
section 4.

**Critical path.** R-01 and R-02 ship together and ship first. R-04 waits on
R-03; R-06 sequences after R-05 (same file); R-15 after R-14; R-21 closes the
game-crate family after R-22..R-26 and R-48; R-37 and R-38 wait on the request-
parts harness (5.4); R-44 waits on 5.2. R-46 is blocked on owner decision 6.3
and R-52 on 6.2; R-45 should start immediately regardless of 6.1, because its
output is what 6.1 needs to be decided on.

**Unresolved, recorded rather than guessed.** Three sizings in this plan are not
confident and say so at the package: R-25 (nine crates whose per-crate effort
varies by an order of magnitude - re-size per crate), R-11's second half (the
never-implemented half of ws F55, unsized pending owner decision 6.3), and R-45
(the sweep is small; whatever it finds is not yet knowable). One open question
survives the review itself: **F-59's status**, since the `lords-of-vegas-1` WIP
ruling names only F-50 and F-57. Section 11 records it as "ambiguous coverage,
unresolved". Treat it as excluded until the owner says otherwise, but do not
record it as settled.
