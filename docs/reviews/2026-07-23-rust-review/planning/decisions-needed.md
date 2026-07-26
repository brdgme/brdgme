# Decisions needed - brdgme Rust review remediation

Each item is self-contained: context, question, options, recommendation.
Answer with the item number and a short reply (e.g. "D-3: option B").
D-1..D-34 are the 34 items from REVIEW.md section 6; D-35..D-40 were
surfaced during triage grouping. Work packages in `work-packages.md`
reference these as their unblock condition.

Suggested answering order: the security/data-integrity group (D-1, D-3,
D-5, D-8, D-6) unblocks the top of the backlog; the parity group
(D-35 then D-26..D-34) can be answered in one sitting; the dependency
group (D-17..D-25) mostly has obvious recommendations.

---

## ANSWERED - 2026-07-25 session (all 10 critical-path gating groups)

The user answered the 10 gating decision groups that block the critical
path. Each item below carries an **ANSWERED** block in place. Two answers
deviate from the recommendation: **D-5** (bots stay referenced by name)
and **D-12+D-14** (no session expiry; email-change re-verification
instead).

| Group | Items | Answer | Deviates? |
|---|---|---|---|
| 1 | D-1 | B - settings token + SPF/DKIM + remove account-security cmds from email | no |
| 2 | D-3 (+ D-4) | A - forbid undo once finished; yes to shared `undo_core`/`concede_core` | no |
| 3 | D-5 | C-lite **MODIFIED** - sweep + alert + Progress heartbeat, but **bots stay by NAME**; dangling bot names are a SUPPORTED no-op state + admin warning | **yes** |
| 4 | D-8 | C - validate at all 4 entry points AND at game start; reconciled with D-5 | no |
| 5 | D-6 (+ D-13) | A - gate details/feeds on participation-or-public; stats anonymize; rules public | no (D-13 label, see its block) |
| 6 | D-2 | A - at-least-once; marker after success; 5xx to force retry | no |
| 7 | D-12 + D-14 | A **MODIFIED** - fail closed in prod, expiring claims, windowed cap, revoke-all, but **NO session expiry**; email change requires re-verification | **yes** |
| 8 | D-33 (+ D-35) | A - counts/aggregates public, secrets in `player_state`; official rules authoritative on port-parity conflicts | see D-35 block |
| 9 | D-36 | A - bounds-check player index at requester boundary + per-game validate hook | no |
| 10 | D-37 | A - error on non-empty `rest` + escape braces in `to_string` | no |

Package status flips are applied in `work-packages.md`. Still-open
decisions (D-7, D-9, D-10, D-11, D-15..D-32, D-34, D-38..D-41) are
unaffected by this session.

---

## REFINEMENTS - 2026-07-25 (later session)

The user then refined four things. Each is recorded in place; this is the index.

| # | Item | Change |
|---|---|---|
| 1 | **D-1** | Command removal **NARROWED**: only `emails add`/`confirm`/`active`\|`use`/`remove` leave email. Username (`name`), `theme`, `colors` and notification prefs are **KEPT**. Token + SPF/DKIM work unchanged. Cold start **resolved**: web UI manages settings and surfaces the tokenised settings address as an opt-in reveal; **never** in email footers. |
| 2 | **D-1** | `emails remove` **confirmed** removed; the spec's revert-path caveat is withdrawn. |
| 3 | **D-13** | Now **ANSWERED** (option B shape), label ambiguity resolved. `/ws` verified **anonymous** today but the session layer already covers the route. Recommended design + answers to the user's three technical questions recorded under D-13. Scope reflected in WP-42/WP-47. |
| 4 | **D-35, D-26..D-32, D-34** | **PARKED-PENDING-USER-RULES-REVIEW.** See the Group D banner, which also lists five **egregious-fix candidates** (a F1, b F4, b F7, e F30, d F37) flagged for the user, and the not-parked carve-outs (WP-25, WP-15, WP-10, WP-19, WP-22, WP-23, WP-29). |

Sequencing facts for the implementing agent live in `planning/landing-order.md`.

---

## ANSWERED - 2026-07-26 session (ALL remaining decisions)

**Every open decision is now closed.** The user answered the 34-row table in
`open-decisions-for-user.md` (now a stub); the resolved record, with the
constraints attached to each ruling, is **`planning/decisions-ANSWERED.md`** -
read that first. Each item below also carries its own `ANSWERED (2026-07-26)`
block.

Closed in this session: **D-7, D-8 (refined), D-10 (extended), D-11, D-14,
D-15 (redesigned), D-16, D-17, D-18, D-19, D-20, D-21, D-22, D-23, D-24,
D-25, D-35 (park confirmed), D-37 (corrected), D-38, D-39, D-40**, plus the
`bo F25` cluster rider and the six N-items (N-1..N-6) that lived only in the
user-facing table. `D-9` was also confirmed (option B). `D-41` was already
resolved by a Lead ruling.

**Five rulings SUPERSEDE previously recorded text. Do not follow the old text:**

| item | old position | new ruling |
|---|---|---|
| **D-7** | option A: `--redact-private`, default ON for a user-facing export path | **OVERRULED** - no redacted export at all; the only export is the full bundle, admin-only |
| **D-8** | restart path exempted, falling into D-5's dangling-name no-op + admin warning | **REFINED** - restart resolves a deprecated bot to the **latest non-deprecated** version |
| **D-15** | "A-plus": keep a reserved-verb list, game-scoped override | **REDESIGNED** - game parser first, platform commands as fallback; only a small hard-reserved escape-hatch set (`help`) always wins |
| **D-16** | option A: explicit Turnstile `render()` from a login-component effect | **OVERRULED** - option B: `/login` is a normal unrouted link forcing a full page load |
| **D-37** | bare `{{` as the literal-brace escape | **CORRECTED** - `{{lbrace}}`; a bare `{{` cannot be implemented soundly |

**Parity outcomes for the five egregious candidates** (Group D banner updated
in place): `a F1` FIX NOW; `b F7` FIX NOW; `e F30` CONDITIONAL; `b F4` PARKED
with the user's correction; `d F37` REJECTED, not a bug.

**Standing constraints introduced this session** (they bind more than the row
that produced them - full text in `decisions-ANSWERED.md`): dependency work
upgrades everything to latest FIRST (D-17); macro surfaces stay small and
obvious (D-20); WP-04 keeps the parser straightforward and obvious (D-38); no
Sentry functionality may be lost (D-18); `lib/cost` gains suitable automated
tests (D-25).

---

## Group A - security and data integrity

### D-1. Email From-header authentication redesign [unblocks WP-56]
Context: the inbound-email settings route is authenticated solely by the
spoofable From header, and email-management commands (add/confirm/remove
addresses) run over that path - together a full account takeover (both
remaining criticals). Unrouted None addresses also fall through to the
settings handler.
Question: how should inbound email be authenticated, and do
account-security commands belong on the email path at all?
- A. Per-user secret settings token in the reply address (like the
  existing per-game email_token), drop the None fallthrough, require
  Resend SPF/DKIM pass verdicts. Keeps email settings commands.
- B. Option A's token+SPF/DKIM, but remove account-security commands
  (add/remove email) from the email path entirely; email keeps only
  game/notification commands.
- C. Kill the email settings route; all settings via web only.
Recommendation: B. Tokenised auth fixes the spoofing class, but
account-security mutations have web UI equivalents and are not worth the
attack surface on a forgeable channel. Low product cost, removes both
criticals structurally.

**ANSWERED (2026-07-25): Option B.** User's words: "Require the s- token
for the settings route, verify SPF/DKIM on inbound, AND remove
account-security commands (add/confirm/activate email address) from the
email interface entirely."
Consequences for WP-56: (a) the settings reply address must carry a
per-user secret `s-` token, (b) inbound SPF/DKIM verdicts from Resend
must be consulted and a fail must reject, (c) the unrouted-`None`
fallthrough to the settings handler is removed, (d) `emails
add`/`confirm`/`active` (account-security mutations) are deleted from the
email command surface. Interacts with D-12+D-14: the email-change flow
therefore lives in the web UI with a confirmation link.

**REFINED (2026-07-25) - scope of (d) NARROWED, and cold start RESOLVED.** The
user does **not** want all settings commands off the email interface. Only these
four verbs leave email:

- `emails add <addr>` (adding an additional email address)
- `emails confirm <code>` (completing that add)
- `emails active <addr>` / `emails use <addr>` (changing the default/active
  address)
- `emails remove <addr>` - **confirmed yes**, removed too. The WP-56 spec's
  earlier "one-arm revert if the Lead disagrees" caveat is **withdrawn**.

**KEPT on the email interface** (the user's ruling: not sensitive): `name`
(username / display name), `theme <name>`, `colors`/`colours`, bare `emails`
(listing), `emails on`/`off`, `emails invite on|off`, `emails reminder on|off`,
and `settings` (summary). Notification preferences, username and theme by email
are a product feature and stay.

Consequences (a)-(c) are **unchanged**: the `s-` token, the SPF/DKIM verdict gate
and the removal of the unrouted-`None` fallthrough all remain in full. They are
now the controls that protect the *retained* commands, so narrowing (d) does not
narrow them.

**Cold start - RESOLVED, no longer an open question.** Settings are managed in the
**web UI**, and the tokenised inbound settings address (`s-{token}@brdg.me`) is
surfaced **there** as an **opt-in reveal** on the settings page. The token is
**NOT** to appear in email footers (turn emails, proposal emails,
`List-Unsubscribe`, or anywhere else) - a bearer secret in a footer leaks with
every forwarded message. Building the web reveal is not WP-56's; it belongs to the
`settings.rs` owner. WP-56 must simply not invent a fallback discovery path.

### D-2. Sweep/webhook delivery semantics [unblocks WP-46, WP-57]
Context: the inbound webhook inserts its dedupe marker before processing
(post-marker failures permanently dropped, always 200); every outbound
sweep marks before doing; FOR UPDATE SKIP LOCKED runs under autocommit
fetch_all and is a no-op, so concurrent replicas can double-send.
Question: at-least-once or at-most-once delivery, and sync or enqueued
webhook processing (svix timeout is 15s)?
- A. At-least-once: marker/mark after success, 5xx on transient failure
  so svix retries, claim-then-send (real transaction) in sweeps.
  Occasional duplicate email possible.
- B. Keep at-most-once but fix the claim atomicity only (no retries);
  failures still drop silently.
- C. A plus enqueue: webhook verifies+dedupes+persists then 200s;
  processing happens in a worker.
Recommendation: A now, C only if webhook processing time actually
approaches the svix timeout. Dropped turn commands are worse than a rare
duplicate notification.

**ANSWERED (2026-07-25): Option A.** At-least-once: write the dedupe
marker only **after** successful processing, and return 5xx so the
provider retries. The same shape applies to the turn-reminder sweep - do
not mark `sent` on skip paths. Processing idempotency rests on the
marker. No enqueue/worker split (option C) for now.
Unblocks WP-57 fully; WP-46 remains blocked on D-11 only.

### D-3. Undo-vs-ratings semantics [unblocks WP-40]
Context: undo_game on a finished game reverts state but never rewinds
rating_change, and the idempotency guard then blocks re-rating the real
outcome - permanent rating corruption (the web-domain critical). Same on
the email undo path.
Question: may a finished game be undone at all; if yes, how do ratings
recover?
- A. Forbid undo once is_finished (guard in db::undo_game). Simplest;
  loses the "oops, misclicked the winning move" escape hatch.
- B. Allow undo of finished games; atomically rewind ratings using the
  stored per-player rating_change deltas, clear rating_change so
  re-finish re-rates. (Recompute-only is known-unsound: double-counts.)
- C. Allow within a short grace window (e.g. 5 min) with B's rewind.
Recommendation: A. It is the only option with no rating-math risk, and
undo-after-finish is a rare edge; revisit B later if users ask.

**ANSWERED (2026-07-25): Option A.** Forbid undo once a game is
finished, making the ratings corruption **unreachable**. Do **NOT**
attempt any rating rewind - no delta-reversal code, no recompute. The
missing rating rewind is therefore explicitly out of scope for WP-40.

### D-4. concede/undo TOCTOU + path unification [informs WP-40, no separate block]
Context: db::undo_game and db::concede_game skip the optimistic-locking
(updated_at guard) discipline the move path has; the email path
duplicates the server-fn logic near-verbatim and has the same races.
Question: confirm the intended shape: fix once in db.rs with guards, and
extract shared concede_core/undo_core used by both web and email paths?
- A. Yes - guards in db.rs + shared core fns.
- B. Guards in db.rs only; leave the duplication.
Recommendation: A. The duplication has already drifted once; this is the
cheap moment to unify.

**ANSWERED (2026-07-25): Option A (yes).** Share `undo_core` /
`concede_core` between the web and email paths "so the missing
concurrency guards are fixed once", with the `is_finished` /
`updated_at` guards living in `db.rs`.

### D-5. Bot-turn wedge recovery + NATS delivery [unblocks WP-38]
Context: every bot-turn failure mode wedges a game permanently and
silently: UserError acked without re-publish, retry exhaustion, publish
lost after DB commit, bot rename/delete/disable causes skip-and-ack
(games reference bots by NAME). No ack-deadline heartbeat for long turns.
Question: what is the recovery architecture?
- A. Reconciliation sweep: periodic "bot on turn for > N minutes ->
  re-publish bot.turn" job. Self-heals every wedge mode including lost
  publishes; simple to reason about.
- B. Per-error handling: re-publish on UserError, DLQ + alert on retry
  exhaustion, transactional outbox for publishes. More moving parts,
  faster recovery.
- C. A + B's DLQ/alerting for visibility.
Sub-questions: (i) reference bots by id via migration, or keep name +
warn-on-rename? (ii) AckKind::Progress heartbeat or raise ack_wait?
Recommendation: C-lite: reconciliation sweep as the safety net (A), plus
a metric/log alert on retry exhaustion; bots by id (migration - rename
should never strand games); Progress heartbeat (ack_wait raising just
moves the cliff).

**ANSWERED (2026-07-25): C-lite, MODIFIED - DEVIATES FROM THE
RECOMMENDATION.** User's words, verbatim: "reconciliation sweep +
retry-exhaustion alert + Progress heartbeat on long turns, BUT bots stay
referenced BY NAME (the user wants to swap bots by name; do NOT convert
to bot ids). Dangling bot player names are an explicitly SUPPORTED state:
they no-op rather than wedge the game, and the admin page shows a warning
listing dangling bot player names. Disabling all bots must remain a valid
intentional configuration."
Consequences for WP-38:
- **No bot-id migration.** Games keep referencing bots by name; renaming
  or swapping a bot by name is a supported product capability.
- A bot player name that resolves to nothing (deleted, renamed away, or
  disabled) must **no-op**: the game does not wedge, the message is
  acked, and the condition is surfaced, not retried forever.
- The admin page gains a warning listing dangling bot player names.
- "All bots disabled" is a valid intentional configuration and must not
  trip alerts or blocking validation.
- Sub-question (ii): `AckKind::Progress` heartbeat on long turns (not an
  `ack_wait` raise).
See D-8 for the creation-time-vs-later reconciliation.

### D-6. game_visibility scope [unblocks WP-47, WP-49]
Context: the game_visibility setting exists and is_game_visible_to_user
is implemented, but no read endpoint uses it: any authed user can read
any game's details; anonymous stats endpoints name private users. The
rules/game-info pages also have an auth-posture wrinkle (anonymous
game-info page links to an auth-gated rules endpoint).
Question: which endpoints does game_visibility gate, and do stats
anonymize or filter private users?
- A. Gate game details + history/feeds; stats anonymize private users
  ("Anonymous" label, aggregates kept).
- B. Gate details/feeds only; stats unchanged (documented as public).
- C. Gate everything including stats rows (filter out private users'
  games entirely) - distorts aggregate stats.
Also: make rules pages public (they are game rules, not user data)?
Recommendation: A, and yes - rules/game-info fully public. Anonymize
keeps stats useful while honouring the setting; filtering breaks
head-to-head math.

**ANSWERED (2026-07-25): Option A.** Gate game details and activity
feeds on **participation-or-public**. Stats compute **globally** but
**anonymize** private users - do **not** exclude them from aggregates.
Rules pages stay public. Answered jointly with D-13; see D-13.

### D-7. Export bundle privacy [unblocks WP-48]
Context: the export bundle includes private log bodies and hidden state;
its stated purpose is paste-into-issues debugging.
Question: accept that, or add redaction?
- A. Add --redact-private (default ON for the user-facing path; full
  bundle behind an explicit flag/admin).
- B. Accept and document.
Recommendation: A with default-on redaction; debugging rarely needs
other players' hidden hands, and pasted bundles are effectively public.

**ANSWERED (2026-07-26): OVERRULED - NEITHER OPTION AS WRITTEN. Do not
build a redacted user-facing export at all.**
**This SUPERSEDES the recommendation above (option A, `--redact-private`
default ON for a user-facing path). Do not implement it.**
- **The only export path is the full bundle, admin-only.**
- The `--redact-private` flag is **out of scope**.
- The user-facing export path is **out of scope**.
- Bug reporting is by **game ID**, not by pasted bundle.
- The user **explicitly accepts the risk** that game state may change
  after a report is filed and render that report useless.
**WP-48's scope shrinks accordingly** - see its entry in
`work-packages.md`. wd F7's privacy work becomes "make the export
admin-only"; the remaining import nits are unchanged mechanical riders.

### D-8. Bot-slot validation choke point [unblocks WP-45]
Context: client-supplied bot slots are unvalidated at 4 entry points
(create_proposal, add_proposal_player, restart_core, email `new`); a
bogus/disabled bot name creates a game that wedges unrecoverably
(compounds D-5).
Question: where does the single validation live?
- A. One shared fn (validate against enabled bots) called at all 4
  entry points.
- B. Validate at game-start time only (start_proposal_tx /
  create_game_from_service) - single true choke point, but users learn
  about the bad bot late.
- C. A + B (defence in depth; B is the invariant, A is the UX).
Recommendation: C - B is cheap once A's helper exists, and the invariant
must hold even for future entry points.

**ANSWERED (2026-07-25): Option C** - validate at all 4 entry points AND
at game start.
**Reconciliation with D-5 (state this explicitly in both WP-45 and WP-38
specs):** validation applies to **names at creation/start time**, so the
user gets immediate feedback for a typo or a bot that does not exist
right now. A name that goes missing or is disabled **LATER** must NOT
wedge the game and must NOT cause a rejection at turn time - it falls
into D-5's dangling-name no-op path plus the admin warning. In other
words: validate on write, tolerate on read.

**REFINED (2026-07-26) - the restart path gets an ACTIVE resolution, not a
carve-out.** Option C and "validate on write, tolerate on read" still
stand. What changes is the game-restart case:
- **On restart, resolve a deprecated bot to the LATEST NON-DEPRECATED
  version of that bot**, and start the game with that.
- **This SUPERSEDES** the proposal put to the user, which was to merely
  exempt the restart path from write-validation and let it fall into
  D-5's dangling-name no-op plus admin warning. **That no-op fallback is
  NOT the answer for restart** - do not implement it there.
- The no-op-plus-admin-warning path remains correct for the *other* case
  D-8 describes: a bot name that goes missing or is disabled **after** a
  game has started must not wedge the game and must not be rejected at
  turn time.
Reflect this in the WP-45 spec (`specs/WP-45-bot-slot-validation.md`) and
keep WP-38's D-5 text consistent with it.

### D-9. Email canonicalization policy [unblocks WP-50]
Context: emails are stored/compared untrimmed and case-sensitive across
auth, invites, new-game and settings; duplicate accounts and
invite-policy bypass are possible. A storage-normalization change
touches the unique constraint and existing rows.
Question: normalize where?
- A. Trim + lowercase at all input boundaries only (no migration).
- B. A + one-off migration lowercasing stored rows + citext/lower-index
  unique constraint.
Recommendation: B. Boundary-only leaves existing mixed-case rows
permanently un-matchable against new normalized input; the migration is
small and the collision risk (two accounts differing only by case) is
worth surfacing once, deliberately.

**ANSWERED (2026-07-26): Option B**, as recommended. Trim + lowercase at
all four input boundaries, PLUS the one-off migration lowercasing stored
rows, PLUS the lower-index (or citext) unique constraint. Surface the
case-collision risk once, deliberately, during the migration. Unblocks
WP-50.

## Group B - platform behaviour

### D-10. Unsubscribe RFC 8058 compliance [unblocks WP-58]
Context: List-Unsubscribe-Post is advertised but the mailto path can
never honor one-click semantics; Gmail/Yahoo bulk-sender rules expect a
working HTTPS one-click endpoint. The standalone email dispatch also
rejects subscribe/unsubscribe verbs its own help text advertises.
Options:
- A. Build the HTTPS one-click endpoint (tokenised, no auth redirect).
- B. mailto-only; drop the Post header; fix the help text.
Recommendation: A - deliverability to Gmail/Yahoo is a real product
concern for a turn-notification product.

**ANSWERED (2026-07-26): Option A, WITH AN ADDITION.** Build the HTTPS
one-click endpoint (tokenised, no auth redirect) as recommended, and
additionally the mail must carry **two visible links**:
1. A **type-specific** unsubscribe link matching the email type actually
   received - e.g. "Unsubscribe from game reminders" on a reminder mail.
2. A **"Manage my subscriptions"** link to the user settings page.
The `List-Unsubscribe` / `List-Unsubscribe-Post` headers **still point at
the one-click endpoint**; the visible links are **additional, not a
replacement** for the headers. Also fix the help text that advertises
subscribe/unsubscribe verbs the standalone dispatch rejects. Unblocks
WP-58.

### D-11. Reminder preference semantics [unblocks WP-46]
Context: the reminder sweep gates on turn_emails_enabled, but a separate
reminder_emails_enabled flag exists.
Question: which flag governs reminders?
- A. reminder_emails_enabled governs reminders; turn_emails_enabled
  governs turn notifications only.
- B. Reminders require BOTH (a reminder is a kind of turn email).
Recommendation: A - it is what the flag names promise users; B silently
makes reminder_emails_enabled dead when turn emails are off, which is
the current bug's shape.

**ANSWERED (2026-07-26): Option A.** `reminder_emails_enabled` alone
governs reminder emails; `turn_emails_enabled` governs turn
notifications only. The reminder sweep must **not** consult
`turn_emails_enabled`.
**User's rationale, which is the design intent to preserve:** some users
play mainly by web and do not want turn emails, but reminders are still
useful to them if they have **missed or forgotten** a game. Option B
would make the reminder flag dead for exactly those users.
Unblocks WP-46 - its last remaining blocker (D-2 was already answered:
at-least-once, do not mark `sent` on skip paths).

### D-12. Fail-open posture: Turnstile + encryption key [unblocks WP-35]
Context: Turnstile verifier errors fail open and an unset secret
silently disables it; an unset DATABASE_ENCRYPTION_KEY silently uses a
hardcoded public fallback key (one warn line).
Options:
- A. Fail closed in prod: refuse startup on missing key; Turnstile
  errors reject (or challenge) instead of pass; explicit DEV_MODE opt-in
  re-enables the lenient behaviour locally.
- B. Keep fail-open, add loud alerting.
Recommendation: A. A silently-disabled encryption key is a data breach
in waiting; dev ergonomics are preserved by the explicit opt-in.

**ANSWERED (2026-07-25): Option A** - fail closed in production on auth
failure; refuse startup on a missing `DATABASE_ENCRYPTION_KEY`; Turnstile
verifier errors reject rather than pass. Answered jointly with D-14; see
D-14 for the modification.

### D-13. /ws unauthenticated site-wide firehose [unblocks WP-42]
Context: any connection to /ws receives every site event, no auth, no
subscription filtering.
Options:
- A. Accept at current scale; document; revisit at growth. (Combined
  with D-6 this leaks private-game activity.)
- B. Require a session and filter to per-connection subscriptions
  (games the user can see).
- C. Session required now; subscription filtering later.
Recommendation: B if game_visibility (D-6) is being enforced -
otherwise the visibility work leaks through the socket; the filtering
predicate is the same is_game_visible_to_user.

**ANSWERED (2026-07-25): answered jointly with D-6 as "Option A".**
Substance recorded: activity feeds are gated on
participation-or-public using the same `is_game_visible_to_user`
predicate, which necessarily includes the `/ws` event stream (otherwise
the D-6 work leaks straight through the socket).
**FLAG RESOLVED 2026-07-25 - D-13 is now ANSWERED, option B shape.** The earlier
label ambiguity (literal "A" = accept the firehose, which contradicted "gate
activity feeds") is settled. **The user's stated intent, verbatim in substance:**
gate the `/ws` feed; only send public-game events to a client that actually has
that public game's page open. The user recalls the previous version of brdg.me
supporting client `sub`/`unsub` commands for specific public games, and asked
whether user-specific events could instead ride on websocket authentication.

#### Answers to the user's technical questions (verified by reading source)

**(a) `/ws` authentication today: none. Fully anonymous.**
`rust/web/src/router.rs:142` registers `.route("/ws", get(websocket::ws_handler))`.
`ws_handler` (`rust/web/src/websocket.rs:82-87`) is five lines and takes exactly
two extractors - `WebSocketUpgrade` and `State<GameBroadcaster>`. No `Session`
extractor, no `HeaderMap`, no cookie read, no `get_current_user`, no token query
param. The upgrade is accepted unconditionally, and
`rust/web/tests/websocket_hygiene.rs:71-81` *asserts* that a cookie-less connect
gets `101 Switching Protocols`.
**But identity IS available on that path.** `/ws` is registered at router.rs:142
**before** `.layer(session_layer)` at router.rs:155, so the Postgres-backed
tower-sessions `SessionManagerLayer` (`auth/session.rs:26-39`) does wrap it - the
`Session` is in request extensions and a `Session` extractor would resolve.
(Contrast `/healthz` at router.rs:162, deliberately registered *after* the layer
to bypass it.) The one thing that could have blocked authentication is already
correct.

**(b) What it broadcasts: an unfiltered site-wide firehose.**
`GameBroadcaster` (`websocket.rs:29-80`) publishes `GameUpdateSignal { game_id }`
to NATS subject `game.{game_id}` and `ProposalUpdateSignal { proposal_id }` to
`proposal.{proposal_id}`. Every socket then independently subscribes to the
**wildcards** `game.>` and `proposal.>` (`websocket.rs:112`, `:119`) and forwards
each payload verbatim as `Message::Text` with no filtering and no per-socket state.
Payloads are skinny JSON - only UUIDs cross the wire, no game state or names - so
the data leak is bounded, but the *existence and timing* of every move and every
proposal event site-wide is visible to any anonymous connection. There is no
`user.>` subject; `websocket.rs:200,228` actively assert nothing is published to
`user.>` or `ws.>`. Filtering is entirely client-side in the WASM
(`websocket_client.rs:37-85`, `app.rs:842-856` `track_game_seq`), and the global
`trigger.last_update` counter is bumped for **every** frame - which keys the
sidebar `active_games` resource (`app.rs:129-133`) and `HomePage`'s `public_index`
(`app.rs:294-299`). So every site-wide event causes a server-fn refetch on every
connected client: an N-clients x all-events amplification, a real load bug
independent of the privacy question.

**(c) No subscribe/unsubscribe protocol exists, server or client.**
The server polls inbound frames (needed for pong/close) but discards the payload -
`websocket.rs:165-172`, comment verbatim: "we don't act on client-sent data here".
No command parsing, no client->server message enum. The client never sends: the
`send` handle of `UseWebSocketReturn` is not bound (`websocket_client.rs:51-55`),
only `on_message_raw`. **No vestige of the old `sub`/`unsub` protocol survives in
`rust/`** - a repo-wide grep finds only `tracing-subscriber`, the two NATS
`client.subscribe` calls, email "unsubscribe" footer strings, and a migration
comment. `rust/web/public/` has no legacy JS. So the previous version's `sub`/`unsub`
would be **new work**, not a restoration.

#### Recommended design (record; WP-42's spec is NOT written this unit)

The socket is anonymous today but is **cheaply authenticatable**, so the user's
preferred shape - user-scoped events filtered server-side by identity, `sub`/`unsub`
only for public-game pages - is the right target. Answering the user's question
directly: **yes, user-specific events can ride on websocket authentication, but
identity alone is not sufficient today because the subject scheme carries no user
dimension.** Three pieces:

1. **Authenticate the upgrade.** Add `session: tower_sessions::Session` (and
   `State<PgPool>`, whose `FromRef<AppState>` impl already exists at
   `state.rs:24`) to `ws_handler`. Both are `FromRequestParts`, so extractor
   ordering is unconstrained. No layer or router reordering needed. Resolve
   identity **before** `ws.on_upgrade(...)`, not inside the closure: once the 101
   is returned the connection is hijacked and the session layer's response-side
   save pass has already run. Use `auth::session::get_user_from_session` +
   `validate_session_token` directly - **not** `get_current_user`
   (`auth/server.rs:503-528`), which is a `#[server]` fn depending on leptos
   `extract()` and an `expect_context::<PgPool>()` that only
   `leptos_routes_with_context` provides and which does not cover the plain `/ws`
   route.
2. **Do NOT reject anonymous upgrades.** Logged-out visitors legitimately need the
   public-game stream (`HomePage`'s `public_index`), and
   `websocket_hygiene.rs:71-81` asserts a 101 for an unauthenticated connect. The
   shape is **"authenticate if a session exists, degrade to a public-only stream if
   not"**, never a 401.
3. **Give the socket something to filter on.** Two viable options, both requiring
   more than the auth change:
   - **(i) Wildcard + per-socket membership filter.** Keep `game.>`/`proposal.>`,
     load the user's participating game/proposal ids from Postgres at connect, drop
     non-matching frames in the loop. Cheapest diff; the cost is invalidation when
     membership changes mid-connection (new game, new invite).
   - **(ii) Per-user fan-out subjects.** Publish additionally to
     `user.{user_id}.game.{game_id}` and subscribe only to `user.{uid}.>`. Cleaner
     at read time, but `broadcast_game_update(game_id)` currently takes only a game
     id and does no DB read, so every publisher must learn the recipient set - that
     is the bulk of the work. Note this **inverts** the assertions at
     `websocket.rs:200,228` that `user.>` stays empty.
   **Recommendation: (i) first**, because it is one handler's diff and does not
   touch eleven publish sites; revisit (ii) only if mid-connection invalidation
   proves awkward.
4. **`sub`/`unsub` is then needed only for public-game pages** - exactly the user's
   intuition. For a viewer who is not a participant, identity carries no
   information about what page they are on, so the client must say. Two shapes:
   a genuine client->server `sub {game_id}` / `unsub {game_id}` protocol (server
   starts reading inbound frames instead of discarding them at
   `websocket.rs:165-172`; client binds the `send` handle it currently drops), or
   the cheaper no-protocol option of a single always-subscribed "public games"
   subject, since public-game activity is public by definition. **The user's stated
   intent - "only send public-game events to a client that actually has that public
   game's page open" - requires the real `sub`/`unsub` protocol**; the
   public-subject shortcut does not satisfy it. Spec it as `sub`/`unsub`.
5. **Client-side**, `track_game_seq` and the `(Uuid, seq)` signals can stay as they
   are; the win is that `trigger.last_update` stops firing on irrelevant events,
   killing the refetch amplification.

**Scope split:** the auth + identity filtering (items 1-3, 5) is **WP-42**; the
visibility predicate it filters against is **WP-47**'s `is_game_visible_to_user`,
which must be the *same* predicate the HTTP endpoints use. The `sub`/`unsub`
protocol (item 4) is also WP-42 but is a separable second task - do not let it
block the auth work. Neither spec is written yet.

### D-14. Auth edges: squatting, enumeration, send caps, expiry [unblocks WP-35]
Context: four related auth-flow semantics calls: (i) unverified
add_email_address blocks the real owner's signup forever (squatting);
(ii) blocked-domain check leaks account existence; (iii) send-cap
accounting is cumulative-forever (over-counts); (iv) session tokens
never expire and there is no revoke-all.
Question: settle each:
- (i) A. Unverified claims expire (e.g. 24h) and a successful code
  confirmation by the true owner steals the claim. B. Status quo.
- (ii) A. Accept the differential response (it only reveals
  blocked-domain membership). B. Uniform handling. (Verification: the
  originally suggested uniform-reject would lock out existing verified
  users - if B, it must special-case them.)
- (iii) Windowed counter (the only sound fix per verification).
- (iv) A. Add expiry + GC + "log out everywhere". B. Document as
  intentional.
Recommendation: (i) A, (ii) A with a comment, (iii) windowed, (iv) A -
expiry/GC is cheap and revoke-all is table stakes after any incident.

**ANSWERED (2026-07-25): MODIFIED - DEVIATES FROM THE RECOMMENDATION on
(iv).** User's words, verbatim: "fail closed in production on auth
failure, expiring unverified email claims, windowed confirmation-send
cap, revoke-all-sessions. NO session expiry - the user explicitly does
not want sessions to expire. INSTEAD, changing an account email must
require email re-verification (step-up confirmation to the new address).
Note the interaction with D-1: since account-security commands leave the
email interface, the email-change flow lives in the web UI with a
confirmation link."
Settled per sub-item:
- (i) **A** - unverified `add_email_address` claims expire and the true
  owner's successful confirmation steals the claim.
- (ii) **A** - accept the differential blocked-domain response, with an
  explanatory comment.
- (iii) **windowed** confirmation-send counter.
- (iv) **NEITHER A NOR B as written.** Sessions must **NOT** expire and
  no session-expiry GC is to be added. `revoke-all-sessions` ("log out
  everywhere") **is** in scope. The compensating control for a
  compromised address is that **changing an account email requires
  re-verification** - a step-up confirmation sent to the new address.
  That flow lives in the web UI with a confirmation link (D-1 removed
  account-security commands from email).

**CONFIRMED (2026-07-26) - link-vs-code is a NON-GOAL. Keep the 6-digit
code.** The 2026-07-25 answer says "confirmation link", but the
email-change flow already exists in live code, is already compliant, and
uses a **6-digit code**. The specs correctly marked link-vs-code
cosmetic. WP-35 and WP-56 ship as specced; nothing changes.
**User's rationale:** it is "low value UI we need to maintain into the
future." An actual link is new UI work with no security gain; if it is
ever wanted it needs its own package.

### D-15. Reserved email verbs vs game move grammars [informs WP-59]
Context: game-scoped email dispatch reserves verbs (undo, concede, ...)
that could collide with a game whose move grammar uses the same word.
Options:
- A. Document the reservation as a platform constraint for game authors.
- B. Escape prefix (e.g. "move <text>" forces game interpretation).
Recommendation: A now (no current collision), B only when a real game
needs it.

**REOPENED 2026-07-25 (unit 4b Lead; the amendment itself was written by the
unit 4c Lead after finding the intended edit had never landed). The recorded
basis "no current collision" is FALSE.** Verified by reading live source:
- `end` is a live top-level game move in two shipped crates:
  `rust/game/acquire-1/src/command.rs:192-197`
  (`Doc::name_desc("end", "trigger the end of the game at the end of your
  turn", ...)`) and `rust/game/starship-catan-1/src/command.rs:309-313`
  (`Doc::name_desc("end", "end the flight early", Token::new("end"))`).
- The email dispatcher intercepts it first: `rust/web/src/email/commands.rs:1217`
  is `"end" => return run_end(ctx).await,`, which runs BEFORE the game path at
  `:1264`. (Added post-review-snapshot by issue #47.)
So an acquire-1 or starship-catan-1 player **cannot issue `end` by email today**.
This is a live functional defect, not merely a docs matter. A repo-wide grep over
`rust/game/` finds no other reserved-verb collision.
Consequence: **WP-59 Task 14** (the COMMANDS.md "Reserved verbs on the email
path" section) is fully specced but HARD-GATED - do not execute it until this
decision is re-made. Option A can no longer be adopted as written; it would have
to become "A-plus": document the reservation AND fix or exempt the two colliding
games.

**ANSWERED (2026-07-26): NEITHER A NOR B NOR "A-plus". The design is
REDESIGNED and now SETTLED.** Michael proposed the design; the
Orchestrator ruled on it.

**The ruling:**
- **Do NOT hardcode a reserved-verb list.**
- On **game-scoped** messages, **try the game command parser FIRST.**
- **Platform commands are the FALLBACK**, tried only when the game parser
  fails on that input.
- **One carve-out:** keep a **small hard-reserved set of escape-hatch
  verbs** (`help` and equivalents) that **always win**, even on the game
  path. Rationale: a game with a greedy parser must not be able to
  swallow the only command that unsticks a user. **Keep this set small
  and obvious.**

**This SUPERSEDES** the "A-plus" recommendation put to the user (keep the
reserved-verb list and disambiguate on the game-scoped path so a
declaring game wins there). There is **no reserved-verb list** in the new
design beyond the escape-hatch set. Do not implement a reservation table.

**Consequences:**
- The live defect is fixed: acquire-1 and starship-catan-1 players can
  issue `end` by email again, because the game parser is consulted before
  `"end" => run_end(ctx)` in `rust/web/src/email/commands.rs`.
- **WP-59 Task 14 is UNGATED**, but its content changes: the COMMANDS.md
  section must document **parser-first dispatch plus the escape-hatch
  set**, not a "Reserved verbs on the email path" reservation. Rewrite it
  rather than executing it as specced.
- `wfe F29` follows this outcome.

### D-16. Turnstile rendering after client-side nav [unblocks WP-55]
Context: the Turnstile widget likely never renders when /login is
reached by SPA navigation (script only scans on full page load).
Options:
- A. Explicit render() call from the login component effect.
- B. Force full-page load for /login links.
Recommendation: A - keeps SPA behaviour; B is a one-line fallback if A
misbehaves.

**ANSWERED (2026-07-26): Option B. OVERRULED in favour of the simpler
option.**
**This SUPERSEDES the recommendation above (option A, explicit `render()`
from the login component effect). Do NOT call Turnstile's `render()` from
an effect.** Make `/login` a **normal, unrouted link that forces a full
page load**, so Turnstile's automatic rendering just works.
**User's reasons:** complexity concern, and the login page should load
very fast.

**Mechanism VERIFIED 2026-07-26 by reading the vendored router source
(read-only). `rel="external"` works in the version actually in the tree:**
- `rust/web/Cargo.toml`: `leptos = "0.8.20"`, `leptos_router = "0.8.14"`
  (`Cargo.lock` resolves 0.8.14 exactly).
- `leptos_router-0.8.14/src/location/mod.rs` reads the DOM `rel`
  attribute, splits on space/tab, and returns early - letting the browser
  handle the click - if any token is `external` (or the anchor has
  `download`). So `rel="external"` and `rel="noopener external"` both opt
  out of client-side routing.
- **A plain `<a>` is NOT sufficient on its own.** Interception is a
  **window-level** click listener
  (`leptos_router-0.8.14/src/location/history.rs`) that walks
  `composed_path()` for any `HtmlAnchorElement` - it does not care whether
  the anchor came from `<A>` or a literal `<a>`. `rel="external"` is
  required either way.
- `<A>` has **no `rel` prop**. Use either `attr:rel="external"` spread
  onto `<A>` (attribute spreading on `<A>` is already proven in this
  codebase - `rust/web/src/app.rs` uses `attr:class` on the `/login`
  link), or a plain `<a href="/login" rel="external">`. The plain anchor
  is simplest; `<A>`'s only extra behaviour is `aria-current` active
  marking, irrelevant for a login link.
- Current `/login` links, both `<A>` and both client-side routed today:
  `rust/web/src/app.rs` (the `index-cta` "Start a game" link) and
  `rust/web/src/components/layout.rs` (the "Login" nav link).

**GAP - WP-55 must also close this; `rel` cannot cover it.** Three
navigations to `/login` go through `use_navigate`, which never touches an
anchor and is therefore never subject to the `rel` check:
`rust/web/src/components/layout.rs` (post-logout),
`rust/web/src/settings.rs` (anonymous redirect), and
`rust/web/src/admin.rs` (anonymous redirect). These need a hard
navigation (a location assignment) instead, or Turnstile will still fail
to render for users who arrive at `/login` by those paths.

## Group C - dependencies and build

> **STANDING PROCESS CHANGE (2026-07-26), from the D-17 answer, binding on
> this WHOLE GROUP - not just D-17.**
>
> Michael's strategy is to stay **as close to latest dependencies as
> possible so they never go stale.** Therefore, for this and **any similar
> dependency problem**, the **first** step is:
>
> **"Upgrade all dependencies to latest and see where we stand."**
>
> The problem may simply resolve. Only if it does **not** should the
> recorded workaround (vendoring, pinning, feature-juggling) be taken.
> Apply this ordering to WP-64, WP-65, WP-66, WP-67, WP-69, WP-70, WP-71,
> WP-72 and WP-73 alike: upgrade first, then re-assess whether the
> workaround is still needed, and record what the upgrade changed.

### D-17. sqlx 0.8/0.9 unification [unblocks WP-66]
Context: web is on sqlx 0.8 (pinned by tower-sessions-sqlx-store);
bot/operator on 0.9. Both stacks compile into the workspace; two
type-mapping behaviours against one DB.
Options:
- A. Wait for an sqlx-0.9-compatible tower-sessions-sqlx-store release.
- B. Vendor the (small) session store now and move everything to 0.9.
Recommendation: B if no compatible release exists at fix time - the
store is trivial; check crates.io first, A if it has shipped.

**ANSWERED (2026-07-26): ACCEPTED, but with an explicit FIRST STEP that
comes before either option.**
1. **Upgrade all dependencies to latest and see where we stand.** The
   sqlx 0.8/0.9 split may simply resolve.
2. Only if it does **not**, vendor the `tower-sessions-sqlx-store`
   (option B) and move everything to 0.9.
**This is a standing process change, not a one-off** - see the Group C
banner above. Michael's strategy is to stay as close to latest as
possible so dependencies never go stale. Unblocks WP-66.

### D-18. sentry feature trim [unblocks WP-67]
Context: sentry default features drag actix-web + ureq into every
server build; the native-tls transport choice is deliberate.
Question: confirm trimming to explicit features (backtrace, contexts,
panic, tracing/tower as used + native-tls transport), verified with
cargo tree?
Recommendation: Yes - mechanical once the feature list is confirmed
against actual usage; no product trade-off.

**ANSWERED (2026-07-26): Yes**, trim to explicit features (backtrace,
contexts, panic, tracing/tower as used, native-tls transport), verified
with `cargo tree`.
**STANDING CONSTRAINT - it is CRITICAL that no Sentry functionality is
lost.** The trim must be verified to **preserve current behaviour**, not
merely to shrink the dependency tree. Enumerate the sentry features
actually in use before removing any, and check the resulting build still
reports what it reports today. Preserve the deliberate native-tls
transport choice. Unblocks WP-67.

### D-19. [workspace.dependencies] migration [unblocks WP-64]
Context: no workspace dependency/package/lints tables; shared versions
copy-pasted across 40 manifests and already drifting. Touches every
manifest; natural umbrella for later bumps.
Question: proceed, and in what scope?
- A. Full: workspace.dependencies + workspace.package + workspace.lints
  in one migration PR, early in the backlog.
- B. Dependencies table only.
Recommendation: A - the marginal cost of package+lints inside the same
sweep is near zero and lints enforcement helps every later package.

**ANSWERED (2026-07-26): Option A.** All three tables -
`[workspace.dependencies]`, `[workspace.package]`, `[workspace.lints]` -
in one migration, early. Unblocks WP-64 and resolves the `dp F9`
version-pin row in the T3-B8 checklist. Sequence per the Group C banner:
upgrade everything to latest first, then migrate.

### D-20. 108 boilerplate game binaries [unblocks WP-73]
Context: 27 crates x 4 near-identical bins; binary-only deps (tokio
full, brdgme_cmd, fuzz) declared as lib deps. The reviewed
[dev-dependencies] fix is invalid (dev-deps do not apply to src/bin).
Options:
- A. brdgme_game_bins!(Game) macro in lib/cmd generating the 4 bins;
  deps stay per-crate but feature-trimmed.
- B. One generic parameterised bin crate (game selected by feature or
  a thin per-game bin crate depending on it); tokio/fuzz deps live once.
- C. Keep files; just trim tokio features 27 times.
Recommendation: B - it is the only option that actually removes the
files and centralises the heavy deps; the k8s images already build
per-crate so a per-game thin bin crate maps cleanly.

**ANSWERED (2026-07-26): Option B - a generic bin crate parameterised
over the `Gamer` trait, with thin per-game wrapper bin crates.
EXPLICITLY NOT option A (the macro).**
Michael approved B **partly because it avoids macros**. Do not
"simplify" it back into a macro.

**STANDING CONSTRAINT on macros, wider than this item:** Michael is wary
of custom macros because of their maintenance and cognitive cost. **Keep
any macro surface small and obvious, and PAUSE AND DISCUSS if a macro
starts getting really complex.**

**Concrete name, VERIFIED against the repo's layout (2026-07-26,
read-only):** `rust/lib/game_bin`, with `[package] name =
"brdgme_game_bin"`. The convention is snake_case directories under `lib/`
and `tools/` with package names `brdgme_<snake_dir>` - consistent across
all ten (`lib/cmd` -> `brdgme_cmd`, `lib/game_client` ->
`brdgme_game_client`, `tools/fuzz` -> `brdgme_fuzz`, ...). Hyphens are
the **game-crate** convention (`game/red7-1` -> `red7-1`), and
`brdgme-operator` is the single hyphenated outlier, not under `lib/`.
**Do NOT name it `game-bin` / `brdgme-game-bin`.**

**Structural note for WP-73:** today the 4 bins are `[[bin]]` targets
**inside each game crate** at
`src/bin/<snake_name>_{cli,fuzz,http,repl}.rs`, each a 3-10 line
`Gamer`-parameterised call (e.g. `http::serve::<Game>(addr)`). Moving to
thin per-game **bin crates** is therefore a structural change to the
workspace, not just a file move - factor that into the spec.

Unblocks WP-73. Sequence after WP-64.

### D-21. serde_yaml migration [unblocks WP-70]
Context: serde_yaml is archived; consumers are bot + lib/game_client
(must move together).
Options: A. serde_yaml_ng. B. serde-yml. C. saphyr. D. Switch the
surface to JSON.
Recommendation: A (serde_yaml_ng) - drop-in API, actively maintained;
D touches file formats users/ops may have.

**ANSWERED (2026-07-26): Option A - `serde_yaml_ng`.** Drop-in API,
maintained. Not JSON: that would change a file format ops and users may
depend on. bot and lib/game_client move together. Unblocks WP-70.

### D-22. warp -> axum in lib/cmd [unblocks WP-71]
Context: warp serves the game-service HTTP layer while the platform is
axum; two HTTP stacks in the tree. Touches all 28 game binaries' HTTP
surface (mechanically small handler).
Question: port now or defer?
Recommendation: Port - the handler is one endpoint; do it in the same
window as WP-06's http.rs fixes so the surface is touched once.

**ANSWERED (2026-07-26): Port now**, in the same window as WP-06's
`http.rs` fixes so the surface is touched once. It is one endpoint,
though it is the HTTP layer of all 28 game binaries. Unblocks WP-71.

### D-23. deny.toml hardening [unblocks WP-69]
Context: bans are warn-level (toothless); 4 stale advisory ignores for
crates absent from the lock.
Question: confirm flip multiple-versions to deny AFTER the dedup
packages (WP-66/67/68) land, with the residual duplicates enumerated in
skip/skip-tree, and clear the stale ignores now?
Recommendation: Yes, in that order.

**ANSWERED (2026-07-26): Yes, in exactly that order.** Clear the 4 stale
advisory ignores now; flip `multiple-versions` to deny only **after**
WP-66/67/68 land, with the residual duplicates enumerated in
skip/skip-tree. **Land WP-69 LAST among the dependency packages** so the
skip-list starts minimal. Unblocks WP-69.

### D-24. combine dependency posture [unblocks WP-72]
Context: combine 4.6 is dormant and sits at the heart of markup/game
parsing.
Options:
- A. Accept as recorded risk; note in deny.toml; migrate only when the
  parser is next rewritten.
- B. Migrate brdgme_markup to winnow / in-house now.
Recommendation: A - it works, has no advisory, and WP-02 already
touches markup enough for one release.

**ANSWERED (2026-07-26): Option A - accept as a recorded risk.** Note it
in `deny.toml`; migrate markup off `combine` only when the parser is next
rewritten. WP-02 already changes markup enough for one release, and
combine carries no advisory today. Unblocks WP-72.

### D-25. lib/cost consolidation [unblocks WP-17]
Context: lib/cost has one consumer (seven-wonders-1) while splendor-2
reimplements the same Go-origin cost logic locally; half-shared is the
worst state.
Options:
- A. Port splendor-2 onto lib/cost (add get/set + keep splendor's
  gold-joker can_afford as a crate-local extension).
- B. Fold lib/cost into seven-wonders-1 and delete the lib.
Recommendation: A - two consumers justify the lib and the API additions
are small; B throws away the shared abstraction the next economic game
will want.

**ANSWERED (2026-07-26): Option A - port splendor-2 onto `lib/cost`.**
Add generic `get`/`set`; keep splendor's gold-joker `can_afford` as a
crate-local extension.
**CONSTRAINT: the shared `lib/cost` must have a suitable amount of
automated testing as part of the port.** It gains a second consumer, so
it stops being incidentally covered by seven-wonders-1's tests; give it
its own.
**Scope reminder:** D-25 gates only **3 of WP-17's 8 findings**
(`b F31`, `ls F39`, `dp F27` - one indivisible consolidation). The other
5 (`b F30`, `b F32`, `b F34`, `b F35`, `ls F38`) were always
implementable. `checklists/T3-B3-splendor-libcost-holdem.md` holds the
authoritative row-by-row split. WP-17 is now fully unblocked.

## Group D - game port-parity vs official rules

> ## PARKED-PENDING-USER-RULES-REVIEW (2026-07-25)
>
> **D-35 and every per-game item D-26, D-27, D-28, D-29, D-30, D-31, D-32, D-34
> are PARKED.** No implementing agent may pick up a package gated on them.
>
> **The user's ruling, in two parts:**
> 1. **Policy (unchanged, still stands):** official rules are authoritative, and
>    docs may be corrected. But **NO gameplay change without per-game sign-off**
>    from the user.
> 2. **The whole question is parked pending the user's own review of the game
>    rules.** Two reasons, both from the user: (a) some `RULES.md` content was
>    **AI-generated and may simply be wrong**, so it cannot be trusted as the
>    baseline for adjudicating "code vs docs"; and (b) **edition and variation
>    choices are the user's to make** - which printing of Acquire, which Modern Art
>    payout, which No Thanks player count are product decisions, not review
>    findings.
>
> **What this means concretely:**
> - Do **not** change gameplay in any game crate under a parity finding.
> - Do **not** "correct" a `RULES.md` to match official rules under these items
>   either - the doc may be the thing that is wrong, and the user is reviewing it.
> - Packages gated on these items are `BLOCKED-ON-USER-RULES-REVIEW` in
>   `work-packages.md`. That status is stronger than `BLOCKED-ON-DECISION`: it does
>   not clear when a decision is answered, only when the user completes the rules
>   review and signs off per game.
> - The park is lifted **per game**, not globally.
>
> **NOT parked - two carve-outs, read them before assuming a game is stuck:**
> - **Mechanical liveness/correctness packages that were never parity-gated**
>   continue as normal: **WP-25** (modern-art-2 infinite busy-loop d F34 + round-4
>   soft-lock d F35 - its own note already says "does not wait on the D items in
>   WP-26"), **WP-15** (seven-wonders-1 b F1/F2/F3, including the reachable
>   permanent soft-lock in the DrawDiscard resolver), **WP-10** (pub_state
>   redaction - D-33 and D-35's redaction half are already ANSWERED option A and
>   WP-10 stays `READY`; its heading mentions D-35 but its scope is hidden-info
>   leakage, not rules parity), and the mechanical siblings WP-19, WP-22, WP-23,
>   WP-29.
> - **Egregious cases flagged for immediate fix** - see the block immediately
>   below. These are *not* edition questions and the user has been asked to rule on
>   them separately. They are **flagged, not specced, and not unparked** until the
>   user says so.
>
> ### RESOLVED 2026-07-26 - the five egregious candidates, ruled on individually
>
> **The park itself STAYS** (D-35 answer below). The user reviewed the five
> flagged candidates one by one; these five rulings are the **only** movement
> out of the park. Everything else in Group D remains
> `BLOCKED-ON-USER-RULES-REVIEW` and clears only on per-game sign-off.
>
> | Finding | Crate | RULING |
> |---|---|---|
> | **a F1** | `roll-through-the-ages-2` | **FIX NOW, outside the park.** As recommended. Cross-player state corruption (the previous player's `roll()` decrements the NEXT player's `remaining_rolls`) is in no edition. The fix must adjudicate the crate's own `test_game_keep_skulls_all_disaster_leadership`, which asserts the opposite for the `next`-command path. The **rest of WP-12 stays parked.** |
> | **b F4** | `seven-wonders-1` | **REMOVED from this list and PARKED** under the rules review. **The user's correction is binding: 7 Wonders resources are NOT depleted by trade** - they are printed on cards and both players use them, so there is **no competition for a resource** and the "asymmetric advantage by seat" framing recorded below was **WRONG**. **Residual narrower question, recorded so it is not lost, parked for the user's review and NOT scheduled:** because players resolve in seat order against live state, player p+1 can trade for a resource card player p **built on that same turn**, which p could not have done in reverse. That is a **simultaneity** question, not a scarcity one. |
> | **b F7** | `seven-wonders-1` | **FIX NOW, outside the park.** Ensure **only one of each physical board can be in play**. Every printing has 7 boards with one side chosen each; 14 independent boards are physically unreachable. The **rest of WP-16 stays parked.** |
> | **e F30** | `red7-1` | **CONDITIONAL - and the condition is SATISFIED, so FIX NOW.** The user's rule: fix the seat-order tie-break only if the correct behaviour is officially described or universally accepted; if resolving it needed a **subjective judgement on our part, park it**. **It is described.** `rust/game/red7-1/DATA_DOCS.md` states the second tie-break verbatim - "Ties within a rule are broken by the highest card in the winning set, **then by the highest card overall in the palette**" - and official Red7 rules agree (card value = number then colour, exactly what `Card::rank_key` already encodes). The code simply never implements it. Cause: `leader()` in `card.rs` only ever receives the **already-filtered winning sets** (`lib.rs` pushes `rule_fn(&self.palettes[p])`), so the full palette is unreachable; all-empty means every `len()` is 0 and every max is `(0,0)`, the strict `>` never fires, and seat 0 wins. Fix = fall through to the **full palette's** `rank_key()` max, which needs the unfiltered palette plumbed into `leader()`. **The D-29 half - "can an empty winning set win at all" - STAYS PARKED.** Verified read-only 2026-07-26. |
> | **d F37** | `modern-art-2` | **REJECTED - NOT A BUG. Do not "fix" this later.** The user: **this is the accepted way to play** - if only one artist has cards, 2nd and 3rd go to the artists **in order from the top**. `suits()` already returns the canonical top-to-bottom order (Lite Metal top, Krypto bottom), and `end_round` scans `suits()` in declared order with a strict `>`, so the first suit in that order wins among equal counts - which is the correct behaviour. **There is no value-board-order-vs-array-index discrepancy and no follow-up.** The Go-parity caveat below is moot: the behaviour is intended, not inherited by accident. |
>
> The original flagging table is retained below **for context only**. Where it
> disagrees with the rulings above - in particular its `b F4` "asymmetric by
> player index" reasoning and its `d F37` "not producible by any rulebook"
> claim - **the rulings above win.**
>
> ### Egregious candidates for immediate fix (SUPERSEDED - historical context)
>
> These were plain bugs, not "which edition" questions: they produce invalid,
> unreachable, or seat-order-dependent state that **no printing of the game
> specifies**. Listed for the user's decision only. **Now resolved - see the
> ruling table immediately above.**
>
> | Finding | Crate | Defect | Why it is a bug, not an edition choice | Currently owned by |
> |---|---|---|---|---|
> | **a F1** | `roll-through-the-ages-2` | `roll()` calls `keep_skulls()` then re-matches `self.phase`, which `keep_skulls` may already have advanced. A Leadership extra roll is silently skipped; worse, an all-skull reroll cascades into `next_turn()` so the previous player's `roll()` decrements the **next** player's `remaining_rolls`. | Corrupts another player's state across a turn boundary, and the crate's own test `test_game_keep_skulls_all_disaster_leadership` asserts the opposite outcome for the `next`-command path - identical states diverge on which command reached them. No edition says "sometimes the next player loses a reroll". | WP-12 |
> | **b F4** | `seven-wonders-1` | `execute_actions` resolves p0..pn sequentially against live-mutated `cards`/`coins`, so p+1 can trade for goods p built **this same turn**. | Asymmetric by player index - low-index players can never reciprocate. Every edition of 7 Wonders is simultaneous-action symmetric; an unfair-by-seat trade window is not a variant. | WP-16 |
> | **b F7** | `seven-wonders-1` | `cities()` lists all 14 A/B board entries; `start_game` shuffles and takes the first `players`, so "Rhodes A" and "Rhodes B" can both be in play. | Physically unreachable: there are 7 boards, one side chosen each, in every printing. No edition has 14 independent boards. | WP-16 |
> | **e F30** | `red7-1` | When **all** palettes have an empty winning set (reachable under Green/Violet), counts tie at 0 and `rank_key` maxes tie at `(0,0)`, so the strict `>` at `card.rs:311` leaves the **first non-eliminated player** as leader; they survive `done`, and the `discard` pre-check lets the **lowest-index** player discard into a rule nobody satisfies. | The "can you win with nothing" half genuinely IS the rules question (D-29) and stays parked. The half that is not: **the tie at zero is broken by seat order**. No edition breaks ties by player index. | WP-30 |
> | **d F37** | `modern-art-2` | `end_round`'s ranking loop initialises `highest_count = -1`, so artists with **zero cards on the table** are still awarded 2nd ($20) and 3rd ($10) when fewer than three artists had paintings played (common, e.g. 5-0-0-0-0). The values enter `value_board` and inflate every later round. | A price for an artist with no paintings played is not producible by any rulebook. **Caveat, stated honestly:** `modern_art.go:389-403` is identical, so this *is* a Go-parity item and a strict parity framing can claim it - which is exactly why it needs the user's word rather than an agent's. | WP-26 |
>
> **Explicitly NOT egregious - genuine edition/parity questions, leave parked:**
> D-28 splendor tie-break (most vs fewest cards); all of D-30's player caps; all
> three D-31 acquire items (the findings name later-Hasbro vs classic 3M/AH
> editions explicitly, and bag-exhaustion "differs by edition"); D-32 jaipur (the
> (i) premise is uncorroborated, (ii)/(iii) are adjudications); D-27 F5/F6/F8; and
> **all eight** WP-11 items (f F2, F14, F15, F21, F33, F43, F50, F54) - every one is
> annotated "Go quirk"/"port-parity" and none produces invalid or asymmetric state.

### D-35. Global port-parity policy (answer first) [informs D-26..D-34, unblocks WP-11] - ANSWERED then PARKED-PENDING-USER-RULES-REVIEW
Context: many crates faithfully reproduce their Go origin while
diverging from official rules; RULES.md sometimes documents the Go
behaviour, sometimes contradicts the code. Verification established one
precedent: where in-crate docs (RULES.md/PORTING_NOTES) document the
behaviour, the code is correct as documented.
Question: what is the default policy?
- A. Official rules win; Go quirks are bugs unless in-crate docs
  explicitly claim the deviation.
- B. Go parity wins (preserve historical game records/replays); RULES.md
  updated to document every deviation.
- C. Documented-in-crate wins (the verification precedent): where
  RULES.md documents the behaviour, keep it and only fix docs; where
  undocumented, official rules win.
Recommendation: C - it matches the precedent already applied, protects
deliberate choices, and gives a clean test for every item below. Items
D-26..D-34 below then only need per-game adjudication where the
deviation is undocumented or produces broken states.

**ANSWERED (2026-07-25): Option A - official rules win.** User's words:
"For port-parity conflicts, the OFFICIAL rules are authoritative -
correct both the code and RULES.md, noting the Go divergence in the
commit/doc."
This is **stronger than option A as written** and rejects option C's
"documented-in-crate wins" precedent: where a crate's RULES.md documents
a Go-derived deviation from the official rules, **both** the code and
RULES.md get corrected, and the commit message / doc notes the Go
divergence. It was answered in the context of D-33 / WP-10 port-parity
conflicts, but D-35 is by construction the *global* policy item, so it is
recorded as the global default for D-26..D-34.
**Scope caveat for the Orchestrator:** this flips the working assumption
of several already-triaged parity items whose recommendations were
written under "C". Those per-game items (D-26..D-32, D-34) are still
individually open and must be re-read under policy A before their
packages are specced. WP-11 remains blocked on D-30 (player-count caps).

**PARKED 2026-07-25 - the policy stands but nothing may act on it yet.** The user
added two constraints that supersede the sequencing above:
1. **No gameplay change without per-game sign-off** by the user. "Official rules
   win" is the tie-breaker the user will apply *when reviewing*, not a licence for
   an agent to change game behaviour.
2. **The whole parity question is parked pending the user's own review of the game
   rules**, because some `RULES.md` content was **AI-generated and may be wrong**
   (so it is not a trustworthy baseline for "code vs docs" adjudication), and
   because **edition/variation choices are the user's to make**.
Consequently the "docs may be corrected" half is **also** suspended for these
items: do not rewrite a `RULES.md` toward official rules under D-26..D-32/D-34
either, since the doc may be the thing that is wrong. **D-33 is unaffected** - its
`pub_state` redaction answer (option A) is independent of rules parity and WP-10
stays `READY`. See the Group D banner above for the full ruling, the two
not-parked carve-outs, and the egregious-fix candidates.

**ANSWERED (2026-07-26): KEEP THE PARK.** The user's answer to "when and
in what order do you do the rules review":
- **Keep the park.** Do **not** lift it globally.
- Do the review **per game**, prioritising **acquire-1**,
  **seven-wonders-1 / splendor-2**, **modern-art-2** and **red7-1** -
  those four unblock the most other work.
- `BLOCKED-ON-USER-RULES-REVIEW` remains **stronger** than
  `BLOCKED-ON-DECISION`. It does not clear when a decision is answered,
  only on the user's per-game sign-off.
**The only movement out of the park** is the five individually-ruled
egregious candidates in the Group D banner above: **`a F1` FIX NOW**,
**`b F7` FIX NOW**, **`e F30` FIX NOW** (its condition was verified
satisfied), **`b F4` PARKED** with the user's binding correction, and
**`d F37` REJECTED** as not a bug. Nothing else moves.

### D-26. Modern Art cluster [unblocks WP-26] - PARKED-PENDING-USER-RULES-REVIEW
Context: (i) round-4 end semantics underlie the critical hang + soft
lock (WP-25 fixes the liveness mechanically regardless); (ii) payout
pays cumulative value for ALL purchases incl. non-top-3 artists -
documented in RULES.md, may canonize a Go defect; (iii) zero-card
artists are ranked and awarded $20/$10 (undocumented, inflates (ii));
(iv) sealed/once-around bid ties go to the auctioneer.
Question: for each of (ii)-(iv): keep-and-document or fix to official?
Recommendation: under D-35-C: (ii) documented -> keep, but flag to you
explicitly because it materially changes scoring; (iii) undocumented ->
fix to official (zero-card artists unranked); (iv) undocumented -> fix
(ties to earliest bid/challenger per official rules).

### D-27. seven-wonders deviations [unblocks WP-16] - PARKED-PENDING-USER-RULES-REVIEW
Context: F4 same-turn trade of freshly built resources (asymmetric by
player index); F5 MimicGuild copies only Bonus guilds; F6 wonder-stage
sacrifice enters shared discard (contradicts own RULES.md); F7 both
sides of one wonder can be dealt (fix perturbs RNG draw ordering); F8
discard pile hidden from all (Halicarnassus takes blind).
Recommendation: F4 fix (snapshot tradable goods - it is asymmetric,
i.e. unfair, not just deviant); F5 extend to Science guilds (official);
F6 fix (contradicts own RULES.md = documented-wins says docs are the
contract); F7 fix distinct boards, accepting the RNG ordering change;
F8 add discard contents to PubState (official: discard is open
information; also needed for informed Halicarnassus play).

### D-28. splendor prestige tie-break [unblocks WP-16] - PARKED-PENDING-USER-RULES-REVIEW
Context: ties broken by MOST cards (Go parity, locked by a test);
official rules say FEWEST cards.
Options: A. Fix to official (update test). B. Keep documented Go parity.
Recommendation: A - undocumented in RULES.md, so D-35-C says official
wins; the test locks the bug in, not a decision.

### D-29. red7 empty-winning-set [unblocks WP-30] - PARKED-PENDING-USER-RULES-REVIEW
(the seat-order tie-break half of e F30 is flagged as an egregious-fix candidate
in the Group D banner above; the "can an empty set win" half is what is parked)
Context: a player with zero rule-fulfilling cards is treated as
winning; official rules say they cannot win. Adopting official needs a
defined outcome when ALL players have empty sets.
Options:
- A. Official: empty set cannot win; all-empty resolved by elimination
  order (last player standing by highest card per red7 tie rules).
- B. Document the deviation.
Recommendation: A - the current behaviour lets a player win with
nothing, which is strategy-breaking, and DATA_DOCS already contradicts
the code.

### D-30. Player-count caps vs official [unblocks WP-11, WP-20, WP-26 items] - PARKED-PENDING-USER-RULES-REVIEW
Context: texas-holdem 8 vs Go's 9; category-5 8 vs official 10;
lords-of-vegas 2-6 vs official 2-4; no-thanks 3-5 (2004 edition) vs
3-7 (later editions).
Question: per game, restore official/Go count or document the cap?
Recommendation: category-5 -> 10 (its own RULES.md already says 10, so
the code contradicts in-crate docs); texas-holdem -> document 8 (render
width constraint is plausible, no doc contradiction); lords-of-vegas ->
document 2-6 in RULES.md (extra capacity is a feature; but note the
WP-22 render fix must then cover 5-6p); no-thanks -> keep 3-5 and note
the edition in RULES.md.

### D-31. acquire edition behaviours [unblocks WP-20] - PARKED-PENDING-USER-RULES-REVIEW
Context: random start player (official: initial tile draw decides);
full-hand redraw permanently discards temporarily-unplayable tiles
(and mass redraw can drain the bag - compounds); bag-exhaustion ends
the game mid-turn. No Go port to match; edition-dependent.
Question: pick a reference edition and align all three?
Recommendation: Align to the current Hasbro/Avalon Hill rules: tile-draw
start, redraw only permanently-unplayable tiles, finish the turn on bag
exhaustion. Decide all three together (F14+F15 interact).

### D-32. jaipur adjudications [unblocks WP-26] - PARKED-PENDING-USER-RULES-REVIEW
Context: (i) next-round starter is not the round loser (original
finding's premise uncorroborated - needs the rulebook quote); (ii)
camel token counted as a bonus token in the end-of-round tie-break;
(iii) camel count hidden in render but exact in PubState.
Recommendation: (i) check the rulebook at spec time; official Jaipur:
the round LOSER deals but the WINNER... (verify - if confirmed
loser-starts, fix and restore major); (ii) official: camel token is not
a bonus token for the tie-break - fix; (iii) hide in both (counts-only
in PubState) for consistency with the renderer's intent.

### D-33. pub_state redaction design [unblocks WP-10]
Context: zombie-dice serializes the shuffled cup in draw order (next
draws readable - a NEW bug vs Go); for-sale leaks selling-phase secret
plays via PubState.bids. Needs one shape for all game crates.
Options:
- A. Counts-only / canonicalized public fields (cup as counts; bids
  only after reveal), private detail re-added per-player in
  player_state where the viewer is entitled.
- B. Per-player private field in a unified state envelope (bigger
  refactor).
Recommendation: A - minimal serde surface change, matches how other
crates already handle hidden info, and Go parity is irrelevant here
(zombie-dice's leak is new).

**ANSWERED (2026-07-25): Option A.** Public view data exposes
**counts/aggregates only**; per-player secrets move into `player_state`
where the viewer is entitled. Answered jointly with D-35; see D-35.

### D-34. rtta-2 fidelity policy [unblocks WP-12] - PARKED-PENDING-USER-RULES-REVIEW
(F1 is flagged as an egregious-fix candidate in the Group D banner above; F7/F9
and the quirk-preservation policy are what is parked)
Context: the crate deliberately preserves Go quirks (annotated), but F1
(phase re-match after keep_skulls skips/loses rolls) produces
objectively wrong state and diverges from the crate's own next-path
test; F7/F9 are smaller quirk-vs-fix calls.
Options:
- A. Fix F1 (declare the roll-path canonical, update the next-path
  test); keep other annotated quirks; F7/F9 fix (cheap, no replay
  impact).
- B. Strict Go fidelity: document F1 as a quirk too.
Recommendation: A - "wrong state reachable in normal play" is past the
line that the crate's own quirk-preservation policy drew.

## Group E - additional items surfaced during triage

### D-36. Deserialized-state trust strategy [unblocks WP-09]
Context: the requester layer (lib/cmd/src/requester/gamer.rs)
deserializes stored/forwarded Game state and player indices verbatim;
unchecked indexing panics exist across ~15 game crates (19 findings in
WP-09 alone; the two lost-cities player_state panics are
request-reachable today).
Options:
- A. Requester-boundary fix: bounds-check player index + a
  validate-after-deserialize hook (trait method with default no-op)
  games can implement; per-crate panics become defence-in-depth,
  fixed opportunistically.
- B. Per-crate defensive sweep only (.get() everywhere) - ~15 crates,
  no structural guarantee for future crates.
- C. Accept: state comes from our own DB; fix only the two
  request-reachable player_state panics.
Recommendation: A - one fix covers current and future crates; the
request-reachable pair gets fixed either way, and per-crate cleanups
can then ride each crate's own package.

**ANSWERED (2026-07-25): Option A.** Bounds-check the player index at the
requester boundary **plus** a per-game `validate` hook run after
deserialization. Per-crate unchecked-index cleanups become defence in
depth and ride each crate's own package.
Sequencing (from critical-path.md): land WP-09's boundary fix before the
bulk of Phase 3 per-crate work, and coordinate with WP-28.

### D-37. Markup literal-{ escape and unmatched-rest handling [unblocks WP-02]
Context: unmatched markup silently truncates output (parser succeeds
with tail in `rest`, callers discard it), and to_string emits raw text
with no escaping - no round-trip, markup injection through text. Both
need a convention for literal braces.
Options:
- A. Error on non-empty rest + define an escape ({{ or backslash) +
  escape on to_string.
- B. Error on non-empty rest only; document that text must not contain
  braces.
Recommendation: A with {{-style escaping (already visually consistent
with the {{tag}} syntax); B leaves the injection hole.

**ANSWERED (2026-07-25): Option A.** Error on a non-empty parse remainder
**AND** escape braces in `to_string`.
**User flag to carry into the WP-02 spec:** existing stored content which
currently renders partially may start **erroring**. The spec must include
a step to assess stored-content risk **by reading code and migrations
only - NOT by querying any database**.

**CORRECTED (2026-07-26): the escape is `{{lbrace}}`, NOT a bare `{{`.**
Option A still stands in full (error on a non-empty parse remainder AND
escape braces in `to_string`); only the escape token changes.
**This SUPERSEDES the recommendation's `{{`-style escaping. Do not
implement a bare `{{`.** The failure mode: with a bare `{{`, the parser
cannot distinguish an escaped literal brace from the start of a closing
tag like `{{/b}}`, so a nested `markup()` consumes its own terminator -
it cannot be implemented soundly. `{{lbrace}}` stays inside the
`{{...}}` family the decision asked for. `}` needs no escape.
WP-02 (`specs/WP-02-markup-robustness-dedup.md`) already pins
`{{lbrace}}`; that spec is correct as written.

### D-38. lib-game parser design items [unblocks WP-04]
Context: (i) OneOf furthest-error ranking is dead code (all offsets
provably 0) - implement offset propagation or delete the ranking; (ii)
typed-vs-spec expected() impls diverge (Doc and Many) - align or
document as deliberate; (iii) case folding differs (to_lowercase vs
UniCase) between suggest and parse; (iv) depth guard for deserialized
specs.
Recommendation: (i) implement offset propagation - better parse errors
are user-visible value and the plumbing is localized; (ii) align spec
impls to typed behaviour and extend the existing parity tests to cover
expected(); (iii) adopt UniCase in suggest; (iv) skip the depth guard
unless specs ever cross a trust boundary (they do not today).

**ANSWERED (2026-07-26): ACCEPTED as recommended, all four sub-items.**
- (i) **Implement OneOf offset propagation** (do not delete the ranking).
- (ii) **Align the spec impls to typed behaviour** and **extend the
  existing parity tests to cover `expected()`**.
- (iii) **Adopt UniCase in `suggest`.**
- (iv) **Skip the depth guard** - deserialized specs cross no trust
  boundary today.

**STANDING CONSTRAINT, binding on WP-04 GENERALLY and not just these four
items: keep the parser as straightforward and obvious as possible.** It
is complex but critical to the app and must stay **reliable and
maintainable**. At every choice point in WP-04, prefer the plainer
implementation over the cleverer one - including in the offset-propagation
plumbing, which is the item most likely to tempt an elegant abstraction.

Unblocks WP-04.

### D-39. Color parse API delete-vs-keep [unblocks WP-05]
Context: regex + lazy_static exist solely for from_hex/from_str which
have no runtime caller; three divergent color-name alias tables exist
across color and markup.
Options:
- A. Delete the dead parse API (drops regex+lazy_static workspace-wide;
  resolves the alias-table divergence by removing two tables).
- B. Keep the API, reimplement hex parsing in std, unify the tables.
Recommendation: A - dead public API in an internal lib; resurrect from
git if ever needed.

**ANSWERED (2026-07-26): Option A - delete the dead parse API**
(`from_hex` / `from_str`). This drops `regex` and `lazy_static`
workspace-wide and resolves the three-way alias-table divergence by
deletion. Git can resurrect it if it is ever wanted. Unblocks WP-05.

### D-40. Write-only stats subsystems keep-or-drop [unblocks WP-20, WP-30 items]
Context: acquire-1 tracks per-game stats but to_brdgme_stats has zero
callers; lost-cities-1/-2 Stats fields are never written or write-only
(and one increment counts the wrong thing). Either wire stats into
status()/the platform stats pipeline or delete the machinery.
Options:
- A. Wire them up (needs a defined platform consumption path).
- B. Delete the dead machinery in all three crates; re-add when the
  platform grows a per-game-stats feature.
Recommendation: B - no consumer exists today; dead-but-buggy tracking
code is pure liability. Note the platform-level "game stats" feature as
a future product idea instead.

**ANSWERED (2026-07-26): Option B - delete the dead machinery** in
acquire-1 (`to_brdgme_stats`, finding `c F12`) and lost-cities-1/-2
(`e F39`, `e F40`), **and split these items out of WP-20/WP-30 into their
own package** so they can land **ahead of the rules review**. They are
stats questions, not rules questions.
**For the record:** Michael wants to revisit **"game specific stats" in
future from a CLEAN SLATE.** That is precisely why deleting the dead
machinery now is right - there is no platform path to wire it into, and
the future feature will not want to inherit this shape.
**The split-out package is `WP-81` in `work-packages.md`.** WP-20 and
WP-30 lose their D-40 blocker and their stats items; both remain
`BLOCKED-ON-USER-RULES-REVIEW` for their rules halves.

### D-41. Friends-page select revert after a rejected change [informs WP-54, WP-53]
Context: a rejected invite-policy or game-visibility change on `/friends`
keeps displaying the unsaved value until a page reload. The WP-54 review
proved the draft's assumed fix (bump a refresh signal so the refetched
overview re-syncs the control) CANNOT work: a rejected mutation returns
identical data, `AttributeValue for bool::rebuild` skips equal values,
`AnyView::rebuild` rebuilds in place on a matching TypeId so the `<select>`
is never re-created, and `<option selected>` will not reassign a
user-dirtied option. A real fix means converting both `<select>`s from
per-`<option>` `selected=` to a `prop:value`-over-signal binding driven by
an `Effect` (the shape `docs/CODING.md:305-310` already prescribes).
Options:
- A. Absorb the conversion into WP-54 Task 2 - bigger markup diff in a file
  WP-54 otherwise only reads, but the user-visible behaviour is correct.
- B. Ship WP-54 with the error message only; file the conversion against
  the `friends.rs` owner (WP-53 already touches the file).
Recommendation: B - the error message alone removes the silent failure
(the user at least learns the change was rejected), and the binding
conversion belongs with the other `friends.rs` component work.
**Lead ruling applied so WP-54 is not blocked: B.** WP-54 ships the error
message and records the residual desync as EXPECTED in its manual checklist
(step 7) so it is not later mistaken for a regression, plus cross-package
item #7 routing the conversion onward. Overriding this to A only requires
editing WP-54 Task 2; nothing else depends on the choice.
