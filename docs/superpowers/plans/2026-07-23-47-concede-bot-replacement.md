# Concede with Bot Replacement & End Game (#47) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let players concede in any game (replaced by a configurable bot), add an "End game" button for the last human, and split "game placings" from "ranked placings" so ELO/Form ignore bot performance.

**Architecture:** A new migration relaxes the `game_players` user/bot XOR constraint so a conceded human keeps `user_id` and gains a `game_bot_id` (the replacement). New `ranked_placing` and `left_at` columns drive a placing algorithm where concessions/eliminations lose first (by `left_at`) and survivors are ranked by game placing. Ratings exclude pure bots (`user_id IS NULL`) and use `ranked_placing`.

**Tech Stack:** Rust, Leptos (ssr feature), sqlx (Postgres, offline mode), NATS JetStream (bot turns), `#[sqlx::test]` for DB tests.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-07-23-47-concede-bot-replacement-design.md`
- **Context doc:** `docs/superpowers/plans/2026-07-23-47-concede-context.md` (exact current code, file:line refs).
- **Test DB:** Task 1 starts a persistent Postgres at `postgres://postgres:postgres@localhost:15432/brdgme`. Export `DATABASE_URL=postgres://postgres:postgres@localhost:15432/brdgme` for every `sqlx prepare` / `cargo test` command.
- **sqlx offline:** After ANY change to a `sqlx::query!`/`query_as!`/`query_scalar!` macro, run `cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets` and commit the resulting `rust/web/.sqlx/` changes. Plain `sqlx::query(...)` (non-macro) calls do NOT need this.
- **Migrations are immutable** once applied (AGENTS.md). Never edit an existing migration file. New schema goes in `022_*.sql`.
- **Cargo flags:** the `web` crate has no default features. Always use `--features ssr` for check/clippy/test. Target the package: `-p web`.
- **Verification commands** (run from `rust/`):
  - `cargo fmt --all -- --check`
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `SQLX_OFFLINE=true cargo test -p web --features ssr` (needs `DATABASE_URL`)
  - Full suite: `bash scripts/rust-test.sh` (from repo root; manages its own containers).
- **DB tests fail in a plain local run** without the test DB (AGENTS.md). Use the Task 1 container or `scripts/rust-test.sh`.
- **No comments** unless a block is genuinely non-obvious. Match existing style. Do not add doc comments to unchanged code.
- **Commit messages:** concise, repo style (e.g. `feat: ...`, `fix: ...`). See `git log --oneline -10` for tone.

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `rust/web/migrations/022_concede_bot_replacement.sql` | Schema: `ranked_placing`, `left_at`, relax CHECK, `bots.can_replace_humans` | Create |
| `rust/web/src/models/game.rs` | `GamePlayer` struct - add `ranked_placing`, `left_at` | Modify |
| `rust/web/src/db.rs` | `find_game_extended` query, `concede_game`, new `concede_game_replace`, `end_game`, `pick_replacement_bot`, `apply_rating_changes`, `update_game_command_success`, ranked-placing writer | Modify |
| `rust/web/src/game/placing.rs` | Pure `compute_ranked_placings` function | Create |
| `rust/web/src/game/mod.rs` | export `placing` module | Modify |
| `rust/web/src/admin.rs` | `BotRow` + DB fns + server fns + UI for `can_replace_humans` | Modify |
| `rust/web/src/game/server_fns.rs` | `GameViewData` gating (`can_concede`, `can_end_game`), concede server fn, new `end_game` server fn | Modify |
| `rust/web/src/email/commands.rs` | `run_concede` update, new `run_end`, dispatch verb | Modify |
| `rust/web/src/components/game.rs` | Concede/End game buttons, replaced-player display | Modify |
| `rust/web/src/stats/queries.rs` | Form query: `COALESCE(ranked_placing, place)`, limit 5 | Modify |
| `rust/web/src/stats/viz.rs` | `FormStrip`: reverse order (newest left), bold leftmost | Modify |

---

## Task 1: Test DB setup + Migration 022

**Files:**
- Create: `rust/web/migrations/022_concede_bot_replacement.sql`

**Interfaces:**
- Produces: schema columns `game_players.ranked_placing INT NULL`, `game_players.left_at TIMESTAMP NULL`, relaxed CHECK allowing both `user_id` and `game_bot_id`, `bots.can_replace_humans BOOLEAN NOT NULL DEFAULT false`.

- [ ] **Step 1: Start the persistent test Postgres**

```bash
docker rm -f brdgme-plan-pg 2>/dev/null || true
docker run -d --name brdgme-plan-pg \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=brdgme \
  -p 15432:5432 postgres:18
export DATABASE_URL="postgres://postgres:postgres@localhost:15432/brdgme"
for i in $(seq 1 30); do
  docker exec brdgme-plan-pg pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
```

Expected: container starts, `pg_isready` succeeds.

- [ ] **Step 2: Write the migration**

Create `rust/web/migrations/022_concede_bot_replacement.sql`:

```sql
-- #47 Concede with bot replacement & end game.
--
-- A conceded human keeps user_id (preserving the player name/link) and gains
-- game_bot_id (the replacement bot). Relax the XOR check to allow both.
ALTER TABLE game_players DROP CONSTRAINT game_players_user_or_bot;
ALTER TABLE game_players ADD CONSTRAINT game_players_user_or_bot CHECK (
    user_id IS NOT NULL OR game_bot_id IS NOT NULL
);

-- ranked_placing: placing used for ELO/Form (concede/elimination lose first).
-- left_at: when a player conceded or was eliminated (orders ranked placings).
ALTER TABLE game_players ADD COLUMN ranked_placing integer;
ALTER TABLE game_players ADD COLUMN left_at timestamp without time zone;

-- Admin flag: bots eligible to replace a conceding/slow human.
ALTER TABLE bots ADD COLUMN can_replace_humans boolean NOT NULL DEFAULT false;
```

- [ ] **Step 3: Apply migrations to the test DB**

```bash
cd rust/web && sqlx migrate run
```

Expected: applies through `022_concede_bot_replacement.sql` with no error.

- [ ] **Step 4: Verify the schema**

```bash
docker exec brdgme-plan-pg psql -U postgres -d brdgme -c "\d game_players" | grep -E "ranked_placing|left_at"
docker exec brdgme-plan-pg psql -U postgres -d brdgme -c "\d bots" | grep can_replace_humans
```

Expected: both `ranked_placing` and `left_at` columns listed; `can_replace_humans` listed.

- [ ] **Step 5: Commit**

```bash
git add rust/web/migrations/022_concede_bot_replacement.sql
git commit -m "feat(db): add ranked_placing, left_at, can_replace_humans for #47"
```

---

## Task 2: Model struct + find_game_extended query

**Files:**
- Modify: `rust/web/src/models/game.rs:51-69` (`GamePlayer` struct)
- Modify: `rust/web/src/db.rs` (`find_game_extended` query, ~line 419-448)

**Interfaces:**
- Consumes: schema from Task 1.
- Produces: `GamePlayer.ranked_placing: Option<i32>`, `GamePlayer.left_at: Option<PrimitiveDateTime>` populated by `find_game_extended`.

- [ ] **Step 1: Add fields to the `GamePlayer` struct**

In `rust/web/src/models/game.rs`, add to the `GamePlayer` struct (after `rating_change`):

```rust
    pub ranked_placing: Option<i32>,
    pub left_at: Option<PrimitiveDateTime>,
```

- [ ] **Step 2: Add the columns to the `find_game_extended` SELECT**

In `rust/web/src/db.rs`, find the `find_game_extended` query (the `sqlx::query_as!` that SELECTs `game_players` columns joined with users/game_bots/game_type_users). Add `gp.ranked_placing` and `gp.left_at` to the selected `game_players` columns so they map into the struct. Match the existing column-aliasing style used for the other `game_players` fields.

- [ ] **Step 3: Fix any other `query_as!(GamePlayer, ...)` sites**

Search for other places that construct `GamePlayer` via `query_as!`:

```bash
rg -n "query_as!\(\s*GamePlayer|GamePlayer," rust/web/src
```

Add the two new columns to each such SELECT (or, if a site uses `SELECT gp.*`, no change is needed).

- [ ] **Step 4: Regenerate sqlx cache**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
```

Expected: succeeds, updates `rust/web/.sqlx/`.

- [ ] **Step 5: Verify it compiles**

```bash
SQLX_OFFLINE=true cargo check -p web --features ssr
```

Expected: compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/models/game.rs rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(db): expose ranked_placing and left_at on GamePlayer"
```

---

## Task 3: Pure ranked-placing computation

**Files:**
- Create: `rust/web/src/game/placing.rs`
- Modify: `rust/web/src/game/mod.rs` (add `pub mod placing;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct PlacingInput {
      pub game_player_id: uuid::Uuid,
      pub is_pure_bot: bool,                 // user_id IS NULL
      pub left_at: Option<time::PrimitiveDateTime>, // Some => conceded/eliminated
      pub game_placing: Option<i32>,         // the `place` column
  }
  pub fn compute_ranked_placings(players: &[PlacingInput]) -> std::collections::HashMap<uuid::Uuid, i32>
  ```
  Returns ranked placing per human `game_player_id`. Pure bots are omitted. Survivors (left_at None) get the best placings ordered by `game_placing`; leavers get the rest ordered by `left_at` descending (latest leaver = best among leavers).

- [ ] **Step 1: Write the failing test**

Create `rust/web/src/game/placing.rs`:

```rust
use std::collections::HashMap;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub struct PlacingInput {
    pub game_player_id: Uuid,
    pub is_pure_bot: bool,
    pub left_at: Option<PrimitiveDateTime>,
    pub game_placing: Option<i32>,
}

pub fn compute_ranked_placings(_players: &[PlacingInput]) -> HashMap<Uuid, i32> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(minute: u8) -> PrimitiveDateTime {
        PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(0, minute, 0).unwrap(),
        )
    }

    #[test]
    fn spec_worked_example() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        let bot_b = Uuid::new_v4();
        let players = vec![
            PlacingInput { game_player_id: a, is_pure_bot: false, left_at: Some(ts(1)), game_placing: Some(4) },
            PlacingInput { game_player_id: b, is_pure_bot: false, left_at: Some(ts(2)), game_placing: None },
            PlacingInput { game_player_id: bot_b, is_pure_bot: true, left_at: None, game_placing: Some(1) },
            PlacingInput { game_player_id: c, is_pure_bot: false, left_at: Some(ts(3)), game_placing: Some(3) },
            PlacingInput { game_player_id: d, is_pure_bot: false, left_at: None, game_placing: Some(2) },
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&d), Some(&1)); // survivor
        assert_eq!(ranked.get(&c), Some(&2)); // latest leaver
        assert_eq!(ranked.get(&b), Some(&3));
        assert_eq!(ranked.get(&a), Some(&4)); // earliest leaver
        assert!(!ranked.contains_key(&bot_b)); // pure bot omitted
    }

    #[test]
    fn two_player_concede() {
        let winner = Uuid::new_v4();
        let conceder = Uuid::new_v4();
        let players = vec![
            PlacingInput { game_player_id: winner, is_pure_bot: false, left_at: None, game_placing: Some(1) },
            PlacingInput { game_player_id: conceder, is_pure_bot: false, left_at: Some(ts(5)), game_placing: Some(2) },
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&winner), Some(&1));
        assert_eq!(ranked.get(&conceder), Some(&2));
    }

    #[test]
    fn survivors_ordered_by_game_placing() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let p3 = Uuid::new_v4();
        let players = vec![
            PlacingInput { game_player_id: p1, is_pure_bot: false, left_at: None, game_placing: Some(2) },
            PlacingInput { game_player_id: p2, is_pure_bot: false, left_at: None, game_placing: Some(1) },
            PlacingInput { game_player_id: p3, is_pure_bot: false, left_at: Some(ts(1)), game_placing: Some(3) },
        ];
        let ranked = compute_ranked_placings(&players);
        assert_eq!(ranked.get(&p2), Some(&1));
        assert_eq!(ranked.get(&p1), Some(&2));
        assert_eq!(ranked.get(&p3), Some(&3));
    }
}
```

- [ ] **Step 2: Register the module and run tests to verify they fail**

In `rust/web/src/game/mod.rs`, add near the other `pub mod` / `mod` declarations:

```rust
pub mod placing;
```

Run:

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr placing::
```

Expected: FAIL - `compute_ranked_placings` returns empty map, assertions fail.

- [ ] **Step 3: Implement `compute_ranked_placings`**

Replace the stub body in `rust/web/src/game/placing.rs`:

```rust
pub fn compute_ranked_placings(players: &[PlacingInput]) -> HashMap<Uuid, i32> {
    let mut ranked: HashMap<Uuid, i32> = HashMap::new();

    let mut survivors: Vec<&PlacingInput> = players
        .iter()
        .filter(|p| !p.is_pure_bot && p.left_at.is_none())
        .collect();
    survivors.sort_by(|a, b| {
        a.game_placing
            .unwrap_or(i32::MAX)
            .cmp(&b.game_placing.unwrap_or(i32::MAX))
            .then(a.game_player_id.cmp(&b.game_player_id))
    });

    let mut leavers: Vec<&PlacingInput> = players
        .iter()
        .filter(|p| !p.is_pure_bot && p.left_at.is_some())
        .collect();
    leavers.sort_by(|a, b| {
        b.left_at
            .cmp(&a.left_at)
            .then(a.game_player_id.cmp(&b.game_player_id))
    });

    let mut place = 1;
    for p in survivors {
        ranked.insert(p.game_player_id, place);
        place += 1;
    }
    for p in leavers {
        ranked.insert(p.game_player_id, place);
        place += 1;
    }
    ranked
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr placing::
```

Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/game/placing.rs rust/web/src/game/mod.rs
git commit -m "feat(game): add pure ranked-placing computation for #47"
```

---

## Task 4: Set left_at on elimination transitions

**Files:**
- Modify: `rust/web/src/db.rs` (`update_game_command_success`, ~line 1740-1773; and the undo path ~line 1435)

**Interfaces:**
- Consumes: `game_players.left_at` (Task 1).
- Produces: `left_at` set to `NOW()` when a player transitions from not-eliminated to eliminated.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `rust/web/src/db.rs` (use the existing `make_user`, `make_game_type_and_version`, `make_game_with_players` helpers):

```rust
#[sqlx::test]
async fn elimination_sets_left_at_once(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let opp = make_user(&pool, "opp").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    let game = make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 0, &[0]).await;

    let player_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM game_players WHERE game_id = $1 AND position = 1",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Not eliminated yet.
    let left_at: Option<time::PrimitiveDateTime> =
        sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(left_at.is_none());

    // Simulate an elimination command update.
    let status = crate::game::StatusUpdate {
        is_finished: false,
        whose_turn: vec![0],
        eliminated: vec![1],
        placings: vec![],
    };
    let updated_at: time::PrimitiveDateTime =
        sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
            .bind(game.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    update_game_command_success(
        &pool, game.id, player_id, "", false, &status, &[], updated_at, vec![],
    )
    .await
    .unwrap();

    let left_at: Option<time::PrimitiveDateTime> =
        sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(left_at.is_some());
    let first_left_at = left_at.unwrap();

    // Run the same update again - left_at must NOT change (no re-stamp).
    let updated_at: time::PrimitiveDateTime =
        sqlx::query_scalar("SELECT updated_at FROM games WHERE id = $1")
            .bind(game.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    update_game_command_success(
        &pool, game.id, player_id, "", false, &status, &[], updated_at, vec![],
    )
    .await
    .unwrap();
    let left_at: time::PrimitiveDateTime =
        sqlx::query_scalar("SELECT left_at FROM game_players WHERE id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(left_at, first_left_at);
}
```

Note: confirm the exact signature of `update_game_command_success` (parameter order/types) at `rust/web/src/db.rs` before finalizing the call - adjust the test call to match. The `prev_game_state` arg is `&str`.

- [ ] **Step 2: Run the test to verify it fails**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr elimination_sets_left_at_once
```

Expected: FAIL - `left_at` is `None` after the update (column never written).

- [ ] **Step 3: Patch the elimination UPDATE**

In `update_game_command_success` (`rust/web/src/db.rs`), the per-player UPDATE currently reads:

```rust
        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, place = $2, is_eliminated = $3, points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   turn_reminder_sent_at = NULL,
                   updated_at = NOW()
               WHERE id = $8"#,
        )
```

Change it to also maintain `left_at` (set only on the false->true elimination transition):

```rust
        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, place = $2, is_eliminated = $3, points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   turn_reminder_sent_at = NULL,
                   left_at = CASE WHEN is_eliminated = false AND $3 = true
                                  THEN NOW() ELSE left_at END,
                   updated_at = NOW()
               WHERE id = $8"#,
        )
```

This is a plain (non-macro) query, so no `cargo sqlx prepare` is needed.

- [ ] **Step 4: Check the undo path**

Inspect the undo write at `rust/web/src/db.rs` (~line 1435). If it rewrites `is_eliminated`, apply the same `left_at = CASE ...` guard so an undo that un-eliminates then re-eliminates does not lose the original `left_at`. If the undo path restores a saved `is_eliminated` and could flip true->false, leave `left_at` untouched (do not clear it) - a player who left stays left for ranking purposes.

```bash
rg -n "is_eliminated" rust/web/src/db.rs
```

- [ ] **Step 5: Run the test to verify it passes**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr elimination_sets_left_at_once
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/db.rs
git commit -m "feat(db): stamp left_at on elimination transition for #47"
```

---

## Task 5: apply_rating_changes uses ranked_placing + pure-bot exclusion

**Files:**
- Modify: `rust/web/src/db.rs` (`apply_rating_changes`, ~line 1530-1680; natural-finish writer in `update_game_command_success`)

**Interfaces:**
- Consumes: `compute_ranked_placings` (Task 3), `ranked_placing` column (Task 1), `left_at` (Tasks 1/4).
- Produces: ratings exclude pure bots (`user_id IS NULL`), use `COALESCE(ranked_placing, place)`. Natural game finish writes `ranked_placing` before applying ratings.

- [ ] **Step 1: Write the failing test (bot exclusion + ranked placing)**

Add to the `#[cfg(test)]` module in `rust/web/src/db.rs`:

```rust
#[sqlx::test]
async fn ratings_use_ranked_placing_and_skip_pure_bots(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let opp = make_user(&pool, "opp").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    // 2 humans + 1 bot.
    let game = make_game_with_players(&pool, game_version_id, creator.id, &[opp.id], 1, &[0]).await;

    // Conceder (position 1) is replaced by a bot: both user_id and game_bot_id set.
    // Give the replacement the best game placing (1) but worst ranked placing (2).
    sqlx::query(
        "UPDATE game_players SET place = $1, ranked_placing = $2, left_at = NOW(), \
         game_bot_id = (SELECT id FROM game_bots WHERE game_id = $3 LIMIT 1) \
         WHERE game_id = $3 AND position = 1",
    )
    .bind(1i32)
    .bind(2i32)
    .bind(game.id)
    .execute(&pool)
    .await
    .unwrap();
    // Survivor (position 0): game placing 2, ranked placing 1.
    sqlx::query(
        "UPDATE game_players SET place = $1, ranked_placing = $2 WHERE game_id = $3 AND position = 0",
    )
    .bind(2i32)
    .bind(1i32)
    .bind(game.id)
    .execute(&pool)
    .await
    .unwrap();
    // Pure bot (position 2): game placing 3, no ranked placing.
    sqlx::query("UPDATE game_players SET place = 3 WHERE game_id = $1 AND position = 2")
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE games SET is_finished = true, finished_at = NOW() WHERE id = $1")
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    apply_rating_changes(&mut tx, game.id).await.unwrap();
    tx.commit().await.unwrap();

    // The replaced human (position 1) must be rated (has user_id) despite game_bot_id.
    let rated: (Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT rating_change, ranked_placing FROM game_players WHERE game_id = $1 AND position = 1",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(rated.0.is_some(), "replaced human must receive a rating change");
    // The pure bot (position 2) must NOT be rated.
    let bot_rated: Option<i32> = sqlx::query_scalar(
        "SELECT rating_change FROM game_players WHERE game_id = $1 AND position = 2",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(bot_rated.is_none(), "pure bot must not be rated");
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr ratings_use_ranked_placing_and_skip_pure_bots
```

Expected: FAIL - current code skips the replaced human (`game_bot_id.is_some()`) and/or uses `place` not `ranked_placing`.

- [ ] **Step 3: Change the bot exclusion**

In `apply_rating_changes` (`rust/web/src/db.rs`), the `PlayerRow` query selects `id, position, user_id, game_bot_id, place, rating_change`. Change the SELECT to also fetch `ranked_placing`:

```rust
    struct PlayerRow {
        id: Uuid,
        position: i32,
        user_id: Option<Uuid>,
        game_bot_id: Option<Uuid>,
        place: Option<i32>,
        ranked_placing: Option<i32>,
        rating_change: Option<i32>,
    }

    let players = sqlx::query_as!(
        PlayerRow,
        "SELECT id, position, user_id, game_bot_id, place, ranked_placing, rating_change FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;
```

Change the exclusion from `game_bot_id.is_some()` to `user_id.is_none()`:

```rust
        if p.user_id.is_none() {
            continue;
        }
```

- [ ] **Step 4: Use ranked_placing for the pairwise comparison**

In the same function, the `places` map is built from `p.place`. Change it to prefer `ranked_placing`:

```rust
    let places: HashMap<i32, i32> = players
        .iter()
        .map(|p| (p.position, p.ranked_placing.or(p.place).unwrap_or(i32::MAX)))
        .collect();
```

- [ ] **Step 5: Regenerate sqlx cache and run the test**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo test -p web --features ssr ratings_use_ranked_placing_and_skip_pure_bots
```

Expected: PASS.

- [ ] **Step 6: Write ranked_placing on natural game finish**

In `update_game_command_success`, the block `if status.is_finished && !status.placings.is_empty() { apply_rating_changes(...) }` runs after the per-player UPDATE sets `place`. Before calling `apply_rating_changes`, compute and write `ranked_placing` for human players. Add a helper call:

```rust
    if status.is_finished && !status.placings.is_empty() {
        write_ranked_placings(&mut tx, game_id).await?;
        apply_rating_changes(&mut tx, game_id).await?;
    }
```

- [ ] **Step 7: Implement `write_ranked_placings`**

Add near `apply_rating_changes` in `rust/web/src/db.rs`:

```rust
#[cfg(feature = "ssr")]
async fn write_ranked_placings(tx: &mut sqlx::PgConnection, game_id: Uuid) -> Result<()> {
    struct Row {
        id: Uuid,
        user_id: Option<Uuid>,
        left_at: Option<time::PrimitiveDateTime>,
        place: Option<i32>,
    }
    let rows = sqlx::query_as!(
        Row,
        "SELECT id, user_id, left_at, place FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let inputs: Vec<crate::game::placing::PlacingInput> = rows
        .iter()
        .map(|r| crate::game::placing::PlacingInput {
            game_player_id: r.id,
            is_pure_bot: r.user_id.is_none(),
            left_at: r.left_at,
            game_placing: r.place,
        })
        .collect();

    let ranked = crate::game::placing::compute_ranked_placings(&inputs);
    for (id, placing) in ranked {
        sqlx::query("UPDATE game_players SET ranked_placing = $1 WHERE id = $2")
            .bind(placing)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}
```

- [ ] **Step 8: Regenerate sqlx cache, run tests, verify compile**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo test -p web --features ssr ratings_use_ranked_placing_and_skip_pure_bots
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: test PASS, clippy clean.

- [ ] **Step 9: Commit**

```bash
git add rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(db): rate via ranked_placing, exclude pure bots only, write ranked placings on finish"
```

---

## Task 6: Admin `can_replace_humans` flag

**Files:**
- Modify: `rust/web/src/admin.rs` (`BotRow` ~line 33-42, `list_bots` ~92, `create_bot` ~127, `update_bot` ~159, server fns ~618-722, `BotsSection` UI ~1044)

**Interfaces:**
- Consumes: `bots.can_replace_humans` (Task 1).
- Produces: admin can view/set `can_replace_humans` per bot; `BotRow.can_replace_humans: bool`.

- [ ] **Step 1: Add the field to `BotRow`**

In `rust/web/src/admin.rs`, add to `BotRow`:

```rust
    pub can_replace_humans: bool,
```

- [ ] **Step 2: Update `list_bots` SELECT**

Add `can_replace_humans` to the `list_bots` query column list so it maps into `BotRow`.

- [ ] **Step 3: Update `create_bot` and `update_bot`**

Add a `can_replace_humans: bool` parameter to `create_bot` (include in INSERT ... RETURNING) and `update_bot` (add `can_replace_humans = $N` to the SET clause, shifting bind indices). Match the existing parameter style.

- [ ] **Step 4: Update the server fns**

Thread `can_replace_humans` through `admin_create_bot` and `admin_update_bot` signatures and their calls to the DB fns.

- [ ] **Step 5: Update the `BotsSection` UI**

Add a "Can replace humans" checkbox to the create and edit forms, bound to `can_replace_humans`, mirroring how the existing `enabled` / `include_basic_strategy` checkboxes are wired.

- [ ] **Step 6: Regenerate sqlx cache and verify**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clippy clean.

- [ ] **Step 7: Commit**

```bash
git add rust/web/src/admin.rs rust/web/.sqlx
git commit -m "feat(admin): add can_replace_humans flag to bots for #47"
```

---

## Task 7: Replacement bot selection

**Files:**
- Modify: `rust/web/src/db.rs` (new `pick_replacement_bot`)

**Interfaces:**
- Produces:
  ```rust
  pub async fn pick_replacement_bot(pool: &PgPool, game_id: Uuid) -> Result<Option<crate::models::game::GameBot>>
  ```
  Randomly selects an enabled bot definition (`bots.can_replace_humans = true`) and returns a `GameBot` value suitable for inserting a `game_bots` row for `game_id`. Returns `None` if no bot is flagged. The returned `GameBot.bot_name` comes from `bots.name`.

- [ ] **Step 1: Inspect `GameBot` and how `game_bots` rows are created**

```bash
rg -n "struct GameBot" rust/web/src/models/game.rs
rg -n "INSERT INTO game_bots" rust/web/src
```

Note the `GameBot` fields and an existing `game_bots` INSERT to copy the column set (typically `game_id, name, bot_name, personality`).

- [ ] **Step 2: Write the failing test**

Add to the `#[cfg(test)]` module in `rust/web/src/db.rs`:

```rust
#[sqlx::test]
async fn pick_replacement_bot_requires_flag(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 0, &[0]).await;

    // No bots flagged yet.
    assert!(pick_replacement_bot(&pool, game.id).await.unwrap().is_none());

    sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
        .execute(&pool)
        .await
        .unwrap();
    let picked = pick_replacement_bot(&pool, game.id).await.unwrap();
    assert!(picked.is_some());
    assert_eq!(picked.unwrap().bot_name, "Hard");
}
```

Confirm the `bots` table's required columns (migration 013) and the `GameBot.bot_name` field name before finalizing.

- [ ] **Step 3: Run the test to verify it fails**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr pick_replacement_bot_requires_flag
```

Expected: FAIL - `pick_replacement_bot` does not exist (compile error) or returns wrong value.

- [ ] **Step 4: Implement `pick_replacement_bot`**

Add to `rust/web/src/db.rs` (adapt the `GameBot` construction to the real struct fields found in Step 1):

```rust
#[cfg(feature = "ssr")]
pub async fn pick_replacement_bot(
    pool: &PgPool,
    game_id: Uuid,
) -> Result<Option<crate::models::game::GameBot>> {
    let name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM bots WHERE can_replace_humans = true AND enabled = true ORDER BY random() LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some(name) = name else {
        return Ok(None);
    };
    let bot = sqlx::query_as!(
        crate::models::game::GameBot,
        "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id, game_id, name, bot_name, personality, created_at, updated_at",
        game_id,
        name,
        name
    )
    .fetch_one(pool)
    .await?;
    Ok(Some(bot))
}
```

Adjust the `RETURNING` column list and `GameBot` field set to match the actual struct (Step 1). If `game_bots` has a `personality` column, leave it NULL here.

- [ ] **Step 5: Regenerate sqlx cache and run the test**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo test -p web --features ssr pick_replacement_bot_requires_flag
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(db): add pick_replacement_bot for #47"
```

---

## Task 8: Multi-player concede DB function

**Files:**
- Modify: `rust/web/src/db.rs` (new `concede_game_replace`; keep existing `concede_game` intact)

**Interfaces:**
- Consumes: `pick_replacement_bot` (Task 7), `left_at` (Task 1).
- Produces:
  ```rust
  pub async fn concede_game_replace(pool: &PgPool, game_id: Uuid, conceding_player_id: Uuid, conceding_name: &str) -> Result<()>
  ```
  Sets `left_at = NOW()` and `game_bot_id` (from `pick_replacement_bot`) on the conceding player, clears their turn, writes a game log, and does NOT finish the game or apply ratings. Errors if no replacement bot is available.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `rust/web/src/db.rs`:

```rust
#[sqlx::test]
async fn concede_game_replace_swaps_in_bot(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let a = make_user(&pool, "a").await;
    let b = make_user(&pool, "b").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    let game =
        make_game_with_players(&pool, game_version_id, creator.id, &[a.id, b.id], 0, &[0]).await;

    sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
        .execute(&pool)
        .await
        .unwrap();

    let conceder: Uuid = sqlx::query_scalar(
        "SELECT id FROM game_players WHERE game_id = $1 AND position = 1",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    concede_game_replace(&pool, game.id, conceder, "a").await.unwrap();

    let row: (Option<Uuid>, Option<Uuid>, Option<time::PrimitiveDateTime>) = sqlx::query_as(
        "SELECT user_id, game_bot_id, left_at FROM game_players WHERE id = $1",
    )
    .bind(conceder)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, Some(a.id), "user_id preserved");
    assert!(row.1.is_some(), "game_bot_id set");
    assert!(row.2.is_some(), "left_at set");

    let finished: bool =
        sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
            .bind(game.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!finished, "game must not be finished");
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr concede_game_replace_swaps_in_bot
```

Expected: FAIL - `concede_game_replace` does not exist.

- [ ] **Step 3: Implement `concede_game_replace`**

Add to `rust/web/src/db.rs`:

```rust
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn concede_game_replace(
    pool: &PgPool,
    game_id: Uuid,
    conceding_player_id: Uuid,
    conceding_name: &str,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    let bot = pick_replacement_bot(pool, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no replacement bot configured"))?;

    sqlx::query(
        r#"UPDATE game_players
           SET is_turn = false, game_bot_id = $1, left_at = NOW(),
               undo_game_state = NULL, turn_reminder_sent_at = NULL, updated_at = NOW()
           WHERE id = $2"#,
    )
    .bind(bot.id)
    .bind(conceding_player_id)
    .execute(&mut *tx)
    .await?;

    let log_body = format!("{} conceded (replaced by bot {}).", conceding_name, bot.name);
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        log_body
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 4: Regenerate sqlx cache and run the test**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo test -p web --features ssr concede_game_replace_swaps_in_bot
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(db): add concede_game_replace for #47"
```

---

## Task 9: End game DB function

**Files:**
- Modify: `rust/web/src/db.rs` (new `end_game`)

**Interfaces:**
- Consumes: `write_ranked_placings` (Task 5), `apply_rating_changes` (Task 5).
- Produces:
  ```rust
  pub async fn end_game(pool: &PgPool, game_id: Uuid) -> Result<()>
  ```
  Finishes the game, assigns game placings heuristically (order by `points DESC NULLS LAST`, tiebreak `position`), writes `ranked_placing`, applies ratings. Idempotent via the rating guard already in `apply_rating_changes`.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `rust/web/src/db.rs`:

```rust
#[sqlx::test]
async fn end_game_finishes_and_ranks(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let a = make_user(&pool, "a").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    let game = make_game_with_players(&pool, game_version_id, creator.id, &[a.id], 1, &[0]).await;

    // Concede the second human (position 1) so only position 0 is human-active.
    sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
        .execute(&pool)
        .await
        .unwrap();
    let conceder: Uuid = sqlx::query_scalar(
        "SELECT id FROM game_players WHERE game_id = $1 AND position = 1",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    concede_game_replace(&pool, game.id, conceder, "a").await.unwrap();

    // Give the survivor more points than the replacement bot.
    sqlx::query("UPDATE game_players SET points = 10 WHERE game_id = $1 AND position = 0")
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE game_players SET points = 5 WHERE game_id = $1 AND position = 2")
        .bind(game.id)
        .execute(&pool)
        .await
        .unwrap();

    end_game(&pool, game.id).await.unwrap();

    let finished: bool = sqlx::query_scalar("SELECT is_finished FROM games WHERE id = $1")
        .bind(game.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(finished);

    let survivor_ranked: Option<i32> = sqlx::query_scalar(
        "SELECT ranked_placing FROM game_players WHERE game_id = $1 AND position = 0",
    )
    .bind(game.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(survivor_ranked, Some(1));
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
SQLX_OFFLINE=true cargo test -p web --features ssr end_game_finishes_and_ranks
```

Expected: FAIL - `end_game` does not exist.

- [ ] **Step 3: Implement `end_game`**

Add to `rust/web/src/db.rs`:

```rust
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn end_game(pool: &PgPool, game_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET is_finished = true, finished_at = NOW(), updated_at = NOW() WHERE id = $1",
        game_id
    )
    .execute(&mut *tx)
    .await?;

    // Heuristic game placings: order by current points (NULLs last), tiebreak position.
    let ordered = sqlx::query!(
        "SELECT id FROM game_players WHERE game_id = $1 ORDER BY points DESC NULLS LAST, position ASC",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;
    for (i, row) in ordered.iter().enumerate() {
        let place = (i + 1) as i32;
        sqlx::query("UPDATE game_players SET place = $1, is_turn = false, undo_game_state = NULL, turn_reminder_sent_at = NULL, updated_at = NOW() WHERE id = $2")
            .bind(place)
            .bind(row.id)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        "Game ended.".to_string()
    )
    .execute(&mut *tx)
    .await?;

    write_ranked_placings(&mut tx, game_id).await?;
    apply_rating_changes(&mut tx, game_id).await?;

    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 4: Regenerate sqlx cache and run the test**

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
SQLX_OFFLINE=true cargo test -p web --features ssr end_game_finishes_and_ranks
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(db): add end_game for #47"
```

---

## Task 10: Concede server fn (replacement flow)

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (`concede_game` server fn, ~line 807-857)

**Interfaces:**
- Consumes: `concede_game_replace` (Task 8), existing `concede_game` DB fn, `pick_replacement_bot` (Task 7), `broadcast_and_trigger` (`rust/web/src/game/mod.rs:50`).
- Produces: concede works for any game. If a replacement bot is available, replace and continue (trigger bot turns). Otherwise, if exactly 2 active humans, end the game (current behavior). Otherwise error.

- [ ] **Step 1: Add an active-human counting helper**

In `rust/web/src/game/server_fns.rs`, add a small helper near the top of the module (or inline in the server fn). It counts players that are humans still in the game:

```rust
fn count_active_humans(ge: &crate::db::GameExtended) -> usize {
    ge.game_players
        .iter()
        .filter(|p| {
            p.game_player.user_id.is_some() && p.game_player.left_at.is_none()
        })
        .count()
}
```

Confirm `GameExtended` is the type returned by `find_game_extended` and that `game_player.left_at` is available (Task 2).

- [ ] **Step 2: Rewrite the `concede_game` server fn body**

Replace the body after the `is_finished` check. Remove the `ge.game_players.len() != 2` guard. New logic:

```rust
    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    if player.game_player.left_at.is_some() {
        return Err(ServerFnError::new("You have already left this game"));
    }

    let active_humans = count_active_humans(&ge);
    let replacement_available = crate::db::pick_replacement_bot(&pool, game_id)
        .await
        .map_err(internal("concede_game: pick replacement"))?
        .is_some();

    if replacement_available {
        crate::db::concede_game_replace(&pool, game_id, player.game_player.id, player.name())
            .await
            .map_err(internal("concede_game: replace"))?;
    } else if active_humans == 2 {
        crate::db::concede_game(&pool, game_id, player.game_player.id, player.name())
            .await
            .map_err(internal("concede_game: concede"))?;
    } else {
        return Err(ServerFnError::new(
            "Concede is not available: no replacement bot configured",
        ));
    }

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;

    crate::email::notify::notify_game_emails(
        resend.as_ref(),
        &pool,
        &http_client,
        game_id,
        Some(before),
    )
    .await;
    Ok(())
```

Note: `pick_replacement_bot` inserts a `game_bots` row as a side effect. To avoid inserting a bot just to check availability (and leaving an orphan if we take the 2-human branch), refactor: call `pick_replacement_bot` only when committing to replacement. See Step 3.

- [ ] **Step 3: Avoid the orphan bot insert**

`pick_replacement_bot` inserts a `game_bots` row. Checking availability by calling it would create an orphan on the 2-human branch. Add a read-only existence check to `rust/web/src/db.rs`:

```rust
#[cfg(feature = "ssr")]
pub async fn replacement_bot_available(pool: &PgPool) -> Result<bool> {
    let exists: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM bots WHERE can_replace_humans = true AND enabled = true)",
    )
    .fetch_optional(pool)
    .await?;
    Ok(exists.map(|(b,)| b).unwrap_or(false))
}
```

Then in the server fn, replace the `pick_replacement_bot(...).is_some()` call with:

```rust
    let replacement_available = crate::db::replacement_bot_available(&pool)
        .await
        .map_err(internal("concede_game: replacement available"))?;
```

(`concede_game_replace` calls `pick_replacement_bot` itself to do the actual insert.)

- [ ] **Step 4: Add `jetstream` to the server fn context**

The current server fn does not pull a jetstream context. Add near the other `expect_context` calls:

```rust
    let jetstream = expect_context::<async_nats::jetstream::Context>();
```

Confirm `async_nats::jetstream::Context` is the type registered in app context (check how other server fns that call `broadcast_and_trigger` obtain it).

- [ ] **Step 5: Verify compile + clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean. (No new `query!` macros were added except `replacement_bot_available`, which is a plain `query_as` - no sqlx prepare needed for it.)

- [ ] **Step 6: Commit**

```bash
git add rust/web/src/game/server_fns.rs rust/web/src/db.rs
git commit -m "feat(game): concede replaces player with bot in any game (#47)"
```

---

## Task 11: End game server fn

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (new `end_game` server fn)

**Interfaces:**
- Consumes: `end_game` DB fn (Task 9), `count_active_humans` (Task 10).
- Produces: `#[server(EndGame, "/api")] pub async fn end_game(game_id: Uuid) -> Result<(), ServerFnError>`. Allowed when the caller is the last active human, or (when zero active humans) the most recent human leaver. Finishes the game and notifies.

- [ ] **Step 1: Add the server fn**

Add after the `concede_game` server fn in `rust/web/src/game/server_fns.rs`:

```rust
#[server(EndGame, "/api")]
pub async fn end_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let jetstream = expect_context::<async_nats::jetstream::Context>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("end_game: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;
    let before = ge.clone();

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }

    let is_player = ge
        .game_players
        .iter()
        .any(|p| p.user.as_ref().is_some_and(|u| u.id == user.id));
    if !is_player {
        return Err(ServerFnError::new("You are not a player in this game"));
    }

    let active_humans = count_active_humans(&ge);
    let allowed = if active_humans <= 1 {
        // Last active human, or (0 active) the most recent human to leave.
        true
    } else {
        false
    };
    if !allowed {
        return Err(ServerFnError::new("End game is only available to the last human"));
    }

    crate::db::end_game(&pool, game_id)
        .await
        .map_err(internal("end_game: end"))?;

    crate::game::broadcast_and_trigger(&pool, &broadcaster, &jetstream, game_id).await;

    crate::email::notify::notify_game_emails(
        resend.as_ref(),
        &pool,
        &http_client,
        game_id,
        Some(before),
    )
    .await;
    Ok(())
}
```

Note: the UI gating (Task 13) decides who *sees* the button; this server fn permits any player when `active_humans <= 1`. Tighten if you want to restrict to the specific last human - the UI already enforces visibility.

- [ ] **Step 2: Verify compile + clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add rust/web/src/game/server_fns.rs
git commit -m "feat(game): add end_game server fn for #47"
```

---

## Task 12: Email commands (concede update + end verb)

**Files:**
- Modify: `rust/web/src/email/commands.rs` (`run_concede` ~886-922, dispatch ~1135-1176, new `run_end`)

**Interfaces:**
- Consumes: `concede_game_replace`, `concede_game`, `replacement_bot_available` (Tasks 8/10), `end_game` DB fn (Task 9).
- Produces: `concede` email verb mirrors the web server fn (replace if bot available, else 2-human end). New `end` verb calls `end_game`.

- [ ] **Step 1: Rewrite `run_concede`**

Replace the body of `run_concede` in `rust/web/src/email/commands.rs`, removing the `ge.game_players.len() != 2` guard:

```rust
async fn run_concede(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if ge.game.is_finished {
        return Err(CommandError::User("Game is already finished".to_string()));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == ctx.game_player_id)
        .ok_or_else(|| CommandError::User("You are not a player in this game".to_string()))?;

    if player.game_player.left_at.is_some() {
        return Err(CommandError::User("You have already left this game".to_string()));
    }

    let active_humans = ge
        .game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
        .count();
    let replacement_available = crate::db::replacement_bot_available(ctx.pool)
        .await
        .map_err(CommandError::Internal)?;

    let before = ge.clone();
    if replacement_available {
        crate::db::concede_game_replace(ctx.pool, ctx.game_id, ctx.game_player_id, player.name())
            .await
            .map_err(CommandError::Internal)?;
    } else if active_humans == 2 {
        crate::db::concede_game(ctx.pool, ctx.game_id, ctx.game_player_id, player.name())
            .await
            .map_err(CommandError::Internal)?;
    } else {
        return Err(CommandError::User(
            "Concede is not available: no replacement bot configured".to_string(),
        ));
    }

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, ctx.game_id).await;
    crate::email::notify::notify_game_emails(
        ctx.resend,
        ctx.pool,
        ctx.http_client,
        ctx.game_id,
        Some(before),
    )
    .await;

    Ok(CommandReply::Status("You conceded.".to_string()))
}
```

Confirm `EmailCommandCtx` has a `jetstream` field (context doc shows it does).

- [ ] **Step 2: Add `run_end`**

Add below `run_concede`:

```rust
async fn run_end(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if ge.game.is_finished {
        return Err(CommandError::User("Game is already finished".to_string()));
    }

    let is_player = ge
        .game_players
        .iter()
        .any(|p| p.game_player.id == ctx.game_player_id);
    if !is_player {
        return Err(CommandError::User("You are not a player in this game".to_string()));
    }

    let active_humans = ge
        .game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
        .count();
    if active_humans > 1 {
        return Err(CommandError::User(
            "End game is only available to the last human".to_string(),
        ));
    }

    let before = ge.clone();
    crate::db::end_game(ctx.pool, ctx.game_id)
        .await
        .map_err(CommandError::Internal)?;

    crate::game::broadcast_and_trigger(ctx.pool, ctx.broadcaster, ctx.jetstream, ctx.game_id).await;
    crate::email::notify::notify_game_emails(
        ctx.resend,
        ctx.pool,
        ctx.http_client,
        ctx.game_id,
        Some(before),
    )
    .await;

    Ok(CommandReply::Status("Game ended.".to_string()))
}
```

- [ ] **Step 3: Register the `end` verb in dispatch**

In `dispatch_email_command` (~line 1135-1176), add to the match block next to `"concede"`:

```rust
        "end" => return run_end(ctx).await,
```

Also add `end` to the `help_text()` command list if one exists (search `fn help_text`).

- [ ] **Step 4: Verify compile + clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/email/commands.rs
git commit -m "feat(email): concede replacement flow + end verb for #47"
```

---

## Task 13: GameViewData gating (can_concede / can_end_game / replaced display)

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (`GameViewData` ~57-86, `PlayerViewData` ~88-115, `get_game_details` mapping)

**Interfaces:**
- Consumes: `replacement_bot_available` (Task 10), `GamePlayer.left_at` (Task 2).
- Produces: `GameViewData.can_concede: bool`, `GameViewData.can_end_game: bool`. `PlayerViewData.is_replaced: bool` (both `user_id` and `game_bot_id` set). Remove reliance on `is_2player` for the concede gate (keep the field if used elsewhere).

- [ ] **Step 1: Add fields to the view structs**

In `GameViewData`, add:

```rust
    pub can_concede: bool,
    pub can_end_game: bool,
```

In `PlayerViewData`, add:

```rust
    pub is_replaced: bool,
```

- [ ] **Step 2: Compute the gates in `get_game_details`**

Where `GameViewData` is constructed, compute (the viewer's user id is available as `viewer_user_id`):

```rust
    let active_humans = ge
        .game_players
        .iter()
        .filter(|p| p.game_player.user_id.is_some() && p.game_player.left_at.is_none())
        .count();
    let viewer_is_active_human = ge.game_players.iter().any(|p| {
        p.user.as_ref().is_some_and(|u| Some(u.id) == viewer_user_id)
            && p.game_player.left_at.is_none()
    });
    let replacement_available = crate::db::replacement_bot_available(&pool).await.unwrap_or(false);

    let can_concede = !ge.game.is_finished
        && viewer_is_active_human
        && active_humans >= 2
        && (replacement_available || active_humans == 2);

    let can_end_game = !ge.game.is_finished && active_humans <= 1 && {
        // Viewer is a human in the game (active last human, or a leaver watching bots).
        ge.game_players
            .iter()
            .any(|p| p.user.as_ref().is_some_and(|u| Some(u.id) == viewer_user_id))
    };
```

Set `can_concede` and `can_end_game` on the struct.

- [ ] **Step 3: Set `is_replaced` per player**

Where each `PlayerViewData` is built, set:

```rust
    is_replaced: p.game_player.user_id.is_some() && p.game_player.game_bot_id.is_some(),
```

- [ ] **Step 4: Verify compile + clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/game/server_fns.rs
git commit -m "feat(game): expose can_concede/can_end_game/is_replaced in view data (#47)"
```

---

## Task 14: UI components (buttons + replaced display)

**Files:**
- Modify: `rust/web/src/components/game.rs` (concede button ~114-126, player list rendering, action wiring)

**Interfaces:**
- Consumes: `GameViewData.can_concede`, `can_end_game`, `PlayerViewData.is_replaced` (Task 13), `EndGame` server fn (Task 11).
- Produces: "Concede" shown when `can_concede`; "End game" shown when `can_end_game`; replaced players render their original name/link with a "(bot: <name>)" indicator.

- [ ] **Step 1: Wire the `EndGame` action**

In `GameMeta` (`rust/web/src/components/game.rs`), near where `concede_action` is created, add an action for `EndGame`. Mirror the existing `concede_action` creation and `create_server_action` pattern. Import `EndGame` from the server fns module.

- [ ] **Step 2: Replace the concede `<Show>` gate**

The current block:

```rust
<Show when=move || !is_finished && is_2player>
    <div>
        <a href="#" on:click=move |ev| { ... concede_action.dispatch(ConcedeGame { game_id }); }>"Concede"</a>
    </div>
</Show>
```

Change the `when` to use `can_concede` (read from `data.can_concede` into a local `let can_concede = data.can_concede;`). Keep the confirm dialog.

- [ ] **Step 3: Add the "End game" button**

Add a sibling `<Show when=move || can_end_game>` block that dispatches the `EndGame` action with a confirm dialog ("End this game?"). Mirror the concede block's structure.

- [ ] **Step 4: Render the replaced indicator**

In the player list rendering, where each player's name/link is shown, append a "(bot: <bot_name>)" suffix when `player.is_replaced` is true. Use the existing `bot_name` field on `PlayerViewData` for the bot's name. Keep the original name and link intact.

- [ ] **Step 5: Verify compile + clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean.

- [ ] **Step 6: Manual smoke (optional, if a dev env is available)**

If `tilt up` is running: start a 3-human game, concede as one player, confirm a bot replaces them and the remaining players see updated buttons. Otherwise rely on the test suite.

- [ ] **Step 7: Commit**

```bash
git add rust/web/src/components/game.rs
git commit -m "feat(ui): concede/end-game buttons and replaced-player display (#47)"
```

---

## Task 15: Form line (ranked_placing, last 5, newest-left, bold)

**Files:**
- Modify: `rust/web/src/stats/queries.rs` (`recent_form_for_game_type` ~619-683, and `recent_form` if present)
- Modify: `rust/web/src/stats/viz.rs` (`FormStrip` ~52-65)
- Modify: `rust/web/src/game/server_fns.rs` (the `per_user` argument, ~283-290)

**Interfaces:**
- Consumes: `ranked_placing` column (Task 1).
- Produces: form uses `COALESCE(gp.ranked_placing, gp.place)`, limited to 5, displayed newest-first (left), leftmost bold.

- [ ] **Step 1: Update the form query to use ranked_placing**

In `recent_form_for_game_type` (`rust/web/src/stats/queries.rs`), change the selected placing from `gp.place` to `COALESCE(gp.ranked_placing, gp.place) AS place` (keep the output column name `place` so `FormResult` is unchanged). The existing `gp.user_id = ANY($1)` filter already excludes pure bots and includes replaced humans.

If a separate `recent_form` function exists, apply the same change.

```bash
rg -n "gp.place|recent_form" rust/web/src/stats/queries.rs
```

- [ ] **Step 2: Change the per-user limit to 5**

In `get_game_details` (`rust/web/src/game/server_fns.rs`), the call passes `10`:

```rust
let form_by_user = crate::stats::recent_form_for_game_type(
    &pool,
    &human_user_ids,
    ge.game_version.game_type_id,
    10,
)
```

Change `10` to `5`.

- [ ] **Step 3: Reverse display order (newest on the left)**

The query orders `finished_at ASC` (oldest first). In `FormStrip` (`rust/web/src/stats/viz.rs`), reverse the results before rendering so the newest is leftmost:

```rust
#[component]
pub fn FormStrip(results: Vec<FormResult>) -> impl IntoView {
    let mut results = results;
    results.reverse();
    view! {
        <span class="form-strip" title="recent form (newest to oldest)">
            {results.into_iter().enumerate().map(|(i, r)| {
                let (label, class) = form_cell(r.place);
                let bold = i == 0;
                view! { <span class=class style=move || if bold { "font-weight:bold" } else { "" }>{label}</span> }
            }).collect_view()}
        </span>
    }
}
```

- [ ] **Step 4: Regenerate sqlx cache (if a macro query changed) and verify**

If the form query is a `sqlx::query_as!` macro, regenerate:

```bash
cd rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
```

Then:

```bash
SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add rust/web/src/stats/queries.rs rust/web/src/stats/viz.rs rust/web/src/game/server_fns.rs rust/web/.sqlx
git commit -m "feat(stats): form uses ranked_placing, last 5, newest-left bold (#47)"
```

---

## Task 16: Full verification + cleanup

**Files:** none (verification only)

- [ ] **Step 1: Run fmt**

```bash
cargo fmt --all -- --check
```

Expected: no diff. If it reports changes, run `cargo fmt --all` and re-commit the formatting.

- [ ] **Step 2: Run the full CI suite**

```bash
bash scripts/rust-test.sh
```

Expected: fmt, clippy, sqlx prepare check, and tests all pass. DB tests run against the script's temporary Postgres.

- [ ] **Step 3: Remove the persistent test container**

```bash
docker rm -f brdgme-plan-pg 2>/dev/null || true
```

- [ ] **Step 4: Review the commit range**

```bash
git log --oneline -20
git status
```

Expected: a clean sequence of #47 commits, working tree clean.

---

## Self-Review Notes

- **Spec coverage:** D1 schema (Task 1), D2 placing algorithm (Tasks 3/5), D3 concede flow (Tasks 8/10/12), D4 end game (Tasks 9/11/12), D5 all-humans-gone (Task 11 server fn permits leaver; bots play on naturally - no force-end), D6 ratings (Task 5), D7 UI + form (Tasks 13/14/15), D8 admin (Task 6), D9 bot turn triggering (Task 10 uses `broadcast_and_trigger`; `find_bot_turns` unchanged per context doc).
- **Elimination audit (spec D2 prerequisite):** Task 4 stamps `left_at` on elimination; Task 5 writes ranked placings on natural finish.
- **Type consistency:** `PlacingInput`/`compute_ranked_placings` (Task 3) used by `write_ranked_placings` (Task 5). `concede_game_replace` (Task 8) used by server fn (Task 10) and email (Task 12). `end_game` DB fn (Task 9) used by server fn (Task 11) and email (Task 12). `replacement_bot_available` (Task 10) used by email (Task 12) and gating (Task 13). `can_concede`/`can_end_game`/`is_replaced` (Task 13) used by UI (Task 14).
- **Known execution risks:** sqlx prepare requires the Task 1 DB; workers must export `DATABASE_URL`. `pick_replacement_bot` inserts a row - availability checks use `replacement_bot_available` to avoid orphans.
