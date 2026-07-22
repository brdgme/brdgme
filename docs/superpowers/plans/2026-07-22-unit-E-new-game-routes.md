# Unit E - New-game stepped routes + restart-with-editing (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/web` (Leptos SSR + WASM, Axum backend). Read
`docs/superpowers/plans/2026-07-22-unit-F-pending-page.md` for the template
this doc follows. This doc is self-contained for the new-game routes and
restart work.

Unit E's requirements (verbatim intent from the session handover):
- E-req1 New-game stepped routes: `/games/new` -> `/games/new/{type}`. The user
  selects a game TYPE first, then configures players on a second page.
- E-req2 Restart reuses the new-game form PREFILLED (players by username from
  the finished game). The owner can edit the roster before re-proposing.
- E-req3 Race handling: only the FIRST restart of a finished game is accepted;
  a second (concurrent or later) restart gets an error that LINKS to the invite
  page (if the first restart opened a proposal) or the restarted game (if the
  first restart created a game directly).
- E-req4 Decline re-enables restart: if a player declines the restart proposal,
  the restart option becomes available again on the original finished game.
- E-req5 Reuses the extracted `OpponentSlotEditor` (unit F2,
  `components/opponent_slot.rs`).

Note: units F1-F5 are already landed in the working tree (the
`OpponentSlotEditor` extraction, `confirm()` helper, `add_proposal_player`,
`start_proposal`, `proposal_ready_to_start`, accept<->decline toggle, and the
`notify_changed_reinvite`/`notify_owner_ready` mailer methods all exist in the
current code). This plan builds on that current state.

---

## 1. Current behaviour

### New-game flow (single page today)

- Route `/games` -> `NewGamePage` (`new_game.rs:79`) -> `GameBrowser`
  (`new_game.rs:101`). It is a SINGLE page with a two-column layout
  (`.new-game-layout`):
  - Left (`.new-game-browser`): a filterable/sortable game-card grid. Data from
    `get_available_game_types` (`game/server_fns.rs:373`) ->
    `db::find_available_game_types` (`db.rs:282`, returns each game type with its
    public, non-deprecated versions; types with no live version are dropped).
    Client-side `filter_and_sort` (`new_game.rs:38`) does player-count filter,
    text search, and alpha/weight sort. `GameTypeInfo` shape
    (`game/server_fns.rs:120`): `id, name, player_counts, weight, blurb,
    versions[{id,name}]`.
  - Right (`.new-game-panel`): the setup panel, rendered only once a game type
    is selected (`selected_type_id` signal). Contains: a version `<select>` (if
    >1 version), a "View rules" link, player-count radios
    (`gt.player_counts`), then one `OpponentSlotEditor` per OPPONENT slot
    (`player_count - 1`; the viewer is the implicit first player). Submit button
    "Start game".
- `?game=<name>` preselect (`new_game.rs:178-194`): an `Effect` matches the
  query param against the type list case-insensitively and calls `select_game`
  once. This is the existing "deep-link to a game type" mechanism (GameInfoPage's
  "Start a game" link uses it, `game_info/mod.rs:122` -> `/games?game={name}`).
- Opponent slots track `player_count - 1`, resized not rebuilt on count change
  (`new_game.rs:159-162`). `taken` (`new_game.rs:112-124`) is the set of user
  ids already claimed by a slot, used to dedupe suggestions/search.
- Submit (`on_submit`, `new_game.rs:220-247`) folds the slots into
  `(version_id, ids, emails, bots)` and dispatches `create_proposal`
  (`proposals.rs:1036`). The navigate effect (`new_game.rs:210-218`) goes to
  `/games/{id}` (solo-vs-bots, game created directly) or `/invites/{id}`
  (proposal opened).
- `OpponentSlot` (`components/opponent_slot.rs:18`): `Player { query, selected:
  Option<(Uuid, String)> }` (the `String` is the USERNAME), `Email(String)`,
  `Bot { name, bot_name }`. `SlotMode` (`:9`) = Player/Email/Bot. The editor is
  single-slot: props `get: Signal<OpponentSlot>`, `set: Callback<OpponentSlot>`,
  `taken: Signal<Vec<Uuid>>`, plus two `LocalResource`s (`suggestions`,
  `bot_names`) created by the parent. **Prefilling a human by username maps
  directly to `OpponentSlot::Player { query: "", selected: Some((id, name)) }`;
  a bot maps to `OpponentSlot::Bot { name, bot_name }`.**

### Restart flow today (instant, no editing)

- UI: `GameMeta` (`components/game.rs:26`) shows a "Restart" link when
  `can_restart = is_finished && restarted_game_id.is_none()` (`:34`). Clicking
  dispatches the `RestartGame` `ServerAction` (`:44`, `:134-141`); a navigate
  effect (`:66-74`) goes to `/games/{id}` or `/invites/{id}` from the outcome.
  When `restarted_game_id.is_some()` it shows "Go to new game" (`:142-148`);
  `previous_game_id` (the reverse link) shows "Previous game" (`:149-155`).
- `restart_game` server fn (`game/server_fns.rs:886`) -> `restart_game_impl`
  (`:713`):
  - Guards: game `is_finished` (`:724`); `restarted_game_id.is_none()` (`:727`,
    else "Game has already been restarted"); caller is a player (`:730`).
  - Restarts onto the latest non-deprecated version of the game type
    (`find_latest_non_deprecated_game_version`, `:742`), falling back to the
    original version.
  - Re-attaches the EXACT old roster: `opponent_ids` = the other humans (`:748`),
    `bot_slots` from `game_bots` (`:753`), `player_count = game_players.len()`
    (`:763`). Validates `roster_error` (`:769`). Re-checks invite policy
    (`check_invite_policy_tx`, `:780`).
  - Solo-vs-bots (`opponent_ids.is_empty()`, `:787`): `create_game_from_service`
    (`:421`) then `UPDATE games SET restarted_game_id = new_game WHERE id =
    old_game` (`:803-810`), all in one tx. Returns
    `ProposalOutcome { game_id: Some(..), proposal_id: None }`.
  - With humans (`:822`): `insert_proposal(.., restarted_game_id = Some(old_game))`
    (`:823`; `insert_proposal` at `proposals.rs:537` already takes the
    `restarted_game_id`), owner accepted, humans `pending` with fresh
    `email_token`, bots accepted. The old->new game link is NOT written here -
    it is written when the proposal STARTS (`start_proposal_tx`,
    `proposals.rs:1002-1009`). Returns `ProposalOutcome { proposal_id: Some(..),
    game_id: None }`.
  - `restart_game` (the server fn, `:886`) then broadcasts and emails invites to
    pending humans (`:911-925`).
- The email command `restart` (`email/commands.rs:983`, dispatched at `:1083`)
  calls `restart_game_impl` DIRECTLY (instant restart over email - there is no
  form over email). This path must keep working.

### The race condition (E-req3)

The `restarted_game_id.is_some()` guard is read at `:727` BEFORE the transaction
begins (`:773`). Two concurrent restarts of the same finished game can both pass
this check:
- Solo-vs-bots: both pass the guard, both open a tx, both `UPDATE games SET
  restarted_game_id`. The second `UPDATE` silently overwrites the first link
  (last writer wins) and creates a second orphan game. No error to the loser.
- With humans: the old->new link is only written on proposal START, so the guard
  (`restarted_game_id.is_some()`) does not even fire for a second restart while
  the first proposal is still open. Two players can each open a restart proposal
  for the same finished game; both proposals carry `restarted_game_id =
  old_game`. When the first starts, it links the old game; the second proposal
  is left dangling and, if it later starts, would re-link/overwrite.

Fix direction (section 2c): serialize restarts on the old game row with
`SELECT ... FOR UPDATE` inside the tx, and treat "an OPEN proposal carrying
`restarted_game_id = old_game`" as an in-flight restart that blocks a second one.

### "Decline re-enables restart" today (E-req4)

`respond_proposal` (`proposals.rs:1200`) allows pending->accepted,
pending->declined, and accepted->declined (`:1235-1238`); declined is terminal.
On decline it only fires `notify_owner_decline` (`:1273-1275`). `start_proposal`
(`:1288`) refuses to start with any declined player (`:1331-1336`).

So today, if a restart proposal exists and a player declines:
- The proposal stays `open` with a declined player; it can never start.
- The old game's `games.restarted_game_id` is still NULL (link only written on
  start), so `can_restart` is still TRUE and "Restart" still shows - but
  clicking it would create a SECOND dangling proposal (the race above). There is
  no notion of "an active restart attempt blocks restart" and no "decline clears
  it".

E-req4 design (section 2d): a game is restartable iff `is_finished &&
restarted_game_id.is_none() && NO open restart-proposal exists`. A decline on a
restart-proposal AUTO-CANCELS that proposal, clearing the block so the game is
restartable again.

### Routing (`app.rs:215-232`)

Leptos `<Routes>` with `StaticSegment`/`ParamSegment`. Relevant existing routes:
- `/games` -> `NewGamePage` (`:219`).
- `/games/type/:name` -> `GameInfoPage` (`:227`).
- `/games/:id` -> `GamePage` (`:228`).

leptos_router ranks routes so a STATIC segment outranks a PARAM segment at the
same position. Therefore adding `/games/new` (static `new`) will match before
`/games/:id` (param), and `/games/new/:type` (3 segments) does not collide with
`/games/:id` (2 segments) or `/games/type/:name` (second segment `type` !=
`new`). This MUST be proven with an SSR test (gotcha G-route).

Parameterized pages read the param via `use_params_map` (e.g. `InvitePage`,
`proposals.rs`; `GameInfoPage`, `game_info/mod.rs:99`). URL-encoding of names
uses `players::encode_path_segment` (`players.rs:35`).

### Server fns / DB helpers reusable for E

- `create_proposal` (`proposals.rs:1036`): no human invitees -> create game
  directly; else open proposal (owner+bots accepted, humans pending). Enforces
  `roster_error` (`:1067`) + `check_invite_policy_tx` (`:1076`) + uniqueness
  (`:1089-1098`). Does NOT take a `restarted_game_id` (always inserts `None`).
- `create_game_from_service` (`game/server_fns.rs:421`): shared game-creation
  helper; caller owns the tx + broadcast. `CreateGameSeed` (`:406`).
- `roster_error(player_counts, count)` (`game/server_fns.rs:478`): pure fn,
  `None` if valid.
- `insert_proposal(tx, version_id, owner, restarted_game_id)` (`proposals.rs:537`)
  and `insert_proposal_player(...)` (`:555`).
- `start_proposal_tx` (`proposals.rs:958`): builds the game from the ACCEPTED
  roster, links `restarted_game_id` (`:1002-1009`), flips proposal to `started`.
- `find_predecessor_game_id` (`db.rs:773`): `SELECT id FROM games WHERE
  restarted_game_id = $1` (reverse link, used for `previous_game_id`).
- `find_game_type_player_counts` (`db.rs:240`), `find_available_game_types`
  (`db.rs:282`), `find_latest_non_deprecated_game_version` (used at
  `server_fns.rs:742`).
- `cancel_proposal` (`proposals.rs:1511`): owner-only; `update_proposal_status`
  -> `cancelled`; `notify_cancelled` to accepted invitees.
  `cancel_proposal_for_expiry` (`:770`) does the raw
  `UPDATE game_proposals SET status='cancelled' WHERE id=$1 AND status='open'`.
- `GameViewData` (`game/server_fns.rs:57`): has `restarted_game_id` (`:66`) and
  `previous_game_id` (`:68`); built in `get_game_details` (`:263-306`,
  `previous_game_id` populated at `:259-261`). **No field for an open restart
  proposal yet** (E5 adds one).

### SSR page tests (`tests/ssr_pages.rs`)

- `games_page_anonymous` (`:260`): GET `/games` asserts marker "New Game".
- `new_game_page_with_game_query_param_renders_shell` (`:1335`): GET
  `/games?game=Some%20Game` asserts "New Game".
- `game_info_page_renders_for_existing_game_type` (`:1267`): asserts the
  "Start a game" href is `/games?game=` (`:1311`). **E1 changes this href to
  `/games/new/{name}`; update this assertion.**
- `restart_game_on_finished_game_succeeds` (`:457`) and
  `restart_game_creates_new_game_on_latest_non_deprecated_version` (`:501`):
  POST to the `RestartGame` server-fn route. **These keep passing as long as the
  `RestartGame` server fn / `restart_game_impl` remain (E4 keeps them for the
  email path); if E4 changes `restart_game_impl`'s signature, update these
  call sites.**
- Test helpers worth reusing: `make_user`, `login_cookie`, `make_game_version`,
  `spawn_mock_new_game_service` (answers `Request::New`), `insert_finished_two_player_game`,
  `restart_game_via_http` (POSTs a server-fn url-encoded body).

---

## 2. Shared building blocks

### 2a. `OpponentSlotEditor` (already extracted, reuse as-is)

`components/opponent_slot.rs` (unit F2). Single-slot API: `get:
Signal<OpponentSlot>`, `set: Callback<OpponentSlot>`, `taken: Signal<Vec<Uuid>>`,
plus parent-owned `suggestions`/`bot_names` `LocalResource`s. The new-game setup
page (E2) renders N of these over a `Vec<OpponentSlot>` exactly as `GameBrowser`
does today (`new_game.rs:405-437`). The restart-prefill (E6) seeds that `Vec`
with `Player { selected: Some((id, name)) }` / `Bot { .. }` from the finished
game. No change to the editor is needed for E.

### 2b. `create_proposal` vs a restart-aware create

`create_proposal` (`proposals.rs:1036`) creates a fresh game/proposal with NO
`restarted_game_id`. Restart needs the same roster-folding + policy/uniqueness
checks BUT also (a) a race-safe guard on the old game and (b) the
`restarted_game_id` link. Two options:
- **(preferred) A dedicated restart core** (section 2c) that the new
  `restart_game_with_roster` server fn calls. It mirrors `create_proposal`'s
  roster folding but adds the guard and passes `Some(old_game)` to
  `insert_proposal`. Keeps `create_proposal` unchanged for the plain new-game
  path.
- (alt) Add an `Option<Uuid> restarted_game_id` param to `create_proposal`.
  Rejected: it tangles the race guard (which is restart-specific) into the
  general create path and changes a hot, well-tested fn's signature.

### 2c. Race-safe restart core (E-req3)

Refactor `restart_game_impl` (`game/server_fns.rs:713`) so the guard + create
logic lives in ONE race-safe core that takes an explicit roster + version:

```text
restart_core(pool, http, user_id, old_game_id, version, roster) -> RestartOutcome
  begin tx
  SELECT ... FROM games WHERE id = old_game_id FOR UPDATE   -- serializes restarts
  require is_finished
  if games.restarted_game_id IS NOT NULL:
      return AlreadyRestarted { game_id: Some(restarted_game_id), proposal_id: None }
  if let Some(pid) = find_open_restart_proposal_tx(tx, old_game_id):   -- status='open' AND restarted_game_id=old_game
      return AlreadyRestarted { game_id: None, proposal_id: Some(pid) }
  re-check invite policy + roster_error on the EDITED roster
  if no human invitees:
      create_game_from_service; UPDATE games SET restarted_game_id=new WHERE id=old
      commit; return Created(ProposalOutcome { game_id: Some(new), .. })
  else:
      insert_proposal(.., restarted_game_id=Some(old)); insert players
      commit; return Created(ProposalOutcome { proposal_id: Some(pid), .. })
```

- The `FOR UPDATE` lock on the old game row is the serialization point: the
  second concurrent restart blocks until the first commits, then re-reads and
  sees the first's `restarted_game_id` (solo) or open proposal (humans) and
  returns `AlreadyRestarted`.
- `RestartOutcome` (new enum, in `game/server_fns.rs` next to `ProposalOutcome`
  re-export): `Created(ProposalOutcome)` | `AlreadyRestarted { proposal_id:
  Option<Uuid>, game_id: Option<Uuid> }`. The client prefers `game_id` (link to
  `/games/{id}`) else `proposal_id` (link to `/invites/{id}`).
- `restart_game_impl` (instant, exact-roster) becomes a thin wrapper that builds
  the old roster + latest version and calls `restart_core`, returning
  `Created(..)` or surfacing `AlreadyRestarted` as the existing "Game has already
  been restarted" error (so the EMAIL path and the existing SSR tests keep their
  current behaviour). The new `restart_game_with_roster` server fn calls
  `restart_core` with the EDITED roster and returns the full `RestartOutcome`
  (so the web UI can link the loser to the winner).
- New DB helper `find_open_restart_proposal_tx(tx, old_game_id) -> Option<Uuid>`
  (and a pool variant for E5): `SELECT id FROM game_proposals WHERE
  restarted_game_id = $1 AND status = 'open' ORDER BY created_at LIMIT 1`. Plain
  (non-macro) sqlx query per CODING.md.

### 2d. Decline re-enables restart (E-req4)

- `can_restart` becomes `is_finished && restarted_game_id.is_none() &&
  restart_proposal_id.is_none()`, where `restart_proposal_id` is the open restart
  proposal for this game (new `GameViewData` field, E5). `GameMeta` shows
  "Restart" only when `can_restart`; when `restart_proposal_id.is_some()` it
  shows a "Restart invite pending - Go to invite" link to `/invites/{pid}`.
- In `respond_proposal` (`proposals.rs:1200`), when a player DECLINES and the
  proposal is a restart proposal (`proposal.restarted_game_id.is_some()`),
  AUTO-CANCEL the proposal inside the same tx (`update_proposal_status(..,
  "cancelled", None)`) and `notify_cancelled` to the accepted invitees, instead
  of merely marking the player declined. This clears the open-restart-proposal
  block on the old game, re-enabling "Restart". (A decline on a NON-restart
  proposal keeps the current behaviour: mark declined + `notify_owner_decline`.)
  See decisions D-decline-cancel and D-decline-who.

### 2e. Restart prefill (E-req2)

New server fn `get_restart_prefill(game_id) -> Result<RestartPrefill,
ServerFnError>` (E3). Returns everything the setup page needs to prefill the
form for a restart, WITHOUT a game-service render call (cheaper than reusing
`get_game_details`):
- `game_type_name` (to build/verify the `/games/new/{type}` segment),
- `version_id` + `version_name` (the latest non-deprecated version restart
  targets, per `restart_game_impl:742`),
- `player_counts` (valid counts for the type),
- `opponents: Vec<PrefillSlot>` = the finished game's players EXCLUDING the
  viewer: humans as `{ user_id, name }`, bots as `{ bot_name (display),
  bot_difficulty }`. (The viewer is the implicit first player / owner of the
  restart, matching the new-game form's `player_count - 1` opponent slots.)
- Gated: caller authenticated + is a player in the game + game `is_finished`.
  It does NOT gate on "still restartable" - the form is shown regardless; the
  submit-time race guard (2c) is authoritative.

---

## 3. Implementation units (dependency order)

Each unit is one layer/concern, sized to stay well under the 150k budget. Every
unit ends with fmt + clippy green and its own commit; DB-touching units add
`#[sqlx::test]`s. Suggested commit order: **E1, E2** (stepped routes; plain
new-game works throughout), then **E3, E4, E5** (backend restart machinery),
then **E6, E7** (wire restart into the UI). Backend (E3-E5) and the route split
(E1-E2) are independently landable; E6 needs E2+E3+E4; E7 needs E5+E6.

### E1. Stepped routes + type-selection page `/games/new` (frontend)
- Goal: E-req1 step 1 - a page at `/games/new` that lists game types; clicking
  one navigates to `/games/new/{type}`.
- Files: `rust/web/src/app.rs` (routes), `rust/web/src/new_game.rs` (split
  `NewGamePage`/`GameBrowser`), `rust/web/src/game_info/mod.rs` (start href),
  `rust/web/tests/ssr_pages.rs` (assertions).
- Change:
  - Add routes in `app.rs:215-232`: `<Route path=(StaticSegment("games"),
    StaticSegment("new")) view=crate::new_game::NewGameTypePage/>` and `<Route
    path=(StaticSegment("games"), StaticSegment("new"), ParamSegment("type"))
    view=crate::new_game::NewGameSetupPage/>`. Place them BEFORE the
    `/games/:id` route (order is cosmetic given static-beats-param ranking, but
    keep it readable). Keep `/games` (E1 makes it redirect, below).
  - New `NewGameTypePage` (`new_game.rs`): `MainLayout` + the game-card grid
    (today's left column, `new_game.rs:251-310`) rendered FULL width. Reuse
    `get_available_game_types` + `filter_and_sort` + `player_range`/`weight_text`
    (all already in `new_game.rs`). Each card becomes a link/`<A>` to
    `/games/new/{encode_path_segment(name)}` instead of a radio that selects an
    in-page panel. Keep the filter/search/sort controls.
  - `/games` redirect: make `NewGamePage` (the `/games` route) a thin redirect
    to `/games/new` (preserve a `?game=X` query by redirecting to
    `/games/new/{X}`). Use the `Effect`-navigate pattern (CODING.md "Redirect
    anonymous users ... via an Effect") so SSR renders a harmless shell and the
    navigate fires client-side. (Alternatively keep `/games` rendering
    `NewGameTypePage` directly and add `/games/new` as an alias - see D-route.)
  - `game_info/mod.rs:122`: change `start_href` from `/games?game={name}` to
    `/games/new/{encode_path_segment(name)}`.
- Acceptance: `/games/new` renders the grid; clicking a game goes to
  `/games/new/{name}`; `/games` and `/games?game=X` still land the user on the
  right place; GameInfoPage "Start a game" points at `/games/new/{name}`.
  Hydration clean (no structural swaps on async data).
- Tests: SSR tests - `/games/new` returns 200 + a grid marker (e.g. the
  `game-card-grid` class) and no panic; `/games` redirect shell renders without
  panic; update `game_info_page_renders_for_existing_game_type` to assert
  `/games/new/` href. Keep `games_page_anonymous` green (it now asserts the
  redirect shell or the grid, per D-route).
- Depends on: nothing.

### E2. New-game setup page `/games/new/{type}` (frontend)
- Goal: E-req1 step 2 - the player-configuration page for a chosen type.
- Files: `rust/web/src/new_game.rs`.
- Change:
  - New `NewGameSetupPage` (`new_game.rs`): read the `type` param via
    `use_params_map`; `LocalResource` on `get_available_game_types`; resolve the
    type by name case-insensitively (reuse the preselect matching logic from
    `new_game.rs:178-194`). Render today's setup panel (`new_game.rs:311-468`):
    version select, "View rules" link, player-count radios, one
    `OpponentSlotEditor` per opponent slot (reuse the `taken` derive + the
    per-slot `get`/`set` wiring at `new_game.rs:405-437`), submit -> 
    `create_proposal` -> navigate effect (`new_game.rs:196-218`). Add a "Back to
    games" link to `/games/new`.
  - Unknown type name: render a clean "Game not found" state (200, no panic),
    matching GameInfoPage's `Ok(None)` arm.
  - The default player count = `gt.player_counts.first()` (as `select_game` does
    today, `new_game.rs:167`).
- Acceptance: `/games/new/{name}` shows the setup panel for that type; choosing
  player count resizes the opponent slots; submit creates a game/proposal and
  navigates; unknown name shows "Game not found". Existing `new_game.rs` unit
  tests (`player_range`, `filter_and_sort`, `sort_variants`) still pass.
- Tests: SSR test - `/games/new/{known}` returns 200 + a setup-panel marker (e.g.
  the player-count `radiogroup` or the type name) and no panic; `/games/new/{unknown}`
  returns 200 + "Game not found".
- Depends on: E1.

### E3. Restart prefill server fn (backend)
- Goal: E-req2 enabler - feed the setup page the finished game's roster.
- Files: `rust/web/src/game/server_fns.rs` (server fn + `RestartPrefill` /
  `PrefillSlot` structs), possibly `rust/web/src/db.rs` (a small roster query if
  not reusing `find_game_extended`).
- Change:
  - `#[server(GetRestartPrefill, "/api")] pub async fn get_restart_prefill(game_id:
    Uuid) -> Result<RestartPrefill, ServerFnError>`: auth via `get_current_user`;
    load the game (`find_game_extended`); require `is_finished` and that the
    caller is a player; resolve the latest non-deprecated version
    (`find_latest_non_deprecated_game_version`, fallback to the game's version)
    and `player_counts`; build `opponents` from `ge.game_players` excluding the
    viewer (human -> `{user_id, name}`; bot -> `{bot display name, bot_name}`).
  - `RestartPrefill { game_type_name: String, version_id: Uuid, version_name:
    String, player_counts: Vec<i32>, opponents: Vec<PrefillSlot> }`;
    `PrefillSlot { user_id: Option<Uuid>, name: String, bot_name: Option<String> }`
    (human: `user_id=Some, bot_name=None`; bot: `user_id=None,
    bot_name=Some(difficulty)`).
- Acceptance: returns the opponent roster (humans by id+name, bots) for a
  finished game to one of its players; rejects a non-player and a non-finished
  game.
- Tests (`#[sqlx::test]`): prefill of a finished 2-human game returns the OTHER
  human by id+name; prefill of a solo-vs-bot game returns the bot; non-player
  caller is rejected; unfinished game is rejected.
- Depends on: nothing (loosely informed by E4's roster shape; safe to land
  first).

### E4. Race-safe restart core + `restart_game_with_roster` (backend)
- Goal: E-req3 - atomic "first restart wins"; a web server fn that restarts with
  an EDITED roster and returns a link-rich outcome for the loser.
- Files: `rust/web/src/game/server_fns.rs` (refactor `restart_game_impl`, new
  `restart_core`, new `restart_game_with_roster` server fn, `RestartOutcome`
  enum), `rust/web/src/db.rs` (`find_open_restart_proposal_tx` + pool variant),
  `rust/web/src/proposals.rs` (no signature change; reuses `insert_proposal`).
- Change:
  - Add `find_open_restart_proposal_tx(tx, old_game_id) -> Option<Uuid>` and a
    pool variant `find_open_restart_proposal(pool, old_game_id)` (plain sqlx;
    `WHERE restarted_game_id = $1 AND status = 'open' ORDER BY created_at LIMIT
    1`).
  - Extract `restart_core(pool, http, user_id, old_game_id, version,
    opponent_ids, opponent_emails, bot_slots) -> Result<RestartOutcome,
    ServerFnError>` implementing section 2c: `FOR UPDATE` lock on the old game
    row, finished check, `restarted_game_id`/open-restart-proposal guard
    returning `AlreadyRestarted`, policy + `roster_error` on the edited roster,
    then solo-create-game-and-link OR insert-restart-proposal. Caller-owned
    broadcast/notify stays in the server fns (mirror `restart_game:905-928`).
  - `restart_game_impl` (`:713`) becomes a wrapper: build the exact old roster +
    latest version (existing logic `:742-763`), call `restart_core`, and map
    `Created(outcome) -> Ok(outcome)`, `AlreadyRestarted -> Err("Game has
    already been restarted")`. **Keeps the email path (`email/commands.rs:983`)
    and the existing SSR tests (`ssr_pages.rs:457,501`) behaviourally
    unchanged.**
  - New `#[server(RestartGameWithRoster, "/api")] pub async fn
    restart_game_with_roster(game_id, game_version_id, opponent_ids:
    Option<Vec<Uuid>>, opponent_emails: Option<Vec<String>>, bot_slots:
    Option<Vec<BotSlot>>) -> Result<RestartOutcome, ServerFnError>`: auth;
    resolve the version (or trust `game_version_id` and re-validate it belongs
    to the game's type - D-version); call `restart_core` with the edited roster;
    on `Created`, do the same post-commit broadcast + invite emails as
    `restart_game` (`:905-928`); on `AlreadyRestarted`, just return it (no
    broadcast). Return the full `RestartOutcome` so the UI can link.
- Acceptance: two concurrent restarts of the same finished game - first wins,
  second gets `AlreadyRestarted` linking to the winner's game (solo) or open
  proposal (humans); the edited roster is what gets created; invite policy +
  count are enforced on the edited roster; email `restart` still works.
- Tests (`#[sqlx::test]`, reuse `spawn_mock_new_game_service`): solo restart
  links `restarted_game_id`; human restart opens a proposal carrying
  `restarted_game_id`; a SECOND restart (after the first committed) returns
  `AlreadyRestarted` with the right link (game_id for solo, proposal_id for an
  open proposal); edited roster (e.g. drop a human, add a bot) is honoured;
  `roster_error` rejects an invalid edited count. Drive `restart_core` directly
  (like the existing `restart_game_impl` tests at `:1286,1339`) and/or via HTTP
  (like `restart_game_via_http`).
- Depends on: E3 (shares the roster/prefill vocabulary; not a hard code dep).

### E5. Decline cancels a restart proposal + `GameViewData.restart_proposal_id` (backend)
- Goal: E-req4 - decline re-enables restart; expose the open restart proposal on
  the game view so the UI can compute `can_restart` and link to the invite.
- Files: `rust/web/src/proposals.rs` (`respond_proposal`), `rust/web/src/game/server_fns.rs`
  (`GameViewData` + `get_game_details`), `rust/web/src/db.rs` (reuse
  `find_open_restart_proposal` from E4).
- Change:
  - `respond_proposal` (`proposals.rs:1200`): on a DECLINE, if
    `proposal.restarted_game_id.is_some()`, auto-cancel the proposal in the same
    tx (`update_proposal_status(.., "cancelled", None)`) and, after commit,
    `notify_cancelled` to the accepted invitees (mirror `cancel_proposal:1563-1569`).
    Skip the `notify_owner_decline` for the restart-cancel case (the cancel
    notification supersedes it) - or keep it; see D-decline-cancel. A decline on
    a non-restart proposal is unchanged.
  - `GameViewData` (`game/server_fns.rs:57`): add `restart_proposal_id:
    Option<Uuid>`. In `get_game_details` (`:263-306`), populate it via
    `find_open_restart_proposal(pool, game_id)` (E4). (Only meaningful for
    finished games; cheap single-row query.)
- Acceptance: declining a restart proposal cancels it and emails the accepted
  invitees; the old game's `restart_proposal_id` clears (so `can_restart` is
  true again); declining a normal proposal still just marks declined + notifies
  the owner; `get_game_details` surfaces the open restart proposal id.
- Tests (`#[sqlx::test]`): decline on a restart proposal sets status='cancelled'
  and the old game has no open restart proposal afterward; decline on a normal
  proposal leaves it open with the player declined; `get_game_details` (or a
  focused query test) returns `restart_proposal_id` when an open restart
  proposal exists and `None` otherwise.
- Depends on: E4 (`find_open_restart_proposal`).

### E6. Setup-page restart mode (frontend, `new_game.rs`)
- Goal: E-req2 + E-req3 UI - the setup page handles `?restart={game_id}`:
  prefill the roster and submit through the race-safe restart fn.
- Files: `rust/web/src/new_game.rs` (setup page from E2).
- Change:
  - In `NewGameSetupPage`, read `?restart={game_id}` via `use_query_map`. When
    present, add a `LocalResource` on `get_restart_prefill(game_id)` (E3) and
    seed the opponent slots from `prefill.opponents` (human ->
    `OpponentSlot::Player { query: "", selected: Some((id, name)) }`; bot ->
    `OpponentSlot::Bot { name, bot_name }`), set `player_count =
    opponents.len() + 1`, and pin the version to `prefill.version_id`. Show a
    "Restarting {type} - edit players below" heading and a link back to the
    finished game.
  - Submit in restart mode dispatches `restart_game_with_roster` (E4) instead of
    `create_proposal`. A navigate effect handles `RestartOutcome`:
    `Created(outcome)` -> `/games/{id}` or `/invites/{id}` (as today);
    `AlreadyRestarted { game_id: Some(g), .. }` -> navigate `/games/{g}` with an
    error flash "This game was already restarted"; `AlreadyRestarted {
    proposal_id: Some(p), .. }` -> navigate `/invites/{p}` (or show an inline
    error linking there). Use the `confirm()` helper if a destructive confirm is
    wanted (not required here).
  - Keep the non-restart submit path (`create_proposal`) exactly as E2 left it.
- Acceptance: opening `/games/new/{type}?restart={game_id}` prefills the
  finished game's players (by username) and bots; editing then submitting
  creates the restarted game/proposal; losing the race navigates to the winner's
  game/invite with a clear message; hydration stays clean (prefill is a
  `LocalResource`, slots seeded in an effect, no structural SSR/client swap).
- Tests: SSR test - `/games/new/{type}?restart={game_id}` for a finished game
  returns 200 + the setup-panel marker and no panic (prefill is a LocalResource
  so it is `None` on SSR; the shell must render without the resolved roster).
  Roster-folding logic can reuse the existing `new_game.rs` unit-test style.
- Depends on: E2, E3, E4.

### E7. `GameMeta` restart UI (frontend, `components/game.rs`)
- Goal: point "Restart" at the new form and reflect the open-restart-proposal
  state; remove the old instant-restart action.
- Files: `rust/web/src/components/game.rs`.
- Change:
  - `can_restart = is_finished && restarted_game_id.is_none() &&
    data.restart_proposal_id.is_none()` (E5 field).
  - "Restart" link (`:134-141`): instead of dispatching `RestartGame`, navigate
    to `/games/new/{encode_path_segment(data.type_name)}?restart={game_id}` (an
    `<A href>` is fine; no confirm needed). Remove the `RestartGame`
    `ServerAction` (`:44`) and its navigate effect (`:66-74`) - restart no longer
    happens from this page.
  - When `data.restart_proposal_id.is_some()`, show a "Restart invite pending"
    link to `/invites/{pid}` (in place of "Restart"). Keep "Go to new game"
    (`:142-148`) for `restarted_game_id.is_some()` and "Previous game"
    (`:149-155`).
- Acceptance: a finished, not-yet-restarted game shows "Restart" linking to the
  prefilled form; a game with an open restart proposal shows the pending-invite
  link and NOT "Restart"; a restarted game shows "Go to new game". No instant
  restart remains in the web UI.
- Tests: SSR test - a finished game page (logged-in player) shows the "Restart"
  link href `/games/new/...?restart=`; with an open restart proposal it shows
  the `/invites/` link instead. (Build on `game_page_logged_in_player_renders_game`,
  `ssr_pages.rs:305`.)
- Depends on: E5 (GameViewData field), E6 (the form route exists).

Suggested commit order: E1, E2, E3, E4, E5, E6, E7. Push deferred to a final
cleanup unit per the orchestrate handover rules.

---

## 4. Decisions for the user

1. **D-route - what happens to `/games`?** (a) `/games` redirects to `/games/new`
   (recommended; preserves old bookmarks and the `?game=` deep links by
   redirecting `/games?game=X` -> `/games/new/X`), or (b) keep `/games` rendering
   the type grid directly and add `/games/new` as an alias (two URLs, same page).
   Proposed: (a) redirect. Either way, GameInfoPage's "Start a game" moves to
   `/games/new/{name}`.
2. **D-type-param - is `{type}` a name or an id?** Proposed: the game type NAME,
   URL-encoded via `encode_path_segment`, resolved case-insensitively (matches
   GameInfoPage `/games/type/:name` and the existing `?game=<name>` preselect).
   Names with spaces become `%20`-encoded segments. Alternative: a Uuid (ugly
   URLs, but no name-resolution ambiguity). Confirm name.
3. **D-version - which version does a restart target, and can the editor change
   it?** Today `restart_game_impl` silently restarts onto the LATEST
   non-deprecated version (`:742`). Proposed: the prefill (E3) pins the version
   to the latest non-deprecated one and the setup page shows the version select
   (if >1) so the owner CAN change it; `restart_game_with_roster` validates the
   chosen version belongs to the same game type. Confirm whether the owner may
   change the version on restart (proposed: yes, like a normal new game).
4. **D-decline-cancel - exact decline-cancels-restart semantics (E-req4).**
   Proposed: when ANY player declines a restart proposal, the whole proposal is
   auto-cancelled (status='cancelled') and the accepted invitees get
   `notify_cancelled`; the old game becomes restartable. Confirm: (a) auto-cancel
   on the first decline (vs. only when the OWNER declines, or only marking the
   player declined and letting the owner cancel manually); (b) the cancel email
   wording reuses `notify_cancelled` ("The game invite was cancelled.") - or a
   restart-specific "{player} declined the rematch, so it was cancelled" note
   (proposed: reuse `notify_cancelled` + still send `notify_owner_decline` to the
   owner so they know who declined).
5. **D-decline-who - who may restart afterwards?** After a decline-cancel, the
   old game is restartable by ANY of its players (the "Restart" link shows for
   all players, as today). Confirm we do not restrict restart to the original
   owner. (Today `restart_game_impl` only requires caller-is-a-player, not
   owner.) Proposed: keep any-player.
6. **D-already-restarted-ux - loser UX for the race (E-req3).** Proposed: on
   `AlreadyRestarted`, navigate the loser to the winner's game (`/games/{id}` if
   `game_id` set) or invite (`/invites/{id}` if `proposal_id` set) and show a
   flash/inline message "This game was already restarted." Confirm navigate-and-
   flash vs. stay-on-form-with-error-link. (The requirement says "error linking
   to invite page or restarted game" - navigate satisfies the link.)
7. **D-restart-email - does the form-based restart re-send invite emails?**
   Proposed: yes, identical to today's `restart_game` (`:911-925`) - pending
   humans get `send_invite` after the proposal is created. The restart proposal
   threads like a normal invite. Confirm.
8. **D-prefill-owner - is the viewer included in the prefilled slots?** Proposed:
   no - the viewer is the implicit first player/owner (matching the new-game
   form's `player_count - 1` opponent slots), so prefill lists only the OTHER
   players. Confirm.
9. **D-keep-restart-fn - keep the instant `restart_game` server fn?** Proposed:
   keep `restart_game_impl` (email path + existing SSR tests depend on it) as a
   wrapper over the new race-safe `restart_core`; the `RestartGame` server fn can
   stay (harmless, tested) or be removed once the web UI no longer uses it.
   Proposed: keep both for now; removing `RestartGame` is optional cleanup that
   would require updating `ssr_pages.rs:457,501`. Confirm.
10. **D-prefill-vs-details - prefill data source.** Proposed: a dedicated
    `get_restart_prefill` (E3) that avoids the game-service render call inherent
    in `get_game_details`. Alternative: reuse `get_game_details` (more code reuse
    but does an unnecessary render + is heavier to mock in SSR tests). Confirm
    dedicated fn.

---

## 5. Known issues / gotchas (carry forward to every Lead)

- **G-route - prove static-beats-param routing.** Adding `/games/new` and
  `/games/new/{type}` relies on leptos_router ranking the static `new` segment
  above the param `:id` in `/games/:id`. This is expected behaviour but MUST be
  proven with an SSR test (GET `/games/new` returns the grid, NOT the GamePage
  "Game not found"/error). If ranking does not hold, reorder routes or rename
  (e.g. `/games-new`). Verify before building E6/E7 on top.
- **G-migrations - none expected; next number is 021.** `020_drop_user_last_seen_at.sql`
  is the current max. Unit E adds NO schema (the `restarted_game_id` columns
  already exist on both `games` and `game_proposals`; E only adds queries + a
  `GameViewData` field). If a migration becomes necessary, it must be a NEW
  numbered file `021+` (migrations are immutable - sqlx checksum break, happened
  with 005 on 2026-07-11).
- **G-race-lock - the `FOR UPDATE` lock is the whole fix.** The restart guard
  MUST `SELECT ... FROM games WHERE id = old_game FOR UPDATE` inside the tx
  BEFORE checking `restarted_game_id` / open-restart-proposal, or the race is
  not actually closed. The open-restart-proposal check must be in the SAME tx
  (after the lock) so the second restart sees the first's committed proposal.
- **G-link-timing - the old->new game link is written on proposal START, not on
  proposal creation** (`start_proposal_tx:1002-1009`). That is why the race guard
  must ALSO treat an OPEN restart proposal as an in-flight restart (checking only
  `games.restarted_game_id` would miss the human/proposal case). Keep both
  checks.
- **G-email-path - do not break email `restart`.** `email/commands.rs:983` calls
  `restart_game_impl` directly. E4's refactor must keep `restart_game_impl`'s
  observable behaviour (instant, exact-roster, "Game has already been restarted"
  error) - it becomes a wrapper over `restart_core`. The existing SSR tests
  `ssr_pages.rs:457,501` guard this.
- **G-hydration - prefill is a `LocalResource`.** The setup page's restart
  prefill (E6) must be a `LocalResource` (None on SSR), with slots seeded in an
  `Effect`/`spawn_local` after it resolves, so SSR and hydration run 0 are
  identical (docs/hydration.md). Do NOT read the prefill under a
  `Suspense`/`Transition` that SSR must render, and do not swap element
  STRUCTURE on the resolved roster - toggle attributes/classes. The
  `?restart=` mode and the plain mode should render the same shell.
- **G-sqlx-offline - prefer plain queries for new column touches.** New DB
  helpers (`find_open_restart_proposal_tx`, any prefill query) should use plain
  `sqlx::query`/`query_as` (not macros) to avoid `.sqlx` cache regeneration
  (CODING.md "Plain (non-macro) sqlx queries"). Canonical gates (DEV.md):
  `cargo fmt --all -- --check`;
  `SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo check -p web --features hydrate`; web test suite under real
  Postgres+NATS (reuse `brdgme-test-{pg,nats}-47116` on 15432/14222/18222, or
  `scripts/rust-test.sh`). Target `-p web` only.
- **G-clippy-all-targets - mandatory.** Never commit with outstanding fmt/clippy;
  a prior unit left `--all-targets` red.
- **G-db-tests - need real Postgres.** Plain local runs fail DB tests
  (pre-existing, backlog #40 - not a regression). `#[sqlx::test]` gives each
  test an isolated migrated DB.
- **G-flake - `invite_expiry_threshold_defaults_to_14_days`** (env-var race) is a
  pre-existing flake; do not chase it as a regression.
- **G-no-panics** in handlers/components (CODING.md): `use_params_map().get("type")`
  is `Option`-ish (`.get` returns `Option<&String>`), `web_sys::window()` is
  `Option` - handle `None` (the type-resolution "Game not found" arm covers the
  missing/unknown param).
- **G-email-gating** - any new invite email reused by restart (`send_invite`,
  `notify_cancelled`) already routes through `fetch_invite_recipient` +
  `invite_recipient_should_send` (verified primary email + `invite_emails_enabled`
  + web-presence suppression, unit I-2). Restart reuses these unchanged; do not
  add a new mailer path that bypasses the gate.
- **G-games-updated-at** - `games.updated_at` is trigger-maintained (CODING.md
  "Database"); the solo-restart `UPDATE games SET restarted_game_id` bumps it
  (already the case today). Irrelevant to proposal edits.
- **G-shared-files - sequence the edits.** `new_game.rs` is touched by E1, E2,
  E6 (land in that order). `game/server_fns.rs` + `db.rs` are touched by E3, E4,
  E5 (land in that order; E5 reuses E4's `find_open_restart_proposal`).
  `proposals.rs` `respond_proposal` is touched only by E5. `components/game.rs`
  only by E7. Keep units serial to avoid conflicting edits.
- **G-org** - the GitHub org is `brdgme` (not `beefsack`) for any image/URL refs.
