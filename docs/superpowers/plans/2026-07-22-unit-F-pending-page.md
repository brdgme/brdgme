# Unit F - Pending game page improvements (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/web` (Leptos SSR + WASM, Axum backend). This doc is
self-contained for the pending page.

The pending game page is the InvitePage / proposal-management page at
`/invites/:id` (`rust/web/src/proposals.rs:1567`). The user's eight
requirements are quoted inline below as R1..R8.

Requirements (verbatim intent):
- R1 Cancel button is destructive -> needs a confirmation dialog.
- R2 Owner can ADD a player using the SAME controls as the new-game form -
  ideally a SHARED component (`OpponentSlotEditor`, `new_game.rs:483`).
- R3 Remove "Start early". Instead, when a player is dropped, changed to a
  bot, or a new player is added, RESET currently-accepted players back to
  pending so they must accept again. If a previously-accepted player is
  reverted to pending, send them a NEW invite email worded like "the owner has
  made changes to the game; you need to accept again for the game to start."
- R4 Replace the "Drop / replace" dropdown with a simple remove link "(X)" with
  a confirmation dialog. "Replace" = remove + add; no special replace feature.
- R5 Owner can PASS OWNERSHIP to another human player via a "(make owner)" link
  with a confirmation dialog.
- R6 It must be possible to add/remove players down to an INVALID number of
  players, but show a warning and PREVENT starting while invalid.
- R7 The game must NOT auto-start when the last human accepts. Add an explicit
  "Start game" button. When the last human accepts, email the OWNER that the
  game is ready to start. (Reuse/align with unit A's ready-to-start predicate.)
- R8 Players can change an existing "accept" to "decline". "Decline" must ALWAYS
  have a confirmation dialog. A game cannot be started if ANY player has
  declined - the owner must remove them, then everyone must accept again.

---

## 1. Current behaviour

### Data model (`proposals.rs:23-108`, migration 015)

- `Proposal` (`:25`): `owner_user_id`, `status` (`'open'|'started'|'cancelled'`),
  `started_game_id`, `restarted_game_id`, `nudged_at`.
- `ProposalPlayer` (`:39`): `position` (i32), `user_id` (None => bot),
  `bot_name` (bot display name), `bot_difficulty` (bot type e.g. "medium"),
  `response` (`'pending'|'accepted'|'declined'`), `responded_at`, `email_token`.
- `ProposalView` (`:101`): `proposal`, `game_type_name`, `version_name`,
  `player_counts: Vec<i32>` (valid counts), `players: Vec<ProposalPlayerView>`,
  `viewer_role` (Owner/Invitee/Other, `:93`).
- `ProposalPlayerView` (`:80`): adds resolved `name` (username or bot name).
- `SlotAction { Drop | ReplaceWithBot }` + `SlotPolicy` (`:66-76`) - the
  start-early policy machinery to be REMOVED (R3/R4).

### Server fns / handlers

- `create_proposal` (`:881`): no human invitees => create game directly; else
  open proposal (owner+bots `accepted`, humans `pending` with a fresh
  `email_token = Uuid::new_v4().simple()`, `:991`), enforce
  `db::check_invite_policy_tx` (`:921`) + uniqueness (`:934-943`), then
  `mailer().send_invite(...)` per invitee (`:1029`). Owner/bot rows get
  `email_token = None`.
- `respond_proposal` (`:1044`): accept/decline. Guards: proposal `open`
  (`:1067`); caller is an invitee (`:1075`); **`me.response == "pending"`
  (`:1080`) - so today an accepted player CANNOT change to declined (R8 needs
  this loosened)**. On accept, if `count_pending_human_invitees_tx == 0`
  (`:1094-1097`) it **auto-starts** via `start_proposal_tx` (`:1106`) - R7
  removes this. Decline => `notify_owner_decline` (`:1127`); start =>
  `notify_started` (`:1125`).
- `start_proposal_early` (`:1148`): owner-only; resolves each pending slot per
  `SlotPolicy` (Drop => `delete_proposal_player`; ReplaceWithBot =>
  `convert_proposal_player_to_bot`, `:1205-1225`); requires no pending remain
  (`:1230`) + `roster_error` ok (`:1235`); then `start_proposal_tx`. **R3/R4
  delete this whole fn and its UI.**
- `cancel_proposal` (`:1262`): owner-only; status => `cancelled`;
  `notify_cancelled` to accepted invitees (`:1320`). No confirmation today (R1).
- `replace_proposal_slot` (`:1328`): owner-only; declined/pending human => bot;
  validates `roster_error`. **R4 removes this (replace = remove + add).**
- `remove_proposal_slot` (`:1411`): owner-only; **only declined/pending slots
  (`:1458`)** => `delete_proposal_player`; validates `roster_error`. R4/R6 need
  this generalized to ANY slot (incl. accepted) and to allow invalid counts.
- `get_proposal` (`:1488`): builds `ProposalView`; `viewer_role` from
  owner/invitee/other (`:1519`).
- `start_proposal_tx` (`:816`): builds the game from the **accepted** roster via
  `create_game_from_service` (`server_fns.rs:367`), links `restarted_game_id`
  (`:860`), flips proposal to `started` (`:869`). Caller owns tx + broadcast.
- `prospective_count` (`:1140`): count of `response != "declined"` (accepted +
  pending). Used by start-early/replace/remove roster validation.

### DB helpers (`proposals.rs`)

`find_proposal` (:386), `find_proposal_players` (:396), `find_proposal_roster`
(:409, resolves names), `insert_proposal` (:439), `insert_proposal_player`
(:457), `update_proposal_status` (:482), `update_proposal_player_response`
(:500), `delete_proposal_player` (:516), `convert_proposal_player_to_bot`
(:528 - sets `response='accepted'`), `count_pending_human_invitees_tx` (:731),
`lock_proposal_for_update` (:705), `find_proposal_players_tx` (:718).

### InvitePage view (`proposals.rs:1567-1900`)

- `InvitePage` (`:1567`): `use_params_map` -> `proposal_id`; reads
  `ProposalUpdate` WS context (`:1577`); per-proposal seq memo
  (`track_proposal_seq`, `:1550`); `LocalResource` on `get_proposal` (`:1583`,
  refetched on WS bump); `ServerAction`s respond/cancel/start_early/replace/
  remove (`:1592-1596`). Effects: navigate to `/games/{id}` on respond-start
  (`:1601`) and on start_early (`:1614`); navigate `/dashboard` on cancel
  (`:1621`); `bump_proposal_update` on replace/remove (`:1628-1641`).
- `ProposalDetail` (`:1665`):
  - Players list with `invite-status-{response}` class (`:1800-1816`).
  - "Your invite" Accept | Decline for a pending invitee (`:1818-1833`). Decline
    has NO confirmation today (R8).
  - Owner actions (`:1835-1876`):
    - "Start early - resolve pending slots": one `<select>` per pending slot
      (Drop / bot easy/medium/hard) (`:1716-1746`, `:1839-1860`) - R4 removes.
    - "Declined slots": "Replace with bot" | "Remove" links (`:1748-1779`,
      `:1862-1867`) - R4 removes replace; remove becomes "(X)".
    - "Cancel invite" link, no confirm (`:1869-1874`) - R1.
  - Per-action error `<Show>` blocks (`:1878-1898`).

### Emails (`proposals.rs:118-367`, `RealInviteMailer`)

Trait `InviteMailer` (`:119`): `send_invite`, `notify_owner_decline`,
`notify_cancelled`, `notify_started`. All resolve the recipient via
`fetch_invite_recipient` (`:148`) and gate on `invite_recipient_should_send`
(`:167`: has verified primary email + `invite_emails_enabled` + NOT suppressed
by web presence via `email::outbound::suppress_for_web_presence`,
`outbound.rs:42`). This per-recipient presence suppression was added by unit I-2
and MUST be carried into any new invite email.

- `send_invite` (`:184`): subject "{type} invite from {owner}", header
  "{owner} invited you to play {type}.", `you_can` = 'Reply "accept" to join,
  or "decline" to pass.', `is_first_message=true`, reply `i-{token}@brdg.me`.
- `notify_owner_decline` (`:236`): to owner, "{invitee} declined your invite.",
  `is_first_message=false`, reply `i-{proposal_id}@brdg.me`.
- `notify_cancelled` (`:280`): to accepted invitees, "The game invite was
  cancelled.".
- `notify_started` (`:323`): to accepted invitees, "The game has started!",
  `browser_url` = game URL.
- `EmailContent` blocks (`email/render.rs:15`): `subject, header, digest,
  board, you_can, browser_url, rules_url, footer`. Renderer
  `render_game_email` (`render.rs:86`).

There is NO "owner made changes, accept again" email yet (R3) and NO
"game is ready to start" owner email yet (R7).

### Ready-to-start predicate (unit A) - reuse for R7

- `db::find_pending_game_summaries` (`db.rs:657`): per open proposal,
  `is_ready_to_start` starts `true` and is set `false` if ANY human player
  (`player_user_id.is_some()`) has `player_response != "accepted"` (`:698`).
  Bots never block. **It does NOT check player-count validity** - the explicit
  Start button (R6/R7) must add that check on top.
- `PendingGameSummary.is_ready_to_start` (`game/server_fns.rs:39`); sidebar
  yellow-highlights an owner's ready game (`components/layout.rs:212-216`,
  `class:my-turn=needs_action`). Tests: `db.rs:3455`
  (`pending_summaries_roles_and_ready_to_start`).
- Note: because a player is exactly one of pending/accepted/declined, "all
  humans accepted" already implies "none declined". The Start button's extra
  guards beyond the sidebar predicate are: (a) valid player count (R6), and
  (b) explicit owner action (R7).

### Roster validation (R6)

- `roster_error(player_counts, player_count)` (`game/server_fns.rs:478`):
  `None` if `player_counts.contains(count)`, else "This game supports {counts}
  players, but the request has {n} (including you)". Pure fn, unit-tested
  (`:1548-1556`). Reuse its message for the invalid-count warning.
- `db::find_game_type_player_counts` (`db.rs:240`) returns the valid `Vec<i32>`;
  already surfaced on `ProposalView.player_counts`.

### Confirmation-dialog pattern (R1/R4/R5/R8)

There is NO custom modal/dialog component. The established codebase pattern is
the native browser confirm:
`web_sys::window().and_then(|w| w.confirm_with_message("...").ok())` then branch
on the bool. Used in `friends.rs:411,443`, `admin.rs:1204,1491,1854`,
`app.rs:528`, `components/game.rs:125,173` (concede / delete game). See section
2 for the recommendation.

---

## 2. Shared building blocks

### 2a. Confirmation dialog - recommend NATIVE confirm (reuse existing pattern)

- The codebase already uses `web_sys::window().confirm_with_message(...)` in 5+
  places for destructive actions (concede, delete game, block, delete bot).
  There is no modal infrastructure (no overlay/focus-trap/z-index layer in
  `main.scss` or `components/`).
- **Recommendation: reuse the native-confirm pattern.** Lowest rework, zero
  hydration risk (it is a client-side event-handler call, no SSR markup), and
  consistent with existing UX. Wrap it in a tiny shared helper to keep the
  message style consistent, e.g. in `components/mod.rs` or a new
  `components/confirm.rs`:
  `pub fn confirm(message: &str) -> bool { web_sys::window().and_then(|w| w.confirm_with_message(message).ok()).unwrap_or(false) }`
  (fails closed => no action, matching the existing inline `if confirmed { ... }`
  guards). This is a ~5-line unit.
- If the user wants a branded custom modal instead, that is a materially larger
  unit (overlay component, scroll lock, focus, ESC/cancel, hydration-safe
  always-present markup per `docs/CODING.md` "Structural vs attribute hydration
  mismatches"). Flagged as a decision (D-confirm).

### 2b. Shared player-input component - extract `OpponentSlotEditor`

Current coupling (`new_game.rs`):
- `OpponentSlotEditor` (`:483`) is a **module-private** `#[component] fn`. Its
  props are `i: usize`, `slots: ReadSignal<Vec<OpponentSlot>>`,
  `set_slots: WriteSignal<Vec<OpponentSlot>>`, plus two `LocalResource`s
  (`suggestions: Vec<OpponentSuggestion>`, `bot_names: Vec<String>`). It edits
  `slots[i]` by index.
- `OpponentSlot` enum (`:88`: `Player{query,selected} | Email(String) |
  Bot{name,bot_name}`) and `SlotMode` (`:79`) are also module-private.
- It calls `friends::search_users`, `friends::get_opponent_suggestions`,
  `game::server_fns::get_available_bots`, `generate_bot_name`. The `taken()`
  closure (`:532`) dedupes across the whole `Vec`.

What extraction needs:
1. Move `OpponentSlot`, `SlotMode`, and `OpponentSlotEditor` into a shared
   module (suggest `rust/web/src/components/opponent_slot.rs`, re-exported from
   `components/mod.rs`), making them `pub`.
2. Decouple the editor from "index into a `Vec`". The pending page needs a
   SINGLE "add one player" input, not N slots. Two viable APIs:
   - **(preferred) single-slot API**: the editor operates on one
     `OpponentSlot` via `get: Signal<OpponentSlot>` / `set: Callback<OpponentSlot>`
     (or a `RwSignal<OpponentSlot>`), and takes `taken: Signal<Vec<Uuid>>` so the
     parent controls dedupe. The new-game form then renders N of these over its
     `Vec`, computing `taken` from the other slots. This serves new-game (multi),
     pending-page add-player (single), and restart-with-editing (multi) uniformly.
   - (alt) keep the `Vec`+index API and have the pending page keep a 1-element
     `Vec`. Less clean; leaks the index concept into a single-add UI.
3. The two `LocalResource`s (`suggestions`, `bot_names`) are created by the
   parent (`GameBrowser` today). Keep that: the parent creates them and passes
   them in, so a single-add usage creates them once at the page level.

Coupling with Unit E (new-game stepped-route refactor + restart reuses the form):
- Both E and F want this exact component. **Whoever runs first should OWN the
  extraction (unit F2 below); the other reuses the result.** The extracted API
  must be designed for THREE consumers at once: new-game multi-slot form (E),
  restart-with-player-editing roster editor (E), and pending-page single
  add-player input (F). Designing it once for all three avoids a second refactor.
- **Recommended ordering:** land F2 (extraction) first as a small, behaviour-
  preserving refactor (new-game form keeps working unchanged), THEN do E's
  stepped-route/restart work and F's add-player work against the shared module.
  If the schedule puts E before F, swap ownership: E does the extraction, F2
  becomes a no-op "reuse" note. Either way: extract once, first; do not let E
  and F each fork the editor.

---

## 3. Implementation units (dependency order)

Each unit is one layer/concern, sized to stay well under the 150k budget.
Backend units (F3-F5) can land independently of the frontend (F6-F7) since the
existing UI keeps compiling against unchanged fns until F6/F7 rewire it - but
see the per-unit "ships dark" notes. Every unit ends with fmt + clippy green and
its own commit; DB-touching units add `#[sqlx::test]`s.

### F1. Shared confirm helper (frontend, ~5 lines)
- Goal: one consistent confirmation entry point for R1/R4/R5/R8.
- Files: new `rust/web/src/components/confirm.rs` (or add to
  `components/mod.rs`); export from `components/mod.rs`.
- Change: `pub fn confirm(message: &str) -> bool` wrapping
  `web_sys::window().confirm_with_message` (fails closed). No SSR markup.
- Acceptance: compiles under `--features ssr` and hydrate; clippy clean.
- Tests: none needed (thin FFI wrapper). If a custom modal is chosen (D-confirm),
  this unit grows and gains a hydration-safe always-present component + a
  Playwright hard-load smoke assertion.
- Depends on: nothing.

### F2. Extract `OpponentSlotEditor` into a shared module (frontend refactor)
- Goal: R2 enabler - a reusable player-input component (section 2b).
- Files: new `rust/web/src/components/opponent_slot.rs`; `new_game.rs` (remove
  the private defs, import the shared ones, adapt `GameBrowser` to the new API);
  `components/mod.rs` (export).
- Change: move `OpponentSlot`, `SlotMode`, `OpponentSlotEditor` to the shared
  module; switch to the single-slot API (`get`/`set`/`taken`) per 2b; update
  `GameBrowser` (`new_game.rs:432-447`) to map its `Vec<OpponentSlot>` onto N
  single-slot editors. Behaviour-preserving for the new-game form.
- Acceptance: new-game page works exactly as before (manual: pick player / email
  / bot, dedupe across slots). Existing `new_game.rs` unit tests still pass
  (`player_range`, `filter_and_sort`, etc.). SSR page test for `/games` stays
  green (`tests/ssr_pages.rs`).
- Tests: keep existing; add nothing page-specific here.
- Depends on: nothing. **Coordinate ownership with Unit E (section 2b).**

### F3. Roster editing with reset-on-change semantics + re-invite email (backend)
- Goal: R2 (add), R4 (remove any slot, no replace), R5 (make owner), R3 (reset
  accepted->pending on any roster change + "owner made changes" re-invite).
- Files: `rust/web/src/proposals.rs` (server fns + DB helpers + mailer);
  possibly a new migration `021_*.sql` ONLY if a schema change is needed (see
  gotchas - likely none: `email_token` already nullable, `position` exists).
- Change:
  - New `add_proposal_player` server fn (owner-only, open proposal): resolve a
    human by user id OR email (reuse `find_or_create_user_by_email_tx`, `:744`)
    or a bot; enforce `check_invite_policy_tx` + uniqueness against current
    roster (mirror `create_proposal:921-943`); insert at a fresh `position`
    (human => `pending` with a new `email_token`; bot => `accepted`); then run
    the reset (below); `send_invite` to a newly-added human.
  - Generalize `remove_proposal_slot` (`:1411`) to allow removing ANY slot
    (drop the `declined|pending` restriction at `:1458`); owner may remove down
    to an invalid count (R6) - so REMOVE the `roster_error` gate here (it now
    only blocks at Start, see F5). Optionally re-normalize `position` gaps.
  - Remove `replace_proposal_slot` (`:1328`) and `start_proposal_early`
    (`:1148`) and `SlotAction`/`SlotPolicy` (`:66-76`) (R3/R4). (Removing the
    "convert to bot" path: "change a player to a bot" in R3 is achieved by
    remove + add-bot; if we want an in-place convert, keep
    `convert_proposal_player_to_bot` but route it through the reset. See D-convert.)
  - New `transfer_proposal_ownership` server fn (R5): owner-only; target must be
    a HUMAN player (`user_id.is_some()`) in the roster; set
    `game_proposals.owner_user_id = target`. Decide interaction with target's
    acceptance state (D-owner-accept). Removing the old owner is a separate
    remove action; ownership transfer itself does not change responses.
  - **Reset-on-change core (R3):** after any add/remove(/convert), every
    currently-`accepted` HUMAN player is set back to `pending` and gets a fresh
    `email_token` (accepted rows have `NULL` tokens today - gotcha). Capture the
    set of reverted user ids BEFORE the update, then `mailer().send_invite(...)`
    each with the new "owner made changes" wording (new mailer method, below).
    Bots stay `accepted`. The owner's own row: decide whether the owner is reset
    (D-owner-reset). Declined rows are untouched by the reset (they block start
    until removed, R8).
  - New mailer method `notify_changed_reinvite(proposal_id, user_id, token)`
    (or extend `send_invite` with a `reason` enum): same gating as `send_invite`
    (presence suppression + opt-in), `is_first_message=false`, reply
    `i-{token}@brdg.me`, header along the lines of "The owner has made changes
    to the game. Accept again for the game to start." (exact wording D-wording).
- Acceptance: add/remove/transfer fns enforce owner-only + open-status; a roster
  change resets accepted humans to pending and emails them; invalid counts are
  ALLOWED at edit time (no error). 
- Tests (`#[sqlx::test]`, real Postgres): reset flips accepted->pending and
  preserves bots/declined; add-player inserts pending human with token /
  accepted bot; remove works on an accepted slot and allows invalid count;
  transfer rejects a bot target and a non-player; re-invite gate truth table
  (extend the `invite_gate` helper at `:1931`).
- Depends on: nothing backend-side; the re-invite email is self-contained.

### F4. Explicit Start + ready-to-start owner email + start guards (backend)
- Goal: R7 (no auto-start; explicit Start; owner "ready to start" email) and the
  start-time guards for R6 (valid count) and R8 (no declined).
- Files: `rust/web/src/proposals.rs`.
- Change:
  - `respond_proposal` (`:1044`): REMOVE the auto-start block (`:1091-1109`).
    On accept, instead of starting, check whether this accept made the proposal
    "ready" (all humans accepted, none declined, count valid) and if so
    `mailer().notify_owner_ready(proposal_id)` (new). Keep `notify_owner_decline`
    on decline.
  - New `start_proposal` server fn (owner-only, open): lock; require NO human
    `pending` and NO `declined` (R8) and `roster_error(player_counts,
    prospective_count) == None` (R6) - return descriptive errors otherwise; then
    `start_proposal_tx` (`:816`); `notify_started` to accepted invitees;
    broadcast. (This replaces `start_proposal_early`'s start path without the
    policy machinery.)
  - New mailer method `notify_owner_ready(proposal_id)`: to the OWNER, gated like
    the others (decide presence suppression - D-ready-presence), header e.g.
    "Everyone has accepted - your {type} game is ready to start.", `browser_url`
    = invite URL, `is_first_message=false`, reply `i-{proposal_id}@brdg.me`.
  - Align the "ready" test with unit A's predicate (`db.rs:698`: all humans
    accepted) PLUS the count guard, so the sidebar highlight and the Start
    button agree. Consider extracting a shared
    `proposal_ready_to_start(players, player_counts) -> bool` helper used by
    both `find_pending_game_summaries` and `start_proposal`/`respond_proposal`
    (keeps them from drifting).
- Acceptance: accepting the last human no longer starts the game; owner gets the
  ready email; `start_proposal` refuses with a clear message when count invalid
  or any declined/pending; on success it creates the game and notifies.
- Tests (`#[sqlx::test]`): respond-accept does not start; ready email fires once
  when the last human accepts (and not before); `start_proposal` rejects
  invalid-count / declined / pending and starts when valid + all accepted.
- Depends on: F3 (reset semantics define what "all accepted" means after edits).

### F5. Accept <-> decline toggle (backend)
- Goal: R8 - a player who accepted can switch to declined (and the existing
  pending->accept/decline stays).
- Files: `rust/web/src/proposals.rs`.
- Change: in `respond_proposal` (`:1044`), relax the `me.response == "pending"`
  guard (`:1080`) to allow `pending -> accepted|declined` AND
  `accepted -> declined`. Keep `declined` terminal (a declined player cannot
  self-reaccept; the owner must remove + re-add them, per R8). On
  accepted->decline, fire `notify_owner_decline` (owner now sees a decline again)
  and re-evaluate "ready" (it un-readies the game; no owner email needed for
  un-ready, or optionally a "X declined" note - D-unready-email). The decline
  confirmation itself is client-side (F7).
- Acceptance: an accepted player can decline; a declined player cannot re-accept
  via this fn; owner is notified on a fresh decline.
- Tests (`#[sqlx::test]`): accepted->declined transitions; declined->accepted is
  rejected; pending->accepted still works.
- Depends on: F4 (shares the respond fn and ready logic).

### F6. Pending-page OWNER controls (frontend)
- Goal: wire R2 (add player), R4 ((X) remove + confirm), R5 (make owner +
  confirm), R1 (cancel confirm), R7 (explicit Start button + invalid-count
  warning) into `InvitePage`/`ProposalDetail`.
- Files: `rust/web/src/proposals.rs` (view + actions); maybe `main.scss` for the
  warning style (reuse `.form-error`).
- Change:
  - Add ServerActions for `add_proposal_player`, `transfer_proposal_ownership`,
    `start_proposal`; drop the start_early/replace actions and their effects
    (`:1594-1595`, `:1614-1619`, `:1628-1634`).
  - Owner section: a single shared `OpponentSlotEditor` (from F2) bound to a
    local `RwSignal<OpponentSlot>` + an "Add player" button that dispatches
    `add_proposal_player` (mapping Player/Email/Bot like `new_game.rs:255-271`);
    `taken` = current roster user ids.
  - Per-player row: an "(X)" remove link (all slots) gated by `confirm(...)` (F1)
    -> `remove_proposal_slot`; a "(make owner)" link on human non-owner rows
    gated by `confirm(...)` -> `transfer_proposal_ownership`.
  - Cancel link gated by `confirm(...)` (R1).
  - Explicit "Start game" button (owner) -> `start_proposal`; show an
    invalid-count warning (reuse `roster_error` message client-side from
    `pv.player_counts` + current roster size) and disable/Hide-when-invalid is a
    structural risk - keep the button always present and show the warning text +
    let the server reject, OR toggle a `disabled` ATTRIBUTE (not a structural
    swap) per `docs/CODING.md`. Show start errors inline.
  - Keep WS bump-on-change effects for the new actions.
- Acceptance: owner can add/remove/transfer/cancel with confirmations; Start
  button starts when valid + all accepted; invalid count shows a warning and
  cannot start; hydration stays clean (no conditional element swaps on async
  data).
- Tests: SSR page test `/invites/:id` stays 200/no-panic (`tests/ssr_pages.rs`);
  Playwright hard-load smoke (zero console errors) per `docs/CODING.md` 11.6.
- Depends on: F1, F2, F3, F4.

### F7. Pending-page INVITEE controls (frontend)
- Goal: R8 UI - accept<->decline toggle with an ALWAYS-on decline confirmation.
- Files: `rust/web/src/proposals.rs` (view).
- Change: in "Your invite" (`:1818-1833`), show Accept and Decline for a pending
  invitee (decline gated by `confirm(...)`, F1); for an invitee who already
  accepted, show a "Decline" affordance (gated by `confirm(...)`) ->
  `respond_proposal(accept=false)`; a declined invitee sees a "you declined"
  state (no self-reaccept). Keep the navigate/bump effect for respond.
- Acceptance: decline always prompts; accepted players can decline; declined
  players cannot re-accept from the UI.
- Tests: SSR page test stays green; Playwright smoke.
- Depends on: F1, F5.

Suggested commit order: F1, F2, F3, F4, F5, F6, F7 (F6/F7 may land together).
Push deferred to a final cleanup unit per the orchestrate handover rules.

---

## 4. Decisions for the user

1. **D-confirm - confirmation UX:** native browser `confirm()` (recommended;
   matches existing concede/delete/block UX, zero hydration risk, ~5 lines) vs a
   branded custom modal (much larger: overlay, focus trap, ESC, hydration-safe
   markup). All four confirms (cancel/remove/make-owner/decline) follow the
   choice.
2. **D-wording - re-invite email (R3):** exact subject/header text for "the
   owner has made changes to the game; you need to accept again for the game to
   start." Proposed header: "The owner has made changes to the game. Accept
   again for the game to start." Confirm wording + whether it threads on the
   existing `proposal-{id}` thread (proposed: yes, `is_first_message=false`).
3. **D-reset-scope - who gets reset/re-emailed on a roster change (R3):** reset
   ALL currently-accepted humans (simplest, matches "reset the currently-accepted
   players"), or only players affected by the specific change? And do we re-send
   to players who were ALREADY pending (proposed: no - only newly-reverted
   accepted players get the re-invite; already-pending players keep their
   original invite)?
4. **D-owner-reset - does the owner reset themselves?** The owner is auto-
   accepted. On a roster change, is the owner also reverted to pending (and
   emailed), or does the owner stay accepted (they are driving the change)?
   Proposed: owner stays accepted (no self-reinvite).
5. **D-bot-reset - do bot changes trigger a reset (R3)?** Adding/removing a BOT
   changes the roster - does that reset accepted humans too? R3 literally says
   "a new player is added" / "changed to a bot" => reset. Proposed: any roster
   mutation (incl. bots) resets, for consistency. Confirm.
6. **D-convert - "change a player to a bot" (R3):** implement as remove + add-bot
   (R4 says replace = remove + add, so no in-place convert), or keep an in-place
   `convert_proposal_player_to_bot` path that also triggers the reset? Proposed:
   remove + add only; delete `convert_proposal_player_to_bot` usage.
7. **D-owner-accept - ownership transfer vs acceptance (R5):** if ownership
   moves to a player who is currently PENDING, does the new owner need to accept
   (they would block ready-to-start until they do), or does the transfer
   auto-accept them? Also: can the old owner be removed afterwards (leaving the
   new owner)? Proposed: transfer does not change responses; the new owner must
   accept like anyone else; old owner can be removed by the new owner.
8. **D-ready-email - "ready to start" owner email (R7):** exact wording, and does
   it respect web-presence suppression like the other automated invite emails
   (unit I-2 added per-recipient suppression)? Proposed: yes, suppress if the
   owner is active on the web; header "Everyone has accepted - your {type} game
   is ready to start." Also: send the ready email exactly once when the last
   human accepts (re-fire after a reset re-completes)?
9. **D-unready-email - notify on un-ready?** If a player declines after the game
   was ready (R8), does the owner get a "X declined" email (the existing
   `notify_owner_decline` covers this) - confirm we reuse it and do NOT send a
   separate "no longer ready" note.
10. **D-invalid-count - bots and the warning (R6):** do BOTS count toward the
    invalid-player-count warning? `roster_error`/`prospective_count` count all
    non-declined slots incl. bots (proposed: keep counting bots, matching
    `create_proposal`). Confirm the warning is shown but editing is still allowed.
11. **D-position - position normalization:** after add/remove, should `position`
    gaps be re-normalized (start order follows `position`)? Proposed: re-normalize
    on every roster mutation to keep start order stable. Low-risk; confirm.
12. **D-reinvite-token - fresh token on reset:** reverting accepted->pending
    requires a fresh `email_token` (accepted rows are NULL). Confirm we mint a new
    token per reset (old tokens, if any, are discarded).

---

## 5. Known issues / gotchas (carry forward to every Lead)

- **Migrations are immutable.** Never edit an applied `.sql` file (sqlx checksum
  break - happened with 005 on 2026-07-11). New schema work => a NEW numbered
  file; next number is **021** (`020_drop_user_last_seen_at.sql` is current max).
  F3 is expected to need NO migration (email_token/position already exist) - if
  one is needed, it must be 021+.
- **SQLX_OFFLINE=true for clippy/check.** Canonical gates (DEV.md):
  `cargo fmt --all -- --check`;
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo clippy --workspace --exclude web --all-targets -- -D warnings`;
  `cargo test -p web --features ssr` (needs live Postgres). New plain
  (`sqlx::query`/`query_as`) queries avoid `.sqlx` regeneration (CODING.md
  "Plain (non-macro) sqlx queries") - prefer plain queries for any new column
  touch, matching the existing proposal helpers.
- **clippy `--all-targets` gate is mandatory** - a prior unit left it red; never
  commit with outstanding fmt/clippy.
- **DB tests need real Postgres.** Plain local runs fail DB tests (pre-existing,
  backlog #40 - not a regression). Use `scripts/rust-test.sh` (spins up temp
  Postgres+NATS, ports 15432/14222/18222) or the long-lived
  `brdgme-test-{pg,nats}-47116` containers. `#[sqlx::test]` gives each test an
  isolated migrated DB.
- **Pre-existing flake:** `invite_expiry_threshold_defaults_to_14_days` (env
  race) - do not chase it as a regression.
- **Hydration safety** (`docs/hydration.md`, CODING.md "Leptos: SSR and
  Hydration"): the InvitePage uses a `LocalResource` (safe default). For any new
  resource/Suspense or conditional rendering, keep layout outside `Suspense`,
  bind the resource read unconditionally first, and never swap element STRUCTURE
  on async data - toggle attributes/classes (e.g. `disabled`, `hidden`,
  `class:`) instead. The Start button's invalid-state and the confirm dialogs
  must not introduce structural SSR/client divergence. Native `confirm()` is
  client-only and hydration-safe.
- **No panics** in handlers/components (CODING.md): `NodeRef::get()` is `Option`;
  `web_sys::window()` is `Option` - the confirm helper fails closed.
- **Email gating:** every new invite email MUST reuse `fetch_invite_recipient` +
  `invite_recipient_should_send` (verified primary email + `invite_emails_enabled`
  + web-presence suppression via `suppress_for_web_presence`), per unit I-2.
- **`games.updated_at` is trigger-maintained** (CODING.md "Database"): any
  `UPDATE games` bumps it; irrelevant to proposal edits but relevant if Start
  touches `games`.
- **`respond_proposal` is shared** by F4 and F5 - land F4 first, then F5 amends
  the same fn, to avoid conflicting edits.
- **Org is `brdgme`** (not `beefsack`) for any image/URL references.
