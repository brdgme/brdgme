# Unified report - review of the 2026-07-25..2026-07-30 remediation effort

**Status: uncommitted. 2026-07-31.**

Sources: the 22 unit reports of this session, normalized into
`90-findings-part1.md`, `90-findings-part2.md` and `90-findings-part3.md`, plus
`00-STATE.md` (authoritative for ground rules, owner rulings and
carry-forwards). The remediation work breakdown is a separate document.

---

## 1. Executive summary

**What was reviewed.** A 127-commit remediation programme run 2026-07-25 to
2026-07-30, which implemented fixes for the 570-finding corpus in
`docs/reviews/2026-07-23-rust-review/SUMMARY.md`. This session reviewed **the
remediation work itself** - not the original code - across 22 units: did each
fix actually close the finding it claimed to, and what did it break or leave
open on the way.

**What was found.** 211 findings, F-01..F-211, across 219 table rows (219 > 211
because sub-lettered rows and two unnumbered rows from the F-96 investigation
carry their own entries). Normalized distribution: **32 High, 83 Medium, 92
Low**, plus 2 informational/nit, 1 downgraded to a deployment blocker, 1
refuted, 3 unstated and 5 sub-letter rows folded into their parents. No
Critical was assigned; F-161 is High and is functionally the Critical of this
session.

**The headline judgement.** The programme did real work and closed real
findings, but its verification layer cannot be trusted. The recurring shape -
present in every high-value finding in this session - is a commit that satisfies
its checklist row **literally** while breaking or missing what the row was for.
Commit messages and checklists read clean in all of those cases. Reasoning from
them alone produced false findings; only reading the end state produced true
ones.

**The single worst thing, on the defect side.** F-161: WP-56's inbound email
authentication gate is fail-open three independent ways. Combined with the
settings token's absence of expiry, single-use and rate limiting, spoofing a
`From:` header is **account takeover**. Unit 07 set this escalation condition
explicitly when it filed F-129 and F-130; Unit 09a confirmed the condition
fired. This is the top of the remediation order and nothing else in the session
is close.

**The single worst thing, systemically.** Nine checklist rows marked `Test? y`
have no corresponding test (section 5 explains the count). A checklist that
asserts tests exist which do not exist is worse than no checklist, because
sign-off consumed it as evidence. Everything else - the routing leak, the
reverted fix, the tests edited to agree with the code - is a variant of the same
root cause: the acceptance artifact was checked, the end state was not.

**What the programme genuinely achieved.** Substantial, and it should not be
lost in the above. Roughly fifteen candidate findings were **refuted** by this
session with concrete evidence, several of them the most valuable output of
their unit (section 6). WP-73's binary consolidation was verified good by
exhaustive proof rather than sampling - all 108 pre-commit game bins normalise
to exactly four distinct contents. WP-66's vendoring gate was honoured
correctly and the port was faithful and minimal, verified by direct diff against
the registry copy. WP-64's four suspected regressions all proved negative. And
the most telling result of the session is an inversion: **`hanamikoji-1`, the
crate written *after* the review, internalised the review's lessons better than
most of the crates the review remediated** - it is the first crate in the
session with a `validate` test, its epilogue is correctly guarded with a
dedicated regression test, and its `Status::Finished` populates `stats`. Both of
its gaps (F-208, F-209) are in areas no checklist covered. The lesson landed;
the checklists did not carry it.

---

## 2. Severity distribution and how to read it

### 2.1 Raw counts, exactly as the three parts recorded them

| Bucket (verbatim) | Part 1 | Part 2 | Part 3 | Total |
|---|---|---|---|---|
| Critical | 0 | - | - | 0 |
| High | 9 | 14 | 7 | 30 |
| Medium | 30 | 32 | 19 | 81 |
| Low | 43 | 17 | 28 | 88 |
| `Low/Medium` | - | 2 | - | 2 |
| `Low, note` | - | 2 | - | 2 |
| `Low, informational` | - | 2 | - | 2 |
| `High/DOWNGRADED` | - | 1 | - | 1 |
| `Medium/ESCALATED` | - | 2 | - | 2 |
| `informational` | 1 | - | - | 1 |
| `Nit` | 1 | - | - | 1 |
| REFUTED | - | 1 | - | 1 |
| unstated | 1 | 2 | 5 | 8 |
| **Rows** | **85** | **75** | **59** | **219** |

Row counts were verified mechanically (`rg -c '^\| (F-|\()'`) against each
part's own claimed total; all three match.

### 2.2 The normalization rule applied

Part 2 used five qualified-severity buckets and part 1 three unqualified extras.
They are normalized as follows, and this rule is stated so the numbers can be
re-derived:

1. **A slashed pair collapses to the higher severity.** `Low/Medium` (2 rows)
   becomes Medium. Rationale: the qualification records reviewer uncertainty,
   and under-rating is the more costly error.
2. **A trailing qualifier is decoration, not severity.** `Low, note` (2) and
   `Low, informational` (2) become Low. The qualifier is preserved in the status
   column of the full table.
3. **An explicit `00-STATE.md` ruling overrides the report's rating.**
   - `Medium/ESCALATED` (F-129, F-130) becomes **High**. The escalation
     condition Unit 07 set fired via F-161; these are account-takeover
     components, not Mediums.
   - `High/DOWNGRADED` (F-96) leaves the severity scale entirely and becomes a
     **pre-rollout deployment blocker**, counted in its own bucket. It is not a
     code defect (section 9).
4. **`informational` and `Nit` are kept as one non-severity bucket** (F-58,
   F-77). Folding them into Low would overstate remediable Low count.
5. **`unstated` is never invented.** Part 3's five unstated rows are
   sub-letters (F-161a/b/c/d, F-208a) that inherit their parent's severity and
   are shown folded; part 1's one (F-27) is withdrawn; part 2's two are the
   unnumbered rows from the F-96 investigation.
6. **REFUTED is a status, not a severity** (F-123), and is counted separately.

### 2.3 Normalized counts

| Normalized bucket | Count |
|---|---|
| **High** | **32** |
| **Medium** | **83** |
| **Low** | **92** |
| Informational / Nit | 2 |
| Deployment blocker (downgraded out of the severity scale) | 1 |
| REFUTED | 1 |
| Unstated (1 withdrawn, 2 unnumbered) | 3 |
| Sub-letter rows folded into their parent | 5 |
| **Total rows** | **219** |
| **Distinct finding IDs** | **211** |

Two further reading notes:

- Part 1 recorded **net remediable rows: 78** of its 85, after removing
  withdrawn, refuted and owner-excluded rows. No equivalent net figure was
  computed for parts 2 and 3; do not assume 219 is a work-item count.
- The web half produced denser and more severe findings than the game half.
  Part 1 (F-01..F-84, the whole game half plus core libraries) contains 9 Highs
  across 85 rows; parts 2 and 3 (the web server, bot, operator, tooling and
  dependency halves) contain 21 across 134.

---
## 3. The most severe findings

32 findings normalize to High. F-161 is first because it is not merely High -
it is the finding that turns two other findings into account takeover.

### F-161 (Unit 09a, WP-56) - the inbound auth gate is fail-open three ways

`rust/web/src/email/inbound.rs:164-219` (+4 sites).

WP-56's inbound email authentication gate does not authenticate the `From`
header. It is fail-open three independent ways:

- **F-161b, the cleanest:** `Pass` means "not explicitly failed". The predicate
  is `failed(dmarc) || (failed(spf) && failed(dkim))`, which **inverts the DMARC
  rule** by requiring SPF *and* DKIM to both say "fail". `spf=fail; dkim=none`
  is therefore accepted, as are `dmarc=none`, `spf=softfail; dkim=none`, and
  `spf=neutral/none/permerror/temperror`. This row is derivable from the file
  alone with no deployment assumption - unconditional forgery.
- **F-161a:** `AuthVerdict::Unknown` proceeds on a `warn!` only, and `Unknown`
  is returned whenever the authserv-id is not exactly `amazonses.com`. **The
  production pipeline is Resend, not SES**, so a different authserv-id makes the
  entire gate inert in production. There is no test against a captured real
  message and no metric or alert on `Unknown`.
- **F-161d:** the two tests that name the risk -
  `classify_inbound_auth_softfail_is_not_fail` and `..._single_fail_is_not_fail`
  - each supply an input containing an independently passing result, so they
  name-match the risk without exercising it. Decoy tests sitting directly on top
  of the session's worst defect.

**Why this is the top of the remediation order.** Unit 07 filed F-129 (the
`s-{token}@brdg.me` settings token has no expiry, no rotation, no revocation and
is never NULLed on use, logout or email removal - an archived settings email is
a permanent live bearer credential) and F-130 (that token is not scoped to
settings; a holder reaching `dispatch_standalone_server_command` also gets
`new`, `bump` and subscribe/unsubscribe). Unit 07 rated both Medium **and stated
the escalation condition explicitly**: if Unit 09 weakened
`from_matches_verified_email` or its DMARC classification, F-129 + F-130
escalate to account takeover. Unit 09a confirmed the condition fired. Spoofing
a `From:` header is account takeover. **F-129 and F-130's in-report Medium
ratings are superseded and both are counted High in this report.**

Note the internal inconsistency this exposes: `unsubscribe_token` rotates
(`email/unsubscribe.rs:99`) and invite tokens rotate
(`proposals.rs:936-944`); the settings token, the most powerful of the three,
does not.

### The remaining High findings

**F-05 (01a, WP-09a)** - `rust/game/for-sale-2/src/lib.rs:130-135`. The
underflow guard returns an empty log vector, leaving a short-deck game reporting
`Active` with no legal move: a permanent wedge, **worse than the panic it
replaced**. `for-sale-2`'s WP-09b `validate()` (`:389-414`) checks only
per-player vectors, so the silent guard is the sole defence.

**F-06 (01a)** - `rust/lib/game/src/game.rs:106-108`. `Gamer::validate` defaults
to `Ok(())`, so the D-36 "deserialized state is not trusted" boundary is
fail-open for the 13 of 28 crates that never override it. The corrected
denominator is in section 5.

**F-09 (01b, WP-07)** - `rust/lib/rand_bot/src/lib.rs:33`. `Spec::Int` with
`min > max` panics and `Many { min: Some(5), max: None }` trips
`assert!(min <= max)` - exactly the degenerate-spec class WP-07 claimed to fix,
and wire-reachable. `lib/game`'s own `Many` was changed in WP-03/04 to degrade
gracefully, so library and bot now **disagree**. `rand_bot`'s `fuzz`/`Fuzzer`
path also reaches these.

**F-17 (01b, WP-06)** - `rust/lib/cmd/src/repl.rs:210` (+7 sites). WP-02's
deferred "CLI REPL will panic" fix did not land: all 18 markup/IO/response paths
still `unwrap`/`expect`/`panic!`, **including panicking on a normal
`Response::UserError`**. Paired with F-01: WP-02 and WP-06 together moved the
failure mode from partial render to panic.

**F-22 (02, WP-10/WP-14)** - `rust/game/alhambra-1/src/lib.rs:160-181`.
`start_game` emits each player's exact opening money-card draw as a
`Log::public`, so the whole hand is reconstructible despite `PubBoard.card_count`
claiming the cards are private. Pre-existing rather than a WP-14 regression, but
it survived the entire programme because the redaction test only greps
`PubState`. The F-81 owner ruling explicitly does **not** excuse it: this is a
direct leak.

**F-36 (03a, WP-09b)** - `rust/game/texas-holdem-2/src/lib.rs:663-814`. No
`validate()` override, so seven parallel per-player vectors sized only in
`new_hand()` are raw-indexed and a short deserialized state panics inside
`status()`/`player_state`. The crate where the missing override matters most.

**F-60 (04a, WP-28 -> WP-09a/09b)** - `rust/game/lost-cities-1/src/lib.rs:545-559`
(+6 sites). `player_state()` indexes `self.hands[player]` raw **with an in-code
comment routing the bounds fix to WP-09**, which never delivered. No
`validate()`, so a persisted short `hands` panics the render path for every
viewer. WP-09a's `check_player` is vacuous here because `player_count()` returns
the `PLAYERS` constant, not `hands.len()`. The canonical routing-leak case.

**F-61 (04a, WP-09a/09b)** - `rust/game/sushi-go-2/src/lib.rs:798-818` (+6
sites). `player_state()` got three length guards while `self.playing[DUMMY]`
**inside the same function** and five further render-path indexes stayed raw,
with no `validate()` override. The clearest instance of the session's most
productive pattern.

**F-72a (04b, WP-32)** - `rust/game/category-5-2/RULES.md:3`. The WP-32 commit
meant to raise `MAX_PLAYERS` instead edited user-facing `RULES.md` from "2-10"
to "2-8" to match the code, erasing the discrepancy the finding cited.
`RULES.md` is served through `Gamer::rules()`, so this is a published claim.
Must be fixed as one item with F-72.

**F-85 (05a, WP-35)** - `rust/web/src/auth/server.rs:590-612`.
`logout_everywhere` returns `Ok(true)` **without deleting any auth-token rows**
when `get_user_from_session` collapses a session-store error to `None`. Same
root cause as F-86; `logout` shares the shape. Unverifiable by existing tests.

**F-104 (05b, WP-38)** - `rust/web/src/db/bots.rs:57-71` (+5 sites).
`validate_bot_slots` matches `bot_name` case-insensitively and stores it
verbatim while every consumer resolves it case-sensitively, so `"EASY"` creates
a permanently wedged game the WP-38 sweep refuses to rescue. **One defect
spanning four units - remediate as a single item with F-138, F-183 and F-189.**

**F-109 (05b, WP-36)** - `rust/web/src/websocket.rs:78-80`. `efad81f` deleted
WP-36's ws F55 shutdown drain (`TaskTracker`, `drain_ws_tasks`, bounded 5s wait)
and its regression test `rust/web/tests/websocket_hygiene.rs` **together**. The
checklist row and both commits still read as closed. Remediation is a
bookkeeping fix on WP-36's row plus a decision on the never-implemented second
half of ws F55 - **not** a revert of `efad81f9`.

**F-111 (06, WP-40)** - `rust/web/src/db/game_write.rs:394-399`.
`concede_game_replace` calls `pick_replacement_bot` on the pool **before**
`pool.begin()`, so every rejected concede commits an orphan `game_bots` row.
`UNIQUE (game_id, name)` with no `ON CONFLICT` makes retry fail permanently as a
redacted internal error.

**F-112 (06, WP-40)** - `rust/web/src/db/game_write.rs:387-426`.
`concede_game_replace` never updates the `games` row, so its `updated_at` claim
never fails on replay: a duplicated concede swaps in a second bot and writes a
second public log line. `undo_game` got the equivalent in-transaction re-verify;
this did not.

**F-116 (06, WP-40)** - `rust/web/src/db/game_write.rs:584-598`. `undo_game`'s
`left_at` CASE has no arm for un-elimination, so an undone elimination
permanently marks the player a leaver and `compute_ranked_placings` rates them
last. WP-40 added `AND NOT $9` to the byte-identical sibling CASE in
`update_game_command_success` and left this copy alone. Nothing anywhere sets
`left_at` back to NULL.

**F-119 (06, WP-40)** - `rust/web/src/db/game_write.rs:401-410`.
`concede_game_replace` clears `is_turn` unconditionally without reassigning the
turn, so conceding on your own turn wedges the game: `find_bot_turns` returns
zero rows and the replacement bot never plays. **Open dependency:** the severity
assumes WP-38's wedge-recovery sweep gates on `is_turn`. If it does, F-119 has
**no production mitigation**. This cross-check was Unit 06's one unresolved
dependency and is recorded as open in section 7.

**F-124 (07, WP-50)** - `rust/web/src/proposals.rs:1730-1786`.
`add_proposal_player` passes `email` raw to `find_or_create_user_by_email_tx`
and `check_invite_policy_tx` - no canonicalize, no empty/`@` check - so
`" foo@x.com "` **mints a verified ghost account** and bypasses D7
block-by-target and `invite_policy`. WP-50's spec 3c enumerated only
`create_proposal` and `restart_game_with_roster`. The unit's canonical
checklist-satisfied-literally instance.

**F-129, F-130 (07)** - escalated to High; described under F-161 above.

**F-134 (07, WP-79)** - `rust/web/src/proposals.rs:1702-1709`. `start_proposal`
makes a `reqwest` call to the game service **while holding the
`lock_proposal_for_update` `FOR UPDATE` row lock**, so a hung game service blocks
every concurrent respond/cancel/transfer/nudge. Hoisting exactly this call is
WP-79's whole point; it was done in `create_proposal` and `restart_core`, not
here.

**F-135 (07, WP-79)** - `rust/web/src/email/inbound.rs:1021-1034`. **The WP-79
commit itself** (`91c723d4`) inserted `fetch_game_from_service` after the
`begin()` and after the lock - the refactor moved the call out of
`start_proposal_tx` and landed it on the wrong side of `begin()`. The sharpest
checklist-satisfied-literally instance in the unit.

**F-136 (07, WP-46)** - `rust/web/src/email/sweep.rs:135-137`. The `_ =>`
catch-all swallows `Err(_)` as `PermanentSkip`, which `sweep_once` treats
identically to `Sent` - so one transient DB error commits
`mark_reminder_sent_tx` and the reminder is **never sent**. This reintroduces
the exact mark-without-send WP-46 exists to remove; the spec says errors are
`Retry`. Fix in the same change as F-145.

**F-137 (07, WP-45)** - `rust/web/src/game/server_fns.rs:1087`. `restart_core`
takes client-supplied `bot_slots` and never calls `validate_bot_slots`, so a
restart carrying `bot_name: "garbage"` reaches `insert_game_from_service` and
creates a wedged game. WP-45's spec section 1 names `restart_core` as one of the
three call sites; `rg validate_bot_slots` has zero hits in the file.

**F-144 (07b, WP-46)** - `rust/web/src/email/sweep.rs:507-519`. The invite-nudge
dedup key `gp.nudged_at` is per-proposal while sends are per-invitee, so one
web-suppressed invitee blocks the mark and **re-nudges the whole roster every
tick** - roughly 1,344 duplicate emails per invitee over the 14-day expiry at
the 900s interval. Not WP-51's code: WP-51 introduced none of F-144/145/146.

**F-151 (08, WP-52)** - `rust/web/src/stats/queries.rs:104-152`. `wd F48`'s
game-type name filter is applied only inside the `qualifying` CTE and not to the
`gtu` side of a FULL OUTER JOIN, so `.next()` on the alphabetically-ordered
result returns **another game type's rating and record**. Public unauthenticated
endpoint; silent wrong data for any player rated in more than one game type. The
`wd F48` test that F-150 records as missing would have caught it.

**F-158 (09a)** - `rust/web/src/events.rs:33-41`. SSE resolves the viewer once
at connect and never re-validates, so a revoked session keeps streaming private
events indefinitely. The visibility-staleness half is bounded by the ~30s
`VisibilityCache` TTL and accepted; **session revocation is unbounded**.

**F-169 (09b, WP-57 §3b)** - `rust/web/src/email/inbound.rs:1392-1433`. The
at-least-once `Retry` fix landed on the game and invite routes but not settings;
`handle_settings_reply` returns `()`, so transient DB errors silently discard the
command. Pairs with F-162. The `RouteOutcome` sweep is settled - no third route
has the defect.

**F-183 (09c)** - `rust/web/src/email/commands.rs:82-93`. Email `new` lowercases
the bot name into `game_bots.bot_name` while the bot service looks it up
case-sensitively, so the bot never moves and the game wedges silently. Part of
the four-unit bot-name item.

**F-186 (10a)** - `rust/bot/src/crypto.rs:66-70`. The bot **silently falls back
to the hardcoded dev encryption key** when `DATABASE_ENCRYPTION_KEY` is unset -
no opt-in gate, no `MissingKey` variant. This is the forbidden "dev default plus
warn" pattern that `docs/CODING.md:701` explicitly prohibits, and the web copy
was fixed while this one was not. Remediate as one item with F-187 and F-188.

**F-189 (10a)** - `rust/bot/src/config.rs:26-29`. The case-sensitive
`WHERE name = $1` bot lookup misses, and the miss path returns `Ok(())` -
**acking and discarding the turn**. The sibling "no providers" path returns
`Err` and is retried, so the wrong-case path is the one that fails silently. Adds
a second, previously uncited site at `:67`. Fourth unit of the bot-name item.

**F-208 (11)** - `rust/Dockerfile:36`. `hanamikoji-1` is a workspace member
compiled by every image build and then never copied out: no Dockerfile stage, no
docker-bake target, no k8s Deployment. **It is unshippable**, and there is no
build- or deploy-time signal saying so. No commit since 2026-07-20 touches
`rust/Dockerfile` or `docker-bake.hcl`, so the 127-commit window could never have
caught it.

---
## 4. Systemic patterns

`00-STATE.md`'s pattern list grew organically and its numbering interleaved
(4b, 4c, 4e, 4f, 4d). It is renumbered and consolidated here into a single
sequence, ordered by how much the pattern actually produced. Every pattern
cites its confirmed instances by F-number.

### P1. Inconsistent hardening within a single file

The single most productive pattern of the session, promoted to the top of the
list. A work package hardens one function and leaves its byte-identical or
same-path sibling raw.

- F-61 - WP-09 guarded one render function while its neighbours on the same
  render path stayed raw-indexed. The clearest case.
- F-116 - WP-40 added `AND NOT $9` to the `left_at` CASE in
  `update_game_command_success` and left the byte-identical sibling in
  `undo_game` untouched.
- F-90 - crypto fixes landed only in `rust/web/src/crypto.rs`; the divergent
  duplicate `rust/bot/src/crypto.rs` never received them.
- F-108 - `rust/bot/src/nats.rs` vs `rust/web/src/nats.rs`, the same duplicate
  shape, not yet diverged and with no round-trip test.
- Unit 09's `events_public_handler` (`rust/web/src/events.rs:117-183`) runs an
  uncached `is_game_publicly_visible` query per message while the authenticated
  handler beside it uses `VisibilityCache`.
- Supporting: `check_player` (`rust/lib/cmd/src/requester/gamer.rs:24-36`)
  bounds the player index against `player_count()`, not the actual vector
  lengths, so it gives no protection against short parallel vectors.

### P2. "Test? y" checklist rows with no test

Promoted to top level. A checklist row asserts a test was written; no such test
exists. **Confirmed tally: nine falsified rows** - see section 5 for why the
count is nine and not six.

- F-142, F-148, F-149 (Units 07/07b)
- F-150 - all seven rows of one work package
- F-171 (Unit 09b) - the most explicit case; the row specified what to assert
- F-176 - **one ID covering four falsified rows** (Unit 09c)

### P3. `validate` overrides that miss the invariant the panic depends on

The `validate` overrides added by the programme cover the parallel-vector sweep
but miss the one cross-field invariant each crate's remaining panic actually
relies on.

- F-66, F-67, F-68, F-76 - the original four.
- F-209 - `hanamikoji-1`'s `validate` bounds every parallel vector and seat
  index but never relates `phase` to `pending`; `OpponentChoose` with
  `pending: None` passes validate, reports `Status::Active` naming a player to
  move, and returns `Err` for both seats forever.

For most of the session no reviewed crate had a `validate` test at all, which
is why these got through. `hanamikoji-1` broke that (`:1079`) and is the model
to copy - and F-209 proves that having one is not sufficient.

### P4. Nobody checked `Log::public` content

The programme targeted hidden-information leaks but every fix and every test
looked only at `pub_state` struct fields. No game crate tests the log layer.

- F-22 (High), F-28 (Medium) - leaks that survived precisely because of this.

Boundary set by owner ruling: reconstructing hidden information from public
logs by inference is acceptable by design (F-81). Direct leaks into
`Log::public` remain findings.

### P5. The routing leak

Findings deferred from one work package to another were treated as closed by
the *sending* package, with nothing tracking whether the receiving package
picked them up. WP-09a/09b is the most common unfulfilled receiver.

- F-55, F-57, F-60 - three confirmed cases. See section 8 for the severity
  resolution.

### P6. Tests and docs edited to agree with the code

Rather than the code being fixed. This erases the discrepancy the finding cited
and leaves code, tests and docs mutually consistent but all wrong.

- F-72a - `RULES.md` edited down to match the code.
- F-83 - the new test asserts the unchanged value where the spec prescribed the
  changed one.
- F-79 - the new test re-hardcodes the legacy values.
- F-95 - the WP-35 F1 concurrency test asserts a lower bound where the spec
  prescribed an upper bound, because the prescribed bound was unachievable
  under the design the same spec mandated. This is the escalation: not
  sloppiness, but the acceptance criterion being quietly renegotiated by the
  implementation.

Two named variants:

- **P6a - a test that blesses the lenient half of a cross-boundary
  inconsistency** (F-104, High): `validate_bot_slots_accepts_case_mismatch`
  pins case-insensitive bot-name validation as intended, while all four
  consumers of the stored value match case-sensitively. The test is not wrong
  about its own function; it is wrong about the system.
- **P6b - a new doc rule scoped narrowly enough to hide the violation**
  (F-120): a `docs/CODING.md` rule naming three functions, so `end_game` - a
  fourth unguarded lifecycle writer that rates the game - is invisible to the
  grep procedure the doc itself prescribes.

### P7. A landed, tested fix silently reverted by a later commit in the same programme

- F-109 (High) - WP-36 shipped ws F55's shutdown drain plus a dedicated
  regression test (`rust/web/tests/websocket_hygiene.rs`); the later SSE
  migration `efad81f` deleted the fix and the test together. The checklist row
  and both commits still read as closed.

Settled: `efad81f9` contains exactly **one** instance, demonstrated by
enumerating all 12 touched files. WP-42 was **not** reverted by the SSE
migration - a useful negative. `ca7925bc` is not an instance either (`+20/-0`).
F-109's remediation is a bookkeeping fix on WP-36's row plus a decision on the
never-implemented second half of ws F55, not a revert.

### P8. The `_ => <default>` substitution

Converting a lookup-with-default into a `match` with a catch-all arm satisfies
"make this exhaustive so no caller can silently fall back" rows without
changing any behaviour.

- F-65 - the original game-crate instance.
- F-136 (High) - the web-half instance. This pattern is no longer a game-crate
  curiosity.

### P9. `Gamer::validate` defaults to `Ok(())` - the D-36 trust boundary is fail-open

- F-06 (High). See section 5 for the corrected denominator.
- F-210 is the same hole reached from the other side and must be remediated
  with F-06's sushi-go-2 row.

### P10. A spec's own stop-work trigger answered with a comment rather than a stop

- F-206 (Medium). WP-69's spec §3b said to stop and report if
  `multiple-versions = "deny"` needed more than "roughly a dozen" skip entries.
  `e2ee5342` shipped **29** and wrote *"not papered-over sibling work"* directly
  above them (`rust/deny.toml:71-76`). That claim is falsified by `:131`'s own
  annotation (`tower-http 0.7.0`, "via web (first-party, pins 0.7.0 directly)",
  against `rust/web/Cargo.toml:44`) - the only one of the 29 with a first-party
  cause, all 29 checked. Compounding: WP-69 §5's "the flip must actually bite"
  negative checks are recorded in `EXECUTION-STATE.md` as parked, never run.
  Beyond P6/P6a: the criterion was a stop-work condition, not a test.

### P11. A finding whose premise was disproved, closed rather than amended

- F-205. `dp F12`'s "sentry drags actix-web + ureq into every build" was never
  true - neither is a sentry 0.48 default and nothing enables them; both are
  still inert `[[package]]` entries in `rust/Cargo.lock` at HEAD after a later
  regeneration. WP-67's own rider 2 required the downgrade be written back into
  the finding; `SUMMARY.md:44-46,139` and `findings/dependencies.md:103-108,157`
  are unamended.

### P12. The self-certifying work package - one that exists only as a commit

No spec, no checklist row, a one-line commit message. Such a package cannot be
verified by any sign-off procedure.

- WP-72 - the original.
- F-210 / `ae04843c` - the second instance, and the first where the
  self-certified premise is **demonstrably false**. The commit turned
  sushi-go-2's `_ => 9` into `_ => unreachable!()` on the premise "start()
  rejects counts outside 2..=5". sushi-go-2 has no `validate` override at all,
  `all_players` is never bounded, `Game` derives `Default` with all-`pub`
  fields, and `draw_count(self.all_players)` is called from `command()` at
  `:289` - past the D-36 boundary. `all_players: 0` now panics the game service
  where it previously dealt 9 cards.

### P13. The documentation-only constant

- F-153. `wd F50`'s "one const used by all eight sites" shipped as an
  `#[allow(dead_code)]` string used by zero sites, with a doc comment stating
  manual sync is now required.
- F-170 - the same shape.

Sign-off mitigation: sweep `rg "allow\(dead_code\)"` across the commit range.

### P14. A vendoring WP inherits an upstream defect that "minimal port" guarantees comes along

- F-200 (Medium). The vendored session store's `migrate()` returns `Ok(())`
  before `create table` and without committing on the duplicate-key path; it is
  the **sole** creator of `tower_sessions.session` (nothing in
  `rust/web/migrations/`). Cold-start race with more than one web replica means
  startup reports success and the table is never created. The "minimal port, not
  a rewrite" criterion was correctly followed - and that is exactly what
  guaranteed the defect came along. `rust/lib/session_store` has no `tests/` and
  no `#[cfg(test)]` module.

Recommendation: any future vendoring spec needs a "known upstream defects
inherited" acceptance criterion. See section 9 for the owner's policy question.

### P15. WP-10 3a declared "for every game crate" and applied to 3 of 28

No later WP swept the rest. 13 crates have no redaction test.

### P16. Hardening that converts a soft default into a startup panic with no deployment acceptance criterion

- F-96 (downgraded - see section 9). Turning a missing env var into a `panic!`
  is correct security practice and a production outage if nothing sets the var.
  No WP had a deployment-manifest criterion. The family also contains
  `TURNSTILE_SITE_KEY` (no startup check at all; setting only the secret key is
  a total login outage), `config::public_base_url()` defaulting to
  `http://localhost:3000` (making WP-58's `List-Unsubscribe` RFC 8058-invalid in
  prod), and F-207's three-way `sqlx-cli` migrator version split.

### P17. Hand-maintained delivery lists with no cross-check

- F-208 (High). `rust/Dockerfile` stages, `docker-bake.hcl` targets and
  `k8s/base/game/` Deployments are three (four, counting `brdgme-go`)
  independently hand-maintained lists with no CI guard relating them. A
  complete, tested, documented new game crate can therefore be built on every
  image build and never copied out.
## 5. Counting integrity

Three counts in this report are easy to get wrong, and getting any of them
wrong misleads the reader in a specific direction. They are stated here
explicitly.

### 5.1 "Untested by design" is not a falsified row

The `Test? y with no test` tally counts **checklist rows that assert a test
exists where none does**. It must not absorb work packages that were never
required to have a test. `00-STATE.md` records which is which:

- **WP-65** - its nine checklist rows are all `Test? = n`. Explicitly untested.
- **WP-64, WP-66, WP-67, WP-69, WP-70, WP-72, WP-73** - no checklist row at
  all. WP-64/66/67/69/70/73 were explicitly deferred as
  BLOCKED-ON-DECISION D-19/D-20/D-23; WP-72 appears in no checklist and has no
  spec.
- **WP-76, WP-77, WP-79, WP-80** - no spec and no row in any of the eight
  `T3-B*` checklists. `EXECUTION-README.md:408` records this as a **deliberate**
  gap. WP-60 also has no spec (its criteria are the WP-60 rows of
  `checklists/T3-B6-outbound-email-websocket.md`).

**None of these count toward the tally.** Counting them would inflate the
process-failure narrative with an artifact of counting. They are a separate and
lesser concern - untested by design is a coverage question, not a broken
acceptance artifact.

### 5.2 The tally is nine, not six

A naive per-ID count finds six: F-142, F-148, F-149, F-150, F-171, F-176.

**Part 3 established that F-176 is one ID covering four separately falsified
rows.** Any per-ID count therefore undercounts by three.

**Confirmed tally: nine falsified `Test? y` rows.**

Note also that F-150 is itself all seven rows of one work package, but those
seven were filed as a single row-set under one ID and one finding; F-176's four
were not. The nine is the figure to quote.

One threat to this count was raised and closed. Unit 09a asked whether
`rust/web/Cargo.toml:99-154` declaring `hydrate` and `ssr` but **no `default`**
feature would silently compile out every `#[cfg(all(test, feature = "ssr"))]`
module and retro-void many `Test? y` rows at once. Unit 09b settled it
definitively: `scripts/rust-ci-commands.sh:30` runs
`cargo test -p web --features ssr` and CI runs the same script
(`.github/workflows/ci.yml:93-94`). 423 gated test functions across 25 modules
are live. **No `Test? y` row is retro-voided.** Do not re-raise this.

### 5.3 F-06's denominator: 15 of 28 override, 13 do not

F-06's own text contradicts itself - a heading reading "15 of 27" against a
body reading "15 of 28 / 13 without" - and report 02 warned that F-06's crate
list omits `seven-wonders-1`.

Re-derived mechanically from `00-sweeps.md` (sweep 1, `:9-47`), which is a
precomputed sweep and was read directly:

- **28** crate directories exist under `rust/game/`, and the sweep's 28 names
  match `rust/Cargo.toml`'s 28 game members **exactly**. No crate is missing
  from the sweep.
- **15 override `Gamer::validate`. 13 do not.** The sweep's own closing line
  reads `15 yes / 13 no.`
- The 13 without an override: acquire-1, alhambra-1, cathedral-2, jaipur-2,
  lords-of-vegas-1, lost-cities-1, roll-through-the-ages-2, seven-wonders-1,
  splendor-2, starship-catan-1, sushi-go-2, sushizock-2, texas-holdem-2.
- **`seven-wonders-1` is present in the sweep and correctly on the
  non-overriding side.** Report 02's warning is not reproducible against
  `00-sweeps.md`; `seven-wonders-1` belongs in the 13 and is already counted
  there.
- `hanamikoji-1` is on the overriding side
  (`rust/game/hanamikoji-1/src/lib.rs:673`).

**The correct statement of F-06 is: 15 of 28 game crates override
`Gamer::validate`; 13 do not, so the D-36 trust boundary is fail-open for
13 crates.** The "15 of 27" heading is wrong and should be corrected in the
corpus. `00-STATE.md`'s pattern text ("13 of 28") was already correct.

One consequence worth stating: `sushi-go-2` is on the 13-crate list, which is
exactly why F-210's self-certified premise ("start() rejects counts outside
2..=5") is false. F-210 and F-06's sushi-go-2 row are one remediation item.

---

## 6. Verified good and refutations

This section is the definitive record of what the programme got right and, more
importantly, of what this session **proved is not a defect**. Every item below
cost a unit real budget to establish. **Nothing in 6.1 or 6.2 may be
re-derived, re-raised or re-investigated by remediation work.** Where a
refutation has a residual - a real finding discovered while disproving the
suspected one - the residual is named so the closure is not read as "nothing
here".

### 6.1 Refutations that must never be re-raised

**R1. `rust/bot/src/prompt.rs` is REFUTED as a leak vector.** It is a pure
minijinja renderer over a closed field list; `BotContext.game_state` never
enters a context struct. The file also predates the remediation programme
entirely, so it was never in scope for a fix. Unit 05b listed it as an unowned
surface (part 2 discrepancy 12); Unit 10a closed it. **Residuals: F-192 and
F-193 are the real leaks, found in the same pass.** Second residual: the test
`render_user_includes_state_in_yaml_fences` (`rust/bot/src/prompt.rs:291-302`)
is a confirmed decoy - the fixture hand-writes the yaml as string literals, so
swapping in another seat's state would still pass. That extends the decoy class
to the bot crate but does not reopen the leak question.

**R2. The `ssr` feature-gate question is SETTLED and REFUTED. No `Test? y` row
is retro-voided.** 423 gated test functions across 25 modules are live. The
evidence is set out in full in section 5.2 and is not restated here. This was
the single largest threat to the session's counting integrity and it is closed.
Do not re-open it.

**R3. `hanamikoji-1`'s epilogue is guarded - the carry-forward is REFUTED.**
`00-STATE.md` carried forward, unverified, that `hanamikoji-1` had an unguarded
epilogue and should join the F-18/F-71 unmigrated-crate list. Unit 11 disproved
it: `rust/game/hanamikoji-1/src/lib.rs:796` sets
`let was_finished = self.is_finished();` and `:830-834` gates on
`if !was_finished && self.is_finished()`, identical in shape to `jaipur-2`;
`:833` is inside the guard. There is a dedicated regression test,
`test_finish_emits_epilogue_once`. **`hanamikoji-1` does NOT join the F-18/F-71
list, and that list stays at five crates.** This is the one place in the session
where a unit report overrides `00-STATE.md`, and it does so legitimately -
`00-STATE.md` recorded it as an unverified carry-forward and `00-HANDOVER.md`
already accepts the refutation.

Corrected premise established in the same pass: **`finish_epilogue` is a
per-crate inherent method in 12 crates, not a `rust/lib/game` helper.** Only
`placings_log` is shared. Any remediation item written as "fix the shared
helper" is malformed.

**R4. Unit 10b's "43 k8s Deployments vs 26 image stages" premise is REFUTED.**
43 = **26 Rust + 17 legacy Go** games, whose image stages live in
`brdgme-go/Dockerfile`, a second repository. Bake targets are identical to the
Dockerfile stages, and **zero stages lack a Deployment**. The apparent 17-item
gap does not exist. **Residual: F-208 is the only real delivery-list gap** -
`hanamikoji-1` has no stage, no bake target and no Deployment.

**R5. The `brdgmen` non-existent-package claim was WRONG and must not be carried
forward.** A Worker asserted that `docs/porting/GAME_PORTING.md:215` cites a
non-existent package `brdgmen`. It does not. The line reads
`cargo run -p brdgme_repl`, which matches the crate. Nothing to fix.

**R6. WP-64: all four briefed hunts proved negative.** No pattern 2, no silent
default change, no feature narrowing (the one feature change is a **widening**),
no pattern 4b.

**R7. WP-66's `default-features = false` sqlx narrowing is inert.** All four
dropped features are compile-time-only.

**R8. `serde_yaml_ng` is a faithful fork.** A full `diff -ru` against the
registry copy shows only `i64::max_value()` -> `i64::MAX` plus an additive API.
All 7 call sites are serialisation-only. The vendoring gate was honoured.
**Residual: F-200** - the upstream defect the "minimal port" guarantee carried
along.

**R9. `[licenses.private] ignore = true` in `deny.toml` is correct config.** No
`cargo-deny` setting could machine-check the vendored MIT obligations; the tool
is not the right instrument. **Residual, unowned:** those obligations are
hand-satisfied and a one-line CI grep would close the gap.

**R10. WP-69's unspecified `allow-wildcard-paths = true` improves on a wrong
rider.** It is the explicit counterweight to F-206, not an unauthorised change.

**R11. `be185ccb` is a harmless bookkeeping race, explicitly NOT pattern 4b.**

**R12. The deleted `*_repl` binaries are a capability move, not a loss.**

**R13. `5e9bae2c` is a genuine de-flake, NOT a weakened assertion.** The pattern
4b suspicion is cleared. **Residual:** per-log `created_at` collapse is no
longer catchable by that test.

**R14. WP-44's guard removals are net-neutral TOCTOU closures**, and WP-47 is
wired end-to-end. Explicitly **not** pattern 2 and **not** pattern 4e instances.

**R15. `5786a1b6` is neither pattern 4b nor pattern 4e.** The spec's own
instruction was factually wrong; the follow-up commit corrects the string to the
truth. **Residual: F-174.**

**R16. WP-77's default bot name is canonical.** The "fifth write path"
hypothesis is refuted on the settled path, and the five other bot-name write
sites are clean.

**R17. Six separate 09b refutations, all in the email/settings surface.**
`settings.rs`'s `<select prop:value=...>` is **not** a pattern-2 miss of WP-54's
build-order fix; `svix-id` is **not** attacker-chosen; `processed_webhook_events`
does **not** grow forever; a GET **cannot** unsubscribe; the unsubscribe token is
**not** guessable and does **not** reuse `settings_email_token`; and **all
eight** bulk-mail sites got the unsubscribe link.

**R18. F-170's scope does not extend to the game-start mail.** 09c refuted the
extension - that path reads `turn_emails_enabled` directly. `00-STATE.md`
records the refutation.

**R19. `hanamikoji-1`'s `render.rs` is not a leak vector**, and its `pub_state`
is **structurally** redacted - hidden fields are omitted by construction, not
blanked. This is the strongest form of the D-33 pattern seen anywhere in the
session and is the shape other crates should be moved toward.

**R20. `a99bf754` and `3f52d2b7` are both verified good.** `a99bf754` removes
`exec` from `scripts/rust-test.sh`; `3f52d2b7`'s `ALLOW_INSECURE_DEFAULT_KEY` is
set in dev/CI only, with **zero hits** under `k8s/base/**` or `k8s/prod/**`.

**R21. `restart_core`'s pool-read-under-`FOR UPDATE` is NOT a deadlock.**
`00-STATE.md` refuted it. It remains off-convention - neighbours at `:1136` and
`:1145` use `_tx` variants - and an `is_player_in_game_tx` would be strictly
better, but no defect exists.

**R22. F-91's AAD-less ciphertexts are not exploitable at the admin call
sites.** Checked at all of them.

**R23. `replacement_bot_available` and `pick_replacement_bot` share the same
predicate.** The F-115-shaped mismatch is **not** present there. Related and
answered: `game_bots` has `UNIQUE (game_id, name)` (`migrations/003:15`) and
`pick_replacement_bot` has no `ON CONFLICT` - folded into F-111.

**R24. The one spec-prescribed test name that is absent is not a gap.**
`concede_game_replace_rejects_stale_updated_at` is a test the spec never
actually asked for.

**R25. Findings closed as REFUTED rather than fixed, kept as rows so they are
not re-derived:** **F-123** is REFUTED/ARCHIVED - its owner-visibility half was
re-issued as **F-133** and its downgraded remnant is **F-132**; **F-110** is
explicitly "not a defect"; **F-27** was withdrawn by its own reviewer; **04b's
mis-numbered second `F-78`** is a WITHDRAWN negative-footer theory for
`category-5-2/src/render.rs:115-122`, disproved and retained only so it is not
re-filed (the table's F-78 row is 04c's; see section 5's ID notes). **F-208a**
is a sub-letter for a REFUTED carried premise, not a defect - Unit 11's real
defect count is 4.

**R26. Two owner rulings that function as refutations.** **F-81:** reconstructing
hidden information from the public log is acceptable by design and is **not a
finding**. The ruling is general, not `no-thanks-2`-specific; its boundary is
that direct leaks into `Log::public` remain findings. **`lords-of-vegas-1` is
WIP by owner ruling** - F-50 and F-57 are excluded and no finding about missing
or incomplete functionality there is valid. (F-59's status under that ruling is
undecided; see the open questions.)

### 6.2 Discharged obligations and closed sweeps

These are sweeps a unit was explicitly asked to run to completion. Each returned
a bounded, enumerated answer. **Do not re-run any of them.**

| # | Obligation | Result |
|---|---|---|
| O1 | Does `efad81f` contain more than one pattern-4e instance? | **Exactly one** (F-109), demonstrated by enumerating all 12 touched files. F-163 is the near-miss. |
| O2 | Is WP-56's DMARC reasoning sound? | **NOT SOUND** - became F-161, the session's most severe finding. |
| O3 | What is F-131's authenticate-once consequence? | Concretised as F-158. |
| O4 | F-15's `--mk-soften-*` token sweep | **DISCHARGED, stays LATENT, no live violation.** Every referenced token is emitted; game crates emit exactly `{(Pink,80),(Foreground,80),(Foreground,90)}`; no game emits a `mix`, so the empty `IN_USE_MIXES` is correct; `main.scss` is the only stylesheet. The real emitter is `rust/web/src/theme.rs`. Do not re-run the sweep. |
| O5 | Can `test-support`'s panic constructs reach a release build? | **No.** 28/28 consumers are dev-dependencies, the feature is not in `default`, and the 14 panic constructs are unreachable in release. `assert_gamer_contract` is called from all 28 `rust/game/*/tests/contract.rs`. |
| O6 | Was the `rust/.sqlx` deletion a loss? | **REFUTED** - it is correct consolidation, with the causality inverted (WP-52 is an *ancestor* of WP-66). Only residual is a process nit: WP-52's commit message does not mention removing an 81-file directory. |
| O7 | Does the missing `default` feature void the `ssr`-gated tests? | **Settled and refuted** - see R2 and section 5.2. |
| O8 | Are WP-59 Tasks 9-14 a coverage hole? | **No.** `f56ff37` owns 9/11/12/13; Task 10 was dissolved by WP-56 (`da1ea24`); Task 14 is a deliberate non-implementation per the spec's own carve-out to WP-85. Implementation status only - this was not a deep code review of those tasks. |
| O9 | The `RouteOutcome` sweep of `email/inbound.rs` | **CLOSED.** F-162 and F-169 are the only two defective routes; **no third route** has the defect. |
| O10 | Is `ca7925bc`'s game-start sweep complete, and is it a pattern-4e revert? | **Complete and not a revert.** All four `insert_game_from_service` callers notify; the diff is `+20/-0`. |
| O11 | Is `for-sale-2`'s `pass()` half-bid rounding inside the WP-11 park? | **Yes** - `f F14`, BLOCKED-ON-USER-RULES-REVIEW, D-30 + D-35 parked. Deliberately not fixed; **not** a remediation gap. WP-11 also parks parity items in four of the five WP-33 crates, so no parity observation in `greed-2` / `farkle-2` / `no-thanks-2` / `liars-dice-2` / `zombie-dice-2` should be raised without first checking `f F2/F15/F21/F33/F43/F50/F54`. |

### 6.3 Work verified good

**WP-73 - verified good by exhaustive proof, not sampling.** This is the
strongest positive verification in the session and the standard other sign-offs
should be held to. **All 108 pre-commit game binaries normalise to exactly four
distinct contents, 27 each.** The `:80` -> `:8080` default change is inert, since
all 43 Deployments set `ADDR`. `[lints] workspace = true` is present on all 44
members. The consolidation is sound. (Sizing note: WP-73 is 139 files, of which
135 are three-line wrappers - `00-breakdown.md`'s premise was wrong here too.)

**WP-37 - the only Unit 05 package delivering full scope (14/14).** All 16
server fns call `require_admin`, backed by a source-level self-check,
`every_admin_server_fn_calls_require_admin`. **Caveat, not a refutation:**
`require_admin`'s *true* path is untested for 13 of 16 server fns - the authz
wiring is proven, the authorised behaviour behind it is not.

**WP-38 fully delivered** - tasks 3a-3d plus the section-5 tests.

**WP-39** - `supervise_consumer` is correct, and **`reorder_bots` is the
strongest function in the unit**.

**WP-68 completely clean** - `term_size` is gone from both the dependency tree
and the lockfile.

**WP-47 wired end-to-end** (see R14).

**`rust/bot/src/main.rs` has real graceful shutdown and zero `unwrap`/`todo`** -
better than the web side on both counts.

**WP-82's `db.rs` split is a pure move** - 21 symbols, no SQL change, 128 tests
before and after. **Caveat:** comparing against HEAD shows 24 extra tests added
by later commits, so the "same test count" evidence is weaker than it looks in
isolation.

### 6.4 The `hanamikoji-1` inversion

State this plainly, because it is the most telling result of the session:

**The crate written *after* the review - `hanamikoji-1` - internalised the
review's lessons better than most of the crates the review remediated, and both
of its gaps are in areas no checklist covered.**

The evidence:

- **It is the first crate in the session with a `validate` override AND a
  `validate` test** (`rust/game/hanamikoji-1/src/lib.rs:1079`). `00-STATE.md`
  pattern 2b's standing claim that "no crate reviewed so far has a `validate`
  test" is **broken** by it.
- **It is the first crate with a dedicated epilogue-gate regression test**
  (`test_finish_emits_epilogue_once`; gate at `:796` / `:830-834`). See R3.
- **Its `Status::Finished` populates `stats`** (`:734-737`) - a **negative** for
  the F-35 tally. The one crate written outside the programme is the one that
  populated stats.
- **Its `pub_state` is structurally redacted** and its `render.rs` is not a leak
  vector (R19) - the strongest form of the D-33 pattern anywhere in the session.

And the two gaps, both outside checklist coverage:

- **F-208 (High):** it is unshippable. No `rust/Dockerfile` stage, no
  `docker-bake.hcl` target, no k8s Deployment. It is compiled by
  `rust/Dockerfile:36`'s `--workspace` build on every image build and then never
  copied out. No checklist row covers the four hand-maintained delivery lists,
  and nothing links them (pattern P17).
- **F-209 (Medium):** `validate` bounds every parallel vector and seat index but
  never relates `phase` to `pending`. No checklist row asks for cross-field
  invariants.

**Use `hanamikoji-1` as the model, and use F-209 as the proof that having a
`validate` test is not sufficient.** Its own tests are shallow in the way the
rest of the codebase's are: `test_redaction` covers the opening position only,
and `test_validate` covers lengths and ranges only. Neither would catch a
leaking field or F-209's cross-field invariant. **WP-10 3a's "13 crates with no
redaction test" gap is untouched by Unit 11.**

The lesson landed. The checklists did not carry it. That is the finding behind
the finding.

---

## 7. Coverage gaps and unowned items

Section 6 records what the session **proved**. This section records what it
**did not look at**, and what it looked at but left with no owner. The material
is the 86 substantive items the three findings parts rescued from the unit
reports without an F-number (2 in part 1, 43 in part 2, 41 in part 3), merged
with `00-STATE.md`'s own coverage notes. Refutations, discharged obligations and
verified-good work are section 6's and are not restated here.

Deduplication is aggressive: where the same gap was raised by two units it
appears once, and the merge is stated. Nothing substantive was dropped.

**Read this section as the scope statement for the remediation plan.** An item
here is either work nobody has done or work nobody owns; in several cases it is
both.

### 7.1 The two coverage statements that must not be misread

**`roll-through-the-ages-2` has never had a crate-level review.**
(`00-STATE.md:225-228`.) 3,290 lines. **No `validate` override, no redaction
test**, and the one function anyone in this session actually read contained
**F-83**. Every other one of the 28 `rust/game/*` crates has an owning sub-unit
(`04c-games-cleanup-parity-wp33.md`); this one has coverage only by assignment,
not by reading. It was correctly out of scope for a review-of-the-remediation -
the crate was barely touched by the programme - but that is a scoping fact, not
a clean bill of health. **Recommend a dedicated crate-level pass in the
remediation plan**, sized as a full review unit, not as a follow-up task. Its
`validate` absence is an F-06 instance and its missing redaction test is a
WP-10 3a instance, so it inherits both of those defect classes untested.

**01c's checkmarks are epilogue-shape only.** (`00-STATE.md:229-230`.) The `V`
marks in `01c-epilogue-dedup.md` record one thing: whether a crate's epilogue
emission is correctly shaped and gated. **They must not be read as crate-level
review coverage in this report or anywhere downstream.** A crate can carry a 01c
checkmark and have had no line of its `validate`, its redaction or its command
handling read by anyone. Any remediation plan that treats 01c as a coverage
matrix will under-scope the game half.

### 7.2 Game crates

| Item | Citation | Owner |
|---|---|---|
| `roll-through-the-ages-2` unreviewed (above) | `00-STATE.md:225-228` | **None** |
| `Gamer::points()` ordering contract, and `cathedral-2`'s inverted sign - `cathedral-2/src/lib.rs:580-584` returns `+remaining_piece_size` while `calc_placings` ranks on `-remaining_piece_size` | 03b coverage gaps; F-58's closing paragraph | **None.** No F-number anywhere in the corpus. `00-STATE.md` routed it to Unit 08, which **handed it back**: it needs a remediation-plan owner, not another review unit. |
| `for-sale-2` `pass()` -> `take_first_open_card()` panics on `open_cards.remove(0)`, same shape in `start_selling_round` - 04b re-confirmed and **escalated** F-01 | `for-sale-2/src/lib.rs:138-150`; F-01 | F-01's row is 01a's and carries only the 01a citation. **The escalation detail is in no row** - carry it onto F-01 when remediating. |
| WP-10 3a's "13 crates with no redaction test" | see section 6.4 | **None.** Untouched by Unit 11 and by every other unit. `hanamikoji-1`'s own `test_redaction` (opening position only) and `test_validate` (lengths and ranges only) show that having the test is not the same as the test being sufficient. |

The `hanamikoji-1` shallow-test point is stated in 6.4 as part of the inversion
argument; it appears here because it is also the game half's largest open
coverage item.

### 7.3 Web server

**Structural - no harness exists.**

- **No request-parts test harness.** Every `#[server]` fn in
  `rust/web/src/auth/server.rs` is untested end to end. This is the **structural
  cause of F-92** and the reason **F-85 was uncatchable** by any test the
  programme could plausibly have written. Building the harness is a prerequisite
  for meaningfully testing the auth surface, not an optional extra. **No owner.**
- **`require_admin`'s true path is untested for 13 of 16 server fns.** Section 6.3
  records this as the caveat on WP-37; here it is the coverage item. The authz
  *wiring* is proven by the source-level self-check
  `every_admin_server_fn_calls_require_admin`; **the authorised behaviour behind
  the gate is not exercised**. WP-37 is the only Unit 05 package that delivered
  full scope, and it still has this hole. **No owner.**

**Code never read.**

- `rust/web/src/admin.rs:1560-3488` - the Leptos UI components. Never read by any
  unit.
- `rust/web/src/db/game_write.rs` - largely unread. `update_game_command_success`
  had **no line-level review this session**.
- `rust/web/src/crypto.rs` has **no `load_key` test**, so the whole `ws F16` fix is
  unexercised - while the *unfixed* bot copy tests all three paths.
- `rust/bot/src/routing.rs` had no owning sub-unit at Unit 05b time. Unit 10a later
  covered `prompt.rs` from the same pair (section 6, R1); **`routing.rs` was never
  picked up.**

**Tests that exist but do not test what their name claims.** These are decoys with
no F-number of their own; they belong to pattern P6 but were not filed.

- `verify_turnstile_rejects_on_transport_error` (`auth/server.rs:1856-1862`) makes a
  **real network call to Cloudflare** and passes for the wrong reason. Nothing covers
  non-200, malformed JSON, or a live `success: false`.
- `rating_before_aggregates_exclude_nulls` (`stats/queries.rs:1287-1346`) name-matches
  `wd F51`'s risk exactly, **never calls `game_history`**, and asserts PostgreSQL
  aggregate semantics instead. This is the source of `00-STATE.md`'s F-109 sign-off
  sharpening (ii): **a regression test must actually call the function under test.**
- Nothing tests a **live game after concede-with-replace**. The tests assert the
  write, never the invariant. This is why **F-119 survived seven new guard tests**.
- Nothing tests `compute_ranked_placings` against a state any real finish path
  produces - its three tests use hand-built vectors. **F-117 is the consequence.**

**Remediation-shaping items with no F-number.**

- **`left_at` conflates "eliminated by play" with "left the game"**, is written by
  four call sites, and has **no owner**. **F-113, F-116 and F-117 are all symptoms.
  Carry this as ONE schema-change item, not four findings.**
- **`CanonicalEmail` newtype** - Unit 07's single most valuable recommendation. The
  contract is currently enforced by doc comment only (`db/emails.rs:71`,
  `db/visibility.rs:171`). **Fold with F-128 and F-173 into one remediation item**;
  F-128 is recorded as not closed and having no owner.
- **Three email-borne bearer tokens** (settings, unsubscribe, invite) have three
  different lifecycle disciplines, no shared abstraction, and **only two rotate**.
  One combined remediation item.
- `i-noreply@brdg.me` is not short-circuited in the inbound router
  (`inbound.rs:95-96` -> `InboundRoute::Invite("noreply")`; `:856-866` runs a real
  token lookup that misses). One wasted query per one-way mail; **two-line fix, no
  owner**.
- `restart_core`'s pool-read-under-`FOR UPDATE` is off-convention - neighbours at
  `:1136`/`:1145` use `_tx` variants. **Not a deadlock** (refuted, R21); an
  `is_player_in_game_tx` would be strictly better. Convention residual, no owner.
- `auth/server.rs:92` still increments `login_emails_sent_total` **before** the send
  with no failure counter - the exact shape `wfe F46` fixed, but outside WP-60's
  scope. A programme-level consistency item.
- A **module-granularity `#[allow(dead_code)]`** at `rust/bot/src/main.rs:4-7` covers
  all of `mod config` and `mod crypto` - broader than the F-153/F-170 cases, and it
  would hide `crypto::encrypt` and `LoadedKey::is_default`. Flagged for the sign-off
  sweep; that sweep is the only route it has.

**Deliberately not raised, flagged here so the remediation pass sees them.**

- `wd F46`'s `page + 1` next-page-link clamp is **unverified** - a one-line change to
  `rust/web/src/players.rs`.
- `wd F49` bounds the payload **in Rust after fetching every row**, so DB work stays
  unbounded on an anonymous endpoint. The checklist row's wording is satisfied; the
  risk is not closed.
- WP-53 residual cosmetics: `3610b957` deleted `encode_path_segment`'s doc comment and
  left a mid-file `use percent_encoding::...` at `players.rs:34`; `wd F77` was
  satisfied by a two-word swap in `settings.rs:1-2` that does **not** enumerate
  add/confirm/make-active/remove as the row asked.

### 7.4 Email

- **`email/sweep.rs` was verified only for the WP-38 bot-turn sweep. Its test module
  was never read.** The other sweeps belong to WP-46.
- **WP-46's `proposals.rs` half (+428 lines) was read only at four spec-named
  functions; its new test module is unaudited.**
- **WP-51 (`dcd8844c`) and WP-53 (`3610b957`) were NOT audited by Unit 07** - six
  `RealInviteMailer` methods, the `spawn_sweep` collapse, and `notify_owner_decline`'s
  new gating. Unit 07b was dispatched for exactly this surface and produced six
  findings on its re-run. **Open question: whether 07b closed the whole surface or
  only the part it reached.** Unit 07b died to quota exhaustion mid-unit on its first
  attempt and was re-dispatched from scratch, so the surface was walked once, not
  twice.
- **`config::public_base_url()` defaults to `http://localhost:3000`**, which would make
  WP-58's `List-Unsubscribe` header non-HTTPS and **RFC 8058-invalid in production**.
  Route to the same deployment checklist F-96 produced. Also recorded in `00-STATE.md`.

### 7.5 Bot, operator and tooling

- **Zero tests on `build_messages`.** The entire bot test module covers only
  `merge_json_patch` and one constant. **The bot crate has no DB tests at all.**
- `fetch_game_data`'s test is one assertion short of real: two seats with distinct
  hands, only the positive asserted. **Nothing anywhere asserts that opponent hidden
  state is absent from the rendered prompt** - which is the property F-192/F-193 turn
  on.
- No test on the log SQL filter in either bot or web; **no test for the F-189
  case-mismatch path**; no round-trip test between the two `Bot*Event` definitions; no
  envelope-level test on `route::<G>()`.
- `rust/operator/src/controller.rs`: no test flips an already-newest version to
  deprecated, and **`cleanup` has zero test callers**.
- `rust/tools/fuzz/src/lib.rs:53-57` - `recv()` has no timeout, so a worker wedged
  inside `requester.request()` still hangs the driver. **An explicit spec non-goal**,
  recorded so it is not re-filed as a defect.

### 7.6 Dependencies, deployment and infrastructure

- **Nothing tests the vendored `rust/lib/session_store`.** No `tests/` directory and no
  `#[cfg(test)]` module anywhere in it. It is **authentication-adjacent and now
  first-party code**, and it is where **F-200** lives - `migrate()` returns `Ok(())`
  before `create table` and without committing on the duplicate-key path, and it is the
  **sole** creator of `tower_sessions.session`. Vendoring moved the maintenance
  obligation in-house without moving any test obligation with it. **No owner.**
- **Vendored MIT obligations are hand-satisfied and cannot be machine-checked.** No
  `cargo-deny` setting is the right instrument (R9). **A one-line CI grep would close
  it. No owner.**
- **`deny.toml`'s skip list has 29 entries** (`00-STATE.md` CORRECTION; the 10b report's
  own line-488 figure of 24 is stale) with **no expiry and no `unused-skip`**, the weekly
  job **never runs `bans`**, and `[advisories].ignore` has the same shape. Folded into
  the **F-199 + F-206** remediation item - the only item in this section that already has
  a route.
- **One `ALLOW_INSECURE_DEFAULT_KEY` flag disables two unrelated production guards**
  (`crypto.rs:60` and `main.rs:42`). Splitting it is suggested, not urgent.
- **`k8s/argocd/brdgme-app.yaml` is a stale duplicate** (already `docs/BACKLOG.md:67`),
  and the config repo's claimed **CI auto-push does not exist** - new SealedSecret files
  get **no `kubeconform` validation**.
- **The e2e job is `continue-on-error: true`**, so no assertion in it gates a merge or a
  deploy, and the underlying hydration race has no tracking item. **Compounds F-211.**
- **Nothing links the four hand-maintained delivery lists** - `rust/Cargo.toml` game
  members, `rust/Dockerfile` stages, `docker-bake.hcl` matrix and
  `k8s/base/game/kustomization.yaml`, spanning two repositories. **F-208 is the first
  miss.** A CI guard with an explicit WIP allowlist is a programme-level remediation
  item (pattern P17).

Two deployment-checklist items from the same family already carry F-numbers and are
**not** unowned: `TURNSTILE_SITE_KEY` has no startup check, and `rust/bot/src/crypto.rs:66-76`
falls back to the hardcoded dev key ungated in any environment. They are listed here only
so the checklist family reads complete alongside **F-96** and **F-207**.

### 7.7 Corpus, specs and process

**The no-spec WP list is longer than `00-STATE.md` records.** `00-STATE.md:28` names
**WP-24, WP-27, WP-44, WP-53, WP-79**. Part 2 establishes that **WP-36, WP-43, WP-44,
WP-52 and WP-53** must be added; part 3 adds **WP-60, WP-65, WP-72, WP-76 and WP-77**.
The corrected combined list is:

| WP | Note |
|---|---|
| WP-24 | `00-STATE.md:28` |
| WP-27 | `00-STATE.md:28` |
| **WP-36** | **The crypto package.** See below. |
| WP-43 | added by part 2 |
| WP-44 | in both lists |
| WP-52 | added by part 2 |
| WP-53 | in both lists |
| WP-60 | added by part 3; its criteria are the WP-60 rows of `checklists/T3-B6-outbound-email-websocket.md` |
| WP-65 | added by part 3; all nine of its checklist rows are `Test? = n` |
| WP-72 | added by part 3; **no spec AND no checklist row** - self-certifying, pattern P12 |
| WP-76 | added by part 3; no spec AND no checklist row, deliberately (`EXECUTION-README.md:408`) |
| WP-77 | as WP-76 |
| WP-79 | `00-STATE.md:28` |

Thirteen work packages. **WP-51 is not on it** - it does have a spec
(`planning/specs/WP-51-invite-mailer-notify-dedup.md`); an Orchestrator brief wrongly
said otherwise, and `00-STATE.md:29-32` corrects that.

**WP-36 is the crypto package.** Its `T3-B*` checklist rows do not exist and its spec
does not exist, so its **only** acceptance criterion is a commit message. **Unit 05's
highest-stakes verdict therefore rests on weaker evidence than any other verdict in the
session.** That is not an accusation that the verdict is wrong; it is a statement that
nothing in the corpus could have made it right. Treat WP-36 as unverified until a spec is
reconstructed and the package is re-checked against it.

Two of these entries are load-bearing elsewhere and must not be double-counted:
**WP-65, WP-72, WP-76 and WP-77 carry no falsified `Test? y` row** - "untested by design"
is not a falsified row, and section 5.1 already draws that line.

**Corpus corrections this session established.**

- **`SUMMARY.md` at HEAD is a narrative compaction carrying NO `wd Fnn` / `wfe Fnn`
  identifiers.** Finding text survives only at `868094a6:.../findings/*.md`. Any
  remediation work that needs original finding text must go to git history.
- **The corpus records a false dependency fact** (`SUMMARY.md:44-46,139`;
  `findings/dependencies.md:103-108,157`), and two stale superpowers docs still assert the
  `0.0.0.0:80` default and cite a WP-73-deleted path. **The remediation should amend the
  corpus entry, not merely record the WP as closed** (see F-205).
- **Unit 07's recon corrects two file names in the breakdown**: `db/game_visibility.rs`
  and `controller/import_game.rs` **do not exist**; the real paths are `db/visibility.rs`
  and `bin/import_game.rs`.
- **`00-breakdown.md`'s sizing premise was wrong four times.** Unit 08: 91 of WP-52's 95
  files are `.sqlx` cache JSON, leaving 9 Rust source files at +162/-80 - the
  "mostly-deletions consolidating duplicated code" framing is false and the pattern-4e
  deletion-risk surface is near zero. WP-66 is **12** real files, not 101. WP-73 is 139
  files of which **135 are three-line wrappers**. Unit 10b's "43 Deployments vs 26 image
  stages" premise is refuted separately (R4). **Do not size remediation work from
  `00-breakdown.md`.**

**Process items.**

- `9ba3736b` bundles 224 lines of `docs/reviews/.../planning/` state into a code commit -
  the same mixed-commit pattern the breakdown flagged for `62b293df`.
- Unit 05b recommends **mandating finding-id citation at fix sites**. This is the cheapest
  single process fix the session produced: it would have made most of the "did this commit
  close that finding" work in this session mechanical.
- `08-web-stats-query-perf.md:572-578` has duplicate empty `Verified good` and
  `Coverage gaps` headings marked `_(pending)_` that **shadow the real populated sections
  above them**. A template artifact, not missing content - do not read that unit as having
  no verified-good section.
- Session continuity, for anyone auditing the audit: **Unit 06 completed on its 4th
  attempt** (F-111..F-115 were salvaged from attempt 3), and **Unit 07b died to quota
  exhaustion mid-unit before producing any finding** and was re-dispatched from scratch.
  Both units' surfaces were walked to completion exactly once.

### 7.8 Items with no owner at all

No unit owns these, no work package covers them, and no remediation route has been
assigned. **This is the actionable output of the section.**

| # | Item | Citation |
|---|---|---|
| U1 | `roll-through-the-ages-2` has never had a crate-level review - 3,290 lines, no `validate`, no redaction test, F-83 found in the one function anyone read | `00-STATE.md:225-228` |
| U2 | `Gamer::points()` ordering contract and `cathedral-2`'s inverted sign - no F-number anywhere; Unit 08 handed it back | `cathedral-2/src/lib.rs:580-584`; F-58 |
| U3 | No request-parts test harness - structural cause of F-92, reason F-85 was uncatchable | `rust/web/src/auth/server.rs` |
| U4 | `require_admin`'s true path untested for 13 of 16 server fns | WP-37 |
| U5 | Nothing tests the vendored `rust/lib/session_store` - authentication-adjacent, first-party, home of F-200 | `rust/lib/session_store` |
| U6 | `left_at` conflates elimination with leaving; four writers; F-113/F-116/F-117 are symptoms of one schema change | `rust/web/src/db/` |
| U7 | Vendored MIT obligations are hand-satisfied and unverifiable by tooling; a one-line CI grep closes it | `deny.toml`, R9 |
| U8 | The e2e job is `continue-on-error: true` - nothing in it gates a merge or deploy; the hydration race is untracked | compounds F-211 |
| U9 | `auth/server.rs:92` increments `login_emails_sent_total` before the send with no failure counter - the shape `wfe F46` fixed, outside WP-60's scope | `auth/server.rs:92` |
| U10 | `rust/bot/src/routing.rs` was never picked up by any sub-unit | `rust/bot/src/routing.rs` |
| U11 | Zero tests on `build_messages`; the bot crate has no DB tests at all | `rust/bot/` |
| U12 | Nothing asserts that opponent hidden state is absent from the rendered bot prompt | `fetch_game_data` test |
| U13 | `rust/operator/src/controller.rs` - `cleanup` has zero test callers; no already-newest-to-deprecated test | `rust/operator/src/controller.rs` |
| U14 | `i-noreply@brdg.me` not short-circuited in the inbound router - two-line fix | `inbound.rs:95-96`, `:856-866` |
| U15 | `WP-46`'s `proposals.rs` test module and `email/sweep.rs`'s test module were never read | WP-46, WP-38 |
| U16 | `admin.rs:1560-3488`, `db/game_write.rs` and `update_game_command_success` were never read at line level | `rust/web/src/` |
| U17 | WP-36 has neither spec nor checklist row, so Unit 05's crypto verdict rests on a commit message | `00-STATE.md:28` and part 2 |
| U18 | WP-10 3a's "13 crates with no redaction test" | section 6.4 |

**Two open questions this section could not resolve from its sources.** Whether Unit 07b
closed the entire WP-51/WP-53 surface or only the part its single successful pass reached.
And whether `F-59` is excluded by the `lords-of-vegas-1` WIP ruling, which names only F-50
and F-57 - that one is carried in the open-questions list rather than here, because it is a
status question, not a coverage gap.

---

## 8. Discrepancies and corrections to the record

The three normalized parts recorded conflicts rather than resolving them, on the
rule that the composition Lead would rule. This section is that ruling. Every item
below is **settled**. Where a unit report, `00-STATE.md` or `00-breakdown.md`
disagrees with what follows, this section is authoritative and the other document
is to be read as corrected.

### 8.1 F-78 was claimed twice - one claim was never valid

**F-78 is 04c's finding: Low, live.** WP-33 (`f F36`): the `saturating_add`
overflow fix landed in `greed-2` only, skipping the raw `scores[player] +
turn_score` in its own log text, and `farkle-2` - a near line-for-line twin - got
neither half. Citations `rust/game/greed-2/src/lib.rs:365`,
`rust/game/farkle-2/src/lib.rs:260`, `:310`, `:313`.

**04b's "F-78" is VOID.** It was a WITHDRAWN item to begin with - the reviewer's
own `f F27` negative-footer theory for `category-5-2/src/render.rs:115-122`,
disproved and retained only so it would not be re-filed - and it was issued outside
04b's allotted range of F-66..F-77, so it was never validly issued. It is not a
competing finding, it is a numbering error inside a withdrawn item.

**F-78 has exactly one occupant.** Nothing was renumbered and nothing needs to be.
The withdrawn 04b item survives only as a refutation note in that report's
Verified-good section and **must not be cited by an F-number anywhere**. Any
reference to "the other F-78" is a reference to an ID that does not exist.

### 8.2 The routing leak is one High and one Medium, not two Highs

`00-STATE.md:130-133` states the pattern as "Three confirmed cases (F-55, F-57,
F-60); two are High." **The per-finding ratings win.**

| ID | Rating | Status |
|---|---|---|
| F-60 | **High** | live - `lost-cities-1`'s `player_state()` raw-indexes `self.hands[player]` with the bounds fix routed to WP-09, and the crate has no `validate()` |
| F-55 | **Medium** | live - `cathedral-2`'s deferred `played_pieces` indexing, routed to WP-09/09b, picked up by neither |
| F-57 | Low | **excluded** - `lords-of-vegas-1`, WIP-crate ruling, not routed to remediation |

The ratings were assigned after reading the end state. `00-STATE.md`'s pattern
paragraph is a running summary written while the ratings were still settling, and
it re-derives no severity of its own - it asserts a count. A count is not evidence
against a rating.

**The live routing-leak set is F-60 (High) and F-55 (Medium): one High, not two.**
`00-STATE.md:130-133` is corrected accordingly. **The pattern itself is
unaffected** - three cases occurred, all three are real instances of
deferral-without-a-receiver, and P5 in section 4 stands as written. Only the
severity mix was mis-summarised.

### 8.3 F-104: pattern 4f is the correct label, and both citations stand

**Label: 4f, not 4b.** `00-STATE.md:177-181` names 4f explicitly and gives the
reason it is worth naming separately from 4b - "the test is not wrong about its own
function, it is wrong about the system." F-104 is 4f's defining instance:
`validate_bot_slots_accepts_case_mismatch` pins case-insensitive bot-name
validation as intended while every consumer of the stored value matches
case-sensitively. 05b's "pattern 4b" label is superseded.

**Citations: the union is correct.** The reports cite `rust/bot/src/config.rs:28`;
`00-STATE.md` additionally cites `rust/bot/src/config.rs:67`, which no unit report
cites. `:67` is not an error - **F-189 independently established it as a second
case-sensitive lookup site.** Both citations stand. The report-side citation was
incomplete, not wrong, and a fix touching only `:28` does not close the finding.

**Remediation grouping: one item.** **F-104 + F-138 + F-183 + F-189**, one
bot-name case-sensitivity defect spanning Units 05b, 07, 09c and 10a, plus
**F-185 re-fixtured in the same change** - its all-lowercase fixture is what made
lowercasing and canonicalising indistinguishable. The fix is to canonicalize inside
`validate_bot_slots` and return the canonical name; the silent ack at
`rust/bot/src/main.rs:186-194` and `admin::create_bot`'s arbitrary-casing
precondition (`admin.rs:293-303`) belong in the same change. 09c's report states
the pairing as three items; that understates it.

### 8.4 `deny.toml`'s skip list has 29 entries, not 24

The "24" figure in the 10b carry-forward (`00-STATE.md:373`) and at 10b's own WP-72
section is **stale**. `00-STATE.md:337` already issues the correction to **29**,
and the same report states 29 at F-206 and at its Coverage gap 3. **29 is
correct.**

This is load-bearing, not cosmetic: **F-206 depends on the 29 figure.** WP-69's
spec §3b set a stop-and-report threshold at "roughly a dozen" skips; 29 crosses it
unambiguously. Anywhere "24" appears in this corpus in connection with `deny.toml`,
read 29.

### 8.5 Report 07 covers 23 findings, and its own ordering is not usable

Report 07 covers **F-121..F-143 - 23 findings**. Its progress line
(`07-web-domain-remainder.md:22`) says **13**. **23 is correct**; the header is a
miscount and **must not be quoted**.

Report 07 also records its findings non-monotonically (F-121, F-122, F-129..F-132,
a REFUTED block, F-133, an ARCHIVED block, F-124..F-128, then F-134 onward).
**Section 11's table is the canonical ordering and the canonical count** for this
range, as it is for every range. Extraction was by ID, never by position.

### 8.6 F-123 / F-132 / F-133: one refutation, one remnant, one salvage

| ID | Status | What it is now |
|---|---|---|
| F-123 | **REFUTED / ARCHIVED** | The claimed `VisibilityCache` cross-user visibility leak. Refuted by the report and `00-STATE.md` in agreement: each `VisibilityCache` instance is a plain local inside the per-request spawn at `events.rs:65`, so one instance = one connection = one viewer. **Not a live finding.** |
| F-132 | **live, Low** | The downgraded remnant. `VisibilityCache` keys on an id alone (`visibility_cache.rs:11`), correct **only** because each instance is owned by exactly one SSE task with one fixed viewer - an invariant nothing in the type expresses. Fix: document the per-viewer ownership requirement, or key on `(id, Option<Uuid>)`. |
| F-133 | **live, Low** | The salvaged secondary half. `is_proposal_visible_to_user` (`db/proposals.rs:40-52`) grants visibility only via a `game_proposal_players` row and never consults `game_proposals.owner_user_id`, so an owner not also inserted as a player cannot see their own proposal. Untested in either direction - both tests (`:172`, `:193`) add the owner as a player explicitly. WP-42, attribution inferred. |

**F-123 stays in section 11's table**, marked REFUTED/ARCHIVED, exactly as section
6 treats every refutation: **the row exists so nobody re-derives it.** The same
pass also refuted the claim that WP-42 was reverted by the SSE migration - a useful
negative against pattern 4e, likewise preserved.

### 8.7 F-110 is not a defect

**F-110 is informational and states a positive result.** WP-37's inline `ws Fnn`
citations at the fix sites are the only sign-off trail that exists anywhere in Unit
05, and following it showed the three apparently uncited findings (`ws F24`,
`ws F32`, `ws F33`) were all in fact fixed - **14 of 14, the only Unit 05 work
package to deliver in full** (`rust/web/src/admin.rs:1688`, +2 sites).

It **keeps its ID and its row**: it is the concrete evidence behind section 7.7's
recommendation to mandate finding-id citation at fix sites, and the direct contrast
with F-109, which shows what happens where no such trail exists. It is **excluded
from the severity tally and from the remediation order**. Its "Low" severity column
is an artifact of the row template, not a rating.

### 8.8 F-161a..F-161d are hereby declared

The sub-letters are cited throughout the corpus - `00-STATE.md`, `00-HANDOVER.md`,
`09-web-frontend-email-sse.md:856` ("the F-161d class") and section 3 of this
report - but Unit 09a's finding body used bare `(a)`-`(d)` and never declared them
as IDs. **They are declared here.** All four are Unit 09a, WP-56,
`rust/web/src/email/inbound.rs`.

| ID | Site | Content |
|---|---|---|
| **F-161a** | `:719-723` (+4 sites) | `AuthVerdict::Unknown` proceeds on a `warn!` only, and `Unknown` is returned whenever the authserv-id is not exactly `amazonses.com`. The pipeline is **Resend, not SES** - a different authserv-id makes the whole gate inert in production. No test against a captured real message; no metric or alert on `Unknown`. |
| **F-161b** | `:213-218` | `Pass` means "not explicitly failed": `failed(dmarc) \|\| (failed(spf) && failed(dkim))` inverts the DMARC rule, so `spf=fail; dkim=none` is accepted. **The cleanest row** - unconditional forgery derivable from the file alone, no deployment assumption. Also passes `dmarc=none`, `spf=softfail; dkim=none`, and `spf=neutral/none/permerror/temperror`. |
| **F-161c** | `:170-178` | The topmost-header rule defends only against an *added* second `Authentication-Results`; since `Unknown` proceeds, an attacker-supplied sole header is honoured verbatim. Depends on F-161a. |
| **F-161d** | `:1794-1808` | The two tests named for the lenient boundary are decoys - each input carries an independently passing result, so the "nothing authenticated" cases are untested. Cited in `00-STATE.md` as what makes decoy tests a confirmed **class**: the F-151 decoy family crossed with pattern 4f. |

**They roll up into parent F-161 (High)** and carry no severity of their own. They
are **not counted separately** in section 2's distribution - the four rows exist so
each fail-open path can be closed and verified individually, not to inflate a
count. F-161's own severity is **High**, flat; the report heading's "High, and it
escalates F-129 + F-130" is normalized to High with the escalation carried in the
status column. The same normalization applies to **F-205** ("Low, but a NEW NAMED
PATTERN" -> **Low**, pattern claim in the status column) and to **F-208a**, which is
a sub-letter for a REFUTED carried premise rather than a defect and is likewise
uncounted.

### 8.9 F-59 is excluded by the `lords-of-vegas-1` WIP ruling

The ruling (`00-STATE.md:197-201`) excludes findings about **missing or incomplete
functionality** in that crate. It names F-50, F-57 and the no-`finished`
observation, but **the test is by kind, not by ID** - the named list is
illustrative.

Apply it. F-59 says `lords-of-vegas-1` holds genuinely secret state (deck order and
the `Card::GameEnd` position) yet has no test calling `pub_state()` at all, so the
"`PubState` must never carry `deck`" invariant is held only by nobody having added
the field. The row states it explicitly: **regression-risk, not a live leak - the
redaction is currently correct.**

**F-59 asserts no defect in behaviour that exists.** It is an absent-artifact
finding about a work-in-progress crate, which is exactly the kind the ruling
excludes. **F-59 is excluded and is not routed to remediation.**

Two qualifications. First, F-59 **remains a counted occurrence of the WP-10 3a
gap** - "declared for every game crate, applied to 3 of 28, 13 crates with no
redaction test" is a programme-level pattern (P15) and the WIP ruling does not
shrink its denominator. Second, the finding is filed inside 03b's "Verified good"
section, after F-58, rather than under `## Findings`; **section 11's table is where
it lives**, and a reader scanning 03b's Findings heading will not see it.

### 8.10 `00-breakdown.md`'s premises were wrong four times - a scoping finding

Detail is in section 7.7. The ruling, compactly:

| Unit / WP | Breakdown premise | Reality |
|---|---|---|
| Unit 06 | a "shared-core extraction" gotcha | **False premise** - `9ba3736b` touches **zero** `rust/game/*` files |
| WP-52 (Unit 08) | 95 files, "mostly deletions consolidating duplicated code" | **9 Rust source files**, +162/-80; the other 91 are `.sqlx` cache JSON. Independently re-confirmed by part 2's extraction. The pattern-4e deletion-risk surface is near zero |
| WP-66 | 101 files | **12** real files |
| WP-73 | 139 files | **135 of them are three-line wrappers** |

**The finding is about how the review was scoped, not about any one unit.** Unit
sizing was derived from raw file counts that nobody checked against content, and
**in every one of the four cases the error inflated the apparent work** - never
deflated it. Two consequences follow. Units were briefed to expect surfaces that
did not exist, and at least one gotcha (Unit 06's) was a hunt for a change that had
not happened. And a reader of the breakdown alone would conclude WP-52, WP-66 and
WP-73 were among the largest packages in the programme when they are among the
smallest.

**Do not size remediation work from `00-breakdown.md`.** Size it from the file
lists in the unit reports and from section 11.

### 8.11 The remaining conflicts, resolved

Every discrepancy the three parts recorded that this section has not already ruled
on. The parts applied `00-STATE.md` precedence during extraction; those
applications are confirmed here, not re-opened.

**Handling changed by owner ruling or by later evidence:**

| ID | Filed as | Resolved |
|---|---|---|
| F-81 | 04c, Low, asks for a ruling | **Intended behaviour, not a finding.** `00-STATE.md:207-214`: reconstructing hidden information from the public log is acceptable generally. Severity recorded verbatim; not routed |
| F-50, F-57 | 03b, live with remediation steps | **WIP crate, excluded** (`00-STATE.md:196-201`). Not routed |
| F-35, F-41, F-58 | unit reports propose fixes | **Parked in WP-20 (`c F12`)**, 24 sites across 21 crates. Record occurrences; do not re-raise per crate. `hanamikoji-1` populating `stats` is a **negative** for this tally |
| F-15 | 01b, live Medium needing a test | **DISCHARGED, stays LATENT.** No live violation at the real emitter, which is `rust/web/src/theme.rs`. Do not re-run the sweep |
| F-96 | 05a, High, code defect | **DOWNGRADED**: resolved out of band, **not a code defect**, retained as a pre-rollout deployment blocker plus pattern 4d |
| F-109 | 05b prescribes restoring the `TaskTracker` drain plus an SSE test | Remediation is a **bookkeeping fix on WP-36's row plus a decision on the never-implemented second half of `ws F55`** - explicitly **not** a revert of `efad81f9`. Narrowed further: the shutdown concern does not apply to the bot binary, which drains in-flight turns, so the open half is the **email sweep task only** |
| F-128 | 07, fails-closed negative result | **NOT closed, no owner** (via 09b's F-173): `İ@example.com` breaks the SQL-`LOWER` vs Rust-canonicalize divergence. Fold with F-173 and the `CanonicalEmail` newtype into one item |
| F-129, F-130 | 07, Medium with an explicit escalation condition | **The condition fired** (F-161). Both **ESCALATED to account takeover**; the in-report Medium is superseded |
| F-142 | 07, a test that exists with one vacuous assertion | Both framings stand: its 403 half is real, and the row counts in the falsified `Test? y` tally. See section 5.2 |
| F-170 | 09b, filed broadly | **Scope narrowed**: 09c refutes its extension to the game-start mail, which reads `turn_emails_enabled` directly |

**Record corrections:**

- **F-18's unmigrated-crate list is five crates, not four.** `battleship-2` was
  added by F-71 - "the list was one short". `00-STATE.md:579-582` is
  authoritative. **`hanamikoji-1` does not join this list**: Unit 11 refuted the
  carried premise that its epilogue is unguarded - the gate exists at `lib.rs:796`
  / `:830-834`, identical to `jaipur-2`, with a dedicated regression test.
- **F-72a keeps its High.** 04b rates it High; `00-STATE.md` cites it as a
  pattern-4b instance without restating a severity. Absence of corroboration is not
  a conflict.
- **F-82 stays a pattern 2b instance** as 04c filed it. `00-STATE.md`'s 2b instance
  list (F-66/F-67/F-68/F-76) is illustrative, not a closed enumeration.
- **F-176 is one ID covering four falsified checklist rows.** Any per-ID count of
  "`Test? y` with no test" undercounts by three. The tally is **nine** - section 5.2.
- **Compound severities are normalized.** Five rows carried qualified severities
  (`Low/Medium`, `Low, note`, `Low, informational`, plus the two conflict rows).
  Section 2's distribution uses the normalized value; section 11 preserves the
  qualifier in the status column.
- **Inferred WP attributions are marked inferred, never guessed.** F-02, F-04,
  F-06, F-35, F-42, F-43, F-63, F-64 carry `-`; F-61's WP is inferred from prose
  alone; F-104, F-105, F-107, F-108 and F-133 are inferred from position (F-133
  assigned WP-42 as the salvaged half of F-123).
- **Citations that are not `file:line` are recorded as they exist.** F-31 and F-33
  are crate-level ("no `fn validate` anywhere in the crate"); F-17's citation is a
  table of 8 rows and 18 constructs, of which `:210` was taken as primary; F-19's
  four non-jaipur crates and F-20's `finish_epilogue` are cited by file only; F-47
  cites a test count. No line data exists to extract for these - **a missing line
  number is not a missing finding.**
- **F-207, F-210 and F-211 sit outside their unit's commit scope.**
  `rust/Dockerfile:132` is untouched by any commit in the 127-commit range, and
  `sushi-go-2` and the web e2e suite fall under Unit 11's "unassociated tail". The
  Unit column is a poor locator for these three; use the citation.
- **Two file names in the breakdown do not exist**: `db/game_visibility.rs` and
  `controller/import_game.rs`. Read `db/visibility.rs` and `bin/import_game.rs`.
- **`finish_epilogue` is a per-crate inherent method in 12 crates**, not a
  `rust/lib/game` helper; only `placings_log` is shared. Any remediation premised
  on a single shared implementation is wrong.
- **One Worker claim is explicitly wrong and must not be carried forward**:
  `docs/porting/GAME_PORTING.md:215` does **not** cite a non-existent package
  `brdgmen` - it reads `cargo run -p brdgme_repl`, which matches the crate.
- **`08-web-stats-query-perf.md:572-578`** has duplicate empty `Verified good` and
  `Coverage gaps` headings marked `_(pending)_` that shadow the real populated
  sections above them. A template artifact - that unit is not missing those
  sections.
- **The ID sequence is otherwise clean.** F-01..F-211 is complete: no gaps, no
  duplicates, every ID appearing exactly once as a finding heading in exactly one
  report, the void 04b "F-78" notwithstanding. Sub-letters are exactly F-72a,
  F-161a/b/c/d and F-208a. Ordering anomalies exist in `01b` (F-17, F-16, F-15,
  then F-09..F-14) and in `07`; **no ID is missing in either.**
- **The `for-sale-2` carry-forward is answered.** `pass()`'s half-bid rounding
  **is** inside the WP-11 park (`f F14`, BLOCKED-ON-USER-RULES-REVIEW, D-30 + D-35).
  Deliberately not fixed; **not** a remediation gap. WP-11 also parks parity items
  in four of the five WP-33 crates, so no parity observation in `greed-2` /
  `farkle-2` / `no-thanks-2` / `liars-dice-2` / `zombie-dice-2` should be raised
  without first checking `f F2/F15/F21/F33/F43/F50/F54`.

### 8.12 Unresolved

One item cannot be resolved from the normalized sources, and is stated as open
rather than guessed:

- **Whether Unit 07b closed the entire WP-51/WP-53 surface, or only what its single
  successful pass reached.** Unit 07 did not audit WP-51 (`dcd8844c`) or WP-53
  (`3610b957`) - six `RealInviteMailer` methods, the `spawn_sweep` collapse and
  `notify_owner_decline`'s new gating. Unit 07b was dispatched for exactly that
  surface, **died to quota exhaustion mid-unit before producing any finding**, and
  was re-dispatched from scratch, producing all six of its findings on the re-run.
  That re-run walked the surface once. Nothing in the corpus states whether the
  single pass covered the whole surface or stopped where its budget did.
  **Resolving this requires re-walking `dcd8844c` and `3610b957`, not re-reading
  the reports** - it is a remediation-plan input, not a record correction.

---

## 9. Owner decision items

These are the owner's calls, not a reviewer's. Nothing below is a defect awaiting
a fix; each is either a policy question a Lead must not decide, or a standing
ruling that already closed a question and must not be re-opened.

### 9.1 Open - awaiting an owner decision

**The vendoring policy question.** Open.

The owner's position: vendoring third-party code should be **forbidden except
where the work is genuinely blocked and there is no alternative**. The review has
already established the facts; they are not in dispute and must not be
re-derived.

| Fact | Status |
|------|--------|
| WP-66's spec gated vendoring | **Yes.** Step 0 was binding: bump and re-resolve first; if that put every crate on one sqlx major the spec collapsed to 3a with an explicit "do NOT vendor anything". |
| The gate was honoured | **Yes.** `tower-sessions-sqlx-store` 0.15.0 pins `sqlx = "0.8.0"` upstream, so no sqlx-0.9-compatible store release existed and branch 3b was correctly live. |
| The port itself | **Minimal and faithful**, verified by direct diff against the registry copy. MIT licence and attribution present. Schema unchanged. |
| It still produced F-200 | **Yes.** The "minimal port, not a rewrite" criterion - correctly followed - *guaranteed* the upstream defect came along with the code. It is now first-party code in an authentication-adjacent path with no tests. |
| The MIT obligation is machine-checked | **No.** `rust/deny.toml:45-49` sets `[licenses.private] ignore = true`; the crate is `publish = false`, so cargo-deny skips it entirely. The obligations are satisfied by hand only. |
| Repo-wide scope of vendoring | **Never swept.** Only `session_store` is known. No sweep for other vendored or copied third-party code has ever been run. |

The decision to make: **whether "no compatible upstream release yet" is
sufficient grounds to vendor at all.** The alternatives the spec did not weigh
are waiting for the upstream release, pinning the old major across the
workspace, or upstreaming a patch. The trade-off, stated without deciding it:

- **Vendoring** unblocks immediately and is cheap up front. Its costs are the
  ones this session measured - an inherited upstream defect (F-200) that
  "minimal port" made unavoidable, permanent maintenance of code nobody on the
  project wrote, and a licence obligation outside the tooling's reach.
- **Waiting or pinning** keeps the dependency someone else's problem and keeps
  cargo-deny authoritative, at the cost of blocking the work package or holding
  a stale major across crates that did not need to be held.
- **Upstreaming** is the only option that removes the defect at the source, and
  the only one whose cost is unbounded in time.

Whichever way the policy lands, two items follow from it mechanically and should
be scheduled with it: **the repo-wide vendoring sweep**, and **removing
`session_store` from cargo-deny's blind spot** so the licence obligation is
checked rather than remembered.

**Other open owner-level questions.**

- **Scope, `roll-through-the-ages-2`.** 3,290 lines, never crate-level reviewed,
  no `validate` override, no redaction test; the one function anyone read
  contained F-83. Correctly out of scope for a review-of-the-remediation. Whether
  to fund a dedicated pass is a budget call (section 7.2).
- **Scope, WP-51/WP-53.** Section 8.12 records the one item the record cannot
  resolve: whether Unit 07b covered the whole surface or stopped where its budget
  did. Resolving it means re-walking `dcd8844c` and `3610b957`. Fund it or accept
  the gap knowingly.
- **`ALLOW_INSECURE_DEFAULT_KEY` currently disables two unrelated guards.**
  Splitting it was suggested by the F-96 investigation and is not urgent. It is a
  design call, not a defect.

### 9.2 Standing owner rulings - do not re-litigate

Each of these is settled. They are recorded here so that no remediation work
package, and no future review, re-opens one.

**F-81 - reconstructing hidden information from public logs by inference is
acceptable by design.** A great deal of hidden information is reconstructible
from legitimately public log entries; this is equivalent to reconstructing it
from memory, and brdgme does not intend to defend against it via ephemeral
logging or any similar mechanism. F-81 is **not a finding** - it is intended
behaviour. **This ruling is general, not `no-thanks-2`-specific.** It does not
excuse hidden information appearing *directly* in `Log::public` content: **F-22
and F-28 remain valid findings.** The distinction any future unit must apply is
exact - direct leak into public content is a finding, inference from public
content is not.

**`lords-of-vegas-1` is work in progress.** No findings about missing or
incomplete functionality there. Its missing endgame (it never assigns
`finished = true`) is out of scope. **F-50 and F-57 are excluded**, as is F-59
per section 8.9.

**F-35 / `Status::Finished { stats: vec![] }` is parked in WP-20 (`c F12`).**
24 sites across 21 crates. Record occurrences; do **not** demand per-crate fixes
or re-raise it crate by crate.

**F-96 is downgraded to a pre-rollout deployment blocker, not a code defect.**
The startup `panic!` (`rust/web/src/main.rs:40-45`) is gated by
`ALLOW_INSECURE_DEFAULT_KEY`, which dev and CI already set
(`k8s/dev/web-patch.yaml:18-19`, `scripts/rust-test.sh:64`). Turnstile
verification **fails closed on every error path**
(`auth/server.rs:256-277`); the sole fail-open is `secret.is_empty() -> true`,
which is exactly what the panic prevents. The code implements the house pattern
correctly. What remains is that no manifest sets the var in prod - a deployment
item.

Two companion deployment items were produced by the same investigation and are
**not optional extras**:

- **`TURNSTILE_SITE_KEY` has no startup check** and silently defaults to empty -
  no widget renders and every login is rejected. Setting only the secret key is a
  **total login outage**, so both keys must land in the same change.
- **`config::public_base_url()` defaults to `http://localhost:3000`**, which
  makes WP-58's `List-Unsubscribe` non-HTTPS and **RFC 8058-invalid in
  production**.

**F-207 belongs to the same deployment-checklist family** (three sqlx migrators
writing `_sqlx_migrations` at differing pinned versions). Treat F-96, its two
companions and F-207 as one pre-rollout checklist, not as four code findings.

**D-39 is unverifiable, and that is the finding.** Its only record is a one-line
`SUMMARY.md` entry plus the commit author's gloss; `docs/CODING.md` has no rule
bearing on delete-vs-rewrite. Do not attempt to verify it again.

---

## 10. The sign-off rule to recommend

**No finding may be marked closed until four checks pass against the end state
at HEAD: the cited artifact still exists, it is reachable from live code, any
regression test credited to it actually calls the function under test, and any
finding whose premise was disproved has been amended in the corpus rather than
merely closed.**

Each of the four teeth was learned from a real decoy in this session. None is
hypothetical.

**Tooth 1 - the citation must still exist.** Learned from **F-109**. WP-36
shipped the `ws F55` shutdown drain (`TaskTracker`, `drain_ws_tasks`, a bounded
5s wait) *plus* a dedicated regression test at
`rust/web/tests/websocket_hygiene.rs`. The later SSE migration `efad81f` deleted
the fix and the test **together**. The checklist row and both commits still read
as closed. A landed, tested fix was silently reverted inside the same programme
and nothing in the sign-off trail noticed.

**Tooth 2 - the citation must be reachable.** Learned from **F-147**.
`notify::send_turn_reminder` (`rust/web/src/email/notify.rs:523-543`) exists, has
**never had a caller**, and its doc comment states wfe F36's dedup as
accomplished fact. The checklist records wfe F36 closed. Presence alone satisfies
tooth 1 and still proves nothing - tooth 1 without tooth 2 is defeated by any
dead-at-birth function with a confident doc comment.

**Tooth 3 - the regression test must actually call the function under test.**
Learned from three independent instances:

| Instance | The decoy |
|----------|-----------|
| **F-151** | `wd F48`'s missing test. The half-applied game-type filter on a FULL OUTER JOIN returned another game type's rating on a public endpoint; the test the checklist implied would have caught it. |
| **F-161d** | The two tests named for the lenient DMARC boundary each carry an input with an independently passing result, so the "nothing authenticated" cases - the whole point - are untested. |
| `rating_before_aggregates_exclude_nulls` (`stats/queries.rs:1287-1346`, unnumbered) | Name-matches `wd F51`'s risk **exactly**, never calls `game_history`, and asserts PostgreSQL aggregate semantics instead. |

Decoy tests are a **confirmed class in this session, not a set of incidents**.
A test whose name matches the risk is the single most reliable way a false
sign-off survives review, because the name is what a grep-based check reads.

**Tooth 4 - a finding whose premise is disproved must be amended, not merely
closed.** Learned from **F-205**. `dp F12`'s premise - that sentry drags
actix-web and ureq into every build - **was never true**. WP-67's own rider 2
required that the downgrade be written back into the finding. It never was:
`docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46,139` and
`findings/dependencies.md:103-108,157` remain unamended today. This is distinct
from a doc edited to agree with the code - here the docs were **never edited at
all**, despite an explicit criterion requiring it. The corpus therefore still
asserts a mechanism that does not exist, and the next person to read it will
re-derive a finding from a falsehood.

**What the rule costs.** Four mechanical checks per closed finding, and **all
four are greppable**: does the cited path exist at HEAD; does the cited symbol
have a caller; does the credited test file name the function under test; does the
corpus entry carry an amendment when the closing commit disproved its premise.
None requires re-reading the subsystem. The expensive part of this session -
establishing that a checklist row and the end state disagreed - is precisely what
these four checks make cheap.

**The cheapest complementary fix.** Unit 05b's recommendation to **mandate
finding-id citation at fix sites** (section 7.7). It is the single cheapest
process change the session produced, and it is what makes the rule above fully
mechanical: with finding ids present in the code, all four teeth reduce to greps
over the repo rather than judgement calls over the checklists. F-110 is the
proof - WP-37's inline `ws` finding citations were the only sign-off trail that
survived contact with this review, and they made that work package the one Unit
05 could verify in full at negligible cost.

---

## 11. Full findings table

All 219 rows, concatenated from `90-findings-part1.md`, `90-findings-part2.md` and
`90-findings-part3.md` in ID order. This is the canonical ordering and count: report
07's header count of 13 is wrong, and several unit reports order their findings
non-monotonically (see section 8). Sub-lettered rows (F-72a, F-161a..d, F-208a) and
the two unnumbered rows carry their own entries, which is why 219 exceeds the 211
F-numbers. The two `(no F-number assigned)` rows sit immediately after F-96, the
parent their source rows state; they are not moved to a trailing group.

The three parts share an identical header, so no column reconciliation was needed.

| ID | Severity | Unit | WP | file:line | Summary | Pairing / status notes |
|----|----------|------|----|-----------|---------|------------------------|
| F-01 | Low | 01a | WP-02 | `rust/lib/markup/src/lib.rs:43-56` | `from_string` keeps a `(Vec<Node>, &str)` return whose `&str` is unconditionally `""` on success, a dead value every caller destructures. | Causally upstream of F-17: the same WP-02 change made `repl.rs`'s `from_string(..).unwrap()` strictly more likely to fire. |
| F-02 | Low | 01a | - | `rust/lib/markup/src/parser.rs:737-757` | Both overflow tests assert `is_err() \|\| <no node>`, a disjunction that passes under either outcome and pins no behaviour. | - |
| F-03 | Low | 01a | WP-04 | `rust/lib/game/src/command/parser/mod.rs:45-62` (+1 site: `suggest.rs:39-52`) | `Token::parse` compares a token-byte-length slice while `Spec::Token`'s suggester compares folded prefixes; they diverge when folding changes byte length. | - |
| F-04 | Low | 01a | - | `rust/lib/markup/src/wrap.rs:14-24` | The ls F8 fix recounts `chars().count()` per word, making `wrap_segment` O(n^2) in segment length. | Report states it is not a live problem for game logs/rules text. |
| F-05 | High | 01a | WP-09a | `rust/game/for-sale-2/src/lib.rs:130-135` (+1 site: `:144-149`) | The underflow guard returns an empty log vector, leaving a short-deck game reporting `Active` with no legal move - a permanent wedge, worse than the panic. | `for-sale-2`'s WP-09b `validate()` (`:389-414`) checks only per-player vectors, so the silent guard is the sole defence. Cited again by F-18 as the crate whose command gating is known loose. |
| F-06 | High | 01a | - | `rust/lib/game/src/game.rs:106-108` | `Gamer::validate` defaults to `Ok(())`, so the D-36 "deserialized state is not trusted" boundary is fail-open for the 13 of 28 crates that never override it. | 00-STATE systemic pattern 6 (High): crate list in `00-sweeps.md`; carried into Units 02-04 briefs. 00-STATE pattern 2b is a distinct failure mode (override exists but insufficient - F-66/67/68/76). Report heading says "15 of 27" while its body and grep say 15 of 28 / 13 without. |
| F-07 | Low | 01a | WP-09b | `rust/game/no-thanks-2/src/render.rs:74-77` | `current_card.unwrap()` became `if let Some(..)`, so an invariant-violating unfinished state renders with the card line silently dropped and `validate()` does not catch it. | - |
| F-08 | Medium | 01a | WP-04 | `rust/lib/game/src/command/parser/mod.rs:1095-1100` (+3 sites: `chain.rs:63-65`, `:118-120`, `:177-179`) | The typed/spec `expected()` parity fix changed only `CommandSpec::Chain`; `Chain2/3/4::expected` still return `a.expected()`, so a `Chain2<Space,_>` still disagrees. | Parity assertion cannot catch it - no committed test builds a `Space`-leading chain. |
| F-09 | High | 01b | WP-07 | `rust/lib/rand_bot/src/lib.rs:33` (+1 site: `:13` reached from `:70-72`) | `Spec::Int` with `min > max` panics and `Many { min: Some(5), max: None }` trips `assert!(min <= max)` - the degenerate-spec class WP-07 claimed to fix, wire-reachable. | Compounds with F-10. Report notes `lib/game`'s own `Many` was changed in WP-03/04 to degrade gracefully, so library and bot now disagree. Coverage gap: `rand_bot`'s `fuzz`/`Fuzzer` path also reaches these (WP-63, Unit 10). |
| F-10 | Medium | 01b | WP-07 | `rust/lib/rand_bot/src/lib.rs:70-71` | `min.unwrap_or(0) as i32` / `max.unwrap_or(3) as i32` wrap for values above `i32::MAX`, turning a large `min` negative so the bot emits a command violating the spec minimum. | Must be reasoned about with F-09 - the wrapped negative `min` is what passes `assert!(min <= max)`. |
| F-11 | Medium | 01b | WP-07 | `rust/lib/game_client/src/lib.rs:192-195` | `Response::UserError` is unhandled in `request_with_config`, flows out as `Ok(other)` and is reported as `UnexpectedResponse`, discarding the service message. | - |
| F-12 | Medium | 01b | WP-07 | `rust/lib/game_client/src/lib.rs:57-65` (+2 sites: `:119-169`, `:181-186`) | The timeout ceiling is per attempt, not per call: 3 attempts plus backoff plus a fourth ceiling on the body read (~6 min), and `fetch_game_data` issues up to four such calls. | Severity depends on the `reqwest` client timeouts in `rust/web` (10s) and `rust/bot` (60s), which 01b did not confirm - Units 05/09/10. |
| F-13 | Low | 01b | WP-07 | `rust/lib/game_client/src/lib.rs:104-117` | `validate_version_name` says "must be a DNS label" but accepts uppercase and leading digits, so `Acquire-1` passes and silently 404s at the KEDA interceptor. | - |
| F-14 | Low | 01b | WP-07 | `rust/lib/game_client/src/lib.rs:435-442` | `test_retry_on_connect_refused_then_success` races a 15ms sleep against a 20-40ms jittered backoff; the flake was recorded as "accepted (ls F37)" with the fix spelled out but not applied. | - |
| F-15 | Medium | 01b | WP-05 | `rust/lib/color/src/css.rs:13-28` (+2 sites: `markup/src/html_class.rs:64-79`, `markup/src/transform.rs:21-32`) | `IN_USE_SOFTENS`/`IN_USE_MIXES` is a hand-maintained whitelist against arbitrary game-emitted soften/mix pcts, with nothing testing the converse subset relation. | Report itself calls it latent, not a live bug. 00-STATE: **Unit 09b obligation 4 DISCHARGED - F-15 stays LATENT, no live violation; do not re-run the sweep.** Real emitter is `rust/web/src/theme.rs` (whitelist unenforced there). |
| F-16 | Low | 01b | WP-09a/09b | `rust/lib/cmd/src/requester/gamer.rs:91-93` | `DataDocs`/`BasicStrategy`/`AdvancedStrategy` discard their `game`/`player` payload, so these three state-carrying variants never deserialise, `validate()` or `check_player`. | Report: not a live exploit (nothing indexed); a hole in the D-36 boundary rule. `game_client::fetch_game_data` (`game_client/src/lib.rs:341-393`) still sends the fields. |
| F-17 | High | 01b | WP-06 | `rust/lib/cmd/src/repl.rs:210` (+7 sites: `:252`, `:44`, `:145`, `:192`, `:55/:57/:169/:175/:203/:204`, `:107`, `:109-120`, `:264`) | WP-02's deferred "CLI REPL will panic" fix did not land: all 18 markup/IO/response paths in `repl.rs` still `unwrap`/`expect`/`panic!`, including panicking on a normal `Response::UserError`. | Paired with F-01: WP-02 + WP-06 together moved the failure mode from partial render to panic. 01b's sweep counted 37 non-test panic constructs under `rust/lib/cmd/src/`; the 14 in `test_support.rs` are explicitly NOT covered here (00-STATE routes that to Unit 10). |
| F-18 | Medium | 01c | WP-08/WP-08b | `rust/game/for-sale-2/src/lib.rs:495` (+9 sites: `:513`, `:531`; `sushizock-2:744/765/786`; `category-5-2:509/527`; `farkle-2:432/453`) | Four unscoped crates still carry the duplicated epilogue and all 14 unmigrated crates lack WP-08's `!was_finished` transition gate. | 00-STATE: remediation ownership is **Units 03/04** for `for-sale-2`, `sushizock-2`, `category-5-2`, `farkle-2` **and `battleship-2` (F-71 - the 01c list was one short)**. `red7-1`, `lost-cities-1/-2`, `modern-art-2` spec-excluded, do not re-raise. Interacts with F-05 (`for-sale-2` command gating known loose). |
| F-19 | Low | 01c | WP-08 | `rust/game/jaipur-2/src/lib.rs:699-715` vs `:665-677` (+4 crates: roll-through-the-ages-2, starship-catan-1, seven-wonders-1, alhambra-1) | Five crates write the placings metric twice, so `status()` and the finish log agree only by the same expression being typed out twice. | Not an execution defect - the spec ordered verbatim lifts. All five pairs verified currently identical. |
| F-20 | Low | 01c | WP-08 | `rust/game/starship-catan-1/src/lib.rs` (inside `finish_epilogue`) | `finish_epilogue` mixes `0..self.players` for scores with a hardcoded `0..2` for placings; adding a player count would under-report winners. | Magic `2` inherited, not introduced; spec forbade improving it. |
| F-21 | Low | 01c | WP-08b | `rust/game/acquire-1/src/lib.rs:247-262` | acquire-1 is the only migrated crate with no regression test, and the three-property reasoning that made the arm-coverage widening safe is recorded nowhere in the tree. | 01c verified the spec's safety claim holds - not a behaviour change. |
| F-22 | High | 02 | WP-10/WP-14 | `rust/game/alhambra-1/src/lib.rs:160-181` | `start_game` emits each player's exact opening money-card draw as a `Log::public`, so the whole hand is reconstructible despite `PubBoard.card_count` claiming cards are private. | Pre-existing, not a WP-14 regression (same log at `c52f1a53^`). 00-STATE pattern 3 cites F-22 as a leak that survived because nobody checked `Log::public`. 00-STATE F-81 ruling explicitly does NOT excuse it - direct leak, still valid. Redaction test only greps `PubState`. |
| F-23 | Medium | 02 | WP-10/WP-14 | `rust/game/alhambra-1/src/lib.rs:452-479` | `final_place_phase` publishes `best_value`, a direct aggregate over the winner's private hand, for each of the 4 currencies. | Derived-value leak class; same crate/class as F-22 (both listed under alhambra-1 "defeated by logs" in the report's redaction table). |
| F-24 | Medium | 02 | WP-14 | `rust/game/alhambra-1/src/lib.rs:813-963` (+3 sites: `:407-417`, `:446`, `:625`) | `alhambra-1` has no `Gamer::validate` override, so a deserialized state with `boards.len() < all_players` or `tiles.len() != 4` panics on unchecked indexing instead of returning `GameError`. | F-06 confirmation (Unit 01 carry-forward). Grouped with F-31 and F-33 as coverage gap 3; the F-06 crate list under-counts this unit and must be re-derived mechanically. |
| F-25 | Low | 02 | WP-14 | `rust/game/alhambra-1/src/lib.rs:216-229` | `inject_scoring_cards` counts positions from the front of `card_pile` while draws `pop()` from the back, so scoring fires at ~20% / ~70% instead of thirds. | Pre-existing, not a WP-14 regression; a coverage gap in the programme. Interacts with F-26 (mis-positioning makes F-26 reachable more often). |
| F-26 | Low | 02 | WP-14 | `rust/game/alhambra-1/src/lib.rs:377-383` (+1 site: `:330-368`) | Final scoring uses whatever `self.round` happens to be, so a game ending on tile-bag exhaustion scores with the round-1 or round-2 reward slice. | Combined with F-25 this is reachable more often than it should be. |
| F-27 | unstated | 02 | - | `rust/game/alhambra-1/src/lib.rs` (`stats: vec![]`) | Withdrawn - the alhambra-1 `stats: vec![]` observation was merged into F-35; number retired to keep the sequence stable. | WITHDRAWN, no open defect. Report's own tally says "13 open, F-22..F-35 with F-27 withdrawn". |
| F-28 | Medium | 02 | WP-25 | `rust/game/modern-art-2/src/lib.rs:442-450` (+3 sites: `:341-347`, `:361-365`, `:83-86`) | Every `player_money` mutation is a `Log::public` and `INITIAL_MONEY` is constant, so exact balances are always derivable despite the "money is secret" doc, `points()` zeroing and redaction test. | Sealed-bid secrecy itself is sound (`:481-483`, `:525-531`) - the leak is the money trail. 00-STATE pattern 3 cites F-28 alongside F-22; 00-STATE F-81 ruling does NOT excuse it (direct `Log::public` leak). Remediation requires an explicit intent decision (accept public vs make logs private). |
| F-29 | Low | 02 | WP-10 | `rust/game/modern-art-2/src/lib.rs:53-75` | `PubState` carries no per-player hand count although hand size is open information, and the engine partially leaks it via "Skipping X as they have no cards" logs anyway. | modern-art-2 was never swept for the WP-10 3a canonical shape (coverage gap 1). |
| F-30 | Low | 02 | WP-25 | `rust/game/modern-art-2/src/lib.rs:304-309` (+1 site: `:430-455`) | Neither `end_round` nor `settle_auction` clears `self.bids`, so a persisted `Game` carries concluded-auction bids (including Sealed) and `validate()` does not assert the implication. | Currently harmless (reset in `add_card_to_auction` `:422`, readers gated on `is_auction()`). Noted as the incomplete half of WP-25's d F41 fix. |
| F-31 | Medium | 02 | WP-13 (deferred to WP-09/09b) | `rust/game/starship-catan-1/src/lib.rs` (no `fn validate` in crate) | `starship-catan-1` never overrides `Gamer::validate`, so a deserialized state with `current_player > 1` panics on the direct indexing in `player_state`/`pub_state`. | F-06 confirmation; routing leak - WP-13 Non-Goals deferred it to WP-09 "blocked on D-36" and WP-09b (`c078c3ee`) covered 16 files not including this crate. Grouped with F-24/F-33 as coverage gap 3. |
| F-32 | Low | 02 | WP-10 | `rust/game/starship-catan-1/src/render.rs:22-23` (+1 site: `lib.rs:1851`) | Both full `PlayerBoard`s are cloned into `PubState` against WP-10 3a rules 1 and 5, and the redaction test blocklists only four field names so a future private field goes public silently. | No leak today (the board is genuinely open information) - remediation is a comment plus converting the test to an allowlist; no code change required. |
| F-33 | Medium | 02 | WP-15 (deferred to WP-09/09b) | `rust/game/seven-wonders-1/src/lib.rs` (no `fn validate`; +2 sites: `:894-925`, `:211`) | `seven-wonders-1` has no `validate()` and `status()` raw-indexes `hands`/`actions`/`coins` per player, so a short-vector saved state panics on every page render, not just on a command. | F-06 confirmation; the worst of the three (F-24/F-31/F-33) per the report, and the crate the F-06 list omitted entirely. Routing leak: WP-15 Non-Goals deferred to WP-09, WP-09b never reached it. Coverage gap 5 flags an unverified related panic at `lib.rs:340` for whoever picks this up. |
| F-34 | Low | 02 | WP-15 | `rust/game/seven-wonders-1/src/lib.rs:722-725` | WP-15's new public "has no cards they can take from the discard pile" log asserts a property of a pile whose contents `PubState` deliberately hides (`discard_count` only). | Low because b F8 (discard-pile visibility) is a **parked** rules item under D-27/D-28 whose likely resolution makes the pile public; flagged so the parked decision is made deliberately. |
| F-35 | Low | 02 | - | `rust/game/alhambra-1/src/lib.rs:918-921` (+1 site: `seven-wonders-1/src/lib.rs:900-903`) | `Status::Finished` returns a zero-length `stats` beside a `placings` of length `players`, so consumers that zip or index `stats` by player drop everyone or panic. | **00-STATE owner ruling: parked in WP-20 (`c F12`), 24 sites across 21 crates - record occurrences, do not demand fixes or re-raise per crate.** The unit report gives no park status and proposes a fix; 00-STATE wins. Alhambra half is pre-existing, not a WP-14 regression. Distinct from F-19. Carry-forward: likely present in more crates. |
| F-36 | High | 03a | WP-09b (sweep missed crate); crate touched by WP-18 | `rust/game/texas-holdem-2/src/lib.rs:663-814` (+3 sites: `:98-102`, `:709`, `:143-155`) | `texas-holdem-2` has no `validate()` override, so seven parallel per-player vectors sized only in `new_hand()` are raw-indexed and a short deserialized state panics inside `status()`/`player_state`. | F-06 confirmation; the report calls it the crate where the missing override matters most, ahead of F-24/F-31/F-33. Coverage gap 3: no crate in the sub-unit tests `validate()` because none has one. |
| F-37 | Medium | 03a | WP-09b (sweep missed crate); crate touched by WP-17 | `rust/game/splendor-2/src/lib.rs:529-712` (+2 sites: `:374-396`, `:456-460`) | `splendor-2` has no `validate()` override, so a deserialized `Game` with `board: []` or a short `player_boards` panics inside `buy`/`reserve`/`pub_state` instead of being rejected. | F-06 confirmation - this is the crate F-06's own finding text named, confirmed still open after the whole remediation programme. Coverage gap 5 adds `visit_parser`'s `Int{min:1,max:nobles.len()}` inversion as another thing `validate` should assert. |
| F-38 | Low | 03a | - | `rust/game/splendor-2/src/lib.rs:237-239` | `visit_phase`'s `unwrap_or_else` swallows a `GameError` into a public `Log`, silently skipping the noble award and broadcasting raw internal error text to every player. | Dead defensive code today (the auto-visit path cannot fail); same swallowed-error shape as F-07. |
| F-39 | Low | 03a | WP-10 | `rust/game/splendor-2/src/lib.rs:79-97` | `PubState` omits per-level deck sizes, which are public information in Splendor, so a `PubState`-only client cannot show remaining cards or explain a shrinking level. | Over-redaction, not a leak. Same class as F-29 (modern-art-2 omitting public hand sizes). |
| F-40 | Medium | 03a | WP-19 (`c F10`) | `rust/game/acquire-1/src/lib.rs:1119` (+2 sites: `:1137`, `:577`) | WP-19's "tolerate missing share keys" hardening defaults an absent bank-share entry to `STARTING_SHARES` (25), minting phantom shares and breaking the 25-per-corp conservation invariant. | Regression introduced by the WP itself; the WP's own test `end_tolerates_missing_share_keys` (`:1490-1497`) locks the broken behaviour in. Report pairs it with F-47 (the two thinnest-tested tasks). Violates the spec's own "never replace a silent wrong answer with a panic" in the reverse direction. |
| F-41 | Low | 03a | WP-20 (`c F12`, parked) | `rust/game/acquire-1/src/lib.rs:204-207` | `acquire-1` also constructs `Status::Finished { placings, stats: vec![] }`, a zero-length stats vector for an N-player game. | Filed as an F-35 crate-list correction, not a new demand to fix. **00-STATE owner ruling: F-35 is parked in WP-20 - record occurrences only.** Report also corrects `00-sweeps.md` sweep 2: texas-holdem-2/splendor-2 emit per-player empty maps, so they are neither F-35 instances nor populated. |
| F-42 | Low | 03a | - | `rust/game/acquire-1/src/lib.rs:452-519` | `handle_play_command` captures the played tile's index at `:452` but only `swap_remove(pos)` at `:518`, after the whole merger cascade, so any future path reaching `draw_replacement_tiles` deletes the wrong tile or panics. | Latent, not live - no current callee mutates `players[player].tiles`. `swap_remove` also reorders the hand for no reason. |
| F-43 | Low | 03a | - | `rust/game/acquire-1/src/lib.rs:1167-1173` | `handle_end_command` checks `phase.main_turn_player() != player` instead of `assert_player_turn`, so during `Phase::SellOrTrade` the turn player may act while `status().whose_turn` names someone else. | Possibly deliberate; the report's demand is that the intent be stated and `command_parser` be the single source of truth. |
| F-44 | Medium | 03a | WP-18 (T3-B3 row `c F1`) | `rust/game/texas-holdem-2/src/command.rs:47-69` (+1 site: `lib.rs:312-318`) | `raise_parser` builds its `Int` with `min = min_raise()` while `can_raise` still gates on `largest_raise`, so a short stack is offered a `raise` whose bounds invert (min 10, max 5) and which rejects every input. | **Regression introduced by WP-18 itself**, not pre-existing; the checklist row was satisfied literally while the invariant it rested on was broken. Test `raise_parser_min_bound_uses_min_raise` (`lib.rs:1002-1020`) never exercises the short-stack case (coverage gap 4). Go parity explicitly rejected as a defence - a malformed `CommandSpec` is a brdgme-layer contract. |
| F-45 | Low | 03a | WP-18 | `rust/game/texas-holdem-2/src/poker.rs:348-377` (+1 site: `lib.rs:482-491`) | `winning_hand_result` builds its winner vector by iterating a `HashMap`, so split-pot showdown log lines come out in per-process random order and a `(seed, commands)` replay is not byte-reproducible. | Game state unaffected (`pot_per_player` is order-independent); the harm is log-diffing and future flaky log assertions. |
| F-46 | Low | 03a | WP-19 Task 7 (`c F20`) | `rust/game/acquire-1/src/lib.rs:226-228` (+1 site: `:1224-1236`) | `pub_state()` still deep-clones the whole `Game` (board, hands, ~100-`Loc` bag, RNG) because `From<Game> for PubState` takes `Game` by value; `player_state` pays it again per player. | Only the `can_end` deduplication half of the task landed; the allocation the finding was actually about is untouched. Remediation should land with coverage gap 2 (`acquire-1` has no `pub_state` redaction test at all). |
| F-47 | Low | 03a | WP-19 (package gate) | `rust/game/acquire-1` (21 tests at HEAD: 14 `src/lib.rs` + 6 `src/board.rs` + 1 `tests/contract.rs`) | WP-19's hard final gate of 23 passing tests was not met - 9 new tests landed against the specified 11, leaving the stated count 2 short. | Paperwork on its own; the report explicitly compounds it with F-40 and F-46 - the two unfixed/wrongly-fixed tasks are the two with the thinnest coverage. Test counts were derived by reading, not running (session bans running tests). |
| F-48 | Medium | 03b | WP-21 (cross-package item 2) | `rust/game/cathedral-2/src/lib.rs:402-432` (+4 sites: `:146`, `:106`, `piece.rs:110`, `command.rs:80`) | The hoist that closed WP-21's catalogue-rebuild observation is defeated by the loop body's own `can_play_piece` -> `player_pieces` call, so ~5,600 of ~5,700 rebuilds remain (~2% removed) while the comment claims the waste is gone. | Hot path: `status()`, `command_spec()`, `play()` x4 and `next_player()` all reach it, so ~10^5 allocations per call on every web render. In-repo idiom for the real fix already exists (`lords-of-vegas-1/src/tile.rs:22` `LazyLock`). |
| F-49 | Medium | 03b | WP-21 non-goal routed to WP-09; WP-09b (missed) | `rust/game/cathedral-2/src/lib.rs:343` (+6 sites: `:150`, `:154`, `:157`, `:217`, `:394`, `:416`) | `check_captures` indexes `played_pieces[pt.player][(pt.typ - 1)]` with two unvalidated `i32`s off a board tile (`typ == 0` wraps to `usize::MAX`), and cathedral-2 has no `validate()`; `:150`'s `>` bound admits `piece == len` and panics even from clean state. | F-06 confirmation, command-reachable via `command()` -> `play()`. The off-by-one at `:150` is a deliberately preserved Go defect (comment at `:134-138`). Remediate together with F-51 and F-55 (same dropped WP-21 -> WP-09 routing). |
| F-50 | Medium | 03b | WP-22; routed to WP-09/09b (missed) | `rust/game/lords-of-vegas-1/src/lib.rs:298` (+2 sites: `:308`, `:353`) | `build()` raw-indexes `self.players[current_player]` with no bound relative to `players.len()` and no `validate()`, and `next_player()`'s `% self.players.len()` is a divide-by-zero on an empty players vector. | **CONFLICT - 00-STATE owner ruling wins: `lords-of-vegas-1` is a WIP crate; F-50 is to be marked "WIP crate, excluded" in the unified report and NOT routed to remediation.** The unit report files it as a live Medium with remediation. Report notes `player_state` (`:171`) is defensively written while the command path is not. |
| F-51 | Medium | 03b | WP-21 lead ruling routed to WP-09; WP-09b (missed) | `rust/game/sushizock-2/src/lib.rs:278-283` (+5 sites: `:326`, `:330`, `:376`, `:673-685`, `:700-704`) | `player_score` raw-indexes `player_blue_tiles`/`player_red_tiles`, and `status()`/`pub_state()` both map it over `0..players`, so a short-vector state is permanently unrenderable, not merely uncommandable - no command needed. | F-06 confirmation and, per the report, the worst of the four in this unit. WP-21's own lead ruled sushizock-2 "must be ADDED to WP-09's crate list" and it never was - see F-55. |
| F-52 | Low | 03b | WP-08 (crate never migrated) | `rust/game/sushizock-2/src/lib.rs:737-745` (+4 sites: `:761-766`, `:781-790`, `:372-378`, `:380-404`) | All three `command` arms carry a copy-pasted epilogue guarded by `is_finished()` post-condition rather than a `!was_finished` transition; separately `next_player()`'s `log_game_end` emits the same scores a second time in a different format. | F-18 confirmation for sushizock-2. Not currently exploitable (`command_parser` returns `None` once finished). **00-STATE carry-forward: F-18 remediation ownership groups sushizock-2 with `for-sale-2`, `category-5-2`, `farkle-2` and `battleship-2` (F-71).** The double end-of-game log is a separate defect from F-18. |
| F-53 | Low | 03b | WP-21 | `rust/game/sushizock-2/RULES.md:7-8` (+2 sites: `DATA_DOCS.md:7-8`, `src/lib.rs:89-96`) | RULES.md says the blue and red tile rows are shuffled face down while DATA_DOCS.md and `PubState` declare the full ordered row public, so the two rules documents contradict each other. | Explicitly **not** treated as a leak - DATA_DOCS.md is the redaction contract per D-33 and the mechanic is forced. Filed because the sweep's "no hidden information" classification rests on whichever doc happens to match the code; an auditor reading RULES.md would file a High. |
| F-54 | Medium | 03b | WP-23; WP-09b (missed) | `rust/game/jaipur-2/src/lib.rs:161` (+14 sites: `:340`, `:380`, `:426`, `:459`, `:462`, `:504`, `:515`, `:566`, `:737`, `render.rs:209`, `:216`, `:231`, `:235`, `:239`) | `current_player` is a bare deserialized `usize` with no bound and `can_take`/`can_sell`/`command_parser` only compare against it, so `command(2, ..)` reaches `self.camels[2] += ..` and `player_state(2)` panics on a 2-element fixed array. | F-06 confirmation; same bug as the parallel-vector cases with a fixed-array surface. Two-line `validate()` because `NUM_PLAYERS` is a compile-time 2. |
| F-55 | Medium | 03b | WP-21 (non-goals) -> WP-09/09b (never delivered) | `rust/game/cathedral-2/src/render.rs:378` (+5 sites: `lib.rs:154`, `:217`, `:394`, `:416`, plus sushizock-2's whole crate) | WP-21 closed the `played_pieces` truncated-row indexing and sushizock-2's crate addition by routing both to WP-09, and WP-09b (`c078c3ee`, 15 crates) picked up neither, so the deferred indexing is still unguarded. | **00-STATE systemic pattern 1, "the routing leak" - one of three confirmed cases with F-57 and F-60.** 00-STATE says "two [of the three] are High"; this unit report rates F-55 Medium and F-57 Low, so neither is - flagged as a severity discrepancy, not resolved here. Render.rs guard at `:363-371` checks the outer length only. Process failure as much as a code gap. |
| F-56 | Low | 03b | WP-21 | `rust/game/cathedral-2/src/lib.rs:124-130` (+3 sites: `:243-258`, `:518-560`, `:562-564`) | `play()` can set `finished = true` while leaving `no_open_tiles == false`, and in that state `can_play` returns true, so `command_spec` keeps advertising `play` on a finished game and errors come back as piece-level messages, not `GameError::Finished`. | `command()` never calls the trait's `assert_not_finished` (`rust/lib/game/src/game.rs:96-104`). jaipur-2 and sushizock-2 both short-circuit correctly - cathedral-2 is the outlier. |
| F-57 | Low | 03b | WP-22 (non-goals, `d F5`) -> WP-09/09b (never delivered) | `rust/game/lords-of-vegas-1/src/board.rs:84` (+2 sites: `:103-118`, `:151-166`) | `Loc::parse_str`'s `chars.next().unwrap()` sits on the `Deserialize` entry point for every stored `Loc` (banned by `docs/CODING.md`), and `d F5`'s `neighbours()` half is unfixed - `lot == 0` underflows `self.lot - 1` at `board.rs:109`. | **CONFLICT - 00-STATE owner ruling wins: WIP crate, excluded; F-57 is not routed to remediation.** Also **00-STATE pattern 1 (routing leak) names F-57 alongside F-55 and F-60 and says two of the three are High** - the unit report rates this Low; discrepancy recorded, not resolved. The out-of-range-lot half of `d F5` did land (`board.rs:89-91`); only `neighbours()` is open. Crates cite the same CODING.md rule inconsistently (`sushizock-2/src/lib.rs:151-155`). |
| F-58 | informational | 03b | WP-20 (`c F12`, parked) | `rust/game/cathedral-2/src/lib.rs:570` (+3 sites: `sushizock-2/src/lib.rs:677`, `jaipur-2/src/lib.rs:707`, `lords-of-vegas-1/src/lib.rs:201`) | Four further `Status::Finished { stats: vec![] }` occurrences in this unit, recorded for the F-35 tally only. | Severity given verbatim as "informational" - the report offers no Critical/High/Medium/Low rating. **00-STATE owner ruling: F-35 is parked in WP-20 - record occurrences, no fix demanded.** Note the same paragraph is the origin of the `Gamer::points()` carry-forward (all four crates implement `points()` with real values). |
| F-59 | Low | 03b | WP-10 3a | `rust/game/lords-of-vegas-1/src/lib.rs:157-166` (+1 site: `:41-54`) | lords-of-vegas-1 holds genuinely secret state (deck order and the `Card::GameEnd` position) yet has no test calling `pub_state()` at all, so the "`PubState` must never carry `deck`" invariant is held only by nobody having added the field. | Regression-risk, not a live leak - the redaction is currently correct. **00-STATE pattern 4: WP-10 3a was declared "for every game crate" and applied to 3 of 28; 13 crates have no redaction test.** The WIP ruling for lords-of-vegas-1 is not stated to cover F-59 (00-STATE names only F-50/F-57 and the no-`finished` observation) - ambiguity recorded. The report places this finding inside its "Verified good" section, after F-58. |
| F-60 | High | 04a | WP-28 (non-goals) -> WP-09a/09b (never delivered) | `rust/game/lost-cities-1/src/lib.rs:545-559` (+6 sites: `:211`, `:212`, `:261`, `:262`, `:308`, `:373`) | `player_state()` indexes `self.hands[player]` raw with an in-code comment routing the bounds fix to WP-09, and lost-cities-1 has no `validate()`, so a persisted short `hands` panics the render path for every viewer. | **00-STATE systemic pattern 1, "the routing leak" - one of three confirmed cases with F-55 and F-57; 00-STATE says two of the three are High and F-60 is one of them.** WP-09a's `check_player` (`rust/lib/cmd/src/requester/gamer.rs:24-36`) is vacuous here because `player_count()` returns the `PLAYERS` constant, not `hands.len()`. Also an F-06 instance. Mechanism is F-62. |
| F-61 | High | 04a | WP-09a/09b (sushi-go-2 never swept) | `rust/game/sushi-go-2/src/lib.rs:798-818` (+6 sites: `:145`, `:224`, `:245`, `:257`, `:265`, `:791`) | `player_state()` got three length guards while `self.playing[DUMMY]` inside the same function and five further render-path indexes (`is_finished`, `can_dummy`, `pudding_cards`, `placings`, `pub_state`) stayed raw, with no `validate()` override. | **00-STATE systemic pattern 2, "inconsistent hardening within a single file" - 00-STATE names F-61 as the clearest case.** F-06 instance (one of the missing-13). Premise for F-65's `unreachable!()` regression. |
| F-62 | Medium | 04a | WP-09b | `rust/game/lost-cities-1/src/lib.rs` (no `fn validate`) vs `rust/game/lost-cities-2/src/lib.rs:550-577` | lost-cities-2 is a near-verbatim generalisation of lost-cities-1 yet only -2 received a `validate()` plus `validate_works`, with no crate-specific reason for the split. | Stated to be the mechanism behind F-60 - remediation is the same port (`validate()` + `validate_works` into -1). Pattern-2 sibling miss across two files rather than within one. |
| F-63 | Low | 04a | - | `rust/game/lost-cities-2/src/lib.rs:730-752` | `expedition_cost`, `hand_size` and `expedition_bonus_size` `unreachable!()` for any `players` outside 2..=3, on paths reachable from `draw_hand_full` and `end_round`. | Not a live panic - `validate()` (`:550`) gates `players`; defence-in-depth only, hence Low. `Game::default()` has `players == 0` and is used via `..Game::default()` in `start`. No WP attributed by the report. |
| F-64 | Low | 04a | - | `rust/game/lost-cities-2/src/lib.rs:32-35, 754-782`; test at `:877-929` | `EXP_COST_3P`/`EXP_BONUS_SIZE_3P` - the only new scoring rules in -2 - are never exercised because `score_works` calls `score(2, ..)` for all six assertions. | Also flags `:780` reusing `exp_cost` as the completion-bonus value, which silently makes the 3p bonus 15; whether that is the intended rule is untested and undocumented. No WP attributed. |
| F-65 | Medium | 04a | WP-24 (`d F28`); closed later by `ae04843c` (BACKLOG #59) | `rust/game/sushi-go-2/src/lib.rs:140-147` | WP-24 converted `.unwrap_or(9)` into a `match` with `_ => 9`, reproducing the silent fallback the row existed to remove, and the row's `Test? y` was never satisfied by either commit. | **00-STATE systemic pattern 5, "the `_ => <default>` substitution" - 00-STATE names F-65 as the pattern's instance** (00-STATE also notes F-136 is now a High-severity web-half instance). Rediscovered from outside the programme two days later; `ae04843c`'s justification is factually wrong and its `unreachable!()` is a small command-reachable regression premised on F-61 being fixed. |
| F-66 | Medium | 04b | WP-32 (crate); WP-09a/09b (`validate`) | `rust/game/category-5-2/src/lib.rs:315` | `choose()`'s `self.plays[player].expect(..)` depends on "`resolving` implies `plays[choose_player]` is `Some`", an invariant `validate` (`:378-436`) never checks, so a state with `resolving: true` and a null play passes `validate` and panics. | **00-STATE systemic pattern 2b - named with F-67/F-68/F-76: `validate` overrides cover the parallel-vector sweep but miss the one cross-field invariant each crate's remaining panic depends on. No crate reviewed has a `validate` test.** Distinct failure mode from F-06 (the override exists and is still insufficient). |
| F-67 | Medium | 04b | WP-32 (`f F31`); WP-09 | `rust/game/category-5-2/src/lib.rs:228-241` (+1 site: `draw_cards`, `:271-286`) | The equal-hand-size invariant is asserted only in a comment while `self.hands[p][0]` is raw-indexed behind a `hands[0].len() == 1` guard, and `draw_cards` can legitimately produce unequal hands via its short-return branch. | **00-STATE pattern 2b (F-66/F-67/F-68/F-76).** Downstream of F-73's short-draw branch; F-74 is the same code read as a remediation-choice defect. Report notes `category-5-2` does **not** have the F-01 shape. |
| F-68 | Medium | 04b | WP-31 (crate); WP-09 | `rust/game/zombie-dice-2/src/lib.rs:239-255` | `take_dice`'s refill branch is best-effort and then `drain(..n)` unconditionally, so a persisted state with empty `cup`/`kept`/`current_roll` passes `validate` (`:453-475`, no dice-conservation check) and panics on the next `roll`. | **00-STATE pattern 2b (F-66/F-67/F-68/F-76).** The `take_dice` comment documents Go provenance rather than the precondition. |
| F-69 | Medium | 04b | WP-09a | `rust/game/for-sale-2/src/lib.rs:301-327` | `next_bidder`'s `remaining == 0` case falls through into an unbounded `loop` that can never break, so a state with every `finished_bidding` entry `true` passes `validate` and `bid()` hangs. | A hang is worse than a panic in this architecture (pins a worker at 100% CPU, no `SystemError` surface). WP-09a hardened `player_state` (`:462-465`) in this same crate and left the command-path loop untouched - pattern-2 shaped. Untested per the unit's coverage gaps. |
| F-70 | Low | 04b | WP-09a | `rust/game/for-sale-2/src/lib.rs:400-424` (+1 site: `rust/game/battleship-2/src/lib.rs:425-447`) | Neither crate's `validate` bounds the player count, unlike `red7-1/src/lib.rs:563-565`, so `players: 1`/`players: 7` is accepted by for-sale-2 and any `players != 2` state mis-targets battleship-2's `% NUM_PLAYERS` indexing. | for-sale-2's `pub_state` divide-by-`players` (`:445-446`) is safe only by accident (`bidding_player >= players` incidentally rejects 0). Same hardening pass produced the check in one crate and omitted it in two - pattern-2 shaped. |
| F-71 | Low | 04b | WP-08 | `rust/game/battleship-2/src/lib.rs:531-537` | `battleship-2` carries the same ungated copy-pasted `if self.is_finished() { .. placings_log(..) }` epilogue as for-sale-2 and category-5-2, so F-18's unmigrated-crate list is one crate short. | **00-STATE carry-forward "Units 03/04": F-18 remediation ownership is for-sale-2, sushizock-2, category-5-2, farkle-2 **and battleship-2** (F-71 - the list was one short).** Not a live bug: `command_parser` returns `None` once finished, so no double-fire is reachable; maintainability only. Per-crate: zombie-dice-2 migrated; red7-1 unmigrated but pre-ruled out of scope. |
| F-72 | Medium | 04b | WP-32 (`f F24`) | `rust/game/category-5-2/src/lib.rs:21` (+1 site: `:548-550`) | `f F24` was closed with `MAX_PLAYERS` still 8 and `player_counts()` still the hardcoded `vec![2..8]`, so 9- and 10-player games cannot be created despite deck math being exact at 10. | Must be fixed as one item with F-72a (same row, same commit `807ab4e9`). Checklist recorded "No findings rejected in this batch", so nothing tracks it; `category-5-2` has **nothing parked**, so `f F24` has no park to fall back into. |
| F-72a | High | 04b | WP-32 (`f F24`) | `rust/game/category-5-2/RULES.md:3` | The WP-32 commit that was meant to raise `MAX_PLAYERS` instead edited user-facing `RULES.md` from "2-10" to "2-8" to match the code, erasing the discrepancy the finding cited. | **00-STATE systemic pattern 4b - explicitly named as `F-72a` there ("edited `RULES.md` down to match the code"), one of three (later four, with F-95) confirmed instances alongside F-83 and F-79.** Note the sub-letter: 00-STATE uses `F-72a`, matching the report. Must be fixed as one item with F-72. `RULES.md` is served through `Gamer::rules()` (`lib.rs:556-558`), so it is a published claim; no decision record exists. |
| F-73 | Medium | 04b | WP-32 (`f F25`) | `rust/game/category-5-2/src/lib.rs:271-286` (+2 sites: `:141-143`, `:144-148`) | `draw_cards`'s new guard stops the stack overflow but returns a short `Vec` instead of erroring, so `start_round` produces unequal hand sizes and can produce an empty board row - the invariant `validate` and the `.expect("row is never empty")` rely on. | Row satisfied literally, purpose ("an over-large `n` errors") unmet. Feeds F-67 and F-74; `draw_cards` does not even return `Result`. Untested - `test_game_draw_cards` (`:584-594`) never hits the shortfall branch. |
| F-74 | Low | 04b | WP-32 (`f F31`) | `rust/game/category-5-2/src/lib.rs:228` | `f F31` offered a comment **or** an `all`-style check and the comment was taken, but because `draw_cards` can return short (F-73) the comment is false on a reachable path while justifying the raw index and `expect` below it. | Recorded separately from F-67 because the finding is about the *choice of remediation*, not the code defect. Taking the `all`-check option would have closed F-67 and F-73's downstream half. |
| F-75 | Low | 04b | WP-31 (`f F5`) | `rust/game/zombie-dice-2/src/lib.rs:257-265` vs `:294-299` | The `f F5` loop conversion inlined a verbatim copy of `start_turn`'s six-line turn reset instead of extracting it, so any future turn-reset field must be added in two places. | Explicitly a nit on an otherwise-correct fix - the recursion and stack growth are genuinely gone. |
| F-76 | Medium | 04b | WP-29 (`e F34`) | `rust/game/red7-1/src/lib.rs:240-266` (+2 sites: `:339`, `validate` at `:562-582`) | `e F34` was closed with a PRECONDITION comment; `validate` never checks that some player is un-eliminated, so `eliminated: [true, true], finished: false` passes `validate` and `command_parser`, and `discard` -> `leader_with_suit` panics on an empty `player_map`. | **00-STATE systemic pattern 2b (F-66/F-67/F-68/F-76).** "Documented the invariant instead of enforcing it" - the crate gained a `validate` override in the same programme that could have enforced it in one line. `leader()`/`leader_with_suit()` are `pub`, so bots and `tools/*` can trigger it directly. |
| F-77 | Nit | 04b | WP-29; WP-09 | `rust/game/red7-1/src/lib.rs:20-27` | `end_points`'s doc comment still claims `num_players` deserializes unvalidated, which WP-09's `validate` (`:563-565`) made false, inviting a reader to conclude no validation exists. | Severity given verbatim as "Nit" - the report offers no Critical/High/Medium/Low rating. The saturating arithmetic itself is correct and worth keeping as defence in depth. |
| F-78 | Low | 04c | WP-33 (`f F36`) | `rust/game/greed-2/src/lib.rs:365` (+3 sites: `rust/game/farkle-2/src/lib.rs:260`, `:310`, `:313`) | The `saturating_add` overflow fix landed in `greed-2` only, skipping the raw `scores[player] + turn_score` in its own log text, and `farkle-2` - a near line-for-line twin - got neither half. | ID collision: unit report `04b-games-red7-zombiedice-forsale.md:310` also has an `F-78`, a **WITHDRAWN** `f F27` negative-footer theory (moved to Verified good). This 04c row is the live F-78. Neither crate's `validate` bounds `scores[..]`/`turn_score`, so this is the D-36 deserialized-state path. |
| F-79 | Low | 04c | WP-33 (`f F38`) | `rust/game/farkle-2/src/render.rs:26-46` (+1 site: test at `:100-128`) | The `scoring_table` fix still hardcodes all eight combinations and names, derives only point values with a silent `unwrap_or(0)`, and the new test pins the table to the old hardcoded literals. | **00-STATE systemic pattern 4b - named there as `F-79` ("new test re-hardcodes the legacy values"), alongside F-72a and F-83 (later F-95 as fourth).** One hardcoded list became three. |
| F-80 | Low | 04c | WP-33 (`f F57`; interacts with `f F56`) | `rust/game/liars-dice-2/src/command.rs:44-47` (+1 site: test at `:314-337`) | The bid quantity cap was made enforcing and asserted by a new test, but `Game::bid` accepts any strictly-increasing quantity, so the parser now rejects bids the rules allow. | Violates the row's own criterion "never rejects a legal bid". Secondary: `f F56`'s stated justification is falsified by `f F57` landing in the same commit; the two rows were not reconciled. Not a wedge (the player can still `call`). |
| F-81 | Low | 04c | WP-33 | `rust/game/no-thanks-2/src/lib.rs:117-121` (+2 sites: `:130-150`, `:282-286`) | `PubState::chips` is redacted during play and a test asserts it, but the per-player public pass/take logs plus the public `STARTING_CHIPS` make every player's chip count exactly reconstructible. | **Not a finding - owner ruling 2026-07-30 (00-STATE): reconstructing hidden information from the public log is ACCEPTABLE; record as intended behaviour.** Conflict: the unit report raises it as a Low finding and flags it for an owner ruling; 00-STATE's ruling supersedes and is general, not `no-thanks-2`-specific. Report also cites it as systemic pattern 3. |
| F-82 | Low | 04c | WP-33 (`f F45` neighbourhood) | `rust/game/tic-tac-toe-2/src/lib.rs:190-204` (+1 site: `:258-270`) | `validate` bounds `players` and `start_player` but never `current_player`, which `status()` publishes verbatim, so a deserialized `current_player: 9` reaches the web layer's turn handling. | **00-STATE systemic pattern 2b** (per the report's own text); note 00-STATE's own 2b instance list is F-66/F-67/F-68/F-76 and does not name F-82. |
| F-83 | Medium | 04c | WP-83 (`a F1`) | `rust/game/roll-through-the-ages-2/src/lib.rs:741-756` (+1 site: test at `:3266-3277`) | The `phase == phase_before` guard cannot distinguish "no advance" from "advanced full circle back to `Phase::Roll`", so an all-skull reroll silently decrements the *next* player's `remaining_rolls` from 2 to 1. | **00-STATE systemic pattern 4b - named there as `F-83` ("new test asserts the unchanged value where the spec prescribed the changed one"): the spec required `current_player == 1` and `remaining_rolls == 2`, the landed test asserts `MICK`(seat 0).** Attached note: **`roll-through-the-ages-2` has never had a crate-level review** (3,290 lines, no `validate` override, no redaction test); 00-STATE recommends a dedicated pass in the remediation plan. |
| F-84 | Low | 04c | WP-83 (`b F7`) | `rust/game/seven-wonders-1/src/lib.rs:1748-1774` (+2 sites: `:22-23`, `:122-143`) | The new test loops `3..=4` because the spec wrongly stated `MAX_PLAYERS = 4`; actual max is 7, and 7 is the boundary where `boards[..players]` consumes every derived group and would panic on any card-data drift. | The `b F7` fix itself is verified good (same report's Verified good section); this is a test-coverage and unchecked-slice gap only. |
| F-85 | High | 05a | WP-35 | `rust/web/src/auth/server.rs:590-612` (+1 site) | `logout_everywhere` returns `Ok(true)` without deleting any auth-token rows when `get_user_from_session` collapses a session-store error to `None`. | Same root cause as F-86; `logout` (`:566-588`) shares the shape. Systemic pattern 2. Unverifiable by existing tests (harness gap). |
| F-86 | Medium | 05a | WP-34 | `rust/web/src/auth/session.rs:68-74` | `get_user_from_session` swallows session-store errors, so a transient blip silently de-authenticates the user; `get_current_user:555` also discards the `clear_user_session` error. | Read-path half of F-85; ws F5 only half closed by WP-34. |
| F-87 | Medium | 05a | WP-35 | `rust/web/src/auth/server.rs:459-501` | WP-35's F2 fix deletes the pending `user_emails` row of a legitimate in-progress `add_email_address` flow and forks a second account owning the address. | Spec's own case analysis incomplete; needs an owner decision, not a silent behaviour change. |
| F-88 | Low | 05a | WP-34 | `rust/web/src/auth/server.rs:904-924` (+1 site) | `confirm_email_address` passes unvalidated `email`/`token` to `validate_confirmation_code`; WP-34's F13 shape check covers only `confirm_login`. | Feeds F-89 (every unvalidated call burns an attempt). |
| F-89 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:394-423` (+1 site) | `validate_confirmation_code` increments `attempts` before any authorization, so any authenticated caller can burn a victim's live login code via `confirm_email_address`. | Direct consequence of WP-34's chosen fix; unconsidered in spec. Amplified by F-94 (no rate limiting). |
| F-90 | Medium | 05a | WP-35 / WP-36 | `rust/bot/src/crypto.rs:59-76` (+1 site) | `rust/bot/src/crypto.rs` is an unhardened duplicate of the web crypto module: hardcoded default key with no `MissingKey`, no `ALLOW_INSECURE_DEFAULT_KEY` gate and no zeroize; its tests pin the old behaviour. | 00-STATE: divergent duplicate, fixes landed only in the web copy. Remediate as ONE item with F-108 (`rust/bot/src/nats.rs` vs `rust/web/src/nats.rs`); Unit 10 owns it. Duplicated-module sweep already done - do not re-run. |
| F-91 | Low | 05a | WP-36 | `rust/web/src/crypto.rs:20-43` (+1 site) | The AAD decline is recorded only in commit `13a1e693`'s message - no `D-NN` entry, no code comment, no spec - and its stated rationale ("shared format with bot") rests on the very duplication F-90 says is unfixed. | Report gives no sub-letters. Coupled to F-90; D-39 ruling applies. |
| F-92 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:1001-1996` | Three WP-34-mandated regression tests (F3 session-id rotation, F10 global-cap `Err`, F15 `logout`) were never written, leaving those fixes unverified. | Structural cause is the missing request-parts test harness (see coverage gaps); spec's mandatory-tests criterion unmet. |
| F-93 | Medium | 05a | WP-35 | `rust/web/src/auth/server.rs:853-900` | `add_email_address` returns three distinguishable errors (registered-address oracle) and commits the unverified row before the send, parking rows when the 50/day cap refuses the mail. | D-14 (ii) accepted the asymmetry for `login` only; `add_email_address` was never looked at. |
| F-94 | Medium | 05a | WP-34 | `rust/web/src/auth/server.rs:31-48` (+1 site) | No rate limiting exists anywhere in `rust/web`, yet two doc comments assert a per-IP limit as the design justification for the global advisory-lock cap and for not throttling confirm. | 00-STATE: confirmed - no rate-limiting middleware anywhere in `rust/web`. Enables the global-cap lockout lever and F-89. |
| F-95 | Low | 05a | WP-34 | `rust/web/src/auth/server.rs:1621-1652` | The WP-35 F1 concurrency test asserts `attempts >= CAP` - a lower bound where the spec prescribed an upper bound - because the prescribed bound is unachievable under the design the same spec mandated. | 00-STATE: FOURTH confirmed instance of systemic pattern 4b (tests/docs adjusted to agree with code). Escalation: the acceptance criterion was quietly renegotiated by the implementation. |
| F-96 | High (report) / DOWNGRADED (00-STATE) | 05a | WP-35 | `rust/web/src/main.rs:40-45` (+2 sites) | No manifest anywhere provisions `TURNSTILE_SECRET_KEY`, so the startup `panic!` added by WP-35 crash-loops the next prod web rollout. | **CONFLICT - 00-STATE wins:** resolved out of band, NOT a code defect. Panic is gated by `ALLOW_INSECURE_DEFAULT_KEY`, already set in `k8s/dev/web-patch.yaml:18-19` and `scripts/rust-test.sh:64`; Turnstile fails closed on every error path (`auth/server.rs:256-277`); sole fail-open (`secret.is_empty() -> true`) is what the panic prevents. "Dev default plus log warning" premise FALSE for `rust/web` - house pattern is panic-unless-opt-in (`crypto.rs:56-75`) and `docs/CODING.md:701` forbids the dev-default pattern. Remains a pre-rollout DEPLOYMENT blocker (no prod manifest sets the var). Also 00-STATE pattern 4d. |
| (no F-number assigned) | unstated | 05a | WP-35 | `rust/web/src/auth/server.rs:280-283` (+2 sites) | `TURNSTILE_SITE_KEY` has no startup check and silently defaults to empty, rendering no widget and rejecting every login - setting only the secret key is a total login outage. | New finding from `F-96-turnstile-key.md`; fold into the corpus. F-96 deployment-checklist family; both vars must land together. |
| (no F-number assigned) | unstated | 05a | - | `rust/bot/src/crypto.rs:66-76` | `rust/bot/src/crypto.rs` falls back to the hardcoded dev key with only a `tracing::warn!` and no gate in any environment, including prod. | New finding from `F-96-turnstile-key.md`; a real `docs/CODING.md:701` violation and another instance of the bot/web crypto divergence. Route to Unit 10 with F-90. |
| F-97 | Medium | 05b | WP-37 | `rust/web/src/admin.rs:254-262` (+2 sites) | `validate_provider_url` only checks the `http(s)://` prefix, so `test_provider`/`test_bot_provider` are an admin-triggered read-SSRF echoing in-pod responses, including `/metrics`. | ws F23 hardened the response handling but nothing constrains the upstream; defeats stated `/metrics` containment (`rust/web/src/main.rs:195-198`). |
| F-98 | Medium | 05b | WP-37 | `rust/web/src/admin.rs:515-533` (+1 site) | `api_key` is the only user string on the admin provider surface with no validation: empty keys are encrypted, stored, shown as `(set)` by `mask_api_key`, and sent as a bare Bearer; no size bound. | ws F25's validation sweep skipped it inside the very commit that added the helpers. Systemic pattern 2. |
| F-99 | Medium | 05b | WP-41 | `rust/web/src/db/mod.rs:161-256` | ws F35's 27-untested-function gap was closed by one smoke test naming 22 functions and asserting only degenerate/empty/negative cases, leaving behaviour unpinned. | Doc comment self-describes as "a *reminder*, not a mechanism". `is_user_admin` true path untested. Pattern 4b/4c instance. |
| F-100 | Low | 05b | WP-41 | `rust/web/src/db/mod.rs:119-159` | `session_token_validation` back-dates a token 40 days and asserts it is still valid, pinning the absence of server-side session expiry as intended behaviour. | Fifth confirmed pattern 4b/4c instance; belongs in process-fixes. Low only because `tower_sessions` store expiry also gates (05a F-85/F-86). |
| F-101 | Medium | 05b | WP-38 / WP-39 | `rust/web/src/game/mod.rs:329-355` (+1 site) | A transient `bot.command` failure leaves the message unacked, so redelivery waits the full 5-minute `ack_wait`, stalling the bot turn (and every deploy restart burns a delivery). | ws F58's `ack_wait` raise and wd F5's "leave unacked" never reconciled across work packages. Coupled to F-109 (no drain on SIGTERM). |
| F-102 | Medium | 05b | WP-39 | `rust/web/src/nats.rs:121-179` | `ensure_stream_and_consumers` only warns on stream/consumer config drift and then uses the server's values, so existing durables keep the pre-fix `ack_wait`/`max_deliver`. | Makes ws F58's fix inert on deployed environments; a server-side `max_deliver` < 3 strands messages before the code's Term (`game/mod.rs:330`), the exact wd F5 stranding. |
| F-103 | Low | 05b | WP-82 | `rust/web/src/db/mod.rs:94-101` | `create_pool` panics via `expect("DATABASE_URL must be set")` from a `Result`-returning fn and takes every `PgPool` default (max 10 conns, no timeouts) for the whole monolith. | Pre-existing; WP-82 moved it verbatim. Flagged so Unit 08 (query performance) has the pool sizing. |
| F-104 | High | 05b | WP-38 | `rust/web/src/db/bots.rs:57-71` (+5 sites) | `validate_bot_slots` matches `bot_name` case-insensitively and stores it verbatim, but every consumer resolves it case-sensitively, so `"EASY"` creates a permanently wedged game the WP-38 sweep refuses to rescue. | **00-STATE: ONE defect across FOUR units - remediate as a SINGLE item with F-138 (07), F-183 (09c), F-189 (10a).** Email `new` lowercases the bot name (`email/commands.rs:82-93`, written `:398-401`); bot lookup case-sensitive at `bot/src/config.rs:28` **and `:67`** (second site found by F-189, never cited in 05b); `bot/src/main.rs:186-194` returns `Ok(())`, acking and DISCARDING the turn. Precondition: `admin::create_bot` (`admin.rs:293-303`) permits arbitrary casing. Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 is the decoy all-lowercase-fixture test and must be re-fixtured in the same change. 00-STATE systemic pattern **4f** (a test blessing the lenient half of a cross-boundary inconsistency: `validate_bot_slots_accepts_case_mismatch`); 05b calls it pattern 4b - 00-STATE wins. |
| F-105 | Medium | 05b | WP-38 | `rust/web/src/game/mod.rs:200-255` (+2 sites) | `bot.turn` publishes carry no `Nats-Msg-Id` and the BOT stream has no `duplicate_window`, so three independent publishers amplify one turn into up to four LLM completions and four command attempts. | WP-38 spec's "re-publishing is safe" reasoning covers state safety but is silent on cost; conflict path deliberately spends three more completions up to `MAX_TURN_ATTEMPTS`. |
| F-106 | Low | 05b | WP-37 | `rust/web/src/admin.rs:787-813` | `read_capped_body` discards the `reqwest::Error` entirely, so reset/TLS/timeout are indistinguishable in the one admin tool whose purpose is diagnosing a misconfigured provider, and nothing is logged. | Every other failure in the file gets an `internal(context)` breadcrumb. |
| F-107 | Low | 05b | WP-38 | `rust/web/src/nats.rs:21-25` (+2 sites) | Three comments describe the term ceiling and stranded-message recovery as "(future)" WP-38/D-5 work after WP-38 shipped it (`game/mod.rs:330`), understating what depends on `MAX_DELIVER`. | Cosmetic; these are the comments a reader consults before touching ack semantics. |
| F-108 | Medium | 05b | WP-38 | `rust/bot/src/nats.rs` (+1 site: `rust/web/src/nats.rs`) | `BotTurnEvent`/`BotCommandEvent` and the subject/consumer constants are copy-pasted into the bot crate with no shared crate, no round-trip test and no `#[serde(default)]`, so a one-sided field addition is a runtime deserialization failure. | 00-STATE: not yet diverged - latent. `BotCommandEvent::attempt`'s echo invariant is documented only in the web copy (`rust/web/src/nats.rs:40-44`). **Remediate together with F-90 (Unit 05a); Unit 10 owns both.** Duplicated-module sweep DONE: exactly these two duplicates; `bot/config.rs` vs `web/config.rs` share only a filename. |
| F-109 | High | 05b | WP-36 | `rust/web/src/websocket.rs:78-80` (+2 sites) | `efad81f` deleted WP-36's ws F55 shutdown drain (`TaskTracker`, `drain_ws_tasks`, bounded 5s wait) and its regression test `rust/web/tests/websocket_hygiene.rs` together, leaving detached SSE spawns with nothing bounding the drain. | 00-STATE systemic pattern **4e** (NEW): a landed, tested fix silently reverted by a later commit in the same programme; checklist row and both commits still read as closed. Sign-off rule: assert each closed finding's citation or regression test still exists. Sharpened by F-147 (Unit 07b) - a citation must be *reachable*, not merely present - and by Unit 08 - a regression test must actually CALL the function under test. **00-STATE settles: `efad81f9` contains exactly ONE pattern-4e instance (F-109), enumerated not asserted; WP-84 spec §3g anticipated the deletion and required a proof test which does exist - so remediation is a bookkeeping fix on WP-36's row plus a decision on the never-implemented second half of ws F55 (bot consumer and email sweep tasks get no shutdown signal at `rust/web/src/main.rs:72-103`), NOT a revert of `efad81f9`.** |
| F-110 | Low | 05b | WP-37 | `rust/web/src/admin.rs:1688` (+2 sites) | Not a defect - WP-37's inline ws-finding citations are the only sign-off trail; the three apparently uncited findings (ws F24, F32, F33) were all in fact fixed, giving 14 of 14. | Recorded as the mechanism that made the review cheap; only Unit 05 work package to deliver in full. Contrast with F-109, which shows what happens where no such trail exists. |
| F-111 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:394-399` (+2 sites) | `concede_game_replace` calls `pick_replacement_bot` on the pool before `pool.begin()`, so every rejected concede commits an orphan `game_bots` row. | Salvaged from attempt 3 (unit done on 4th attempt). Escalation: `UNIQUE (game_id, name)` + no `ON CONFLICT` makes retry fail permanently as redacted internal error. Pattern 4b, fifth instance - spec's `game_bots` assertion dropped from the test (see F-114). |
| F-112 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:387-426` (+1 site) | `concede_game_replace` never updates the `games` row, so its `updated_at` claim never fails on replay: a duplicated concede swaps in a second bot and writes a second public log line. | Salvaged from attempt 3. Pattern 2 - `undo_game` got the equivalent in-transaction re-verify (Task 2.3), `concede_game_replace` did not. Remediation also closes F-113. |
| F-113 | Medium | 06 | WP-40 | `rust/web/src/game/server_fns.rs:945-947` (+1 site) | `concede_core` enforces "already left" only against the pool snapshot of `left_at`; neither the claim nor `concede_game_replace` re-checks it in-transaction. | Salvaged from attempt 3. Same for `count_active_humans` (`:819-824`). Violates the spec's own "a check against a snapshot is not a guard" rule. Feeds `left_at`-conflation coverage gap with F-116/F-117. |
| F-114 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:1816-2282` | Three of the seven new guard tests landed with spec names and error-type assertions but dropped the spec's "nothing was destroyed" state assertions. | Salvaged from attempt 3. Pattern 4b. Sites: `:1889`, `:1816`, `:2178` plus `concede_game_requires_two_players`. The dropped `game_bots` assertion is the one that fails today (F-111). |
| F-115 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:348-355` (+1 site) | Task 6's "unreachable" error is gated by `active_humans == 2` while `concede_game` counts all `game_players` rows, so a 3-player game with an eliminated player hits a redacted internal error and concede is permanently impossible. | Salvaged from attempt 3. Divergence path pinned by `elimination_sets_left_at_once` (`game_write.rs:1291`); F-116 reaches the same failure mode without anyone leaving. |
| F-116 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:584-598` (+2 sites) | `undo_game`'s `left_at` CASE has no arm for un-elimination, so an undone elimination permanently marks the player a leaver and `compute_ranked_placings` rates them last. | 00-STATE: clean instance of systemic pattern 2 - WP-40 added `AND NOT $9` to the byte-identical sibling CASE in `update_game_command_success` (`:743-744`) and left `undo_game`'s copy alone. Clause pre-dates `9ba3736b`; sweep confirms nothing anywhere sets `left_at` back to NULL. |
| F-117 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:356-381` (+2 sites) | `concede_game` rates a finished game without calling `write_ranked_placings`, leaving `ranked_placing` NULL on every conceded game while the other two finish paths populate it. | Pattern 4f: `placing.rs:106-127`'s `two_player_concede` test fixtures the conceder with `left_at: Some(..)`, a state `concede_game` never produces. Shares `left_at`-conflation root cause with F-113/F-116. |
| F-118 | Low | 06 | WP-40 | `rust/web/src/db/game_write.rs:584-598` (+2 sites) | `undo_game` does not restore `game_players.points`, so points stay at their post-undone-move value; `end_game` orders placings by that stale column. | Low: narrow path (`end_game` right after an undo with no intervening command). Feeds directly into F-120's unguarded `ORDER BY points`. |
| F-119 | High | 06 | WP-40 | `rust/web/src/db/game_write.rs:401-410` (+2 sites) | `concede_game_replace` clears `is_turn` unconditionally without reassigning the turn, so conceding on your own turn wedges the game - `find_bot_turns` returns zero rows and the replacement bot never plays. | 00-STATE: unit 06's ONE open dependency. At unified-report time cross-check WP-38's bot-turn wedge-recovery sweep (Unit 05); if it gates on `is_turn` rather than re-deriving from the game service, F-119 has NO production mitigation. Severity stated on the `is_turn`-gating assumption. |
| F-120 | Medium | 06 | WP-40 | `rust/web/src/db/game_write.rs:430-473` (+2 sites) | `end_game` is a fourth lifecycle writer that reads then writes and ends in an irreversible rating write with no `expected_updated_at` and no claim, violating the `docs/CODING.md` rule shipped in the same commit. | 00-STATE: systemic pattern 4b's mirror-image variant - not a test edited to agree with code, but a new doc rule scoped to three named functions so `end_game` is invisible to the grep procedure the doc itself prescribes. Must be named in the unified report's process-fixes section. |
| F-121 | Low, informational | 07 | - | `rust/web/src/bin/import_game.rs:20-32` | The 100 MiB import guard `stat`s the file then reads unbounded, so a FIFO or `/dev/stdin` reports len 0 and bypasses it, and the byte cap bounds no row counts in `import_bundle`. | CF-3. Dev-only CLI; operator supplies the path, so nothing attacker-controlled. |
| F-122 | Low, informational | 07 | - | `rust/web/src/game/import.rs:109,124` (+1 site) | `import_bundle` writes bundle-supplied `undo_game_state` verbatim with no validation and `undo_game` later replays it after only a non-NULL check. | CF-4 from Unit 06; downgraded - no HTTP route, no `#[server]` fn, sole caller is the dev CLI `bin/import_game.rs:35`. Reachability independently confirmed in Verified good (`router.rs:147-148`). |
| F-123 | REFUTED | 07 | WP-42 | `rust/web/src/visibility_cache.rs:12-13,25-31,58` | Claimed `VisibilityCache` cross-user visibility leak because the key omits the viewer while `is_proposal_visible_to_user` is user-scoped. | **REFUTED (report + 00-STATE agree, do not re-derive):** each instance is a plain local inside the per-request spawn at `events.rs:65`; one instance = one connection = one viewer. Archived text retained in report. Same pass also refuted the claim that WP-42 was reverted by the SSE migration - a useful negative against pattern 4e. Downgraded remnant is F-132; secondary owner-visibility half became F-133. |
| F-124 | High | 07 | WP-50 | `rust/web/src/proposals.rs:1730-1786` (+2 sites) | `add_proposal_player` passes `email` raw to `find_or_create_user_by_email_tx` and `check_invite_policy_tx` - no canonicalize, no empty/`@` check - so `" foo@x.com "` mints a verified ghost account and bypasses D7 block-by-target and `invite_policy`. | Unit's canonical instance of checklist-satisfied-literally: WP-50 spec 3c enumerated only `create_proposal` and `restart_game_with_roster`. Exploitable only because F-125's index omits the trim half. **00-STATE remediation proposal: a `CanonicalEmail` newtype whose only constructor is `canonicalize_email` would permanently close this class - the contract is today enforced only by doc comment. Fold into ONE item with F-127, F-128 and F-173.** |
| F-125 | Medium | 07 | WP-50 | `rust/web/migrations/026_canonical_emails.sql:33` | The unique index is on `lower(email)` while the backfill one line above normalizes with `lower(btrim(email))`, so trim-variant duplicates coexist and no `CHECK` enforces canonical storage. | The enabling half of F-124 (whitespace variant lives; case-only variant merely 500s). `CHECK (email = lower(btrim(email)))` is the stronger fix. |
| F-126 | Medium | 07 | WP-50 | `rust/web/src/proposals.rs:1733 -> :1772 -> :1191` | `add_proposal_player` is the only caller not honouring WP-50's "callers validate emptiness" contract: `email: Some("")` reaches `INSERT INTO user_emails ... VALUES ($1, '', true, NOW())`, creating a junk verified account then 500ing on the 23505. | Every other path confirmed rejecting (`login:296`, `confirm_login:349`, `add_email_address:856`, `create_proposal:1388`, `restart_game_with_roster:1293`, both client boundaries). Same fn as F-124. |
| F-127 | Low | 07 | WP-50 | `rust/web/src/db/game_write.rs:81-115` | `create_game_with_users_tx` resolves `opts.opponent_emails` by exact match and inserts the raw string, and is the one db helper that never got WP-50 criterion 3a's "callers must pass canonicalized addresses" doc comment. | Latent - all thirteen production callers pass `&[]`. `db/emails.rs:71` and `db/visibility.rs:171` got the comment. **00-STATE remediation proposal: `CanonicalEmail` newtype (only constructor `canonicalize_email`) closes this class permanently; contract is currently doc-comment-only. Fold into ONE item with F-124, F-128, F-173.** |
| F-128 | Low, note | 07 | WP-50 | `rust/web/src/email/inbound.rs:538` (+3 sites) | Canonicalization is Rust full-Unicode `to_lowercase` (`auth/email_addr.rs:3-5`) while the unique index (`026:33`) and the inbound authorization compare use Postgres `lower()`; inbound `extract_addr_spec` (`:134,150`) trims but never lowercases. | **CONFLICT - 00-STATE wins:** report records this as a fails-closed negative result; 00-STATE (via Unit 09b's F-173) says **F-128 is NOT closed and has NO owner** - `from_matches_verified_email` compares in SQL (`LOWER`) while every write path canonicalizes in Rust, and `İ@example.com` breaks. F-173 strengthens the `CanonicalEmail` newtype proposal. **Fold F-128, F-173 and the F-124/F-127 newtype proposal into ONE remediation item.** |
| F-129 | Medium (report) / **ESCALATED - ACCOUNT TAKEOVER** (00-STATE) | 07 | - | `rust/web/src/email/inbound.rs:520-530` (+2 sites) | The `s-{token}@brdg.me` settings-email token has no expiry, no rotation and no revocation - `ensure_settings_email_token` returns the same value forever and nothing NULLs it on use, logout or email removal, so any archived settings email is a permanent live bearer credential. | CF-2. **00-STATE: Unit 07 set an escalation condition and it FIRED.** F-161 (High, Unit 09a) escalates F-129+F-130 to account takeover: WP-56's inbound auth gate is fail-open three independent ways (cleanest: `spf=fail; dkim=none` -> `Pass`, because the code requires SPF *and* DKIM to both say "fail", inverting the DMARC rule); combined with this token's lack of expiry/single-use/rate-limit, spoofing `From:` is account takeover. **Session's most severe finding - top of the unified report's remediation order. In-report Medium is superseded.** Pattern 2 within one subsystem: `unsubscribe_token` rotates (`email/unsubscribe.rs:99`), invite tokens rotate (`proposals.rs:936-944`), this one does not. |
| F-130 | Medium (report) / **ESCALATED - ACCOUNT TAKEOVER** (00-STATE) | 07 | - | `rust/web/src/email/commands.rs:329-346` (+2 sites) | The "settings" token is not scoped to settings: a holder reaching `dispatch_standalone_server_command` also gets `new` (create a real game naming arbitrary opponents and bots), `bump` and subscribe/unsubscribe. | **00-STATE: ESCALATED with F-129 - see that row.** The report's sole mitigating control, `from_matches_verified_email` + SPF/DKIM/DMARC (`inbound.rs:1421-1433`, `:191-214`), is exactly what F-161 shows is fail-open, so the Medium rating's stated precondition no longer holds. In-report severity superseded. |
| F-131 | Low/Medium | 07 | WP-42 | `rust/web/src/events.rs:33-41` | SSE streams call `validate_session_token` exactly once at connect and never again, so after logout or session revocation the stream keeps delivering frames indefinitely - only visibility is refreshed (30s TTL), never authentication. | 00-STATE: routed to **Unit 09 for confirmation**. Adjacent to Unit 09's ownership of `efad81f`; raised here because the visibility work is WP-42's. |
| F-132 | Low | 07 | WP-42 | `rust/web/src/visibility_cache.rs:11` | `VisibilityCache` keys on an id alone, correct only because each instance is owned by exactly one SSE task with one fixed viewer - an invariant nothing in the type expresses. | Downgraded remnant of the REFUTED F-123 (see that row); 00-STATE agrees the leak is refuted. Fix: doc the per-viewer ownership requirement or key on `(id, Option<Uuid>)`. |
| F-133 | Low | 07 | WP-42 | `rust/web/src/db/proposals.rs:40-52` | `is_proposal_visible_to_user` grants visibility only via a `game_proposal_players` row and never consults `game_proposals.owner_user_id`, so an owner not also inserted as a player cannot see their own proposal. | Secondary half salvaged from the REFUTED F-123 (see that row). Untested in either direction - both tests (`:172`, `:193`) add the owner as a player explicitly. |
| F-134 | High | 07 | WP-79 | `rust/web/src/proposals.rs:1702-1709` | `start_proposal` calls `fetch_game_from_service` (a `reqwest` call) at `:1702` while still holding the `lock_proposal_for_update` `FOR UPDATE` row lock taken at `:1652`, so a hung game service blocks every concurrent respond/cancel/transfer/nudge. | Hoisting exactly this call is WP-79's whole point; done in `create_proposal` (`:1105`) and `restart_core` (`game/server_fns.rs:1091`), not here. The commit message reads clean. WP-79's own breakdown gotcha coming back positive. |
| F-135 | High | 07 | WP-79 | `rust/web/src/email/inbound.rs:1021-1034` | `91c723d4` - the WP-79 commit itself - inserted `fetch_game_from_service` at `:1022`, after the `begin()` at `:922` and the lock at `:931`, so the refactor moved the call out of `start_proposal_tx` but landed it on the wrong side of `begin()`. | Sharpest checklist-satisfied-literally instance in the unit. Harder to hoist than F-134 (`accepted_count` depends on the in-tx response UPDATE). Inbound-webhook path also holds the lock across the whole render. |
| F-136 | High | 07 | WP-46 | `rust/web/src/email/sweep.rs:135-137` | The `_ =>` catch-all after `fetch_email_recipient` swallows `Err(_)` as `PermanentSkip`, which `sweep_once` (`:289-305`) treats identically to `Sent` - so one transient DB error commits `mark_reminder_sent_tx` and the reminder is never sent. | Reintroduces the wfe F30 mark-without-send that WP-46 exists to remove; spec says errors are `Retry`. **00-STATE: the High-severity web-half instance of systemic pattern 5 (`_ => <default>` substitution, cf. F-65) - promote in the unified report, no longer a game-crate curiosity.** **Remediation pairing: fix with F-145 (Unit 07b) in the SAME change** - F-136 lives in the surviving duplicate of the abandoned wfe F36 dedup. |
| F-137 | High | 07 | WP-45 | `rust/web/src/game/server_fns.rs:1087` | `restart_core` takes client-supplied `bot_slots` from `restart_game_with_roster` (`:1271`, `:1299`, `:1334`) and never calls `validate_bot_slots`, so a restart carrying `bot_name: "garbage"` reaches `insert_game_from_service` and creates a wedged game. | WP-45 spec section 1 names `restart_core` as one of the three wd F27 call sites; `rg validate_bot_slots` has zero hits in the file. Solo-vs-bots branch (`:1178`) unguarded; multi-human branch saved only incidentally by `proposals.rs:1411`. |
| F-138 | Medium | 07 | WP-45 | `rust/web/src/db/bots.rs:61-63` (+4 sites) | `validate_bot_slots` matches with `n.eq_ignore_ascii_case(&slot.bot_name)` and neither returns nor imposes a canonical name, so all four entry points persist the client's string and no case-sensitive consumer will ever match it. | **00-STATE: closes the loop on Unit 05b's F-104 from the write side - ONE defect spanning FOUR units; remediate F-104 + F-138 + F-183 (09c) + F-189 (10a) as a SINGLE item.** Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 is the decoy test that hid it and must be re-fixtured in the same change. Entry points: `proposals.rs:1264`, `:1812`, `:1411`, `email/commands.rs:420`. |
| F-139 | Medium | 07 | WP-48 | `rust/web/src/game/import.rs:190-210` | The wd F10 unique-violation fallback runs `generate_unique_username` and a second INSERT on the same aborted import transaction (`placeholder_user` called at `:103` with `&mut tx`, no SAVEPOINT), so it always fails with 25P02 and only changes the error text. | Guard is present and satisfies its checklist row while changing nothing. Capped at Medium because the path is the dev-only CLI. Fix: nested savepoint before the retry, or a separate connection. |
| F-140 | Medium | 07 | WP-49 | `rust/web/src/db/game_types.rs:81-91` (+1 site) | `find_game_version_rules` / `find_game_version_render_meta` filter on `is_public = true AND is_deprecated = false`, but `run_rules` resolves the version from the game, so a player in an in-flight game on a deprecated version gets "Game version not found". | Consumed at `rust/web/src/email/commands.rs:939-946`; same breakage for `/rules/<version_id>` links from `email/notify.rs::rules_url`. Spec asked only for the public-page filter and never considered the by-game callers; public page itself verified correct. |
| F-141 | Low | 07 | WP-46 | `rust/web/src/proposals.rs:973` (+2 sites) | Rider wfe F40 required a LIMIT on all four sweep candidate queries; `fetch_nudge_candidates`, `fetch_expiry_candidates` and `fetch_auto_decline_candidates` (`:973`, `:1007`, `:1082`) are still unbounded - only `email/sweep.rs:44-53` got one. | Systemic pattern 2 (one of a set hardened, siblings left). Fix: apply the same shared const limit to all three. |
| F-142 | Low | 07 | WP-48 | `rust/web/tests/ssr_pages.rs:1290-1300` | `admin_export_route_rejects_non_admin` uses a fresh `Uuid::new_v4()` as the game id, so the spec's "body must not contain the private log body" assertion is vacuous - there is no game to leak. | **00-STATE: a confirmed "Test? y checklist row with no real test" instance; Unit 08 elevates this to the session's most-confirmed pattern (with F-148, F-149, F-150). Process fix: grep the checklists for "Test? y" rows and confirm a test exists for each.** Minor framing difference: report says the test exists with one vacuous assertion (the 403 half is real); 00-STATE files it under no-test-exists. Fix: seed a real game with a private log. |
| F-143 | Low, note | 07 | WP-46 | `rust/web/src/email/sweep.rs:260-306` | By WP-46 spec 3a the claim transaction holds a `game_players` `FOR UPDATE` lock across `send_reminder` - a game-service render plus the Resend API call - serialised over up to 200 candidates per tick. | Not a deviation from its own spec; recorded so WP-46 and WP-79 are not read as contradicting each other (WP-79 removes network-calls-under-lock on the proposal path while WP-46 mandates it on the sweep path). Unified report should reconcile into one policy. |
| F-144 | High | 07b | WP-46 (`69bcd1e`) - NOT WP-51 | `rust/web/src/email/sweep.rs:507-519` (+2 sites) | Invite-nudge dedup key `gp.nudged_at` is per-proposal while sends are per-invitee, so one web-suppressed invitee blocks the mark and re-nudges the whole roster every tick. | **00-STATE attribution: WP-51 (`dcd8844c`) introduced none of F-144/F-145/F-146** - this is WP-46's code, first review pass over it. Live duplicate-email bug (~1,344 dupes/invitee over the 14-day expiry at the 900s interval), not a nit. Carry to whoever owns the invite/proposal email surface. |
| F-145 | Medium | 07b | WP-46 (`69bcd1e`) - NOT WP-51 | `rust/web/src/proposals.rs:257-296` (+1 site) | `send_invite_core` folds `Err(_)` into `Ok(None)` at three `let-else` returns, so a transient DB failure returns `true` ("permanently unsendable"), marking the proposal nudged with nothing sent. | **00-STATE: must be fixed in the SAME change as F-136 (Unit 07)** - F-136 is the High-severity instance of systemic pattern 5 (`_ => <default>` substitution) living in the surviving duplicate of the abandoned wfe F36 dedup; same defect class in the two halves of one sweep module. **Attribution (00-STATE): WP-51 introduced none of F-144/F-145/F-146**, though WP-51 rewrote these exact three lines for wd F34 without fixing them. Third pattern-5 instance in the web half (F-65, F-136, F-145). |
| F-146 | Low/Medium | 07b | original #24 invite work (`4bd3135`/`db8f4b6`/`b88ff26`) - NOT WP-51 | `rust/web/src/proposals.rs:401` (+8 sites) | Five distinct proposal notifications (reinvite, decline, cancelled, started, ready) all render subject `"{game_type_name} invite"` and thread id `proposal-{id}`, collapsing actionable mails into one hidden conversation. | **00-STATE attribution: WP-51 introduced none of F-144/F-145/F-146.** Violates the de-threading house rule stated at `notify.rs:88-94` and applied on the turn path. Capped below High: nothing dropped, threading is client-dependent. |
| F-147 | Medium | 07b | WP-51 (`dcd8844c`) - WP-51's own | `rust/web/src/email/notify.rs:523-543` (+1 site) | wfe F36's dedup was consciously abandoned, but `notify::send_turn_reminder` shipped dead-at-birth with a doc comment stating the dedup as accomplished fact and the checklist records wfe F36 closed. | **00-STATE: sharpens F-109's sign-off rule** - a closed finding's citation must be *reachable*, not merely present; `send_turn_reminder` exists, has never had a caller, and its doc comment defeats F-109's check as originally written. Purest instance of pattern 1 (routing leak). Also a live trap: uses `SendMode::Normal` (turn opt-out) for a reminder. **00-STATE REFUTED, do not re-derive: there is no pattern-4e revert in `dcd8844c`** - it edited `sweep.rs::send_reminder` in place. F-147 + F-136 are one remediation item. WP-51 DOES have a spec (`planning/specs/WP-51-invite-mailer-notify-dedup.md`, Tier-2); WP-53 has none. |
| F-148 | Medium | 07b | WP-53 (`3610b957`) | `rust/web/src/db/game_write.rs:739` | `wd F6`'s `CASE WHEN $9` elimination guard is correct but wholly unpinned - deleting it fails no test in the repository, on a row the checklist marked "Test? y". | **00-STATE: one of four confirmed "Test? y checklist row with no test actually existing" instances (with F-142 (07), F-149, F-150 (08)); Unit 08 elevates this to the session's most-confirmed pattern, to be a top-level systemic pattern in the unified report.** Explicitly NOT pattern 4b - fix correct, commit clean, row honestly closed, guard simply unpinned. Process fix: grep checklists for "Test? y" rows and confirm a test exists. |
| F-149 | Low | 07b | WP-53 (`3610b957`) | `rust/web/src/friends.rs:229-231` | `wd F61`'s required test is absent and `friends.rs` (634 lines) has no `#[cfg(test)]` module at all, so nothing asserts `block_user`'s "User not found" guard. | **00-STATE: same "Test? y with no test" cluster as F-142, F-148, F-150** - session's most-confirmed pattern. Lead's recorded decision (`EXECUTION-STATE.md:175`) excuses only the *integration* test on "db layer already tested", but the new guard is not in the db layer. Code itself correct; TOCTOU unreachable (no `DELETE FROM users` in `rust/`). Only `wd F25` of WP-53's three "Test? y" rows got a test. |
| F-150 | Medium | 08 | WP-52 | `docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B5-web-domain-stats-misc.md` (+7 checklist rows) | All seven `Test? y` rows of WP-52 shipped with no test - `f374434d` touches no test file and adds no `#[cfg(test)]` block, while four of the seven change query result semantics. | **00-STATE: with F-142 (07), F-148 and F-149 (07b) this is the fourth and largest confirmed "Test? y checklist row with no test actually existing" instance - the session's most-confirmed pattern, to be elevated to a top-level systemic pattern in the unified report.** Aggravated by a pre-existing `#[sqlx::test]` module + fixtures at `rust/web/src/stats/queries.rs:762-2287`. A `wd F48` test would have caught F-151. Also: WP-52 has NO spec - extend 00-STATE's no-spec list (WP-24, WP-27, WP-44, WP-53, WP-79). |
| F-151 | High | 08 | WP-52 | `rust/web/src/stats/queries.rs:104-152` (+1 site) | `wd F48`'s game-type name filter is applied only inside the `qualifying` CTE and not to the `gtu` side of a FULL OUTER JOIN, so `.next()` on the alphabetically-ordered result returns another game type's rating and record. | Public unauthenticated endpoint (`viewer_user_id: None` valid); silent wrong data for any player rated in >1 game type. Regression is the pairing of the half-applied filter with the caller's `.find` -> `.next` switch at `rust/web/src/stats/mod.rs:266-279`. Also renders the `.unwrap_or_else` zeroed fallback at `:271-279` near-unreachable. The `wd F48` test F-150 records as missing would have caught it. |
| F-152 | Medium | 08 | WP-52 | `rust/web/src/stats/queries.rs:713` | `wd F55`'s `NULLS LAST` fix skipped `recent_form_for_game_type`, whose byte-identical `finished_at DESC` window still sorts NULL-`finished_at` legacy rows to `rn = 1`, displacing recent games on the game-type leaderboard. | Systemic pattern 2 (inconsistent hardening within one file) - web-half instance beside F-61 and F-116. Aggravating: the same commit edited this function for `wd F50` (`:700-701`). Named sites fixed at `:312` and `:638`; `rating_series:195`, `game_history:455` unaffected. |
| F-153 | Medium | 08 | WP-52 | `rust/web/src/stats/queries.rs:7-20` | `wd F50`'s "one `const` used by all eight sites" shipped as an `#[allow(dead_code)]` string with ZERO referents and a doc comment stating manual sync is now required - nine hand-synced copies instead of eight. | **00-STATE: NEW named pattern, "the documentation-only constant"** - a row asking for extraction to a shared definition satisfied by creating the definition and touching no call site. Relative of pattern 5 but distinct: it leaves a greppable `#[allow(dead_code)]` marker. **Sign-off action: sweep `rg "allow\(dead_code\)"` across the commit range.** Durability finding only - nine copies verified in sync today. Stated `sqlx::query!` blocker is real but `macro_rules!` + `concat!` would have satisfied the row. |
| F-154 | Medium | 08 | WP-52 | `rust/web/src/stats/mod.rs:343-348` | `wd F52`'s canonicalization binds `find_game_type_name`'s `None` for an unknown game type straight into the `($3::text IS NULL OR gt.name = $3)` predicate, so an unknown filter returns the player's entire history instead of nothing. | Fails the row's one explicit criterion - parity with `get_player_game_type_stats`, which 404s at `stats/mod.rs:258-264`. `filters.game_type` at `:384` then tells the client no filter was applied. Second result-semantics change in this commit with an unfilled `Test? y` box - see F-150. |
| F-155 | Low | 08 | WP-52 | `rust/web/src/stats/queries.rs:511` (+2 sites) | `wd F53`'s justifying comment is copy-pasted verbatim to three `query_as` sites and is factually wrong at `game_history_count`, whose destination is the tuple `(i64,)`, not a named `FromRow` struct; "binds are static" argues FOR the macro it declines. | Third row in this commit satisfied by an artifact rather than the effect (with F-153, F-150). Exactly the F-147/F-109 hazard: the citation exists at all three sites and a sign-off grep marks the row closed. Sites `:232`, `:429-430`, `:509-510`. |
| F-156 | Medium | 08 | WP-52 | `rust/web/src/index.rs:47-73` | `wd F74`'s `take(20)` bound is applied to `list_friends`' `ORDER BY lower(u.name)` output, so the home page's friends-recent-games feed is truncated alphabetically - friends sorting after the 20th are permanently invisible with no UI indication. | Milder form of F-151's class: a perf row that changed which subjects the feature covers. Concurrency half correct (`try_join_all`), though it fires up to 20 simultaneous pooled queries per render - `buffer_unordered` suggested in the same change. `list_friends` at `rust/web/src/db/social.rs:205-217`. |
| F-157 | Low | 08 | WP-52 | `rust/web/src/friends.rs:100-108` (+1 site) | `tokio::try_join!` rewrites collapsed eleven per-query `internal(...)` error contexts into two catch-alls, so on-call cannot tell which of six friends queries or five game-info queries failed. | Observability regression only; no behaviour change and no row prohibited it. Second site `rust/web/src/game_info/mod.rs:164-172`. `wd F62`/`wd F75` otherwise satisfied correctly. Fix: keep per-future `.map_err` inside the `try_join!` arguments. |
| F-158 | High | 09a | - | `rust/web/src/events.rs:33-41` (+1 site) | SSE resolves the viewer once at connect and never re-validates, so a revoked session keeps streaming private events indefinitely. | Discharges obligation 3; concretises F-131. Visibility-staleness half is bounded (~30s `VisibilityCache` TTL) and recorded as acceptable; only session revocation is unbounded. |
| F-159 | Medium | 09a | - | `rust/web/src/events.rs:47-112` (+1 site) | Both SSE tasks exit only on a `tx.send` failure needing a visible event, so disconnected/idle/anonymous viewers leak tasks and NATS subs forever. | Interacts with F-109: `efad81f` deleted WP-36's ws F55 shutdown drain and the SSE replacement reintroduces the same lifecycle family. `sse_connections` gauge counts leaked tasks as live, hiding it. |
| F-160 | Medium | 09a | - | `rust/web/src/events.rs:117-183` (+2 sites) | Unauthenticated public SSE handler skips `VisibilityCache`, subscribes to the `game.>` firehose and has no rate limit - attacker-scaled DB/decode amplification. | Confirmed pattern 2 (hardened sibling ten lines up). Confirms F-94 (no rate-limiting middleware anywhere in `rust/web`; the two doc comments asserting a per-IP limit are false). Compounds with F-159. |
| F-161 | High | 09a | WP-56 | `rust/web/src/email/inbound.rs:164-219` (+4 sites) | WP-56's inbound auth gate is fail-open three independent ways, so the `From` header is not authenticated and Unit 07's DMARC premise is unsound. | **Session's most severe finding** (`00-STATE.md`, `00-HANDOVER.md`); top of the remediation order. Discharges obligation 2 with answer NOT SOUND and **escalates F-129 + F-130 to account takeover** under the condition Unit 07 set. Settings token has no expiry/single-use/rate limit. Report heading states severity as "High, and it escalates F-129 + F-130". |
| F-161a | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:719-723` (+4 sites) | `AuthVerdict::Unknown` proceeds on a `warn!` only, and `Unknown` is returned whenever the authserv-id is not exactly `amazonses.com`. | Sub-letter of F-161 (High); no separate severity given. Pipeline is Resend, not SES - a different authserv-id makes the whole gate inert in production. No test against a captured real message; no metric or alert on `Unknown`. |
| F-161b | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:213-218` | `Pass` means "not explicitly failed": `failed(dmarc) \|\| (failed(spf) && failed(dkim))` inverts the DMARC rule, so `spf=fail; dkim=none` is accepted. | Sub-letter of F-161 (High). The cleanest row - unconditional forgery derivable from the file alone, no deployment assumption. Also passes `dmarc=none`, `spf=softfail; dkim=none`, and `spf=neutral/none/permerror/temperror`. |
| F-161c | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:170-178` | The topmost-header rule defends only against an *added* second `Authentication-Results`; since `Unknown` proceeds, an attacker-supplied sole header is honoured verbatim. | Sub-letter of F-161 (High). Depends on F-161a. |
| F-161d | unstated | 09a | WP-56 | `rust/web/src/email/inbound.rs:1794-1808` | The two tests named for the lenient boundary are decoys - each input carries an independently passing result, so the "nothing authenticated" cases are untested. | Sub-letter of F-161 (High). Cited in `00-STATE.md` as making decoy tests a confirmed *class*; F-151 decoy family crossed with pattern 4f. Third tooth of the sign-off rule. |
| F-162 | Medium | 09a | - | `rust/web/src/email/inbound.rs:992-1060` (+7 sites) | Seven pre-commit transient failures in `handle_invite_reply` return `Done` instead of `Retry`, so svix never redelivers and an authenticated invite response is lost silently. | **Pairs with F-169** - same `RouteOutcome` contract (`inbound.rs:742-750`), different route. `00-STATE.md`: the `RouteOutcome` sweep is SETTLED - F-162 and F-169 are the only two sites, no third route has the defect. |
| F-163 | Low | 09a | - | `rust/web/tests/sse_events.rs:456-457` (+1 site) | The SSE migration's replacement for a deleted default-running regression test is `#[ignore = "takes 32+ seconds"]`, so the timeout property is no longer checked. | Explicitly **NOT pattern 4e** - the original test predates the programme, so no checklist row is falsified. Recorded as the obligation-1 near-miss alongside the single true 4e instance (F-109). |
| F-164 | Low | 09b | - (obligation 4 product) | `rust/web/style/main.scss:1091-1094` | `.friend-request-badge` hardcodes the CSS keyword `orange` instead of a `var(--mk-*)` token, breaking under all 34 themes including colour-blind palettes, with no non-hue cue. | Product of obligation 4, which DISCHARGED F-15 as LATENT (no live violation at the real emitter). Sole exception in 1,095 lines; the file's own rule at `:761-762` forbids it. |
| F-165 | Medium | 09b | T3-B6 (ws F60, `Test? n`) | `rust/web/src/websocket_client.rs:84-101` | The reconnect fix put the global `last_update` refetch bump inside the `ready_state == Closed` guard, so the friend-request badge never refreshes in a healthy tab. | Over-application beyond the checklist row (the row asked only to guard `open()`). No SSE event exists for friend requests. `Test? n`, so no test was owed - does NOT count toward the "Test? y with no test" tally. |
| F-166 | Medium | 09b | `dec967b6` | `rust/web/src/game_info/queries.rs:14-24` (+1 site) | Pattern 2: the `, name DESC` tiebreak was added to one "latest game version" query and not its sibling, so rules links and game creation can pick different versions. | Sweep complete - exactly two `ORDER BY ... LIMIT 1` sites in `rust/web`; the fixed direction is confirmed correct against the operator. Carries a secondary Low maintainability note (three disagreeing definitions of "latest version"). |
| F-167 | Low | 09b | - (obligation 4 product) | `rust/web/src/theme.rs:12-19` | `CHROME_SOFTENS` keeps a dead `(Red, 86)` entry with zero consumers, emitting 72 dead CSS declarations per page, and its doc comment misdescribes the set. | Obligation 4 concluded. Deadness predates the remediation window, so not programme-introduced; obligation 4 simply did not clean it. `chrome_softens_meet_contrast_floor` explicitly NOT a decoy. |
| F-168 | Low | 09b | WP-54 (wfe F61) | `rust/web/tests/ssr_pages.rs:256-266` (+1 site) | Both accessibility regression tests assert absence of the `cursor:pointer` marker rather than presence of `href="#"`, so reverting the fix still passes. | Explicitly NOT a decoy in the F-151 sense - the weakness is inherited from the spec's own acceptance criterion. Filed as "criterion falsifiable in only one direction". Fix verified present at HEAD. |
| F-169 | High | 09b | WP-57 (`65c22edc`) §3b | `rust/web/src/email/inbound.rs:1392-1433` | Pattern 2: the at-least-once `Retry` fix landed on the game and invite routes but not settings; `handle_settings_reply` returns `()`, so transient DB errors silently discard the command. | **Pairs with F-162** - same `RouteOutcome` contract, settings route vs invite route; remediate together. `00-STATE.md`: the `RouteOutcome` sweep is SETTLED - no third route has the defect. |
| F-170 | Medium | 09b | WP-58 (`390dd3b8`) §3a | `rust/web/src/email/render.rs:35-42` (+1 site) | `EmailKind::pref_column()` has zero `src/` callers; the live column mapping is an untested duplicate `match`, and the only test asserting it guards nothing. | Instance of F-153's "documentation-only constant" pattern in `pub fn` form, crossed with the decoy-test class. `00-STATE.md`: F-170 is **NOT extended** by the game-start mail (REFUTED in 09c) - that path reads `turn_emails_enabled` directly. |
| F-171 | Medium | 09b | WP-58 §6 rider row 2 (`Test? y`) | `rust/web/src/email/inbound.rs:1377-1380` | The `List-Unsubscribe*` deletion from `send_rules_reply_response` landed but the promised absence test does not exist; the function has no test at all. | **Fifth confirmed "Test? y with no test"** row (joins F-142, F-148, F-149, F-150) and the most explicit - the row named the assertion. Later rolled into the nine-row tally with WP-60's four (F-176). |
| F-172 | Low | 09b | WP-59 (`f56ff375`) Task 1 | `rust/web/src/email/inbound.rs:135` | The CRLF sanitiser truncates at the first CR/LF where the spec required replacement with a space, so a legally folded `From` parses to `None` and the move is dropped with a 200. | Near-decoy: the covering test exercises only the injection case where truncate and replace agree. The injection half of the criterion IS satisfied. |
| F-173 | Low | 09b | WP-59 (out of mandate) | `rust/web/src/email/inbound.rs:532-545` (+1 site) | Inbound `from_matches_verified_email` normalises via SQL `LOWER()` while every write path uses Rust `canonicalize_email`, so e.g. U+0130 addresses can never match. | **F-128 is NOT closed and has NO OWNER.** Folds into ONE `CanonicalEmail` newtype remediation item with F-128, F-124 and F-127. Explicitly outside WP-59's mandate. Breadth bounded by deployment DB collation - not verifiable this session. |
| F-174 | Low | 09b | WP-58 follow-up (`5786a1b6`) | `rust/web/src/email/commands.rs:179-208` (`:192`) | `help_text()` still advertises `rules` and four game-only verbs to standalone/no-game users, who are then told the command is unavailable. | Residual of `5786a1b6`, whose fix corrected only the rejection string despite a "help text" commit subject. Pattern 4b/4e for `5786a1b6` is REFUTED; F-174 is the residual that refutation points to. |
| F-175 | Medium | 09c | WP-60 | `rust/web/src/email/outbound.rs:123-139` (+1 site) | `ensure_settings_email_token` / `ensure_unsubscribe_token` keep the pre-fix select-then-update body, so tokens can be returned unpersisted or lost to a concurrent writer. | Pattern 2; same shape as F-116, F-166, F-169. The checklist scoped `wfe F44`/`F45` by function name, so the row reads satisfied. `00-STATE.md`: the WP-60 token expiry/single-use hypothesis is REFUTED - this is the real pattern-2 gap, and F-161's substance is untouched by WP-60. |
| F-176 | Medium | 09c | WP-60 | `rust/web/src/email/outbound.rs:301-364` (+2 sites) | `e5513ec6` adds no test at all, yet all four of WP-60's `Test? y` rows (`wfe F44`, `F45`, `F46`, `F63`) are marked tested. | One ID covering FOUR falsified rows; brings the session "Test? y with no test" tally to **nine**. The F44/F45 guard is a pre-existing decoy (F-151 / F-161d class). WP-76/WP-77 rows must NOT be added to this tally - `EXECUTION-README.md:408` records them as a deliberate no-spec/no-row gap. |
| F-177 | Low | 09c | WP-60 | `rust/web/src/email/render.rs:252-262` | Two of four `href` interpolations in the same function (`unsub`, `manage`) were left unescaped ten lines below the two that got `escape_html_attr`. | Pattern 2 inside `wfe F49`. Impact theoretical today - no attacker-controlled byte reaches either URL. F49 offered escape OR documenting the trusted-URL precondition; neither was done fully, so the row is undischarged. |
| F-178 | Low | 09c | WP-60 | `rust/web/src/email/render.rs:152-164` | The new `escape_html_attr` duplicates the existing `html_escape` in the same module tree, whose "no public HTML-escape helper exists" comment is still in place. | Maintenance duplicate, not a live defect. `html_escape` has a test; `escape_html_attr` has none. |
| F-179 | Medium | 09c | WP-76 (`bc051164` / `ca7925bc`) | `rust/web/src/email/inbound.rs:1076` (+2 sites) | The invite-accept auto-start mails the same invitee three times for one event, gated by three different preference columns, so no single unsubscribe damps the burst. | WP-76 has NO spec and NO checklist row (deliberate, `EXECUTION-README.md:408`) - no `Test?` column to falsify. Unit 10-adjacent: hits the sending-domain reputation WP-57/WP-58 were spent protecting. |
| F-180 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:1471` | The solo-game start notify is unreachable in practice - `suppress_for_web_presence` suppresses it every time in the normal hydrated-page flow. | Harmless but contradicts the commit message; untested. |
| F-181 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:1470-1478` (+4 sites) | All start sites publish `bot.turn` before `notify_game_emails`, so a fast bot move lets both the start-path notify and `handle_bot_command_event` mail the same transition. | Ordering pre-dates `ca7925bc` (hence Low) but the commit widened it to two more sites. |
| F-182 | Low | 09c | WP-76 | `rust/web/src/email/proposals.rs:111-120` | Both WP-76 commits call the free notify function instead of the existing `ProposalMailer` seam, so none of the new wiring is spyable or tested. | Explicitly a DISCLOSED gap, not a false `Test? y` - WP-76 has no `Test?` column and `EXECUTION-STATE.md:18` records the missing spy infra. Route as testability debt, not checklist integrity. |
| F-183 | High | 09c | WP-77 (defect is WP-59-era code) | `rust/web/src/email/commands.rs:82-93` (written at `:398-401`) | Email `new` lowercases the bot name into `game_bots.bot_name` while the bot service looks it up case-sensitively, so the bot never moves and the game wedges silently. | **Remediate as ONE item with F-104, F-138 and F-189** - one bot-name case-sensitivity defect spanning four units. Fix = canonicalize inside `validate_bot_slots` and return the canonical name. F-185 must be re-fixtured in the same change. Precondition: `admin::create_bot` (`admin.rs:293-303`) permits arbitrary casing. NOT introduced by `33150afe`. Report states the pairing as three items; `00-STATE.md` adds F-189 - `00-STATE.md` wins. |
| F-184 | Low | 09c | WP-77 | `rust/web/src/components/opponent_slot.rs:93-97` (+2 sites) | `set_mode` runs on an ungated radio before `bot_names` settles, so the hard-coded `"medium"` default can still be stored, rendering a blank `<select>` and failing submit. | Pre-settle residual of an otherwise correct fix; the settled path is REFUTED as a defect (WP-77's own default IS canonical - a byte-for-byte copy of the `bots.name` column). No spec, no checklist row, so no test was owed. |
| F-185 | Low | 09c | WP-77 | `rust/web/src/email/commands.rs:1435-1455` | `classify_opponent_detects_bots` uses an all-lowercase fixture, so lowercasing and canonicalising are indistinguishable and it asserts the lowercased output as correct. | Pattern 4b decoy - **the test that hid F-183**; must be re-fixtured in the same change as F-183 / F-104 / F-138 / F-189. Its partner `validate_bot_slots_accepts_case_mismatch` is already filed as F-104 (pattern 4f in `00-STATE.md`). |
| F-186 | High | 10a | - | `rust/bot/src/crypto.rs:66-70` (+1 site) | The bot silently falls back to the hardcoded dev encryption key when `DATABASE_ENCRYPTION_KEY` is unset - no opt-in gate, no `MissingKey` variant. | The new finding routed from the F-96 investigation with F-90; the forbidden "dev default + warn" pattern (`docs/CODING.md:701`). Remediate as ONE item with F-187 and F-188. |
| F-187 | Medium | 10a | - | `rust/bot/src/crypto.rs` (+1 site) | `rust/bot/src/crypto.rs` is a divergent duplicate of `rust/web/src/crypto.rs` on four axes; every web hardening is absent from the bot copy. | **F-90 is NOT closed at HEAD** - recorded as fixed, but the fix landed only in the web copy. Pattern 2 at file granularity. One item with F-186/F-188; `00-STATE.md` says fix F-90 and F-108 together. |
| F-188 | Medium | 10a | - | `rust/bot/src/nats.rs:1-36` (+2 sites) | Bot and web NATS wire types and constants are copy-paste duplicates with no shared type and no cross-crate round-trip test. | F-108 still open. No live wire drift today, but non-wire divergence already exists (bot hardcodes web's local `ack_wait`). One item with F-186/F-187. |
| F-189 | High | 10a | - | `rust/bot/src/config.rs:26-29` (+2 sites) | The case-sensitive `WHERE name = $1` bot lookup misses and the miss path returns `Ok(())`, acking and discarding the turn - the game wedges. | **Extends F-183** (bot-side half CONFIRMED) and adds a second, previously uncited site `rust/bot/src/config.rs:67`. Remediate as ONE item with F-104, F-138 and F-183 - one bot-name defect spanning four units. The silent ack at `main.rs:186-194` must be in the same change. |
| F-190 | Low | 10a | - | `rust/bot/src/main.rs:809-816` (+2 sites) | An invalid `DATABASE_ENCRYPTION_KEY` only warns and sets the key to `None`; every turn then errors and strands in the WorkQueue stream. | Consistent-fix candidate with F-186 (fail startup outright, as the web crate does). |
| F-191 | Low | 10a | WP-06 | `rust/lib/cmd/src/http.rs:26-29` (+1 site) | A malformed request *envelope* is rejected by axum's `JsonRejection` (400/422 text) instead of the documented HTTP 200 `Response::SystemError`; untested at any layer. | Confirms the `00-STATE.md` Unit 10 carry-forward about `http.rs`'s axum final form. The WP-06 test is explicitly NOT a decoy - only the WP-06 acceptance narrative overstates. |
| F-192 | Medium | 10a | - | `rust/lib/game_client/src/lib.rs:25-35` (+2 sites) | `HttpStatus`/`ParseResponse` embed the whole game-service body - every seat's private state - in `Display`, reaching `tracing::error!` and Sentry. | Medium pair with F-193 (F-193 is the cause); remediate together. Belongs in the hidden-information section with F-22/F-28, not logging. Found because `prompt.rs` was REFUTED as a leak vector. |
| F-193 | Medium | 10a | - | `rust/lib/game_client/src/lib.rs:310-331` (+1 site) | `fetch_game_data` requests `Request::Status`, pulling every seat's `player_renders` plus raw `game.state` into the bot, then discards all but one. | Cause half of the F-192/F-193 pair. Narrower `PubRender` + `PlayerRender{player}` endpoints already exist. |
| F-194 | Low | 10a | - | `rust/bot/src/main.rs:585` (+1 site) | `players[].score` reaches the prompt from `gamer.points()` - the one prompt input bypassing the `pub_state` redaction boundary. | Not a bot defect: the platform already treats points as public. Pairs with the carried-forward unnumbered item "`Gamer::points()` has no documented ordering contract", which `00-STATE.md` hands to the remediation plan. |
| F-195 | Low | 10a | - | `rust/bot/src/main.rs:276-282` | TRACE logging emits `system_prompt`/`user_prompt` verbatim, exposing the bot's own hand to anyone with log access. | Own-seat only, off by default. |
| F-196 | Medium | 10a | WP-62 | `rust/operator/src/controller.rs:240` (+2 sites) | The authoritative-version guard only writes forward, so deprecating or deleting the newest version leaves the stale `game_types` row permanently unrepaired. | `cleanup` (`:174`) has the same shape and **zero test callers**. A fresh instance of "satisfied the row literally, missed what it was for" plus cross-file pattern 2. |
| F-197 | Low | 10b | WP-65 | `rust/game/love-letter-2/.rls.toml` (+3 sites) | The `e F28` sweep worked from an enumerated file list, so four byte-identical `.rls.toml`/`.gitignore` siblings survived and the row was accepted as complete. | Textbook pattern 2. Row is `Test? = n`, so NOT a falsified row. The `build-release` eradication itself did land. |
| F-198 | Low | 10b | - | `rust/bot/src/main.rs:776-827` (+2 sites) | `rust/bot` is the only TLS-capable binary with no rustls process-default install and declares no `rustls` dependency at all. | Explicitly NOT a checklist falsification (the `docs/CODING.md` rule is conditional) and NOT a WP-64 regression - the omission is original. No live panic demonstrable, hence Low. |
| F-199 | Low | 10b | WP-65 | `.github/workflows/deps-currency.yml` (+1 site) | The weekly `cargo deny` job can fail with no notification wiring and checks `advisories` only - never `bans`, `licenses` or `sources`. | **Remediates as ONE item with F-206 and 10b's Coverage gap 3** - three views of one unenforced `bans` section. Row `dp F23` is `Test? = n`, not a falsified row. |
| F-200 | Medium | 10b | WP-66 | `rust/lib/session_store/src/postgres_store.rs:87-130` (+2 sites) | `migrate()`'s duplicate-key branch returns `Ok(())` before `create table` and without committing, so a concurrent cold start reports success with no session table. | **The vendoring finding and a new named pattern**: an upstream defect inherited *because* the correctly-followed "minimal port, not a rewrite" criterion guarantees it comes along. WP-66's spec gate was honoured; the cost landed anyway. Recommend a "known upstream defects inherited" criterion for future vendoring specs. Feeds the owner's open vendoring-policy question. |
| F-201 | Low | 10b | WP-66 | `rust/web/src/db/users.rs:256` (+2 sites) | Three sqlx error-classification sites - the only ones in the workspace - crossed the 0.8 -> 0.9 major bump with no re-check and no test. | Filed Low as an unverified risk, not a demonstrated defect; the point is procedural. Same class as F-200's second-order risk. |
| F-202 | Low | 10b | WP-66 | `rust/web/src/db/test_support.rs:146-152` | `count_rows` interpolates `table: &str` with no validation; the 0.9 migration added the `AssertSqlSafe` wrapper to satisfy the compiler and audited nothing. | Not a live injection surface and not introduced here. Contrast recorded: the vendored store's eight `AssertSqlSafe` sites are genuinely safe. |
| F-203 | Low | 10c | WP-64 | `rust/Cargo.toml:78-79` | WP-64 shipped `[workspace.lints.clippy]` but silently dropped the spec-prescribed `[workspace.lints.rust]` table, and no later commit added it. | Spec-vs-code gap, explicitly NOT a regression. No stricter per-crate config was displaced (zero `#![deny/warn/allow]` at `4fb252da^`). WP-64 has no checklist row, so not a falsified `Test? y`. |
| F-204 | Low | 10c | WP-64 | `rust/Cargo.toml:56-76` | Ten of 21 `[workspace.dependencies]` entries are bare-major/bare-minor, violating WP-64's rider 1 - which the same spec's §3b contradicts. | Spec-vs-code gap, not a regression, and NOT a clean falsification: the acceptance criterion is internally inconsistent. `sqlx = "0.9"` was set by WP-66, not WP-64. |
| F-205 | Low | 10c | WP-67 | `docs/reviews/2026-07-23-rust-review/SUMMARY.md:44-46` (+3 sites) | `dp F12` was closed on a premise that was never true (sentry defaults dragging in actix-web/ureq), and rider 2's mandated downgrade of the finding never happened. | **New named pattern: "the finding whose premise was disproved, closed anyway, never amended."** Distinct from 4b - the docs were never edited at all despite an explicit criterion requiring it. Sign-off fix: a disproved mechanism must amend the finding, not merely close it. Report states severity as "Low, but a NEW NAMED PATTERN". |
| F-206 | Medium | 10c | WP-69 | `rust/deny.toml:71-76` (+3 sites) | WP-69's spec set a STOP-AND-REPORT threshold ("roughly a dozen" skips); 29 landed and the commit wrote a pre-emptive rebuttal into `deny.toml` instead of stopping. | Unit 10c's headline and a new process-fix pattern: *a spec's own escalation trigger fired and the implementation answered it with a comment*. **Remediate as ONE item with F-199 and 10b's Coverage gap 3.** The rebuttal is falsified by `rust/deny.toml:131`. Compounding: WP-69 §5's negative checks are recorded as parked, never run. `00-STATE.md` correction applied: **29** skip entries, not 24. |
| F-207 | Low | 10c | WP-66 | `rust/Dockerfile:132` (+2 sites) | Three sqlx migrators write `_sqlx_migrations` - prod `sqlx-cli` pinned 0.8.6, CI unpinned, library 0.9 - with no commit or spec justifying the split. | **Deployment-checklist item, not a code finding - groups with the F-96 deployment-checklist family.** No commit in the 127-commit range touches `rust/Dockerfile` at all. Mitigating: `rg 'migrate!' rust` is empty, so nothing validates checksums at runtime. |
| F-208 | High | 11 | - (`f4cbc51d`, `c882d413`, `16dae9dd`) | `rust/Dockerfile:36` (+3 sites) | `hanamikoji-1` is a workspace member compiled by every image build but has no Dockerfile stage, no docker-bake target and no k8s Deployment - it is unshippable. | A complete, tested, documented new game with no build- or deploy-time signal that it does not ship. `rg 'hanamikoji'` finds zero hits outside the crate and the manifests. No commit since 2026-07-20 touches `rust/Dockerfile` or `docker-bake.hcl`, so the 127-commit window could never have caught it. Pairs with the new process-fix item: a CI guard over the four hand-maintained delivery lists. `lords-of-vegas-1` is the other absentee (WIP, owner-excluded). |
| F-208a | unstated | 11 | - | `k8s/base/game/kustomization.yaml` (+2 sites) | The carried "43 k8s Deployments vs 26 image stages" discrepancy: 43 = 26 Rust stages + 17 legacy Go games with stages in `brdgme-go/Dockerfile`. | **REFUTED - a carried premise, not a defect.** Zero stages lack a Deployment; zero bake targets lack a stage. Drop the 43-vs-26 framing from the unified report. Sub-letter carries no severity. |
| F-209 | Medium | 11 | - (crate never subject to any WP) | `rust/game/hanamikoji-1/src/lib.rs:673-730` (+4 sites) | `validate` bounds every parallel vector and seat index but never relates `phase` to `pending`, so a deserialized state can wedge the game forever or silently destroy three cards. | Textbook systemic pattern 2b - the parallel-vector sweep is present, the one cross-field invariant is missed; here the consequence is a wedge/corruption rather than a panic, which is quieter. Proof that having a `validate` test is not sufficient. |
| F-210 | Medium | 11 | - (`ae04843c`; WP-72-class self-certification) | `rust/game/sushi-go-2/src/lib.rs:140-147` (+1 site) | `ae04843c` replaced `_ => 9` with `unreachable!()` on the false premise that `start()` is the only entry point, so `all_players` outside `2..=5` now panics the game service. | **Remediate as ONE item with F-06's sushi-go-2 row** - sushi-go-2 has no `validate` override at all and `all_players` is never bounded. Second WP-72-class self-certifying commit (no spec, no `T3-B*` row, acceptance evidence is a "Done" row it wrote itself) and the **first where the self-certified premise is demonstrably false**. F-96-class hardening-into-panic; exact inverse of pattern 5. |
| F-211 | Low | 11 | - (`e2aef66b`; code change `68ebef7`) | `rust/web/end2end/tests/page-loads.spec.ts:8` (+1 site) | The e2e smoke assertion was edited down to the `<h1>`'s own `brdg.me` fallback string, so it no longer distinguishes a healthy index page from a degraded one. | Pattern 4b (test adjusted to agree with the code), milder than F-72a/F-83/F-79/F-95 because the code change was intentional. Compounding: the `e2e` job is `continue-on-error: true` (`ci.yml:148`), so no assertion in the file can fail a merge or deploy. Group with the F-96 deployment-checklist family. |

The proposed remediation work breakdown is a separate document; this report does not contain it.
