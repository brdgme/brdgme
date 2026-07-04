# Phase 12: ELO Ratings

**Status:** Complete (backfill decided 2026-07-04: no backfill needed)

**Why blocking:** the legacy `rust/api` updates ratings when a game finishes;
`rust/web` does not. Both systems share the DB during side-by-side operation,
and the legacy idempotency guard (skip if any player already has
`rating_change` set) means a game finished via `rust/web` is never rated - not
even retroactively. Every game finished through the new system before this
lands is permanently unrated. Decided 2026-07-02: implement before the new
system serves real games in production (not yet deployed, so no live bleed).

**Reference implementation:** `rust/api/src/db/query/mod.rs:718-846`
(`update_game_placings`, `elo_rating_change`, `elo_expected_score`). Port the
logic, do not redesign it.

**Algorithm (from legacy, keep identical for human-only games):**
- Runs when a game transitions to `Finished` with non-empty `placings`.
- Idempotency guard: skip entirely if any `game_players.rating_change` is
  already non-null for the game.
- For every unordered pair of players (a, b): score `a_score` = 1.0 if a placed
  better (lower placing) than b, 0.5 if equal, 0.0 if worse.
- `expected = 10^(a_rating/400) / (10^(a_rating/400) + 10^(b_rating/400))`
- `change = round(K * (a_score - expected))` with `K = 32.0`; add `change` to
  a's accumulator and subtract from b's.
- Ratings come from `game_type_users` (create the row with default rating 1500
  if missing - `rust/web` currently only fabricates a default in memory on
  read; the write path must INSERT). Implemented as a bare
  `INSERT ... ON CONFLICT DO NOTHING` with no explicit rating column, so the
  actual DB column default (1200, per `game_type_users.rating integer DEFAULT
  1200`) applies - matching `create_game_with_users`'s existing insert
  pattern and legacy's own NULL-lets-column-default-apply behavior. The 1500
  figure here matches only the in-memory fallback in `build_game_type_user`
  used for display when no row exists yet, not the real column default.
- Apply accumulated changes to `game_type_users.rating` and store per-player
  `game_players.rating_change`. Skip zero changes.
- Legacy never updates `peak_rating` (writes only NULL on creation). Optional
  improvement while here: `peak_rating = GREATEST(peak_rating, new_rating)`.

**New rule (not in legacy - legacy predates bots):** any game that includes at
least one bot player must not affect any rating. Leave `rating_change` NULL for
all players in such games. Only human-vs-human games are rated.

**Also broken - concede path:** legacy `concede_game`
(`rust/api/src/db/query/mod.rs:572`) assigns placings (non-conceder 1,
conceder 2) and rates the game. `rust/web`'s `db::concede_game` (db.rs:688)
only sets `is_finished = true`: conceded games get no `place`, no
`rating_change`, no rating update. Fix as part of this task.

**Tasks:**
- [x] Add rating update to the finish path of `update_game_command_success`
      (or a helper it calls) in `rust/web/src/db.rs`, inside the same
      transaction as the placings write.
- [x] Fix `db::concede_game` to write `game_players.place` (non-conceder 1,
      conceder 2, matching legacy) and run the same rating update helper.
      (`concede_game` already wrote `place`; only the rating call was
      missing.)
- [x] Port `elo_rating_change` + `elo_expected_score` + the
      `elo_rating_change_works` unit test from `rust/api`.
- [x] INSERT `game_type_users` row when missing, ON CONFLICT DO NOTHING (bare
      column-list insert, matching the existing `create_game_with_users`
      pattern - the DB column default is 1200, not 1500; see report).
- [x] Skip rating updates entirely when any `game_players.game_bot_id` is
      non-null in the game.
- [x] Regenerate SQLx offline metadata (`cargo sqlx prepare -- --features ssr`).
- [x] Tests: unit tests for the pairwise math (ported `elo_rating_change_works`
      plus a 3-player pairwise case); `#[sqlx::test]` integration: 2-player and
      3-player games rated correctly on finish; idempotency guard (second
      finish write does not re-rate); game with a bot player not rated;
      `game_type_users` row created on first rated game; concede assigns
      places and rates.
- [x] Decide whether to backfill unrated games finished via `rust/web` before
      this change (list: finished games where all `rating_change` are NULL and
      no bot players). Optional - low game volume may not justify it.
      Decided 2026-07-04: no backfill - `rust/web` has never served production,
      so only dev databases contain unrated games; prod data is unaffected.

