# Current Status

## Session: 2026-04-04

### Pending infra (must do before Tilt restart if not already applied)

These steps were identified last session and may not yet be applied:

1. `kubectl apply -f k8s/base/operator/crd.yaml` - removes `playerCounts` from
   CRD required fields
2. `cd rust/web && sqlx migrate run` - applies migration 004 which adds `rules`
   column to `game_versions`

After both, Tilt restart is safe. The operator will reconcile all GameVersion
CRs, fetching `player_counts` and `rules` from each game service and writing
them to the DB.

### What was completed this session

**Operator: player counts and rules from game service**
- Operator now calls `Request::PlayerCounts` and `Request::Rules` against each
  game service during reconcile and writes both to the DB
- `playerCounts` removed from the GameVersion CRD spec - the operator derives
  it from the game service instead
- Bot reads rules from `gv.rules` DB column; no longer calls the game service
  for rules at runtime
- Migration 004 adds `rules TEXT NOT NULL DEFAULT ''` to `game_versions`

**Lost Cities rules (both versions)**
- `rust/game/lost-cities-1/RULES.md` and `rust/game/lost-cities-2/RULES.md`
  created with full rules, scoring examples, commands, real brdgme markup
  renders, and strategy section
- Real renders extracted locally: built `lost_cities_2_cli`, fetched game state
  from DB via `psql`, piped as a `Status` request, extracted with `jq`. Game
  used: `700c8363-70c7-436d-9d05-7bab2c48649d` (round 2 of 3, 2-player)
- Both games' `rules()` method now uses `include_str!("../RULES.md")` — Tilt
  rebuilds the game container whenever RULES.md changes
- 3-player render still pending: user to create a 3-player game in a future
  session and add the render to `lost-cities-2/RULES.md`

**docs/RULES.md**
- Updated real-render extraction to document the local CLI approach:
  build `<game>_cli`, fetch game state with `psql`, construct `Status` request
  with `jq -Rs`, extract render with `jq -r`
- Replaces the previous `kubectl exec` approach

**Leptos hydration fixes (`rust/web/src`)**
Three separate SSR/hydration mismatch bugs fixed. All caused the same symptom:
Leptos's `tachys` panicking on hard refresh with "Unrecoverable hydration
error". None were caused by application logic panics — they were structural
mismatches between server-rendered HTML and the client's initial reactive state.

- `GameLogs` and `RecentGameLogs` (`components/game.rs`): changed `Resource::new`
  to `LocalResource::new`. The old serializable resource was immediately
  available on the client but rendered `None` on SSR, causing a mismatch when
  logs were present.
- `App()` `active_games` (`app.rs`): changed from `Resource::new` to
  `LocalResource::new`. `Resource::new_blocking` was tried first but didn't
  resolve the issue because the resource was passed via context and the `Suspense`
  in `SidebarMenu` could not track it correctly across the component boundary.
- `GamePage` (`app.rs`): changed `Transition` to `Suspense`. `Transition` renders
  children directly on SSR with no fallback mechanism — when `game_data` was
  `None` synchronously, SSR emitted `<!-- -->` while the client had serialised
  data immediately and rendered the full game layout.

**Coding guidelines and plan updates**
- `docs/CODING.md` created: project-wide rules for error handling, Leptos
  resource types, SSR/hydration contracts, context usage, and component design
- `docs/PLAN.md`: Phase 5.6.1 added (move `active_games` out of `App()` context
  into `SidebarMenu`) and Phase 5.7 added (eliminate runtime panics in
  `rust/web/src` — four specific cases documented)
- `docs/DEV.md`: bot/LLM config section added; `.env` contents no longer need
  to live in STATUS.md

### Known open issues

- **3-player render**: `lost-cities-2/RULES.md` has a placeholder. User to
  create a 3-player game and extract a real render to replace it.
- **`active_games` in context** (Phase 5.6.1): `active_games` resource still
  lives in `App()` and is passed via context to `SidebarMenu`. This violates
  the ownership rule in `docs/CODING.md` and was the original cause of the
  hydration bug. Fix is described in PLAN.md Phase 5.6.1.
- **Runtime panics in `rust/web`** (Phase 5.7): Four cases identified in the
  audit — `db.rs:407` (`last_mut().unwrap()`), co-nullable LEFT JOIN unwraps in
  `db.rs`, `NodeRef::get().unwrap()` in `app.rs` form handlers,
  `websocket_client.rs` JS API `.expect()` calls. Details in PLAN.md Phase 5.7.
- **Restart 500 error**: `restart_game` returns "Game service error: error
  parsing JSON response". Diagnostics in place but root cause not yet found.
- **Bot restart limitation**: `restart_game` only collects human `opponent_ids`;
  bots are not recreated in the restarted game.
- **Optimistic locking**: race condition in `execute_command` +
  `update_game_command_success`. Design in PLAN.md.

### Next steps (in order)

1. **Apply CRD + migration** if not yet done, then restart Tilt
2. **Phase 5.6.1** — move `active_games` resource into `SidebarMenu`
3. **Phase 5.7** — fix runtime panics in `rust/web`
4. **3-player render** — create a 3-player Lost Cities game, extract render,
   update `lost-cities-2/RULES.md`
5. **Phase 9: NATS bot eventing**
6. **Continue bot quality testing** on Acquire
7. **Rules for other games** — `lords-of-vegas-1`, remaining games
8. **Phase 6.5** — Production CD (ArgoCD)
9. **Phase 7** — Side-by-side validation then legacy decommission
