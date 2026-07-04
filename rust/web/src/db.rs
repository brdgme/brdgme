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
    login_confirmation: Option<String>,
    login_confirmation_at: Option<time::PrimitiveDateTime>,
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
        login_confirmation,
        login_confirmation_at,
    }))
}

#[cfg(feature = "ssr")]
fn build_game_bot_from_row(
    id: Option<Uuid>,
    game_id: Option<Uuid>,
    name: Option<String>,
    difficulty: Option<String>,
) -> Result<Option<crate::models::game::GameBot>> {
    let Some(id) = id else { return Ok(None) };
    Ok(Some(crate::models::game::GameBot {
        id,
        game_id: game_id
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: game_id missing from LEFT JOIN row"))?,
        name: name
            .ok_or_else(|| anyhow::anyhow!("game_bot {id}: name missing from LEFT JOIN row"))?,
        difficulty: difficulty.ok_or_else(|| {
            anyhow::anyhow!("game_bot {id}: difficulty missing from LEFT JOIN row")
        })?,
    }))
}

#[cfg(feature = "ssr")]
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
            rating: 1500,
            peak_rating: 1500,
        },
    }
}

#[cfg(feature = "ssr")]
pub async fn create_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await?;

    Ok(pool)
}

#[cfg(feature = "ssr")]
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.login_confirmation, u.login_confirmation_at
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
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, created_at, updated_at, name, pref_colors, login_confirmation, login_confirmation_at
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
        "SELECT id, created_at, updated_at, name, player_counts, weight FROM game_types ORDER BY name"
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
        "SELECT id, created_at, updated_at, name, player_counts, weight FROM game_types WHERE id = $1",
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
            u.login_confirmation as "u_login_confirmation?", u.login_confirmation_at as "u_login_confirmation_at?",
            gtu.id as "gtu_id?", gtu.created_at as "gtu_created_at?", gtu.updated_at as "gtu_updated_at?",
            gtu.game_type_id as "gtu_game_type_id?", gtu.user_id as "gtu_user_id?",
            gtu.last_game_finished_at as "gtu_last_game_finished_at?", gtu.rating as "gtu_rating?",
            gtu.peak_rating as "gtu_peak_rating?",
            gb.id as "gb_id?", gb.game_id as "gb_game_id?", gb.name as "gb_name?",
            gb.difficulty as "gb_difficulty?"
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
            p.u_login_confirmation,
            p.u_login_confirmation_at,
        )?;
        let game_bot = build_game_bot_from_row(p.gb_id, p.gb_game_id, p.gb_name, p.gb_difficulty)?;

        game_players.push(GamePlayerExtended {
            game_player: crate::models::game::GamePlayer {
                id: p.gp_id,
                created_at: p.gp_created_at,
                updated_at: p.gp_updated_at,
                game_id: p.gp_game_id,
                user_id: p.gp_user_id,
                position: p.gp_position,
                color: p.gp_color,
                has_accepted: p.gp_has_accepted,
                is_turn: p.gp_is_turn,
                is_turn_at: p.gp_is_turn_at,
                place: p.gp_place,
                last_turn_at: p.gp_last_turn_at,
                is_eliminated: p.gp_is_eliminated,
                is_read: p.gp_is_read,
                points: p.gp_points,
                undo_game_state: p.gp_undo_game_state,
                rating_change: p.gp_rating_change,
            },
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
pub async fn find_active_games_for_user(
    user_id: &Uuid,
    pool: &PgPool,
) -> Result<Vec<GameExtended>> {
    // Fetch all active games for this user in a single query joining all required tables.
    let rows = sqlx::query!(
        r#"
        SELECT
            g.id as g_id, g.created_at as g_created_at, g.updated_at as g_updated_at,
            g.game_version_id, g.is_finished, g.finished_at, g.game_state,
            g.chat_id, g.restarted_game_id,
            gv.id as gv_id, gv.created_at as gv_created_at, gv.updated_at as gv_updated_at,
            gv.game_type_id, gv.name as gv_name, gv.uri, gv.is_public, gv.is_deprecated,
            gt.id as gt_id, gt.created_at as gt_created_at, gt.updated_at as gt_updated_at,
            gt.name as gt_name, gt.player_counts, gt.weight,
            gp.id as gp_id, gp.created_at as gp_created_at, gp.updated_at as gp_updated_at,
            gp.game_id as gp_game_id, gp.user_id as gp_user_id, gp.position as gp_position,
            gp.color as gp_color, gp.has_accepted as gp_has_accepted, gp.is_turn as gp_is_turn,
            gp.is_turn_at as gp_is_turn_at, gp.place as gp_place,
            gp.last_turn_at as gp_last_turn_at, gp.is_eliminated as gp_is_eliminated,
            gp.is_read as gp_is_read, gp.points as gp_points,
            gp.undo_game_state as gp_undo_game_state, gp.rating_change as gp_rating_change,
            u.id as "u_id?", u.created_at as "u_created_at?", u.updated_at as "u_updated_at?",
            u.name as "u_name?", u.pref_colors as "u_pref_colors?",
            u.login_confirmation as "u_login_confirmation?",
            u.login_confirmation_at as "u_login_confirmation_at?",
            gtu.id as "gtu_id?", gtu.created_at as "gtu_created_at?",
            gtu.updated_at as "gtu_updated_at?", gtu.game_type_id as "gtu_game_type_id?",
            gtu.user_id as "gtu_user_id?", gtu.last_game_finished_at as "gtu_last_game_finished_at?",
            gtu.rating as "gtu_rating?", gtu.peak_rating as "gtu_peak_rating?",
            gb.id as "gb_id?", gb.game_id as "gb_game_id?", gb.name as "gb_name?",
            gb.difficulty as "gb_difficulty?"
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        JOIN game_players gp ON gp.game_id = g.id
        LEFT JOIN users u ON u.id = gp.user_id
        LEFT JOIN game_type_users gtu ON gtu.user_id = u.id AND gtu.game_type_id = gv.game_type_id
        LEFT JOIN game_bots gb ON gp.game_bot_id = gb.id
        WHERE g.is_finished = false
          AND g.id IN (
              SELECT game_id FROM game_players WHERE user_id = $1
          )
        ORDER BY g.id, gp.position
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // Group rows by game_id, building GameExtended structs.
    let mut games: Vec<GameExtended> = Vec::new();
    for row in rows {
        let game_id = row.g_id;
        if games.last().map(|g| g.game.id) != Some(game_id) {
            games.push(GameExtended {
                game: crate::models::game::Game {
                    id: row.g_id,
                    created_at: row.g_created_at,
                    updated_at: row.g_updated_at,
                    game_version_id: row.game_version_id,
                    is_finished: row.is_finished,
                    finished_at: row.finished_at,
                    game_state: row.game_state.clone(),
                    chat_id: row.chat_id,
                    restarted_game_id: row.restarted_game_id,
                },
                game_type: crate::models::game::GameType {
                    id: row.gt_id,
                    created_at: row.gt_created_at,
                    updated_at: row.gt_updated_at,
                    name: row.gt_name.clone(),
                    player_counts: row.player_counts.clone(),
                    weight: row.weight,
                },
                game_version: crate::models::game::GameVersion {
                    id: row.gv_id,
                    created_at: row.gv_created_at,
                    updated_at: row.gv_updated_at,
                    game_type_id: row.game_type_id,
                    name: row.gv_name.clone(),
                    uri: row.uri.clone(),
                    is_public: row.is_public,
                    is_deprecated: row.is_deprecated,
                },
                game_players: Vec::new(),
            });
        }

        let gtu = build_game_type_user(
            row.gtu_id,
            row.gtu_created_at,
            row.gtu_updated_at,
            row.gtu_game_type_id,
            row.gtu_user_id,
            row.gtu_last_game_finished_at,
            row.gtu_rating,
            row.gtu_peak_rating,
            row.u_id,
            row.game_type_id,
            row.gp_created_at,
        );
        let user = build_user_from_row(
            row.u_id,
            row.u_created_at,
            row.u_updated_at,
            row.u_name,
            row.u_pref_colors,
            row.u_login_confirmation,
            row.u_login_confirmation_at,
        )?;
        let game_bot =
            build_game_bot_from_row(row.gb_id, row.gb_game_id, row.gb_name, row.gb_difficulty)?;

        let game = games.last_mut().ok_or_else(|| {
            anyhow::anyhow!("game_players row for game {game_id} encountered before its game row")
        })?;
        game.game_players.push(GamePlayerExtended {
            game_player: crate::models::game::GamePlayer {
                id: row.gp_id,
                created_at: row.gp_created_at,
                updated_at: row.gp_updated_at,
                game_id: row.gp_game_id,
                user_id: row.gp_user_id,
                position: row.gp_position,
                color: row.gp_color,
                has_accepted: row.gp_has_accepted,
                is_turn: row.gp_is_turn,
                is_turn_at: row.gp_is_turn_at,
                place: row.gp_place,
                last_turn_at: row.gp_last_turn_at,
                is_eliminated: row.gp_is_eliminated,
                is_read: row.gp_is_read,
                points: row.gp_points,
                undo_game_state: row.gp_undo_game_state,
                rating_change: row.gp_rating_change,
            },
            user,
            game_bot,
            game_type_user: gtu,
        });
    }

    Ok(games)
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
}

#[cfg(feature = "ssr")]
enum PlayerSlotInternal {
    User(User),
    Bot { name: String, difficulty: String },
}

#[cfg(feature = "ssr")]
pub async fn create_game_with_users(
    pool: &PgPool,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    let mut tx = pool.begin().await?;
    let game = create_game_with_users_tx(pool, &mut tx, opts).await?;
    tx.commit().await?;
    Ok(game)
}

/// Creates a game and its players within an existing transaction, so callers
/// can commit them atomically alongside other writes (e.g. the restarted-game
/// linkage in `restart_game`).
#[cfg(feature = "ssr")]
pub async fn create_game_with_users_tx(
    pool: &PgPool,
    tx: &mut sqlx::PgConnection,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    // 1. Find or create users; collect all slots (users + bots)
    let mut slots: Vec<PlayerSlotInternal> = Vec::new();

    // Creator
    let creator = sqlx::query_as!(
        crate::models::user::User,
        "SELECT * FROM users WHERE id = $1",
        opts.creator_id
    )
    .fetch_one(&mut *tx)
    .await?;
    slots.push(PlayerSlotInternal::User(creator));

    // Opponent IDs
    for &id in opts.opponent_ids {
        let opponent = sqlx::query_as!(
            crate::models::user::User,
            "SELECT * FROM users WHERE id = $1",
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
            r#"SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.login_confirmation, u.login_confirmation_at
               FROM users u JOIN user_emails ue ON u.id = ue.user_id WHERE ue.email = $1"#,
            email
        ).fetch_optional(&mut *tx).await? {
            u
        } else {
            // Create new user for email
            let new_user_id = Uuid::new_v4();
            let username = email.split('@').next().unwrap_or("user").to_string();

            let u = sqlx::query_as!(
                crate::models::user::User,
                "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING *",
                new_user_id,
                username,
                &Vec::<String>::new()
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
                new_user_id,
                email
            )
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
            difficulty: bot.difficulty.clone(),
        });
    }

    // 2. Randomize player order
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        slots.shuffle(&mut rng);
    }

    // 3. Assign colors
    let colors = vec![
        "Green", "Red", "Blue", "Amber", "Purple", "Brown", "BlueGrey",
    ];

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
    let game_type_id = find_game_version(pool, opts.game_version_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Game version not found"))?
        .game_type_id;

    for (pos, slot) in slots.iter().enumerate() {
        let color = colors.get(pos).unwrap_or(&"BlueGrey").to_string();
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
                    user.id == opts.creator_id,
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
            PlayerSlotInternal::Bot { name, difficulty } => {
                let bot_id = sqlx::query_scalar!(
                    "INSERT INTO game_bots (game_id, name, difficulty) VALUES ($1, $2, $3) RETURNING id",
                    game.id,
                    name,
                    difficulty
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

    for p in &players {
        let place: i32 = if p.id == conceding_player_id { 2 } else { 1 };
        sqlx::query!(
            r#"UPDATE game_players
               SET is_turn = false, place = $1, undo_game_state = NULL, updated_at = NOW()
               WHERE id = $2"#,
            place,
            p.id
        )
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

#[cfg(feature = "ssr")]
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
pub async fn undo_game(
    pool: &PgPool,
    game_id: Uuid,
    undo_state: &str,
    player_position: usize,
    whose_turn: &[usize],
    eliminated: &[usize],
    placings: &[usize],
) -> Result<()> {
    let is_finished = !placings.is_empty();
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = NULL, updated_at = NOW() WHERE id = $3",
        undo_state,
        is_finished,
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
        let is_turn = whose_turn.contains(&pos);
        let is_eliminated = eliminated.contains(&pos);
        let place: Option<i32> = placings.get(pos).map(|&pl| pl as i32);

        sqlx::query!(
            r#"UPDATE game_players
               SET is_turn = $1, is_eliminated = $2, place = $3, undo_game_state = NULL, updated_at = NOW()
               WHERE id = $4"#,
            is_turn,
            is_eliminated,
            place,
            p.id
        )
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
/// has a rating_change) or if any player is a bot.
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
    if players.iter().any(|p| p.game_bot_id.is_some()) {
        // New rule (post-legacy): games with any bot player are never rated.
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
        sqlx::query!(
            "UPDATE game_players SET rating_change = $1 WHERE id = $2",
            change,
            p.id
        )
        .execute(&mut *tx)
        .await?;
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn update_game_command_success(
    pool: &PgPool,
    game_id: Uuid,
    played_player_id: Uuid,
    prev_game_state: &str,
    new_game_state: &str,
    can_undo: bool,
    is_finished: bool,
    whose_turn: &[usize],
    eliminated: &[usize],
    placings: &[usize],
    points: &[f32],
    expected_updated_at: time::PrimitiveDateTime,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let now = {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    };
    let finished_at: Option<time::PrimitiveDateTime> = if is_finished { Some(now) } else { None };

    let mut tx = pool.begin().await?;

    let update_result = sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = COALESCE($3, finished_at), updated_at = NOW() WHERE id = $4 AND updated_at = $5",
        new_game_state,
        is_finished,
        finished_at,
        game_id,
        expected_updated_at
    )
    .execute(&mut *tx)
    .await?;

    if update_result.rows_affected() == 0 {
        return Err(anyhow::anyhow!(
            "Game was updated by another action, please retry"
        ));
    }

    let players = sqlx::query!(
        "SELECT id, position, is_turn_at, last_turn_at FROM game_players WHERE game_id = $1",
        game_id
    )
    .fetch_all(&mut *tx)
    .await?;

    for p in players {
        let pos = p.position as usize;
        let is_turn = whose_turn.contains(&pos);
        let place = placings.get(pos).map(|&pl| pl as i32);
        let is_eliminated = eliminated.contains(&pos);
        let player_points = points.get(pos).copied();
        let is_turn_at = if is_turn { now } else { p.is_turn_at };
        let is_played = p.id == played_player_id;
        let last_turn_at = if is_played { now } else { p.last_turn_at };
        let undo_game_state: Option<&str> = if is_played && can_undo {
            Some(prev_game_state)
        } else {
            None
        };

        sqlx::query!(
            r#"UPDATE game_players
               SET is_turn = $1, place = $2, is_eliminated = $3, points = $4,
                   undo_game_state = $5, last_turn_at = $6, is_turn_at = $7,
                   updated_at = NOW()
               WHERE id = $8"#,
            is_turn,
            place,
            is_eliminated,
            player_points,
            undo_game_state,
            last_turn_at,
            is_turn_at,
            p.id
        )
        .execute(&mut *tx)
        .await?;
    }

    if is_finished && !placings.is_empty() {
        apply_rating_changes(&mut tx, game_id).await?;
    }

    insert_game_logs_tx(&mut tx, game_id, logs).await?;

    tx.commit().await?;
    Ok(())
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

    // --- Fixture helpers ---

    async fn make_user(pool: &PgPool, name: &str) -> User {
        sqlx::query_as!(
            User,
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3) RETURNING *",
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
                difficulty: "easy".to_string(),
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
            },
        )
        .await
        .unwrap()
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

    #[sqlx::test]
    async fn find_game_extended_missing_game_type_user_defaults_to_1500(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (game_type_id, game_version_id) = make_game_type_and_version(&pool).await;

        // Explicitly insert a game_type_users row for a *different* game type to
        // make sure the LEFT JOIN filter (game_type_id match) is respected, and
        // that a genuinely missing row still defaults correctly.
        let (_other_game_type_id, _) = make_game_type_and_version(&pool).await;

        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 0, &[0]).await;

        // create_game_with_users auto-creates a game_type_users row; delete it
        // to exercise the genuinely-missing-row default path in
        // build_game_type_user (rating/peak_rating default to 1500).
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
        assert_eq!(human.game_type_user.rating, 1500);
        assert_eq!(human.game_type_user.peak_rating, 1500);
        assert_eq!(human.game_type_user.game_type_id, game_type_id);
    }

    // --- 3. find_active_games_for_user ---

    #[sqlx::test]
    async fn find_active_games_for_user_groups_and_filters(pool: PgPool) {
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
            },
        )
        .await
        .unwrap();

        let games = find_active_games_for_user(&user.id, &pool).await.unwrap();
        let game_ids: Vec<Uuid> = games.iter().map(|g| g.game.id).collect();

        assert!(game_ids.contains(&game1.id));
        assert!(game_ids.contains(&game2.id));
        assert!(
            !game_ids.contains(&game3.id),
            "finished games must be excluded"
        );
        assert_eq!(games.len(), 2, "no duplicate/mis-grouped rows");

        for g in &games {
            // Exactly the two players we created for that game, correctly grouped.
            assert_eq!(g.game_players.len(), 2);
            let user_player = g
                .game_players
                .iter()
                .find(|p| p.user.as_ref().map(|u| u.id) == Some(user.id))
                .expect("user's own player row must be present in their grouped game");
            // Player order is randomized by `create_game_with_users`, so
            // check turn flags by position rather than assuming creator ==
            // position 0: `whose_turn: &[0]` marks position 0 active.
            let expected_turn = user_player.game_player.position == 0;
            assert_eq!(user_player.game_player.is_turn, expected_turn);
            assert!(!user_player.game_player.is_read);
        }

        // A user in no games gets an empty vec, not an error.
        let lonely = make_user(&pool, "lonely").await;
        let none = find_active_games_for_user(&lonely.id, &pool).await.unwrap();
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
            true,  // can_undo
            false, // is_finished -> Active
            &[1],  // whose_turn moves to position 1
            &[],
            &[],
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
        assert!(!p0.game_player.is_eliminated);
        assert_eq!(p0.game_player.points, Some(3.5));
        assert_eq!(p1.game_player.points, Some(1.5));
        // Only the played player gets undo state stashed.
        assert_eq!(
            p0.game_player.undo_game_state,
            Some("prev_state".to_string())
        );
        assert_eq!(p1.game_player.undo_game_state, None);
        // last_turn_at only bumped for the played player.
        assert!(p0.game_player.last_turn_at > played_player.game_player.last_turn_at);
        // is_turn_at bumped for whoever's turn it now is (p1).
        assert!(p1.game_player.is_turn_at >= played_player.game_player.is_turn_at);
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
            true, // is_finished -> Finished
            &[],
            &[],
            &[1, 2], // placings by position
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
            false, // is_finished = false -> finished_at param is None
            &[0],
            &[],
            &[],
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
            false,
            &[1],
            &[],
            &[],
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
            &[0],
            &[],
            &[],
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

    #[sqlx::test]
    async fn login_confirmation_token_expiry_boundary(pool: PgPool) {
        // NOTE: production expiry window (auth/server.rs confirm_login) is
        // 1 hour, not the 29/31 day window described in the plan - see report.
        let user = make_user(&pool, "auth-user").await;
        let now = {
            let t = time::OffsetDateTime::now_utc();
            time::PrimitiveDateTime::new(t.date(), t.time())
        };

        sqlx::query!(
            "UPDATE users SET login_confirmation = $1, login_confirmation_at = $2 WHERE id = $3",
            "123456",
            now - time::Duration::minutes(55),
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let valid = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE login_confirmation = $1 AND login_confirmation_at > NOW() - INTERVAL '1 hour'",
            "123456"
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(valid.is_some(), "token within the 1 hour window is valid");

        sqlx::query!(
            "UPDATE users SET login_confirmation_at = $1 WHERE id = $2",
            now - time::Duration::minutes(65),
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let expired = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE login_confirmation = $1 AND login_confirmation_at > NOW() - INTERVAL '1 hour'",
            "123456"
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(
            expired.is_none(),
            "token past the 1 hour window is rejected"
        );
    }

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
            true,
            &[],
            &[],
            &placings,
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
            true,
            &[],
            &[],
            &placings,
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
            true,
            &[],
            &[],
            &[1, 2],
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
            true,
            &[],
            &[],
            &[1, 2],
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
    async fn game_with_bot_player_is_not_rated(pool: PgPool) {
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
            true,
            &[],
            &[],
            &[1, 2],
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
                "no player in a game with a bot should be rated"
            );
        }
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
            true,
            &[],
            &[],
            &placings,
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
}
