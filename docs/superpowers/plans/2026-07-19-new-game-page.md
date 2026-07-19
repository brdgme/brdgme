# New Game Page Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the `/games` page as a browsable, filterable game grid with a sticky detail/setup panel, backed by a new `blurb` field flowing through the operator CRD pipeline, weight/blurb exposure in `GameTypeInfo`, server-side roster validation, and a user-search server fn.

**Architecture:** Backend first: a `blurb` column on `game_types` follows the existing `weight` pattern end to end (CRD spec field, hand-maintained `crd.yaml`, 39 per-game YAMLs, migration, operator upsert, model, queries). The web layer then exposes `weight`/`blurb` in `GameTypeInfo`, validates rosters in `create_new_game`, and gains a `search_users` server fn. The UI moves out of `app.rs` into a new `rust/web/src/new_game.rs` module: radio-card grid + client-side filters on the left, sticky setup panel on the right, per-slot Player/Email/Bot modes with typeahead and suggestion chips.

**Tech Stack:** Rust, Leptos 0.8 (CSR pages via `LocalResource`/`Action`, `#[server]` fns), sqlx 0.9 (compile-checked queries + `.sqlx` offline cache), Postgres, kube-rs operator, SCSS (`rust/web/style/main.scss`), Kustomize YAML.

**Spec:** `docs/superpowers/specs/2026-07-19-new-game-page-design.md`. The companion preview spec (`2026-07-19-new-game-preview-design.md`) is OUT of scope for this plan.

## Global Constraints

- Standard HTML inputs only. No images, no animations, no custom widgets replacing native form controls.
- Colors only via `--mk-*` custom properties or `mk-fg-*`/`mk-bg-*` classes; must work under all 33 themes; no meaning encoded in hue alone; translucency via `color-mix(... transparent)`. Never hard-code a color.
- Follow existing breakpoints: `@media only screen and (max-width: 80em)` (sidebar collapse) and `@media only screen and (max-width: 60em)`. The new page's two-pane/one-pane switch happens at 60em.
- The page stays inside the `.content-page` wrapper (max-width 1220px, centered).
- Migration text exactly: `ALTER TABLE` on `game_types` adding `blurb text NOT NULL DEFAULT ''`, numbered per `rust/web/migrations/` convention (next number: `012`).
- Bot difficulty remains a plain string end to end (easy/medium/hard); no enum, no bots-in-DB work.
- On successful game creation, redirect to `/games/{id}` (unchanged behavior).
- All shell commands run from `/home/beefsack/Development/brdgme/rust` unless a step says otherwise.
- DB-backed tests need Postgres running with `DATABASE_URL` set, migrations applied. CI uses `DATABASE_URL=postgres://postgres:postgres@localhost/brdgme`. `#[sqlx::test]` provisions an isolated database per test.
- NEVER run `cargo test -p web` or `cargo check -p web` without `--features ssr` — the crate has no default features and fails to compile by design without them.
- Test commands (mirroring `.github/workflows/ci.yml`):
  - `cargo test --workspace --exclude web` (covers the operator crate, package name `brdgme-operator`)
  - `cargo test -p web --features ssr`
  - `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo fmt --all -- --check`
- Whenever a `sqlx::query!`/`query_as!`/`query_scalar!` macro call or the schema changes, regenerate the offline cache: `cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets` (requires live DB), and commit the changed `.sqlx/` files. CI enforces this with `cargo sqlx prepare --check`.
- `k8s/base/operator/crd.yaml` is hand-maintained to mirror `rust/operator/src/crd.rs` — there is no generator. "Regenerate" means edit it by hand in the same commit as the `crd.rs` change.
- The blurbs written in Task 3 are Claude-drafted content for USER REVIEW before merge. Do not treat them as final copy.
- Commit frequently, one commit per task minimum, conventional-commit style messages (`feat(web): ...`, `feat(operator): ...`).

---

### Task 1: `blurb` column — migration, model, web queries

**Files:**
- Create: `rust/web/migrations/012_game_type_blurb.sql`
- Modify: `rust/web/src/models/game.rs` (GameType struct, lines 6-14)
- Modify: `rust/web/src/db.rs` (`find_available_game_types` ~line 239, `find_game_extended`'s game_type query ~line 366, tests mod ~line 2072)
- Modify: `rust/web/.sqlx/` (regenerated cache)

**Interfaces:**
- Consumes: existing `game_types` table; `crate::models::game::GameType`.
- Produces: `game_types.blurb text NOT NULL DEFAULT ''` column; `GameType` struct gains `pub blurb: String`; `db::find_available_game_types` returns blurb-carrying `GameType`s. Tasks 2, 4 rely on the column and struct field.

- [ ] **Step 1: Write the migration**

Create `rust/web/migrations/012_game_type_blurb.sql`:

```sql
-- #44 new game page (spec 2026-07-19-new-game-page-design.md): short
-- 1-2 sentence description per game type, upserted by the operator from
-- the GameVersion CRD alongside weight.
ALTER TABLE public.game_types
    ADD COLUMN blurb text NOT NULL DEFAULT '';
```

- [ ] **Step 2: Apply the migration**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust/web && sqlx migrate run
```
Expected: `Applied 12/migrate game type blurb` (exact wording varies; it must list 012 as applied).

- [ ] **Step 3: Write the failing test**

In `rust/web/src/db.rs`, inside the existing `#[cfg(all(test, feature = "ssr"))] mod tests` block (starts ~line 2072), add after the existing `migrations_apply_and_pool_connects` test:

```rust
    #[sqlx::test]
    async fn find_available_game_types_carries_weight_and_blurb(pool: PgPool) {
        // Unchecked queries: `weight`/`blurb` are exercised through the
        // function under test, not through compile-time macros here.
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts, weight, blurb)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind("Blurby")
        .bind(vec![2i32, 3])
        .bind(2.5f64)
        .bind("A short blurb.")
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, 'blurby-1', 'http://localhost:0/mock', true, false)",
        )
        .bind(game_type_id)
        .execute(&pool)
        .await
        .unwrap();

        let types = find_available_game_types(&pool).await.unwrap();
        let (gt, versions) = types
            .iter()
            .find(|(gt, _)| gt.name == "Blurby")
            .expect("Blurby game type present");
        assert_eq!(gt.weight, 2.5);
        assert_eq!(gt.blurb, "A short blurb.");
        assert_eq!(versions.len(), 1);
    }
```

- [ ] **Step 4: Run the test to verify it fails**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr find_available_game_types_carries_weight_and_blurb
```
Expected: FAIL to compile with `no field 'blurb' on type '&crate::models::game::GameType'`.

- [ ] **Step 5: Add the field to the model**

In `rust/web/src/models/game.rs`, change the `GameType` struct to:

```rust
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
    pub blurb: String,
}
```

- [ ] **Step 6: Update the compile-checked queries that select `GameType`**

In `rust/web/src/db.rs`, in `find_available_game_types` (~line 247), change the game_types query string to:

```rust
    let types = sqlx::query_as!(
        crate::models::game::GameType,
        "SELECT id, created_at, updated_at, name, player_counts, weight, blurb FROM game_types ORDER BY name"
    )
```

In `find_game_extended` (~line 366), change the game_type query string to:

```rust
    let game_type = sqlx::query_as!(
        crate::models::game::GameType,
        "SELECT id, created_at, updated_at, name, player_counts, weight, blurb FROM game_types WHERE id = $1",
        game_version.game_type_id
    )
```

Then check for any other `query_as!` site selecting the `GameType` struct:

```bash
grep -n "crate::models::game::GameType," /home/beefsack/Development/brdgme/rust/web/src/db.rs
```
Expected: only the two sites above (plus the struct-path mentions in `find_available_game_types`'s return type). If any other `query_as!(crate::models::game::GameType, ...)` call appears, add `, blurb` to its SELECT list the same way — the compile/prepare step below fails loudly on any missed site.

- [ ] **Step 7: Regenerate the sqlx offline cache**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
```
Expected: `query data written to .sqlx in the current directory` and modified/added files under `rust/web/.sqlx/`.

- [ ] **Step 8: Run the test to verify it passes**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr find_available_game_types_carries_weight_and_blurb
```
Expected: PASS (1 passed).

- [ ] **Step 9: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/migrations/012_game_type_blurb.sql rust/web/src/models/game.rs rust/web/src/db.rs rust/web/.sqlx
git commit -m "feat(web): game_types blurb column, model and queries (#44)"
```

---

### Task 2: `blurb` through the operator — CRD spec, crd.yaml, upsert

**Files:**
- Modify: `rust/operator/Cargo.toml` (add `[dev-dependencies]`)
- Modify: `rust/operator/src/crd.rs` (`GameVersionSpec`, lines 16-26)
- Modify: `rust/operator/src/controller.rs` (`reconcile` upsert call ~line 141, `upsert_game_type_and_version` ~line 165, tests mod ~line 235)
- Modify: `k8s/base/operator/crd.yaml` (spec properties, lines 18-29)

**Interfaces:**
- Consumes: `game_types.blurb` column from Task 1 (the operator test applies `rust/web/migrations`).
- Produces: `GameVersionSpec.blurb: String` (serde default, camelCase `blurb` in YAML); `upsert_game_type_and_version(pool, type_name, player_counts, weight, blurb, version_name, uri, is_deprecated, rules)`. Task 3's YAML `blurb:` keys rely on this CRD field.

- [ ] **Step 1: Enable `#[sqlx::test]` in the operator crate**

`rust/operator/Cargo.toml` currently has no `[dev-dependencies]` section. Append at the end of the file:

```toml

[dev-dependencies]
# Adds the test/migrate machinery for #[sqlx::test]; feature-unified with the
# main sqlx dependency when building tests.
sqlx = { version = "0.9", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "macros", "migrate"] }
```

- [ ] **Step 2: Write the failing test**

In `rust/operator/src/controller.rs`, inside the existing `#[cfg(test)] mod tests` block (line 235, currently containing only `interceptor_uri_defaults_to_keda_proxy`), add:

```rust
    // Applies the web crate's migrations so the schema matches production.
    // The operator itself never runs migrations (docs/DEV.md).
    #[sqlx::test(migrations = "../web/migrations")]
    async fn upsert_writes_weight_and_blurb(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            2.5,
            "A test blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            "rules text",
        )
        .await
        .unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Test Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 2.5);
        assert_eq!(blurb, "A test blurb.");

        // Upsert path: a second reconcile updates the existing row in place.
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            3.0,
            "New blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            "rules text",
        )
        .await
        .unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Test Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 3.0);
        assert_eq!(blurb, "New blurb.");
        let versions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_versions WHERE name = 'test-game-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(versions, 1);
    }
```

- [ ] **Step 3: Run the test to verify it fails**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p brdgme-operator upsert_writes_weight_and_blurb
```
Expected: FAIL to compile — `upsert_game_type_and_version` takes 8 arguments but 9 were supplied.

- [ ] **Step 4: Add the CRD spec field**

In `rust/operator/src/crd.rs`, add to `GameVersionSpec` after the `weight` field (line 21):

```rust
    /// Short 1-2 sentence description shown on the new game page.
    #[serde(default)]
    pub blurb: String,
```

- [ ] **Step 5: Thread blurb through the upsert**

In `rust/operator/src/controller.rs`, change `upsert_game_type_and_version` (line 165) to:

```rust
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
async fn upsert_game_type_and_version(
    pool: &PgPool,
    type_name: &str,
    player_counts: &[i32],
    weight: f32,
    blurb: &str,
    version_name: &str,
    uri: &str,
    is_deprecated: bool,
    rules: &str,
) -> Result<(), sqlx::Error> {
    let game_type_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO game_types (name, player_counts, weight, blurb)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (name) DO UPDATE
            SET player_counts = EXCLUDED.player_counts,
                weight        = EXCLUDED.weight,
                blurb         = EXCLUDED.blurb,
                updated_at    = NOW()
        RETURNING id
        "#,
    )
    .bind(type_name)
    .bind(player_counts)
    .bind(weight as f64)
    .bind(blurb)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated, rules)
        VALUES ($1, $2, $3, true, $4, $5)
        ON CONFLICT (game_type_id, name) DO UPDATE
            SET uri           = EXCLUDED.uri,
                is_public     = true,
                is_deprecated = EXCLUDED.is_deprecated,
                rules         = EXCLUDED.rules,
                updated_at    = NOW()
        "#,
    )
    .bind(game_type_id)
    .bind(version_name)
    .bind(uri)
    .bind(is_deprecated)
    .bind(rules)
    .execute(pool)
    .await?;

    Ok(())
}
```

And in `reconcile` (call site at line 141), change the call to:

```rust
    upsert_game_type_and_version(
        &ctx.pool,
        &obj.spec.type_name,
        &player_counts,
        obj.spec.weight,
        &obj.spec.blurb,
        &name,
        &uri,
        obj.spec.is_deprecated,
        &rules,
    )
    .await?;
```

- [ ] **Step 6: Mirror the field into `k8s/base/operator/crd.yaml`**

This file is hand-maintained (no generator). In the `spec.properties` block, after the `weight` property (lines 22-25) and before `isDeprecated`, add:

```yaml
              blurb:
                type: string
                default: ""
                description: Short 1-2 sentence description shown on the new game page.
```

The properties block then reads `typeName`, `weight`, `blurb`, `isDeprecated`.

- [ ] **Step 7: Run the test to verify it passes**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p brdgme-operator
```
Expected: PASS — `upsert_writes_weight_and_blurb` and `interceptor_uri_defaults_to_keda_proxy` both pass (2 passed; plus any tests in `main.rs`).

- [ ] **Step 8: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/operator/Cargo.toml rust/operator/src/crd.rs rust/operator/src/controller.rs k8s/base/operator/crd.yaml rust/Cargo.lock
git commit -m "feat(operator): blurb field through CRD and game_types upsert (#44)"
```

---

### Task 3: Draft blurbs in the 39 per-game YAMLs

**Files:**
- Modify: all 39 `k8s/base/game/<dir>/game-version.yaml` files.

**Interfaces:**
- Consumes: `blurb` CRD field from Task 2.
- Produces: content only. NOTE: these are Claude-drafted blurbs, flagged for user review/edit before merge.

- [ ] **Step 1: Add a `blurb:` line to every game-version.yaml**

In each file below, insert the `blurb:` line directly after the existing `weight:` line (same 2-space indent under `spec:`, before `isDeprecated:` where present). Where a game has `-1` and `-2` directories, both share a `typeName` and upsert the same `game_types` row, so both files get the identical blurb.

`k8s/base/game/acquire-1/game-version.yaml`:
```yaml
  blurb: "Build hotel chains, trigger lucrative mergers, and invest in the stock of the chains most likely to grow. A classic economic strategy game of timing and majority shareholding."
```

`k8s/base/game/age-of-war-1/game-version.yaml` and `k8s/base/game/age-of-war-2/game-version.yaml`:
```yaml
  blurb: "Roll dice to muster armies and conquer the castles of feudal Japan, stealing them from rivals until the clans are decided. A quick push-your-luck dice game."
```

`k8s/base/game/battleship-1/game-version.yaml` and `k8s/base/game/battleship-2/game-version.yaml`:
```yaml
  blurb: "Hide your fleet on a secret grid and call shots to hunt down the enemy ships before yours are sunk. The classic naval guessing game."
```

`k8s/base/game/category-5-1/game-version.yaml` and `k8s/base/game/category-5-2/game-version.yaml`:
```yaml
  blurb: "Play numbered cards into rows and avoid picking up the sixth card, because every card you take costs you points. A simultaneous card game of risk and second-guessing."
```

`k8s/base/game/cathedral-1/game-version.yaml` and `k8s/base/game/cathedral-2/game-version.yaml`:
```yaml
  blurb: "Place your buildings to wall off districts of a medieval city and squeeze out your opponent's pieces. An abstract territory battle for two."
```

`k8s/base/game/farkle-1/game-version.yaml` and `k8s/base/game/farkle-2/game-version.yaml`:
```yaml
  blurb: "Roll six dice, set aside scoring combinations, and decide whether to press on for more points or bank before a bust wipes out the turn. A classic push-your-luck dice game."
```

`k8s/base/game/for-sale-1/game-version.yaml` and `k8s/base/game/for-sale-2/game-version.yaml`:
```yaml
  blurb: "Bid on properties from cardboard shacks to space stations, then flip them for the best cheques in a second round of sales. A fast, sharp auction filler."
```

`k8s/base/game/greed-1/game-version.yaml` and `k8s/base/game/greed-2/game-version.yaml`:
```yaml
  blurb: "Roll for cash and loot, banking your haul before a bad roll takes it all away. A push-your-luck dice game about knowing when to stop."
```

`k8s/base/game/jaipur-2/game-version.yaml`:
```yaml
  blurb: "Trade goods and camels in the markets of Jaipur, timing your sales for maximum rupees. A brisk two-player duel of set collection and tempo."
```

`k8s/base/game/liars-dice-1/game-version.yaml` and `k8s/base/game/liars-dice-2/game-version.yaml`:
```yaml
  blurb: "Bid on the dice hidden under everyone's cups, bluffing and calling bluffs until someone is caught out. Lose dice when you are wrong; the last player still rolling wins."
```

`k8s/base/game/lost-cities-1/game-version.yaml` and `k8s/base/game/lost-cities-2/game-version.yaml`:
```yaml
  blurb: "Fund expeditions to five lost cities, committing cards to routes that must pay off before the deck runs out. A tense two-player card game of investment and restraint."
```

`k8s/base/game/love-letter-1/game-version.yaml` and `k8s/base/game/love-letter-2/game-version.yaml`:
```yaml
  blurb: "Get your love letter into the princess's hands by deducing and knocking out your rivals with a hand of just one card. A tiny game of deduction and luck."
```

`k8s/base/game/modern-art-1/game-version.yaml` and `k8s/base/game/modern-art-2/game-version.yaml`:
```yaml
  blurb: "Buy and sell paintings at auction, riding the market for the season's most fashionable artists. A classic auction game of valuation and salesmanship."
```

`k8s/base/game/no-thanks-1/game-version.yaml` and `k8s/base/game/no-thanks-2/game-version.yaml`:
```yaml
  blurb: "Take the card or pay a chip to pass the problem along, collecting runs to keep your score low. A one-decision game that is harder than it looks."
```

`k8s/base/game/roll-through-the-ages-1/game-version.yaml` and `k8s/base/game/roll-through-the-ages-2/game-version.yaml`:
```yaml
  blurb: "Roll dice to feed your people, raise monuments, and research developments through the Bronze Age. A quick civilization-building dice game."
```

`k8s/base/game/splendor-1/game-version.yaml` and `k8s/base/game/splendor-2/game-version.yaml`:
```yaml
  blurb: "Collect gem chips and build a tableau of mines and merchants where every purchase discounts the next. An engine-building race for prestige."
```

`k8s/base/game/sushi-go-1/game-version.yaml` and `k8s/base/game/sushi-go-2/game-version.yaml`:
```yaml
  blurb: "Draft sushi dishes from hands passed around the table, assembling combos before your neighbours snatch the pieces you need. A light, speedy drafting game."
```

`k8s/base/game/sushizock-1/game-version.yaml` and `k8s/base/game/sushizock-2/game-version.yaml`:
```yaml
  blurb: "Roll dice to grab tasty sushi tiles and dodge the fish bones, stealing from rivals when the dice allow. A push-your-luck tile-snatching dice game."
```

`k8s/base/game/texas-holdem-1/game-version.yaml` and `k8s/base/game/texas-holdem-2/game-version.yaml`:
```yaml
  blurb: "No-limit Texas hold'em poker: bet, bluff, and go all-in to take every chip at the table."
```

`k8s/base/game/tic-tac-toe-2/game-version.yaml`:
```yaml
  blurb: "Take turns marking the grid and make three in a row before your opponent does. The old classic, for one quick game or ten."
```

`k8s/base/game/zombie-dice-1/game-version.yaml` and `k8s/base/game/zombie-dice-2/game-version.yaml`:
```yaml
  blurb: "Shake the dice cup and eat brains, but stop before you collect three shotgun blasts. A quick zombie-themed push-your-luck dice game."
```

Example result (`k8s/base/game/age-of-war-1/game-version.yaml`, full file after edit):

```yaml
apiVersion: brdgme.com/v1
kind: GameVersion
metadata:
  name: age-of-war-1
  namespace: brdgme
spec:
  typeName: Age of War
  weight: 1.0
  blurb: "Roll dice to muster armies and conquer the castles of feudal Japan, stealing them from rivals until the clans are decided. A quick push-your-luck dice game."
  isDeprecated: true
```

- [ ] **Step 2: Verify all 39 files got a blurb**

Run:
```bash
grep -rl "  blurb: \"" /home/beefsack/Development/brdgme/k8s/base/game --include=game-version.yaml | wc -l
```
Expected: `39`

- [ ] **Step 3: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add k8s/base/game
git commit -m "feat(k8s): draft blurbs for all 39 game versions (#44)"
```

---

### Task 4: Expose `weight` and `blurb` in `GameTypeInfo`

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (`GameTypeInfo` struct lines 83-89, `get_available_game_types` lines 274-303, `GameVersionInfo` lines 77-81)

**Interfaces:**
- Consumes: `GameType.blurb`/`GameType.weight` from Task 1.
- Produces: `GameTypeInfo { id: Uuid, name: String, player_counts: Vec<i32>, weight: f32, blurb: String, versions: Vec<GameVersionInfo> }`, with `PartialEq` derived on `GameTypeInfo` and `GameVersionInfo` (needed by the `Memo` and unit tests in Task 8). Tasks 8-9 consume this struct.

- [ ] **Step 1: Extend the structs**

In `rust/web/src/game/server_fns.rs`, replace the two structs at lines 77-89 with:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameVersionInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeInfo {
    pub id: Uuid,
    pub name: String,
    pub player_counts: Vec<i32>,
    /// Complexity, 0.0 (light) to 5.0 (heavy), from game_types.weight.
    pub weight: f32,
    /// 1-2 sentence description; empty string renders nothing.
    pub blurb: String,
    pub versions: Vec<GameVersionInfo>,
}
```

- [ ] **Step 2: Map the new fields**

In `get_available_game_types`, change the mapping closure to:

```rust
    Ok(game_types
        .into_iter()
        .map(|(gt, versions)| GameTypeInfo {
            id: gt.id,
            name: gt.name,
            player_counts: gt.player_counts,
            weight: gt.weight,
            blurb: gt.blurb,
            versions: versions
                .into_iter()
                .map(|gv| GameVersionInfo {
                    id: gv.id,
                    name: gv.name,
                })
                .collect(),
        })
        .collect())
```

- [ ] **Step 3: Verify it compiles clean and existing tests pass**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr find_available_game_types_carries_weight_and_blurb
```
Expected: clippy clean; test PASS. (The struct fields are exercised end-to-end by Task 1's DB test plus the UI unit tests in Task 8; the server fn body is a straight field copy checked at compile time.)

- [ ] **Step 4: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/game/server_fns.rs
git commit -m "feat(web): expose weight and blurb in GameTypeInfo (#44)"
```

---

### Task 5: Server-side roster validation in `create_new_game`

**Files:**
- Modify: `rust/web/src/db.rs` (new `find_game_type_player_counts` next to `find_latest_non_deprecated_game_version` ~line 236, new test in tests mod)
- Modify: `rust/web/src/game/server_fns.rs` (`create_new_game` lines 375-438, new `roster_error` helper, new tests mod at end of file)
- Modify: `rust/web/.sqlx/` (regenerated cache — new macro query)

**Interfaces:**
- Consumes: `game_types.player_counts`, `create_new_game`'s existing `player_count` computation (`1 + opponent_ids + opponent_emails + bot_slots`).
- Produces: `db::find_game_type_player_counts(pool: &PgPool, game_version_id: Uuid) -> Result<Option<Vec<i32>>>`; `server_fns::roster_error(player_counts: &[i32], player_count: usize) -> Option<String>`; `create_new_game` rejects invalid rosters with a `ServerFnError` whose message names the supported counts.

- [ ] **Step 1: Write the failing unit tests for the validation rule**

At the very end of `rust/web/src/game/server_fns.rs`, add:

```rust
#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn roster_error_accepts_supported_counts() {
        assert_eq!(roster_error(&[2, 3, 4], 2), None);
        assert_eq!(roster_error(&[2, 3, 4], 3), None);
        assert_eq!(roster_error(&[2, 3, 4], 4), None);
    }

    #[test]
    fn roster_error_rejects_unsupported_counts() {
        let err = roster_error(&[2, 3, 4], 5).expect("5 players rejected");
        assert!(err.contains("2, 3, 4"), "message lists counts: {err}");
        assert!(err.contains('5'), "message names the bad count: {err}");
        // Non-contiguous counts: the gap is rejected.
        assert!(roster_error(&[2, 4], 3).is_some());
        // Solo (no opponents) rejected when unsupported.
        assert!(roster_error(&[2, 3, 4], 1).is_some());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr roster_error
```
Expected: FAIL to compile — `cannot find function 'roster_error' in this scope`.

- [ ] **Step 3: Implement `roster_error`**

In `rust/web/src/game/server_fns.rs`, immediately above `create_new_game` (line 375), add:

```rust
/// Returns a user-facing error when `player_count` is not one of the game
/// type's supported counts; `None` means the roster is valid. Unreachable
/// through the constrained UI, but the API must not trust the client.
#[cfg(feature = "ssr")]
fn roster_error(player_counts: &[i32], player_count: usize) -> Option<String> {
    if player_counts.contains(&(player_count as i32)) {
        return None;
    }
    let counts = player_counts
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "This game supports {counts} players, but the request has {player_count} (including you)"
    ))
}
```

- [ ] **Step 4: Run the unit tests to verify they pass**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr roster_error
```
Expected: PASS (2 passed).

- [ ] **Step 5: Write the failing DB-lookup test**

In `rust/web/src/db.rs` tests mod, add (uses the existing `make_game_type_and_version` fixture helper, which creates `player_counts = [2, 3, 4]`):

```rust
    #[sqlx::test]
    async fn find_game_type_player_counts_by_version(pool: PgPool) {
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        assert_eq!(
            find_game_type_player_counts(&pool, game_version_id)
                .await
                .unwrap(),
            Some(vec![2, 3, 4])
        );
        assert_eq!(
            find_game_type_player_counts(&pool, Uuid::new_v4())
                .await
                .unwrap(),
            None
        );
    }
```

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr find_game_type_player_counts_by_version
```
Expected: FAIL to compile — `cannot find function 'find_game_type_player_counts'`.

- [ ] **Step 6: Implement the DB lookup**

In `rust/web/src/db.rs`, after `find_latest_non_deprecated_game_version` (ends ~line 236), add:

```rust
#[cfg(feature = "ssr")]
pub async fn find_game_type_player_counts(
    pool: &PgPool,
    game_version_id: Uuid,
) -> Result<Option<Vec<i32>>> {
    Ok(sqlx::query_scalar!(
        "SELECT gt.player_counts FROM game_types gt
         JOIN game_versions gv ON gv.game_type_id = gt.id
         WHERE gv.id = $1",
        game_version_id
    )
    .fetch_optional(pool)
    .await?)
}
```

- [ ] **Step 7: Wire validation into `create_new_game`**

In `rust/web/src/game/server_fns.rs`, in `create_new_game`, directly after the `find_game_version` block (the `let game_version = ...;` statement ending ~line 402) and before `let mut tx = ...`, insert:

```rust
    let player_counts = crate::db::find_game_type_player_counts(&pool, game_version_id)
        .await
        .map_err(internal("create_new_game: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;
    if let Some(msg) = roster_error(&player_counts, player_count) {
        return Err(ServerFnError::new(msg));
    }
```

- [ ] **Step 8: Regenerate the sqlx cache and run the tests**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust/web && cargo sqlx prepare -- --tests --features ssr --all-targets
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr roster_error && cargo test -p web --features ssr find_game_type_player_counts_by_version
```
Expected: prepare writes `.sqlx` entries; both test filters PASS.

- [ ] **Step 9: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/db.rs rust/web/src/game/server_fns.rs rust/web/.sqlx
git commit -m "feat(web): server-side roster validation in create_new_game (#44)"
```

---

### Task 6: User search — DB function and server fn

**Files:**
- Modify: `rust/web/src/db.rs` (new `search_users` next to `get_user_by_name` ~line 1884, new tests)
- Modify: `rust/web/src/friends.rs` (new `UserSearchResult` struct after `OpponentSuggestion` line 47, new `search_users` server fn after `get_opponent_suggestions` line 251)

**Interfaces:**
- Consumes: `users` table (`id`, `name`); `require_user()` helper in friends.rs.
- Produces: `db::search_users(pool: &PgPool, user_id: Uuid, query: &str) -> Result<Vec<(Uuid, String)>>` (empty result under 2 trimmed chars, cap 10, excludes `user_id`, case-insensitive substring, LIKE wildcards escaped); `friends::UserSearchResult { user_id: Uuid, name: String }`; `#[server] friends::search_users(query: String) -> Result<Vec<UserSearchResult>, ServerFnError>` (login required). Task 9's typeahead calls `crate::friends::search_users`.

- [ ] **Step 1: Write the failing DB tests**

In `rust/web/src/db.rs` tests mod, add:

```rust
    #[sqlx::test]
    async fn search_users_min_length_cap_and_excludes_self(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        for i in 0..12 {
            make_user(&pool, &format!("player{i:02}")).await;
        }

        // Under 2 trimmed characters: no results, no query.
        assert!(search_users(&pool, me.id, "p").await.unwrap().is_empty());
        assert!(search_users(&pool, me.id, " a ").await.unwrap().is_empty());
        assert!(search_users(&pool, me.id, "").await.unwrap().is_empty());

        // Results are capped at 10 of the 12 matches.
        assert_eq!(search_users(&pool, me.id, "player").await.unwrap().len(), 10);

        // The searching user is never in their own results.
        assert!(search_users(&pool, me.id, "search").await.unwrap().is_empty());

        // Case-insensitive substring match.
        let hits = search_users(&pool, me.id, "PLAYER00").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "player00");
    }

    #[sqlx::test]
    async fn search_users_escapes_like_wildcards(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        make_user(&pool, "percent%name").await;
        make_user(&pool, "underscore_name").await;

        let hits = search_users(&pool, me.id, "percent%").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "percent%name");

        // A raw "%%" query must not match everything.
        assert!(search_users(&pool, me.id, "%%").await.unwrap().is_empty());

        // "_" is a literal underscore, not a single-char wildcard.
        let hits = search_users(&pool, me.id, "score_n").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, "underscore_name");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr search_users_
```
Expected: FAIL to compile — `cannot find function 'search_users' in this scope`.

- [ ] **Step 3: Implement the DB function**

In `rust/web/src/db.rs`, after `get_user_by_name` (ends ~line 1884), add:

```rust
/// Display-name substring search for the new game page typeahead (#44):
/// case-insensitive, excludes the searching user, capped at 10. Queries
/// under 2 trimmed characters return nothing without touching the DB.
#[cfg(feature = "ssr")]
pub async fn search_users(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
) -> Result<Vec<(Uuid, String)>> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    // Escape LIKE wildcards so users named "a%b" are findable and "%"
    // queries cannot match everyone.
    let escaped = q
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Ok(sqlx::query_as(
        "SELECT id, name FROM users
         WHERE id <> $1 AND name ILIKE $2 ESCAPE '\\'
         ORDER BY lower(name)
         LIMIT 10",
    )
    .bind(user_id)
    .bind(format!("%{escaped}%"))
    .fetch_all(pool)
    .await?)
}
```

- [ ] **Step 4: Run the DB tests to verify they pass**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr search_users_
```
Expected: PASS (2 passed).

- [ ] **Step 5: Add the server fn**

In `rust/web/src/friends.rs`, after the `OpponentSuggestion` struct (line 47), add:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSearchResult {
    pub user_id: Uuid,
    pub name: String,
}
```

After `get_opponent_suggestions` (ends line 251), add:

```rust
/// #44 new game page typeahead: display-name substring search. Login
/// required; under 2 trimmed characters returns empty; capped at 10;
/// excludes the caller (all enforced in db::search_users).
#[server(SearchUsers, "/api")]
pub async fn search_users(query: String) -> Result<Vec<UserSearchResult>, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();
    let user = require_user().await?;
    let rows = crate::db::search_users(&pool, user.id, &query)
        .await
        .map_err(internal("search_users: query"))?;
    Ok(rows
        .into_iter()
        .map(|(user_id, name)| UserSearchResult { user_id, name })
        .collect())
}
```

- [ ] **Step 6: Verify compile and clippy**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings
```
Expected: clean.

- [ ] **Step 7: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/db.rs rust/web/src/friends.rs
git commit -m "feat(web): user search server fn for opponent typeahead (#44)"
```

---

### Task 7: SCSS for the new game page

**Files:**
- Modify: `rust/web/style/main.scss` (append after the `.form-strip` rules that currently end the form section, ~line 642)

**Interfaces:**
- Consumes: `--mk-*` variables (`--mk-foreground`, `--mk-blue`, `--mk-grey`, `--mk-soften-foreground-90`), existing `.form-field`/`.form-label`/`.form-control`/`.form-error`/`.form-actions` rules.
- Produces: classes used by Tasks 8-9: `.new-game-layout`, `.new-game-browser`, `.new-game-panel`, `.new-game-panel-empty`, `.new-game-filters`, `.new-game-filter-players`, `.new-game-filter-search`, `.new-game-blurb`, `.game-card-grid`, `.game-card`, `.game-card-name`, `.game-card-meta`, `.game-card-blurb`, `.player-count-radios`, `.slot-modes`, `.chip`, `.chip-row`, `.chip-friend`, `.chip-selected`, `.typeahead-results`, `.sr-only`.

- [ ] **Step 1: Append the styles**

Add to `rust/web/style/main.scss`, after the `.form-strip` block:

```scss
/* #44 new game page (spec 2026-07-19-new-game-page-design.md).
   Two-pane >= 60em: grid left, sticky setup panel right. Colors only via
   --mk-* vars; selection and friend markers carry a non-hue cue (outline
   weight / border style) so no meaning rides on hue alone. */

.new-game-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(18em, 24em);
  gap: 1.5em;
  align-items: start;
}

.new-game-filters {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
  margin-bottom: 1em;
}

.new-game-filter-players {
  width: 6em;
}

.new-game-filter-search {
  flex: 1 1 10em;
  min-width: 8em;
}

.game-card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14em, 1fr));
  gap: 0.75em;
}

.game-card {
  display: flex;
  flex-direction: column;
  gap: 0.25em;
  border: 1px solid color-mix(in srgb, var(--mk-foreground) 25%, transparent);
  border-radius: 4px;
  padding: 0.75em;
  cursor: pointer;
}

.game-card:hover {
  background-color: color-mix(in srgb, var(--mk-soften-foreground-90) 60%, transparent);
}

/* Selected: hue plus outline weight, so it reads under every theme. */
.game-card:has(input:checked) {
  border-color: var(--mk-blue);
  outline: 2px solid var(--mk-blue);
  outline-offset: -2px;
  background-color: color-mix(in srgb, var(--mk-blue) 12%, transparent);
}

/* The radio itself is visually hidden; surface keyboard focus on the card. */
.game-card:has(input:focus-visible) {
  outline: 2px solid var(--mk-foreground);
  outline-offset: 2px;
}

.game-card-name {
  font-weight: 700;
}

.game-card-meta {
  font-size: 0.8em;
  color: var(--mk-grey);
}

.game-card-blurb,
.new-game-blurb {
  font-size: 0.9em;
}

.new-game-panel {
  position: sticky;
  top: 1em;
  border: 1px solid color-mix(in srgb, var(--mk-foreground) 25%, transparent);
  border-radius: 4px;
  padding: 1em;
}

.new-game-panel h2 {
  margin-top: 0;
}

.new-game-panel-empty {
  color: var(--mk-grey);
}

.player-count-radios,
.slot-modes {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75em;
}

.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35em;
}

.chip {
  display: inline-block;
  border: 1px solid color-mix(in srgb, var(--mk-foreground) 35%, transparent);
  border-radius: 1em;
  padding: 0 0.6em;
  text-decoration: none;
}

/* Friend chips: double border = non-hue cue. */
.chip.chip-friend {
  border-style: double;
  border-width: 3px;
}

.chip.chip-selected {
  border-color: var(--mk-blue);
  background-color: color-mix(in srgb, var(--mk-blue) 12%, transparent);
}

.typeahead-results {
  list-style: none;
  margin: 0.35em 0 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.35em;
}

/* Visually hidden but accessible (the radio inside each game card). */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
  border: 0;
}

@media only screen and (max-width: 60em) {
  .new-game-layout {
    grid-template-columns: 1fr;
  }

  .new-game-panel {
    position: static;
  }
}
```

- [ ] **Step 2: Verify the stylesheet still builds**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings
```
Expected: clean (cargo-leptos compiles the SCSS at build time in dev/CI; clippy at minimum confirms nothing else broke — if a `just`/`cargo leptos` dev build is running, confirm no SCSS compile error in its output).

- [ ] **Step 3: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/style/main.scss
git commit -m "feat(web): styles for new game page grid, panel and chips (#44)"
```

---

### Task 8: New game page UI — module, grid, filters, panel

**Files:**
- Create: `rust/web/src/new_game.rs`
- Modify: `rust/web/src/lib.rs` (module list, lines 3-8)
- Modify: `rust/web/src/app.rs` (route line 196; delete `OpponentSlot` + `GamesPage`, lines 437-763)
- Modify: `rust/web/Cargo.toml` (web-sys features, line 65)

**Interfaces:**
- Consumes: `GameTypeInfo`/`GameVersionInfo` (Task 4), `create_new_game`/`BotSlot`, `get_opponent_suggestions`/`OpponentSuggestion`, SCSS classes (Task 7).
- Produces: `pub fn NewGamePage` component at `/games`; module-privates consumed by Task 9: `enum OpponentSlot { Player { query: String, selected: Option<(Uuid, String)> }, Email(String), Bot { name: String, difficulty: String } }`, `enum SlotMode { Player, Email, Bot }`, `fn player_range(&[i32]) -> String`, `fn weight_text(f32) -> String`, `fn filter_and_sort(&[GameTypeInfo], Option<i32>, &str, &str) -> Vec<GameTypeInfo>`, `fn is_narrow() -> bool`, component `OpponentSlotEditor { i, slots, set_slots, suggestions }`. In this task, Player mode offers suggestion chips only; the typeahead input arrives in Task 9.

- [ ] **Step 1: Add the `MediaQueryList` web-sys feature**

In `rust/web/Cargo.toml` line 65, change the web-sys line to:

```toml
web-sys = { version = "0.3.77", features = ["Location", "Window", "Document", "HtmlDocument", "Element", "MediaQueryList"] }
```

- [ ] **Step 2: Write the failing unit tests**

Create `rust/web/src/new_game.rs` containing (for now) only the pure helpers' tests and a module doc — the helpers themselves come next so the test run below fails first:

```rust
//! #44 new game page: browsable game grid, filters, and a setup panel.
//! Spec: docs/superpowers/specs/2026-07-19-new-game-page-design.md

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::server_fns::GameTypeInfo;
    use uuid::Uuid;

    fn gt(name: &str, counts: &[i32], weight: f32) -> GameTypeInfo {
        GameTypeInfo {
            id: Uuid::new_v4(),
            name: name.to_string(),
            player_counts: counts.to_vec(),
            weight,
            blurb: String::new(),
            versions: Vec::new(),
        }
    }

    fn names(list: &[GameTypeInfo]) -> Vec<&str> {
        list.iter().map(|g| g.name.as_str()).collect()
    }

    #[test]
    fn player_range_formats() {
        assert_eq!(player_range(&[2]), "2 players");
        assert_eq!(player_range(&[2, 3, 4]), "2-4 players");
        assert_eq!(player_range(&[2, 4, 6]), "2, 4, 6 players");
        assert_eq!(player_range(&[]), "");
    }

    #[test]
    fn weight_text_formats() {
        assert_eq!(weight_text(2.5), "Weight 2.5 / 5");
        assert_eq!(weight_text(1.0), "Weight 1.0 / 5");
    }

    #[test]
    fn filter_by_player_count() {
        let types = vec![gt("Duel", &[2], 1.0), gt("Party", &[3, 4, 5], 1.0)];
        assert_eq!(names(&filter_and_sort(&types, Some(2), "", "alpha")), ["Duel"]);
        assert_eq!(
            names(&filter_and_sort(&types, Some(4), "", "alpha")),
            ["Party"]
        );
        assert!(filter_and_sort(&types, Some(9), "", "alpha").is_empty());
        // Cleared filter shows all.
        assert_eq!(filter_and_sort(&types, None, "", "alpha").len(), 2);
    }

    #[test]
    fn filter_by_text_is_case_insensitive_substring() {
        let types = vec![gt("Acquire", &[2], 1.0), gt("Lost Cities", &[2], 1.0)];
        assert_eq!(
            names(&filter_and_sort(&types, None, "cIt", "alpha")),
            ["Lost Cities"]
        );
        assert_eq!(filter_and_sort(&types, None, "  ", "alpha").len(), 2);
    }

    #[test]
    fn sort_variants() {
        let types = vec![
            gt("Beta", &[2], 3.0),
            gt("Alpha", &[2], 2.0),
            gt("Gamma", &[2], 2.0),
        ];
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "alpha")),
            ["Alpha", "Beta", "Gamma"]
        );
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "weight-asc")),
            ["Alpha", "Gamma", "Beta"]
        );
        assert_eq!(
            names(&filter_and_sort(&types, None, "", "weight-desc")),
            ["Beta", "Alpha", "Gamma"]
        );
    }
}
```

- [ ] **Step 3: Register the module and run the tests to verify they fail**

In `rust/web/src/lib.rs`, change:

```rust
pub mod friends;
pub mod players;
```
to:
```rust
pub mod friends;
pub mod new_game;
pub mod players;
```

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr new_game::
```
Expected: FAIL to compile — `cannot find function 'player_range' in this scope` (and the same for `weight_text`, `filter_and_sort`).

- [ ] **Step 4: Write the page implementation**

Fill in `rust/web/src/new_game.rs` above the tests mod with the complete implementation:

```rust
use leptos::html;
use leptos::prelude::*;
use leptos_router::{NavigateOptions, hooks::use_navigate};
use uuid::Uuid;

use crate::friends::OpponentSuggestion;
use crate::game::server_fns::{BotSlot, GameTypeInfo, create_new_game};

/// Formats supported player counts, honoring non-contiguous sets:
/// [2,3,4] -> "2-4 players", [2] -> "2 players", [2,4,6] -> "2, 4, 6 players".
fn player_range(counts: &[i32]) -> String {
    match counts {
        [] => String::new(),
        [n] => format!("{n} players"),
        _ => {
            let contiguous = counts.windows(2).all(|w| w[1] == w[0] + 1);
            if contiguous {
                format!("{}-{} players", counts[0], counts[counts.len() - 1])
            } else {
                let list = counts
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{list} players")
            }
        }
    }
}

fn weight_text(weight: f32) -> String {
    format!("Weight {weight:.1} / 5")
}

/// Client-side filter + sort over the already-fetched list. `sort_key` is
/// one of "alpha" (default), "weight-asc", "weight-desc"; weight ties break
/// alphabetically.
fn filter_and_sort(
    types: &[GameTypeInfo],
    count_filter: Option<i32>,
    text: &str,
    sort_key: &str,
) -> Vec<GameTypeInfo> {
    let text = text.trim().to_lowercase();
    let mut list: Vec<GameTypeInfo> = types
        .iter()
        .filter(|gt| count_filter.is_none_or(|c| gt.player_counts.contains(&c)))
        .filter(|gt| text.is_empty() || gt.name.to_lowercase().contains(&text))
        .cloned()
        .collect();
    match sort_key {
        "weight-asc" => list.sort_by(|a, b| {
            a.weight
                .partial_cmp(&b.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        "weight-desc" => list.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        }),
        _ => list.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    list
}

/// True below the 60em breakpoint (single-column layout), where selecting a
/// game should scroll the setup panel into view.
fn is_narrow() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(max-width: 60em)").ok().flatten())
        .map(|m| m.matches())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SlotMode {
    Player,
    Email,
    Bot,
}

/// Per-opponent slot state. Player = a site user, picked via suggestion
/// chip or typeahead; Email = invite by address; Bot = name + difficulty
/// (difficulty stays a plain string pending the bots-in-DB work).
#[derive(Debug, Clone)]
enum OpponentSlot {
    Player {
        query: String,
        selected: Option<(Uuid, String)>,
    },
    Email(String),
    Bot {
        name: String,
        difficulty: String,
    },
}

impl OpponentSlot {
    fn mode(&self) -> SlotMode {
        match self {
            OpponentSlot::Player { .. } => SlotMode::Player,
            OpponentSlot::Email(_) => SlotMode::Email,
            OpponentSlot::Bot { .. } => SlotMode::Bot,
        }
    }
}

impl Default for OpponentSlot {
    fn default() -> Self {
        OpponentSlot::Player {
            query: String::new(),
            selected: None,
        }
    }
}

#[component]
pub fn NewGamePage() -> impl IntoView {
    use crate::components::layout::MainLayout;
    use crate::game::server_fns::get_available_game_types;

    let game_types = LocalResource::new(get_available_game_types);

    view! {
        <MainLayout>
            <div class="new-game content-page">
                <h1>"New Game"</h1>
                {move || match game_types.get() {
                    None => view! { <p>"Loading..."</p> }.into_any(),
                    Some(Err(e)) => view! { <p class="error">"Error: " {e.to_string()}</p> }.into_any(),
                    Some(Ok(t)) if t.is_empty() => view! { <p>"No games available."</p> }.into_any(),
                    Some(Ok(types)) => view! { <GameBrowser types=types/> }.into_any(),
                }}
            </div>
        </MainLayout>
    }
}

#[component]
fn GameBrowser(types: Vec<GameTypeInfo>) -> impl IntoView {
    let types = StoredValue::new(types);
    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);

    let (selected_type_id, set_selected_type_id) = signal(None::<Uuid>);
    let (selected_version_id, set_selected_version_id) = signal(None::<Uuid>);
    let (player_count, set_player_count) = signal(0i32);
    let (opponent_slots, set_opponent_slots) = signal(Vec::<OpponentSlot>::new());

    let (filter_players, set_filter_players) = signal(String::new());
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_key, set_sort_key) = signal("alpha".to_string());
    let (form_error, set_form_error) = signal(None::<String>);

    let panel_ref = NodeRef::<html::Div>::new();

    let visible_types = Memo::new(move |_| {
        types.with_value(|t| {
            filter_and_sort(
                t,
                filter_players.get().trim().parse::<i32>().ok(),
                &filter_text.get(),
                &sort_key.get(),
            )
        })
    });

    // Filtering out the selected game deselects it: the panel returns to
    // its empty state (spec, "Filters and sort").
    Effect::new(move |_| {
        if let Some(id) = selected_type_id.get()
            && !visible_types.get().iter().any(|gt| gt.id == id)
        {
            set_selected_type_id.set(None);
            set_selected_version_id.set(None);
        }
    });

    // Opponent slots track player_count - 1. Existing slot state survives
    // count changes where possible (resize, not rebuild).
    Effect::new(move |_| {
        let n = (player_count.get() - 1).max(0) as usize;
        set_opponent_slots.update(|v| v.resize_with(n, OpponentSlot::default));
    });

    let select_game = move |gt: &GameTypeInfo| {
        set_selected_type_id.set(Some(gt.id));
        set_selected_version_id.set(gt.versions.first().map(|v| v.id));
        set_player_count.set(gt.player_counts.first().copied().unwrap_or(2));
        set_form_error.set(None);
        // Single-column layout: bring the setup panel into view.
        if is_narrow()
            && let Some(el) = panel_ref.get_untracked()
        {
            el.scroll_into_view();
        }
    };

    let create_action = Action::new(
        |(version_id, ids, emails, bots): &(Uuid, Vec<Uuid>, Vec<String>, Vec<BotSlot>)| {
            let version_id = *version_id;
            let ids = if ids.is_empty() {
                None
            } else {
                Some(ids.clone())
            };
            let emails = if emails.is_empty() {
                None
            } else {
                Some(emails.clone())
            };
            let bots = if bots.is_empty() {
                None
            } else {
                Some(bots.clone())
            };
            async move { create_new_game(version_id, ids, emails, bots).await }
        },
    );

    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(id)) = create_action.value().get() {
            navigate(&format!("/games/{}", id), NavigateOptions::default());
        }
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(version_id) = selected_version_id.get_untracked() else {
            return;
        };
        let mut ids = Vec::new();
        let mut emails = Vec::new();
        let mut bots = Vec::new();
        for slot in opponent_slots.get_untracked() {
            match slot {
                OpponentSlot::Player {
                    selected: Some((id, _)),
                    ..
                } => ids.push(id),
                OpponentSlot::Player { selected: None, .. } => {
                    set_form_error.set(Some(
                        "Choose a player for each Player slot, or switch the slot to Email or Bot"
                            .to_string(),
                    ));
                    return;
                }
                OpponentSlot::Email(email) => emails.push(email),
                OpponentSlot::Bot { name, difficulty } => {
                    bots.push(BotSlot { name, difficulty })
                }
            }
        }
        set_form_error.set(None);
        create_action.dispatch((version_id, ids, emails, bots));
    };

    view! {
        <div class="new-game-layout">
            <div class="new-game-browser">
                <div class="new-game-filters">
                    <input
                        type="number"
                        min="1"
                        class="new-game-filter-players"
                        placeholder="Players"
                        aria-label="Filter by player count"
                        prop:value=filter_players
                        on:input=move |ev| set_filter_players.set(event_target_value(&ev))
                    />
                    <input
                        type="search"
                        class="new-game-filter-search"
                        placeholder="Search games"
                        aria-label="Search games by name"
                        prop:value=filter_text
                        on:input=move |ev| set_filter_text.set(event_target_value(&ev))
                    />
                    <select
                        aria-label="Sort games"
                        on:change=move |ev| set_sort_key.set(event_target_value(&ev))
                    >
                        <option value="alpha">"Alphabetical"</option>
                        <option value="weight-asc">"Weight (low to high)"</option>
                        <option value="weight-desc">"Weight (high to low)"</option>
                    </select>
                </div>
                <div class="game-card-grid">
                    <For
                        each=move || visible_types.get()
                        key=|gt| gt.id
                        children=move |gt: GameTypeInfo| {
                            let id = gt.id;
                            let name = gt.name.clone();
                            let meta = format!(
                                "{} | {}",
                                player_range(&gt.player_counts),
                                weight_text(gt.weight)
                            );
                            let blurb = gt.blurb.clone();
                            view! {
                                <label class="game-card">
                                    <input
                                        type="radio"
                                        name="game-type"
                                        class="sr-only"
                                        prop:checked=move || selected_type_id.get() == Some(id)
                                        on:change=move |_| select_game(&gt)
                                    />
                                    <span class="game-card-name">{name}</span>
                                    <span class="game-card-meta">{meta}</span>
                                    {(!blurb.is_empty())
                                        .then(|| view! { <span class="game-card-blurb">{blurb.clone()}</span> })}
                                </label>
                            }
                        }
                    />
                </div>
            </div>
            <div class="new-game-panel" node_ref=panel_ref>
                {move || {
                    let Some(gt) = selected_type_id
                        .get()
                        .and_then(|id| {
                            types.with_value(|t| t.iter().find(|g| g.id == id).cloned())
                        })
                    else {
                        return view! {
                            <p class="new-game-panel-empty">
                                "Select a game on the left to set up a match."
                            </p>
                        }
                        .into_any();
                    };
                    let version_select = (gt.versions.len() > 1).then(|| {
                        let versions = gt.versions.clone();
                        view! {
                            <div class="form-field">
                                <label class="form-label">"Version"</label>
                                <div class="form-control">
                                    <select on:change=move |ev| {
                                        set_selected_version_id
                                            .set(event_target_value(&ev).parse::<Uuid>().ok());
                                    }>
                                        {versions
                                            .iter()
                                            .map(|v| {
                                                let vid = v.id;
                                                view! {
                                                    <option
                                                        value=vid.to_string()
                                                        selected=move || {
                                                            selected_version_id.get() == Some(vid)
                                                        }
                                                    >
                                                        {v.name.clone()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                </div>
                            </div>
                        }
                    });
                    let counts = gt.player_counts.clone();
                    view! {
                        <h2>{gt.name.clone()}</h2>
                        <p class="game-card-meta">
                            {player_range(&gt.player_counts)} " | " {weight_text(gt.weight)}
                        </p>
                        {(!gt.blurb.is_empty())
                            .then(|| view! { <p class="new-game-blurb">{gt.blurb.clone()}</p> })}
                        <form on:submit=on_submit>
                            {version_select}
                            <div class="form-field">
                                <label class="form-label">"Players"</label>
                                <div class="form-control player-count-radios">
                                    {counts
                                        .iter()
                                        .map(|&n| {
                                            view! {
                                                <label>
                                                    <input
                                                        type="radio"
                                                        name="player-count"
                                                        prop:checked=move || player_count.get() == n
                                                        on:change=move |_| set_player_count.set(n)
                                                    />
                                                    " "
                                                    {n}
                                                </label>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                            {move || {
                                let n = (player_count.get() - 1).max(0) as usize;
                                (0..n)
                                    .map(|i| {
                                        view! {
                                            <OpponentSlotEditor
                                                i=i
                                                slots=opponent_slots
                                                set_slots=set_opponent_slots
                                                suggestions=suggestions
                                            />
                                        }
                                    })
                                    .collect_view()
                            }}
                            <div class="form-actions">
                                <input
                                    type="submit"
                                    value="Start game"
                                    disabled=move || create_action.pending().get()
                                />
                            </div>
                            {move || {
                                form_error
                                    .get()
                                    .map(|e| view! { <div class="form-error">{e}</div> })
                            }}
                            <Show when=move || {
                                create_action.value().get().is_some_and(|r| r.is_err())
                            }>
                                <div class="form-error">
                                    {move || {
                                        create_action
                                            .value()
                                            .get()
                                            .and_then(|r| r.err())
                                            .map(|e| e.to_string())
                                            .unwrap_or_default()
                                    }}
                                </div>
                            </Show>
                        </form>
                    }
                    .into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn OpponentSlotEditor(
    i: usize,
    slots: ReadSignal<Vec<OpponentSlot>>,
    set_slots: WriteSignal<Vec<OpponentSlot>>,
    suggestions: LocalResource<Result<Vec<OpponentSuggestion>, ServerFnError>>,
) -> impl IntoView {
    let slot = move || slots.get().get(i).cloned().unwrap_or_default();
    let mode = move || slot().mode();

    let set_mode = move |m: SlotMode| {
        set_slots.update(|v| {
            if let Some(s) = v.get_mut(i) {
                *s = match m {
                    SlotMode::Player => OpponentSlot::default(),
                    SlotMode::Email => OpponentSlot::Email(String::new()),
                    SlotMode::Bot => OpponentSlot::Bot {
                        name: format!("Bot {}", i + 1),
                        difficulty: "medium".to_string(),
                    },
                };
            }
        });
    };

    let pick_user = move |id: Uuid, name: String| {
        set_slots.update(|v| {
            if let Some(s) = v.get_mut(i) {
                *s = OpponentSlot::Player {
                    query: String::new(),
                    selected: Some((id, name.clone())),
                };
            }
        });
    };

    // Users already taken by other slots never appear as chips again.
    let taken = move || -> Vec<Uuid> {
        slots
            .get()
            .iter()
            .filter_map(|s| match s {
                OpponentSlot::Player {
                    selected: Some((id, _)),
                    ..
                } => Some(*id),
                _ => None,
            })
            .collect()
    };

    view! {
        <div class="form-field opponent-slot">
            <label class="form-label">"Opponent " {i + 1}</label>
            <div class="form-control slot-modes">
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Player
                        on:change=move |_| set_mode(SlotMode::Player)
                    />
                    " Player"
                </label>
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Email
                        on:change=move |_| set_mode(SlotMode::Email)
                    />
                    " Email"
                </label>
                <label>
                    <input
                        type="radio"
                        name=format!("slot-mode-{i}")
                        prop:checked=move || mode() == SlotMode::Bot
                        on:change=move |_| set_mode(SlotMode::Bot)
                    />
                    " Bot"
                </label>
            </div>
            <Show when=move || {
                matches!(slot(), OpponentSlot::Player { selected: Some(_), .. })
            }>
                <div class="form-control">
                    <span class="chip chip-selected">
                        {move || match slot() {
                            OpponentSlot::Player {
                                selected: Some((_, name)),
                                ..
                            } => name,
                            _ => String::new(),
                        }}
                        " "
                        <a
                            href="#"
                            on:click=move |ev| {
                                ev.prevent_default();
                                set_mode(SlotMode::Player);
                            }
                        >
                            "x"
                        </a>
                    </span>
                </div>
            </Show>
            <Show when=move || {
                matches!(slot(), OpponentSlot::Player { selected: None, .. })
            }>
                <div class="form-control chip-row">
                    {move || {
                        match suggestions.get() {
                            Some(Ok(sugs)) if !sugs.is_empty() => {
                                let tk = taken();
                                sugs.iter()
                                    .filter(|s| !tk.contains(&s.user_id))
                                    .map(|s| {
                                        let id = s.user_id;
                                        let name = s.name.clone();
                                        let label = s.name.clone();
                                        view! {
                                            <a
                                                href="#"
                                                class="chip"
                                                class:chip-friend=s.is_friend
                                                on:click=move |ev| {
                                                    ev.prevent_default();
                                                    pick_user(id, name.clone());
                                                }
                                            >
                                                {label}
                                            </a>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                            _ => ().into_any(),
                        }
                    }}
                </div>
            </Show>
            <Show when=move || mode() == SlotMode::Email>
                <div class="form-control">
                    <input
                        type="email"
                        placeholder="Email address"
                        required
                        prop:value=move || match slot() {
                            OpponentSlot::Email(e) => e,
                            _ => String::new(),
                        }
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(s) = v.get_mut(i) {
                                    *s = OpponentSlot::Email(val);
                                }
                            });
                        }
                    />
                </div>
            </Show>
            <Show when=move || mode() == SlotMode::Bot>
                <div class="form-control">
                    <input
                        type="text"
                        placeholder="Bot name"
                        required
                        prop:value=move || match slot() {
                            OpponentSlot::Bot { name, .. } => name,
                            _ => String::new(),
                        }
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(OpponentSlot::Bot { name, .. }) = v.get_mut(i) {
                                    *name = val;
                                }
                            });
                        }
                    />
                    <select
                        aria-label="Bot difficulty"
                        prop:value=move || match slot() {
                            OpponentSlot::Bot { difficulty, .. } => difficulty,
                            _ => "medium".to_string(),
                        }
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(OpponentSlot::Bot { difficulty, .. }) = v.get_mut(i) {
                                    *difficulty = val;
                                }
                            });
                        }
                    >
                        <option value="easy">"Easy"</option>
                        <option value="medium">"Medium"</option>
                        <option value="hard">"Hard"</option>
                    </select>
                </div>
            </Show>
        </div>
    }
}
```

- [ ] **Step 5: Switch the route and delete the old page**

In `rust/web/src/app.rs`:

1. Change the route at line 196 from
   `<Route path=StaticSegment("games") view=GamesPage/>`
   to
   `<Route path=StaticSegment("games") view=crate::new_game::NewGamePage/>`
2. Delete the old `OpponentSlot` enum, its `Default` impl, and the whole `GamesPage` component — lines 437-763, i.e. everything from the comment
   `/// Per-opponent slot state: a human (free-text email), a known user picked`
   through the closing `}` of `fn GamesPage`, stopping just before `#[component]\nfn DashboardPage`.

- [ ] **Step 6: Clean up newly-unused imports**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings
```
If clippy reports unused imports in `app.rs` (candidates: `Uuid`, `use_navigate`, `NavigateOptions` — only if no remaining component uses them), remove exactly the flagged ones and re-run until clean.
Expected final result: clean.

- [ ] **Step 7: Run the unit tests to verify they pass**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr new_game::
```
Expected: PASS (5 passed).

- [ ] **Step 8: Verify in the browser**

With the dev stack running (Postgres, NATS, `cargo leptos watch` or the project's usual dev command per `docs/DEV.md`), open `/games` and check:
- Grid of game cards with name, player range, weight, blurb; empty-state panel prompt on the right.
- Selecting a card highlights it and fills the panel; player-count radios default to the lowest count.
- Filters: player count hides non-matching games; search narrows by name; sort select reorders; filtering out the selected game empties the panel.
- Opponent slots: Player mode shows suggestion chips; Email and Bot modes work; Start game creates and redirects to `/games/{id}`.
- Below 60em: single column, selecting a game scrolls the panel into view.

- [ ] **Step 9: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/new_game.rs rust/web/src/lib.rs rust/web/src/app.rs rust/web/Cargo.toml rust/Cargo.lock
git commit -m "feat(web): rebuild new game page as browsable grid with setup panel (#44)"
```

---

### Task 9: Player typeahead in opponent slots

**Files:**
- Modify: `rust/web/src/new_game.rs` (`OpponentSlotEditor` only; imports)

**Interfaces:**
- Consumes: `crate::friends::search_users(query: String) -> Result<Vec<UserSearchResult>, ServerFnError>` and `crate::friends::UserSearchResult` (Task 6); `OpponentSlot::Player { query, selected }` state (Task 8); `.typeahead-results` styles (Task 7).
- Produces: the final `OpponentSlotEditor` — Player mode gains a search input (min 2 chars client-side), a result-chip list, an inline search error, with suggestion chips shown only before typing.

- [ ] **Step 1: Add the import**

In `rust/web/src/new_game.rs`, change the friends import line to:

```rust
use crate::friends::{OpponentSuggestion, UserSearchResult};
```

- [ ] **Step 2: Add search state and helpers to `OpponentSlotEditor`**

Inside `OpponentSlotEditor`, after the `taken` closure, add:

```rust
    let slot_query = move || match slot() {
        OpponentSlot::Player { query, .. } => query,
        _ => String::new(),
    };

    // One search action per slot; last completed response wins, which is
    // fine for a human typing (out-of-order stale responses only ever show
    // briefly and the next keystroke re-queries).
    let search_action = Action::new(|q: &String| {
        let q = q.clone();
        async move { crate::friends::search_users(q).await }
    });

    let search_results = move || -> Vec<UserSearchResult> {
        if slot_query().trim().chars().count() < 2 {
            return Vec::new();
        }
        match search_action.value().get() {
            Some(Ok(results)) => results,
            _ => Vec::new(),
        }
    };

    // Spec, error handling: search failure is inline under the slot; the
    // slot stays usable via Email mode.
    let search_error = move || -> Option<String> {
        if slot_query().trim().chars().count() < 2 {
            return None;
        }
        match search_action.value().get() {
            Some(Err(e)) => Some(format!("Search failed: {e}")),
            _ => None,
        }
    };
```

- [ ] **Step 3: Replace the Player-unselected `<Show>` block**

In `OpponentSlotEditor`'s view, replace the entire
`<Show when=move || { matches!(slot(), OpponentSlot::Player { selected: None, .. }) }>` block (the one that currently contains only the chip-row) with:

```rust
            <Show when=move || {
                matches!(slot(), OpponentSlot::Player { selected: None, .. })
            }>
                <div class="form-control">
                    <input
                        type="text"
                        placeholder="Search players"
                        aria-label=format!("Search players for opponent {}", i + 1)
                        prop:value=slot_query
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_slots.update(|v| {
                                if let Some(s) = v.get_mut(i) {
                                    *s = OpponentSlot::Player {
                                        query: val.clone(),
                                        selected: None,
                                    };
                                }
                            });
                            if val.trim().chars().count() >= 2 {
                                search_action.dispatch(val);
                            }
                        }
                    />
                </div>
                {move || {
                    search_error()
                        .map(|e| view! { <div class="form-error">{e}</div> })
                }}
                <ul class="typeahead-results">
                    {move || {
                        let tk = taken();
                        search_results()
                            .into_iter()
                            .filter(|r| !tk.contains(&r.user_id))
                            .map(|r| {
                                let id = r.user_id;
                                let name = r.name.clone();
                                let label = r.name.clone();
                                view! {
                                    <li>
                                        <a
                                            href="#"
                                            class="chip"
                                            on:click=move |ev| {
                                                ev.prevent_default();
                                                pick_user(id, name.clone());
                                            }
                                        >
                                            {label}
                                        </a>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
                <Show when=move || slot_query().is_empty()>
                    <div class="form-control chip-row">
                        {move || {
                            match suggestions.get() {
                                Some(Ok(sugs)) if !sugs.is_empty() => {
                                    let tk = taken();
                                    sugs.iter()
                                        .filter(|s| !tk.contains(&s.user_id))
                                        .map(|s| {
                                            let id = s.user_id;
                                            let name = s.name.clone();
                                            let label = s.name.clone();
                                            view! {
                                                <a
                                                    href="#"
                                                    class="chip"
                                                    class:chip-friend=s.is_friend
                                                    on:click=move |ev| {
                                                        ev.prevent_default();
                                                        pick_user(id, name.clone());
                                                    }
                                                >
                                                    {label}
                                                </a>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                                _ => ().into_any(),
                            }
                        }}
                    </div>
                </Show>
            </Show>
```

- [ ] **Step 4: Verify compile, clippy, tests**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr new_game::
```
Expected: clippy clean; 5 unit tests PASS.

- [ ] **Step 5: Verify in the browser**

On `/games`, select a game with 3+ players, set a Player slot:
- Before typing: suggestion chips (friends double-bordered) appear; clicking one fixes the slot and shows the removable selected chip.
- Typing 1 char: no search fires. Typing 2+ chars: matching users appear as result chips (max 10, never yourself); clicking one fixes the slot.
- A user already picked in another slot does not appear in chips or results.
- Inline error path (best-effort manual): in devtools, block the `/api/search_users` request (or briefly stop the web server mid-typing) and confirm "Search failed: ..." renders under the input while Email mode stays usable.

- [ ] **Step 6: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/new_game.rs
git commit -m "feat(web): player typeahead with inline errors in opponent slots (#44)"
```

---

### Task 10: Full verification pass

**Files:** none created; fixes only if checks fail.

**Interfaces:**
- Consumes: everything above.
- Produces: a green CI-equivalent run and a manual UI sign-off.

- [ ] **Step 1: Run the full CI-equivalent check suite**

Run, from `/home/beefsack/Development/brdgme/rust` (Postgres + `DATABASE_URL` required):
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude web --all-targets -- -D warnings
cargo clippy -p web --all-targets --features ssr -- -D warnings
cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets && cd ..
cargo test --workspace --exclude web
cargo test -p web --features ssr
```
Expected: every command exits 0. If `fmt` fails, run `cargo fmt --all` and re-check. If `sqlx prepare --check` fails, re-run `cargo sqlx prepare -- --tests --features ssr --all-targets` in `rust/web` and commit the cache.

- [ ] **Step 2: Manual UI verification across themes and breakpoints**

Existing convention: no browser test harness; verify manually.
- Themes: spot-check at least brdgme-light, brdgme-dark, solarized-light, dracula, and one tritanopia theme via Settings — card borders, selected-card outline, chips, and error text must all be legible; selection must be recognizable without relying on hue (outline weight) in every theme checked.
- Breakpoints: >= 60em two-pane with the panel staying sticky while the grid scrolls; < 60em single column with auto-scroll to the panel on selection; also check ~80em where the sidebar collapses.
- Full flow: filter, search, sort, select, pick versions (a game with two public versions may not exist in dev — version select correctly hidden for single-version games), radios, all three slot modes, create a bot game, land on `/games/{id}`.

- [ ] **Step 3: Commit any verification fixes**

```bash
cd /home/beefsack/Development/brdgme
git add -A
git commit -m "fix(web): verification fixes for new game page (#44)"
```
(Skip if nothing changed.)

---

## Self-Review Notes (kept for the executor)

- Spec coverage: blurb pipeline (Tasks 1-3), GameTypeInfo exposure (Task 4), roster validation (Task 5), user search fn (Task 6), UI grid/filters/sort/panel/version/count/slots (Tasks 7-9), error handling (page-level error retained in `NewGamePage`; inline search error Task 9; roster error surfaces via the existing action-error div), testing section (server fn logic tests Tasks 1/5/6, operator upsert test Task 2, manual UI Task 10). Board preview, rules display, bot enum, invites: intentionally absent (out of scope).
- The old auto-select-first-game behavior is intentionally dropped: the spec requires an empty-state panel until the user picks a game.
- `weight` remains bound as `f64` in the operator's unchecked insert (existing behavior, Postgres assignment-casts to the `real` column); the web model reads it as `f32` via compile-checked macros, which pins the column type.
