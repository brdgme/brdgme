#[cfg(feature = "ssr")]
use crate::game::StatusUpdate;
#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

pub use crate::game::server_fns::BotSlot;

#[cfg(feature = "ssr")]
fn build_user_from_row(
    id: Option<Uuid>,
    created_at: Option<time::PrimitiveDateTime>,
    updated_at: Option<time::PrimitiveDateTime>,
    name: Option<String>,
    pref_colors: Option<Vec<String>>,
) -> Result<Option<crate::models::user::User>> {
    let Some(id) = id else { return Ok(None) };
    Ok(Some(crate::models::user::User {
        id,
        created_at: created_at
            .ok_or_else(|| anyhow::anyhow!("user {id}: created_at missing from LEFT JOIN row"))?,
        updated_at: updated_at
            .ok_or_else(|| anyhow::anyhow!("user {id}: updated_at missing from LEFT JOIN row"))?,
        name: name.ok_or_else(|| anyhow::anyhow!("user {id}: name missing from LEFT JOIN row"))?,
        pref_colors: pref_colors
            .ok_or_else(|| anyhow::anyhow!("user {id}: pref_colors missing from LEFT JOIN row"))?,
        theme: None,
        is_admin: false,
    }))
}

#[cfg(feature = "ssr")]
fn build_game_bot_from_row(
    id: Option<Uuid>,
    game_id: Option<Uuid>,
    name: Option<String>,
    bot_name: Option<String>,
) -> Result<Option<crate::models::game::GameBot>> {
    let Some(id) = id else { return Ok(None) };
    Ok(Some(crate::models::game::GameBot {
        id,
        game_id: game_id
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: game_id missing from LEFT JOIN row"))?,
        name: name
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: name missing from LEFT JOIN row"))?,
        bot_name: bot_name
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: bot_name missing from LEFT JOIN row"))?,
    }))
}

#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
fn build_game_type_user(
    id: Option<Uuid>,
    created_at: Option<time::PrimitiveDateTime>,
    updated_at: Option<time::PrimitiveDateTime>,
    game_type_id: Option<Uuid>,
    user_id: Option<Uuid>,
    last_game_finished_at: Option<time::PrimitiveDateTime>,
    rating: Option<i32>,
    peak_rating: Option<i32>,
    default_user_id: Option<Uuid>,
    default_game_type_id: Uuid,
    default_ts: time::PrimitiveDateTime,
) -> crate::models::game::GameTypeUser {
    match (
        id,
        created_at,
        updated_at,
        game_type_id,
        user_id,
        rating,
        peak_rating,
    ) {
        (
            Some(id),
            Some(created_at),
            Some(updated_at),
            Some(game_type_id),
            Some(user_id),
            Some(rating),
            Some(peak_rating),
        ) => crate::models::game::GameTypeUser {
            id,
            created_at,
            updated_at,
            game_type_id,
            user_id,
            last_game_finished_at,
            rating,
            peak_rating,
        },
        _ => crate::models::game::GameTypeUser {
            id: Uuid::nil(),
            created_at: default_ts,
            updated_at: default_ts,
            game_type_id: default_game_type_id,
            user_id: default_user_id.unwrap_or(Uuid::nil()),
            last_game_finished_at: None,
            rating: 1200,
            peak_rating: 1200,
        },
    }
}

#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
fn build_game_player_from_row(
    id: Uuid,
    created_at: time::PrimitiveDateTime,
    updated_at: time::PrimitiveDateTime,
    game_id: Uuid,
    user_id: Option<Uuid>,
    position: i32,
    color: String,
    has_accepted: bool,
    is_turn: bool,
    is_turn_at: time::PrimitiveDateTime,
    place: Option<i32>,
    last_turn_at: time::PrimitiveDateTime,
    is_eliminated: bool,
    is_read: bool,
    points: Option<f32>,
    undo_game_state: Option<String>,
    rating_change: Option<i32>,
) -> crate::models::game::GamePlayer {
    crate::models::game::GamePlayer {
        id,
        created_at,
        updated_at,
        game_id,
        user_id,
        position,
        color,
        has_accepted,
        is_turn,
        is_turn_at,
        place,
        last_turn_at,
        is_eliminated,
        is_read,
        points,
        undo_game_state,
        rating_change,
    }
}

#[cfg(feature = "ssr")]
pub async fn create_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await?;

    Ok(pool)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool))]
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.theme, u.is_admin
        FROM users u
        JOIN user_emails ue ON u.id = ue.user_id
        WHERE ue.email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %id))]
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn find_game_version(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<crate::models::game::GameVersion>> {
    sqlx::query_as!(
        crate::models::game::GameVersion,
        r#"
        SELECT id, created_at, updated_at, game_type_id, name, uri, is_public, is_deprecated
        FROM game_versions
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn find_latest_non_deprecated_game_version(
    pool: &PgPool,
    game_type_id: Uuid,
) -> Result<Option<crate::models::game::GameVersion>> {
    sqlx::query_as!(
        crate::models::game::GameVersion,
        r#"
        SELECT id, created_at, updated_at, game_type_id, name, uri, is_public, is_deprecated
        FROM game_versions
        WHERE game_type_id = $1 AND is_deprecated = false
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        game_type_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

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

/// Rules text only - keeps the (potentially large) rules blob out of every
/// `GameVersion` call site. Plain query (not `query_scalar!`) to avoid `.sqlx`
/// cache churn; there is no local DB to `cargo sqlx prepare` against.
#[cfg(feature = "ssr")]
pub async fn find_game_version_rules(pool: &PgPool, id: Uuid) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT rules FROM game_versions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(rules,)| rules))
}

/// What the rules page needs to fetch strategy live: `(uri, name,
/// interface_version)`. Plain query to avoid `.sqlx` churn (see
/// `find_game_version_rules`).
#[cfg(feature = "ssr")]
pub async fn find_game_version_render_meta(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<(String, String, i32)>> {
    sqlx::query_as("SELECT uri, name, interface_version FROM game_versions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn find_available_game_types(
    pool: &PgPool,
) -> Result<
    Vec<(
        crate::models::game::GameType,
        Vec<crate::models::game::GameVersion>,
    )>,
> {
    let types = sqlx::query_as!(
        crate::models::game::GameType,
        "SELECT id, created_at, updated_at, name, player_counts, weight, blurb FROM game_types ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    let versions = sqlx::query_as!(
        crate::models::game::GameVersion,
        "SELECT id, created_at, updated_at, game_type_id, name, uri, is_public, is_deprecated \
         FROM game_versions WHERE is_public = true AND is_deprecated = false ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    let result = types
        .into_iter()
        .map(|gt| {
            let gv: Vec<_> = versions
                .iter()
                .filter(|v| v.game_type_id == gt.id)
                .cloned()
                .collect();
            (gt, gv)
        })
        .filter(|(_, gv)| !gv.is_empty())
        .collect();

    Ok(result)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %id))]
pub async fn find_game(pool: &PgPool, id: Uuid) -> Result<Option<crate::models::game::Game>> {
    sqlx::query_as!(
        crate::models::game::Game,
        r#"
        SELECT id, created_at, updated_at, game_version_id, is_finished, finished_at, game_state, chat_id, restarted_game_id
        FROM games
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GamePlayerExtended {
    pub game_player: crate::models::game::GamePlayer,
    pub user: Option<crate::models::user::User>,
    pub game_bot: Option<crate::models::game::GameBot>,
    pub game_type_user: crate::models::game::GameTypeUser,
}

#[cfg(feature = "ssr")]
impl GamePlayerExtended {
    pub fn name(&self) -> &str {
        if let Some(u) = &self.user {
            &u.name
        } else if let Some(b) = &self.game_bot {
            &b.name
        } else {
            "Bot"
        }
    }

    /// This game player's `--mk-{slot}` colour slot token (e.g. "green") -
    /// the web layer's colour representation; never resolve this to a
    /// concrete hex value for display, that bakes in one theme.
    pub fn slot(&self) -> &'static str {
        crate::theme::slot_from_color_name(&self.game_player.color)
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameExtended {
    pub game: crate::models::game::Game,
    pub game_type: crate::models::game::GameType,
    pub game_version: crate::models::game::GameVersion,
    pub game_players: Vec<GamePlayerExtended>,
}

#[cfg(feature = "ssr")]
impl GameExtended {
    /// Names-only semantic players for `transform_semantic` - colour stays
    /// symbolic (`SemanticColType::Player(n)`) and is resolved client-side by
    /// the `--mk-player-{n}` vars this game's `player_style_vars` container
    /// sets, not baked into the HTML here.
    pub fn semantic_players(&self) -> Vec<brdgme_markup::SemanticPlayer> {
        self.game_players
            .iter()
            .map(|p| brdgme_markup::SemanticPlayer {
                name: p.name().to_string(),
            })
            .collect()
    }

    /// The `--mk-player-{n}` container style for this game's board/log HTML.
    pub fn player_style(&self) -> String {
        let slots: Vec<&str> = self.game_players.iter().map(|p| p.slot()).collect();
        crate::theme::player_style_vars(&slots)
    }
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %id))]
pub async fn find_game_extended(pool: &PgPool, id: Uuid) -> Result<Option<GameExtended>> {
    let game = find_game(pool, id).await?;
    let game = match game {
        Some(g) => g,
        None => return Ok(None),
    };

    let game_version = find_game_version(pool, game.game_version_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Game version not found"))?;

    let game_type = sqlx::query_as!(
        crate::models::game::GameType,
        "SELECT id, created_at, updated_at, name, player_counts, weight, blurb FROM game_types WHERE id = $1",
        game_version.game_type_id
    )
    .fetch_one(pool)
    .await?;

    let players_raw = sqlx::query!(
        r#"
        SELECT
            gp.id as gp_id, gp.created_at as gp_created_at, gp.updated_at as gp_updated_at,
            gp.game_id as gp_game_id, gp.user_id as gp_user_id, gp.position as gp_position,
            gp.color as gp_color, gp.has_accepted as gp_has_accepted, gp.is_turn as gp_is_turn,
            gp.is_turn_at as gp_is_turn_at, gp.place as gp_place,
            gp.last_turn_at as gp_last_turn_at, gp.is_eliminated as gp_is_eliminated,
            gp.is_read as gp_is_read, gp.points as gp_points,
            gp.undo_game_state as gp_undo_game_state, gp.rating_change as gp_rating_change,
            u.id as "u_id?", u.created_at as "u_created_at?", u.updated_at as "u_updated_at?",
            u.name as "u_name?", u.pref_colors as "u_pref_colors?",
            gtu.id as "gtu_id?", gtu.created_at as "gtu_created_at?", gtu.updated_at as "gtu_updated_at?",
            gtu.game_type_id as "gtu_game_type_id?", gtu.user_id as "gtu_user_id?",
            gtu.last_game_finished_at as "gtu_last_game_finished_at?", gtu.rating as "gtu_rating?",
            gtu.peak_rating as "gtu_peak_rating?",
            gb.id as "gb_id?", gb.game_id as "gb_game_id?", gb.name as "gb_name?",
            gb.bot_name as "gb_bot_name?"
        FROM game_players gp
        LEFT JOIN users u ON gp.user_id = u.id
        LEFT JOIN game_type_users gtu ON gtu.user_id = u.id AND gtu.game_type_id = $2
        LEFT JOIN game_bots gb ON gp.game_bot_id = gb.id
        WHERE gp.game_id = $1
        ORDER BY gp.position
        "#,
        id,
        game_version.game_type_id
    )
    .fetch_all(pool)
    .await?;

    let mut game_players = Vec::new();
    for p in players_raw {
        let gtu = build_game_type_user(
            p.gtu_id,
            p.gtu_created_at,
            p.gtu_updated_at,
            p.gtu_game_type_id,
            p.gtu_user_id,
            p.gtu_last_game_finished_at,
            p.gtu_rating,
            p.gtu_peak_rating,
            p.u_id,
            game_version.game_type_id,
            p.gp_created_at,
        );
        let user = build_user_from_row(
            p.u_id,
            p.u_created_at,
            p.u_updated_at,
            p.u_name,
            p.u_pref_colors,
        )?;
        let game_bot = build_game_bot_from_row(p.gb_id, p.gb_game_id, p.gb_name, p.gb_bot_name)?;

        game_players.push(GamePlayerExtended {
            game_player: build_game_player_from_row(
                p.gp_id,
                p.gp_created_at,
                p.gp_updated_at,
                p.gp_game_id,
                p.gp_user_id,
                p.gp_position,
                p.gp_color,
                p.gp_has_accepted,
                p.gp_is_turn,
                p.gp_is_turn_at,
                p.gp_place,
                p.gp_last_turn_at,
                p.gp_is_eliminated,
                p.gp_is_read,
                p.gp_points,
                p.gp_undo_game_state,
                p.gp_rating_change,
            ),
            user,
            game_bot,
            game_type_user: gtu,
        });
    }

    Ok(Some(GameExtended {
        game,
        game_type,
        game_version,
        game_players,
    }))
}

#[cfg(feature = "ssr")]
#[derive(Debug)]
pub struct BotTurn {
    pub position: i32,
    pub bot_name: String,
}

/// Returns the position/bot_name of every bot player whose turn it
/// currently is. Empty for games with no bots or no bot on turn (including
/// nonexistent games) - that's a normal outcome, not an error.
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id))]
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

#[cfg(feature = "ssr")]
pub async fn find_enabled_bots(pool: &PgPool) -> Result<Vec<String>> {
    sqlx::query_scalar("SELECT name FROM bots WHERE enabled = true ORDER BY display_order")
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("find_enabled_bots: {e}"))
}

#[cfg(feature = "ssr")]
pub async fn is_player_in_game(pool: &PgPool, game_id: Uuid, user_id: Uuid) -> Result<bool> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM game_players WHERE game_id = $1 AND user_id = $2) AS "exists!""#,
        game_id,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Whether `user_id` has admin privileges - `false` if the user row doesn't
/// exist. Written as a plain (non-macro) query, matching `get_user_theme`
/// below.
#[cfg(feature = "ssr")]
pub async fn is_user_admin(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(a,)| a).unwrap_or(false))
}

#[cfg(feature = "ssr")]
pub async fn find_user_id_by_name(pool: &PgPool, name: &str) -> Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT id FROM users WHERE LOWER(name) = LOWER($1)")
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(|e| anyhow::anyhow!("find_user_id_by_name: {e}"))
}

/// Skinny projection for the sidebar: one row per (game, opponent), already
/// sorted my-turn-first then most recently updated. Opponent rows are LEFT
/// JOINed so games with no opponents still appear; exclusion of the
/// requesting user's own seat is by player-row id, not user id.
#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(user_id = %user_id))]
pub async fn find_active_game_summaries(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<crate::game::server_fns::GameSummary>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            g.id as game_id,
            gv.name as version_name,
            gt.name as type_name,
            me.is_turn as my_is_turn,
            me.is_turn_at as my_is_turn_at,
            opp.id as "opp_id?",
            COALESCE(u.name, gb.name, 'Bot') as "opp_name!",
            opp.color as "opp_color?"
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
        LEFT JOIN game_players opp ON opp.game_id = g.id AND opp.id <> me.id
        LEFT JOIN users u ON u.id = opp.user_id
        LEFT JOIN game_bots gb ON gb.id = opp.game_bot_id
        WHERE g.is_finished = false
        ORDER BY me.is_turn DESC, g.updated_at DESC, g.id, opp.position
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut summaries: Vec<crate::game::server_fns::GameSummary> = Vec::new();
    for row in rows {
        if summaries.last().map(|s| s.id) != Some(row.game_id) {
            summaries.push(crate::game::server_fns::GameSummary {
                id: row.game_id,
                name: row.version_name,
                type_name: row.type_name,
                opponents: Vec::new(),
                is_turn: row.my_is_turn,
                is_turn_at: row.my_is_turn_at,
            });
        }
        if row.opp_id.is_some() {
            let color = crate::theme::slot_from_color_name(row.opp_color.as_deref().unwrap_or(""))
                .to_string();
            let summary = summaries.last_mut().ok_or_else(|| {
                anyhow::anyhow!("opponent row for game {} has no summary", row.game_id)
            })?;
            summary
                .opponents
                .push(crate::game::server_fns::OpponentSummary {
                    name: row.opp_name,
                    color,
                });
        }
    }

    Ok(summaries)
}

#[cfg(feature = "ssr")]
pub struct CreateGameOpts<'a> {
    pub game_version_id: Uuid,
    pub whose_turn: &'a [usize],
    pub eliminated: &'a [usize],
    pub placings: &'a [usize],
    pub points: &'a [f32],
    pub creator_id: Uuid,
    pub opponent_ids: &'a [Uuid],
    pub opponent_emails: &'a [String],
    pub bot_slots: &'a [BotSlot],
    pub chat_id: Option<Uuid>,
    pub game_state: &'a str,
    pub all_accepted: bool,
}

#[cfg(feature = "ssr")]
enum PlayerSlotInternal {
    User(User),
    Bot { name: String, bot_name: String },
}

/// D2 username rules (docs/superpowers/specs/2026-07-11-35-user-settings-design.md):
/// `^[a-zA-Z0-9_-]{1,16}$`. Uniqueness is enforced separately by the
/// `users_name_lower_key` index (migration 009). Pure and ungated so the
/// client-side form and server fns share one definition.
pub fn validate_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 16
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Generates a default username: a 2-word petname (e.g. "scary-walrus"),
/// regenerated until it satisfies D2 (length; the crate's charset is already
/// safe) and is case-insensitively unused. Long words make regeneration
/// expected and cheap. The uuid fallback is unreachable in practice but keeps
/// this total. Takes a connection so it can run inside callers' transactions.
#[cfg(feature = "ssr")]
pub async fn generate_unique_username(conn: &mut sqlx::PgConnection) -> Result<String> {
    for _ in 0..100 {
        let Some(candidate) = petname::petname(2, "-") else {
            continue;
        };
        if !validate_username(&candidate) {
            continue;
        }
        let taken: Option<(bool,)> =
            sqlx::query_as("SELECT true FROM users WHERE lower(name) = lower($1)")
                .bind(&candidate)
                .fetch_optional(&mut *conn)
                .await?;
        if taken.is_none() {
            return Ok(candidate);
        }
    }
    Ok(format!(
        "user-{}",
        &Uuid::new_v4().simple().to_string()[..11]
    ))
}

/// Normalizes legacy stored preference names onto the current palette, so
/// prefs saved before the 2026-07 palette change still match. See
/// `theme::slot_from_color_name` for the same mapping applied to stored
/// `game_players.color`/`users.pref_colors` values.
#[cfg(feature = "ssr")]
pub(crate) fn normalize_pref_color(name: &str) -> String {
    if name.eq_ignore_ascii_case("Amber") {
        return "Orange".to_string();
    }
    if name.eq_ignore_ascii_case("BlueGrey") {
        return "Cyan".to_string();
    }
    crate::theme::PLAYER_COLOR_NAMES
        .iter()
        .find(|c| c.eq_ignore_ascii_case(name))
        .map(|c| c.to_string())
        .unwrap_or_else(|| name.to_string())
}

#[cfg(feature = "ssr")]
type LocPref = (usize, Vec<String>);

/// Drops each remaining pref's highest-ranked entry, returning `None` once no
/// pref has anything left (signals the caller to stop looping).
#[cfg(feature = "ssr")]
fn remove_highest_prefs(prefs: &[LocPref]) -> Option<Vec<LocPref>> {
    let mut some_remain = false;
    let new_prefs = prefs
        .iter()
        .map(|(pos, pref)| {
            let new_pref = if pref.is_empty() {
                vec![]
            } else {
                let p = pref[1..].to_owned();
                if !some_remain && !p.is_empty() {
                    some_remain = true;
                }
                p
            };
            (*pos, new_pref)
        })
        .collect::<Vec<LocPref>>();
    if some_remain { Some(new_prefs) } else { None }
}

/// Chooses colors for players based on preferences. Ported from the old
/// `api::db::color::choose` (see `git show ba975b5^:rust/api/src/db/color.rs`),
/// but operating on plain strings against a caller-supplied palette rather
/// than a fixed `Color` enum.
///
/// First tries to assign everyone's highest still-available preference, then
/// everyone's next, and so on, until all players have a color or the palette
/// runs out. When multiple players want the same color at the same rank, the
/// preference order is shuffled up front so the winner is randomly tiebroken.
/// Players with no remaining matching prefs get whatever's left of the
/// palette, in palette order. Legacy pref names ("Amber", "BlueGrey") are
/// normalized onto their current equivalents before matching. If there are
/// more players than the palette holds, players beyond the palette length
/// repeat the same assignment recursively (mirroring the old algorithm), and
/// exhausting the palette entirely falls back to "Pink".
#[cfg(feature = "ssr")]
fn choose_colors(prefs: &[Vec<String>], palette: &[&str]) -> Vec<String> {
    if palette.is_empty() || prefs.is_empty() {
        return prefs.iter().map(|_| "Pink".to_string()).collect();
    }

    use rand::seq::SliceRandom;
    use std::collections::HashMap;

    let sub_len = prefs.len().min(palette.len());
    let (sub_prefs, tail_prefs) = prefs.split_at(sub_len);

    let mut rng = rand::rng();
    let mut remaining: Vec<String> = palette.iter().map(|s| s.to_string()).collect();
    let mut assigned: HashMap<usize, String> = HashMap::new();

    let mut rem_prefs: Vec<LocPref> = sub_prefs
        .iter()
        .enumerate()
        .map(|(pos, pref)| {
            let normalized = pref
                .iter()
                .map(|s| normalize_pref_color(s))
                .filter(|s| palette.contains(&s.as_str()))
                .collect::<Vec<String>>();
            (pos, normalized)
        })
        .collect();
    rem_prefs.shuffle(&mut rng);

    'outer: loop {
        for (pos, pref) in rem_prefs.clone() {
            if assigned.contains_key(&pos) || pref.is_empty() {
                continue;
            }
            let want_color = &pref[0];
            if let Some(idx) = remaining.iter().position(|c| c == want_color) {
                assigned.insert(pos, remaining.remove(idx));
            }
            if remaining.is_empty() {
                break 'outer;
            }
        }
        if let Some(new_prefs) = remove_highest_prefs(&rem_prefs) {
            rem_prefs = new_prefs;
        } else {
            break 'outer;
        }
    }

    let mut left = remaining.into_iter();
    let mut res = Vec::with_capacity(sub_prefs.len());
    for pos in 0..sub_prefs.len() {
        res.push(
            assigned
                .remove(&pos)
                .unwrap_or_else(|| left.next().unwrap_or_else(|| "Pink".to_string())),
        );
    }

    if !tail_prefs.is_empty() {
        res.extend(choose_colors(tail_prefs, palette));
    }

    res
}

#[cfg(feature = "ssr")]
pub async fn create_game_with_users(
    pool: &PgPool,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    let mut tx = pool.begin().await?;
    let game = create_game_with_users_tx(&mut tx, opts).await?;
    tx.commit().await?;
    Ok(game)
}

/// Creates a game and its players within an existing transaction, so callers
/// can commit them atomically alongside other writes (e.g. the restarted-game
/// linkage in `restart_game`).
#[cfg(feature = "ssr")]
#[tracing::instrument(skip_all)]
pub async fn create_game_with_users_tx(
    tx: &mut sqlx::PgConnection,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    // 1. Find or create users; collect all slots (users + bots)
    let mut slots: Vec<PlayerSlotInternal> = Vec::new();

    // Creator
    let creator = sqlx::query_as!(
        crate::models::user::User,
        "SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin FROM users WHERE id = $1",
        opts.creator_id
    )
    .fetch_one(&mut *tx)
    .await?;
    slots.push(PlayerSlotInternal::User(creator));

    // Opponent IDs
    for &id in opts.opponent_ids {
        let opponent = sqlx::query_as!(
            crate::models::user::User,
            "SELECT id, created_at, updated_at, name, pref_colors, theme, is_admin FROM users WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        slots.push(PlayerSlotInternal::User(opponent));
    }

    // Opponent Emails
    for email in opts.opponent_emails {
        let user = if let Some(u) = sqlx::query_as!(
            crate::models::user::User,
            r#"SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.theme, u.is_admin
               FROM users u JOIN user_emails ue ON u.id = ue.user_id WHERE ue.email = $1"#,
            email
        )
        .fetch_optional(&mut *tx)
        .await?
        {
            u
        } else {
            // Create new user for email
            let new_user_id = Uuid::new_v4();
            let username = generate_unique_username(&mut *tx).await?;

            let u = sqlx::query_as!(
                crate::models::user::User,
                "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
                new_user_id,
                username,
                &Vec::<String>::new()
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO user_emails (user_id, email, is_primary, verified_at)
                 VALUES ($1, $2, true, NOW())",
            )
            .bind(new_user_id)
            .bind(email)
            .execute(&mut *tx)
            .await?;

            u
        };
        slots.push(PlayerSlotInternal::User(user));
    }

    // Bot slots
    for bot in opts.bot_slots {
        slots.push(PlayerSlotInternal::Bot {
            name: bot.name.clone(),
            bot_name: bot.bot_name.clone(),
        });
    }

    // 2. Randomize player order
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        slots.shuffle(&mut rng);
    }

    // 3. Assign colors, honouring each user's preferred colors where possible.
    let palette = crate::theme::PLAYER_COLOR_NAMES;
    let prefs: Vec<Vec<String>> = slots
        .iter()
        .map(|slot| match slot {
            PlayerSlotInternal::User(user) => user.pref_colors.clone(),
            PlayerSlotInternal::Bot { .. } => vec![],
        })
        .collect();
    let colors = choose_colors(&prefs, &palette);

    // 4. Create Game
    let is_finished = !opts.placings.is_empty();
    let game = sqlx::query_as!(
        crate::models::game::Game,
        r#"
        INSERT INTO games (game_version_id, is_finished, game_state, chat_id)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        opts.game_version_id,
        is_finished,
        opts.game_state,
        opts.chat_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // 5. Create Players
    let game_type_id = sqlx::query_scalar!(
        "SELECT game_type_id FROM game_versions WHERE id = $1",
        opts.game_version_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("Game version not found"))?;

    for (pos, slot) in slots.iter().enumerate() {
        let color = colors
            .get(pos)
            .cloned()
            .unwrap_or_else(|| "Pink".to_string());
        let is_turn = opts.whose_turn.contains(&pos);
        let is_eliminated = opts.eliminated.contains(&pos);
        let place = opts.placings.get(pos).map(|&p| p as i32);

        match slot {
            PlayerSlotInternal::User(user) => {
                sqlx::query!(
                    r#"
                    INSERT INTO game_players (game_id, user_id, position, color, has_accepted, is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place)
                    VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW(), $7, false, $8)
                    "#,
                    game.id,
                    user.id,
                    pos as i32,
                    color,
                    opts.all_accepted || user.id == opts.creator_id,
                    is_turn,
                    is_eliminated,
                    place
                )
                .execute(&mut *tx)
                .await?;

                sqlx::query!(
                    r#"
                    INSERT INTO game_type_users (game_type_id, user_id)
                    VALUES ($1, $2)
                    ON CONFLICT DO NOTHING
                    "#,
                    game_type_id,
                    user.id
                )
                .execute(&mut *tx)
                .await?;
            }
            PlayerSlotInternal::Bot { name, bot_name } => {
                let bot_id = sqlx::query_scalar!(
                    "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id",
                    game.id,
                    name,
                    bot_name
                )
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query!(
                    r#"
                    INSERT INTO game_players (game_id, game_bot_id, position, color, has_accepted, is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place)
                    VALUES ($1, $2, $3, $4, true, $5, NOW(), NOW(), $6, true, $7)
                    "#,
                    game.id,
                    bot_id,
                    pos as i32,
                    color,
                    is_turn,
                    is_eliminated,
                    place
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    Ok(game)
}

/// Inserts game logs within an existing transaction, so callers can commit
/// them atomically alongside other writes (e.g. the game state update in
/// `update_game_command_success`).
#[cfg(feature = "ssr")]
pub async fn insert_game_logs_tx(
    tx: &mut sqlx::PgConnection,
    game_id: Uuid,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    // Get player IDs by position
    let players = sqlx::query!(
        "SELECT id, position FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut pos_to_id = std::collections::HashMap::new();
    for p in players {
        pos_to_id.insert(p.position as usize, p.id);
    }

    for log in logs {
        let log_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO game_logs (id, game_id, body, is_public, logged_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            log_id,
            game_id,
            log.content,
            log.public,
            log.at
        )
        .execute(&mut *tx)
        .await?;

        for &pos in &log.to {
            if let Some(&player_id) = pos_to_id.get(&pos) {
                sqlx::query!(
                    "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
                    log_id,
                    player_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn create_game_logs(
    pool: &PgPool,
    game_id: Uuid,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    insert_game_logs_tx(&mut tx, game_id, logs).await?;
    tx.commit().await?;
    Ok(())
}

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

/// #34 admin force delete (spec D3): hard-deletes a game and all dependent
/// rows in one transaction. Any game referencing the deleted one via
/// `restarted_game_id` has that link nulled (making it restartable again), and
/// any proposal referencing it via `started_game_id`/`restarted_game_id` has
/// that link nulled (preserving the proposal history). Ratings are deliberately
/// NOT rewound. Returns false if the game did not exist.
#[cfg(feature = "ssr")]
pub async fn delete_game(pool: &PgPool, game_id: Uuid) -> Result<bool> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET restarted_game_id = NULL, updated_at = NOW() WHERE restarted_game_id = $1",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    // game_proposals (migration 015) FK-reference games via started_game_id and
    // restarted_game_id; null both or the game delete violates those FKs.
    sqlx::query(
        "UPDATE game_proposals SET started_game_id = NULL, updated_at = NOW() WHERE started_game_id = $1",
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_proposals SET restarted_game_id = NULL, updated_at = NOW() WHERE restarted_game_id = $1",
    )
    .bind(game_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM game_log_targets WHERE game_log_id IN (SELECT id FROM game_logs WHERE game_id = $1)",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM game_logs WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    // game_players before game_bots: game_players.game_bot_id FK.
    sqlx::query!("DELETE FROM game_players WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM game_bots WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query!("DELETE FROM games WHERE id = $1", game_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip(pool), fields(game_id = %game_id, user_id = %user_id))]
pub async fn mark_game_read(pool: &PgPool, game_id: Uuid, user_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE game_players SET is_read = true, updated_at = NOW() WHERE game_id = $1 AND user_id = $2",
        game_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
#[tracing::instrument(skip_all, fields(game_id = %game_id))]
pub async fn undo_game(
    pool: &PgPool,
    game_id: Uuid,
    undo_state: &str,
    player_position: usize,
    status: &StatusUpdate,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = NULL, updated_at = NOW() WHERE id = $3",
        undo_state,
        status.is_finished,
        game_id
    )
    .execute(&mut *tx)
    .await?;

    let players = sqlx::query!(
        "SELECT id, position FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    for p in players {
        let pos = p.position as usize;
        let is_turn = status.whose_turn.contains(&pos);
        let is_eliminated = status.eliminated.contains(&pos);
        let place: Option<i32> = status.placings.get(pos).map(|&pl| pl as i32);

        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, is_eliminated = $2, place = $3, undo_game_state = NULL,
                   turn_reminder_sent_at = NULL, updated_at = NOW()
               WHERE id = $4"#,
        )
        .bind(is_turn)
        .bind(is_eliminated)
        .bind(place)
        .bind(p.id)
        .execute(&mut *tx)
        .await?;
    }

    let undo_log_body = format!("{{{{player {}}}}} used an undo", player_position);
    sqlx::query!(
        "INSERT INTO game_logs (game_id, body, is_public, logged_at) VALUES ($1, $2, true, NOW())",
        game_id,
        undo_log_body,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_all_game_logs(
    pool: &PgPool,
    game_id: Uuid,
) -> Result<Vec<crate::models::game::GameLog>> {
    sqlx::query_as!(
        crate::models::game::GameLog,
        r#"
        SELECT id, created_at, updated_at, game_id, body, is_public, logged_at
        FROM game_logs
        WHERE game_id = $1
        ORDER BY logged_at ASC
        "#,
        game_id,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn get_game_logs(
    pool: &PgPool,
    game_id: Uuid,
    game_player_id: Uuid,
) -> Result<Vec<crate::models::game::GameLog>> {
    sqlx::query_as!(
        crate::models::game::GameLog,
        r#"
        SELECT id, created_at, updated_at, game_id, body, is_public, logged_at
        FROM game_logs
        WHERE game_id = $1
          AND (is_public = true OR id IN (
              SELECT game_log_id FROM game_log_targets WHERE game_player_id = $2
          ))
        ORDER BY logged_at ASC
        "#,
        game_id,
        game_player_id,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

const ELO_K: f32 = 32.0;

#[cfg(feature = "ssr")]
fn elo_transformed_rating(rating: i32) -> f32 {
    10f32.powf(rating as f32 / 400.0)
}

#[cfg(feature = "ssr")]
fn elo_expected_score(a_rating: i32, b_rating: i32) -> f32 {
    let a_trans = elo_transformed_rating(a_rating);
    let b_trans = elo_transformed_rating(b_rating);
    a_trans / (a_trans + b_trans)
}

#[cfg(feature = "ssr")]
fn elo_rating_change(a_rating: i32, b_rating: i32, a_score: f32) -> i32 {
    let a_expected = elo_expected_score(a_rating, b_rating);
    (ELO_K * (a_score - a_expected)).round() as i32
}

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
        // Idempotency guard: this game has already been rated.
        return Ok(());
    }
    if players.iter().all(|p| p.place.is_none()) {
        return Ok(());
    }

    let game_type_id = sqlx::query_scalar!(
        r#"
        SELECT gv.game_type_id
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        WHERE g.id = $1
        "#,
        game_id
    )
    .fetch_one(&mut *tx)
    .await?;

    struct RatedPlayer {
        position: i32,
        user_id: Uuid,
        rating: i32,
    }

    let mut rated_players = Vec::with_capacity(players.len());
    for p in &players {
        if p.game_bot_id.is_some() {
            continue;
        }
        let user_id = p.user_id.ok_or_else(|| {
            anyhow::anyhow!("game_player {}: user_id missing for human player", p.id)
        })?;

        sqlx::query!(
            "INSERT INTO game_type_users (game_type_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            game_type_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let rating = sqlx::query_scalar!(
            "SELECT rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            game_type_id,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        rated_players.push(RatedPlayer {
            position: p.position,
            user_id,
            rating,
        });
    }

    let rating_befores: std::collections::HashMap<i32, i32> = rated_players
        .iter()
        .map(|p| (p.position, p.rating))
        .collect();

    if rated_players.len() < 2 {
        return Ok(());
    }

    let places: std::collections::HashMap<i32, i32> = players
        .iter()
        .map(|p| (p.position, p.place.unwrap_or(i32::MAX)))
        .collect();

    let mut rating_changes: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    for (a_index, a) in rated_players
        .iter()
        .take(rated_players.len().saturating_sub(1))
        .enumerate()
    {
        for b in rated_players.iter().skip(a_index + 1) {
            let a_place = places.get(&a.position).copied().unwrap_or(i32::MAX);
            let b_place = places.get(&b.position).copied().unwrap_or(i32::MAX);
            let a_score: f32 = match a_place.cmp(&b_place) {
                std::cmp::Ordering::Less => 1.0,
                std::cmp::Ordering::Equal => 0.5,
                std::cmp::Ordering::Greater => 0.0,
            };
            let change = elo_rating_change(a.rating, b.rating, a_score);
            *rating_changes.entry(a.position).or_insert(0) += change;
            *rating_changes.entry(b.position).or_insert(0) -= change;
        }
    }

    for p in &rated_players {
        let change = rating_changes.get(&p.position).copied().unwrap_or(0);
        if change == 0 {
            continue;
        }
        sqlx::query!(
            r#"
            UPDATE game_type_users
            SET rating = rating + $1, peak_rating = GREATEST(peak_rating, rating + $1), updated_at = NOW()
            WHERE game_type_id = $2 AND user_id = $3
            "#,
            change,
            game_type_id,
            p.user_id
        )
        .execute(&mut *tx)
        .await?;
    }

    for p in &players {
        let change = rating_changes.get(&p.position).copied().unwrap_or(0);
        if change == 0 {
            continue;
        }
        let rating_before = rating_befores.get(&p.position).copied();
        sqlx::query("UPDATE game_players SET rating_change = $1, rating_before = $2 WHERE id = $3")
            .bind(change)
            .bind(rating_before)
            .bind(p.id)
            .execute(&mut *tx)
            .await?;
    }

    Ok(())
}

/// Distinguishable error so callers (the `bot.command` consumer) can tell a
/// stale-state conflict apart from other failures and react by re-publishing
/// `bot.turn` rather than giving up.
#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
#[error("Game was updated by another action, please retry")]
pub struct StaleStateConflict;

#[cfg(feature = "ssr")]
// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(game_id = %game_id))]
pub async fn update_game_command_success(
    pool: &PgPool,
    game_id: Uuid,
    played_player_id: Uuid,
    prev_game_state: &str,
    new_game_state: &str,
    can_undo: bool,
    status: &StatusUpdate,
    points: &[f32],
    expected_updated_at: time::PrimitiveDateTime,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let now = {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    };
    let finished_at: Option<time::PrimitiveDateTime> =
        if status.is_finished { Some(now) } else { None };

    let mut tx = pool.begin().await?;

    let update_result = sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = COALESCE($3, finished_at), updated_at = NOW() WHERE id = $4 AND updated_at = $5",
        new_game_state,
        status.is_finished,
        finished_at,
        game_id,
        expected_updated_at
    )
    .execute(&mut *tx)
    .await?;

    if update_result.rows_affected() == 0 {
        return Err(StaleStateConflict.into());
    }

    // Plain (non-macro) query, not `query!`; see the `get_user_theme` doc
    // comment above for the same convention.
    let players: Vec<(Uuid, i32, time::PrimitiveDateTime, time::PrimitiveDateTime, Option<String>)> =
        sqlx::query_as(
            "SELECT id, position, is_turn_at, last_turn_at, undo_game_state FROM game_players WHERE game_id = $1",
        )
        .bind(game_id)
        .fetch_all(&mut *tx)
        .await?;

    for (p_id, p_position, p_is_turn_at, p_last_turn_at, p_undo_game_state) in players {
        let pos = p_position as usize;
        let is_turn = status.whose_turn.contains(&pos);
        let place = status.placings.get(pos).map(|&pl| pl as i32);
        let is_eliminated = status.eliminated.contains(&pos);
        let player_points = points.get(pos).copied();
        let is_turn_at = if is_turn { now } else { p_is_turn_at };
        let is_played = p_id == played_player_id;
        let last_turn_at = p_last_turn_at;
        let undo_game_state: Option<&str> = if is_played && can_undo {
            p_undo_game_state.as_deref().or(Some(prev_game_state))
        } else {
            None
        };

        sqlx::query(
            r#"UPDATE game_players
               SET is_turn = $1, place = $2, is_eliminated = $3, points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   turn_reminder_sent_at = NULL,
                   updated_at = NOW()
               WHERE id = $8"#,
        )
        .bind(is_turn)
        .bind(place)
        .bind(is_eliminated)
        .bind(player_points)
        .bind(undo_game_state)
        .bind(last_turn_at)
        .bind(is_turn_at)
        .bind(p_id)
        .execute(&mut *tx)
        .await?;
    }

    if status.is_finished && !status.placings.is_empty() {
        apply_rating_changes(&mut tx, game_id).await?;
    }

    insert_game_logs_tx(&mut tx, game_id, logs).await?;

    tx.commit().await?;
    Ok(())
}

/// Written as a plain (non-macro) query rather than `query_scalar!`. See
/// `docs/DEV.md` for the `cargo sqlx prepare` workflow if this is ever
/// converted to a macro query.
#[cfg(feature = "ssr")]
pub async fn get_user_theme(pool: &PgPool, user_id: Uuid) -> Result<Option<String>> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT theme FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|(theme,)| theme))
}

#[cfg(feature = "ssr")]
pub async fn set_user_theme(pool: &PgPool, user_id: Uuid, theme: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE users SET theme = $1, updated_at = NOW() WHERE id = $2")
        .bind(theme)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- #30 friends (spec docs/superpowers/specs/2026-07-08-30-friends-design.md) ---
// Plain (non-macro) queries throughout, matching get_user_theme above.

#[cfg(feature = "ssr")]
#[derive(Debug, sqlx::FromRow)]
struct FriendRow {
    id: Uuid,
    source_user_id: Uuid,
    has_accepted: Option<bool>,
}

/// D1 lifecycle. Creates a pending request, treating a reverse pending row
/// as mutual intent (auto-accept), a reverse declined row as the decliner
/// changing their mind (flip to accepted), and everything else as a silent
/// no-op. If the target has blocked the source, this is a silent no-op too
/// (D7): the requester must not be able to distinguish any of these.
#[cfg(feature = "ssr")]
pub async fn send_friend_request(pool: &PgPool, source: Uuid, target: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    let target_blocked_source: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(target)
    .bind(source)
    .fetch_one(&mut *tx)
    .await?;
    if target_blocked_source {
        return Ok(()); // tx dropped -> rollback; nothing written
    }
    let row: Option<FriendRow> = sqlx::query_as(
        "SELECT id, source_user_id, has_accepted FROM friends
         WHERE (source_user_id = $1 AND target_user_id = $2)
            OR (source_user_id = $2 AND target_user_id = $1)",
    )
    .bind(source)
    .bind(target)
    .fetch_optional(&mut *tx)
    .await?;
    match row {
        None => {
            sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
                .bind(source)
                .bind(target)
                .execute(&mut *tx)
                .await?;
        }
        // I already have an outgoing row (pending, declined, or accepted):
        // silent no-op in every case.
        Some(r) if r.source_user_id == source => {}
        // Reverse row: they asked me (pending -> mutual intent) or they asked
        // me and I declined (my own request now = both sides opted in).
        Some(r) => {
            if r.has_accepted != Some(true) {
                sqlx::query(
                    "UPDATE friends SET has_accepted = TRUE,
                     updated_at = timezone('utc', now()) WHERE id = $1",
                )
                .bind(r.id)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Returns false when no pending request with this id targets `responder`
/// (already responded, wrong user, or unknown id).
#[cfg(feature = "ssr")]
pub async fn respond_to_friend_request(
    pool: &PgPool,
    request_id: Uuid,
    responder: Uuid,
    accept: bool,
) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE friends SET has_accepted = $1, updated_at = timezone('utc', now())
         WHERE id = $2 AND target_user_id = $3 AND has_accepted IS NULL",
    )
    .bind(accept)
    .bind(request_id)
    .bind(responder)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() == 1)
}

/// The requester behind a pending incoming request - used by the
/// decline-and-block path (D7), which needs the source id to block.
#[cfg(feature = "ssr")]
pub async fn get_pending_request_source(
    pool: &PgPool,
    request_id: Uuid,
    responder: Uuid,
) -> Result<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT source_user_id FROM friends
         WHERE id = $1 AND target_user_id = $2 AND has_accepted IS NULL",
    )
    .bind(request_id)
    .bind(responder)
    .fetch_optional(pool)
    .await?)
}

/// Deletes only ACCEPTED rows: a requester must not be able to delete the
/// declined row that shields the decliner from re-request spam.
#[cfg(feature = "ssr")]
pub async fn unfriend(pool: &PgPool, a: Uuid, b: Uuid) -> Result<()> {
    sqlx::query(
        "DELETE FROM friends WHERE has_accepted = TRUE
         AND ((source_user_id = $1 AND target_user_id = $2)
           OR (source_user_id = $2 AND target_user_id = $1))",
    )
    .bind(a)
    .bind(b)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn are_friends_conn(conn: &mut sqlx::PgConnection, a: Uuid, b: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friends WHERE has_accepted = TRUE
         AND ((source_user_id = $1 AND target_user_id = $2)
           OR (source_user_id = $2 AND target_user_id = $1)))",
    )
    .bind(a)
    .bind(b)
    .fetch_one(conn)
    .await?)
}

/// True when the "Add friend" affordance should be hidden: already friends
/// (either direction) or viewer already has an outgoing row (pending/declined).
#[cfg(feature = "ssr")]
pub async fn should_hide_add_friend(pool: &PgPool, viewer: Uuid, target: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM friends
           WHERE (source_user_id = $1 AND target_user_id = $2)
              OR (has_accepted = TRUE AND source_user_id = $2 AND target_user_id = $1))",
    )
    .bind(viewer)
    .bind(target)
    .fetch_one(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn list_friends(pool: &PgPool, user_id: Uuid) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = CASE WHEN f.source_user_id = $1
                                     THEN f.target_user_id ELSE f.source_user_id END
         WHERE f.has_accepted = TRUE
           AND (f.source_user_id = $1 OR f.target_user_id = $1)
         ORDER BY lower(u.name)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// (request_id, requester_user_id, requester_name), oldest first.
#[cfg(feature = "ssr")]
pub async fn list_incoming_friend_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT f.id, u.id, u.name FROM friends f
         JOIN users u ON u.id = f.source_user_id
         WHERE f.target_user_id = $1 AND f.has_accepted IS NULL
         ORDER BY f.created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// Outgoing requests shown as "pending". DELIBERATELY includes declined
/// rows (has_accepted = FALSE): the requester must not be able to
/// distinguish pending from declined (D1 silent shield).
#[cfg(feature = "ssr")]
pub async fn list_outgoing_friend_requests(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = f.target_user_id
         WHERE f.source_user_id = $1
           AND (f.has_accepted IS NULL OR f.has_accepted = FALSE)
         ORDER BY f.created_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// D7. Idempotent. Severs any friends row for the pair (accepted, pending,
/// or declined, either direction) atomically with the block insert.
#[cfg(feature = "ssr")]
pub async fn block_user(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO blocks (blocker_user_id, blocked_user_id)
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(blocker)
    .bind(blocked)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM friends
         WHERE (source_user_id = $1 AND target_user_id = $2)
            OR (source_user_id = $2 AND target_user_id = $1)",
    )
    .bind(blocker)
    .bind(blocked)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Deletes the block only. Does not restore any friendship; a fresh friend
/// request afterwards is allowed (D7).
#[cfg(feature = "ssr")]
pub async fn unblock_user(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2")
        .bind(blocker)
        .bind(blocked)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn has_block(pool: &PgPool, blocker: Uuid, blocked: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(blocker)
    .bind(blocked)
    .fetch_one(pool)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn has_block_conn(
    conn: &mut sqlx::PgConnection,
    blocker: Uuid,
    blocked: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2)",
    )
    .bind(blocker)
    .bind(blocked)
    .fetch_one(conn)
    .await?)
}

#[cfg(feature = "ssr")]
pub async fn list_blocked(pool: &PgPool, blocker: Uuid) -> Result<Vec<(Uuid, String)>> {
    Ok(sqlx::query_as(
        "SELECT u.id, u.name FROM blocks b
         JOIN users u ON u.id = b.blocked_user_id
         WHERE b.blocker_user_id = $1
         ORDER BY b.created_at DESC",
    )
    .bind(blocker)
    .fetch_all(pool)
    .await?)
}

/// Plain query, matching get_user_theme - invite_policy is deliberately NOT
/// a field on models::user::User.
#[cfg(feature = "ssr")]
pub async fn get_invite_policy(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT invite_policy FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

#[cfg(feature = "ssr")]
pub async fn set_invite_policy(pool: &PgPool, user_id: Uuid, policy: &str) -> Result<()> {
    sqlx::query("UPDATE users SET invite_policy = $1, updated_at = NOW() WHERE id = $2")
        .bind(policy)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// D4 + D7 enforcement choke point. Call after the roster is known but
/// before players are attached: create_new_game and restart_game today,
/// #24's create_proposal and any future matchmaking tomorrow.
///
/// Emails resolving to no account pass (the account is created at game
/// creation with default 'open' and can have no blocks). Block-by-target
/// uses wording identical to policy 'none' so a blocked creator cannot
/// distinguish the two (D7 detectability).
#[cfg(feature = "ssr")]
pub async fn check_invite_policy_tx(
    tx: &mut sqlx::PgConnection,
    creator_id: Uuid,
    opponent_ids: &[Uuid],
    opponent_emails: &[String],
) -> Result<Vec<String>> {
    let mut targets: Vec<Uuid> = opponent_ids.to_vec();
    for email in opponent_emails {
        let existing: Option<Uuid> =
            sqlx::query_scalar("SELECT user_id FROM user_emails WHERE email = $1")
                .bind(email)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(id) = existing {
            targets.push(id);
        }
    }
    targets.sort();
    targets.dedup();

    let mut violations = Vec::new();
    for target in targets {
        if target == creator_id {
            continue;
        }
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT name, invite_policy FROM users WHERE id = $1")
                .bind(target)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((name, policy)) = row else {
            violations.push("Player not found".to_string());
            continue;
        };
        if has_block_conn(&mut *tx, target, creator_id).await? {
            violations.push(format!("{name} is not accepting game invitations"));
            continue;
        }
        if has_block_conn(&mut *tx, creator_id, target).await? {
            violations.push(format!("You have blocked {name}"));
            continue;
        }
        if policy == "none" {
            violations.push(format!("{name} is not accepting game invitations"));
        } else if policy == "friends" && !are_friends_conn(&mut *tx, creator_id, target).await? {
            violations.push(format!("{name} only accepts games from friends"));
        } // 'open' passes
    }
    Ok(violations)
}

/// Exact-name lookup, case-insensitive (users_name_lower_key, migration 009).
#[cfg(feature = "ssr")]
pub async fn get_user_by_name(pool: &PgPool, name: &str) -> Result<Option<(Uuid, String)>> {
    Ok(
        sqlx::query_as("SELECT id, name FROM users WHERE lower(name) = lower($1)")
            .bind(name)
            .fetch_optional(pool)
            .await?,
    )
}

/// Display-name substring search for the new game page typeahead (#44):
/// case-insensitive, excludes the searching user, capped at 10. Users who
/// block the searcher or are blocked by the searcher (either direction) are
/// also excluded. Queries under 2 trimmed characters return nothing without
/// touching the DB.
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
        "SELECT u.id, u.name FROM users u
         WHERE u.id <> $1 AND u.name ILIKE $2 ESCAPE '\\'
           AND NOT EXISTS (SELECT 1 FROM blocks b
                           WHERE (b.blocker_user_id = $1 AND b.blocked_user_id = u.id)
                              OR (b.blocker_user_id = u.id AND b.blocked_user_id = $1))
         ORDER BY lower(u.name)
         LIMIT 10",
    )
    .bind(user_id)
    .bind(format!("%{escaped}%"))
    .fetch_all(pool)
    .await?)
}

/// D6: friends tier (most recently played with first - resolved decision
/// 2026-07-18 - then alphabetical), then distinct human co-players from the
/// caller's last 20 games. Excludes self and any block in either direction.
#[cfg(feature = "ssr")]
pub async fn opponent_suggestions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String, bool)>> {
    let friends: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.name FROM friends f
         JOIN users u ON u.id = CASE WHEN f.source_user_id = $1
                                     THEN f.target_user_id ELSE f.source_user_id END
         WHERE f.has_accepted = TRUE
           AND (f.source_user_id = $1 OR f.target_user_id = $1)
         ORDER BY (SELECT max(g.updated_at) FROM games g
                   JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
                   JOIN game_players them ON them.game_id = g.id AND them.user_id = u.id)
                  DESC NULLS LAST,
                  lower(u.name)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let recent: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT u.id, u.name FROM
           (SELECT g.id AS game_id, g.updated_at FROM games g
            JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
            ORDER BY g.updated_at DESC LIMIT 20) recent
         JOIN game_players op ON op.game_id = recent.game_id AND op.user_id <> $1
         JOIN users u ON u.id = op.user_id
         WHERE NOT EXISTS (SELECT 1 FROM blocks b
                           WHERE (b.blocker_user_id = $1 AND b.blocked_user_id = u.id)
                              OR (b.blocker_user_id = u.id AND b.blocked_user_id = $1))
         GROUP BY u.id, u.name
         ORDER BY max(recent.updated_at) DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<(Uuid, String, bool)> = friends
        .into_iter()
        .map(|(id, name)| (id, name, true))
        .collect();
    for (id, name) in recent {
        if !out.iter().any(|(fid, _, _)| *fid == id) {
            out.push((id, name, false));
        }
    }
    Ok(out)
}

/// D5: in-progress games containing >= 1 accepted friend where the caller
/// is NOT a player (spectating links). Human player names only - bots live
/// in game_bots and are omitted from this lightweight feed.
#[cfg(feature = "ssr")]
pub async fn friends_active_games(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<(Uuid, String, Vec<String>)>> {
    Ok(sqlx::query_as(
        "SELECT g.id, gt.name, array_agg(u.name ORDER BY gp.position)
         FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         JOIN game_types gt ON gt.id = gv.game_type_id
         JOIN game_players gp ON gp.game_id = g.id
         JOIN users u ON u.id = gp.user_id
         WHERE g.is_finished = FALSE
           AND NOT EXISTS (SELECT 1 FROM game_players me
                           WHERE me.game_id = g.id AND me.user_id = $1)
           AND EXISTS (
               SELECT 1 FROM game_players fgp
               JOIN friends f ON f.has_accepted = TRUE
                    AND ((f.source_user_id = $1 AND f.target_user_id = fgp.user_id)
                      OR (f.target_user_id = $1 AND f.source_user_id = fgp.user_id))
               WHERE fgp.game_id = g.id)
         GROUP BY g.id, gt.name, g.updated_at
         ORDER BY g.updated_at DESC
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// D5: last `limit` finished games involving >= 1 friend (the caller's own
/// finished games qualify too). Names ordered by place (NULLS LAST), places
/// COALESCEd to 0 for "not placed".
#[cfg(feature = "ssr")]
pub async fn friends_recent_results(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<
    Vec<(
        Uuid,
        String,
        Option<time::PrimitiveDateTime>,
        Vec<String>,
        Vec<i32>,
    )>,
> {
    Ok(sqlx::query_as(
        "SELECT g.id, gt.name, g.finished_at,
                array_agg(u.name ORDER BY gp.place ASC NULLS LAST, gp.position),
                array_agg(COALESCE(gp.place, 0) ORDER BY gp.place ASC NULLS LAST, gp.position)
         FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         JOIN game_types gt ON gt.id = gv.game_type_id
         JOIN game_players gp ON gp.game_id = g.id
         JOIN users u ON u.id = gp.user_id
         WHERE g.is_finished = TRUE
           AND EXISTS (
               SELECT 1 FROM game_players fgp
               JOIN friends f ON f.has_accepted = TRUE
                    AND ((f.source_user_id = $1 AND f.target_user_id = fgp.user_id)
                      OR (f.target_user_id = $1 AND f.source_user_id = fgp.user_id))
               WHERE fgp.game_id = g.id)
         GROUP BY g.id, gt.name, g.finished_at
         ORDER BY g.finished_at DESC NULLS LAST
         LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// The user's current name straight from the `users` table - the session's
/// cached copy can be stale after a rename. Plain query, matching
/// `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn get_user_name(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Renames a user. Returns `Ok(false)` when the name is already taken
/// case-insensitively (unique violation on `users_name_lower_key`); the
/// caller turns that into a field error. Plain query for the same reason as
/// `get_user_theme`.
#[cfg(feature = "ssr")]
pub async fn set_user_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<bool> {
    let res = sqlx::query("UPDATE users SET name = $1, updated_at = NOW() WHERE id = $2")
        .bind(name)
        .bind(user_id)
        .execute(pool)
        .await;
    match res {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// The user's stored colour preferences, legacy names ("Amber", "BlueGrey")
/// normalized onto the current palette. May be empty (never set) - the
/// settings server fn applies the palette-order default.
#[cfg(feature = "ssr")]
pub async fn get_user_pref_colors(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>> {
    let row: Option<(Vec<String>,)> = sqlx::query_as("SELECT pref_colors FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(|(colors,)| colors.iter().map(|c| normalize_pref_color(c)).collect())
        .unwrap_or_default())
}

#[cfg(feature = "ssr")]
pub async fn set_user_pref_colors(pool: &PgPool, user_id: Uuid, colors: &[String]) -> Result<()> {
    sqlx::query("UPDATE users SET pref_colors = $1, updated_at = NOW() WHERE id = $2")
        .bind(colors)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_user_email_prefs(pool: &PgPool, user_id: Uuid) -> Result<(bool, bool, bool)> {
    let row: (bool, bool, bool) = sqlx::query_as(
        "SELECT turn_emails_enabled, invite_emails_enabled, reminder_emails_enabled FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

#[cfg(feature = "ssr")]
pub async fn set_user_turn_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET turn_emails_enabled = $1, updated_at = NOW() WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn set_user_invite_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET invite_emails_enabled = $1, updated_at = NOW() WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn set_user_reminder_emails_enabled(
    pool: &PgPool,
    user_id: Uuid,
    enabled: bool,
) -> Result<()> {
    sqlx::query("UPDATE users SET reminder_emails_enabled = $1, updated_at = NOW() WHERE id = $2")
        .bind(enabled)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- #22d multiple emails per account (spec 2026-07-05-22, section 22d) ---
// Plain (non-macro) queries throughout, matching get_user_theme above.

/// A user's email address row for the settings list. `verified_at` is NULL
/// until the address is confirmed: added addresses start unverified, while
/// signup / invite / pre-feature rows are verified.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserEmailRow {
    pub id: Uuid,
    pub email: String,
    pub is_primary: bool,
    pub verified_at: Option<time::PrimitiveDateTime>,
}

/// The 22d switch-digest cap: at most this many turn notifications per switch
/// (Resend free-tier quota protection).
#[cfg(feature = "ssr")]
pub const SWITCH_DIGEST_CAP: usize = 20;

/// Pure predicate: an address may be removed only if it is NOT the primary.
#[cfg(feature = "ssr")]
pub fn can_remove_email(is_primary: bool) -> bool {
    !is_primary
}

/// Pure predicate: an address may be switched to (made active) only once
/// verified.
#[cfg(feature = "ssr")]
pub fn can_switch_to_email(verified_at: Option<time::PrimitiveDateTime>) -> bool {
    verified_at.is_some()
}

/// Pure predicate: an unverified address is expired once older than
/// `threshold` (the 22d ~24h cleanup window). Verified addresses never expire.
#[cfg(feature = "ssr")]
pub fn is_expired_unverified(
    verified_at: Option<time::PrimitiveDateTime>,
    created_at: time::PrimitiveDateTime,
    now: time::PrimitiveDateTime,
    threshold: std::time::Duration,
) -> bool {
    if verified_at.is_some() {
        return false;
    }
    let threshold = time::Duration::try_from(threshold).unwrap_or(time::Duration::hours(24));
    (now - created_at) >= threshold
}

/// Pure: cap a switch-digest at the first `cap` items (quota protection).
#[cfg(feature = "ssr")]
pub fn cap_digest<T>(mut items: Vec<T>, cap: usize) -> Vec<T> {
    items.truncate(cap);
    items
}

/// All of a user's addresses, primary first then by creation order, with
/// verification status.
#[cfg(feature = "ssr")]
pub async fn list_user_emails(pool: &PgPool, user_id: Uuid) -> Result<Vec<UserEmailRow>> {
    Ok(sqlx::query_as::<_, UserEmailRow>(
        "SELECT id, email, is_primary, verified_at FROM user_emails
         WHERE user_id = $1
         ORDER BY is_primary DESC, created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

/// Which account (if any) already owns this address. Used to reject re-adding
/// an address already on the caller's account and to reject addresses owned by
/// another account (global UNIQUE(email)).
#[cfg(feature = "ssr")]
pub async fn find_email_owner(pool: &PgPool, email: &str) -> Result<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT user_id FROM user_emails WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?,
    )
}

/// Adds a new UNVERIFIED, non-primary address (the 22d "add address" first
/// step; confirmation later sets `verified_at`). `Ok(None)` on a global
/// UNIQUE(email) violation (address already taken).
#[cfg(feature = "ssr")]
pub async fn insert_unverified_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<Option<Uuid>> {
    let res = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO user_emails (user_id, email, is_primary)
         VALUES ($1, $2, false) RETURNING id",
    )
    .bind(user_id)
    .bind(email)
    .fetch_one(pool)
    .await;
    match res {
        Ok(id) => Ok(Some(id)),
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Marks an address verified (the 22d "confirm address" step). Returns whether
/// a row was updated (false = no matching unverified address on this account).
#[cfg(feature = "ssr")]
pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid, email: &str) -> Result<bool> {
    let res = sqlx::query(
        "UPDATE user_emails SET verified_at = NOW(), updated_at = NOW()
         WHERE user_id = $1 AND email = $2 AND verified_at IS NULL",
    )
    .bind(user_id)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

/// Outcome of a make-active (set-primary) attempt.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPrimaryOutcome {
    Switched,
    NotFound,
    Unverified,
}

/// Sets `email` as the user's primary address in ONE transaction: rejects an
/// unknown or unverified address, otherwise clears the old primary and sets the
/// new one (the partial unique index enforces exactly one primary).
#[cfg(feature = "ssr")]
pub async fn set_primary_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<SetPrimaryOutcome> {
    let mut tx = pool.begin().await?;
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT (verified_at IS NOT NULL) FROM user_emails WHERE user_id = $1 AND email = $2",
    )
    .bind(user_id)
    .bind(email)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((verified,)) = row else {
        return Ok(SetPrimaryOutcome::NotFound);
    };
    if !verified {
        return Ok(SetPrimaryOutcome::Unverified);
    }
    sqlx::query(
        "UPDATE user_emails SET is_primary = false, updated_at = NOW()
         WHERE user_id = $1 AND is_primary = true",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE user_emails SET is_primary = true, updated_at = NOW()
         WHERE user_id = $1 AND email = $2",
    )
    .bind(user_id)
    .bind(email)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(SetPrimaryOutcome::Switched)
}

/// Outcome of a remove-address attempt.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveEmailOutcome {
    Removed,
    NotFound,
    IsPrimary,
}

/// Removes a non-primary address. The primary cannot be removed (switch first).
#[cfg(feature = "ssr")]
pub async fn remove_user_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
) -> Result<RemoveEmailOutcome> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT is_primary FROM user_emails WHERE user_id = $1 AND email = $2")
            .bind(user_id)
            .bind(email)
            .fetch_optional(pool)
            .await?;
    let Some((is_primary,)) = row else {
        return Ok(RemoveEmailOutcome::NotFound);
    };
    if !can_remove_email(is_primary) {
        return Ok(RemoveEmailOutcome::IsPrimary);
    }
    sqlx::query("DELETE FROM user_emails WHERE user_id = $1 AND email = $2 AND is_primary = false")
        .bind(user_id)
        .bind(email)
        .execute(pool)
        .await?;
    Ok(RemoveEmailOutcome::Removed)
}

/// Games where the user currently holds the turn in an unfinished game, oldest
/// turn first, capped at `cap` (the 22d switch-digest targets). Returns
/// `(game_id, game_player_id)` pairs.
#[cfg(feature = "ssr")]
pub async fn find_active_turn_games(
    pool: &PgPool,
    user_id: Uuid,
    cap: usize,
) -> Result<Vec<(Uuid, Uuid)>> {
    let cap = cap as i64;
    Ok(sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT gp.game_id, gp.id
         FROM game_players gp
         JOIN games g ON gp.game_id = g.id
         WHERE gp.user_id = $1 AND gp.is_turn = true AND g.is_finished = false
         ORDER BY gp.is_turn_at ASC NULLS LAST
         LIMIT $2",
    )
    .bind(user_id)
    .bind(cap)
    .fetch_all(pool)
    .await?)
}

/// Deletes unverified addresses older than `threshold` (the 22d expiry
/// cleanup). Verified rows are never touched. Returns the count deleted.
#[cfg(feature = "ssr")]
pub async fn delete_expired_unverified_emails(
    pool: &PgPool,
    threshold: std::time::Duration,
) -> Result<u64> {
    let secs = threshold.as_secs() as i64;
    let res = sqlx::query(
        "DELETE FROM user_emails
         WHERE verified_at IS NULL AND created_at < NOW() - ($1 || ' seconds')::interval",
    )
    .bind(secs.to_string())
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::game::server_fns::BotSlot;

    #[sqlx::test]
    async fn migrations_apply_and_pool_connects(pool: sqlx::PgPool) -> sqlx::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await?;
        assert_eq!(count, 0);
        Ok(())
    }

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

    // --- validate_username ---

    #[test]
    fn validate_username_accepts_valid_names() {
        for name in ["Sam", "big-scary-walrus", "a", "user_1", "ABCDEFGHIJKLMNOP"] {
            assert!(validate_username(name), "{name} should be valid");
        }
    }

    #[test]
    fn validate_username_rejects_invalid_names() {
        for name in [
            "",
            "seventeen-letters!",
            "with space",
            "émile",
            "toolongtoolongtoo",
            "a.b",
        ] {
            assert!(!validate_username(name), "{name} should be invalid");
        }
    }

    #[test]
    fn petname_output_charset_is_username_safe() {
        // Length can exceed 16 (generate_unique_username retries those away);
        // the charset itself must always pass.
        for _ in 0..20 {
            let name = petname::petname(2, "-").expect("petname generates");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "unexpected char in {name}"
            );
        }
    }

    // --- Fixture helpers ---

    async fn make_user(pool: &PgPool, name: &str) -> User {
        sqlx::query_as!(
            User,
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING id, created_at, updated_at, name, pref_colors, theme, is_admin",
            Uuid::new_v4(),
            name,
            &Vec::<String>::new()
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /// Creates a game type + a public, non-deprecated game version pointing at a
    /// dummy URI. None of the db.rs functions under test call out to the game
    /// service over HTTP, so the URI is never dereferenced.
    async fn make_game_type_and_version(pool: &PgPool) -> (Uuid, Uuid) {
        let game_type_id = sqlx::query_scalar!(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
            format!("Test Game {}", Uuid::new_v4()),
            &vec![2, 3, 4]
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let game_version_id = sqlx::query_scalar!(
            r#"INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
               VALUES ($1, $2, $3, true, false) RETURNING id"#,
            game_type_id,
            "1.0.0",
            "http://localhost:0/mock"
        )
        .fetch_one(pool)
        .await
        .unwrap();

        (game_type_id, game_version_id)
    }

    /// Creates a fixture game with `human_users.len()` human players followed by
    /// `bot_count` bot players (positions assigned in that order), using
    /// `create_game_with_users` so the function under test in point 1 doubles as
    /// the fixture builder for the other tests.
    async fn make_game_with_players(
        pool: &PgPool,
        game_version_id: Uuid,
        creator_id: Uuid,
        opponent_ids: &[Uuid],
        bot_count: usize,
        whose_turn: &[usize],
    ) -> crate::models::game::Game {
        let bot_slots: Vec<BotSlot> = (0..bot_count)
            .map(|i| BotSlot {
                name: format!("Bot {}", i),
                bot_name: "easy".to_string(),
            })
            .collect();

        create_game_with_users(
            pool,
            CreateGameOpts {
                game_version_id,
                whose_turn,
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id,
                opponent_ids,
                opponent_emails: &[],
                bot_slots: &bot_slots,
                chat_id: None,
                game_state: "initial_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap()
    }

    // --- #30 friends lifecycle ---

    async fn friend_row_state(pool: &PgPool, a: Uuid, b: Uuid) -> Option<(Uuid, Option<bool>)> {
        sqlx::query_as::<_, (Uuid, Option<bool>)>(
            "SELECT source_user_id, has_accepted FROM friends
             WHERE (source_user_id = $1 AND target_user_id = $2)
                OR (source_user_id = $2 AND target_user_id = $1)",
        )
        .bind(a)
        .bind(b)
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn friend_request_creates_pending_row(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    #[sqlx::test]
    async fn reverse_pending_request_auto_accepts(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
        let mut conn = pool.acquire().await.unwrap();
        assert!(are_friends_conn(&mut conn, a.id, b.id).await.unwrap());
    }

    #[sqlx::test]
    async fn accept_and_decline_update_pending_row(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        // wrong responder: the requester cannot accept their own request
        assert!(
            !respond_to_friend_request(&pool, req_id, a.id, true)
                .await
                .unwrap()
        );
        assert!(
            respond_to_friend_request(&pool, req_id, b.id, true)
                .await
                .unwrap()
        );
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
        // already-responded request is no longer pending
        assert!(
            !respond_to_friend_request(&pool, req_id, b.id, false)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn rerequest_after_decline_is_silent_noop(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        assert!(
            respond_to_friend_request(&pool, req_id, b.id, false)
                .await
                .unwrap()
        );
        // silent shield: re-request succeeds but the row stays declined
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(false)))
        );
        // and the requester still sees it as an outgoing "pending" request
        let outgoing = list_outgoing_friend_requests(&pool, a.id).await.unwrap();
        assert_eq!(outgoing, vec![(b.id, "bob".to_string())]);
    }

    #[sqlx::test]
    async fn decliner_own_request_flips_to_accepted(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        respond_to_friend_request(&pool, req_id, b.id, false)
            .await
            .unwrap();
        // b changed their mind: both sides have now expressed intent
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, Some(true)))
        );
    }

    #[sqlx::test]
    async fn pair_unique_index_rejects_reverse_duplicate(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
            .bind(a.id)
            .bind(b.id)
            .execute(&pool)
            .await
            .unwrap();
        let err =
            sqlx::query("INSERT INTO friends (source_user_id, target_user_id) VALUES ($1, $2)")
                .bind(b.id)
                .bind(a.id)
                .execute(&pool)
                .await;
        assert!(
            err.is_err(),
            "pair-unique index must reject B->A when A->B exists"
        );
    }

    #[sqlx::test]
    async fn self_request_rejected_by_db_check(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        assert!(send_friend_request(&pool, a.id, a.id).await.is_err());
    }

    #[sqlx::test]
    async fn unfriend_deletes_accepted_but_not_declined(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        let (req_id, _, _) = list_incoming_friend_requests(&pool, b.id).await.unwrap()[0];
        respond_to_friend_request(&pool, req_id, b.id, false)
            .await
            .unwrap();
        // declined row survives unfriend (anti-harassment shield stays)
        unfriend(&pool, a.id, b.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_some());
        // flip to accepted, then unfriend from the other side deletes it
        send_friend_request(&pool, b.id, a.id).await.unwrap();
        unfriend(&pool, b.id, a.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        // clean slate: fresh request allowed
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    #[sqlx::test]
    async fn friend_lists_and_name_lookup(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        let c = make_user(&pool, "carol").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        send_friend_request(&pool, c.id, a.id).await.unwrap(); // incoming pending for a
        assert_eq!(
            list_friends(&pool, a.id).await.unwrap(),
            vec![(b.id, "bob".to_string())]
        );
        let incoming = list_incoming_friend_requests(&pool, a.id).await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(
            (incoming[0].1, incoming[0].2.clone()),
            (c.id, "carol".to_string())
        );
        assert_eq!(
            list_outgoing_friend_requests(&pool, c.id).await.unwrap(),
            vec![(a.id, "alice".to_string())]
        );
        assert_eq!(
            get_user_by_name(&pool, "ALICE").await.unwrap(),
            Some((a.id, "alice".to_string()))
        );
        assert_eq!(get_user_by_name(&pool, "nobody").await.unwrap(), None);
    }

    // --- #30 blocks (D7) ---

    #[sqlx::test]
    async fn block_severs_friendship_and_pending(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        block_user(&pool, b.id, a.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        assert!(has_block(&pool, b.id, a.id).await.unwrap());
        assert!(!has_block(&pool, a.id, b.id).await.unwrap()); // directed
        assert_eq!(
            list_blocked(&pool, b.id).await.unwrap(),
            vec![(a.id, "alice".to_string())]
        );
        // idempotent
        block_user(&pool, b.id, a.id).await.unwrap();
    }

    #[sqlx::test]
    async fn blocked_requester_is_silently_ignored(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        block_user(&pool, b.id, a.id).await.unwrap();
        // a's request "succeeds" but writes nothing (silent shield)
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert!(friend_row_state(&pool, a.id, b.id).await.is_none());
        assert!(
            list_incoming_friend_requests(&pool, b.id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn unblock_allows_fresh_request_but_restores_nothing(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        block_user(&pool, b.id, a.id).await.unwrap();
        unblock_user(&pool, b.id, a.id).await.unwrap();
        assert!(!has_block(&pool, b.id, a.id).await.unwrap());
        let mut conn = pool.acquire().await.unwrap();
        assert!(!are_friends_conn(&mut conn, a.id, b.id).await.unwrap());
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        assert_eq!(
            friend_row_state(&pool, a.id, b.id).await,
            Some((a.id, None))
        );
    }

    // --- #30 invite policy (D4) + block enforcement (D7) ---

    async fn check_roster(
        pool: &PgPool,
        creator: Uuid,
        ids: &[Uuid],
        emails: &[String],
    ) -> Vec<String> {
        let mut tx = pool.begin().await.unwrap();
        let v = check_invite_policy_tx(&mut tx, creator, ids, emails)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        v
    }

    #[sqlx::test]
    async fn invite_policy_default_open_allows_everyone(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        assert_eq!(get_invite_policy(&pool, b.id).await.unwrap(), "open");
        assert!(check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
    }

    #[sqlx::test]
    async fn invite_policy_none_blocks_with_generic_message(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        set_invite_policy(&pool, b.id, "none").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
    }

    #[sqlx::test]
    async fn invite_policy_friends_requires_accepted_friendship(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        set_invite_policy(&pool, b.id, "friends").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob only accepts games from friends".to_string()]
        );
        send_friend_request(&pool, a.id, b.id).await.unwrap();
        // pending is not enough
        assert!(!check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
        send_friend_request(&pool, b.id, a.id).await.unwrap(); // accepted
        assert!(check_roster(&pool, a.id, &[b.id], &[]).await.is_empty());
    }

    #[sqlx::test]
    async fn policy_check_covers_email_of_existing_user(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)")
            .bind(b.id)
            .bind("bob@example.com")
            .execute(&pool)
            .await
            .unwrap();
        set_invite_policy(&pool, b.id, "none").await.unwrap();
        assert_eq!(
            check_roster(&pool, a.id, &[], &["bob@example.com".to_string()]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
        // unknown email = account created later with default 'open': passes
        assert!(
            check_roster(&pool, a.id, &[], &["new@example.com".to_string()])
                .await
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn blocks_stop_game_inclusion_both_ways(pool: PgPool) {
        let a = make_user(&pool, "alice").await;
        let b = make_user(&pool, "bob").await;
        block_user(&pool, b.id, a.id).await.unwrap();
        // b blocked a: a's attempt fails with wording identical to policy
        // 'none' (deniability, D7)
        assert_eq!(
            check_roster(&pool, a.id, &[b.id], &[]).await,
            vec!["bob is not accepting game invitations".to_string()]
        );
        // and b cannot rope a into a game either, with an honest message
        assert_eq!(
            check_roster(&pool, b.id, &[a.id], &[]).await,
            vec!["You have blocked alice".to_string()]
        );
    }

    // --- #30 opponent suggestions (D6) ---

    #[sqlx::test]
    async fn suggestions_friends_first_then_recent_coplayers(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend_old = make_user(&pool, "zed").await; // friend, played long ago
        let friend_new = make_user(&pool, "amy").await; // friend, played recently
        let stranger = make_user(&pool, "stranger").await; // co-player, not friend
        for f in [friend_old.id, friend_new.id] {
            send_friend_request(&pool, me.id, f).await.unwrap();
            send_friend_request(&pool, f, me.id).await.unwrap();
        }
        let (_, version) = make_game_type_and_version(&pool).await;
        let g1 = make_game_with_players(&pool, version, me.id, &[friend_old.id], 0, &[0]).await;
        let g2 = make_game_with_players(&pool, version, me.id, &[friend_new.id], 0, &[0]).await;
        let g3 = make_game_with_players(&pool, version, me.id, &[stranger.id], 0, &[0]).await;
        // force distinct recency: g1 oldest, g3 newest
        for (i, gid) in [g1.id, g2.id, g3.id].iter().enumerate() {
            sqlx::query(
                "UPDATE games SET updated_at = NOW() - make_interval(days => $1) WHERE id = $2",
            )
            .bind(3 - i as i32)
            .bind(gid)
            .execute(&pool)
            .await
            .unwrap();
        }
        let s = opponent_suggestions(&pool, me.id).await.unwrap();
        assert_eq!(
            s,
            vec![
                (friend_new.id, "amy".to_string(), true), // friends by recency
                (friend_old.id, "zed".to_string(), true),
                (stranger.id, "stranger".to_string(), false), // then co-players
            ]
        );
    }

    #[sqlx::test]
    async fn suggestions_exclude_blocked_and_self(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let blocked_by_me = make_user(&pool, "villain").await;
        let blocked_me = make_user(&pool, "hermit").await;
        let (_, version) = make_game_type_and_version(&pool).await;
        make_game_with_players(
            &pool,
            version,
            me.id,
            &[blocked_by_me.id, blocked_me.id],
            0,
            &[0],
        )
        .await;
        block_user(&pool, me.id, blocked_by_me.id).await.unwrap();
        block_user(&pool, blocked_me.id, me.id).await.unwrap();
        assert!(opponent_suggestions(&pool, me.id).await.unwrap().is_empty());
    }

    // --- #30 dashboard queries (D5) ---

    #[sqlx::test]
    async fn friends_active_games_excludes_own_and_nonfriend_games(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend = make_user(&pool, "friend").await;
        let other = make_user(&pool, "other").await;
        let bystander = make_user(&pool, "bystander").await;
        send_friend_request(&pool, me.id, friend.id).await.unwrap();
        send_friend_request(&pool, friend.id, me.id).await.unwrap();
        let (_, version) = make_game_type_and_version(&pool).await;
        // friend's game without me: should appear
        let g = make_game_with_players(&pool, version, friend.id, &[other.id], 0, &[0]).await;
        // my own game with the friend: excluded (I am in it)
        make_game_with_players(&pool, version, friend.id, &[me.id], 0, &[0]).await;
        // game with no friends in it: excluded
        make_game_with_players(&pool, version, other.id, &[bystander.id], 0, &[0]).await;
        let rows = friends_active_games(&pool, me.id, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, g.id);
        let mut names = rows[0].2.clone();
        names.sort();
        assert_eq!(names, vec!["friend".to_string(), "other".to_string()]);
    }

    #[sqlx::test]
    async fn friends_recent_results_return_places(pool: PgPool) {
        let me = make_user(&pool, "me").await;
        let friend = make_user(&pool, "friend").await;
        let other = make_user(&pool, "other").await;
        send_friend_request(&pool, me.id, friend.id).await.unwrap();
        send_friend_request(&pool, friend.id, me.id).await.unwrap();
        let (_, version) = make_game_type_and_version(&pool).await;
        let g = make_game_with_players(&pool, version, friend.id, &[other.id], 0, &[0]).await;
        sqlx::query("UPDATE games SET is_finished = TRUE, finished_at = timezone('utc', now()) WHERE id = $1")
            .bind(g.id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE game_players SET place = 1 WHERE game_id = $1 AND user_id = $2")
            .bind(g.id)
            .bind(friend.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE game_players SET place = 2 WHERE game_id = $1 AND user_id = $2")
            .bind(g.id)
            .bind(other.id)
            .execute(&pool)
            .await
            .unwrap();
        let rows = friends_recent_results(&pool, me.id, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        let (game_id, _type_name, finished_at, names, places) = rows[0].clone();
        assert_eq!(game_id, g.id);
        assert!(finished_at.is_some());
        assert_eq!(names, vec!["friend".to_string(), "other".to_string()]);
        assert_eq!(places, vec![1, 2]);
    }

    // --- 1. create_game_with_users ---

    #[sqlx::test]
    async fn create_game_with_users_assigns_positions_and_colors(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(
            &pool,
            game_version_id,
            creator.id,
            &[opponent.id],
            1, // one bot
            &[0],
        )
        .await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge.game_players.len(), 3);

        // Positions are sequential 0..n and colors assigned in the same order.
        let expected_colors = ["Green", "Red", "Blue"];
        for (i, p) in ge.game_players.iter().enumerate() {
            assert_eq!(p.game_player.position, i as i32);
            assert_eq!(p.game_player.color, expected_colors[i]);
        }

        // Creator + opponent rows exist as users; exactly one bot slot.
        let human_ids: Vec<Uuid> = ge
            .game_players
            .iter()
            .filter_map(|p| p.user.as_ref().map(|u| u.id))
            .collect();
        assert!(human_ids.contains(&creator.id));
        assert!(human_ids.contains(&opponent.id));

        let bot_players: Vec<_> = ge
            .game_players
            .iter()
            .filter(|p| p.game_bot.is_some())
            .collect();
        assert_eq!(bot_players.len(), 1);
        let bot_player = bot_players[0];
        assert!(bot_player.game_player.user_id.is_none());
        assert!(bot_player.game_bot.is_some());

        // XOR constraint holds for every player row (checked at DB level too).
        for p in &ge.game_players {
            assert!(p.game_player.user_id.is_some() != p.game_bot.is_some());
        }

        // Underlying game_bots row has game_bot_id set and user_id NULL directly
        // via raw query (belt-and-braces check of the XOR constraint columns).
        let raw = sqlx::query!(
            "SELECT user_id, game_bot_id FROM game_players WHERE game_id = $1 AND user_id IS NULL",
            game.id
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(raw.len(), 1);
        assert!(raw[0].game_bot_id.is_some());

        // Initial is_turn matches whose_turn = [0].
        assert!(ge.game_players[0].game_player.is_turn);
        assert!(!ge.game_players[1].game_player.is_turn);
        assert!(!ge.game_players[2].game_player.is_turn);
    }

    // --- 2. find_game_extended ---

    #[sqlx::test]
    async fn find_game_extended_round_trips_mixed_players(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge.game.id, game.id);
        assert_eq!(ge.game_type.id, game_type_id);
        assert_eq!(ge.game_version.id, game_version_id);
        assert_eq!(ge.game_players.len(), 2);

        let human = ge.game_players.iter().find(|p| p.user.is_some()).unwrap();
        assert_eq!(human.user.as_ref().unwrap().id, creator.id);
        assert!(human.game_bot.is_none());

        let bot = ge
            .game_players
            .iter()
            .find(|p| p.game_bot.is_some())
            .unwrap();
        assert!(bot.user.is_none());

        // create_game_with_users itself inserts a game_type_users row (DB
        // column default rating 1200), so it's present here.
        assert_eq!(human.game_type_user.rating, 1200);
        assert_eq!(human.game_type_user.peak_rating, 1200);
        assert_eq!(human.game_type_user.user_id, creator.id);

        // Nonexistent game id returns Ok(None), not a panic.
        let missing = find_game_extended(&pool, Uuid::new_v4()).await.unwrap();
        assert!(missing.is_none());
    }

    // --- find_bot_turns ---

    #[sqlx::test]
    async fn find_bot_turns_returns_only_on_turn_bots(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;

        // Human on turn, bot off turn: no bot turns.
        sqlx::query!(
            "UPDATE game_players SET is_turn = (user_id IS NOT NULL) WHERE game_id = $1",
            game.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let turns = find_bot_turns(&pool, game.id).await.unwrap();
        assert!(turns.is_empty());

        // Bot on turn: exactly one row with the bot's position and bot_name.
        sqlx::query!(
            "UPDATE game_players SET is_turn = (game_bot_id IS NOT NULL) WHERE game_id = $1",
            game.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let bot_position = sqlx::query_scalar!(
            "SELECT position FROM game_players WHERE game_id = $1 AND game_bot_id IS NOT NULL",
            game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let turns = find_bot_turns(&pool, game.id).await.unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].position, bot_position);
        assert_eq!(turns[0].bot_name, "easy");

        // Nonexistent game id is an empty vec, not an error.
        let missing = find_bot_turns(&pool, Uuid::new_v4()).await.unwrap();
        assert!(missing.is_empty());
    }

    // --- is_player_in_game ---

    #[sqlx::test]
    async fn is_player_in_game_checks_membership(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let outsider = make_user(&pool, "outsider").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;

        assert!(is_player_in_game(&pool, game.id, creator.id).await.unwrap());
        assert!(
            !is_player_in_game(&pool, game.id, outsider.id)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn find_game_extended_missing_game_type_user_defaults_to_1200(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;

        // Explicitly insert a game_type_users row for a *different* game type to
        // make sure the LEFT JOIN filter (game_type_id match) is respected, and
        // that a genuinely missing row still defaults correctly.
        let (_other_game_type_id, _) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 0, &[0]).await;

        // create_game_with_users auto-creates a game_type_users row; delete it
        // to exercise the genuinely-missing-row default path in
        // build_game_type_user (rating/peak_rating default to 1200, matching
        // the DB column default).
        sqlx::query!(
            "DELETE FROM game_type_users WHERE user_id = $1 AND game_type_id = $2",
            creator.id,
            game_type_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let human = &ge.game_players[0];
        assert_eq!(human.game_type_user.rating, 1200);
        assert_eq!(human.game_type_user.peak_rating, 1200);
        assert_eq!(human.game_type_user.game_type_id, game_type_id);
    }

    // --- 3. find_active_game_summaries ---

    #[sqlx::test]
    async fn find_active_game_summaries_groups_and_filters(pool: PgPool) {
        let user = make_user(&pool, "user").await;
        let other = make_user(&pool, "other").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;

        // Game 1: user is player 0 (their turn).
        let game1 =
            make_game_with_players(&pool, game_version_id, user.id, &[other.id], 0, &[0]).await;
        // Game 2: user is player 1 (opponent's turn, not user's).
        let game2 =
            make_game_with_players(&pool, game_version_id, other.id, &[user.id], 0, &[0]).await;
        // Game 3: user in a finished game - must be excluded.
        let game3 = create_game_with_users(
            &pool,
            CreateGameOpts {
                game_version_id,
                whose_turn: &[],
                eliminated: &[],
                placings: &[0, 1],
                points: &[1.0, 0.0],
                creator_id: user.id,
                opponent_ids: &[other.id],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "finished_state",
                all_accepted: false,
            },
        )
        .await
        .unwrap();

        let summaries = find_active_game_summaries(&pool, user.id).await.unwrap();
        let game_ids: Vec<Uuid> = summaries.iter().map(|s| s.id).collect();

        assert!(game_ids.contains(&game1.id));
        assert!(game_ids.contains(&game2.id));
        assert!(
            !game_ids.contains(&game3.id),
            "finished games must be excluded"
        );
        assert_eq!(summaries.len(), 2, "no duplicate/mis-grouped rows");

        for s in &summaries {
            // The other human is the only opponent; the user never appears.
            assert_eq!(s.opponents.len(), 1);
            assert_eq!(s.opponents[0].name, "other");
        }

        // A user in no games gets an empty vec, not an error.
        let lonely = make_user(&pool, "lonely").await;
        let none = find_active_game_summaries(&pool, lonely.id).await.unwrap();
        assert!(none.is_empty());
    }

    // --- 4. update_game_command_success ---

    #[sqlx::test]
    async fn update_game_command_success_writes_active_fields(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        let ge_before = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player = ge_before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let played_player_id = played_player.game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "new_state",
            true, // can_undo
            &StatusUpdate {
                is_finished: false,  // -> Active
                whose_turn: vec![1], // whose_turn moves to position 1
                eliminated: vec![0], // position 0 is eliminated
                placings: vec![],
            },
            &[3.5, 1.5],
            ge_before.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "new_state");
        assert!(!ge_after.game.is_finished);

        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();

        assert!(!p0.game_player.is_turn);
        assert!(p1.game_player.is_turn);
        // eliminated = [0] must land on position 0's is_eliminated flag only,
        // and must not bleed into place (same-typed placings slice).
        assert!(p0.game_player.is_eliminated);
        assert!(!p1.game_player.is_eliminated);
        assert_eq!(p0.game_player.place, None);
        assert_eq!(p0.game_player.points, Some(3.5));
        assert_eq!(p1.game_player.points, Some(1.5));
        // Only the played player gets undo state stashed.
        assert_eq!(
            p0.game_player.undo_game_state,
            Some("prev_state".to_string())
        );
        assert_eq!(p1.game_player.undo_game_state, None);
        // last_turn_at bumped by the DB trigger on the is_turn true->false
        // transition (p0 leaves turn here), not by the played-player override.
        assert!(p0.game_player.last_turn_at > played_player.game_player.last_turn_at);
        // is_turn_at bumped for whoever's turn it now is (p1).
        assert!(p1.game_player.is_turn_at >= played_player.game_player.is_turn_at);
    }

    #[sqlx::test]
    async fn update_game_command_success_mid_turn_keeps_last_turn_at(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        let ge_before = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player = ge_before
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let played_player_id = played_player.game_player.id;
        let last_turn_at_before = played_player.game_player.last_turn_at;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "new_state",
            true, // can_undo
            &StatusUpdate {
                is_finished: false,  // -> Active
                whose_turn: vec![0], // position 0 stays in turn (mid-turn command)
                eliminated: vec![],
                placings: vec![],
            },
            &[3.5, 1.5],
            ge_before.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();

        // No is_turn true->false transition occurred for p0, so the DB
        // trigger does not fire and last_turn_at must be unchanged.
        assert!(p0.game_player.is_turn);
        assert_eq!(p0.game_player.last_turn_at, last_turn_at_before);
    }

    #[sqlx::test]
    async fn update_game_command_success_writes_finished_fields(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true, // -> Finished
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2], // placings by position
            },
            &[10.0, 5.0],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(ge_after.game.is_finished);
        let first_finished_at = ge_after.game.finished_at.expect("finished_at set");

        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(p0.game_player.place, Some(1));
        assert_eq!(p1.game_player.place, Some(2));

        // The COALESCE only guards the case where the finished_at param is NULL
        // (i.e. is_finished = false): finished_at is preserved rather than
        // cleared. When is_finished = true it always passes Some(now), so a
        // second "finished" call actually advances finished_at rather than
        // preserving it - this differs from the plan's phrasing, see report.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "final_state",
            "final_state_2",
            false,
            // is_finished = false -> finished_at param is None
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[10.0, 5.0],
            ge_after.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(
            ge_after_2.game.finished_at,
            Some(first_finished_at),
            "COALESCE preserves finished_at when the new value is NULL"
        );
    }

    #[sqlx::test]
    async fn update_game_command_success_keeps_first_undo_stash_in_a_run(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        // First can_undo=true command by player 0.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Second can_undo=true command by the same player.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_1",
            "state_2",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(
            p0.game_player.undo_game_state,
            Some("state_0".to_string()),
            "the run's undo stash must stay pinned to the first command's prev_game_state"
        );
    }

    #[sqlx::test]
    async fn update_game_command_success_clears_stash_on_non_undoable_command(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        // can_undo=true stashes state_0.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // Same player plays a can_undo=false command; the stash must clear.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "state_1",
            "state_2",
            false,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(p0.game_player.undo_game_state, None);
    }

    #[sqlx::test]
    async fn update_game_command_success_clears_stash_when_opponent_plays(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap()
            .game_player
            .id;
        let p1_id = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap()
            .game_player
            .id;

        // Player 0 plays a can_undo=true command, stashing state_0.
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_0",
            "state_1",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_1 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_after_1 = ge_after_1
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        assert_eq!(
            p0_after_1.game_player.undo_game_state,
            Some("state_0".to_string())
        );

        // Opponent (player 1) plays next; player 0's stash must clear since
        // player 0 is not the played player on this command.
        update_game_command_success(
            &pool,
            game.id,
            p1_id,
            "state_1",
            "state_2",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge_after_1.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after_2 = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_after_2 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1_after_2 = ge_after_2
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert_eq!(
            p0_after_2.game_player.undo_game_state, None,
            "opponent's command must clear player 0's stash"
        );
        assert_eq!(
            p1_after_2.game_player.undo_game_state,
            Some("state_1".to_string())
        );
    }

    // --- 5. undo_game ---

    #[sqlx::test]
    async fn undo_game_restores_state_and_clears_undo(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[1])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0_id = ge.game_players[0].game_player.id;

        // Simulate a played command that stashed undo state for player 0.
        update_game_command_success(
            &pool,
            game.id,
            p0_id,
            "state_before_move",
            "state_after_move",
            true,
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![1],
                eliminated: vec![],
                placings: vec![],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        undo_game(
            &pool,
            game.id,
            "state_before_move",
            0, // player_position that used the undo
            &StatusUpdate {
                is_finished: false,
                whose_turn: vec![0],
                eliminated: vec![],
                placings: vec![],
            },
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(ge_after.game.game_state, "state_before_move");
        assert!(!ge_after.game.is_finished);
        assert!(ge_after.game.finished_at.is_none());

        for p in &ge_after.game_players {
            assert!(p.game_player.undo_game_state.is_none());
        }
        let p0 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();
        assert!(p0.game_player.is_turn);
        assert!(!p1.game_player.is_turn);

        let logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert!(logs.iter().any(|l| l.body == "{{player 0}} used an undo"));
    }

    // --- 6. concede_game ---

    #[sqlx::test]
    async fn concede_game_marks_finished(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceding = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceding_id = conceding.game_player.id;

        concede_game(&pool, game.id, conceding_id, "creator")
            .await
            .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        assert!(ge_after.game.is_finished);
        assert!(ge_after.game.finished_at.is_some());
    }

    // --- 7. game logs ---

    #[sqlx::test]
    async fn game_logs_public_and_private_visibility_and_order(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let p0 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let p1 = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 1)
            .unwrap();

        let base = time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::MIDNIGHT,
        );

        let logs = vec![
            brdgme_cmd::api::CliLog {
                content: "first public".to_string(),
                at: base,
                public: true,
                to: vec![],
            },
            brdgme_cmd::api::CliLog {
                content: "private to p0".to_string(),
                at: base + time::Duration::seconds(1),
                public: false,
                to: vec![0],
            },
            brdgme_cmd::api::CliLog {
                content: "second public".to_string(),
                at: base + time::Duration::seconds(2),
                public: true,
                to: vec![],
            },
        ];

        create_game_logs(&pool, game.id, logs).await.unwrap();

        let all_logs = get_all_game_logs(&pool, game.id).await.unwrap();
        assert_eq!(all_logs.len(), 3);
        // Ordered by logged_at ascending.
        assert_eq!(all_logs[0].body, "first public");
        assert_eq!(all_logs[1].body, "private to p0");
        assert_eq!(all_logs[2].body, "second public");

        let p0_logs = get_game_logs(&pool, game.id, p0.game_player.id)
            .await
            .unwrap();
        assert_eq!(p0_logs.len(), 3);

        let p1_logs = get_game_logs(&pool, game.id, p1.game_player.id)
            .await
            .unwrap();
        assert_eq!(
            p1_logs.len(),
            2,
            "p1 must not see the private log targeted at p0"
        );
        assert!(p1_logs.iter().all(|l| l.body != "private to p0"));
    }

    // --- 8. Auth queries ---
    // Login-code expiry/attempt behaviour now lives in login_confirmations
    // and is covered by the tests in auth/server.rs.

    #[sqlx::test]
    async fn session_token_validation(pool: PgPool) {
        use crate::auth::session::{invalidate_auth_token, validate_session_token};

        let user = make_user(&pool, "session-user").await;
        let token_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
            token_id,
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        assert!(validate_session_token(&pool, token_id).await.unwrap());

        // NOTE: validate_session_token performs a pure existence check with no
        // created_at comparison - the 30-day window described in the plan is
        // enforced only by the tower_sessions cookie expiry
        // (Expiry::OnInactivity(Duration::days(30)) in auth/session.rs
        // create_session_layer), not by this DB query. A token inserted 40 days
        // ago is still "valid" from the DB's point of view.
        sqlx::query!(
            "UPDATE user_auth_tokens SET created_at = NOW() - INTERVAL '40 days' WHERE id = $1",
            token_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            validate_session_token(&pool, token_id).await.unwrap(),
            "DB layer has no created_at expiry check - session expiry is cookie-side only"
        );

        invalidate_auth_token(&pool, token_id).await.unwrap();
        assert!(!validate_session_token(&pool, token_id).await.unwrap());

        // Nonexistent token id returns false, not an error.
        assert!(!validate_session_token(&pool, Uuid::new_v4()).await.unwrap());
    }

    // --- 9. ELO rating updates (Phase 12) ---

    #[test]
    fn elo_rating_change_works() {
        assert_eq!(elo_rating_change(1184, 1200, 0.0), -15i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.0), -29i32);
        assert_eq!(elo_rating_change(2400, 2000, 1.0), 3i32);
        assert_eq!(elo_rating_change(2400, 2000, 0.5), -13i32);
    }

    #[test]
    fn elo_rating_change_three_player_pairwise_sums_to_zero() {
        // Simulates the pairwise accumulation done in apply_rating_changes for
        // a 3-player game with placings [1, 2, 3] (position 0 wins, 1 second,
        // 2 last) and equal starting ratings.
        let ratings = [1200, 1200, 1200];
        let places = [1, 2, 3];
        let mut changes = [0i32; 3];
        for a in 0..ratings.len() - 1 {
            for b in (a + 1)..ratings.len() {
                let a_score: f32 = match places[a].cmp(&places[b]) {
                    std::cmp::Ordering::Less => 1.0,
                    std::cmp::Ordering::Equal => 0.5,
                    std::cmp::Ordering::Greater => 0.0,
                };
                let change = elo_rating_change(ratings[a], ratings[b], a_score);
                changes[a] += change;
                changes[b] -= change;
            }
        }
        // Zero-sum: total rating points gained equals total lost.
        assert_eq!(changes.iter().sum::<i32>(), 0);
        // Winner gains, last place loses.
        assert!(changes[0] > 0);
        assert!(changes[2] < 0);
        assert_eq!(changes, [32, 0, -32]);
    }

    async fn find_rating_change(pool: &PgPool, game_id: Uuid, position: i32) -> Option<i32> {
        sqlx::query_scalar!(
            "SELECT rating_change FROM game_players WHERE game_id = $1 AND position = $2",
            game_id,
            position
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn game_type_rating(pool: &PgPool, game_type_id: Uuid, user_id: Uuid) -> (i32, i32) {
        let row = sqlx::query!(
            "SELECT rating, peak_rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2",
            game_type_id,
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        (row.rating, row.peak_rating)
    }

    /// `create_game_with_users` shuffles slot order before assigning
    /// positions, so a user's position within a game is not predictable from
    /// call order. Look it up explicitly rather than assuming position 0/1/2.
    fn position_of(ge: &GameExtended, user_id: Uuid) -> i32 {
        ge.game_players
            .iter()
            .find(|p| p.user.as_ref().is_some_and(|u| u.id == user_id))
            .unwrap()
            .game_player
            .position
    }

    #[sqlx::test]
    async fn finishing_a_two_player_game_rates_both_players(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        // creator places 1st (winner), opponent 2nd (loser), by position.
        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        // Both players started at the DB default rating (1200): winner (place
        // 1) gains, loser (place 2) loses the same amount.
        let winner_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let loser_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;
        assert_eq!(winner_change, Some(16));
        assert_eq!(loser_change, Some(-16));

        let (winner_rating, winner_peak) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (loser_rating, loser_peak) = game_type_rating(&pool, game_type_id, opponent.id).await;
        assert_eq!(winner_rating, 1216);
        assert_eq!(winner_peak, 1216);
        assert_eq!(loser_rating, 1184);
        assert_eq!(loser_peak, 1200);
    }

    #[sqlx::test]
    async fn finishing_a_three_player_game_rates_all_pairs(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let p1 = make_user(&pool, "p1").await;
        let p2 = make_user(&pool, "p2").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[p1.id, p2.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let p1_pos = position_of(&ge, p1.id) as usize;
        let p2_pos = position_of(&ge, p2.id) as usize;

        // creator 1st, p1 2nd, p2 3rd, by position.
        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[p1_pos] = 2;
        placings[p2_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let c_creator = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let c_p1 = find_rating_change(&pool, game.id, p1_pos as i32).await;
        let c_p2 = find_rating_change(&pool, game.id, p2_pos as i32).await;
        assert_eq!(c_creator, Some(32));
        // A net-zero change is skipped entirely (spec: "skip zero changes"),
        // so rating_change stays NULL rather than being written as 0.
        assert_eq!(c_p1, None);
        assert_eq!(c_p2, Some(-32));
        // Zero-sum across all pairs.
        assert_eq!(
            c_creator.unwrap_or(0) + c_p1.unwrap_or(0) + c_p2.unwrap_or(0),
            0
        );
    }

    #[sqlx::test]
    async fn second_finish_does_not_re_rate(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (rating_after_first, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let ge_after_first = find_game_extended(&pool, game.id).await.unwrap().unwrap();

        // A second "finish" write (e.g. a retry) must not re-rate the game -
        // the idempotency guard trips because rating_change is already set.
        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "final_state",
            "final_state_2",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge_after_first.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (rating_after_second, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        assert_eq!(rating_after_first, rating_after_second);
    }

    #[sqlx::test]
    async fn two_player_game_with_bot_is_not_rated(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 1, &[0]).await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge
            .game_players
            .iter()
            .find(|p| p.user.is_some())
            .unwrap()
            .game_player
            .id;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings: vec![1, 2],
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        for p in &ge_after.game_players {
            assert_eq!(
                p.game_player.rating_change, None,
                "with only one human player, no pairwise rating is possible"
            );
        }
    }

    #[sqlx::test]
    async fn three_player_game_with_bot_rates_humans_only(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 1, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.user.is_none())
            .unwrap()
            .game_player
            .position as usize;

        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;
        placings[bot_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let creator_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let opponent_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;
        let bot_change = find_rating_change(&pool, game.id, bot_pos as i32).await;

        assert!(creator_change.is_some());
        assert!(opponent_change.is_some());
        assert_eq!(bot_change, None);
        assert_eq!(creator_change.unwrap() + opponent_change.unwrap(), 0);

        let (creator_rating, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (opponent_rating, _) = game_type_rating(&pool, game_type_id, opponent.id).await;
        assert_eq!(creator_rating, 1200 + creator_change.unwrap());
        assert_eq!(opponent_rating, 1200 + opponent_change.unwrap());
    }

    #[sqlx::test]
    async fn game_type_users_row_created_on_first_rated_game(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;

        // Explicitly delete the game_type_users rows that create_game_with_users
        // auto-created, so the finish path must INSERT them itself.
        sqlx::query!(
            "DELETE FROM game_type_users WHERE game_type_id = $1",
            game_type_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;

        let mut placings = vec![0usize; 2];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let (winner_rating, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (loser_rating, _) = game_type_rating(&pool, game_type_id, opponent.id).await;
        // DB column default rating is 1200, so the newly-created rows started
        // there before the change was applied.
        assert_eq!(winner_rating, 1216);
        assert_eq!(loser_rating, 1184);
    }

    #[sqlx::test]
    async fn concede_game_assigns_places_and_rates(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 0, &[0])
                .await;
        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceding = ge
            .game_players
            .iter()
            .find(|p| p.game_player.position == 0)
            .unwrap();
        let conceding_id = conceding.game_player.id;

        concede_game(&pool, game.id, conceding_id, "creator")
            .await
            .unwrap();

        let ge_after = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let conceder = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id == conceding_id)
            .unwrap();
        let non_conceder = ge_after
            .game_players
            .iter()
            .find(|p| p.game_player.id != conceding_id)
            .unwrap();
        assert_eq!(conceder.game_player.place, Some(2));
        assert_eq!(non_conceder.game_player.place, Some(1));
        assert_eq!(conceder.game_player.rating_change, Some(-16));
        assert_eq!(non_conceder.game_player.rating_change, Some(16));

        let (non_conceder_rating, _) =
            game_type_rating(&pool, game_type_id, non_conceder.user.as_ref().unwrap().id).await;
        assert_eq!(non_conceder_rating, 1216);
    }

    #[sqlx::test]
    async fn finishing_a_game_captures_rating_before(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let opponent = make_user(&pool, "opponent").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;
        let game =
            make_game_with_players(&pool, game_version_id, creator.id, &[opponent.id], 1, &[0])
                .await;

        sqlx::query(
            "UPDATE game_type_users SET rating = 1300 WHERE game_type_id = $1 AND user_id = $2",
        )
        .bind(game_type_id)
        .bind(creator.id)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "UPDATE game_type_users SET rating = 1100 WHERE game_type_id = $1 AND user_id = $2",
        )
        .bind(game_type_id)
        .bind(opponent.id)
        .execute(&pool)
        .await
        .unwrap();

        let ge = find_game_extended(&pool, game.id).await.unwrap().unwrap();
        let played_player_id = ge.game_players[0].game_player.id;
        let creator_pos = position_of(&ge, creator.id) as usize;
        let opponent_pos = position_of(&ge, opponent.id) as usize;
        let bot_pos = ge
            .game_players
            .iter()
            .find(|p| p.user.is_none())
            .unwrap()
            .game_player
            .position as usize;

        let mut placings = vec![0usize; 3];
        placings[creator_pos] = 1;
        placings[opponent_pos] = 2;
        placings[bot_pos] = 3;

        update_game_command_success(
            &pool,
            game.id,
            played_player_id,
            "prev_state",
            "final_state",
            false,
            &StatusUpdate {
                is_finished: true,
                whose_turn: vec![],
                eliminated: vec![],
                placings,
            },
            &[],
            ge.game.updated_at,
            vec![],
        )
        .await
        .unwrap();

        let creator_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(creator_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(creator_rb, Some(1300));

        let opponent_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(opponent_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(opponent_rb, Some(1100));

        let bot_rb: Option<i32> = sqlx::query_scalar(
            "SELECT rating_before FROM game_players WHERE game_id = $1 AND position = $2",
        )
        .bind(game.id)
        .bind(bot_pos as i32)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(bot_rb, None);

        let creator_change = find_rating_change(&pool, game.id, creator_pos as i32).await;
        let opponent_change = find_rating_change(&pool, game.id, opponent_pos as i32).await;

        let (creator_rating_after, _) = game_type_rating(&pool, game_type_id, creator.id).await;
        let (opponent_rating_after, _) = game_type_rating(&pool, game_type_id, opponent.id).await;

        assert_eq!(
            creator_rb.unwrap() + creator_change.unwrap(),
            creator_rating_after
        );
        assert_eq!(
            opponent_rb.unwrap() + opponent_change.unwrap(),
            opponent_rating_after
        );
    }

    #[sqlx::test]
    async fn user_theme_defaults_none_and_round_trips(pool: PgPool) {
        let user = make_user(&pool, "themed").await;

        assert_eq!(get_user_theme(&pool, user.id).await.unwrap(), None);

        set_user_theme(&pool, user.id, Some("dracula"))
            .await
            .unwrap();
        assert_eq!(
            get_user_theme(&pool, user.id).await.unwrap(),
            Some("dracula".to_string())
        );

        set_user_theme(&pool, user.id, None).await.unwrap();
        assert_eq!(get_user_theme(&pool, user.id).await.unwrap(), None);
    }

    // --- choose_colors ---

    const PALETTE: [&str; 8] = [
        "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
    ];

    #[test]
    fn choose_colors_honours_preference() {
        let prefs = vec![vec!["Blue".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Blue".to_string()]);
    }

    #[test]
    fn choose_colors_same_rank_conflict_resolves_distinctly() {
        // Both players want Blue as their first pref; only one can have it,
        // the other falls back to a leftover palette color. All distinct.
        let prefs = vec![vec!["Blue".to_string()], vec!["Blue".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result.len(), 2);
        assert_ne!(result[0], result[1]);
        assert!(result.contains(&"Blue".to_string()));
        for c in &result {
            assert!(PALETTE.contains(&c.as_str()));
        }
    }

    #[test]
    fn choose_colors_normalizes_legacy_amber_to_orange() {
        let prefs = vec![vec!["Amber".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Orange".to_string()]);
    }

    #[test]
    fn choose_colors_normalizes_legacy_bluegrey_to_cyan() {
        let prefs = vec![vec!["BlueGrey".to_string()]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(result, vec!["Cyan".to_string()]);
    }

    #[test]
    fn choose_colors_no_prefs_fills_from_palette_order() {
        let prefs = vec![vec![], vec![], vec![]];
        let result = choose_colors(&prefs, &PALETTE);
        assert_eq!(
            result,
            vec!["Green".to_string(), "Red".to_string(), "Blue".to_string()]
        );
    }

    // --- delete_game (#34 force delete, spec D3) ---

    #[sqlx::test]
    async fn delete_game_removes_all_dependent_rows(pool: PgPool) {
        let user = make_user(&pool, "deleter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, user.id, &[], 1, &[0]).await;

        // A log targeted at the human player, so game_log_targets is exercised.
        let log_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at)
             VALUES ($1, 'hello', false, timezone('utc', now())) RETURNING id",
            game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let player_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2",
            game.id,
            user.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
            log_id,
            player_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, game.id).await.unwrap();
        assert!(deleted);

        for (table, count) in [
            ("games", count_rows(&pool, "games").await),
            ("game_players", count_rows(&pool, "game_players").await),
            ("game_bots", count_rows(&pool, "game_bots").await),
            ("game_logs", count_rows(&pool, "game_logs").await),
            (
                "game_log_targets",
                count_rows(&pool, "game_log_targets").await,
            ),
        ] {
            assert_eq!(count, 0, "expected no rows left in {}", table);
        }
        // The user survives the delete.
        assert_eq!(count_rows(&pool, "users").await, 1);
    }

    #[sqlx::test]
    async fn delete_game_nulls_restarted_game_id_references(pool: PgPool) {
        let user = make_user(&pool, "restarter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let old_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[]).await;
        let new_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[0]).await;
        sqlx::query!(
            "UPDATE games SET restarted_game_id = $1 WHERE id = $2",
            new_game.id,
            old_game.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, new_game.id).await.unwrap();
        assert!(deleted);

        let restarted: Option<Uuid> = sqlx::query_scalar!(
            "SELECT restarted_game_id FROM games WHERE id = $1",
            old_game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(restarted, None);
    }

    #[sqlx::test]
    async fn delete_game_returns_false_for_missing_game(pool: PgPool) {
        let deleted = delete_game(&pool, Uuid::new_v4()).await.unwrap();
        assert!(!deleted);
    }

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
        assert_eq!(
            search_users(&pool, me.id, "player").await.unwrap().len(),
            10
        );

        // The searching user is never in their own results.
        assert!(
            search_users(&pool, me.id, "search")
                .await
                .unwrap()
                .is_empty()
        );

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

    #[sqlx::test]
    async fn search_users_excludes_blocked_in_either_direction(pool: PgPool) {
        let me = make_user(&pool, "searcher").await;
        let i_block = make_user(&pool, "player_iblock").await;
        let blocks_me = make_user(&pool, "player_blocksme").await;
        make_user(&pool, "player_open").await;
        block_user(&pool, me.id, i_block.id).await.unwrap();
        block_user(&pool, blocks_me.id, me.id).await.unwrap();

        let hits = search_users(&pool, me.id, "player").await.unwrap();
        let names: Vec<String> = hits.into_iter().map(|(_, n)| n).collect();
        assert!(!names.contains(&"player_iblock".to_string()));
        assert!(!names.contains(&"player_blocksme".to_string()));
        assert!(names.contains(&"player_open".to_string()));
    }

    async fn count_rows(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[test]
    fn can_remove_email_only_non_primary() {
        assert!(can_remove_email(false));
        assert!(!can_remove_email(true));
    }

    #[test]
    fn can_switch_to_email_requires_verified() {
        use time::{Date, Month, PrimitiveDateTime, Time};
        let now = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        );
        assert!(can_switch_to_email(Some(now)));
        assert!(!can_switch_to_email(None));
    }

    #[test]
    fn is_expired_unverified_truth_table() {
        use time::{Date, Month, PrimitiveDateTime, Time};
        let now = PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::July, 20).unwrap(),
            Time::from_hms(12, 0, 0).unwrap(),
        );
        let old = now - time::Duration::hours(25);
        let recent = now - time::Duration::hours(1);
        let day = std::time::Duration::from_secs(86400);
        assert!(is_expired_unverified(None, old, now, day));
        assert!(!is_expired_unverified(None, recent, now, day));
        assert!(
            !is_expired_unverified(Some(now), old, now, day),
            "verified never expires"
        );
        assert!(
            is_expired_unverified(None, now - time::Duration::hours(24), now, day),
            "boundary is inclusive"
        );
    }

    #[test]
    fn cap_digest_truncates_to_cap() {
        let items: Vec<i32> = (0..25).collect();
        let capped = cap_digest(items, SWITCH_DIGEST_CAP);
        assert_eq!(capped.len(), SWITCH_DIGEST_CAP);
        assert_eq!(capped[0], 0);
        assert_eq!(
            capped[SWITCH_DIGEST_CAP - 1],
            (SWITCH_DIGEST_CAP - 1) as i32
        );
        let small: Vec<i32> = vec![1, 2, 3];
        assert_eq!(cap_digest(small, SWITCH_DIGEST_CAP).len(), 3);
    }

    #[sqlx::test]
    async fn add_unverified_then_confirm_sets_verified(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let email = format!("add-{}@example.com", Uuid::new_v4());
        insert_unverified_email(&pool, user_id, &email)
            .await
            .unwrap()
            .unwrap();
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        let row = rows.iter().find(|r| r.email == email).unwrap();
        assert!(!row.is_primary);
        assert!(row.verified_at.is_none());

        assert!(mark_email_verified(&pool, user_id, &email).await.unwrap());
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        assert!(
            rows.iter()
                .find(|r| r.email == email)
                .unwrap()
                .verified_at
                .is_some()
        );
        assert!(
            !mark_email_verified(&pool, user_id, &email).await.unwrap(),
            "second mark is a no-op"
        );
    }

    #[sqlx::test]
    async fn set_primary_keeps_exactly_one_primary(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let primary = format!("p-{}@example.com", Uuid::new_v4());
        let secondary = format!("s-{}@example.com", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(&primary)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NOW())",
        )
        .bind(user_id)
        .bind(&secondary)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            set_primary_email(&pool, user_id, &secondary).await.unwrap(),
            SetPrimaryOutcome::Switched
        );
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        let primaries: Vec<_> = rows.iter().filter(|r| r.is_primary).collect();
        assert_eq!(primaries.len(), 1);
        assert_eq!(primaries[0].email, secondary);
    }

    #[sqlx::test]
    async fn set_primary_rejects_unverified_and_unknown(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let unverified = format!("uv-{}@example.com", Uuid::new_v4());
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind(&unverified)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            set_primary_email(&pool, user_id, &unverified)
                .await
                .unwrap(),
            SetPrimaryOutcome::Unverified
        );
        assert_eq!(
            set_primary_email(&pool, user_id, "missing@example.com")
                .await
                .unwrap(),
            SetPrimaryOutcome::NotFound
        );
    }

    #[sqlx::test]
    async fn remove_primary_rejected_non_primary_works(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let primary = format!("p-{}@example.com", Uuid::new_v4());
        let secondary = format!("s-{}@example.com", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, true, NOW())",
        )
        .bind(user_id)
        .bind(&primary)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at) VALUES ($1, $2, false, NOW())",
        )
        .bind(user_id)
        .bind(&secondary)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            remove_user_email(&pool, user_id, &primary).await.unwrap(),
            RemoveEmailOutcome::IsPrimary
        );
        assert_eq!(
            remove_user_email(&pool, user_id, &secondary).await.unwrap(),
            RemoveEmailOutcome::Removed
        );
        assert_eq!(
            remove_user_email(&pool, user_id, &secondary).await.unwrap(),
            RemoveEmailOutcome::NotFound
        );
    }

    #[sqlx::test]
    async fn insert_unverified_rejects_globally_taken_email(pool: PgPool) {
        let u1: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let u2: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let email = format!("shared-{}@example.com", Uuid::new_v4());
        insert_unverified_email(&pool, u1, &email)
            .await
            .unwrap()
            .unwrap();
        assert!(
            insert_unverified_email(&pool, u2, &email)
                .await
                .unwrap()
                .is_none(),
            "global UNIQUE(email)"
        );
        assert_eq!(find_email_owner(&pool, &email).await.unwrap(), Some(u1));
    }

    #[sqlx::test]
    async fn expiry_cleanup_deletes_only_expired_unverified(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();
        let expired = format!("exp-{}@example.com", Uuid::new_v4());
        let fresh = format!("fresh-{}@example.com", Uuid::new_v4());
        let verified_old = format!("vold-{}@example.com", Uuid::new_v4());
        // expired: unverified, created 48h ago
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, created_at) VALUES ($1, $2, false, NOW() - interval '48 hours')",
        )
        .bind(user_id)
        .bind(&expired)
        .execute(&pool)
        .await
        .unwrap();
        // fresh: unverified, created now
        sqlx::query("INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, false)")
            .bind(user_id)
            .bind(&fresh)
            .execute(&pool)
            .await
            .unwrap();
        // verified but old: must survive
        sqlx::query(
            "INSERT INTO user_emails (user_id, email, is_primary, verified_at, created_at) VALUES ($1, $2, true, NOW(), NOW() - interval '48 hours')",
        )
        .bind(user_id)
        .bind(&verified_old)
        .execute(&pool)
        .await
        .unwrap();

        let deleted =
            delete_expired_unverified_emails(&pool, std::time::Duration::from_secs(86400))
                .await
                .unwrap();
        assert_eq!(deleted, 1);
        let rows = list_user_emails(&pool, user_id).await.unwrap();
        assert!(rows.iter().any(|r| r.email == fresh));
        assert!(rows.iter().any(|r| r.email == verified_old));
        assert!(!rows.iter().any(|r| r.email == expired));
    }

    #[sqlx::test]
    async fn email_prefs_default_all_true(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        let (turn, invite, reminder) = get_user_email_prefs(&pool, user_id).await.unwrap();
        assert!(turn);
        assert!(invite);
        assert!(reminder);
    }

    #[sqlx::test]
    async fn set_email_prefs_toggles(pool: PgPool) {
        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        )
        .bind(format!("u-{}", Uuid::new_v4()))
        .bind(Vec::<String>::new())
        .fetch_one(&pool)
        .await
        .unwrap();

        set_user_turn_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        set_user_invite_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        set_user_reminder_emails_enabled(&pool, user_id, false)
            .await
            .unwrap();
        assert_eq!(
            get_user_email_prefs(&pool, user_id).await.unwrap(),
            (false, false, false)
        );

        set_user_turn_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        set_user_invite_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        set_user_reminder_emails_enabled(&pool, user_id, true)
            .await
            .unwrap();
        assert_eq!(
            get_user_email_prefs(&pool, user_id).await.unwrap(),
            (true, true, true)
        );
    }
}
