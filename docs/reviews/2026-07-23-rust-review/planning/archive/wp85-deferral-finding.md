# WP-85 deferral safety check

Worker unit, 2026-07-26. Established by READING the planning corpus and
`docs/authoring/COMMANDS.md`. No build, test or git-mutating command was run;
nothing under `rust/` was read or modified.

## VERDICT

**YES - indefinite deferral of WP-85 is safe.** Nothing in the planning corpus
is gated on WP-85. It is a leaf: zero packages name it as a predecessor, zero
specs assume parser-first dispatch has landed, and the one behavioural cost is
already recorded and accepted by Michael in D-55.

The cost of deferring, restated so it is not rediscovered as a bug later:
**acquire-1's and starship-catan-1's top-level `end` move stays unplayable by
email**, because `dispatch_email_command`'s `"end"` arm intercepts before the
game parser. `decisions-session3.md` D-55 already records this as *"an accepted
cost of the deferral, not an oversight."*

## What WP-85 blocks

**Nothing.** Evidence:

- `grep -rn 'WP-85' planning/ --include='*.md'` matches only five files:
  `decisions-session3.md` (D-54/D-55, the rulings that created and deferred it),
  `specs/WP-85-email-parser-first-dispatch.md` (itself),
  `specs/WP-59-inbound-processing-quality.md` (the carve-out banner and two
  cross-references), `work-packages.md` (its entry, the status legend and the
  recount block), and `specs-LOG.md` (the work records). **No file states a
  dependency ON WP-85** - every mention is provenance, status or bookkeeping.
- `landing-order.md`, read IN FULL (597 lines) - the stated authority on
  inter-package ordering - **does not mention WP-85 at all**. It is in none of
  the chains: sections 4, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 7.1, 8.1, 8.3, 9, 10.1.
- WP-85's own spec lists its Prerequisites (Michael's escape-hatch decision; the
  parse-miss-vs-user-error question) and no dependents. Its only forward
  obligation is self-contained: whoever lands it must fix WP-59's now-false
  "Known collisions" / "no escape prefix" text.
- WP-85 carries **0 findings** (`work-packages.md` WP-85 entry, and the recount
  block). `wfe F29` stays counted in WP-59, so no finding is orphaned by the
  deferral and coverage math (570) is unchanged.
- Actionable-now package count is unaffected: `work-packages.md` recount block
  states "Actionable-now count is **still 77** - unchanged by WP-85, which
  arrives already blocked."

## WP-59 -> WP-58 chain analysis

**The WP-59 -> WP-58 dependency attaches to WP-59 Task 5, NOT to the carved-out
Task 14.** Carving Task 14 out to WP-85 does not touch the chain.

| Claim | File it rests on |
|---|---|
| `landing-order.md` 6.5 opens: *"**WP-59 -> WP-58.** WP-59 Task 5 shrinks `handle_settings_reply_route` and explicitly defers every unsubscribe concern to WP-58/D-10. WP-58 must not land first and re-derive that route."* Task 5 only. | `planning/landing-order.md` (section 6.5) |
| WP-58's own spec states the same: *"**Landing order:** **WP-59 first** (its Task 5 shrinks `handle_settings_reply_route`; WP-59 explicitly defers all unsubscribe work here)."* | `planning/specs/WP-58-unsubscribe-rfc8058.md` |
| WP-58 explicitly fences command dispatch OUT of its scope - its Non-goals list "no change to ... the settings-route auth (WP-56 / WP-59), `EmailContent`'s fields, or the reply-address domain constant (WP-59 Task 8)", and separately "`emails on/off\|invite\|reminder` verbs unchanged". No dispatch-precedence dependency. | `planning/specs/WP-58-unsubscribe-rfc8058.md` (section 4, Non-goals) |
| WP-59's own carve-out banner asserts the same in the other direction: *"**Nothing else in WP-59 depends on Task 14** - the rest of the package is unaffected and proceeds as written."* Restated in the amended D-15 note: *"The only task D-15 gated was **Task 14** ... Nothing else in WP-59 depends on it."* | `planning/specs/WP-59-inbound-processing-quality.md` (Task 14 banner; the 2026-07-26 amendment above it) |

WP-59's other recorded downstreams likewise attach to non-Task-14 tasks, so all
of them survive the carve-out intact:

- **WP-59 -> WP-57** (`landing-order.md` 6.2, and WP-57's spec: *"**WP-59 must
  land first.** It rewrites `select_route`, adds ..."*): Tasks 1/2's
  `select_route` / `fetch_inbound_text`.
- **WP-59 -> WP-40** (`landing-order.md` 2 and 3, and WP-40's spec): Task 9's
  `classify_server_fn_error`.
- **WP-56 x WP-59** (`landing-order.md` 2): Tasks 9, 10, 11 - either order.
- **WP-82 -> WP-59** (`landing-order.md` 7.1): WP-59 lists `web/src/db.rs`.

None of these names Task 14 or dispatch precedence.

## Reverse direction: does any spec assume parser-first dispatch has landed?

**No.** Grepped `planning/specs/`, `planning/checklists/` and the top-level
planning docs for `parser-first`, `parser first`, `parser is tried`,
`escape-hatch`, `escape prefix`, `reserved-verb`, `reserves verb`.

- Every hit that is *about* email dispatch is in the WP-85 / WP-59 / D-15 / D-55
  narrative itself, or in `decisions-needed.md` / `decisions-ANSWERED.md` /
  `work-packages.md` / `specs-CLASSIFICATION.md` recording the D-15 ruling.
- All other hits are unrelated uses of the words: WP-07 (a test-race comment
  described as an "escape hatch"), WP-42 (TTL-cache "escape hatch"),
  `critical-path.md` and `decisions-needed.md` D-3 (the undo "misclick escape
  hatch"), WP-54's implementer-fence wording, WP-21/WP-03 ("preserved verbatim").
- **WP-54 explicitly disclaims any dependency**, and is the only spec that had to:
  *"**Nothing in this package is gated on D-15.** Do not 'harmonise' the UI
  wording with whatever D-15 decides for the email verb."*
- **The one shared file is `docs/authoring/COMMANDS.md`** (WP-56, WP-59, WP-85
  are the only specs naming it). WP-56 is already fenced OFF it: *"`docs/authoring/COMMANDS.md`
  (game-author facing, and WP-59 Task 14 owns its email section - do not edit)."*
  Read live: `docs/authoring/COMMANDS.md` contains **no** match for `reserved`
  or `email` (case-insensitive) - i.e. the email/reserved-verbs section **does
  not exist in the file today**. So deferring WP-85 leaves no stale or wrong
  documentation behind; there is simply nothing there, which is the correct
  state under a deferral. WP-56 can proceed under its existing fence.

## Two stale-text nits (doc hygiene, NOT blockers)

Recording them so they are not mistaken for dependencies. Neither can mislead an
executor into wrong work - both point at a task that now carries a loud
CARVED-OUT banner.

1. `work-packages.md` WP-59 entry still instructs *"do not execute Task 14 as
   specced. Rewrite the COMMANDS.md section ..."*, written before the carve-out.
   It should point at WP-85 and its DEFERRED status.
2. `specs/WP-56-email-from-auth-redesign.md` (Documentation updates) says
   *"WP-59 Task 14 owns its email section"*; the owner is now WP-85.
   `work-packages.md` (the 2026-07-26 decision-session bullet) has the same
   pre-carve-out wording, but that block is dated historical narrative.

`specs-LOG.md` already flags a third of this class: it recommends *"a follow-up
unit fold item 1, `:12` and `:374` into the WP-85 carve-out narrative"* in
`specs/WP-59-inbound-processing-quality.md`.

## Files read

- `planning/landing-order.md` (IN FULL)
- `planning/specs/WP-85-email-parser-first-dispatch.md` (IN FULL)
- `planning/work-packages.md` - WP-85 entry, WP-59 entry, status legend region,
  recount blocks (grep-located regions, not whole file)
- `planning/specs/WP-59-inbound-processing-quality.md` - Task 14 banner region,
  the amended D-15 note, and grep-matched lines (:12, :374). NOT read whole.
- `planning/specs/WP-58-unsubscribe-rfc8058.md` - landing-order header and
  Non-goals
- `planning/specs/WP-57-inbound-webhook-delivery-semantics.md` - grep-matched
  landing-order lines
- `planning/specs/WP-56-email-from-auth-redesign.md` - Documentation-updates
  section and grep-matched WP-59 lines
- `planning/decisions-session3.md` - D-54 and D-55
- `planning/specs-LOG.md` - GREPPED only; matched regions read via grep output
- `docs/authoring/COMMANDS.md` - grepped for `reserved`/`email` (zero hits);
  `ls docs/authoring/`
- Corpus-wide greps: `WP-85`; `WP-59` outside its own spec; parser-first /
  escape-hatch / reserved-verb across `specs/`, `checklists/` and top-level
  planning docs

## UNKNOWN

- **Whether the `end` collision is still live in `rust/`.** WP-85's spec and the
  WP-59 verification claim `acquire-1` and `starship-catan-1` expose top-level
  `end` moves that the dispatcher intercepts. This unit did **not** verify that
  against source (reading `rust/` is out of scope here, and the tree is under
  concurrent edit). The verdict does not rest on it: whether the collision is 1,
  2 or 0 games, D-55 already accepts the cost, and no *package* is gated either
  way.
- **The parse-miss-vs-user-error question** inside WP-85 (can
  `crate::game::execute_command` distinguish "did not parse" from "parsed, user
  wrong"?) remains UNKNOWN, as WP-85's own spec states. It is a second
  prerequisite for landing WP-85 - it does not affect deferral safety, but it
  means unblocking WP-85 needs more than just Michael's verb list.
- **Whether Michael wants the two stale-text nits fixed now** or folded into the
  later destructive/consolidation pass. Not a blocker either way.
