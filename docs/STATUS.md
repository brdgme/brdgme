# Current Work Status

## Phase 5.6: In Progress

All 13 blockers resolved. All 4 missing API endpoints implemented.
Frontend gaps and code quality items remain (see PLAN.md).

---

## Completed this session

### Game log rendering

- `db::get_game_logs(pool, game_id, game_player_id)` - fetches public logs and
  private logs targeted at the player, ordered by `logged_at ASC`.
- `GameLogEntry { body_html, logged_at, is_new }` struct in `server_fns.rs`.
- `get_game_logs(game_id)` server fn - authenticates, finds caller's
  `game_player`, computes `is_new` from `last_turn_at`, renders markup to HTML.
- `GameLogs` component - takes `game_id: Uuid`, fetches via `Resource`, groups
  into 10-minute windows with headings, marks new logs with `log-entry-new` CSS class.
- SQLx offline metadata regenerated; SSR and WASM builds both clean.

### Missing API endpoints (`game/server.rs`, `db.rs`)

**`POST /game/{id}/undo`**
- Authenticates caller; verifies they have a non-NULL `undo_game_state`.
- Calls `Request::Status` on the game service with the saved undo state.
- `db::undo_game`: transaction resets `games.game_state`, clears
  `is_finished`/`finished_at` based on Status result, updates each player's
  `is_turn`/`is_eliminated`/`place`, sets `undo_game_state = NULL` for all
  players, inserts `"Game undone."` public log.
- Broadcasts `GameUpdate`.

**`POST /game/{id}/mark_read`**
- Authenticates caller; verifies they are a player.
- `db::mark_game_read`: `UPDATE game_players SET is_read = true WHERE game_id = $1 AND user_id = $2`.

**`POST /game/{id}/concede`**
- Rejects if game finished or player count != 2.
- `db::concede_game`: transaction sets `is_finished = true/finished_at = NOW()`,
  assigns `place = 1` to winner and `place = 2` to conceder, clears
  `undo_game_state` for all players, inserts `"$name conceded."` public log.
- Broadcasts `GameUpdate`.

**`POST /game/{id}/restart`**
- Rejects if game not finished or `restarted_game_id` already set.
- Calls `Request::New` on the game service with same player count.
- Creates new game via existing `create_game_with_users`.
- `UPDATE games SET restarted_game_id = $new_id WHERE id = $old_id`.
- Broadcasts `GameRestarted { game_id, restarted_game_id }`.
- Returns `201 Created` with new game JSON.

---

## Immediate next tasks (Phase 5.6 frontend gaps)

From PLAN.md Phase 5.6 - remaining frontend/UI items:

1. **New-game UI** - page to create a new game (pick game type, invite players).
2. **Action buttons** - Undo, Concede, Restart wired to their endpoints in `GameMeta`.
3. **Dashboard active game list** - render `get_active_games` results in the
   sidebar/dashboard with turn indicators and navigation links.
4. **Game finished state** - show result/placing when `is_finished = true`.
5. **CSS for log classes** - `log-entry-new`, `log-window`, `log-window-heading`
   need styles in `style/main.scss`.
