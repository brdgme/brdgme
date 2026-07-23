# Context: Concede with Bot Replacement & End Game (#47)

Date: 2026-07-23
Spec: `docs/superpowers/specs/2026-07-23-47-concede-bot-replacement-design.md`

---

## 1. game_players schema

Created in `rust/web/migrations/001_initial_schema.sql:183-201`:

```sql
CREATE TABLE IF NOT EXISTS public.game_players (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    created_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    updated_at timestamp without time zone DEFAULT timezone('utc'::text, now()) NOT NULL,
    game_id uuid NOT NULL,
    user_id uuid NOT NULL,
    "position" integer NOT NULL,
    color text NOT NULL,
    has_accepted boolean NOT NULL,
    is_turn boolean NOT NULL,
    is_turn_at timestamp without time zone NOT NULL,
    last_turn_at timestamp without time zone NOT NULL,
    is_eliminated boolean NOT NULL,
    is_read boolean NOT NULL,
    points real,
    undo_game_state text,
    place integer,
    rating_change integer
);
```

Altered in `rust/web/migrations/003_game_bots.sql:18-23`:

```sql
ALTER TABLE game_players
    ALTER COLUMN user_id DROP NOT NULL,
    ADD COLUMN game_bot_id UUID REFERENCES game_bots(id),
    ADD CONSTRAINT game_players_user_or_bot CHECK (
        (user_id IS NOT NULL) != (game_bot_id IS NOT NULL)
    );
```

The XOR CHECK constraint name is `game_players_user_or_bot`. The new migration
must `DROP CONSTRAINT game_players_user_or_bot` and add
`CHECK (user_id IS NOT NULL OR game_bot_id IS NOT NULL)`.

Additional columns added by later migrations (visible in db.rs queries):
- `turn_reminder_sent_at TIMESTAMPTZ` (referenced in concede_game, sweep)
- `rating_before INTEGER` (migration 017, referenced in apply_rating_changes)
- `email_token` (referenced in email inbound tests)

The `GamePlayer` model struct (`rust/web/src/models/game.rs:51-69`):

```rust
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_id: Uuid,
    pub user_id: Option<Uuid>,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: PrimitiveDateTime,
    pub place: Option<i32>,
    pub last_turn_at: PrimitiveDateTime,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub rating_change: Option<i32>,
}
```

Note: `rating_before`, `turn_reminder_sent_at`, `email_token` are NOT in the
model struct - they are accessed via raw queries only.

---

## 2. game_bots schema

Created in `rust/web/migrations/003_game_bots.sql:7-16`:

```sql
CREATE TABLE game_bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    game_id UUID NOT NULL REFERENCES games(id),
    name TEXT NOT NULL,
    difficulty TEXT NOT NULL CHECK (difficulty IN ('easy', 'medium', 'hard')),
    personality TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (game_id, name)
);
```

Migration 013 renamed `difficulty` to `bot_name` and dropped the check:

```sql
ALTER TABLE game_bots RENAME COLUMN difficulty TO bot_name;
ALTER TABLE game_bots DROP CONSTRAINT IF EXISTS game_bots_difficulty_check;
```

The `bots` table (migration 013, `rust/web/migrations/013_bot_efficacy.sql:1-11`)
is the admin-managed bot *definitions* table (distinct from `game_bots` which is
per-game bot *slots*):

```sql
CREATE TABLE bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL UNIQUE,
    display_order INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    include_basic_strategy BOOLEAN NOT NULL DEFAULT true,
    include_advanced_strategy BOOLEAN NOT NULL DEFAULT false,
    temperature REAL NOT NULL DEFAULT 0.2,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

The spec's `can_replace_humans` flag goes on the `bots` table (admin-managed
definitions), NOT on `game_bots` (per-game slots).

---

## 3. concede_game DB function

`rust/web/src/db.rs:1282-1336`:

```rust
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
pub async fn concede_game(
    pool: &PgPool,
    game_id: Uuid,
    conceding_player_id: Uuid,
    conceding_name: &str,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET is_finished = true, finished_at = NOW(), updated_at = NOW() WHERE id = $1",
        game_id
    )
    .execute(&mut *tx)
    .await?;

    let players = sqlx::query!(
        "SELECT id FROM game_players WHERE game_id = $1 ORDER BY position",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // Assigns place 1 to every non-conceding player and place 2 to the conceder.
    // Only correct for 2-player games; callers must enforce that constraint.
    debug_assert!(players.len() == 2, "concede_game assumes exactly 2 players");
    for p in &players {
        let place: i32 = if p.id == conceding_player_id { 2 } else { 1 };
        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = false, place = $1, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL, updated_at = NOW()
               WHERE id = $2"#,
        )
        .bind(place)
        .bind(p.id)
        .execute(&mut *tx)
        .await?;
    }

    let log_body = format!("{} conceded.", conceding_name);
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        log_body
    )
    .execute(&mut *tx)
    .await?;

    apply_rating_changes(&mut tx, game_id).await?;

    tx.commit().await?;
    Ok(())
}
```

Key observations:
- Ends the game immediately (2-player only).
- Sets `place` directly (1 for opponent, 2 for conceder).
- Calls `apply_rating_changes` in the same tx.
- The new multi-player concede must NOT end the game; it sets `left_at` and
  `game_bot_id` instead.

---

## 4. Concede server fn

`rust/web/src/game/server_fns.rs:807-857`:

```rust
#[server(ConcedeGame, "/api")]
pub async fn concede_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let http_client = expect_context::<reqwest::Client>();
    let resend = expect_context::<Option<resend_rs::Resend>>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let ge = crate::db::find_game_extended(&pool, game_id)
        .await
        .map_err(internal("concede_game: find game"))?
        .ok_or_else(|| ServerFnError::new("Game not found"))?;
    let before = ge.clone();

    if ge.game.is_finished {
        return Err(ServerFnError::new("Game is already finished"));
    }
    if ge.game_players.len() != 2 {
        return Err(ServerFnError::new(
            "Concede is only available in 2-player games",
        ));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.user.as_ref().is_some_and(|u| u.id == user.id))
        .ok_or_else(|| ServerFnError::new("You are not a player in this game"))?;

    crate::db::concede_game(&pool, game_id, player.game_player.id, player.name())
        .await
        .map_err(internal("concede_game: concede"))?;

    broadcaster.broadcast_game_update(game_id).await;

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

### is_2player computation

`rust/web/src/game/server_fns.rs:325`:

```rust
is_2player: ge.game_players.len() == 2,
```

This counts ALL players (humans + bots). The spec replaces this with
active-human-count logic.

### GameViewData struct

`rust/web/src/game/server_fns.rs:57-86`:

```rust
pub struct GameViewData {
    pub id: Uuid,
    pub version_id: Uuid,
    pub type_name: String,
    pub version_name: String,
    pub html: String,
    pub is_my_turn: bool,
    pub is_finished: bool,
    pub can_undo: bool,
    pub restarted_game_id: Option<Uuid>,
    pub previous_game_id: Option<Uuid>,
    pub restart_proposal_id: Option<Uuid>,
    pub is_2player: bool,
    pub players: Vec<PlayerViewData>,
    pub command_spec: Option<brdgme_game::command::Spec>,
    pub player_style: String,
    pub viewer_is_admin: bool,
    pub viewer_user_id: Option<Uuid>,
}
```

### PlayerViewData struct

`rust/web/src/game/server_fns.rs:88-115`:

```rust
pub struct PlayerViewData {
    pub name: String,
    pub color: String,
    pub rating: i32,
    pub rating_change: Option<i32>,
    pub points: f32,
    pub place: Option<i32>,
    pub is_turn: bool,
    pub is_bot: bool,
    pub bot_name: Option<String>,
    pub user_id: Option<Uuid>,
    pub can_add_friend: bool,
    pub form: Vec<crate::stats::FormResult>,
}
```

---

## 5. Concede email command

`rust/web/src/email/commands.rs:886-922`:

```rust
async fn run_concede(ctx: &EmailCommandCtx<'_>) -> Result<CommandReply, CommandError> {
    let ge = crate::db::find_game_extended(ctx.pool, ctx.game_id)
        .await?
        .ok_or_else(|| CommandError::User("Game not found".to_string()))?;

    if ge.game.is_finished {
        return Err(CommandError::User("Game is already finished".to_string()));
    }
    if ge.game_players.len() != 2 {
        return Err(CommandError::User(
            "Concede is only available in 2-player games".to_string(),
        ));
    }

    let player = ge
        .game_players
        .iter()
        .find(|p| p.game_player.id == ctx.game_player_id)
        .ok_or_else(|| CommandError::User("You are not a player in this game".to_string()))?;

    let before = ge.clone();
    crate::db::concede_game(ctx.pool, ctx.game_id, ctx.game_player_id, player.name())
        .await
        .map_err(CommandError::Internal)?;

    ctx.broadcaster.broadcast_game_update(ctx.game_id).await;
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

### Email command dispatch (verb matching)

`rust/web/src/email/commands.rs:1135-1176`:

```rust
pub async fn dispatch_email_command(
    ctx: &EmailCommandCtx<'_>,
    line: &str,
) -> Result<CommandReply, CommandError> {
    let trimmed = line.trim();
    let (verb, arg) = match trimmed.split_once(' ') {
        Some((v, a)) => (v, Some(a.trim())),
        None => (trimmed, None),
    };
    let verb_lower = verb.to_ascii_lowercase();

    match verb_lower.as_str() {
        "concede" => return run_concede(ctx).await,
        "undo" => return run_undo(ctx).await,
        "restart" => return run_restart(ctx).await,
        "rules" => return run_rules(ctx, parse_rules_arg(arg)).await,
        "help" | "commands" => return Ok(CommandReply::Status(help_text())),
        "new" => { /* ... */ }
        "bump" => { /* ... */ }
        "list" => return run_list_command(ctx.pool).await,
        _ => {}
    }
    // falls through to subscribe_toggle, settings, then game command
}
```

### EmailCommandCtx

`rust/web/src/email/commands.rs:1-11`:

```rust
pub struct EmailCommandCtx<'a> {
    pub pool: &'a sqlx::PgPool,
    pub http_client: &'a reqwest::Client,
    pub broadcaster: &'a crate::websocket::GameBroadcaster,
    pub jetstream: &'a async_nats::jetstream::Context,
    pub resend: Option<&'a resend_rs::Resend>,
    pub game_id: uuid::Uuid,
    pub game_player_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub position: usize,
}
```

The new "end" verb would be added to the match block alongside "concede".

---

## 6. Concede UI button

`rust/web/src/components/game.rs:114-126`:

```rust
<Show when=move || !is_finished && is_2player>
    <div>
        <a href="#" on:click=move |ev| {
            ev.prevent_default();
            let confirmed = web_sys::window()
                .and_then(|w| w.confirm_with_message("Are you sure you want to concede?").ok())
                .unwrap_or(false);
            if confirmed {
                concede_action.dispatch(ConcedeGame { game_id });
            }
        }>"Concede"</a>
    </div>
</Show>
```

The `is_2player` value comes from `GameViewData.is_2player` (line 31):

```rust
let is_2player = data.is_2player;
```

The component is `GameMeta` (`rust/web/src/components/game.rs:26`).

---

## 7. Bot turn triggering

`rust/web/src/game/mod.rs:172-186`:

```rust
/// Publishes a `bot.turn` event (attempt 0) for every bot player whose turn
/// it currently is. The bot picks these up from the `bot-turn` durable
/// consumer; the monolith never talks to the bot directly. Gives up with a
/// warn log if the bot-turn query fails.
#[cfg(feature = "ssr")]
pub async fn trigger_bot_turns(
    pool: &sqlx::PgPool,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
) {
    match crate::db::find_bot_turns(pool, game_id).await {
        Ok(turns) => publish_bot_turns(jetstream, game_id, &turns, 0).await,
        Err(e) => tracing::warn!(%game_id, "Failed to query bot turns: {}", e),
    }
}
```

### find_bot_turns query

`rust/web/src/db.rs:520-534`:

```rust
pub async fn find_bot_turns(pool: &PgPool, game_id: Uuid) -> Result<Vec<BotTurn>> {
    sqlx::query_as!(
        BotTurn,
        r#"
        SELECT gp.position, gb.bot_name
        FROM game_players gp
        JOIN game_bots gb ON gp.game_bot_id = gb.id
        WHERE gp.game_id = $1 AND gp.is_turn = true
        "#,
        game_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
```

Key insight: it JOINs on `gp.game_bot_id = gb.id`. Since a replaced human will
have `game_bot_id` set, this query will naturally pick them up as bot turns.
No change needed here (confirms spec D9).

### broadcast_and_trigger

`rust/web/src/game/mod.rs:50-58`:

```rust
pub async fn broadcast_and_trigger(
    pool: &sqlx::PgPool,
    broadcaster: &crate::websocket::GameBroadcaster,
    jetstream: &async_nats::jetstream::Context,
    game_id: uuid::Uuid,
) {
    broadcaster.broadcast_game_update(game_id).await;
    trigger_bot_turns(pool, jetstream, game_id).await;
}
```

The concede server fn currently only calls `broadcaster.broadcast_game_update`
(no bot trigger). The new concede-with-replacement must call
`broadcast_and_trigger` (or at minimum `trigger_bot_turns`) so the replacement
bot acts if it's now their turn.

---

## 8. Rating/ELO application

`rust/web/src/db.rs:1530-1680`:

```rust
/// Applies ELO rating changes for a game that just transitioned to finished
/// with placings. Must be called within the same transaction as the
/// placings write. No-op if the idempotency guard trips (any player already
/// has a rating_change). Bot players are excluded from the calculation;
/// only human players are rated against each other.
#[cfg(feature = "ssr")]
async fn apply_rating_changes(tx: &mut sqlx::PgConnection, game_id: Uuid) -> Result<()> {
    struct PlayerRow {
        id: Uuid,
        position: i32,
        user_id: Option<Uuid>,
        game_bot_id: Option<Uuid>,
        place: Option<i32>,
        rating_change: Option<i32>,
    }

    let players = sqlx::query_as!(
        PlayerRow,
        "SELECT id, position, user_id, game_bot_id, place, rating_change FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    if players.iter().any(|p| p.rating_change.is_some()) {
        return Ok(()); // Idempotency guard
    }
    if players.iter().all(|p| p.place.is_none()) {
        return Ok(());
    }

    // ... gets game_type_id ...

    let mut rated_players = Vec::with_capacity(players.len());
    for p in &players {
        if p.game_bot_id.is_some() {  // <-- CURRENT BOT EXCLUSION
            continue;
        }
        let user_id = p.user_id.ok_or_else(|| {
            anyhow::anyhow!("game_player {}: user_id missing for human player", p.id)
        })?;
        // ... ensures game_type_users row, fetches rating ...
        rated_players.push(RatedPlayer { position: p.position, user_id, rating });
    }

    if rated_players.len() < 2 {
        return Ok(());
    }

    // Pairwise ELO using `place` column
    let places: HashMap<i32, i32> = players
        .iter()
        .map(|p| (p.position, p.place.unwrap_or(i32::MAX)))
        .collect();

    // ... pairwise comparison, elo_rating_change ...

    // Writes rating_change and rating_before to game_players
    // Updates game_type_users.rating and peak_rating
}
```

### Bot exclusion change needed

Current: `if p.game_bot_id.is_some() { continue; }` (line 1582)
Spec D6: change to `if p.user_id.is_none() { continue; }` (pure bots only).

### Placing source change needed

Current: uses `p.place` (the game placing).
Spec D6: must use `ranked_placing` for ELO. The function will need to read
the new `ranked_placing` column instead of (or in addition to) `place`.

---

## 9. Form line computation

### recent_form_for_game_type (used on game page)

`rust/web/src/stats/queries.rs:619-683`:

```rust
pub async fn recent_form_for_game_type(
    pool: &PgPool,
    user_ids: &[Uuid],
    game_type_id: Uuid,
    per_user: i64,
) -> Result<HashMap<Uuid, Vec<super::FormResult>>> {
    // SQL:
    // WITH qualifying AS (
    //     SELECT gp.user_id, g.id AS game_id, g.finished_at, gp.place,
    //            gp.rating_change,
    //            (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS player_count,
    //            row_number() OVER (PARTITION BY gp.user_id ORDER BY g.finished_at DESC, g.id) AS rn
    //     FROM game_players gp
    //     JOIN games g ON g.id = gp.game_id
    //     JOIN game_versions gv ON gv.id = g.game_version_id
    //     JOIN game_types gt ON gt.id = gv.game_type_id
    //     WHERE gp.user_id = ANY($1)
    //       AND gt.id = $2
    //       AND g.is_finished = true
    //       AND (SELECT count(*) FROM game_players gp3
    //            WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL) >= 2
    // )
    // SELECT ... FROM qualifying WHERE rn <= $3
    // ORDER BY user_id, finished_at ASC, game_id
}
```

Called from `get_game_details` at `rust/web/src/game/server_fns.rs:283-290`:

```rust
let form_by_user = crate::stats::recent_form_for_game_type(
    &pool,
    &human_user_ids,
    ge.game_version.game_type_id,
    10,
)
```

### FormResult struct

`rust/web/src/stats/mod.rs:126-132`:

```rust
pub struct FormResult {
    pub game_id: Uuid,
    pub finished_at: Option<PrimitiveDateTime>,
    pub place: Option<i32>,
    pub player_count: i64,
    pub rating_change: Option<i32>,
}
```

### form_cell (display)

`rust/web/src/stats/viz.rs:42-50`:

```rust
pub fn form_cell(place: Option<i32>) -> (String, &'static str) {
    match place {
        Some(1) => ("1".to_string(), "form-gold"),
        Some(2) => ("2".to_string(), "form-silver"),
        Some(3) => ("3".to_string(), "form-bronze"),
        Some(p) => (p.to_string(), "form-other"),
        None => ("-".to_string(), "form-none"),
    }
}
```

### FormStrip component

`rust/web/src/stats/viz.rs:52-65`:

```rust
#[component]
pub fn FormStrip(results: Vec<FormResult>) -> impl IntoView {
    view! {
        <span class="form-strip" title="recent form (oldest to newest)">
            {results.into_iter().map(|r| {
                let (label, class) = form_cell(r.place);
                view! { <span class=class>{label}</span> }
            }).collect_view()}
        </span>
    }
}
```

### Changes needed for spec D7

- Query must use `ranked_placing` instead of `place`.
- Query already filters `gp.user_id IS NOT NULL` (via `gp.user_id = ANY($1)`),
  so pure bots are excluded. But replaced humans (both user_id AND game_bot_id
  set) will still appear - correct per spec.
- The `per_user` limit is currently 10; spec says show last 5. The display
  component may need trimming, or the query limit changed.
- Spec says "most recent on the left" and "only leftmost is bold" - current
  order is oldest-to-newest (ASC). Display logic needs reversal + bold styling.

---

## 10. Admin bot management page

File: `rust/web/src/admin.rs`

### BotRow struct (line 33-42)

```rust
pub struct BotRow {
    pub id: Uuid,
    pub name: String,
    pub display_order: i32,
    pub enabled: bool,
    pub include_basic_strategy: bool,
    pub include_advanced_strategy: bool,
    pub temperature: f32,
}
```

### DB functions

- `list_bots` (line 92): `SELECT id, name, display_order, enabled, include_basic_strategy, include_advanced_strategy, temperature FROM bots ORDER BY display_order`
- `create_bot` (line 127): INSERT with RETURNING
- `update_bot` (line 159): `UPDATE bots SET name = $2, temperature = $3, include_basic_strategy = $4, include_advanced_strategy = $5, enabled = $6, updated_at = now() WHERE id = $1`
- `delete_bot` (line 200): `DELETE FROM bots WHERE id = $1`

### Server fns

- `admin_list_bots` (line 618): `#[server(AdminListBots, "/api")]`
- `admin_create_bot` (line 637): `#[server(AdminCreateBot, "/api")]`
- `admin_update_bot` (line 668): `#[server(AdminUpdateBot, "/api")]`
- `admin_delete_bot` (line 722): `#[server(AdminDeleteBot, "/api")]`

All check `is_user_admin` before proceeding.

### UI component

`BotsSection` component (line 1044) renders the bot list with create/edit/delete
actions. The `can_replace_humans` checkbox would be added to the create/update
forms and the `BotRow` struct.

---

## 11. Migration format

Highest migration: `021_add_game_visibility.sql`. New migration: `022_*`.

Naming convention: `NNN_snake_case_description.sql`

Format example (`021_add_game_visibility.sql`):

```sql
-- Comment explaining purpose and references
ALTER TABLE public.users
    ADD COLUMN game_visibility text NOT NULL DEFAULT 'public'
        CHECK (game_visibility IN ('public', 'friends', 'private'));

CREATE INDEX idx_users_game_visibility ON public.users (game_visibility);
```

Format example (`003_game_bots.sql`):

```sql
-- Comment block
CREATE TABLE game_bots (...);
ALTER TABLE game_players ...;
```

Migrations are run by sqlx (the `sqlx::migrate!()` macro in the web crate).
They are immutable once applied. The new migration will be `022_*.sql`.

---

## 12. Testing approach

### DB tests

Tests use `#[sqlx::test]` which provides a fresh `PgPool` per test (migrated
database). Example from `rust/web/src/db.rs:4356`:

```rust
#[sqlx::test]
async fn find_bot_turns_returns_only_on_turn_bots(pool: PgPool) {
    let creator = make_user(&pool, "creator").await;
    let (_, game_version_id) = make_game_type_and_version(&pool).await;
    let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;
    // ... assertions ...
}
```

### Test fixture helpers (db.rs:3102-3182)

- `make_user(pool, name) -> User` - inserts a user
- `make_game_type_and_version(pool) -> (Uuid, Uuid)` - creates game type + version
- `make_game_with_players(pool, game_version_id, creator_id, opponent_ids, bot_count, whose_turn) -> Game` - creates a game with humans + bots using `create_game_with_users`

### Server fn tests

`rust/web/src/game/server_fns.rs` has a `#[cfg(test)]` module (around line 1295+)
with its own `make_user` helper and tests for `restart_core`, `get_restart_prefill`.

### AGENTS.md caveat

DB tests fail locally in plain agent runs (no Postgres container). This is
pre-existing and not caused by new changes. The full test suite runs via
`scripts/rust-test.sh` which spins up temporary Postgres + NATS containers.

---

## 13. GameViewData / view data flow

1. `get_game_details` server fn (`server_fns.rs:230`) is called by the client.
2. It calls `crate::db::find_game_extended(&pool, game_id)` (`db.rs:400`) which:
   - Fetches the `games` row
   - Fetches `game_versions` and `game_types`
   - JOINs `game_players` with `users`, `game_type_users`, `game_bots`
   - Returns `GameExtended { game, game_type, game_version, game_players: Vec<GamePlayerExtended> }`
3. `GamePlayerExtended` (`db.rs:340-345`):
   ```rust
   pub struct GamePlayerExtended {
       pub game_player: crate::models::game::GamePlayer,
       pub user: Option<crate::models::user::User>,
       pub game_bot: Option<crate::models::game::GameBot>,
       pub game_type_user: crate::models::game::GameTypeUser,
   }
   ```
4. The server fn maps `GameExtended` into `GameViewData` (serializable to client).
5. The `GameMeta` component (`components/game.rs:26`) receives `GameViewData` and
   renders the sidebar (actions, players, form).

The `find_game_extended` query (`db.rs:419-448`) will need to also SELECT the
new `ranked_placing` and `left_at` columns once they exist.

---

## 14. Elimination mechanics

### Game engine signals elimination via Status

`rust/lib/game/src/game.rs:21-30`:

```rust
pub enum Status {
    Active {
        whose_turn: Vec<usize>,
        eliminated: Vec<usize>,
    },
    Finished {
        placings: Vec<usize>,
        stats: Vec<HashMap<String, Stat>>,
    },
}
```

### Web layer records elimination

`rust/web/src/game/mod.rs:12-17` (StatusUpdate):

```rust
pub struct StatusUpdate {
    pub is_finished: bool,
    pub whose_turn: Vec<usize>,
    pub eliminated: Vec<usize>,
    pub placings: Vec<usize>,
}
```

In `update_game_command_success` (`db.rs:1744`):

```rust
let is_eliminated = status.eliminated.contains(&pos);
// ...
sqlx::query(
    r#"UPDATE game_players
       SET is_turn = $1, place = $2, is_eliminated = $3, points = $4, ...
       WHERE id = $8"#,
)
```

### Key finding: no `left_at` exists yet

There is NO `left_at` column currently. The `is_eliminated` boolean is set on
every command update based on the game engine's current `eliminated` list. It
is a state flag (currently eliminated or not), not a timestamp.

The spec's `left_at` is a NEW column that must be set:
- When a player concedes (set explicitly in the concede flow).
- When a player is eliminated (must be set when `is_eliminated` transitions
  from false to true in `update_game_command_success`).

### Audit concern

The elimination flag is re-written on every command (it's positional from the
game engine's `eliminated` vec). If a game un-eliminates a player (unlikely but
possible in some game rules), `left_at` should only be set on the first
transition. The implementation should use:

```sql
SET left_at = CASE WHEN is_eliminated = false AND $new_eliminated = true
                   THEN NOW() ELSE left_at END
```

---

## 15. Restart game

Yes, restart exists. It is a full feature with:

### restart_core (shared by web + email)

`rust/web/src/game/server_fns.rs:868-1040`:

```rust
pub(crate) async fn restart_core(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    user_id: Uuid,
    old_game_id: Uuid,
    version: &crate::models::game::GameVersion,
    opponent_ids: &[Uuid],
    opponent_emails: &[String],
    bot_slots: &[BotSlot],
) -> Result<RestartOutcome, ServerFnError>
```

- Requires `is_finished = true`.
- Uses `FOR UPDATE` lock on the old game row for race safety.
- Solo (no human opponents): creates new game directly, links via `restarted_game_id`.
- Multi-human: creates a restart proposal (invite flow).
- Returns `RestartOutcome::Created` or `RestartOutcome::AlreadyRestarted`.

### Web server fn

`restart_game_with_roster` (`server_fns.rs:1042`): `#[server(RestartGameWithRoster, "/api")]`

### Email command

`run_restart` (`email/commands.rs:983`): rebuilds the roster from the finished
game's players and calls `restart_core`.

### UI

The "Restart" link in `components/game.rs:127-131`:

```rust
<Show when=move || can_restart>
    <div>
        <A href=restart_href.clone()>"Restart"</A>
    </div>
</Show>
```

Where `can_restart = is_finished && restarted_game_id.is_none() && restart_proposal_id.is_none()`.

The spec's "End game" button reveals the same restart affordance.

---

## Decisions

1. **`can_replace_humans` goes on the `bots` table** (admin-managed bot
   definitions), not `game_bots` (per-game slot instances). The spec says
   "admin flag marking a bot as eligible" - this matches the `bots` table which
   is what the admin page manages.

2. **`is_2player` is total player count, not human count.** The current
   `ge.game_players.len() == 2` includes bots. The spec's "exactly 2 humans
   remain" logic must count only players where `user_id IS NOT NULL` and
   `left_at IS NULL` (active humans).

3. **`apply_rating_changes` bot exclusion** currently uses
   `game_bot_id.is_some()`. After the migration, replaced humans will have BOTH
   `user_id` and `game_bot_id` set. The exclusion must change to
   `user_id.is_none()` (pure bots only).

4. **`apply_rating_changes` placing source** currently uses `place`. The spec
   requires `ranked_placing` for ELO. The function must be updated to read
   `ranked_placing` (falling back gracefully if NULL for pure bots).

5. **`left_at` must be set on elimination transitions** in
   `update_game_command_success` (and the undo path at `db.rs:1435`). This is
   the "audit elimination mechanics" prerequisite from the spec.

6. **The concede server fn must call `trigger_bot_turns`** (or
   `broadcast_and_trigger`) after a replacement, since the replacement bot may
   need to act immediately. The current 2-player concede does not trigger bots
   (the game ends).

7. **`find_bot_turns` needs no change** - it JOINs on `game_bot_id` which will
   be set for replaced humans. Confirmed by spec D9.

8. **Form query uses `gp.place`** currently. Must switch to `ranked_placing`.
   The `recent_form` and `recent_form_for_game_type` queries both need updating.

9. **The `GamePlayer` model struct** does not include `rating_before`,
   `turn_reminder_sent_at`, or `email_token`. The new `ranked_placing` and
   `left_at` columns can follow the same pattern (raw queries) or be added to
   the struct. Given that `find_game_extended` already SELECTs most columns,
   adding them to the struct + query is cleaner.

10. **`debug_assert!(players.len() == 2)` in `concede_game`** - the existing
    function is 2-player only. The new multi-player concede should be a
    separate function (e.g. `concede_game_replace`) or the existing function
    refactored with a branch. Keeping the old path intact for the no-bots
    2-player case is safest.
