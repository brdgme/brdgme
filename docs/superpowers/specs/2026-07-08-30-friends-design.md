# 30: Friends - Design

> Extracted 2026-07-08 from `docs/plan/30-friends.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Draft - brainstormed and scoped 2026-07-08. Post-go-live,
non-blocking.

**Problem:** the dominant brdg.me usage pattern is playing many games
with the same people over a long period, but the platform has no
concept of a relationship between users. Starting a new game means
typing opponent email addresses into blank slots every time; the only
streamlined path is restarting an existing game that already had the
right people in it. There is also no control over who can add you to a
game, and no way to see what the people you play with are up to.

## Prior art (audited 2026-07-08)

There is a `friends` table but it has **never had a feature behind
it**:

- The 2017 initial migration (`rust/api/migrations/20170326234713`)
  created `friends (id, created_at, updated_at, source_user_id,
  target_user_id, has_accepted boolean NULL, CHECK (target <> source))`.
- Legacy `rust/api` only ever had Diesel model structs (`Friend`,
  `NewFriend` in `db/models.rs`) and the schema entry - no queries, no
  endpoints. Schema-only, dead on arrival.
- `rust/web/migrations/001_initial_schema.sql` ports the table verbatim
  (plus indexes on both user-id columns) for prod-dump compatibility.
  No `rust/web` code touches it; a dead `friends.rs` was deleted in
  phase 07.
- Phase 24 explicitly deferred "using the legacy `friends` table for
  suggestions" - this phase is where that lands.

Consequence: the table is almost certainly empty in prod (verify with a
`count(*)` during implementation - a nonzero count means some ancient
pre-Rust writer existed and rows must be reviewed before the new
uniqueness indexes land). The shape is exactly what a simple
request/accept model needs, so v1 reuses it rather than replacing it.

## Crossover with #24 game invites

The two phases are **independent** (no shared tables, either can land
first) but touch the same surfaces; build order only changes where two
small pieces attach:

- **Invite policy enforcement** (D4) belongs at the server fn where
  human players are attached to a game, after emails resolve to user
  ids. Today that is `create_new_game`; #24 replaces direct creation
  with `create_proposal` for games with human invitees, and the same
  check moves/extends there. Whichever phase lands second inherits the
  choke point - one helper, called from both.
- **The opponent picker** (D6 suggestions) is the same form #24 turns
  into "send invites". The suggestion component is written once and
  survives the #24 rework unchanged (it fills slots; what submit does
  with the slots is #24's business).
- **Consent interaction:** pre-#24, being added to a game is
  non-consensual - `invite_policy = 'friends'` is the only shield, so
  this phase is arguably *more* valuable if it lands first. Post-#24,
  every invite is accept/decline anyway and the policy becomes a spam
  filter rather than a consent mechanism. Both orderings are fine.
- **Known gap shared with #24/#28:** adding an *unknown* email to a
  game auto-creates a user row (`db.rs` opponent-email path) - the same
  user-table-spam vector #28 D2 closes for login, and a user that does
  not exist yet cannot have a policy. Not fixed here; #24's proposal
  flow is the natural place to make add-by-email consentful. Noted so
  nobody expects `invite_policy` to cover it.

## Design

### D1 - reuse the `friends` table, request/accept semantics

Directed request row: `source_user_id` = requester, `target_user_id` =
addressee. `has_accepted`: `NULL` = pending, `TRUE` = accepted, `FALSE`
= declined. A friendship is **one accepted row per pair** (direction
irrelevant once accepted).

New migration (next free number at implementation time - #28 WP1
claims `005_login_confirmations.sql`):

- `UNIQUE (source_user_id, target_user_id)` (no duplicate requests).
- Unique index on `(LEAST(source_user_id, target_user_id),
  GREATEST(source_user_id, target_user_id))` - one row per pair total,
  so "are we friends" is a single-row lookup and A->B / B->A duplicates
  are impossible.
- `users.invite_policy text NOT NULL DEFAULT 'open' CHECK
  (invite_policy IN ('open', 'friends', 'none'))` (D4).

Lifecycle rules (all enforced in the server fns, kept deliberately
small):

- **Request:** creates a pending row. If a *reverse pending* row
  already exists (they already asked you), the request is treated as an
  accept - mutual intent, no awkward crossed-requests state.
- **Accept:** target sets `has_accepted = TRUE`.
- **Decline:** target sets `has_accepted = FALSE` and the row is
  *kept*. Subsequent requests from the same source are silent no-ops -
  the requester cannot distinguish pending from declined, which is the
  anti-harassment shield (no re-request spam, no "they declined you"
  signal). If the decliner later changes their mind and sends their own
  request, the existing row flips to accepted (both sides have now
  expressed intent).
- **Unfriend:** deletes the row (either side). Clean slate; a fresh
  request later is allowed. No notification.

### D2 - mutual friends, not follows

A one-way follow model does not support "only friends can add me to
games" (the whole point of the policy is that *both* sides opted in).
Mutual-with-request is also what the legacy shape was clearly designed
for.

### D3 - how you add friends

- **From game pages:** an "add friend" affordance next to each human
  opponent in the game meta panel (players list). This is the core
  flow - you friend the people you actually play with, no search
  needed.
- **From the friends page:** add by exact username (`users.name` is
  UNIQUE). Usernames are already public on game pages, so exact-name
  lookup is not an enumeration concern; unknown names error honestly.
- **Not by email** in v1: friend-request-by-email is an
  account-existence oracle (#28 mindset) and adds nothing the two
  flows above do not cover.
- When #29's `/players/:name` profile pages land, they get an
  add-friend button too (one-line tie-in, noted in tasks).

### D4 - invite policy

`users.invite_policy`, default `'open'`:

- `open` - anyone can add you to a game (today's behavior).
- `friends` - only accepted friends can include you.
- `none` - nobody can include you (you can still create your own
  games).

Enforced by one helper (e.g. `check_invite_policy(tx, creator_id,
target_user_ids)`) called from `create_new_game` after opponent
emails/ids resolve to users - so it covers add-by-email of an existing
account as well as picker selections - and later from #24's
`create_proposal`. Violations return an honest error naming the
blocked player ("X only accepts games from friends") so the creator
can fix the roster; the roster is the creator's to see, this is not an
information leak.

Broader privacy (game visibility, stats/profile visibility) is
**deliberately out of scope for v1**: #29's profile pages are public
by design, and gating them is a #29-side change to make against this
table if ever wanted. Scope creep here is the main risk to "simple".

### D5 - dashboard sections

The dashboard (`DashboardPage` in `app.rs`) is currently a stub -
these are its first real sections beyond #24's planned invites block:

- **Pending friend requests** (incoming: accept/decline; outgoing
  pending shown on the friends page only).
- **Friends' active games:** in-progress games with >= 1 friend that
  *you are not in*, linking to the game page - spectating works today
  (`get_game_details` renders the public perspective for non-players).
- **Friends' recent results:** last N finished games involving >= 1
  friend: game type, players, placings, finished date. This is the
  lightweight activity feed.

Plain fetch-on-load resources like the rest of the dashboard; no WS
live-update for friend events in v1.

### D6 - opponent picker suggestions

In the new-game form (and #24's proposal form later), each human slot
offers clickable suggestion chips instead of starting from a blank
email field:

- **Friends first** (most recently played with, then alphabetical),
  then **recent non-friend opponents** (distinct human co-players from
  your last ~20 games) - covering the pre-friendship case the
  "frequent opponents" idea targets, while friends stay the explicit,
  controllable tier.
- Deduped, excluding yourself and already-filled slots. Clicking a
  chip fills the slot with the *user id*; the free-text email input
  remains for everyone else.
- Server fn `get_opponent_suggestions()` returns `(user_id, name)`
  pairs. `create_new_game` gains an `opponent_ids` parameter alongside
  `opponent_emails` - `CreateGameSeed` / `create_game_with_users_tx`
  already plumb `opponent_ids` end-to-end (currently always empty), so
  the db layer needs no change.

### Friends page

New route `/friends`: friend list (with unfriend), incoming/outgoing
pending requests, add-by-username field, and the invite-policy
selector (no settings page exists yet; this is its natural first
home).

## Brainstormed use cases deferred to v2

- **Friend leaderboards:** per-game-type rating table among your
  friends only - far more meaningful than a global leaderboard on a
  small site. Natural once #29 lands (it builds the rating queries).
- **Rivalry links:** friends page linking each friend to #29's
  head-to-head / rivalry views.
- **Groups/crews:** named rosters ("Tuesday group") with one-click
  new-game/proposal pre-fill. Directly serves the "same friends for
  years" pattern if friends + suggestions proves insufficient; kept
  out of v1 because suggestions likely get 80% of the value for a
  fraction of the schema.
- **Friend-request notification email** via the 22b `email_render`
  machinery (v1 is in-app only - requests surface on the dashboard).
- **Presence / online indicators:** rejected, not deferred - wrong fit
  for async play, privacy-hostile, and needs infrastructure v1 does
  not.
- **Turn-nudge a friend:** redundant with 22c turn reminders.

## Open decisions (resolve before delegating)

- Decline semantics: silent shield as specced (requester sees nothing)
  vs notifying the requester - spec says silent, confirm.
- Default `invite_policy` for new users: spec says `'open'` (matches
  today's behavior); consider whether `'friends'` should become the
  default once #24's consent flow exists.
- Suggestion ordering: recency of shared games vs alphabetical within
  the friends tier - spec says recency, cosmetic either way.
