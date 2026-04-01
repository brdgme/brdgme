#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use uuid::Uuid;

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
    match (id, created_at, updated_at, game_type_id, user_id, rating, peak_rating) {
        (Some(id), Some(created_at), Some(updated_at), Some(game_type_id), Some(user_id), Some(rating), Some(peak_rating)) => {
            crate::models::game::GameTypeUser {
                id,
                created_at,
                updated_at,
                game_type_id,
                user_id,
                last_game_finished_at,
                rating,
                peak_rating,
            }
        }
        _ => crate::models::game::GameTypeUser {
            id: Uuid::nil(),
            created_at: default_ts,
            updated_at: default_ts,
            game_type_id: default_game_type_id,
            user_id: default_user_id.unwrap_or(Uuid::nil()),
            last_game_finished_at: None,
            rating: 1500,
            peak_rating: 1500,
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn create_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

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
pub async fn find_game_version(pool: &PgPool, id: Uuid) -> Result<Option<crate::models::game::GameVersion>> {
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
) -> Result<Vec<(crate::models::game::GameType, Vec<crate::models::game::GameVersion>)>> {
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
            let gv: Vec<_> = versions.iter().filter(|v| v.game_type_id == gt.id).cloned().collect();
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

    let game_version = find_game_version(pool, game.game_version_id).await?
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
            p.gtu_id, p.gtu_created_at, p.gtu_updated_at,
            p.gtu_game_type_id, p.gtu_user_id, p.gtu_last_game_finished_at,
            p.gtu_rating, p.gtu_peak_rating,
            p.u_id, game_version.game_type_id, p.gp_created_at,
        );
        let user = p.u_id.map(|id| crate::models::user::User {
            id,
            created_at: p.u_created_at.unwrap(),
            updated_at: p.u_updated_at.unwrap(),
            name: p.u_name.unwrap(),
            pref_colors: p.u_pref_colors.unwrap(),
            login_confirmation: p.u_login_confirmation,
            login_confirmation_at: p.u_login_confirmation_at,
        });
        let game_bot = p.gb_id.map(|id| crate::models::game::GameBot {
            id,
            game_id: p.gb_game_id.unwrap(),
            name: p.gb_name.unwrap(),
            difficulty: p.gb_difficulty.unwrap(),
        });

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
pub async fn find_active_games_for_user(user_id: &Uuid, pool: &PgPool) -> Result<Vec<GameExtended>> {
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
            row.gtu_id, row.gtu_created_at, row.gtu_updated_at,
            row.gtu_game_type_id, row.gtu_user_id, row.gtu_last_game_finished_at,
            row.gtu_rating, row.gtu_peak_rating,
            row.u_id, row.game_type_id, row.gp_created_at,
        );
        let user = row.u_id.map(|id| crate::models::user::User {
            id,
            created_at: row.u_created_at.unwrap(),
            updated_at: row.u_updated_at.unwrap(),
            name: row.u_name.unwrap(),
            pref_colors: row.u_pref_colors.unwrap(),
            login_confirmation: row.u_login_confirmation,
            login_confirmation_at: row.u_login_confirmation_at,
        });
        let game_bot = row.gb_id.map(|id| crate::models::game::GameBot {
            id,
            game_id: row.gb_game_id.unwrap(),
            name: row.gb_name.unwrap(),
            difficulty: row.gb_difficulty.unwrap(),
        });

        games.last_mut().unwrap().game_players.push(GamePlayerExtended {
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
    pub chat_id: Option<Uuid>,
    pub game_state: &'a str,
}

#[cfg(feature = "ssr")]
pub async fn create_game_with_users(
    pool: &PgPool,
    opts: CreateGameOpts<'_>,
) -> Result<crate::models::game::Game> {
    let mut tx = pool.begin().await?;

    // 1. Find or create users
    let mut users = Vec::new();
    
    // Creator
    let creator = sqlx::query_as!(
        crate::models::user::User,
        "SELECT * FROM users WHERE id = $1",
        opts.creator_id
    )
    .fetch_one(&mut *tx)
    .await?;
    users.push(creator);

    // Opponent IDs
    for &id in opts.opponent_ids {
        let opponent = sqlx::query_as!(
            crate::models::user::User,
            "SELECT * FROM users WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        users.push(opponent);
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
        users.push(user);
    }

    // 2. Randomize player order
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        users.shuffle(&mut rng);
    }

    // 3. Assign colors
    let colors = vec!["Green", "Red", "Blue", "Amber", "Purple", "Brown", "BlueGrey"];
    
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
    let game_type_id = find_game_version(pool, opts.game_version_id).await?
        .ok_or_else(|| anyhow::anyhow!("Game version not found"))?
        .game_type_id;

    for (pos, user) in users.iter().enumerate() {
        let color = colors.get(pos).unwrap_or(&"BlueGrey").to_string();
        let is_turn = opts.whose_turn.contains(&pos);
        let is_eliminated = opts.eliminated.contains(&pos);
        let place = opts.placings.get(pos).map(|&p| p as i32);

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

    tx.commit().await?;
    Ok(game)
}

#[cfg(feature = "ssr")]
pub async fn create_game_logs(
    pool: &PgPool,
    game_id: Uuid,
    logs: Vec<brdgme_cmd::api::CliLog>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    
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
) -> Result<()> {
    let now = {
        let t = time::OffsetDateTime::now_utc();
        time::PrimitiveDateTime::new(t.date(), t.time())
    };
    let finished_at: Option<time::PrimitiveDateTime> = if is_finished { Some(now) } else { None };

    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET game_state = $1, is_finished = $2, finished_at = COALESCE($3, finished_at), updated_at = NOW() WHERE id = $4",
        new_game_state,
        is_finished,
        finished_at,
        game_id
    )
    .execute(&mut *tx)
    .await?;

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
        let undo_game_state: Option<&str> = if is_played && can_undo { Some(prev_game_state) } else { None };

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

    tx.commit().await?;
    Ok(())
}