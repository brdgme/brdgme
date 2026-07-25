# Critical path - brdgme Rust review

Scope: the 10 criticals plus the majors that are security / data-corruption /
liveness class and plausibly must ship with them. Everything else in
`work-packages.md` and `BACKLOG.md` is off this path.

- **48 critical-path findings** (10 critical + 38 major) across **21 work packages**.
- **11 packages ready to implement now** (finalized spec, no open decision).
- **1 package spec-gapped but unblocked** (WP-34).
- **10 packages blocked on 10 decisions** (D-1, D-2, D-3, D-5, D-6, D-8, D-12+D-14, D-33, D-36, D-37).

Path conventions: finding locations in `findings/` are relative to `rust/`.
Finding IDs are per-unit and restart at F1, so they are only unique with the
unit prefix (`lg`, `ls`, `a`-`f`, `ws`, `wd`, `wfe`, `bo`).

Verification note: units 1-9 have per-finding verdicts in
`findings/verification/`. Units 10-13 (web-domain, web-frontend-email,
bot-operator-tools, dependencies) were lead-verified in session with no
per-finding verdict field; those rows read `lead-verified`.

---

## 1. Critical inventory (10)

| ID | Unit | Description | Crate / paths | WP | Spec? | Verify |
|---|---|---|---|---|---|---|
| lg F1 | lib-game | `Space::parse` counts whitespace chars then byte-slices; a multi-byte space (NBSP) panics the server and the WASM suggest path | `rust/lib/game/src/command/parser/mod.rs:431` | WP-01 | yes | CONFIRMED |
| lg F2 | lib-game | `Token::parse` byte-length check lets a multi-byte first char be split by `&input[..t_len]` -> panic | `rust/lib/game/src/command/parser/mod.rs:50` | WP-01 | yes | CONFIRMED |
| lg F3 | lib-game | `Enum::parse` byte-slices a char-count `match_len`; a non-ASCII player name panics all command parsing and suggestion | `rust/lib/game/src/command/parser/mod.rs:641` | WP-01 | yes | CONFIRMED |
| ls F1 | lib-support | markup `slice()` byte-indexes char-count offsets; any multi-byte char in a canvas layer panics or corrupts output | `rust/lib/markup/src/transform.rs:274` (offsets from `src/ast.rs:201`) | WP-01 | yes | CONFIRMED |
| b F16 | games-batch-b | Alhambra `take()` pushes cards to hand even when not found in market; `take b1 b1` mints duplicate money cards | `rust/game/alhambra-1/src/lib.rs:570` | WP-14 | yes | CONFIRMED |
| d F34 | games-batch-d | Modern Art `settle_auction` skip-loop has no all-hands-empty guard: infinite busy-loop plus unbounded log growth, legally reachable in round 4 | `rust/game/modern-art-2/src/lib.rs:452` | WP-25 | yes | CONFIRMED |
| e F29 | games-batch-e | red7 `CardParser` checks char count then slices bytes; `play r€` panics the game service | `rust/game/red7-1/src/command.rs:31` | WP-01 | yes | CONFIRMED |
| wd F14 | web-domain | `undo_game` has no `is_finished` guard; reverts state but never rewinds `rating_change`, and the idempotency guard then blocks re-rating -> permanently wrong ratings | `rust/web/src/game/server_fns.rs:731`, `rust/web/src/db.rs:1407,1554` | WP-40 | **no** | lead-verified |
| wfe F1 | web-frontend-email | Settings route (and every unrouted `None` address) authenticated solely by the forgeable inbound `From` header; SPF/DKIM never consulted | `rust/web/src/email/inbound.rs:484,1111` | WP-56 | **no** | lead-verified |
| wfe F17 | web-frontend-email | `emails add/confirm/active/remove` reachable over that From-authed path -> full account takeover (redirects all turn mail and per-game reply tokens) | `rust/web/src/email/commands.rs:546` | WP-56 | **no** | lead-verified |

Both email criticals sit in the single unspecced, decision-blocked WP-56, and
the ratings critical in the unspecced, decision-blocked WP-40. The other seven
criticals are covered by finalized specs (WP-01, WP-14, WP-25) and are ready to
hand off today.

## 2. Also-in-scope majors (38)

### 2a. Auth, credential and privacy

| ID | Unit | Description | Crate / paths | WP | Spec? | Class | Verify |
|---|---|---|---|---|---|---|---|
| wfe F5 | web-frontend-email | Game-reply `From` check is forgeable; `email_token`s are the only real auth, composing with wd F26's token leak | `rust/web/src/email/inbound.rs` | WP-56 | no | auth-bypass chain | lead-verified |
| wd F26 | web-domain | `get_proposal` serialises every invitee's `email_token` (the inbound-email credential) to any authenticated viewer | `rust/web/src/proposals.rs` | WP-44 | yes | credential leak | lead-verified |
| ws F1 | web-server | Concurrent confirm requests race the per-code attempt cap (SELECT/check/UPDATE on the bare pool; the advisory lock covers only the send path) | `rust/web/src/auth/server.rs:354-390` | WP-34 | no | auth-bypass | ADJUSTED (critical->major) |
| ws F2 | web-server | Unverified `add_email_address` rows have no cap or expiry: squatting blocks the real owner's signup forever | `rust/web/src/auth/server.rs:789-831,428-436` | WP-35 | no | account denial | CONFIRMED |
| ws F16 | web-server | Hardcoded public fallback encryption key used silently when `DATABASE_ENCRYPTION_KEY` is unset (presence-only check) | `rust/web/src/crypto.rs:42-57`, `rust/web/src/main.rs:25-29` | WP-35 | no | crypto fail-open | CONFIRMED |
| ws F52 | web-server | Session cookies lack `Secure` in prod (`SECURE_COOKIE` set nowhere in the k8s manifests) | `rust/web/src/auth/session.rs:32-36`, `k8s/base/web/deployment.yaml:40-44` | WP-36 | yes | session-token flaw | CONFIRMED |
| wd F17 | web-domain | `get_game_details` ignores `game_visibility`; `is_game_visible_to_user` is dead code, so any authed user reads any game | `rust/web/src/game/server_fns.rs`, `rust/web/src/db.rs` | WP-47 | no | privacy gate unwired | lead-verified |
| wd F45 | web-domain | Three anonymous stats fns bypass `game_visibility`, naming private users | `rust/web/src/stats/mod.rs`, `rust/web/src/stats/queries.rs` | WP-47 | no | privacy gate unwired | lead-verified |
| f F1 | games-batch-f | zombie-dice `pub_state` clones the cup verbatim and draw order is real (front-drain) -> hidden info leaked to all players | `rust/game/zombie-dice-2/src/lib.rs:194,443` | WP-10 | no | hidden-info leak | CONFIRMED |
| f F13 | games-batch-f | for-sale selling-phase secret plays exposed via `PubState.bids` | `rust/game/for-sale-2/src/lib.rs:258,411-412` | WP-10 | no | hidden-info leak | CONFIRMED |

### 2b. Data corruption and silent loss

| ID | Unit | Description | Crate / paths | WP | Spec? | Class | Verify |
|---|---|---|---|---|---|---|---|
| ws F34 | web-server | `db::undo_game` clears no rating fields; a re-finished game keeps the voided ratings (same root cause as wd F14) | `rust/web/src/db.rs:1407-1463,1536-1680` | WP-40 | no | ratings corruption | CONFIRMED (strengthened) |
| wd F15 | web-domain | `undo_game` has no `updated_at` guard: a concurrent move is silently destroyed and all `undo_game_state` wiped | `rust/web/src/game/server_fns.rs:784`, `rust/web/src/db.rs:1416` | WP-40 | no | silent move loss | lead-verified |
| wd F16 | web-domain | `concede_game` checks `is_finished` on a snapshot; `db::concede_game` has no `is_finished`/`updated_at` guard -> real result clobbered | `rust/web/src/game/server_fns.rs`, `rust/web/src/db.rs:1284` | WP-40 | no | result corruption | lead-verified |
| wfe F19 | web-frontend-email | Email `run_concede` shares that race; rewrites places 1/2 over a finished game's real result | `rust/web/src/email/commands.rs:891`, `rust/web/src/db.rs` | WP-40 | no | result corruption | lead-verified |
| wfe F20 | web-frontend-email | Email `run_undo`: no `updated_at` guard, no `is_finished` check -> reverted moves plus permanently wrong ratings | `rust/web/src/email/commands.rs:966`, `rust/web/src/db.rs:1407` | WP-40 | no | data corruption | lead-verified |
| wfe F2 | web-frontend-email | Idempotency marker inserted before processing and the handler always 200s: any post-marker failure permanently drops a player's move or invite reply | `rust/web/src/email/inbound.rs:456` | WP-57 | no | silent data loss | lead-verified |
| bo F2 | bot-operator-tools | No ack-deadline `Progress` heartbeat: long turns get redelivered -> duplicate command submission | `rust/bot/src/main.rs` | WP-38 | no | duplicate mutation | lead-verified |
| b F17 | games-batch-b | Alhambra place indices diverge after placement (empty sentinels plus raw-vec vs filtered render numbering) -> wrong card acted on, reserve pollution | `rust/game/alhambra-1/src/lib.rs:664` | WP-14 | yes | game-state corruption | CONFIRMED |
| a F14 | games-batch-a | starship-catan unbounded buy/sell amounts overflow i32 from player input (debug panic, silent wrap in release) | `rust/game/starship-catan-1/src/command.rs:121`, `lib.rs:938,373` | WP-13 | yes | overflow / corruption | CONFIRMED |

### 2c. Liveness - permanent wedge, hang, outage

| ID | Unit | Description | Crate / paths | WP | Spec? | Class | Verify |
|---|---|---|---|---|---|---|---|
| ws F27 | web-server | Games reference bots by NAME; admin rename/delete/disable makes the worker silently skip-and-ack -> in-flight games deadlock permanently | `rust/web/src/admin.rs:159-181,200-207` | WP-38 | no | deadlock | ADJUSTED (minor->major) |
| wd F1 | web-domain | Bot command `UserError` acked with no re-publish; the bot stays on turn and the game is permanently wedged | `rust/web/src/game/mod.rs` | WP-38 | no | deadlock | lead-verified |
| wd F2 | web-domain | `MAX_TURN_ATTEMPTS` exhaustion logs and acks: wedged game, no durable signal | `rust/web/src/game/mod.rs` | WP-38 | no | deadlock | lead-verified |
| wd F3 | web-domain | `bot.turn` publish failure after DB commit is warn-only: bot on turn with no stream event | `rust/web/src/game/mod.rs` | WP-38 | no | deadlock | lead-verified |
| ws F53 | web-server | Bot command consumer spawned with the `JoinHandle` discarded; a clean `Ok(())` stream end is unlogged -> silent permanent bot outage | `rust/web/src/main.rs:55-74` | WP-39 | yes | outage | CONFIRMED (strengthened) |
| wd F4 | web-domain | `bot.command` consumer spawned once, never restarted on stream end or error | `rust/web/src/main.rs`, `rust/web/src/game/mod.rs` | WP-39 | yes | outage | lead-verified |
| bo F1 | bot-operator-tools | Reachable `unreachable!()` on the final retry attempt panics the spawned task, leaving the message unacked | `rust/bot/src/main.rs` | WP-39 | yes | outage | lead-verified |
| wd F27 | web-domain | Client-supplied bot slots unvalidated at 3 entry points -> an unrecoverably wedged game is creatable on demand | `rust/web/src/proposals.rs`, `rust/web/src/game/server_fns.rs`, `rust/web/src/db.rs` | WP-45 | no | mass-assignment / liveness | lead-verified |
| wfe F18 | web-frontend-email | `bot:<name>` opponents unvalidated in `classify_opponent`; `new chess bot:garbage` wedges a real game | `rust/web/src/email/commands.rs:59` | WP-45 | no | mass-assignment / liveness | lead-verified |
| b F2 | games-batch-b | seven-wonders DrawDiscard resolver: no queue-time filter, `take` is the only parser arm, status pins the turn -> permanent soft-lock | `rust/game/seven-wonders-1/src/lib.rs:410` | WP-15 | yes | deadlock | CONFIRMED |
| d F35 | games-batch-d | Modern Art round 4 can start on an empty-handed player with no legal command and no pass -> soft-lock (pairs with d F34) | `rust/game/modern-art-2/src/lib.rs:368` | WP-25 | yes | deadlock | CONFIRMED |
| c F22 | games-batch-c | cathedral `Box::leak` of 100 strings per parser construction, per command -> traffic-driven memory leak in the long-running service | `rust/game/cathedral-2/src/command.rs:26` | WP-21 | yes | OOM | CONFIRMED |
| lg F6 | lib-game | `Many` loops (typed, spec, suggest) have no zero-progress guard; a zero-width item parser loops forever | `rust/lib/game/src/command/parser/mod.rs:353` | WP-03 | yes | infinite loop | CONFIRMED |

### 2d. Request-reachable panics (same class as the criticals)

| ID | Unit | Description | Crate / paths | WP | Spec? | Class | Verify |
|---|---|---|---|---|---|---|---|
| lg F4 | lib-game | Exact `Enum` with multi-byte values can never match (chars vs bytes at the full-match check) - same char/byte defect as lg F1-F3 | `rust/lib/game/src/command/parser/mod.rs:622` | WP-01 | yes | char/byte | CONFIRMED |
| ls F2 | lib-support | markup `parse_u8`/`parse_usize` unwrap on overflow: malformed markup panics the process | `rust/lib/markup/src/parser.rs:54,79` | WP-02 | no | panic | CONFIRMED |
| ls F19 | lib-support | `.unwrap()` in the warp handler on the production HTTP path; bad game JSON panics | `rust/lib/cmd/src/http.rs:54` | WP-06 | yes | panic | CONFIRMED |
| c F29 | games-batch-c | sushizock steal `n = i32::MIN` overflows `len as i32 - n`; panic in overflow-check builds from player input | `rust/game/sushizock-2/src/lib.rs:460,502` | WP-21 | yes | panic | CONFIRMED |
| e F18 | games-batch-e | lost-cities-2 `player_state()` unchecked `hands[player]`, reachable via the request envelope | `rust/game/lost-cities-2/src/lib.rs:570` | WP-09 | no | panic | CONFIRMED |
| e F36 | games-batch-e | lost-cities-1 same unchecked `hands[player]` panic via a crafted PlayerRender | `rust/game/lost-cities-1/src/lib.rs:566` | WP-09 | no | panic | CONFIRMED |

### Deliberately excluded

- **WP-46 / wfe F31** (`FOR UPDATE SKIP LOCKED` no-op under autocommit `fetch_all`
  -> concurrent replicas double-send). Impact is duplicate outbound mail, not
  data loss or takeover, and WP-46 is a large 12-finding package. It shares
  **D-2** with WP-57, so answering D-2 unblocks it anyway - land it just off the
  critical path.
- All rules/scoring/port-parity majors, performance, error-classification,
  error-message, refactor, UX, test-coverage, admin-display, unsubscribe,
  address-parsing and dependency majors. See `work-packages.md`.
- **WP-02 caveat**: only `ls F2` is critical-path, but WP-02 is a large
  10-finding markup package gated on D-37. If D-37 stalls, the `ls F2` unwraps
  are separable from the escape-convention work.

---

## 3. Blockers - decisions the user must answer

Ten decisions gate ten critical-path packages. **Answer group A first** - it
holds both email criticals and the ratings critical.

### Group A - the criticals (answer first)

**D-1 - inbound email `From`-header auth. Unblocks WP-56 (wfe F1, wfe F17, wfe F5).**
Inbound email settings routes are authenticated only by the spoofable `From`
header, and every unrouted `None` address falls through to that handler.
`emails add/confirm/active/remove` run over this path, so a single forged mail
is a full account takeover - it redirects all turn mail and every per-game reply
token. SPF/DKIM results from Resend are never consulted. Both remaining email
criticals are this one defect.
Question: how is inbound email authenticated, and do account-security commands
belong on the email path at all?
- **A.** Per-user secret settings token in the reply address plus require Resend SPF/DKIM pass; keep email settings commands. Fixes spoofing but keeps the takeover surface.
- **B.** As A, but remove account-security commands from email entirely. Removes both criticals structurally; small product cost.
- **C.** Drop the email settings route; settings become web-only. Simplest, largest UX loss.
**Recommend B** - it eliminates the takeover class rather than just raising its cost.

**D-3 (+ D-4) - undo vs ratings. Unblocks WP-40 (wd F14, wd F15, wd F16, wfe F19, wfe F20, ws F34).**
`undo_game` on a finished game reverts state but never rewinds `rating_change`,
and the idempotency guard then refuses to re-rate the real outcome, so ratings
are permanently wrong. There is no `updated_at` guard either, so a concurrent
move is silently destroyed. `concede_game` has the same missing guards, and both
web and email paths duplicate the logic independently. Recomputing ratings from
scratch is known-unsound (it double-counts).
Question: may a finished game be undone at all, and if so how do ratings recover?
- **A.** Forbid undo once `is_finished`. Zero rating-math risk; loses the misclick escape hatch.
- **B.** Allow it, atomically rewind using the stored per-player `rating_change` deltas and clear them. Keeps the feature, carries rating-math risk.
- **C.** B within a short grace window (e.g. 5 minutes). Middle ground, more code.
**Recommend A** - the simplest guard that makes the corruption unreachable.
D-4 rides along (not separately blocking): put the guards once in `db.rs` and
share a `concede_core`/`undo_core` between the web and email paths - recommend yes.

### Group B - liveness (answer together; they compound)

**D-5 - bot-turn wedge recovery. Unblocks WP-38 (ws F27, wd F1, wd F2, wd F3, bo F2).**
Every bot-turn failure mode wedges a game permanently and silently: `UserError`
is acked without re-publish, retry exhaustion just logs, a publish lost after
the DB commit leaves the bot on turn with no stream event, and because games
reference bots by NAME an admin rename/delete/disable makes the worker
skip-and-ack. Long turns have no ack-deadline heartbeat, so they get redelivered
and the command is submitted twice.
Question: what recovery architecture?
- **A.** Reconciliation sweep - periodic "bot on turn > N minutes -> re-publish". Self-heals every mode including lost publishes; slower recovery.
- **B.** Per-error handling - re-publish on `UserError`, DLQ plus alert on exhaustion, transactional outbox. Faster, more moving parts.
- **C.** A plus B's DLQ and alerting.
Sub-questions: reference bots by id via migration, or keep names and warn on
rename? `AckKind::Progress` heartbeat, or just raise `ack_wait`?
**Recommend C-lite** - sweep plus retry-exhaustion alert, bots by id (migration),
Progress heartbeat: the sweep covers the modes you cannot enumerate.

**D-8 - bot-slot validation choke point. Unblocks WP-45 (wd F27, wfe F18).**
Client-supplied bot slots are unvalidated at four entry points
(`create_proposal`, `add_proposal_player`, `restart_core`, email `new`), so a
bogus or disabled bot name creates a game that wedges unrecoverably. This is the
supply side of D-5's problem: validation stops new wedged games while the
recovery machinery is built.
Question: where does the single validation live?
- **A.** One shared fn called at all 4 entry points. Good UX (early error), no invariant protecting future entry points.
- **B.** Validate at game start only (`start_proposal_tx`/`create_game_from_service`). A true choke point, but late user feedback.
- **C.** A plus B.
**Recommend C** - B is cheap once A exists and it holds the invariant for code not yet written.

### Group C - privacy and delivery semantics

**D-6 (+ D-13) - `game_visibility` scope. Unblocks WP-47 (wd F17, wd F45).**
The `game_visibility` setting and the `is_game_visible_to_user` predicate both
exist, but no read endpoint calls either: any authenticated user can read any
game's details, and anonymous stats endpoints name private users. The setting is
currently decorative.
Question: which endpoints does it gate, and do stats anonymize or filter private
users?
- **A.** Gate game details plus history/feeds; stats anonymize private users but keep aggregates.
- **B.** Gate details and feeds only; document stats as public. Cheapest, the setting stays partly hollow.
- **C.** Gate everything including filtering stats rows. Distorts head-to-head aggregates.
Also: make rules/game-info pages fully public? (recommend yes).
**Recommend A** - honours the setting without destroying aggregate stats.
**Answer D-13 (websocket firehose) with it** - it uses the same predicate, and
enforcing D-6 without D-13 leaks straight through the socket.

**D-2 - inbound webhook and sweep delivery semantics. Unblocks WP-57 (wfe F2), and WP-46.**
The inbound webhook inserts its dedupe marker before processing and always
returns 200, so any post-marker failure permanently drops a player's move or
invite reply with no retry and no trace. Every outbound sweep likewise marks
before doing. The svix webhook timeout is 15s.
Question: at-least-once or at-most-once, and process synchronously or enqueue?
- **A.** At-least-once - mark after success, 5xx on transient failure so svix retries, claim-then-send inside a real transaction. Cost: a rare duplicate email.
- **B.** Keep at-most-once, fix only the claim atomicity. Failures still drop silently.
- **C.** A plus enqueue - the webhook verifies, persists, 200s; a worker processes. Most machinery.
**Recommend A** now, escalating to C only if processing time approaches the svix timeout.

### Group D - lower-urgency critical-path blockers

**D-33 (+ D-35 first) - `pub_state` redaction shape. Unblocks WP-10 (f F1, f F13).**
zombie-dice serialises the shuffled cup in draw order, so every player can read
the next draws - a new bug not present in the Go original. for-sale leaks
selling-phase secret plays via `PubState.bids`. One redaction shape is needed
for all game crates.
- **A.** Counts-only / canonicalized public fields, with private detail re-added per player in `player_state` where entitled. Minimal serde change.
- **B.** A per-player private field in a unified state envelope. Cleaner long-term, bigger refactor.
**Recommend A**. `decisions-needed.md` puts D-33 in decision batch 2 and says
**D-35 (global port-parity policy) must be answered first** since it informs
D-26..D-34.

**D-36 - deserialized-state trust strategy. Unblocks WP-09 (e F18, e F36).**
`lib/cmd/src/requester/gamer.rs` deserialises stored and forwarded `Game` state
and player indices verbatim, and unchecked indexing panics exist across ~15 game
crates. Two of them (the lost-cities `player_state` panics) are request-reachable
today.
- **A.** Fix at the requester boundary - bounds-check the player index plus a `validate-after-deserialize` trait hook defaulting to no-op; per-crate panics become defence in depth.
- **B.** Per-crate defensive sweep only (~15 crates, no guarantee for future crates).
- **C.** Accept the risk (state comes from our own DB); fix only the two request-reachable panics.
**Recommend A** - one boundary check covers crates not yet written. Note e F18
and e F36 are both fixed by a single `gamer.rs` bounds check under any option.
Sequencing: if A, land WP-09 before the bulk of Phase 3 per-crate work
(`BACKLOG.md:163-167`); if B, fold WP-09's items into the per-crate packages.

**D-37 - markup literal-brace escape and unmatched-rest. Unblocks WP-02 (ls F2).**
Unmatched markup silently truncates output - the parser succeeds with the tail
in `rest` and callers discard it - and `to_string` emits raw text with no
escaping, so there is no round-trip and text can inject markup. Both need a
literal-brace convention.
- **A.** Error on non-empty `rest`, define an escape (`{{`), escape on `to_string`. Closes the injection hole.
- **B.** Error on non-empty `rest` only; document that text must not contain braces. Leaves the injection hole.
**Recommend A** with `{{`-style escaping.

Answering-order note: `BACKLOG.md` Phase 0 already scopes one session covering
D-1, D-3/D-4, D-5, D-8, D-6 and D-2 - that is Groups A-C above. D-33, D-36 and
D-37 (plus D-35 as D-33's prerequisite) sit in later batches and can follow.

---

## 4. Ready to implement now

Finalized spec in `planning/specs/`, no unanswered decision. Handable to a
cheaper implementing agent today.

| WP | Title | Spec file | Critical-path findings |
|---|---|---|---|
| WP-44 | proposals integrity + email_token leak | `WP-44-proposals-integrity-email-token-leak.md` | wd F26 |
| WP-01 | char/byte panic elimination | `WP-01-char-byte-panic-elimination.md` | **lg F1, lg F2, lg F3, ls F1, e F29** (4 criticals), lg F4 |
| WP-14 | alhambra-1 core fixes | `WP-14-alhambra-core-fixes.md` | **b F16** (critical), b F17 |
| WP-25 | modern-art-2 liveness and cleanup | `WP-25-modern-art-liveness.md` | **d F34** (critical), d F35 |
| WP-36 | crypto and deploy hardening | `WP-36-crypto-deploy-hardening.md` | ws F52 |
| WP-39 | bot consumer supervision | `WP-39-bot-consumer-supervision.md` | ws F53, wd F4, bo F1 |
| WP-15 | seven-wonders-1 mechanical fixes | `WP-15-seven-wonders-mechanical.md` | b F2 |
| WP-21 | cathedral-2 + sushizock-2 | `WP-21-cathedral-sushizock-fixes.md` | c F22, c F29 |
| WP-06 | lib cmd tools and http | `WP-06-lib-cmd-tools-http.md` | ls F19 |
| WP-03 | lib-game parser mechanical fixes | `WP-03-lib-game-parser-mechanical.md` | lg F6 |
| WP-13 | starship-catan-1 fixes | `WP-13-starship-catan-fixes.md` | a F14 |

Also ready, and **hard predecessors** of blocked critical-path packages - land
these while the decisions are pending:

| WP | Title | Spec file | Why it is on the path |
|---|---|---|---|
| WP-41 | db quality pass | `WP-41-db-quality-pass.md` | must land before WP-40 and WP-47 |
| WP-37 | admin pass | `WP-37-admin-pass.md` | must land before WP-38 |

### Landing-order constraints

| Constraint | Source |
|---|---|
| **WP-01 before WP-03** - WP-03 Task 3 edits lines WP-01 Task 3 rewrites in `Enum::parse`; all other WP-03 tasks are disjoint | `specs-LOG.md:754-755`; also the "LANDING ORDER" section of `specs/WP-03-lib-game-parser-mechanical.md` |
| **WP-41 before WP-40** - shared `concede_game`, `undo_game`, `apply_rating_changes`; WP-40 restructures them, WP-41 only deletes clauses | `specs-LOG.md:823-824` |
| **WP-41 before WP-47** - WP-47 adds callers of the visibility predicate WP-41 inlines a second copy of, with a mandatory cross-reference comment | `specs-LOG.md:825-826` |
| **WP-37 before WP-38** - shared `admin.rs`; D-5 will touch `delete_bot`/`update_bot` | `specs-LOG.md:877` |
| **WP-59 before WP-57** (Task 5 shrinks `handle_settings_reply_route`) and **before WP-40** (one-line `run_restart` map_err) | `specs-LOG.md:967` |
| WP-59 Task 1 is designed for zero overlap with WP-56 - either order is safe | `specs-LOG.md:963-966` |
| WP-39 is independent of D-5 and WP-38 - do not wait | `work-packages.md:322-323` |
| WP-44 is independent of the D-1 redesign - land immediately | `work-packages.md:363-366` |
| WP-25 does not wait on WP-26's decision items | `work-packages.md:219-220` |
| If D-36 picks the requester-boundary fix, land WP-09 before the bulk of Phase 3 per-crate work; WP-09 and WP-28 must be coordinated (overlapping request-reachable panics) | `BACKLOG.md:163-167`, `BACKLOG.md:72-73` |
| The `db.rs` module split (ws F42) is a new package that must land AFTER WP-35/40/45/47/49/50/52/53 land their `db.rs` edits | `specs-LOG.md:820-821` |

WP-59 is not otherwise critical-path but is a predecessor of WP-57 and WP-40;
it already has a finalized spec (`specs/WP-59-*.md`), so treat it as ready.

Suggested landing sequence for the ready set:
`WP-44 -> WP-41 -> WP-37 -> WP-01 -> WP-03 -> WP-39 -> WP-36 -> WP-14 -> WP-25 -> WP-15 -> WP-21 -> WP-06 -> WP-13 -> WP-59`.
Only WP-01/WP-03, WP-41-before-WP-40/47, and WP-37-before-WP-38 are true
ordering requirements; the rest is priority order.

---

## 5. Gap list - critical-path packages still needing a spec

| WP | Title | Critical-path findings | Total findings | Size | Notes |
|---|---|---|---|---|---|
| WP-40 | undo/concede TOCTOU + ratings integrity | **wd F14** (critical), wd F15, wd F16, wfe F19, wfe F20, ws F34 | 8 | **large** | Rating rewind, `*_core` extraction, three-file dedup. Blocked on D-3. Needs WP-41 (and WP-59) first. |
| WP-56 | email From-auth redesign | **wfe F1, wfe F17** (criticals), wfe F5 | 3 | **medium** | Two files; D-1 essentially fixes the design, so the spec is mostly mechanical once answered. |
| WP-38 | bot-turn wedge recovery | ws F27, wd F1, wd F2, wd F3, bo F2 | 6 | **large** | Four files, whole recovery architecture plus a bot-id migration. Blocked on D-5. Needs WP-37 first. |
| WP-09 | deserialized-state trust hardening | e F18, e F36 | 19 (+2 from WP-21) | **large** | ~15 crates plus the requester boundary; D-36 shapes everything. May split at spec time. |
| WP-02 | markup robustness and dedup | ls F2 | 10 | **large** | Nine markup files, escape convention open (D-37). Only `ls F2` is critical-path - consider carving it out. |
| WP-35 | auth edge semantics and fail-open | ws F2, ws F16 | 6 | **medium** | Four files; D-12 and D-14 carry four sub-calls each. |
| WP-34 | auth races and session mechanical | ws F1 | 9 | **medium** | **No decision blocks it** - specced any time; two files, one atomic-UPDATE race, rest mechanical. |
| WP-47 | game_visibility gates | wd F17, wd F45 | 2 | **small** | One predicate wired into game details plus stats. Blocked on D-6/D-13; needs WP-41 first. |
| WP-45 | bot-slot validation choke point | wd F27, wfe F18 | 2 | **small** | Four entry points, one shared validation helper. Blocked on D-8. |
| WP-10 | pub_state hidden-info redaction | f F1, f F13 | 2 | **small** | Two crates, one redaction shape once D-33 (after D-35) rules. |
| WP-57 | inbound webhook delivery semantics | wfe F2 | 3 | **medium** | One file, but **no AppState test fixture exists for the email handlers** - this will block coverage (`specs-LOG.md:959-960`). Needs WP-59 first. |

Off-path but D-2-adjacent: **WP-46** sweep delivery semantics (12 findings,
large, D-2 + D-11) - unblocked by the same D-2 answer.

Spec-writing budget: 4 large, 4 medium, 3 small. WP-34 (medium) can start
immediately; the other ten wait on their decision. WP-56 and WP-40 are the two
that hold criticals and should be specced the moment D-1 and D-3 land.
