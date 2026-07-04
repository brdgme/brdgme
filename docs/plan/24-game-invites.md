# 24: Game Invites

**Status:** Pending - post-go-live, non-blocking for cutover (added 2026-07-04)

**Problem:** creating a game today immediately calls the game service `New`
and drops every named opponent into a running game whether they want it or
not. Restart does the same. Players need to be invited first and be able to
accept or decline before the game exists.

**Sequencing:** after the Phase 16 cutover, like the Phase 22b-22d email
features. The in-app invite flow (proposals, accept/decline UI, start
policies) depends on nothing in Phase 22; the invite *emails* depend on 22a
(send path) and reuse the 22b `email_render` module and webhook machinery,
so build the email tasks after 22b. Recommended order: 22b → 24 core → 24
email tasks (interleaving with 22c/22d as convenient).

## Design

### Core model: proposals, not nullable games

The game service `New` call needs the final player count, and the whole
codebase assumes `games.game_state` exists. So an invite is a **pre-game
object**, not a game in a special status: new tables `game_proposals` +
`game_proposal_players`. A `games` row is only created when the proposal
starts, through the existing `create_game` path with the final roster.
Existing game queries, renders, and WS paths are untouched.

- `game_proposals`: `id`, `game_version_id`, `owner_user_id`,
  `restarted_game_id uuid NULL` (set when the proposal came from a
  restart), `status` (`open` | `started` | `cancelled`),
  `started_game_id uuid NULL`, timestamps.
- `game_proposal_players`: `proposal_id`, `position` (display order only -
  final seating is still shuffled at game creation, matching today),
  one of `user_id` / (`bot_name`, `bot_difficulty`),
  `response` (`pending` | `accepted` | `declined`), `responded_at`,
  `email_token` (for the 22b-style reply flow). The owner's own row and
  all bot rows are created as `accepted`.
- `game_players.has_accepted` (existing column, currently creator/bots
  only) is set from the proposal outcome at game creation - it becomes
  true for everyone in a started game, preserving its meaning for legacy
  readers.

### Flows

- **Create:** the new-game form becomes "send invites": pick game type,
  opponents, bot slots as today, but submit creates an `open` proposal.
  Solo-vs-bots games (no human invitees) skip the proposal entirely and
  create the game directly - nothing to accept.
- **Respond:** invitees see pending invites on the dashboard (and get an
  invite email, below); accept/decline server fns. Declines notify the
  owner (WS + email). Responses are terminal per invitee (no un-decline;
  the owner can re-invite by cancelling and re-creating).
- **Owner actions on a decline (or any time while `open`):** per declined
  slot - replace with a bot (name + difficulty picker) or remove the
  slot; or cancel the whole proposal. Every action is validated against
  `game_types.player_counts` - the UI disables choices that would make
  the roster count invalid at start time.
- **Start:** auto-start when the last pending invitee accepts (owner gets
  the normal your-turn/new-game notification instead of a separate
  "everyone accepted" mail). The owner may also **start early** while
  responses are outstanding: each still-pending slot is resolved by an
  owner-chosen policy - drop the slot or replace with a bot (one bulk
  choice with per-slot override), again validated against
  `player_counts`. Starting flips the proposal to `started`, records
  `started_game_id`, and runs the existing `create_game` +
  `trigger_bot_turns` path.
- **Cancel:** owner-only, any time before start; invitees who had
  accepted are notified.
- **Restart:** `restart_game` no longer creates a game directly. It
  creates a proposal owned by the restarter with the same human roster
  (all invited, restarter auto-accepted) and the same bot slots
  (auto-accepted, carrying name + difficulty - this subsumes the
  "bot restart limitation" bug in bugs.md; if that bug is fixed
  pre-cutover the fix is superseded here, otherwise close it via this
  phase). `restarted_game_id` links back; the old game remains finished
  and visible while the proposal is open.

### Email integration (after 22b)

- **Invite email:** sent to each human invitee on proposal creation,
  respecting `users.turn_emails_enabled` (22b opt-out) - rendered with
  the 22b `email_render` module, `Reply-To: i-{email_token}@play.brdg.me`
  (note the `i-` prefix vs 22b's `g-`, so the webhook can route without a
  table probe). Body: who invited you, game type, current roster,
  accept/decline instructions, browser link. Replying `accept` or
  `decline` resolves the invite via the 22b webhook; the sender-auth
  rules are identical (token authorises, From must match a verified
  address).
- **Unsubscribed invitees:** the invite is still created and visible
  in-app, but no email is sent - and the owner is warned. Surface it in
  two places: a badge on the invitee row in the proposal UI at creation
  time ("won't receive an email invite"), and in the creation response so
  the form can show it immediately. No email content is disclosed about
  *why* (just "may not see this until they next log in").
- **Decline / cancel / start notifications to the owner and invitees**
  reuse the same render module; all threaded on a per-proposal
  `Message-Id: <proposal-{id}@brdg.me>`.
- **Nudge + expiry (uses the 22c sweep task):** pending invitees are
  reminded once after `INVITE_REMINDER_AFTER` (default 3 days);
  proposals still `open` after `INVITE_EXPIRE_AFTER` (default 14 days)
  are auto-cancelled and the owner notified. Both env-configurable so
  dev can test in minutes.

### Out of scope (v1)

- Open/public invites ("anyone can join") and invite links.
- Using the legacy `friends` table for suggestions - the opponent picker
  stays as it is today.
- Counter-proposals (invitee suggesting a different game type/roster).

## Tasks

Core (no email dependency):

- [ ] Migration: `game_proposals`, `game_proposal_players` (+ indexes on
      `user_id, response` for the dashboard query and on
      `proposal_id`).
- [ ] Server fns: `create_proposal` (replacing direct creation for games
      with human invitees; direct path kept for solo-vs-bots),
      `respond_to_invite`, `replace_slot_with_bot`, `remove_slot`,
      `cancel_proposal`, `start_proposal` (early-start policies +
      `player_counts` validation + auto-start on last accept).
- [ ] `restart_game` rewired to create a proposal (roster + bots carried;
      close or supersede the bugs.md bot-restart item).
- [ ] Dashboard UI: "Invites" section (invites received: accept/decline;
      proposals sent: response status per slot, owner actions, start
      early, cancel). New-game form submits a proposal.
- [ ] WS: publish a skinny signal on proposal changes so open dashboards
      update live (subject/channel naming consistent with whatever
      Phase 17 has done by then - `game.{id}` equivalent for proposals).
- [ ] Tests (Phase 11 patterns): full lifecycle (create → mixed
      accept/decline → replace/remove → start), auto-start on last
      accept, early-start drop and bot-replace policies,
      `player_counts` validation rejects invalid rosters, cancel
      notifications, restart-proposal carries bots, non-owner actions
      rejected, double-response rejected.

Email (after 22b):

- [ ] Invite/decline/cancel/start emails via `email_render`; `i-` token
      routing in the webhook; `accept`/`decline` reply commands.
- [ ] Unsubscribed-invitee warning (creation response + proposal UI
      badge).
- [ ] Reminder + expiry in the 22c sweep task (`INVITE_REMINDER_AFTER`,
      `INVITE_EXPIRE_AFTER`).
- [ ] Tests: email-reply accept/decline (auth rejection cases as in 22b),
      reminder/expiry boundaries, no email to unsubscribed invitees.

## Open decisions (resolve before delegating)

- Whether declining is truly terminal or an invitee can change their mind
  while the proposal is open (spec above says terminal - confirm).
- Whether auto-start on last accept is wanted, or the owner should always
  pull the trigger (spec above says auto-start - confirm).
- Whether solo-vs-bots bypassing the proposal is right, or everything
  should go through the proposal for one code path (spec says bypass -
  confirm).
