# DECISIONS - consolidated decision record

Date: 2026-07-26

## SUPERSEDES

This file replaces the following four planning documents:

- `decisions-needed.md`
- `open-decisions-for-user.md`
- `decisions-ANSWERED.md`
- `decisions-session3.md`

Those four files REMAIN ON DISK pending a later, separate destructive pass. They
are retained only as history. **Where this file and any of them disagree,
`DECISIONS.md` is authoritative.**

Precedence used in assembling this record, later ruling wins:
`decisions-session3.md` > `decisions-ANSWERED.md` > `decisions-needed.md`.

Assembled from two normalized extractions of those sources
(`planning/.merge-part-needed.md`, `planning/.merge-part-rest.md`), not by
re-reading the originals. No claim in this file is asserted about code that the
sources did not already record as verified. Any line number appearing below is
reproduced from a source and is **approximate, verify**.

## Coverage

- **55 source-numbered decisions (D-1..D-55), one of them renumbered on merge,
  giving 56 sections: D-01 .. D-56, no gaps and no duplicates.**
- The extra number exists because `D-41` is a genuine **ID collision**: two
  unrelated decisions were independently given that number. Per Lead ruling,
  `## D-41` keeps the `decisions-session3.md` decision (delete the per-game
  `_fuzz`/`_repl` binaries, the later and ruled one) and the
  `decisions-needed.md` friends-page-select-revert decision is renumbered to
  `## D-56`. Nothing was merged and nothing was dropped.
- The sources use the UNPADDED form (`D-1`, `D-8`, `D-41`). Headings here are
  zero-padded (`## D-01`) purely for stable sorting. Both forms refer to the same
  item.
- **D-35..D-40 were searched for and FOUND - there is no numbering gap.** All six
  exist in `decisions-needed.md`; D-35, D-37, D-38, D-39 and D-40 additionally
  exist in `decisions-ANSWERED.md`. The apparent "D-34 -> D-41 jump" was an
  artefact of `decisions-ANSWERED.md` being incomplete (its own banner wrongly
  claims it covers `D-01..D-34` only) and of `decisions-session3.md` starting at
  D-41. Do not re-open this as a gap.
- `open-decisions-for-user.md` contributes **zero** decisions: it is a 6-line
  pointer stub redirecting to `decisions-ANSWERED.md`. Its full text is preserved
  in the UNKNOWN/notes section at the end.
- Non-numbered content preserved below: standing constraints and process changes,
  the finding-level rulings (`a F1`, `b F4`, `b F7`, `e F30`, `d F37`,
  `bo F25`), and `N-1`..`N-6`.

## Conflicts resolved

Every ID that appeared in more than one source. In all rows below the surviving
ruling is the later one under the stated precedence; where both sources agreed on
substance, the difference was breadth of recorded context, and BOTH bodies of
context are merged into the section rather than dropped.

| ID | superseded version held by | what differed |
|---|---|---|
| D-07 | `decisions-needed.md` | needed-file records the original option-A recommendation (`--redact-private`, default on, user-facing); ANSWERED records it OVERRULED. Merged: OVERRULED wins; needed-file's riders (`wd F7` becomes "make the export admin-only", import nits unchanged) retained. |
| D-08 | `decisions-needed.md` | needed-file holds the option-C ruling plus the D-5 reconciliation; ANSWERED holds the REFINED restart resolution (deprecated bot -> latest non-deprecated). Both retained; REFINED wins on the restart path. |
| D-09 | `decisions-needed.md` | Same ruling (option B) in both; no material conflict. |
| D-10 | `decisions-needed.md` | Same ruling (option A plus two visible links) in both; no material conflict. |
| D-11 | `decisions-needed.md` | Same ruling (option A) in both; no material conflict. |
| D-14 | `decisions-needed.md` | needed-file scopes D-14 across all four sub-items (squatting, enumeration, send caps, expiry); ANSWERED records only the narrow code-vs-link confirmation. Merged: broad ruling retained, link-vs-code non-goal confirmed. |
| D-15 | `decisions-needed.md`, then `decisions-ANSWERED.md` | needed-file + ANSWERED both record the REDESIGN (parser-first, escape-hatch set) and both state the live `end` defect "is fixed". `decisions-session3.md` D-54/D-55 SUPERSEDE that consequence: Task 14 is carved into WP-85, WP-85 is DEFERRED - BLOCKED ON MICHAEL, and the `end` defect stays OPEN. Session 3 wins. |
| D-16 | `decisions-needed.md` | Same ruling (option B, OVERRULED); needed-file carries the router verification inline, ANSWERED as a separate note. Both merged. |
| D-17 | `decisions-needed.md` | Same ruling plus the same standing process change; no material conflict. |
| D-18 | `decisions-needed.md` | Same ruling; no material conflict. |
| D-19 | `decisions-needed.md` | Same ruling (option A); no material conflict. |
| D-20 | `decisions-needed.md` | Same ruling (option B, not the macro) and same verified naming. `decisions-session3.md` amends downstream scope: D-41/D-43 change the entry-point count, D-42 extends WP-73 to `lords-of-vegas-1`, and session 3 records the terminology correction that there is NO macro. Session 3 wins on those points. |
| D-21..D-25 | `decisions-needed.md` | Same rulings in both files; no material conflict. |
| D-35 | `decisions-needed.md` | Both record: policy A (official rules win), then PARKED, then park CONFIRMED per game. No material conflict. |
| D-37 | `decisions-needed.md` | needed-file carries the fuller question (unmatched-`rest` half) and the CORRECTION; ANSWERED carries the `{{lbrace}}` correction plus the no-database constraint. Merged; `{{lbrace}}` wins over a bare `{{`. |
| D-38, D-39, D-40 | `decisions-needed.md` | Same rulings in both; no material conflict. |
| **D-41 / D-56** | **neither - genuine ID COLLISION, not a supersession** | Two UNRELATED decisions were given the number D-41. `decisions-session3.md` D-41 = delete the per-game `_fuzz`/`_repl` binaries. `decisions-needed.md` D-41 = Leptos `/friends` `<select>` desync after a rejected change (a late Group E addition). Precedence cannot resolve a collision. **Lead ruling applied:** `## D-41` keeps the session-3 decision; the friends-page decision is RENUMBERED to `## D-56` and carries a renumbering note. Neither is merged into the other; neither is dropped. |
| D-13 vs D-44 | `decisions-needed.md` (D-13) | No ID collision, but D-44 (COMMIT to SSE, migrate now) makes D-13's WebSocket-hardening design HISTORICAL. D-13's recommended `/ws` design must not be read as current work. Flagged in both sections. |
| `bo F25`, `a F1`, `b F4`, `b F7`, `e F30`, `d F37` | `decisions-needed.md` (as summaries inside D-26/D-27/D-29/D-34 and its egregious-rulings table) | Same rulings; `decisions-ANSWERED.md` holds the canonical per-finding form, reproduced in the Finding-level rulings section. Where the needed-file's historical "egregious candidates" table disagrees (`b F4` "asymmetric by player index"; `d F37` "not producible by any rulebook"), THE RULINGS WIN. |
| `N-1`..`N-6` | `decisions-needed.md` (named only as a group) | Only `decisions-ANSWERED.md` records the individual rulings; reproduced in full in the N-items section. |

---

# Decisions

## D-01 Email From-header authentication redesign

**Status:** ANSWERED (2026-07-25), then REFINED (2026-07-25, later session). Unblocks WP-56.

**Decision:** The inbound-email settings route is authenticated solely by the spoofable From header, and email-management commands (add/confirm/remove addresses) run over that path - together a full account takeover (both remaining criticals). Unrouted `None` addresses also fall through to the settings handler. How should inbound email be authenticated, and do account-security commands belong on the email path at all? Options: A. per-user secret settings token in the reply address (like the existing per-game `email_token`), drop the `None` fallthrough, require Resend SPF/DKIM pass verdicts, keep email settings commands. B. Option A's token+SPF/DKIM but remove account-security commands (add/remove email) from the email path entirely. C. Kill the email settings route; all settings via web only. Recommendation: B.

**Ruling:** Option B. Michael's words: "Require the s- token for the settings route, verify SPF/DKIM on inbound, AND remove account-security commands (add/confirm/activate email address) from the email interface entirely." Consequences for WP-56: (a) the settings reply address must carry a per-user secret `s-` token, (b) inbound SPF/DKIM verdicts from Resend must be consulted and a fail must reject, (c) the unrouted-`None` fallthrough to the settings handler is removed, (d) `emails add`/`confirm`/`active` are deleted from the email command surface. Interacts with D-12 + D-14: the email-change flow therefore lives in the web UI with a confirmation link.

REFINEMENT (2026-07-25) - scope of (d) NARROWED, cold start RESOLVED. The user does NOT want all settings commands off email. Only these four verbs leave email: `emails add <addr>`, `emails confirm <code>`, `emails active <addr>` / `emails use <addr>`, and `emails remove <addr>` - "confirmed yes", removed too; the WP-56 spec's earlier "one-arm revert if the Lead disagrees" caveat is WITHDRAWN. KEPT on the email interface (the user's ruling: not sensitive): `name` (username / display name), `theme <name>`, `colors`/`colours`, bare `emails` (listing), `emails on`/`off`, `emails invite on|off`, `emails reminder on|off`, and `settings` (summary). Notification preferences, username and theme by email are a product feature and stay. Consequences (a)-(c) are UNCHANGED - they are now the controls protecting the retained commands. Cold start RESOLVED: settings are managed in the web UI, and the tokenised inbound settings address (`s-{token}@brdg.me`) is surfaced THERE as an opt-in reveal on the settings page. The token is NOT to appear in email footers (turn emails, proposal emails, `List-Unsubscribe`, or anywhere else) - a bearer secret in a footer leaks with every forwarded message. Building the web reveal belongs to the `settings.rs` owner, not WP-56; WP-56 must simply not invent a fallback discovery path.

**Rationale:** Tokenised auth fixes the spoofing class, but account-security mutations have web UI equivalents and are not worth the attack surface on a forgeable channel. Low product cost, removes both criticals structurally.

**Sources:** `decisions-needed.md`.

## D-02 Sweep/webhook delivery semantics

**Status:** ANSWERED (2026-07-25). Unblocks WP-46, WP-57.

**Decision:** The inbound webhook inserts its dedupe marker before processing (post-marker failures permanently dropped, always 200); every outbound sweep marks before doing; `FOR UPDATE SKIP LOCKED` runs under autocommit `fetch_all` and is a no-op, so concurrent replicas can double-send. At-least-once or at-most-once delivery, and sync or enqueued webhook processing (svix timeout is 15s)? Options: A. at-least-once - marker/mark after success, 5xx on transient failure so svix retries, claim-then-send (real transaction) in sweeps; occasional duplicate email possible. B. keep at-most-once, fix claim atomicity only. C. A plus enqueue: webhook verifies+dedupes+persists then 200s, processing in a worker. Recommendation: A now, C only if webhook processing time actually approaches the svix timeout.

**Ruling:** Option A. At-least-once: write the dedupe marker only AFTER successful processing, and return 5xx so the provider retries. The same shape applies to the turn-reminder sweep - do not mark `sent` on skip paths. Processing idempotency rests on the marker. No enqueue/worker split (option C) for now. Unblocks WP-57 fully; WP-46 remains blocked on D-11 only.

**Rationale:** Dropped turn commands are worse than a rare duplicate notification.

**Sources:** `decisions-needed.md`.

## D-03 Undo-vs-ratings semantics

**Status:** ANSWERED (2026-07-25). Unblocks WP-40.

**Decision:** `undo_game` on a finished game reverts state but never rewinds `rating_change`, and the idempotency guard then blocks re-rating the real outcome - permanent rating corruption (the web-domain critical). Same on the email undo path. May a finished game be undone at all; if yes, how do ratings recover? Options: A. forbid undo once `is_finished` (guard in `db::undo_game`). B. allow undo of finished games, atomically rewind ratings using the stored per-player `rating_change` deltas, clear `rating_change` so re-finish re-rates (recompute-only is known-unsound: double-counts). C. allow within a short grace window (e.g. 5 min) with B's rewind. Recommendation: A.

**Ruling:** Option A. Forbid undo once a game is finished, making the ratings corruption UNREACHABLE. Do NOT attempt any rating rewind - no delta-reversal code, no recompute. The missing rating rewind is explicitly out of scope for WP-40.

**Rationale:** It is the only option with no rating-math risk, and undo-after-finish is a rare edge; revisit B later if users ask.

**Sources:** `decisions-needed.md`.

## D-04 concede/undo TOCTOU + path unification

**Status:** ANSWERED (2026-07-25). Informs WP-40, no separate block.

**Decision:** `db::undo_game` and `db::concede_game` skip the optimistic-locking (`updated_at` guard) discipline the move path has; the email path duplicates the server-fn logic near-verbatim and has the same races. Confirm the intended shape: fix once in `db.rs` with guards, and extract shared `concede_core`/`undo_core` used by both web and email paths? A. yes. B. guards in `db.rs` only; leave the duplication. Recommendation: A.

**Ruling:** Option A (yes). Share `undo_core` / `concede_core` between the web and email paths "so the missing concurrency guards are fixed once", with the `is_finished` / `updated_at` guards living in `db.rs`.

**Rationale:** The duplication has already drifted once; this is the cheap moment to unify.

**Sources:** `decisions-needed.md`.

## D-05 Bot-turn wedge recovery + NATS delivery

**Status:** ANSWERED (2026-07-25) - C-lite MODIFIED, DEVIATES FROM THE RECOMMENDATION. Unblocks WP-38.

**Decision:** Every bot-turn failure mode wedges a game permanently and silently: UserError acked without re-publish, retry exhaustion, publish lost after DB commit, bot rename/delete/disable causes skip-and-ack (games reference bots by NAME). No ack-deadline heartbeat for long turns. What is the recovery architecture? A. reconciliation sweep (periodic "bot on turn for > N minutes -> re-publish `bot.turn`"). B. per-error handling: re-publish on UserError, DLQ + alert on retry exhaustion, transactional outbox for publishes. C. A + B's DLQ/alerting. Sub-questions: (i) reference bots by id via migration, or keep name + warn-on-rename? (ii) `AckKind::Progress` heartbeat or raise `ack_wait`? Recommendation: C-lite - sweep + retry-exhaustion alert; bots by id (migration); Progress heartbeat.

**Ruling:** C-lite, MODIFIED. Michael's words, verbatim as recorded: "reconciliation sweep + retry-exhaustion alert + Progress heartbeat on long turns, BUT bots stay referenced BY NAME (the user wants to swap bots by name; do NOT convert to bot ids). Dangling bot player names are an explicitly SUPPORTED state: they no-op rather than wedge the game, and the admin page shows a warning listing dangling bot player names. Disabling all bots must remain a valid intentional configuration." Consequences for WP-38: no bot-id migration, games keep referencing bots by name and renaming/swapping by name is a supported product capability; a bot player name that resolves to nothing (deleted, renamed away, or disabled) must no-op - the game does not wedge, the message is acked, the condition is surfaced not retried forever; the admin page gains a warning listing dangling bot player names; "all bots disabled" is a valid intentional configuration and must not trip alerts or blocking validation; sub-question (ii) `AckKind::Progress` heartbeat on long turns (not an `ack_wait` raise). See D-08 for creation-time-vs-later reconciliation.

**Rationale:** As recorded in Michael's ruling above - swapping bots by name is a product capability he wants to keep, which is why the bot-id migration was rejected.

**Sources:** `decisions-needed.md`.

## D-06 game_visibility scope

**Status:** ANSWERED (2026-07-25), jointly with D-13. Unblocks WP-47, WP-49.

**Decision:** The `game_visibility` setting exists and `is_game_visible_to_user` is implemented, but no read endpoint uses it: any authed user can read any game's details; anonymous stats endpoints name private users. The rules/game-info pages also have an auth-posture wrinkle (anonymous game-info page links to an auth-gated rules endpoint). Which endpoints does `game_visibility` gate, and do stats anonymize or filter private users? A. gate game details + history/feeds; stats anonymize private users ("Anonymous" label, aggregates kept). B. gate details/feeds only; stats unchanged. C. gate everything including stats rows. Also: make rules pages public? Recommendation: A, and yes - rules/game-info fully public.

**Ruling:** Option A. Gate game details and activity feeds on participation-or-public. Stats compute GLOBALLY but ANONYMIZE private users - do NOT exclude them from aggregates. Rules pages stay public. Answered jointly with D-13.

**Rationale:** Anonymize keeps stats useful while honouring the setting; filtering breaks head-to-head math.

**Sources:** `decisions-needed.md`.

## D-07 Export bundle privacy

**Status:** ANSWERED (2026-07-26) - OVERRULED, neither option as written. Unblocks WP-48.

**Decision:** The export bundle includes private log bodies and hidden state; its stated purpose is paste-into-issues debugging. Accept that, or add redaction? A. add `--redact-private` (default ON for the user-facing path; full bundle behind an explicit flag/admin). B. accept and document. Recommendation: A with default-on redaction.

**Ruling:** OVERRULED - do not build a redacted user-facing export at all. This SUPERSEDES the recommendation (option A, `--redact-private` default ON for a user-facing path); do not implement it. The only export path is the full bundle, admin-only. The `--redact-private` flag is out of scope. The user-facing export path is out of scope. Bug reporting is by game ID, not by pasted bundle. WP-48's scope shrinks accordingly; `wd F7`'s privacy work becomes "make the export admin-only"; the remaining import nits are unchanged mechanical riders.

**Rationale:** Michael explicitly accepts the risk that game state may change after a report is filed and render that report useless.

**Sources:** `decisions-needed.md` (fuller question text and the `wd F7` rider), `decisions-ANSWERED.md` (the OVERRULED ruling, listed in its "five rulings changed" table).

## D-08 Bot-slot validation choke point

**Status:** ANSWERED (2026-07-25), REFINED (2026-07-26). Unblocks WP-45.

**Decision:** Client-supplied bot slots are unvalidated at 4 entry points (`create_proposal`, `add_proposal_player`, `restart_core`, email `new`); a bogus/disabled bot name creates a game that wedges unrecoverably (compounds D-05). Where does the single validation live? A. one shared fn (validate against enabled bots) called at all 4 entry points. B. validate at game-start time only (`start_proposal_tx` / `create_game_from_service`) - single true choke point but late feedback. C. A + B (defence in depth). Recommendation: C.

**Ruling:** Option C - validate at all 4 entry points AND at game start. Reconciliation with D-05 (state explicitly in both WP-45 and WP-38 specs): validation applies to names at creation/start time, so the user gets immediate feedback for a typo or a bot that does not exist right now. A name that goes missing or is disabled LATER must NOT wedge the game and must NOT cause a rejection at turn time - it falls into D-05's dangling-name no-op path plus the admin warning. "Validate on write, tolerate on read."

REFINED (2026-07-26) - the restart path gets an ACTIVE resolution, not a carve-out. Option C and "validate on write, tolerate on read" still stand. On restart, resolve a deprecated bot to the LATEST NON-DEPRECATED version of that bot, and start the game with that. This SUPERSEDES the proposal put to the user (merely exempt the restart path from write-validation and let it fall into D-05's dangling-name no-op plus admin warning) - that no-op fallback is NOT the answer for restart; do not implement it there. The no-op-plus-admin-warning path remains correct for the other case D-08 describes: a bot name that goes missing or is disabled AFTER a game has started must not wedge the game and must not be rejected at turn time. Reflect in `specs/WP-45-bot-slot-validation.md` and keep WP-38's D-05 text consistent.

**Rationale:** D-08's core answer is unchanged - validate on write, tolerate on read. The restart path now actively re-resolves rather than rejecting or no-opping.

**Sources:** `decisions-needed.md` (option C ruling + D-05 reconciliation), `decisions-ANSWERED.md` (the REFINED restart resolution, in its "five rulings changed" table).

## D-09 Email canonicalization policy

**Status:** ANSWERED (2026-07-26), option B. Unblocks WP-50.

**Decision:** Emails are stored/compared untrimmed and case-sensitive across auth, invites, new-game and settings; duplicate accounts and invite-policy bypass are possible. A storage-normalization change touches the unique constraint and existing rows. Normalize where? A. trim + lowercase at all input boundaries only (no migration). B. A + one-off migration lowercasing stored rows + citext/lower-index unique constraint. Recommendation: B.

**Ruling:** Option B, as recommended. Trim + lowercase at all four input boundaries, PLUS the one-off migration lowercasing stored rows, PLUS the lower-index (or citext) unique constraint. Surface the case-collision risk once, deliberately, during the migration.

**Rationale:** Boundary-only leaves existing mixed-case rows permanently un-matchable against new normalized input; the migration is small and the collision risk (two accounts differing only by case) is worth surfacing once, deliberately.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-10 Unsubscribe RFC 8058 compliance

**Status:** ANSWERED (2026-07-26) - option A WITH AN ADDITION (extended). Unblocks WP-58.

**Decision:** `List-Unsubscribe-Post` is advertised but the mailto path can never honor one-click semantics; Gmail/Yahoo bulk-sender rules expect a working HTTPS one-click endpoint. The standalone email dispatch also rejects subscribe/unsubscribe verbs its own help text advertises. A. build the HTTPS one-click endpoint (tokenised, no auth redirect). B. mailto-only; drop the Post header; fix the help text. Recommendation: A.

**Ruling:** Option A, WITH AN ADDITION. Build the HTTPS one-click endpoint (tokenised, no auth redirect) as recommended, and additionally the mail must carry TWO visible links: (1) a type-specific unsubscribe link matching the email type actually received - e.g. "Unsubscribe from game reminders" on a reminder mail; (2) a "Manage my subscriptions" link to the user settings page. The `List-Unsubscribe` / `List-Unsubscribe-Post` headers STILL point at the one-click endpoint; the visible links are ADDITIONAL, not a replacement for the headers. Also fix the help text that advertises subscribe/unsubscribe verbs the standalone dispatch rejects.

**Rationale:** Deliverability to Gmail/Yahoo is a real product concern for a turn-notification product.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-11 Reminder preference semantics

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-46.

**Decision:** The reminder sweep gates on `turn_emails_enabled`, but a separate `reminder_emails_enabled` flag exists. Which flag governs reminders? A. `reminder_emails_enabled` governs reminders; `turn_emails_enabled` governs turn notifications only. B. reminders require BOTH.  Recommendation: A.

**Ruling:** Option A. `reminder_emails_enabled` alone governs reminder emails; `turn_emails_enabled` governs turn notifications only. The reminder sweep must NOT consult `turn_emails_enabled`. Unblocks WP-46 - its last remaining blocker (D-02 already answered: at-least-once, do not mark `sent` on skip paths).

**Rationale:** The user's rationale, which is the design intent to preserve: some users play mainly by web and do not want turn emails, but reminders are still useful to them if they have MISSED or FORGOTTEN a game. Option B would make the reminder flag dead for exactly those users.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-12 Fail-open posture: Turnstile + encryption key

**Status:** ANSWERED (2026-07-25), jointly with D-14 (modification recorded under D-14). Unblocks WP-35.

**Decision:** Turnstile verifier errors fail open and an unset secret silently disables it; an unset `DATABASE_ENCRYPTION_KEY` silently uses a hardcoded public fallback key (one warn line). A. fail closed in prod: refuse startup on missing key; Turnstile errors reject (or challenge) instead of pass; explicit `DEV_MODE` opt-in re-enables lenient behaviour locally. B. keep fail-open, add loud alerting. Recommendation: A.

**Ruling:** Option A - fail closed in production on auth failure; refuse startup on a missing `DATABASE_ENCRYPTION_KEY`; Turnstile verifier errors reject rather than pass. Answered jointly with D-14; see D-14 for the modification (NO session expiry).

**Rationale:** A silently-disabled encryption key is a data breach in waiting; dev ergonomics are preserved by the explicit opt-in.

**Sources:** `decisions-needed.md`.

## D-13 /ws unauthenticated site-wide firehose

**Status:** ANSWERED (2026-07-25) jointly with D-06 as "Option A"; FLAG RESOLVED 2026-07-25 - now ANSWERED in the **option B shape**. Unblocked WP-42.

> **SUPERSEDED IN DIRECTION BY D-44.** D-44 (planning session 3) COMMITS to migrating the live-update transport to **SSE now**, ahead of WP-42's WebSocket hardening. Everything below about `/ws` authentication, per-socket filtering and a `sub`/`unsub` protocol is therefore **historical design context, not current work**. The privacy requirement it encodes - live-update feeds must be gated on the same `is_game_visible_to_user` predicate the HTTP endpoints use - carries forward to the SSE design (see D-44, D-46, D-48, D-49, D-50). Do not execute the WebSocket design below.

**Decision:** Any connection to `/ws` receives every site event, no auth, no subscription filtering. A. accept at current scale, document, revisit at growth (combined with D-06 this leaks private-game activity). B. require a session and filter to per-connection subscriptions (games the user can see). C. session required now, subscription filtering later. Recommendation: B if `game_visibility` (D-06) is being enforced - otherwise the visibility work leaks through the socket; the filtering predicate is the same `is_game_visible_to_user`.

**Ruling:** Option B shape. The earlier label ambiguity (literal "A" = accept the firehose, which contradicted "gate activity feeds") is settled. Activity feeds are gated on participation-or-public using the same `is_game_visible_to_user` predicate, which necessarily includes the event stream. The user's stated intent, verbatim in substance: gate the `/ws` feed; only send public-game events to a client that actually has that public game's page open. The user recalls the previous version of brdg.me supporting client `sub`/`unsub` commands for specific public games, and asked whether user-specific events could instead ride on websocket authentication.

**Rationale / verified findings recorded under D-13** (the source cites line numbers; treat all of them as approximate, verify):

- (a) `/ws` authentication today: NONE, fully anonymous. `router.rs` registers `.route("/ws", get(websocket::ws_handler))`; `ws_handler` in `websocket.rs` takes only `WebSocketUpgrade` and `State<GameBroadcaster>` - no `Session`, no `HeaderMap`, no cookie read, no `get_current_user`, no token query param; `web/tests/websocket_hygiene.rs` asserts a cookie-less connect gets 101. BUT identity IS available: `/ws` is registered BEFORE `.layer(session_layer)`, so the Postgres-backed tower-sessions `SessionManagerLayer` does wrap it and a `Session` extractor would resolve (contrast `/healthz`, deliberately registered after the layer to bypass it).
- (b) What it broadcasts: an unfiltered site-wide firehose. `GameBroadcaster` publishes `GameUpdateSignal { game_id }` to `game.{game_id}` and `ProposalUpdateSignal { proposal_id }` to `proposal.{proposal_id}`; every socket subscribes to wildcards `game.>` and `proposal.>` and forwards each payload verbatim with no filtering and no per-socket state. Payloads are skinny JSON (UUIDs only, no state or names) so the data leak is bounded, but the existence and timing of every move and proposal event site-wide is visible to any anonymous connection. No `user.>` subject exists; `websocket.rs` actively asserts nothing is published to `user.>` or `ws.>`. Filtering is entirely client-side in the WASM, and the global `trigger.last_update` counter is bumped for EVERY frame, keying the sidebar `active_games` resource and `HomePage`'s `public_index` - so every site-wide event causes a server-fn refetch on every connected client: an N-clients x all-events amplification, a real load bug independent of the privacy question.
- (c) No subscribe/unsubscribe protocol exists, server or client. The server polls inbound frames (for pong/close) but discards the payload - comment verbatim: "we don't act on client-sent data here". No command parsing, no client->server message enum. The client never sends (the `send` handle of `UseWebSocketReturn` is not bound; only `on_message_raw`). No vestige of the old `sub`/`unsub` protocol survives in `rust/`; `rust/web/public/` has no legacy JS. So `sub`/`unsub` would be NEW work, not a restoration.

**Recommended design recorded under D-13** (WP-42's spec was NOT written in that unit; retained as history per the D-44 banner above): the socket is anonymous today but cheaply authenticatable, so the user's preferred shape - user-scoped events filtered server-side by identity, `sub`/`unsub` only for public-game pages - was the target. Direct answer to the user's question: YES, user-specific events can ride on websocket authentication, but identity alone is not sufficient because the subject scheme carries no user dimension. (1) Authenticate the upgrade: add `session: tower_sessions::Session` (and `State<PgPool>`, whose `FromRef<AppState>` impl already exists) to `ws_handler`; both are `FromRequestParts` so extractor ordering is unconstrained and no layer/router reordering is needed. Resolve identity BEFORE `ws.on_upgrade(...)`, not inside the closure - once the 101 is returned the connection is hijacked and the session layer's response-side save pass has already run. Use `auth::session::get_user_from_session` + `validate_session_token` directly, NOT `get_current_user` (a `#[server]` fn depending on leptos `extract()` and an `expect_context::<PgPool>()` that only `leptos_routes_with_context` provides, which does not cover the plain `/ws` route). (2) Do NOT reject anonymous upgrades: logged-out visitors legitimately need the public-game stream, and `websocket_hygiene.rs` asserts a 101 for an unauthenticated connect. The shape is "authenticate if a session exists, degrade to a public-only stream if not", never a 401. (3) Give the socket something to filter on - two options: (i) wildcard + per-socket membership filter (keep `game.>`/`proposal.>`, load the user's participating game/proposal ids from Postgres at connect, drop non-matching frames; cheapest diff, cost is invalidation when membership changes mid-connection); (ii) per-user fan-out subjects (`user.{user_id}.game.{game_id}`, subscribe only to `user.{uid}.>`; cleaner at read time but `broadcast_game_update(game_id)` takes only a game id and does no DB read, so every publisher must learn the recipient set - the bulk of the work; also inverts the assertions that `user.>` stays empty). Recommendation: (i) first, because it is one handler's diff and does not touch eleven publish sites; revisit (ii) only if mid-connection invalidation proves awkward. (4) `sub`/`unsub` is then needed only for public-game pages - exactly the user's intuition: for a non-participant viewer, identity carries no information about what page they are on, so the client must say. Either a genuine client->server `sub {game_id}` / `unsub {game_id}` protocol (server starts reading inbound frames instead of discarding them; client binds the `send` handle it currently drops), or the cheaper no-protocol option of a single always-subscribed "public games" subject. The user's stated intent - "only send public-game events to a client that actually has that public game's page open" - REQUIRES the real `sub`/`unsub` protocol; the public-subject shortcut does not satisfy it. (5) Client-side, `track_game_seq` and the `(Uuid, seq)` signals can stay; the win is `trigger.last_update` stops firing on irrelevant events, killing the refetch amplification. Scope split as recorded: auth + identity filtering (items 1-3, 5) was WP-42; the visibility predicate it filters against is WP-47's `is_game_visible_to_user`, which must be the SAME predicate the HTTP endpoints use; the `sub`/`unsub` protocol (item 4) was also WP-42 but a separable second task.

**Sources:** `decisions-needed.md`. Superseded in direction by `decisions-session3.md` D-44.

## D-14 Auth edges: squatting, enumeration, send caps, expiry

**Status:** ANSWERED (2026-07-25) - MODIFIED, DEVIATES FROM THE RECOMMENDATION on (iv); CONFIRMED (2026-07-26). Unblocks WP-35.

**Decision:** Four related auth-flow semantics calls: (i) unverified `add_email_address` blocks the real owner's signup forever (squatting); (ii) blocked-domain check leaks account existence; (iii) send-cap accounting is cumulative-forever (over-counts); (iv) session tokens never expire and there is no revoke-all. Options: (i) A. unverified claims expire (e.g. 24h) and a successful code confirmation by the true owner steals the claim / B. status quo. (ii) A. accept the differential response (it only reveals blocked-domain membership) / B. uniform handling (the originally suggested uniform-reject would lock out existing verified users - if B, it must special-case them). (iii) windowed counter (the only sound fix per verification). (iv) A. add expiry + GC + "log out everywhere" / B. document as intentional. Recommendation: (i) A, (ii) A with a comment, (iii) windowed, (iv) A.

**Ruling:** MODIFIED. Michael's words, verbatim: "fail closed in production on auth failure, expiring unverified email claims, windowed confirmation-send cap, revoke-all-sessions. NO session expiry - the user explicitly does not want sessions to expire. INSTEAD, changing an account email must require email re-verification (step-up confirmation to the new address). Note the interaction with D-1: since account-security commands leave the email interface, the email-change flow lives in the web UI with a confirmation link." Settled per sub-item: (i) A - unverified `add_email_address` claims expire and the true owner's successful confirmation steals the claim. (ii) A - accept the differential blocked-domain response, with an explanatory comment. (iii) windowed confirmation-send counter. (iv) NEITHER A NOR B as written - sessions must NOT expire and no session-expiry GC is to be added; `revoke-all-sessions` ("log out everywhere") IS in scope; the compensating control for a compromised address is that changing an account email requires re-verification, a step-up confirmation sent to the new address, living in the web UI (D-01 removed account-security commands from email).

CONFIRMED (2026-07-26) - link-vs-code is a NON-GOAL; keep the 6-digit code. The 2026-07-25 answer says "confirmation link", but the email-change flow already exists in live code, is already compliant, and uses a 6-digit code. The specs correctly marked link-vs-code cosmetic. WP-35 and WP-56 ship as specced; nothing changes. No new package.

**Rationale:** Michael's rationale on link-vs-code: it is "low value UI we need to maintain into the future." An actual link is new UI work with no security gain; if it is ever wanted it needs its own package.

**Sources:** `decisions-needed.md` (the full four-sub-item ruling), `decisions-ANSWERED.md` (the narrow 6-digit-code confirmation).

## D-15 Reserved email verbs vs game move grammars

**Status:** REOPENED 2026-07-25 (recorded basis was FALSE); ANSWERED (2026-07-26) - REDESIGNED and SETTLED. Informs WP-59. **Its downstream consequences are QUALIFIED by D-54 and D-55 - see the reconciliation below.**

**Decision:** Game-scoped email dispatch reserves verbs (undo, concede, ...) that could collide with a game whose move grammar uses the same word. A. document the reservation as a platform constraint for game authors. B. escape prefix (e.g. `move <text>` forces game interpretation). Recommendation: A now (no current collision), B only when a real game needs it.

**Reopening finding:** The recorded basis "no current collision" is FALSE. Verified by reading live source: `end` is a live top-level game move in two shipped crates - `rust/game/acquire-1/src/command.rs` (`Doc::name_desc("end", "trigger the end of the game at the end of your turn", ...)`) and `rust/game/starship-catan-1/src/command.rs` (`Doc::name_desc("end", "end the flight early", Token::new("end"))`). The email dispatcher intercepts it first: `rust/web/src/email/commands.rs` has `"end" => return run_end(ctx).await,` which runs BEFORE the game path (added post-review-snapshot by issue #47). So an acquire-1 or starship-catan-1 player CANNOT issue `end` by email today - a live functional defect, not merely a docs matter. A repo-wide grep over `rust/game/` finds no other reserved-verb collision.

**Ruling:** NEITHER A NOR B NOR "A-plus". REDESIGNED. Michael proposed the design; the Orchestrator ruled on it. Do NOT hardcode a reserved-verb list. On game-scoped messages, try the game command parser FIRST. Platform commands are the FALLBACK, tried only when the game parser fails on that input. One carve-out: keep a small hard-reserved set of escape-hatch verbs (`help` and equivalents) that ALWAYS win, even on the game path - rationale: a game with a greedy parser must not be able to swallow the only command that unsticks a user. Keep this set small and obvious. This SUPERSEDES the "A-plus" recommendation put to the user (keep the reserved-verb list and disambiguate on the game-scoped path so a declaring game wins there); there is NO reserved-verb list in the new design beyond the escape-hatch set - do not implement a reservation table. `wfe F29` follows this outcome.

**RECONCILIATION - the "defect is fixed" / "Task 14 unblocked" text is SUPERSEDED.** Both `decisions-needed.md` and `decisions-ANSWERED.md` recorded, as a consequence of this redesign, that the live `end` defect "is fixed" and that WP-59 Task 14 is UNGATED (with its COMMANDS.md content rewritten to describe parser-first dispatch). That is NO LONGER TRUE. Per D-54, Task 14 was CARVED OUT of WP-59 into **WP-85**. Per D-55, the escape-hatch verb set is DEFERRED and **WP-85 is DEFERRED - BLOCKED ON MICHAEL**. Therefore the acquire-1 / starship-catan-1 `end` collision **stays OPEN** until WP-85 lands - an accepted cost of the deferral, not an oversight. WP-59 Task 14 must not be read as live work.

**Sources:** `decisions-needed.md` (REOPENED status, verified false basis, `wfe F29` consequence), `decisions-ANSWERED.md` (the SETTLED redesign, in its "five rulings changed" table). Qualified by `decisions-session3.md` D-54 and D-55.

## D-16 Turnstile rendering after client-side nav

**Status:** ANSWERED (2026-07-26) - Option B, OVERRULED in favour of the simpler option; mechanism VERIFIED. Unblocks WP-55.

**Decision:** The Turnstile widget likely never renders when `/login` is reached by SPA navigation (script only scans on full page load). A. explicit `render()` call from the login component effect. B. force full-page load for `/login` links. Recommendation: A - keeps SPA behaviour; B is a one-line fallback if A misbehaves.

**Ruling:** Option B. This SUPERSEDES the recommendation (option A, explicit `render()` from the login component effect) - do NOT call Turnstile's `render()` from an effect. Make `/login` a normal, unrouted link that forces a full page load, so Turnstile's automatic rendering just works.

**Rationale:** The user's reasons: complexity concern, and the login page should load very fast.

**Mechanism verified 2026-07-26 by reading the vendored router source (read-only); `rel="external"` works in the version actually in the tree:** `rust/web/Cargo.toml` has `leptos = "0.8.20"`, `leptos_router = "0.8.14"` (`Cargo.lock` resolves 0.8.14 exactly). `leptos_router-0.8.14/src/location/mod.rs` reads the DOM `rel` attribute, splits on space/tab, and returns early - letting the browser handle the click - if any token is `external` (or the anchor has `download`); so `rel="external"` and `rel="noopener external"` both opt out of client-side routing. A plain `<a>` is NOT sufficient on its own: interception is a window-level click listener (`leptos_router-0.8.14/src/location/history.rs`, `window_event_listener(ev::click, ...)`) that walks `composed_path()` for any `HtmlAnchorElement`, regardless of whether the anchor came from `<A>` or a literal `<a>` - `rel="external"` is required either way. `<A>` has no `rel` prop (its props are `href`, `target`, `exact`, `strict_trailing_slash`, `scroll`, `children`); use either `attr:rel="external"` spread onto `<A>` (attribute spreading on `<A>` is already proven in this codebase - `rust/web/src/app.rs` uses `attr:class` on the `/login` link) or a plain `<a href="/login" rel="external">`. The plain anchor is simplest; `<A>`'s only extra behaviour is `aria-current` active marking, irrelevant for a login link. Current `/login` links, both `<A>` and both client-side routed today: `rust/web/src/app.rs` (the `index-cta` "Start a game" link) and `rust/web/src/components/layout.rs` (the "Login" nav link). Turnstile context confirmed: the `api.js` `<script async defer>` lives in the shell head in `rust/web/src/app.rs`, and the `<div class="cf-turnstile" ...>` widget is in the same file.

**GAP - WP-55 must also close this, `rel` cannot cover it:** three navigations to `/login` go through `use_navigate`, which never touches an anchor and is therefore never subject to the `rel` check - `rust/web/src/components/layout.rs` (post-logout), `rust/web/src/settings.rs` (anonymous redirect), `rust/web/src/admin.rs` (anonymous redirect). These need a hard navigation (a location assignment) instead, or Turnstile will still fail to render for users who arrive at `/login` by those paths.

**Sources:** `decisions-needed.md` (verification inline), `decisions-ANSWERED.md` (same ruling; verification as a separate note).

## D-17 sqlx 0.8/0.9 unification

**Status:** ANSWERED (2026-07-26), with a STANDING PROCESS CHANGE. Unblocks WP-66.

**Decision:** web is on sqlx 0.8 (pinned by `tower-sessions-sqlx-store`); bot/operator on 0.9. Both stacks compile into the workspace; two type-mapping behaviours against one DB. A. wait for an sqlx-0.9-compatible `tower-sessions-sqlx-store` release. B. vendor the (small) session store now and move everything to 0.9. Recommendation: B if no compatible release exists at fix time - the store is trivial; check crates.io first, A if it has shipped.

**Ruling:** ACCEPTED, but with an explicit FIRST STEP before either option. (1) Upgrade all dependencies to latest and see where we stand - the sqlx 0.8/0.9 split may simply resolve. (2) Only if it does not, vendor the `tower-sessions-sqlx-store` (option B) and move everything to 0.9. This is a standing process change binding the WHOLE dependency group, not a one-off - see Standing constraints.

**Rationale:** Michael's strategy is to stay as close to latest as possible so dependencies never go stale.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-18 sentry feature trim

**Status:** ANSWERED (2026-07-26). Unblocks WP-67.

**Decision:** sentry default features drag actix-web + ureq into every server build; the native-tls transport choice is deliberate. Confirm trimming to explicit features (backtrace, contexts, panic, tracing/tower as used + native-tls transport), verified with `cargo tree`? Recommendation: yes - mechanical once the feature list is confirmed against actual usage; no product trade-off.

**Ruling:** Yes, trim to explicit features (backtrace, contexts, panic, tracing/tower as used, native-tls transport), verified with `cargo tree`.

**Rationale / STANDING CONSTRAINT:** it is CRITICAL that no Sentry functionality is lost. The trim must be verified to PRESERVE CURRENT BEHAVIOUR, not merely to shrink the dependency tree. Enumerate the sentry features actually in use before removing any, and check the resulting build still reports what it reports today. Preserve the deliberate native-tls transport choice.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-19 [workspace.dependencies] migration

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-64.

**Decision:** No workspace dependency/package/lints tables; shared versions copy-pasted across 40 manifests and already drifting. Touches every manifest; natural umbrella for later bumps. Proceed, and in what scope? A. full: `workspace.dependencies` + `workspace.package` + `workspace.lints` in one migration PR, early in the backlog. B. dependencies table only. Recommendation: A.

**Ruling:** Option A. All three tables - `[workspace.dependencies]`, `[workspace.package]`, `[workspace.lints]` - in one migration, early. Unblocks WP-64 and resolves the `dp F9` version-pin row in the T3-B8 checklist. Sequence per the standing dependency process change: upgrade everything to latest first, then migrate.

**Rationale:** The marginal cost of package+lints inside the same sweep is near zero and lints enforcement helps every later package.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-20 108 boilerplate game binaries

**Status:** ANSWERED (2026-07-26), option B; naming VERIFIED. Unblocks WP-73; sequence after WP-64.

**Decision:** 27 crates x 4 near-identical bins; binary-only deps (tokio full, brdgme_cmd, fuzz) declared as lib deps. The reviewed `[dev-dependencies]` fix is invalid (dev-deps do not apply to `src/bin`). A. `brdgme_game_bins!(Game)` macro in lib/cmd generating the 4 bins; deps stay per-crate but feature-trimmed. B. one generic parameterised bin crate (game selected by feature or a thin per-game bin crate depending on it); tokio/fuzz deps live once. C. keep files; just trim tokio features 27 times. Recommendation: B.

**Ruling:** Option B - a generic bin crate parameterised over the `Gamer` trait, with thin per-game wrapper bin crates. EXPLICITLY NOT option A (the macro). Michael approved B partly because it avoids macros. Do not "simplify" it back into a macro.

Concrete name, VERIFIED against the repo's layout (2026-07-26, read-only): `rust/lib/game_bin`, with `[package] name = "brdgme_game_bin"`. The convention is snake_case directories under `lib/` and `tools/` with package names `brdgme_<snake_dir>` - consistent across all ten (`lib/cmd` -> `brdgme_cmd`, `lib/game_client` -> `brdgme_game_client`, `tools/fuzz` -> `brdgme_fuzz`, ...); `lib/game_client` -> `brdgme_game_client` is the direct precedent for a two-word snake name. Hyphens are the game-crate convention (`game/red7-1` -> `red7-1`), and `brdgme-operator` is the single hyphenated outlier, not under `lib/`. Do NOT name it `game-bin` / `brdgme-game-bin`.

Structural note for WP-73: today the 4 bins are `[[bin]]` targets INSIDE each game crate at `src/bin/<snake_name>_{cli,fuzz,http,repl}.rs`, each a 3-10 line `Gamer`-parameterised call (e.g. `http::serve::<Game>(addr)`). The `[[bin]]` machinery lives in the game crates, not in separate bin crates, so moving to thin per-game bin crates is a structural change to the workspace, not just a file move - factor that into the spec.

**Downstream amendments from planning session 3 (these win):** D-41 deletes the 27 per-game `_repl` bins and D-43 reverses the `_fuzz` deletion, so `brdgme_game_bin` ships **three** generic entry points (`cli_main`, `http_main`, `fuzz_main`), not four; D-42 extends WP-73 to `lords-of-vegas-1`; and session 3 records a terminology correction that there is **no macro** - `brdgme_game_bin` must remain macro-free.

**Rationale:** B is the only option that actually removes the files and centralises the heavy deps; the k8s images already build per-crate so a per-game thin bin crate maps cleanly. Plus: it avoids macros. STANDING CONSTRAINT on macros, wider than this item - see Standing constraints.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling and naming), amended by `decisions-session3.md` (D-41, D-42, D-43, terminology correction).

## D-21 serde_yaml migration

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-70.

**Decision:** `serde_yaml` is archived; consumers are bot + lib/game_client (must move together). A. `serde_yaml_ng`. B. `serde-yml`. C. `saphyr`. D. switch the surface to JSON. Recommendation: A.

**Ruling:** Option A - `serde_yaml_ng`. Drop-in API, maintained. Not JSON: that would change a file format ops and users may depend on. bot and lib/game_client move together.

**Rationale:** Drop-in API, maintained; JSON would change a depended-on file format.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-22 warp -> axum in lib/cmd

**Status:** ANSWERED (2026-07-26). Unblocks WP-71.

**Decision:** warp serves the game-service HTTP layer while the platform is axum; two HTTP stacks in the tree. Touches all 28 game binaries' HTTP surface (mechanically small handler). Port now or defer? Recommendation: port - the handler is one endpoint; do it in the same window as WP-06's `http.rs` fixes so the surface is touched once.

**Ruling:** Port now, in the same window as WP-06's `http.rs` fixes so the surface is touched once.

**Rationale:** It is one endpoint, though it is the HTTP layer of all 28 game binaries.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-23 deny.toml hardening

**Status:** ANSWERED (2026-07-26). Unblocks WP-69.

**Decision:** bans are warn-level (toothless); 4 stale advisory ignores for crates absent from the lock. Confirm flip `multiple-versions` to deny AFTER the dedup packages (WP-66/67/68) land, with the residual duplicates enumerated in skip/skip-tree, and clear the stale ignores now? Recommendation: yes, in that order.

**Ruling:** Yes, in exactly that order. Clear the 4 stale advisory ignores now; flip `multiple-versions` to deny only AFTER WP-66/67/68 land, with the residual duplicates enumerated in skip/skip-tree.

**Rationale:** Land WP-69 LAST among the dependency packages so the skip-list starts minimal.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-24 combine dependency posture

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-72.

**Decision:** `combine` 4.6 is dormant and sits at the heart of markup/game parsing. A. accept as recorded risk, note in `deny.toml`, migrate only when the parser is next rewritten. B. migrate `brdgme_markup` to winnow / in-house now. Recommendation: A.

**Ruling:** Option A - accept `combine` 4.6 as a recorded risk. Note it in `deny.toml`; migrate markup off `combine` only when the parser is next rewritten.

**Rationale:** WP-02 already changes markup enough for one release, and combine carries no advisory today.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-25 lib/cost consolidation

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-17 (now fully unblocked).

**Decision:** `lib/cost` has one consumer (seven-wonders-1) while splendor-2 reimplements the same Go-origin cost logic locally; half-shared is the worst state. A. port splendor-2 onto `lib/cost` (add get/set + keep splendor's gold-joker `can_afford` as a crate-local extension). B. fold `lib/cost` into seven-wonders-1 and delete the lib. Recommendation: A.

**Ruling:** Option A - port splendor-2 onto `lib/cost`. Add generic `get`/`set`; keep splendor's gold-joker `can_afford` as a crate-local extension.

**CONSTRAINT:** the shared `lib/cost` must have a suitable amount of automated testing as part of the port - it gains a second consumer, so it stops being incidentally covered by seven-wonders-1's tests; give it its own.

Scope reminder: D-25 gates only 3 of WP-17's 8 findings (`b F31`, `ls F39`, `dp F27` - one indivisible consolidation). The other 5 (`b F30`, `b F32`, `b F34`, `b F35`, `ls F38`) were always implementable. `checklists/T3-B3-splendor-libcost-holdem.md` holds the authoritative row-by-row split.

**Rationale:** Two consumers justify the lib and the API additions are small; B throws away the shared abstraction the next economic game will want.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-26 Modern Art cluster

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded - PARKED.** Gates WP-26.

**Decision:** (i) round-4 end semantics underlie the critical hang + soft lock (WP-25 fixes the liveness mechanically regardless); (ii) payout pays cumulative value for ALL purchases incl. non-top-3 artists - documented in RULES.md, may canonize a Go defect; (iii) zero-card artists are ranked and awarded $20/$10 (undocumented, inflates (ii)); (iv) sealed/once-around bid ties go to the auctioneer. For each of (ii)-(iv): keep-and-document or fix to official?

**Ruling:** no ruling recorded - PARKED. Recommendations were written under the superseded D-35 option C and must be re-read under policy A before use; nothing may act on them while the park stands. Recorded recommendation, for history only: (ii) documented -> keep, but flag to the user explicitly because it materially changes scoring; (iii) undocumented -> fix to official (zero-card artists unranked); (iv) undocumented -> fix (ties to earliest bid/challenger per official rules).

**Cross-reference:** item (iii) overlaps finding `d F37`, which the 2026-07-26 ruling **REJECTED as not a bug** - see Finding-level rulings. That is the only movement on this item.

**Sources:** `decisions-needed.md`.

## D-27 seven-wonders deviations

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded beyond the park.** Gates WP-16.

**Decision:** F4 same-turn trade of freshly built resources (asymmetric by player index); F5 MimicGuild copies only Bonus guilds; F6 wonder-stage sacrifice enters shared discard (contradicts own RULES.md); F7 both sides of one wonder can be dealt (fix perturbs RNG draw ordering); F8 discard pile hidden from all (Halicarnassus takes blind).

**Ruling:** no ruling recorded beyond the park, EXCEPT via the per-finding egregious rulings: `b F7` is **FIX NOW** outside the park; `b F4` is **PARKED** with the user's binding correction (7 Wonders resources are NOT depleted by trade). F5/F6/F8 are explicitly listed as "NOT egregious - genuine edition/parity questions, leave parked". Recorded recommendation, history only: F4 fix (snapshot tradable goods); F5 extend to Science guilds; F6 fix (contradicts own RULES.md); F7 fix distinct boards, accepting the RNG ordering change; F8 add discard contents to PubState.

**Sources:** `decisions-needed.md`; per-finding rulings from `decisions-ANSWERED.md`.

## D-28 splendor prestige tie-break

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded.** Gates WP-16.

**Decision:** Ties broken by MOST cards (Go parity, locked by a test); official rules say FEWEST cards. A. fix to official (update test). B. keep documented Go parity. Recommendation (written under the superseded D-35 option C): A.

**Ruling:** no ruling recorded. Explicitly listed as "NOT egregious - genuine edition/parity question, leave parked".

**Sources:** `decisions-needed.md`.

## D-29 red7 empty-winning-set

**Status:** PARKED-PENDING-USER-RULES-REVIEW (the "can an empty set win" half). Gates WP-30.

**Decision:** A player with zero rule-fulfilling cards is treated as winning; official rules say they cannot win. Adopting official needs a defined outcome when ALL players have empty sets. A. official: empty set cannot win; all-empty resolved by elimination order (last player standing by highest card per red7 tie rules). B. document the deviation. Recommendation: A - the current behaviour lets a player win with nothing, which is strategy-breaking, and DATA_DOCS already contradicts the code.

**Ruling:** partial. The seat-order tie-break half, finding `e F30`, was ruled **FIX NOW** (2026-07-26, condition satisfied - see Finding-level rulings). The "can an empty winning set win at all" half **STAYS PARKED**; no ruling recorded for it.

**Sources:** `decisions-needed.md`; `e F30` ruling from `decisions-ANSWERED.md`.

## D-30 Player-count caps vs official

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded.** Gates WP-11, WP-20, WP-26 items.

**Decision:** texas-holdem 8 vs Go's 9; category-5 8 vs official 10; lords-of-vegas 2-6 vs official 2-4; no-thanks 3-5 (2004 edition) vs 3-7 (later editions). Per game, restore official/Go count or document the cap? Recommendation (history only): category-5 -> 10 (its own RULES.md already says 10, so the code contradicts in-crate docs); texas-holdem -> document 8 (render width constraint is plausible, no doc contradiction); lords-of-vegas -> document 2-6 in RULES.md (extra capacity is a feature; but note the WP-22 render fix must then cover 5-6p); no-thanks -> keep 3-5 and note the edition in RULES.md.

**Ruling:** no ruling recorded. Explicitly listed as "NOT egregious - all of D-30's player caps, leave parked".

**Sources:** `decisions-needed.md`.

## D-31 acquire edition behaviours

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded.** Gates WP-20.

**Decision:** Random start player (official: initial tile draw decides); full-hand redraw permanently discards temporarily-unplayable tiles (and mass redraw can drain the bag - compounds); bag-exhaustion ends the game mid-turn. No Go port to match; edition-dependent. Pick a reference edition and align all three? Recommendation (history only): align to the current Hasbro/Avalon Hill rules - tile-draw start, redraw only permanently-unplayable tiles, finish the turn on bag exhaustion. Decide all three together (F14+F15 interact).

**Ruling:** no ruling recorded. Explicitly listed as "NOT egregious - all three D-31 acquire items (the findings name later-Hasbro vs classic 3M/AH editions explicitly, and bag-exhaustion differs by edition), leave parked".

**Sources:** `decisions-needed.md`.

## D-32 jaipur adjudications

**Status:** PARKED-PENDING-USER-RULES-REVIEW. **No ruling recorded.** Gates WP-26.

**Decision:** (i) next-round starter is not the round loser (original finding's premise uncorroborated - needs the rulebook quote); (ii) camel token counted as a bonus token in the end-of-round tie-break; (iii) camel count hidden in render but exact in PubState. Recommendation (history only): (i) check the rulebook at spec time - **source text is MALFORMED here, an unfinished sentence**: "official Jaipur: the round LOSER deals but the WINNER... (verify - if confirmed loser-starts, fix and restore major)"; (ii) official: camel token is not a bonus token for the tie-break - fix; (iii) hide in both (counts-only in PubState) for consistency with the renderer's intent.

**Ruling:** no ruling recorded. Explicitly listed as "NOT egregious - D-32 jaipur ((i) premise uncorroborated, (ii)/(iii) are adjudications), leave parked".

**MALFORMED IN SOURCE:** the recommendation for sub-item (i) is an unfinished sentence in `decisions-needed.md`. Do not reconstruct what it meant - the rulebook must be checked at spec time. Also flagged in the UNKNOWN section.

**Sources:** `decisions-needed.md`.

## D-33 pub_state redaction design

**Status:** ANSWERED (2026-07-25), option A, jointly with D-35. NOT parked. Unblocks WP-10 (stays READY).

**Decision:** zombie-dice serializes the shuffled cup in draw order (next draws readable - a NEW bug vs Go); for-sale leaks selling-phase secret plays via `PubState.bids`. Needs one shape for all game crates. A. counts-only / canonicalized public fields (cup as counts; bids only after reveal), private detail re-added per-player in `player_state` where the viewer is entitled. B. per-player private field in a unified state envelope (bigger refactor). Recommendation: A.

**Ruling:** Option A. Public view data exposes COUNTS/AGGREGATES ONLY; per-player secrets move into `player_state` where the viewer is entitled. Answered jointly with D-35. WP-10 is NOT parity-gated and stays READY - its scope is hidden-info leakage, not rules parity, despite its heading mentioning D-35.

**Rationale:** Minimal serde surface change, matches how other crates already handle hidden info, and Go parity is irrelevant here (zombie-dice's leak is new).

**Sources:** `decisions-needed.md`. See also N-2 for the concrete zombie-dice-2 shape.

## D-34 rtta-2 fidelity policy

**Status:** PARKED-PENDING-USER-RULES-REVIEW (F7/F9 and the quirk-preservation policy). Gates WP-12.

**Decision:** The crate deliberately preserves Go quirks (annotated), but F1 (phase re-match after `keep_skulls` skips/loses rolls) produces objectively wrong state and diverges from the crate's own next-path test; F7/F9 are smaller quirk-vs-fix calls. A. fix F1 (declare the roll-path canonical, update the next-path test); keep other annotated quirks; F7/F9 fix (cheap, no replay impact). B. strict Go fidelity: document F1 as a quirk too. Recommendation: A - "wrong state reachable in normal play" is past the line that the crate's own quirk-preservation policy drew.

**Ruling:** partial. Finding `a F1` was ruled **FIX NOW** outside the park (2026-07-26) - see Finding-level rulings. F7/F9 and the quirk-preservation policy remain **parked with no ruling recorded**; the rest of WP-12 stays parked.

**Sources:** `decisions-needed.md`; `a F1` ruling from `decisions-ANSWERED.md`.

## D-35 Global port-parity policy (answer first)

**Status:** ANSWERED (2026-07-25) then PARKED-PENDING-USER-RULES-REVIEW (2026-07-25); park CONFIRMED (2026-07-26). Informs D-26..D-34, unblocks WP-11. Placed BEFORE D-26 in `decisions-needed.md` ("answer first").

**Decision:** Many crates faithfully reproduce their Go origin while diverging from official rules; RULES.md sometimes documents the Go behaviour, sometimes contradicts the code. Verification established one precedent: where in-crate docs (RULES.md/PORTING_NOTES) document the behaviour, the code is correct as documented. What is the default policy? A. official rules win; Go quirks are bugs unless in-crate docs explicitly claim the deviation. B. Go parity wins (preserve historical game records/replays); RULES.md updated to document every deviation. C. documented-in-crate wins (the verification precedent). Recommendation: C.

**Ruling:** Option A - official rules win. Michael's words: "For port-parity conflicts, the OFFICIAL rules are authoritative - correct both the code and RULES.md, noting the Go divergence in the commit/doc." This is STRONGER than option A as written and rejects option C's "documented-in-crate wins" precedent: where a crate's RULES.md documents a Go-derived deviation from the official rules, BOTH the code and RULES.md get corrected, and the commit message / doc notes the Go divergence. Answered in the context of D-33 / WP-10 port-parity conflicts, but D-35 is by construction the global policy item, so it is recorded as the global default for D-26..D-34. Scope caveat: this flips the working assumption of several already-triaged parity items whose recommendations were written under "C"; those per-game items (D-26..D-32, D-34) must be re-read under policy A before their packages are specced.

PARKED 2026-07-25 - the policy stands but nothing may act on it yet. Two constraints supersede the sequencing above: (1) NO gameplay change without per-game sign-off by the user - "official rules win" is the tie-breaker the user will apply *when reviewing*, not a licence for an agent to change game behaviour. (2) The whole parity question is parked pending the user's own review of the game rules, because some `RULES.md` content was AI-GENERATED AND MAY BE WRONG (so it is not a trustworthy baseline for "code vs docs" adjudication), and because EDITION/VARIATION CHOICES ARE THE USER'S TO MAKE. Consequently the "docs may be corrected" half is ALSO suspended for these items: do not rewrite a `RULES.md` toward official rules under D-26..D-32/D-34 either, since the doc may be the thing that is wrong. D-33 is unaffected - its `pub_state` redaction answer (option A) is independent of rules parity and WP-10 stays READY.

ANSWERED (2026-07-26): **KEEP THE PARK.** The user's answer to "when and in what order do you do the rules review": keep the park, do NOT lift it globally; do the review PER GAME, prioritising **acquire-1, seven-wonders-1 / splendor-2, modern-art-2 and red7-1** - those four unblock the most other work. `BLOCKED-ON-USER-RULES-REVIEW` remains STRONGER than `BLOCKED-ON-DECISION` - it does not clear when a decision is answered, only on the user's per-game sign-off. The only movement out of the park is the five individually-ruled egregious candidates: `a F1` FIX NOW, `b F7` FIX NOW, `e F30` FIX NOW (condition verified satisfied), `b F4` PARKED with the user's binding correction, `d F37` REJECTED as not a bug. Nothing else moves.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling; the keep-the-park answer). Full park text and carve-outs in Standing constraints.

## D-36 Deserialized-state trust strategy

**Status:** ANSWERED (2026-07-25), option A. Unblocks WP-09.

**Decision:** The requester layer (`lib/cmd/src/requester/gamer.rs`) deserializes stored/forwarded Game state and player indices verbatim; unchecked indexing panics exist across ~15 game crates (19 findings in WP-09 alone; the two lost-cities `player_state` panics are request-reachable today). A. requester-boundary fix: bounds-check player index + a validate-after-deserialize hook (trait method with default no-op) games can implement; per-crate panics become defence-in-depth, fixed opportunistically. B. per-crate defensive sweep only (`.get()` everywhere) - ~15 crates, no structural guarantee for future crates. C. accept: state comes from our own DB; fix only the two request-reachable `player_state` panics. Recommendation: A.

**Ruling:** Option A. Bounds-check the player index at the requester boundary PLUS a per-game `validate` hook run after deserialization. Per-crate unchecked-index cleanups become defence in depth and ride each crate's own package. Sequencing (from `critical-path.md`): land WP-09's boundary fix before the bulk of Phase 3 per-crate work, and coordinate with WP-28.

**Rationale:** One fix covers current and future crates; the request-reachable pair gets fixed either way, and per-crate cleanups can then ride each crate's own package.

**Sources:** `decisions-needed.md`.

## D-37 Markup literal-{ escape and unmatched-rest handling

**Status:** ANSWERED (2026-07-25), option A; CORRECTED (2026-07-26). Unblocks WP-02.

**Decision:** Unmatched markup silently truncates output (parser succeeds with tail in `rest`, callers discard it), and `to_string` emits raw text with no escaping - no round-trip, markup injection through text. Both need a convention for literal braces. A. error on non-empty `rest` + define an escape (`{{` or backslash) + escape on `to_string`. B. error on non-empty `rest` only; document that text must not contain braces. Recommendation: A with `{{`-style escaping; B leaves the injection hole.

**Ruling:** Option A. Error on a non-empty parse remainder AND escape braces in `to_string`.

CORRECTED (2026-07-26): the escape is **`{{lbrace}}`, NOT a bare `{{`**. Option A still stands in full; only the escape token changes. This SUPERSEDES the recommendation's `{{`-style escaping - do not implement a bare `{{`. Failure mode: with a bare `{{` the parser cannot distinguish an escaped literal brace from the start of a closing tag like `{{/b}}`, so a nested `markup()` consumes its own terminator - it cannot be implemented soundly. `{{lbrace}}` stays inside the `{{...}}` family the decision asked for. `}` needs no escape. `specs/WP-02-markup-robustness-dedup.md` already pins `{{lbrace}}`; that spec is correct as written.

**CONSTRAINT (user flag to carry into the WP-02 spec):** existing stored content which currently renders partially may start ERRORING; the spec must include a step to assess stored-content risk **BY READING CODE AND MIGRATIONS ONLY - NOT by querying any database.**

**Sources:** `decisions-needed.md` (fuller question, unmatched-`rest` half, the no-database constraint), `decisions-ANSWERED.md` (the `{{lbrace}}` correction, in its "five rulings changed" table).

## D-38 lib-game parser design items

**Status:** ANSWERED (2026-07-26) - ACCEPTED as recommended, all four sub-items. Unblocks WP-04.

**Decision:** (i) `OneOf` furthest-error ranking is dead code (all offsets provably 0) - implement offset propagation or delete the ranking; (ii) typed-vs-spec `expected()` impls diverge (`Doc` and `Many`) - align or document as deliberate; (iii) case folding differs (`to_lowercase` vs `UniCase`) between suggest and parse; (iv) depth guard for deserialized specs.

**Ruling:** ACCEPTED as recommended, all four sub-items. (i) implement `OneOf` offset propagation (do not delete the ranking); (ii) align the spec impls to typed behaviour and extend the existing parity tests to cover `expected()`; (iii) adopt UniCase in `suggest`; (iv) skip the depth guard - deserialized specs cross no trust boundary today.

**Rationale / STANDING CONSTRAINT, binding on WP-04 GENERALLY and not just these four items:** keep the parser as straightforward and obvious as possible. It is complex but critical to the app and must stay reliable and maintainable. At every choice point in WP-04, prefer the plainer implementation over the cleverer one - including in the offset-propagation plumbing, which is the item most likely to tempt an elegant abstraction.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-39 Color parse API delete-vs-keep

**Status:** ANSWERED (2026-07-26), option A. Unblocks WP-05.

**Decision:** `regex` + `lazy_static` exist solely for `from_hex`/`from_str` which have no runtime caller; three divergent color-name alias tables exist across color and markup. A. delete the dead parse API. B. keep the API, reimplement hex parsing in std, unify the tables. Recommendation: A - dead public API in an internal lib; resurrect from git if ever needed.

**Ruling:** Option A - delete the dead parse API (`from_hex` / `from_str`). This drops `regex` and `lazy_static` workspace-wide and resolves the three-way alias-table divergence by deletion. Git can resurrect it if it is ever wanted.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-40 Write-only stats subsystems keep-or-drop

**Status:** ANSWERED (2026-07-26), option B. Unblocks WP-20, WP-30 items; split out as WP-81.

**Decision:** acquire-1 tracks per-game stats but `to_brdgme_stats` has zero callers; lost-cities-1/-2 `Stats` fields are never written or write-only (and one increment counts the wrong thing). Either wire stats into `status()`/the platform stats pipeline or delete the machinery. A. wire them up (needs a defined platform consumption path). B. delete the dead machinery in all three crates; re-add when the platform grows a per-game-stats feature. Recommendation: B.

**Ruling:** Option B - delete the dead machinery in acquire-1 (`to_brdgme_stats`, finding `c F12`) and lost-cities-1/-2 (`e F39`, `e F40`), AND split these items out of WP-20/WP-30 into their own package so they can land AHEAD of the rules review. They are stats questions, not rules questions. The split-out package is **WP-81** in `work-packages.md`. WP-20 and WP-30 lose their D-40 blocker and their stats items; both remain `BLOCKED-ON-USER-RULES-REVIEW` for their rules halves.

**Rationale:** For the record: Michael wants to revisit "game specific stats" in future **from a CLEAN SLATE**. That is precisely why deleting the dead machinery now is right - there is no platform path to wire it into, and the future feature will not want to inherit this shape.

**Sources:** `decisions-needed.md`, `decisions-ANSWERED.md` (same ruling).

## D-41 Delete the per-game `_fuzz` and `_repl` binaries

> **ID COLLISION NOTE.** Two unrelated decisions were numbered D-41. This section is the `decisions-session3.md` one. The `decisions-needed.md` D-41 (friends-page `<select>` revert) has been renumbered to **D-56** and appears at the end of this record.

**Status:** DELETE, but verify first - **partially SUPERSEDED by D-43**, which reverses the `_fuzz` half. D-41 stands for `_repl` only.

**Decision:** Are the 27 per-game `_fuzz` and 27 per-game `_repl` binaries (54 files) redundant now that `rust/tools/fuzz` and `rust/tools/repl` are generic out-of-process drivers shelling out to a game's `_cli` binary?

**Ruling:** DELETE, but verify first. Required before deleting: confirm by reading and searching that nothing outside `rust/tools/fuzz` and `rust/tools/repl` invokes the per-game `_fuzz`/`_repl` binaries. Known: `rust/Dockerfile` copies only `target/release/<snake>_http`. Also check `docker-bake.hcl`, `Tiltfile`, k8s manifests, CI config, `justfile` / `Makefile` equivalents, docs, and any test harness. **If anything does depend on them, STOP and report - do not delete.** Consequence for WP-73: `brdgme_game_bin` then needs only **two** generic entry points (`cli_main`, `http_main`); `fuzz_main` and `repl_main` are dropped from the spec. **As amended by D-43, only the 27 `_repl` bins are deleted and WP-73 ships THREE entry points including `fuzz_main`.**

**Rationale:** Michael's rationale: it is simpler, and the out-of-process boundary is what would make non-Rust game implementations viable again in future.

**Sources:** `decisions-session3.md`. Amended by D-43.

## D-42 `lords-of-vegas-1` gets WP-73 too, but stays undeployed

**Status:** RULED. Heading restored 2026-07-26 - this ruling had been orphaned under D-51's "Home for it" section with no heading of its own, and has nothing to do with the fuzzer.

**Decision:** Does the undeployed `lords-of-vegas-1` crate receive the WP-73 bin consolidation?

**Ruling:** APPLY WP-73 to `lords-of-vegas-1` like every other game crate. Note it is **not** a deployment gap to close: leave it undeployed. Only the bin consolidation applies.

**Rationale:** It is a workspace member with all four bins but is not deployed - no Dockerfile stage, no bake target, no Tiltfile entry, no k8s directory. Michael plans to return to that game and finish it, so it stays a full workspace member and receives the same treatment.

**Sources:** `decisions-session3.md`.

## D-43 SUPERSEDES D-41 for fuzz: throughput beats simplicity

**Status:** REVERSED pending performance evaluation; **RESOLVED 2026-07-26** via `planning/fuzz-throughput-evaluation.md` (Lead-ACCEPTED), with **one part still OPEN and needing Michael**.

**Decision:** Is out-of-process fuzzing acceptable given `LocalRequester` spawns one child process per API request?

**Ruling:** the `_fuzz` deletion is **REVERSED** pending a performance evaluation. D-41 stands for `_repl` only. **Selection criterion for fuzzing is raw throughput.** Simplicity and the non-Rust-game portability argument are explicitly subordinate for this one concern; they still apply to `_repl`, which is interactive and has no throughput requirement.

Resolution as recorded: out-of-process **rejected** (one process spawn per move x `num_cpus` threads, plus a second full JSON layer; directionally strictly slower, **magnitude UNMEASURED** - no cargo in a planning session); `fuzz_main::<G>()` **confirmed speed-neutral** and adopted in WP-73 (three entry points, 27 `_fuzz` bins kept as 3-line wrappers, `fuzz_gamer` kept, only the 27 `_repl` bins deleted); parallelism is not an available win because `fuzz()` already runs `num_cpus::get()` threads with no shared mutable hot-loop state.

**Correction to this decision's premise:** the in-process path is **NOT** free of serialisation - `GameRequester` implements the same `api::Request`/`api::Response` contract and only drops the transport, `Request::Play` carries state as a JSON `String`, so every move already does a full state decode + encode plus a pub render, every player's state JSON and N+1 markup renders that the fuzz loop discards. The real gap is a process spawn plus a second JSON layer, not "serialised vs not".

**OPEN, needs Michael:** evaluation 4(d), the actual throughput project (keep the game live in memory, drive `Gamer` directly, delete the serde/render layer from the hot loop) - trades away incidental render-panic and serialise-panic coverage. Suggested shape: fast path by default, `--check-renders` for the thorough mode, plus one full `renders()` at game end. **Explicitly out of scope for WP-73.** See D-51, which persists this as future work.

**Rationale:** Michael's rationale, verbatim: "we want this to be as fast as possible to maximise the value of the fuzzer, the value of the fuzzer is basically directly correlated by how fast it can run and how many games it can pump through. I think we need to consider which approach is the absolute fastest over simplification or portability."

**Sources:** `decisions-session3.md`.

## D-44 Pivot to SSE now, not after hardening WebSockets

**Status:** COMMITTED.

**Decision:** Migrate to SSE now, or first complete WP-42's WebSocket hardening?

**Ruling:** **COMMIT to SSE. Migrate NOW, ahead of WP-42's WebSocket hardening.** Standing note for future agents: do not re-argue this as "axum supports both". That is true and beside the point. Consequence for the merged record: D-13's `/ws` hardening design is historical, not current work.

**Rationale:** Michael's reasoning: the 101-upgrade hijack is what forces WP-42 to hand-roll pre-upgrade auth; hardening that machinery and then deleting it is wasted effort - "I'd like to consider SSE now purely to avoid wasting effort in the immediate term." The framework argument, corrected and strengthened: the Tokio team (who also maintain axum) have announced a web app framework, **Topcoat** - https://github.com/tokio-rs/topcoat - and Michael **confirmed with them directly in an announcement thread** that Topcoat currently plans to support **SSE and not WebSockets**. The main Leptos maintainer has recently signalled diminished desire to keep developing it, so Michael assesses Leptos as carrying a **strong bus-factor risk**. Leptos has served brdgme well; the strategy is to watch for frameworks from significant, established teams as future options. The argument is about **not building on a transport that a likely future framework will not support** while the maintenance outlook for the current one is uncertain - a real motivation, not a tiebreaker.

**Sources:** `decisions-session3.md`.

## D-45 No `Last-Event-ID` replay

**Status:** RULED.

**Decision:** Should the SSE stream emit `id:` and honour `Last-Event-ID` for replay on reconnect?

**Ruling:** reconnect means "refetch everything visible". **Do not emit `id:`.**

**Rationale:** NATS Core has no replay, so `Last-Event-ID` would be a promise the server cannot keep. Michael: "happy with reconnect meaning refetch everything visible so we can keep implementation as simple as possible. Refetch shouldn't be super expensive."

**Sources:** `decisions-session3.md`.

## D-46 Connection topology: INVESTIGATE the two-stream option

**Status:** **Not yet decided** at time of writing - **RESOLVED by D-48**.

**Decision:** One SSE stream (`GET /events?game=<uuid>`, the evaluation's Option C) or two streams?

**Ruling:** **no ruling recorded in this item - it was flagged for investigation and is superseded by D-48 (TWO STREAMS).** Michael's substantive objection, recorded: a player may switch between public games within one SPA session, and under Option C changing the query param **reopens the single connection**, so the private/player-data stream drops and reconnects every time the user changes which public game they are watching, even though nothing about their private subscription changed. Proposed alternative: **two SSE connections** - a long-lived **private** stream that never drops for navigation, and a **public** stream swapped as the visible game changes. Claimed additional benefit: keeps the door open for further SSE uses, e.g. **chat**, without disturbing the private stream. To be weighed against the ~6-connections-per-origin HTTP/1.1 cap. **Resolve the HTTP/2 question as part of this** - which D-48 did.

**Sources:** `decisions-session3.md`.

## D-47 Cloudflare: Orchestrator's call, with a hard constraint

**Status:** RULED (delegated by Michael, decided by the Orchestrator).

**Decision:** How should rate limiting apply to the SSE endpoint at the Cloudflare edge?

**Ruling:** rate-limit **connection establishment** for `/events`, never stream duration or bytes streamed. Additionally require a **server-side heartbeat** (an SSE comment line) at an interval comfortably below any proxy idle timeout, so an idle stream is never reaped. The implementing spec must **verify the actual Cloudflare configuration and any proxy idle timeout rather than assuming**.

**Rationale:** Michael delegated the decision but set one non-negotiable requirement: **Cloudflare must not impose a timeout that closes the stream. The connection must stay open as long as the page is open.**

**Sources:** `decisions-session3.md`.

## D-48 RESOLVES D-46: browser leg is HTTP/2, so TWO STREAMS

**Status:** RULED, on a **measurement** (2026-07-26 by Michael, not inferred).

**Decision:** Given the measured transport, one stream or two?

**Ruling:** **TWO SSE streams.** `GET /events` - private, identity-scoped, opened once, **never swapped** on navigation. `GET /events/public?game=<uuid>` - unauthenticated, swapped as the visible public game changes; needs **no auth and no visibility predicate**, because public game ids are already public. That **surface reduction, not the reconnect cost, is the load-bearing argument.** **Hard cap, carried forward: never three held streams.** Future SSE uses (chat, notifications) must ride the existing private stream - pending confirmation from Michael that those uses are private/identity-scoped; a third independently swapped *public* feed would reopen this decision. WP-84 is unblocked; finalise it on the two-stream branch and delete the single-stream fallback shape.

**Rationale:** Measured: `curl -sI https://brdg.me | head -1` returns `HTTP/2 200`. The browser leg is HTTP/2 through the Cloudflare edge - the single fact `sse-topology-decision.md` made its recommendation conditional on, so the conditional resolves to its first branch. The ~6-connections-per-origin HTTP/1.1 cap does not bite over h2. **Dev remains permanently HTTP/1.1** (both Tilt modes plain HTTP, no TLS so no ALPN, axum has no `http2` feature for h2c); two streams of the ~6 h1 budget is comfortable, so this does not change the ruling, but any future stream count increase must be re-checked against dev, not just production.

**Sources:** `decisions-session3.md`.

## D-49 Future SSE uses: keep the door open, build nothing extra

**Status:** RULED; one sub-question left **not decided, deliberately**.

**Decision:** Are the hypothetical future SSE uses private, and does anything need building for them now?

**Ruling:** **build exactly the two streams of D-48. Add no topic machinery, no multiplexing layer, no channel registry.** The only thing to get right now is to avoid a shape that would need a redesign later. **The one cheap generalisation to consider:** `GET /events/public?game=<uuid>` bakes "the public stream is about exactly one game" into the URL; a near-free alternative is a **repeatable topic parameter**, e.g. `?topic=game:<uuid>`, where today the server accepts exactly one `game:` topic and rejects everything else - same behaviour, same code path, same one connection, but adding `tournament:<id>` or `chat:<channel>` later becomes additive. WP-84 should **evaluate** this and pick one; if the topic form costs materially more, take the `game` form - a URL shape is cheap to change later precisely because nothing persists it. Do not let this grow into a subscription protocol. Also worth noting: `event:` field naming on the private stream is what makes "one stream, multiple message types" work, so keep event names meaningful from day one rather than sending a single untyped message kind. **Not decided, deliberately:** whether multiple public topics eventually share one connection or get separate connections - revisit only when a second public use case is real; the D-48 hard cap (never three held streams) stands until then. (D-50 subsequently adopted the repeatable topic form.)

**Rationale:** Michael's hypothetical future uses, explicitly "hypothetical and may never happen" - none justifies building anything now: private chat messages; public chat messages (channels/threads, e.g. game-type specific); watching a live tournament (he would like tournaments), possibly including a live view of an elimination tree. His read of the likely shape: **one unified private stream carrying multiple message types**, and **potentially several public channels** (game, tournament, public chat) - so the "never three streams" cap survives on the private side but the public side may eventually need more than one topic. His stated constraint, verbatim: "I don't want to be over-engineering for a future that may never come, I'd just like us to be aware of potential future use cases which might help us avoid implementing something in an overly restrictive way."

**Sources:** `decisions-session3.md`.

## D-50 Public stream takes a REPEATABLE topic param; N games from day one

**Status:** RULED, with one item **to VERIFY, not assume**.

**Decision:** Does the topic architecture support watching several public games at once, and should the array form be adopted immediately?

**Ruling:** **`GET /events/public?topic=game:<a>&topic=game:<b>` - the same key repeated. Parse into a collection from day one. Accept N `game:` topics; reject every other topic kind.** Syntax notes: Michael's `?topic=game:<a>&game:<b>` is malformed - the second fragment has no key; the repeatable form repeats the key. **No `[]` suffix** - that is a PHP/Rails convention for explicit array-ness, repeated keys already carry it, and `[]` only adds percent-encoding noise. **Required with it:** **cap N** (an unbounded topic list is a cheap way to make one connection expensive - see D-52, cap = 16); **reject unknown or malformed topics** with an error rather than silently ignoring them, so a client bug surfaces immediately instead of appearing as a stream that quietly omits things.

**TO VERIFY, not assume:** axum's `Query` extractor over a `HashMap` collapses duplicate keys, so repeated params typically need `serde_qs` or a manual parse; **whether that holds at axum 0.8.9 must be checked against the real crate - it was flagged from general knowledge, not from reading the source.**

**Rationale:** Why N now, when D-49 says build nothing extra - these are different axes and only one is speculative: topic **kinds** (`tournament:`, `chat:`) are speculative per D-49, reject them; topic **count** (several games at once) is a plausible near-term product move (lobby/dashboard of live games) and the cost is `Vec<Topic>` instead of a scalar plus a fan-out loop. The trap being avoided: a single-game assumption does not stay in the URL - it leaks into the subscription bookkeeping and the fan-out path, and that is the expensive part to undo. Parsing into a collection from day one avoids it even if the UI only ever passes one topic.

**Sources:** `decisions-session3.md`.

## D-51 Maximum-performance fuzzer: FUTURE WORK, must be persisted

**Status:** OUT OF SCOPE for WP-73 and for this remediation effort; must be persisted durably.

**Decision:** Build the D-43 evaluation's 4(d) maximum-performance fuzzer now?

**Ruling:** **OUT OF SCOPE for WP-73 and for this remediation effort. Do not build it now. Do persist it properly** so it can be picked up later - Michael asked explicitly that the discoveries and ideas be written somewhere durable.

**The design he wants recorded** - three fuzzing modes, cheapest to most thorough: (1) **"Game logic only"** (default, maximum speed) - game kept live in memory, free of serialisation, no rendering; drives `Gamer` directly. (2) **Opt-in renders** - pub render **and all private renders after every successful command**; note this is stricter than the current loop, which builds renders every move but only via the request contract. (3) **Opt-in serialisation** - the "end to end fuzz", exercising the full `api::Request`/`api::Response` path as today. This supersedes the evaluation's simpler two-mode `--check-renders` sketch: the axes are **renders** and **serialisation**, and they are independent.

**What must survive into the durable record:** that the in-process path is **not** serialisation-free (every move already does a full state decode + encode, a pub render, every player's state JSON and N+1 markup renders, of which the loop uses only the acting player's `command_spec` and the opaque state string); that `fuzz()` is **already parallel** across `num_cpus::get()` threads so parallelism is not an available win; the two free wins in `Fuzzer::command` (a whole-`PlayerRender` clone to take one field, and a full state-string clone, both per move); the tradeoff that the current loop catches render-panics and serialise-panics for free and mode 1 gives that up, which is exactly why modes 2 and 3 exist; and the seven UNKNOWNs and their settling commands in `fuzz-throughput-evaluation.md` section 6 - **nothing here has been measured**, no cargo in a planning session.

**Home for it:** `planning/fuzz-throughput-evaluation.md` already holds the analysis and stays; add a `docs/BACKLOG.md` item so it is discoverable outside the review directory, which is otherwise archival, and cross-reference the evaluation from the item. (That item is #54; D-53 rules on where it sits.)

**Rationale:** Michael, verbatim: "Those suggestions for a maximum performance fuzzer sound excellent, but I totally agree it would be future work." And: "I love the idea of it being totally in memory, being free of serialisation, and not rendering as the 'game logic only' fuzz, but being able to opt into renders ... and maybe opting into serialisation as the 'end to end fuzz' would be cool too." The case for doing it eventually rests on his standing position from D-43: "the value of the fuzzer is basically directly correlated by how fast it can run and how many games it can pump through."

**Sources:** `decisions-session3.md`.

## D-52 WP-84 public topic cap = 16

**Status:** RULED (not derived from measurement - nothing was benchmarked in that session).

**Decision:** What is the cap on the number of public topics per connection?

**Ruling:** **16.** Michael confirmed the Worker's proposed number rather than changing it. Over the cap is a **400**, not a truncation (per WP-84), so a UI asking for too many fails visibly rather than silently dropping topics.

**Rationale:** Context that shaped it: `/events/public` is **unauthenticated** (D-48 - no visibility predicate, since public game ids are already public) and is **deliberately unmatched by any Cloudflare rate rule** (D-47 - navigation reopens it constantly and the free tier's fixed 10s period trips easily), so the cap is the only bound on what a single connection can ask the server to watch. 16 sits comfortably above any plausible single-screen game list while keeping that blast radius small. Note the asymmetry: **raising a cap later is backward compatible; lowering one breaks existing clients** - so 16 is the safe direction to be wrong in.

**Sources:** `decisions-session3.md`.

## D-53 `docs/BACKLOG.md` #54 goes in the "Then" tier, after #31

**Status:** RULED.

**Decision:** Does BACKLOG item #54 (maximum-performance fuzzer) go into the scheduled tier or the unscheduled post-go-live list?

**Ruling:** **promote #54 (maximum-performance fuzzer) into the scheduled tier** alongside #52, #50 and #15 - not the unscheduled post-go-live list. Sequencing note for whoever applies this: #31 (Rust-only repository) lifts `rust/` to root and reworks the workspace layout - the exact ground WP-73 and the fuzz bins sit on - so #54 sitting *after* #31 is consistent with that. Reminder on the file's own convention: `NN` is a permanent ID in assignment order and never implies execution order; priority lives only in the ordered list at the top of `docs/BACKLOG.md`, which is what this ruling edits.

**Rationale:** Michael's reasoning follows the compounding argument: a faster fuzzer makes every subsequent game port and every remediation package cheaper to validate, so front-loading it pays back across the rest of the work rather than being consumed once.

**Sources:** `decisions-session3.md`.

## D-54 WP-59 Task 14 carved out into its own work package (WP-85)

**Status:** RULED and already applied on the record.

**Decision:** Should WP-59 Task 14 stay inside WP-59?

**Ruling:** **CARVE Task 14 out of WP-59 into WP-85.** Done on the record: Task 14's body deleted from `specs/WP-59-inbound-processing-quality.md` (heading kept and marked `- CARVED OUT`, with a banner pointing at WP-85); the stale "D-15 IS STILL OPEN" gate removed and corrected, D-15 having been answered 2026-07-26; `specs/WP-85-email-parser-first-dispatch.md` written; and a `work-packages.md` entry added. WP-85 adds **0 findings** - `wfe F29` stays counted in WP-59, the same bookkeeping stance taken for WP-83.

**Rationale:** Michael, on reading Task 14: *"WP-59 Task 14 sounds like a risk, let's pull it out to a separate item if we can."* Task 14 was **written** as a documentation task - record the email dispatcher's reserved-verb set for game authors - but the work it actually implies is a **behaviour change** in `dispatch_email_command` (`rust/web/src/email/commands.rs`). That mismatch is the risk. WP-59 is otherwise executable end to end; coupling it to a behaviour change that cannot yet be executed would either block the whole package or invite an executor to land the wrong thing (documenting a reservation that D-15 has since decided should not exist).

**Supersession:** this changes the D-15 consequence recorded in BOTH `decisions-ANSWERED.md` and `decisions-needed.md` ("Unblocks WP-59 Task 14" / "WP-59 Task 14 is UNGATED but its content changes"). **WP-59 Task 14 is not live work** - it is WP-85, which D-55 defers.

**Sources:** `decisions-session3.md`.

## D-55 The email escape-hatch verb set is DEFERRED, deliberately

**Status:** DEFER - **WP-85 is DEFERRED - BLOCKED ON MICHAEL.**

**Decision:** Which verbs belong to D-15's small hard-reserved escape-hatch set?

**Ruling:** **DEFER. Do not design the escape-hatch verb set now. Its membership must not be invented.** Consequence: **WP-85 is DEFERRED - BLOCKED ON MICHAEL** and no executor may pick it up; deciding the escape-hatch set is a hard prerequisite, not a nicety. This is a **different** block from the `BLOCKED-ON-USER-RULES-REVIEW` park (WP-11, WP-12, WP-16, WP-20, WP-26, WP-30); do not fold WP-85 into that bucket. Also on the record: the acquire-1 / starship-catan-1 `end` collision - both games' top-level `end` move is unplayable by email today - stays **open** until WP-85 lands. That is an accepted cost of the deferral, not an oversight.

**Rationale:** Michael, in full: *"can we just defer that work? No games use those verbs yet, and I think I'd like the current version of brdgme in place a bit longer so I can get a feel for if and how we want to do this in the future."* D-15 settled the dispatch design - on game-scoped messages the game command parser runs FIRST and platform commands are the FALLBACK - but carved out a small hard-reserved set of escape-hatch verbs (`help` and equivalents) that always wins even on the game path; deciding *which* verbs belong to that set is the deferred part. The rationale is empirical rather than indecision: no game currently uses those verbs, so nothing is broken by waiting.

**Supersession:** this **materially qualifies D-15** as recorded in `decisions-ANSWERED.md` and `decisions-needed.md`, both of which state the live `end` defect "is fixed". It is **NOT** fixed - it stays open until WP-85 lands, and WP-85 is blocked on Michael.

**Sources:** `decisions-session3.md`.

## D-56 Friends-page select revert after a rejected change

**Renumbering note:** originally numbered D-41 in decisions-needed.md; renumbered to D-56 on merge to resolve a genuine ID collision with the session-3 D-41. Same decision, new number.

**Status:** ANSWERED by a Lead ruling (option B), so WP-54 is not blocked. Informs WP-54, WP-53.

**Decision:** A rejected invite-policy or game-visibility change on `/friends` keeps displaying the unsaved value until a page reload. The WP-54 review proved the draft's assumed fix (bump a refresh signal so the refetched overview re-syncs the control) CANNOT work: a rejected mutation returns identical data, `AttributeValue for bool::rebuild` skips equal values, `AnyView::rebuild` rebuilds in place on a matching TypeId so the `<select>` is never re-created, and `<option selected>` will not reassign a user-dirtied option. A real fix means converting both `<select>`s from per-`<option>` `selected=` to a `prop:value`-over-signal binding driven by an `Effect` (the shape `docs/CODING.md` around lines 305-310 - approximate, verify - already prescribes). A. absorb the conversion into WP-54 Task 2 (bigger markup diff in a file WP-54 otherwise only reads, but the user-visible behaviour is correct). B. ship WP-54 with the error message only; file the conversion against the `friends.rs` owner (WP-53 already touches the file). Recommendation: B.

**Ruling:** Lead ruling applied: **B.** WP-54 ships the error message and records the residual desync as EXPECTED in its manual checklist (step 7) so it is not later mistaken for a regression, plus cross-package item #7 routing the conversion onward. Overriding this to A only requires editing WP-54 Task 2; nothing else depends on the choice.

**Rationale:** The error message alone removes the silent failure (the user at least learns the change was rejected), and the binding conversion belongs with the other `friends.rs` component work.

**Sources:** `decisions-needed.md` (where it was numbered D-41, a late Group E addition).

---

# Standing constraints, global rules and process changes

All non-numbered content from both source extractions. Nothing here is optional
and nothing here is scoped to the single row that produced it.

## Authority

- `decisions-ANSWERED.md`'s authority statement, now inherited by this file:
  **where a ruling contradicts an older recommendation in `decisions-needed.md`,
  `work-packages.md`, a spec, or a checklist, THE RULING WINS.** This file is the
  implementer's reference.
- Planning session 3 (`decisions-session3.md`) **supersedes positions recorded
  elsewhere in the planning corpus.** That was the substance of the now-obsolete
  INCOMPLETENESS banner at the top of `decisions-ANSWERED.md`, and it survives
  here.
- `planning/landing-order.md` holds sequencing facts for the implementing agent.
  Package status flips are applied in `work-packages.md`.

## Five rulings that CHANGED a previously recorded position - do NOT follow the old text

| item | old position | new ruling |
|---|---|---|
| D-07 | option A: `--redact-private`, default ON for a user-facing export path | **OVERRULED** - no redacted export at all; the only export is the full bundle, admin-only |
| D-08 | restart path exempted, falling into D-05's dangling-name no-op + admin warning | **REFINED** - restart resolves a deprecated bot to the latest non-deprecated version |
| D-15 | "A-plus": keep a reserved-verb list, game-scoped override | **REDESIGNED** - game parser first, platform commands as fallback; only a small hard-reserved escape-hatch set (`help`) always wins |
| D-16 | option A: explicit Turnstile `render()` from a login-component effect | **OVERRULED** - option B: `/login` is a normal unrouted link forcing a full page load |
| D-37 | bare `{{` as the literal-brace escape | **CORRECTED** - `{{lbrace}}`; a bare `{{` cannot be implemented soundly |

Two further supersessions from session 3, recorded here because they are not in
the original five-row table: **D-54/D-55 supersede D-15's "the `end` defect is
fixed" and "WP-59 Task 14 is unblocked" text** (Task 14 became WP-85; WP-85 is
DEFERRED - BLOCKED ON MICHAEL; the defect stays open), and **D-44 makes D-13's
WebSocket-hardening design historical** (the transport pivots to SSE).

## STANDING PROCESS CHANGE (2026-07-26), from D-17 - binding on the WHOLE dependency group

Michael's strategy is to stay as close to latest dependencies as possible so they
never go stale. Therefore, for this and **ANY SIMILAR dependency problem**, the
FIRST step is: **"Upgrade all dependencies to latest and see where we stand."**
The problem may simply resolve. Only if it does NOT should the recorded
workaround (vendoring, pinning, feature-juggling) be taken. Apply this ordering to
WP-64, WP-65, WP-66, WP-67, WP-69, WP-70, WP-71, WP-72 and WP-73 alike: upgrade
first, then re-assess whether the workaround is still needed, and record what the
upgrade changed.

## The extracted standing-constraints list

These bind implementers beyond the single row that produced them:

1. **Dependencies (D-17):** for any dependency problem, upgrade everything to
   latest FIRST and re-assess. Only then take the recorded workaround. Applies
   across WP-64..WP-73.
2. **Macros (D-20):** Michael is wary of custom macros because of their
   maintenance and cognitive cost. **Keep any macro surface small and obvious, and
   PAUSE AND DISCUSS if a macro starts getting really complex.** He approved
   D-20's option B partly *because* it avoids macros - do not "simplify" it back
   into one.
3. **Parser (D-38):** WP-04 keeps the parser as straightforward and obvious as
   possible. It is complex but critical; reliability and maintainability beat
   cleverness, at every choice point.
4. **Sentry (D-18):** **no Sentry functionality may be lost** to the feature
   trim. Verify behaviour preservation, not just tree size. Enumerate the features
   in use before removing any. Preserve the deliberate native-tls transport.
5. **`lib/cost` (D-25):** the shared crate must gain a suitable amount of
   automated testing as part of the port.
6. **Parity park (D-35):** still in force **per game**. Only `a F1` and `b F7` are
   released from it, plus `e F30` conditionally (condition satisfied).

## Additional cross-cutting constraints embedded in individual rulings

- **No database queries (D-37):** WP-02 assesses the stored-content risk by
  reading code and migrations only, **never** by querying a database.
- **Verify the k8s feature flag (`bo F25`):** the implementer must confirm
  `k8s-openapi` actually ships a `v1_36` feature flag at fix time; if not, use the
  highest flag at or below v1.36 and record the choice in the WP-62 spec.
- **Escape-hatch verb set stays small (D-15):** keep the hard-reserved set small
  and obvious. Its membership is **DEFERRED** by D-55 and must not be invented.
- **Two visible unsubscribe links (D-10):** additional to, never a replacement
  for, the `List-Unsubscribe` headers.
- **Naming convention (D-20):** `rust/lib/game_bin`, package `brdgme_game_bin`.
  Do **not** use `game-bin` / `brdgme-game-bin`.
- **Terminology correction on the record (session 3):** Michael referred to
  `brdgme_game_bin` as "the game-bin macro". **There is no macro.** D-20 chose a
  generic crate parameterised over `Gamer` plus thin per-game wrappers, and he
  approved it partly *because* it avoids macros. **`brdgme_game_bin` must remain
  macro-free.**
- **Dependency package landing order (D-23):** land WP-69 **last** among the
  dependency packages so the `deny.toml` skip-list starts minimal.
- **`BLOCKED-ON-USER-RULES-REVIEW` outranks `BLOCKED-ON-DECISION` (D-35):** it
  clears only on Michael's per-game sign-off, never merely because a decision was
  answered.
- **WP-85's block is its own (D-55):** DEFERRED - BLOCKED ON MICHAEL is a
  different block from the parity park. Do not fold WP-85 into that bucket.
- **Re-read live files before applying drafted edits (N-5):** `docs/BACKLOG.md` is
  modified in the working tree; confirm the drafted item is accurate and correctly
  numbered first. Michael: "please ensure docs/BACKLOG.md is correct."
- **Amendment must state the rationale, not just the permission (N-4):**
  `BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md` are split **so bot difficulty can
  be tiered**, and **must not be folded into RULES.md**.
- **For the record (D-40):** Michael wants to revisit "game specific stats" in
  future **from a clean slate**.
- **Residual parked question (`b F4`):** the 7 Wonders **simultaneity** question
  (seat-order resolution against live state lets p+1 trade for a card p built the
  same turn) is recorded so it is not lost, parked for Michael's review, **NOT
  scheduled**.
- **Parked half (`e F30` / D-29):** "can an empty winning set win at all" stays
  PARKED.
- **No follow-up (`d F37`):** REJECTED, not a bug; no fix, no follow-up.
- **Never three held SSE streams (D-48):** a hard cap carried forward. Future SSE
  uses ride the existing private stream; a third independently swapped *public*
  feed reopens D-48.
- **Do not re-argue the SSE pivot (D-44):** "axum supports both" is true and
  beside the point.
- **Verify, do not assume, at the edge (D-47, D-50):** the implementing spec must
  verify the real Cloudflare configuration and proxy idle timeout, and must check
  axum 0.8.9's actual duplicate-query-key behaviour against the crate source.

## Group D banner - PARKED-PENDING-USER-RULES-REVIEW (2026-07-25), park CONFIRMED 2026-07-26

D-35 and every per-game item D-26, D-27, D-28, D-29, D-30, D-31, D-32, D-34 are
PARKED. **No implementing agent may pick up a package gated on them.**

The user's ruling, in two parts: (1) **Policy** (unchanged, still stands):
official rules are authoritative, and docs may be corrected. **But NO GAMEPLAY
CHANGE WITHOUT PER-GAME SIGN-OFF from the user.** (2) The whole question is parked
pending the user's own review of the game rules. Two reasons, both from the user:
(a) some `RULES.md` content was **AI-GENERATED AND MAY SIMPLY BE WRONG**, so it
cannot be trusted as the baseline for adjudicating "code vs docs"; and (b)
**EDITION AND VARIATION CHOICES ARE THE USER'S TO MAKE** - which printing of
Acquire, which Modern Art payout, which No Thanks player count are product
decisions, not review findings.

Concretely: do NOT change gameplay in any game crate under a parity finding; do
NOT "correct" a `RULES.md` to match official rules under these items either - the
doc may be the thing that is wrong, and the user is reviewing it. Packages gated
on these items are `BLOCKED-ON-USER-RULES-REVIEW` in `work-packages.md`; that
status is STRONGER than `BLOCKED-ON-DECISION`. **The park is lifted PER GAME, not
globally**, prioritising acquire-1, seven-wonders-1 / splendor-2, modern-art-2 and
red7-1.

**NOT parked - two carve-outs:**

- Mechanical liveness/correctness packages that were never parity-gated continue
  as normal: **WP-25** (modern-art-2 infinite busy-loop `d F34` + round-4
  soft-lock `d F35` - its own note already says "does not wait on the D items in
  WP-26"), **WP-15** (seven-wonders-1 `b F1`/`F2`/`F3`, including the reachable
  permanent soft-lock in the DrawDiscard resolver), **WP-10** (pub_state
  redaction - D-33 and D-35's redaction half are already ANSWERED option A and
  WP-10 stays READY; its heading mentions D-35 but its scope is hidden-info
  leakage, not rules parity), and the mechanical siblings **WP-19, WP-22, WP-23,
  WP-29**.
- The five egregious candidates, now individually ruled - see the next section.

## Explicitly NOT egregious - genuine edition/parity questions, leave parked

D-28 splendor tie-break (most vs fewest cards); all of D-30's player caps; all
three D-31 acquire items (the findings name later-Hasbro vs classic 3M/AH editions
explicitly, and bag-exhaustion "differs by edition"); D-32 jaipur (the (i) premise
is uncorroborated, (ii)/(iii) are adjudications); D-27 F5/F6/F8; and **ALL EIGHT
WP-11 items** (`f F2`, `F14`, `F15`, `F21`, `F33`, `F43`, `F50`, `F54`) - every one
is annotated "Go quirk"/"port-parity" and none produces invalid or asymmetric
state.

## Answering-order and preamble notes from `decisions-needed.md`

- Its own instruction to the user: "Each item is self-contained: context,
  question, options, recommendation. Answer with the item number and a short reply
  (e.g. "D-3: option B")."
- D-1..D-34 are the 34 items from REVIEW.md section 6; D-35..D-40 were surfaced
  during triage. Work packages in `work-packages.md` reference these as their
  unblock condition. (The renumbered D-56, originally D-41, is **not** covered by
  that sentence - it was a late Group E addition.)
- Suggested answering order as recorded: the security/data-integrity group (D-01,
  D-03, D-05, D-08, D-06) unblocks the top of the backlog; the parity group (D-35
  then D-26..D-34) can be answered in one sitting; the dependency group
  (D-17..D-25) mostly has obvious recommendations.

## Session index - what was answered when

**ANSWERED 2026-07-25, the 10 critical-path gating groups.** Two answers deviate
from the recommendation: D-05 (bots stay referenced by name) and D-12+D-14 (no
session expiry; email-change re-verification instead).

| Group | Items | Answer | Deviates? |
|---|---|---|---|
| 1 | D-01 | B - settings token + SPF/DKIM + remove account-security cmds from email | no |
| 2 | D-03 (+ D-04) | A - forbid undo once finished; yes to shared `undo_core`/`concede_core` | no |
| 3 | D-05 | C-lite MODIFIED - sweep + alert + Progress heartbeat, but bots stay by NAME; dangling bot names are a SUPPORTED no-op state + admin warning | yes |
| 4 | D-08 | C - validate at all 4 entry points AND at game start; reconciled with D-05 | no |
| 5 | D-06 (+ D-13) | A - gate details/feeds on participation-or-public; stats anonymize; rules public | no (D-13 label, see its section) |
| 6 | D-02 | A - at-least-once; marker after success; 5xx to force retry | no |
| 7 | D-12 + D-14 | A MODIFIED - fail closed in prod, expiring claims, windowed cap, revoke-all, but NO session expiry; email change requires re-verification | yes |
| 8 | D-33 (+ D-35) | A - counts/aggregates public, secrets in `player_state`; official rules authoritative on port-parity conflicts | see D-35 |
| 9 | D-36 | A - bounds-check player index at requester boundary + per-game validate hook | no |
| 10 | D-37 | A - error on non-empty `rest` + escape braces in `to_string` | no |

**REFINEMENTS 2026-07-25 (later session).** Four, each recorded in place: (1) D-01
command removal NARROWED; (2) D-01 `emails remove` confirmed removed, revert-path
caveat withdrawn; (3) D-13 now ANSWERED (option B shape), label ambiguity
resolved; (4) D-35, D-26..D-32, D-34 PARKED-PENDING-USER-RULES-REVIEW.

**ANSWERED 2026-07-26 - all remaining decisions.** Closed in that session: D-07,
D-08 (refined), D-10 (extended), D-11, D-14, D-15 (redesigned), D-16, D-17, D-18,
D-19, D-20, D-21, D-22, D-23, D-24, D-25, D-35 (park confirmed), D-37 (corrected),
D-38, D-39, D-40, plus the `bo F25` cluster rider and the six N-items. D-09 was
also confirmed (option B). The renumbered D-56 was already resolved by a Lead
ruling.

**Planning session 3, 2026-07-26.** Added D-41..D-55: the SSE pivot (D-44), the
two-stream topology measured as HTTP/2 (D-48), the repeatable topic parameter
(D-50), the reversal of the `_fuzz` deletion (D-43), the maximum-performance
fuzzer deferred to `docs/BACKLOG.md` #54 (D-51), the WP-85 carve-out (D-54) and
its deferral (D-55).

---

# Finding-level rulings

Non-`D` IDs, preserved as recorded. Canonical form from `decisions-ANSWERED.md`;
`decisions-needed.md` carries the same rulings as summaries inside D-26/D-27/D-29/
D-34 and in its egregious-rulings table. Where the historical "egregious
candidates" table in `decisions-needed.md` disagrees with these rulings - in
particular its `b F4` "asymmetric by player index" reasoning and its `d F37` "not
producible by any rulebook" claim - **THE RULINGS BELOW WIN.**

The park itself STAYS. These five rulings are the ONLY movement out of it.

## `a F1` - roll-through-the-ages-2, phase re-match after `keep_skulls` (WP-12)

**Status / Ruling:** **FIX NOW, outside the park.** As recommended. `roll()`
re-matches `self.phase` after `keep_skulls()` may already have advanced it. **The
rest of WP-12 stays parked.**

**Rationale:** Cross-player state corruption is in no edition: the previous
player's `roll()` decrements the **next** player's `remaining_rolls`. A Leadership
extra roll is silently skipped; an all-skull reroll cascades into `next_turn()`.
The crate's own `test_game_keep_skulls_all_disaster_leadership` asserts the
opposite outcome for the `next`-command path - identical states diverge on which
command reached them - so **the fix must adjudicate that test.**

## `b F4` - seven-wonders-1, trade simultaneity (WP-16)

**Status / Ruling:** **REMOVED from the egregious list and PARKED** under the
rules review.

**Rationale:** **Michael's correction is binding: 7 Wonders resources are NOT
depleted by trade** - they are printed on cards and both players use them, so
there is **no competition for a resource** and the "asymmetric advantage by seat"
framing recorded in the historical table was **WRONG**.

**Residual narrower question, recorded so it is not lost, parked and NOT
scheduled:** because players resolve in seat order against live state (
`execute_actions` resolves p0..pn sequentially against live-mutated
`cards`/`coins`), player p+1 can trade for a resource card player p built on that
same turn, which p could not have done in reverse. That is a **SIMULTANEITY**
question, not a scarcity one.

## `b F7` - seven-wonders-1, duplicate physical boards (WP-16)

**Status / Ruling:** **FIX NOW, outside the park.** Ensure **only one of each
physical board can be in play.** The rest of WP-16 stays parked.

**Rationale:** `cities()` lists all 14 A/B board entries and `start_game` shuffles
and takes the first `players`, so "Rhodes A" and "Rhodes B" can both be dealt.
Every printing has 7 boards with one side chosen each; 14 independent boards are
physically unreachable.

## `e F30` - red7-1, seat-order tie-break (WP-30)

**Status / Ruling:** **CONDITIONAL - and the condition is SATISFIED, so FIX NOW.**
The user's rule was: fix the seat-order tie-break only if the correct behaviour is
officially described or universally accepted; if resolving it needed a
**subjective judgement** on our part, park it. **It IS described, so no subjective
judgement is required.**

**Evidence (verified read-only 2026-07-26):** `rust/game/red7-1/DATA_DOCS.md`
states the second tie-break verbatim - **"Ties within a rule are broken by the
highest card in the winning set, then by the highest card overall in the
palette"** - and official Red7 rules agree (highest card in palette as the
ultimate tie-break; card value = number then colour, exactly what `Card::rank_key`
in `rust/game/red7-1/src/card.rs` already encodes as `(rank, suit ordinal)`).
`RULES.md` is silent, but the crate's own data doc plus official rules are enough.
The code simply never implements it.

**Cause:** `leader()` in `card.rs` only ever receives the already-filtered winning
sets (`lib.rs` pushes `rule_fn(&self.palettes[p])`), so the full palette is
unreachable from `leader()`. When all winning sets are empty, every `len()` is 0
and every `rank_key()` max is `(0,0)`, the strict `>` never fires (approx line
311, verify), and `leader_idx` stays 0 - the lowest surviving seat wins,
contradicting `DATA_DOCS.md`. Reachable via Green (`most_even_cards`) or Violet
(`most_cards_below_4`) with all-odd / all-rank-4+ palettes. Downstream: that
player survives `done`, and the `discard` pre-check lets the LOWEST-INDEX player
discard into a rule nobody satisfies.

**Fix:** fall through to the **FULL PALETTE's** `rank_key()` max, which requires
plumbing the unfiltered palette into `leader()`.

**Parked half:** the D-29 question - **"can an empty winning set win at all"** -
**STAYS PARKED.**

## `d F37` - modern-art-2, zero-card artist placings (WP-26)

**Status / Ruling:** **REJECTED - NOT A BUG. Do not "fix" this later. No fix, no
follow-up.**

**Rationale:** Michael: **this is the ACCEPTED WAY TO PLAY** - if only one artist
has cards, 2nd and 3rd go to the artists in order from the top. `suits()` in
`rust/game/modern-art-2/src/card.rs` returns the fixed enum-declaration order
`[LiteMetal, Yoko, ChristineP, KarlGitter, Krypto]`, and Michael confirms that
order **IS** the canonical top-to-bottom value-board order (Lite Metal top, Krypto
bottom). `end_round` in `rust/game/modern-art-2/src/lib.rs` scans `suits()` in
declared order with a strict `>`, so the first suit in that order wins among equal
counts - which **is** the correct behaviour. There is **NO** value-board-order-vs-
array-index discrepancy and no follow-up. The Go-parity caveat is moot: the
behaviour is intended, not inherited by accident.

**Historical mechanism, retained for context only:** `end_round`'s ranking loop
initialises `highest_count = -1`, so artists with zero cards on the table are
still awarded 2nd ($20) and 3rd ($10) when fewer than three artists had paintings
played (common, e.g. 5-0-0-0-0), and the values enter `value_board` and inflate
every later round. `modern_art.go` (approx lines 389-403, verify) is identical.
**Now REJECTED as not a bug.**

## `bo F25` - Kubernetes / `k8s-openapi` version pin

**Status / Ruling:** **ANSWERED - the deployed cluster is Kubernetes server
v1.36.0** (client v1.36.2, kustomize v5.8.1). **Pin `k8s-openapi` to the `v1_36`
feature.**

**Constraint:** the implementer **must confirm `k8s-openapi` actually ships a
`v1_36` feature flag at fix time.** If it does not, use the highest available flag
at or below v1.36 and **record the choice in the WP-62 spec.**

---

# N-items

New-issue IDs `N-1`..`N-6`, all ACCEPTED 2026-07-26, preserved as recorded. Only
`decisions-ANSWERED.md` carries the individual rulings; `decisions-needed.md`
names them only as a group.

## N-1 Stuck-bot sweep threshold and ack heartbeat cadence

**Status:** ACCEPTED.
**Decision:** What sweep threshold and ack cadence does WP-38 ship with?
**Ruling:** WP-38 ships with the **15-minute** stuck-bot-turn sweep threshold and
the **60s `AckKind::Progress`** ack-heartbeat cadence.
**Rationale:** Tunable config, not load-bearing on the design; revisit from
production data.

## N-2 zombie-dice-2 cup redaction shape

**Status:** ACCEPTED.
**Decision:** What replaces zombie-dice-2's draw-ordered cup in `PubState`?
**Ruling:** WP-10 replaces it with **`PubState::cup_counts: Vec<(Colour, usize)>`**.
**Rationale:** A bot-client-visible API shape change, not a persisted-state
change. Any redaction that closes the leak changes the shape; fixed
Green/Yellow/Red counts are the cleanest form and no bot can legitimately rely on
the leaked order today.

## N-3 `game_types.player_counts` resolution

**Status:** ACCEPTED.
**Decision:** Does the shared `game_types.player_counts` row union all versions'
counts, or take one version's?
**Ruling:** **newest-non-deprecated-version-wins**, not a union of all versions'
counts.
**Rationale:** A union would let roster validation accept a player count the
actually-selected version cannot run, since new games pick via
`find_latest_non_deprecated_game_version`.

## N-4 Strategy docs satisfy `RULES_AUTHORING.md`

**Status:** ACCEPTED.
**Decision:** Do the separate strategy files satisfy `RULES_AUTHORING.md`'s
mandatory "Strategy Tips" section?
**Ruling:** the separate `BASIC_STRATEGY.md` / `ADVANCED_STRATEGY.md`, surfaced
via `Gamer::basic_strategy` / `advanced_strategy`, **SATISFY** the mandatory
"Strategy Tips" section. **Amend `RULES_AUTHORING.md` accordingly.** Unblocks
WP-75.
**Rationale, which the amendment must STATE - not merely the permission:** the two
files are deliberately separate **so bot difficulty can be tiered.** Every bot
gets BASIC to stop it making game-throwing moves; only hard bots also get
ADVANCED. **They must not be folded into RULES.md.**

## N-5 `BACKLOG.md` item #53 (the parity park)

**Status:** ACCEPTED.
**Decision:** Apply the drafted `docs/BACKLOG.md` item #53?
**Ruling:** **ACCEPTED** - apply the drafted item #53 (the parity park).
**Rationale:** Michael added: "please ensure docs/BACKLOG.md is correct."
**Re-read the live `docs/BACKLOG.md` first** - it is modified in the working tree -
and confirm the drafted item is still accurate and **correctly numbered** before
declaring it ready. Do **not** fold into existing item #37; #37 is about
verification testing and is downgraded, so folding would hide the park.

## N-6 `CODING.md` Request-Path Invariants section

**Status:** ACCEPTED.
**Decision:** Apply the drafted `## Request-Path Invariants` section to
`docs/CODING.md`?
**Ruling:** **ACCEPTED** - apply the drafted **6-rule** `## Request-Path
Invariants` section **as drafted**, at the stated insertion point between
`## Rust: Error Handling` and `## Leptos: SSR and Hydration`.
**Rationale:** Each of the six rules is the root cause of a critical or major
finding; the insertion point was verified against the live file. Source:
`CODING-md-amendment-proposal.md`.

---

# UNKNOWN / malformed in the sources

Flags carried forward from the extractions. These are defects in the SOURCE
documents, recorded so they are not mistaken for facts about the code.

1. **`decisions-ANSWERED.md`'s top banner is WRONG about its own coverage.** It
   claims to cover "**D-01..D-34 ONLY**". Its table actually contains D-35, D-37,
   D-38, D-39, D-40, six `N-` items and six finding IDs, and does **NOT** contain
   D-1..D-6, D-12, D-13, D-26..D-34 or D-36. "All 34" refers to the **34 table
   rows**, not to the range `D-01..D-34`. **This false claim is why the
   D-35..D-40 "gap" looked real. There is no gap.**
2. **The same banner says session 3 added "D-41 through D-53".** `D-54` and `D-55`
   also exist in `decisions-session3.md`, appended later, and are not covered by
   the banner.
3. **Genuine ID COLLISION at D-41** - two unrelated decisions shared the number.
   Resolved on merge by Lead ruling: `## D-41` = the session-3 fuzz/repl binary
   deletion; the friends-page `<select>` decision renumbered to `## D-56`. **Any
   external reference to "D-41" written before this merge is ambiguous and must be
   read in context.**
4. **D-32 sub-item (i): MALFORMED, unfinished sentence in the source.** The
   recommendation reads "official Jaipur: the round LOSER deals but the
   WINNER... (verify - if confirmed loser-starts, fix and restore major)". The
   sentence is cut off. Its intent is UNKNOWN - do not reconstruct it; the
   rulebook must be checked at spec time.
5. **Eight items carry NO ruling, only a park plus recommendations written under a
   superseded policy:** D-26, D-27, D-28, D-29, D-30, D-31, D-32, D-34, all
   `PARKED-PENDING-USER-RULES-REVIEW`. Their recommendations were written under
   D-35 option C ("documented-in-crate wins"), which policy A superseded, and the
   park suspends acting on them regardless. The only movement is via the five
   per-finding egregious rulings. **No rulings have been invented for them here.**
6. **D-46 has NO ruling** - status "Not yet decided" at time of writing; resolved
   later by D-48 (two streams).
7. **D-43 has an OPEN sub-item needing Michael:** evaluation 4(d), the actual
   maximum-throughput fuzzer project. Also: the out-of-process slowdown is
   directionally established but **UNMEASURED in magnitude** - no cargo was run in
   a planning session, and `fuzz-throughput-evaluation.md` section 6 lists seven
   UNKNOWNs with their settling commands.
8. **D-49 has one sub-question left deliberately undecided:** whether multiple
   public topics eventually share one connection or get separate connections.
9. **D-50 carries an UNVERIFIED assumption:** that axum's `Query` extractor over a
   `HashMap` collapses duplicate keys, so repeated params need `serde_qs` or a
   manual parse. Flagged from general knowledge, **not from reading the source**;
   must be checked against axum 0.8.9 itself.
10. **D-42's heading was orphaned in the source.** It was restored 2026-07-26
    after having sat under D-51's "Home for it" section with no heading of its
    own, despite having nothing to do with the fuzzer.
11. **Structural oddity, not an error:** `D-35`'s section is placed BEFORE `D-26`
    in `decisions-needed.md` (deliberate, "answer first"), and
    `decisions-session3.md` is not in numeric order (D-52/D-53 sit between D-50
    and D-51; D-42 comes after D-51; D-54/D-55 appended last). This record is
    sorted ascending.
12. **`open-decisions-for-user.md` is a stub with zero decisions.** Reproduced in
    full so nothing is lost by folding it away:

    > # Open decisions for Michael - CLOSED
    >
    > All 34 decisions in this table were answered on 2026-07-26. The resolved
    > record - same row order, each row stating the ruling and its constraints -
    > is `decisions-ANSWERED.md`. Read that; there are no open questions here.

    Its only substance was the redirect, which this file now supersedes.
13. **Line numbers.** Every line-number reference reproduced in this file came
    from a source document and is **approximate, verify**. A prior audit of this
    corpus found 33-46% of citations wrong.
