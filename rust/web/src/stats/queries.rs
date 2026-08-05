use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub async fn get_profile_user(pool: &PgPool, name: &str) -> Result<Option<super::ProfileUser>> {
    let row = sqlx::query!(
        r#"SELECT id, name, pref_colors, created_at FROM users WHERE lower(name) = lower($1)"#,
        name
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| {
        let pref_colors: Vec<String> = row
            .pref_colors
            .iter()
            .map(|c| crate::db::normalize_pref_color(c))
            .collect();
        let pref_color = pref_colors.first().cloned();
        super::ProfileUser {
            user_id: row.id,
            name: row.name,
            pref_color,
            pref_colors,
            created_at: row.created_at,
        }
    }))
}

pub async fn find_game_type_name(pool: &PgPool, name: &str) -> Result<Option<String>> {
    let row = sqlx::query!(
        r#"SELECT name FROM game_types WHERE lower(name) = lower($1)"#,
        name
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| row.name))
}

#[derive(Debug, sqlx::FromRow)]
struct OverallTotalsRow {
    finished_games: i64,
    wins: i64,
}

pub async fn overall_totals(
    pool: &PgPool,
    user_id: Uuid,
    include_single_human: bool,
) -> Result<super::OverallTotals> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let row: OverallTotalsRow = sqlx::query_as(
        r#"
        SELECT
            count(*) AS finished_games,
            count(*) FILTER (WHERE gp.ranked_placing = 1) AS wins
        FROM game_players gp
        JOIN games g ON g.id = gp.game_id
        WHERE gp.user_id = $1
          AND g.is_finished = true
          AND gp.ranked_placing IS NOT NULL
          AND (
              SELECT count(*) FROM game_players gp2
              WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL
          ) >= CASE WHEN $2 THEN 1 ELSE 2 END
        "#,
    )
    .bind(user_id)
    .bind(include_single_human)
    .fetch_one(pool)
    .await?;

    let win_percent = if row.finished_games == 0 {
        0.0
    } else {
        row.wins as f64 * 100.0 / row.finished_games as f64
    };

    Ok(super::OverallTotals {
        finished_games: row.finished_games,
        wins: row.wins,
        win_percent,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct GameTypeStatsRow {
    game_type_name: String,
    games: i64,
    wins: i64,
    avg_place_percentile: Option<f64>,
    rating: Option<i32>,
    peak_rating: Option<i32>,
}

pub async fn game_type_stats(
    pool: &PgPool,
    user_id: Uuid,
    include_single_human: bool,
    game_type_name: Option<&str>,
) -> Result<Vec<super::GameTypeStats>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<GameTypeStatsRow> = sqlx::query_as(
        r#"
        WITH qualifying AS (
            SELECT
                gt.id AS game_type_id,
                gt.name AS game_type_name,
                gp.ranked_placing AS place,
                (SELECT count(*) FROM game_players gp2
                 WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL) AS n
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND gp.ranked_placing IS NOT NULL
              AND ($3::text IS NULL OR gt.name = $3)
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL AND gp3.ranked_placing IS NOT NULL
              ) >= CASE WHEN $2 THEN 1 ELSE 2 END
        ),
        agg AS (
            SELECT
                game_type_id,
                game_type_name,
                count(*) AS games,
                count(*) FILTER (WHERE place = 1) AS wins,
                avg((n - place)::float8 / (n - 1))
                    FILTER (WHERE place IS NOT NULL AND n >= 2) AS avg_place_percentile
            FROM qualifying
            GROUP BY game_type_id, game_type_name
        )
        SELECT
            COALESCE(agg.game_type_name, gt.name) AS game_type_name,
            COALESCE(agg.games, 0) AS games,
            COALESCE(agg.wins, 0) AS wins,
            agg.avg_place_percentile AS avg_place_percentile,
            gtu.rating AS rating,
            gtu.peak_rating AS peak_rating
        FROM agg
        FULL OUTER JOIN (
            SELECT gtu_f.game_type_id, gtu_f.rating, gtu_f.peak_rating
            FROM game_type_users gtu_f
            JOIN game_types gt_f ON gt_f.id = gtu_f.game_type_id
            WHERE gtu_f.user_id = $1
              AND ($3::text IS NULL OR gt_f.name = $3)
        ) gtu ON gtu.game_type_id = agg.game_type_id
        LEFT JOIN game_types gt ON gt.id = gtu.game_type_id
        ORDER BY game_type_name
        "#,
    )
    .bind(user_id)
    .bind(include_single_human)
    .bind(game_type_name)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let win_percent = if row.games == 0 {
                0.0
            } else {
                row.wins as f64 * 100.0 / row.games as f64
            };
            super::GameTypeStats {
                game_type_name: row.game_type_name,
                games: row.games,
                wins: row.wins,
                win_percent,
                avg_place_percentile: row.avg_place_percentile,
                rating: row.rating,
                peak_rating: row.peak_rating,
            }
        })
        .collect())
}

pub async fn rating_series(
    pool: &PgPool,
    user_id: Uuid,
    game_type_name: &str,
) -> Result<Vec<super::RatingPoint>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            g.finished_at AS "finished_at!",
            gp.rating_change AS "rating_change!"
        FROM game_players gp
        JOIN games g ON g.id = gp.game_id
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        WHERE gp.user_id = $1
          AND gt.name = $2
          AND gp.rating_change IS NOT NULL
          AND g.finished_at IS NOT NULL
        ORDER BY g.finished_at, g.id
        "#,
        user_id,
        game_type_name
    )
    .fetch_all(pool)
    .await?;

    let mut rating = crate::db::INITIAL_RATING;
    Ok(rows
        .into_iter()
        .map(|row| {
            rating += row.rating_change;
            super::RatingPoint {
                finished_at: row.finished_at,
                rating,
            }
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct OpponentRow {
    game_id: Uuid,
    user_id: Option<Uuid>,
    name: String,
    place: Option<i32>,
}

/// Other seats (not `user_id`'s own) for each game in `game_ids`, grouped by
/// game id and ordered by seat position within each game.
async fn opponents_by_game(
    pool: &PgPool,
    game_ids: &[Uuid],
    user_id: Uuid,
    viewer: Option<Uuid>,
) -> Result<HashMap<Uuid, Vec<super::OpponentWithPlace>>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<OpponentRow> = sqlx::query_as(
        r#"
        SELECT
            gp.game_id,
            u.id AS user_id,
            COALESCE(u.name, gb.name, 'Bot') AS name,
            gp.place AS place
        FROM game_players gp
        LEFT JOIN users u ON u.id = gp.user_id
        LEFT JOIN game_bots gb ON gb.id = gp.game_bot_id
        WHERE gp.game_id = ANY($1) AND gp.user_id IS DISTINCT FROM $2
        ORDER BY gp.game_id, gp.position
        "#,
    )
    .bind(game_ids)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let distinct_human_ids: Vec<Uuid> = {
        let mut ids: Vec<Uuid> = rows.iter().filter_map(|r| r.user_id).collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let visible = if distinct_human_ids.is_empty() {
        std::collections::HashSet::new()
    } else {
        crate::db::visible_user_ids(pool, &distinct_human_ids, viewer).await?
    };

    let mut by_game: HashMap<Uuid, Vec<super::OpponentWithPlace>> = HashMap::new();
    for row in rows {
        let (uid, name) = match row.user_id {
            Some(id) if visible.contains(&id) => (Some(id), row.name),
            Some(_) => (None, "Anonymous".to_string()),
            None => (None, row.name),
        };
        by_game
            .entry(row.game_id)
            .or_default()
            .push(super::OpponentWithPlace {
                user_id: uid,
                name,
                place: row.place,
            });
    }
    Ok(by_game)
}

#[derive(Debug, sqlx::FromRow)]
struct FinishedGamesRow {
    game_id: Uuid,
    game_type_name: String,
    finished_at: Option<PrimitiveDateTime>,
    ranked_placing: Option<i32>,
    rating_change: Option<i32>,
    player_count: i64,
}

pub async fn finished_games(
    pool: &PgPool,
    user_id: Uuid,
    game_type_name: Option<&str>,
    include_single_human: bool,
    limit: Option<i64>,
    viewer: Option<Uuid>,
) -> Result<Vec<super::FinishedGameRow>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<FinishedGamesRow> = sqlx::query_as(
        r#"
        SELECT
            g.id AS game_id,
            gt.name AS game_type_name,
            g.finished_at,
            gp.ranked_placing AS ranked_placing,
            gp.rating_change,
            (SELECT count(*) FROM game_players gp2
             WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL) AS player_count
        FROM game_players gp
        JOIN games g ON g.id = gp.game_id
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        WHERE gp.user_id = $1
          AND g.is_finished = true
          AND gp.ranked_placing IS NOT NULL
          AND ($3::text IS NULL OR gt.name = $3)
          AND (
              SELECT count(*) FROM game_players gp3
              WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL AND gp3.ranked_placing IS NOT NULL
          ) >= CASE WHEN $2 THEN 1 ELSE 2 END
        ORDER BY g.finished_at DESC NULLS LAST, g.id
        LIMIT $4::bigint
        "#,
    )
    .bind(user_id)
    .bind(include_single_human)
    .bind(game_type_name)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let game_ids: Vec<Uuid> = rows.iter().map(|row| row.game_id).collect();
    let mut opponents = opponents_by_game(pool, &game_ids, user_id, viewer).await?;

    Ok(rows
        .into_iter()
        .map(|row| super::FinishedGameRow {
            game_id: row.game_id,
            game_type_name: row.game_type_name,
            finished_at: row.finished_at,
            ranked_placing: row.ranked_placing,
            player_count: row.player_count,
            rating_change: row.rating_change,
            opponents: opponents
                .remove(&row.game_id)
                .unwrap_or_default()
                .into_iter()
                .map(|o| super::Opponent {
                    user_id: o.user_id,
                    name: o.name,
                })
                .collect(),
        })
        .collect())
}

pub async fn active_games(
    pool: &PgPool,
    user_id: Uuid,
    viewer: Option<Uuid>,
) -> Result<Vec<super::ActiveGameRow>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            g.id AS game_id,
            gt.name AS game_type_name,
            me.is_turn AS is_turn,
            g.updated_at AS updated_at
        FROM games g
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        JOIN game_players me ON me.game_id = g.id AND me.user_id = $1
        WHERE g.is_finished = false
        ORDER BY me.is_turn DESC, g.updated_at DESC, g.id
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let game_ids: Vec<Uuid> = rows.iter().map(|row| row.game_id).collect();
    let mut opponents = opponents_by_game(pool, &game_ids, user_id, viewer).await?;

    Ok(rows
        .into_iter()
        .map(|row| super::ActiveGameRow {
            game_id: row.game_id,
            game_type_name: row.game_type_name,
            is_turn: row.is_turn,
            opponents: opponents
                .remove(&row.game_id)
                .unwrap_or_default()
                .into_iter()
                .map(|o| super::Opponent {
                    user_id: o.user_id,
                    name: o.name,
                })
                .collect(),
            updated_at: row.updated_at,
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct GameHistoryRow {
    game_id: Uuid,
    game_type_name: String,
    is_finished: bool,
    started_at: PrimitiveDateTime,
    finished_at: Option<PrimitiveDateTime>,
    my_place: Option<i32>,
    my_rating_change: Option<i32>,
    player_count: i64,
    match_min: Option<i32>,
    match_max: Option<i32>,
    match_avg: Option<i32>,
}

#[allow(clippy::too_many_arguments)]
pub async fn game_history(
    pool: &PgPool,
    user_id: Uuid,
    status: Option<bool>,
    game_type: Option<&str>,
    include_single_human: bool,
    limit: i64,
    offset: i64,
    viewer: Option<Uuid>,
) -> Result<Vec<super::HistoryRow>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<GameHistoryRow> = sqlx::query_as(
        r#"
        SELECT g.id AS game_id, gt.name AS game_type_name, g.is_finished AS is_finished,
               g.created_at AS started_at, g.finished_at AS finished_at,
               gp.place AS my_place, gp.rating_change AS my_rating_change,
               agg.player_count AS player_count,
               agg.match_min AS match_min,
               agg.match_max AS match_max,
               agg.match_avg AS match_avg
        FROM game_players gp
        JOIN games g          ON g.id = gp.game_id
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt    ON gt.id = gv.game_type_id
        LEFT JOIN LATERAL (
            SELECT count(*) AS player_count,
                   min(rating_before) AS match_min,
                   max(rating_before) AS match_max,
                   avg(rating_before)::int AS match_avg
            FROM game_players WHERE game_id = g.id
        ) agg ON true
        WHERE gp.user_id = $1
          AND ($2::boolean IS NULL OR g.is_finished = $2)
          AND ($3::text    IS NULL OR gt.name = $3)
          AND (SELECT count(*) FROM game_players gp3 WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL) >= CASE WHEN $4 THEN 1 ELSE 2 END
        ORDER BY g.created_at DESC, g.id
        LIMIT $5::bigint OFFSET $6::bigint
        "#,
    )
    .bind(user_id)
    .bind(status)
    .bind(game_type)
    .bind(include_single_human)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let game_ids: Vec<Uuid> = rows.iter().map(|row| row.game_id).collect();
    let mut opponents = opponents_by_game(pool, &game_ids, user_id, viewer).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let match_elo = match (row.match_min, row.match_max, row.match_avg) {
                (Some(mn), Some(mx), Some(av)) => Some(super::MatchElo {
                    min: mn,
                    max: mx,
                    avg: av,
                }),
                _ => None,
            };
            super::HistoryRow {
                game_id: row.game_id,
                game_type_name: row.game_type_name,
                is_finished: row.is_finished,
                started_at: row.started_at,
                finished_at: row.finished_at,
                my_place: row.my_place,
                player_count: row.player_count,
                my_rating_change: row.my_rating_change,
                opponents: opponents.remove(&row.game_id).unwrap_or_default(),
                match_elo,
            }
        })
        .collect())
}

pub async fn game_history_count(
    pool: &PgPool,
    user_id: Uuid,
    status: Option<bool>,
    game_type: Option<&str>,
    include_single_human: bool,
) -> Result<i64> {
    let (count,): (i64,) = sqlx::query_as(
        r#"
        SELECT count(*)
        FROM game_players gp
        JOIN games g          ON g.id = gp.game_id
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt    ON gt.id = gv.game_type_id
        WHERE gp.user_id = $1
          AND ($2::boolean IS NULL OR g.is_finished = $2)
          AND ($3::text    IS NULL OR gt.name = $3)
          AND (SELECT count(*) FROM game_players gp3 WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL) >= CASE WHEN $4 THEN 1 ELSE 2 END
        "#,
    )
    .bind(user_id)
    .bind(status)
    .bind(game_type)
    .bind(include_single_human)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

#[derive(Debug, sqlx::FromRow)]
struct HeadToHeadRow {
    user_id: Uuid,
    name: String,
    games: i64,
    wins: i64,
    losses: i64,
    ties: i64,
}

pub async fn head_to_head(
    pool: &PgPool,
    user_id: Uuid,
    game_type_name: &str,
    include_single_human: bool,
    viewer: Option<Uuid>,
) -> Result<Vec<super::HeadToHead>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<HeadToHeadRow> = sqlx::query_as(
        r#"
        WITH qualifying AS (
            SELECT g.id AS game_id, gp.ranked_placing AS place
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND gt.name = $2
              AND gp.ranked_placing IS NOT NULL
              AND (
                  SELECT count(*) FROM game_players gp2
                  WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL
              ) >= CASE WHEN $3 THEN 1 ELSE 2 END
        ),
        opponent_rows AS (
            SELECT
                q.place AS my_place,
                gp.user_id AS opp_id,
                u.name AS opp_name,
                gp.ranked_placing AS opp_place
            FROM qualifying q
            JOIN game_players gp
                ON gp.game_id = q.game_id AND gp.user_id IS NOT NULL AND gp.user_id <> $1
                AND gp.ranked_placing IS NOT NULL
            JOIN users u ON u.id = gp.user_id
        )
        SELECT
            opp_id AS user_id,
            opp_name AS name,
            count(*) AS games,
            count(*) FILTER (
                WHERE my_place < opp_place
            ) AS wins,
            count(*) FILTER (
                WHERE my_place > opp_place
            ) AS losses,
            count(*) FILTER (
                WHERE my_place = opp_place
            ) AS ties
        FROM opponent_rows
        GROUP BY opp_id, opp_name
        ORDER BY games DESC, name
        "#,
    )
    .bind(user_id)
    .bind(game_type_name)
    .bind(include_single_human)
    .fetch_all(pool)
    .await?;

    let opp_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
    let visible = if opp_ids.is_empty() {
        std::collections::HashSet::new()
    } else {
        crate::db::visible_user_ids(pool, &opp_ids, viewer).await?
    };

    Ok(rows
        .into_iter()
        .map(|row| {
            let (uid, name) = if visible.contains(&row.user_id) {
                (Some(row.user_id), row.name)
            } else {
                (None, "Anonymous".to_string())
            };
            super::HeadToHead {
                user_id: uid,
                name,
                games: row.games,
                wins: row.wins,
                losses: row.losses,
                ties: row.ties,
            }
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct RecentFormRow {
    game_type_name: String,
    game_id: Uuid,
    finished_at: Option<PrimitiveDateTime>,
    place: Option<i32>,
    rating_change: Option<i32>,
    player_count: i64,
}

pub async fn recent_form(
    pool: &PgPool,
    user_id: Uuid,
    per_type: i64,
    include_single_human: bool,
) -> Result<Vec<super::GameTypeForm>> {
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<RecentFormRow> = sqlx::query_as(
        r#"
        WITH qualifying AS (
            SELECT
                gt.name AS game_type_name,
                g.id AS game_id,
                g.finished_at,
                gp.ranked_placing AS place,
                gp.rating_change,
                (SELECT count(*) FROM game_players gp2
                 WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL) AS player_count,
                row_number() OVER (
                    PARTITION BY gt.id ORDER BY g.finished_at DESC NULLS LAST, g.id
                ) AS rn
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND gp.ranked_placing IS NOT NULL
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL AND gp3.ranked_placing IS NOT NULL
              ) >= CASE WHEN $3 THEN 1 ELSE 2 END
        )
        SELECT
            game_type_name AS game_type_name,
            game_id AS game_id,
            finished_at,
            place,
            rating_change,
            player_count AS player_count
        FROM qualifying
        WHERE rn <= $2
        ORDER BY game_type_name, finished_at ASC, game_id
        "#,
    )
    .bind(user_id)
    .bind(per_type)
    .bind(include_single_human)
    .fetch_all(pool)
    .await?;

    let mut forms: Vec<super::GameTypeForm> = Vec::new();
    for row in rows {
        let result = super::FormResult {
            game_id: row.game_id,
            finished_at: row.finished_at,
            place: row.place,
            player_count: row.player_count,
            rating_change: row.rating_change,
        };
        match forms.last_mut() {
            Some(form) if form.game_type_name == row.game_type_name => {
                form.results.push(result);
            }
            _ => forms.push(super::GameTypeForm {
                game_type_name: row.game_type_name,
                results: vec![result],
            }),
        }
    }

    Ok(forms)
}

/// Recent form for multiple users within a single game type - last
/// `per_user` finished games each, oldest-to-newest, keyed by user id.
#[derive(Debug, sqlx::FromRow)]
struct RecentFormForGameTypeRow {
    user_id: Uuid,
    game_id: Uuid,
    finished_at: Option<PrimitiveDateTime>,
    place: Option<i32>,
    rating_change: Option<i32>,
    player_count: i64,
}

pub async fn recent_form_for_game_type(
    pool: &PgPool,
    user_ids: &[Uuid],
    game_type_id: Uuid,
    per_user: i64,
) -> Result<HashMap<Uuid, Vec<super::FormResult>>> {
    // Single-human games deliberately excluded (>= 2 hardcoded): this fn serves
    // the game-type page's multi-user form, which only shows human-vs-human games.
    // Runtime query_as: result shape maps naturally to a named FromRow struct; binds are static.
    let rows: Vec<RecentFormForGameTypeRow> = sqlx::query_as(
        r#"
        WITH qualifying AS (
            SELECT
                gp.user_id AS user_id,
                g.id AS game_id,
                g.finished_at,
                gp.ranked_placing AS place,
                gp.rating_change,
                (SELECT count(*) FROM game_players gp2
                 WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL AND gp2.ranked_placing IS NOT NULL) AS player_count,
                row_number() OVER (
                    PARTITION BY gp.user_id ORDER BY g.finished_at DESC NULLS LAST, g.id
                ) AS rn
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = ANY($1)
              AND gt.id = $2
              AND g.is_finished = true
              AND gp.ranked_placing IS NOT NULL
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL AND gp3.ranked_placing IS NOT NULL
              ) >= 2
        )
        SELECT
            user_id,
            game_id,
            finished_at,
            place,
            rating_change,
            player_count
        FROM qualifying
        WHERE rn <= $3
        ORDER BY user_id, finished_at ASC, game_id
        "#,
    )
    .bind(user_ids)
    .bind(game_type_id)
    .bind(per_user)
    .fetch_all(pool)
    .await?;

    let mut by_user: HashMap<Uuid, Vec<super::FormResult>> = HashMap::new();
    for row in rows {
        by_user
            .entry(row.user_id)
            .or_default()
            .push(super::FormResult {
                game_id: row.game_id,
                finished_at: row.finished_at,
                place: row.place,
                player_count: row.player_count,
                rating_change: row.rating_change,
            });
    }

    Ok(by_user)
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use time::PrimitiveDateTime;

    const COLORS: [&str; 8] = [
        "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
    ];

    pub(crate) async fn make_user(pool: &PgPool, name: &str) -> Uuid {
        make_user_with_prefs(pool, name, &[]).await
    }

    pub(crate) async fn make_user_with_prefs(
        pool: &PgPool,
        name: &str,
        pref_colors: &[&str],
    ) -> Uuid {
        let prefs: Vec<String> = pref_colors.iter().map(|c| c.to_string()).collect();
        sqlx::query_scalar!(
            r#"INSERT INTO users (id, name, pref_colors) VALUES (uuid_generate_v4(), $1, $2) RETURNING id"#,
            name,
            &prefs
        )
        .fetch_one(pool)
        .await
        .expect("insert user")
    }

    pub(crate) async fn make_game_type(pool: &PgPool, name: &str) -> (Uuid, Uuid) {
        let game_type_id = sqlx::query_scalar!(
            r#"INSERT INTO game_types (id, name, player_counts) VALUES (uuid_generate_v4(), $1, '{2,3,4}') RETURNING id"#,
            name
        )
        .fetch_one(pool)
        .await
        .expect("insert game_type");

        let game_version_id = sqlx::query_scalar!(
            r#"INSERT INTO game_versions (id, game_type_id, name, uri, is_public, is_deprecated)
               VALUES (uuid_generate_v4(), $1, '1.0.0', 'http://localhost:0/mock', true, false)
               RETURNING id"#,
            game_type_id
        )
        .fetch_one(pool)
        .await
        .expect("insert game_version");

        (game_type_id, game_version_id)
    }

    pub(crate) async fn insert_finished_game(
        pool: &PgPool,
        game_version_id: Uuid,
        finished_at: PrimitiveDateTime,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        insert_game(pool, game_version_id, true, Some(finished_at), players).await
    }

    pub(crate) async fn insert_unfinished_game(
        pool: &PgPool,
        game_version_id: Uuid,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        insert_game(pool, game_version_id, false, None, players).await
    }

    async fn insert_game(
        pool: &PgPool,
        game_version_id: Uuid,
        is_finished: bool,
        finished_at: Option<PrimitiveDateTime>,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        let game_id = sqlx::query_scalar!(
            r#"INSERT INTO games (id, game_version_id, is_finished, finished_at, game_state)
               VALUES (uuid_generate_v4(), $1, $2, $3, '')
               RETURNING id"#,
            game_version_id,
            is_finished,
            finished_at
        )
        .fetch_one(pool)
        .await
        .expect("insert game");

        for (i, (user_id, place, rating_change)) in players.iter().enumerate() {
            let game_bot_id = if user_id.is_none() {
                Some(
                    sqlx::query_scalar!(
                        r#"INSERT INTO game_bots (id, game_id, name, bot_name)
                           VALUES (uuid_generate_v4(), $1, $2, 'medium')
                           RETURNING id"#,
                        game_id,
                        format!("bot-{i}")
                    )
                    .fetch_one(pool)
                    .await
                    .expect("insert game_bot"),
                )
            } else {
                None
            };
            // Competitive fixtures default a human seat's ranked_placing to its
            // authoritative place; tests that need a historical null-ranked or a
            // diverging competitive placing override this directly.
            let ranked_placing = if user_id.is_some() { *place } else { None };

            sqlx::query(
                r#"INSERT INTO game_players
                    (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
                     is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, ranked_placing, rating_change)
                   VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, true, false, now(), now(), false, true, $6, $7, $8)"#,
            )
            .bind(game_id)
            .bind(*user_id)
            .bind(game_bot_id)
            .bind(i as i32)
            .bind(COLORS[i % COLORS.len()])
            .bind(*place)
            .bind(ranked_placing)
            .bind(*rating_change)
            .execute(pool)
            .await
            .expect("insert game_player");
        }

        game_id
    }

    pub(crate) async fn set_game_type_rating(
        pool: &PgPool,
        game_type_id: Uuid,
        user_id: Uuid,
        rating: i32,
        peak: i32,
    ) {
        sqlx::query!(
            r#"INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
               VALUES (uuid_generate_v4(), $1, $2, $3, $4)
               ON CONFLICT (game_type_id, user_id) DO UPDATE SET rating = $3, peak_rating = $4"#,
            game_type_id,
            user_id,
            rating,
            peak
        )
        .execute(pool)
        .await
        .expect("upsert game_type_users");
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use time::macros::datetime;

    #[sqlx::test]
    async fn get_profile_user_finds_case_insensitively_and_normalizes_color(pool: PgPool) {
        make_user_with_prefs(&pool, "PlayerOne", &["Amber", "Red"]).await;

        let found = get_profile_user(&pool, "playerone")
            .await
            .expect("query ok")
            .expect("user found");
        assert_eq!(found.name, "PlayerOne");
        assert_eq!(found.pref_color, Some("Orange".to_string()));

        let missing = get_profile_user(&pool, "nobody").await.expect("query ok");
        assert!(missing.is_none());
    }

    #[sqlx::test]
    async fn find_game_type_name_matches_case_insensitively(pool: PgPool) {
        make_game_type(&pool, "Camel Up").await;

        let found = find_game_type_name(&pool, "camel up")
            .await
            .expect("query ok");
        assert_eq!(found, Some("Camel Up".to_string()));

        let missing = find_game_type_name(&pool, "Nonexistent")
            .await
            .expect("query ok");
        assert!(missing.is_none());
    }

    #[sqlx::test]
    async fn overall_totals_applies_d1_inclusion_rule(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[
                (Some(user), Some(2), None),
                (None, Some(1), None),
                (None, Some(3), None),
            ],
        )
        .await;

        insert_unfinished_game(&pool, gv, &[(Some(user), None, None)]).await;

        let excluding = overall_totals(&pool, user, false).await.expect("query ok");
        assert_eq!(excluding.finished_games, 1);
        assert_eq!(excluding.wins, 1);
        assert_eq!(excluding.win_percent, 100.0);

        let including = overall_totals(&pool, user, true).await.expect("query ok");
        assert_eq!(including.finished_games, 2);
        assert_eq!(including.wins, 1);
        assert_eq!(including.win_percent, 50.0);
    }

    #[sqlx::test]
    async fn overall_totals_counts_tied_first_place_as_win_for_both(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(1), None)],
        )
        .await;

        let alice_totals = overall_totals(&pool, alice, false).await.expect("query ok");
        assert_eq!(alice_totals.wins, 1);
        assert_eq!(alice_totals.win_percent, 100.0);

        let bob_totals = overall_totals(&pool, bob, false).await.expect("query ok");
        assert_eq!(bob_totals.wins, 1);
        assert_eq!(bob_totals.win_percent, 100.0);
    }

    /// Historical null-ranked rows (finished authoritative `place` but no
    /// competitive `ranked_placing`) never contribute to competitive counts:
    /// they are excluded from both the player's own rows and the eligibility
    /// denominator, and are never repaired.
    #[sqlx::test]
    async fn competitive_queries_exclude_historical_null_ranked_games(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(opponent), Some(1), None)],
        )
        .await;
        sqlx::query("UPDATE game_players SET ranked_placing = NULL WHERE game_id = $1")
            .bind(legacy)
            .execute(&pool)
            .await
            .expect("null ranked placing");

        let totals = overall_totals(&pool, user, false).await.expect("query ok");
        assert_eq!(totals.finished_games, 1);
        assert_eq!(totals.wins, 1);

        let stats = game_type_stats(&pool, user, false, None)
            .await
            .expect("query ok");
        let camel = stats
            .iter()
            .find(|s| s.game_type_name == "Camel Up")
            .expect("camel up present");
        assert_eq!(camel.games, 1);
        assert_eq!(camel.wins, 1);

        let forms = recent_form(&pool, user, 10, false).await.expect("query ok");
        let camel_form = forms
            .iter()
            .find(|f| f.game_type_name == "Camel Up")
            .expect("camel up form present");
        assert_eq!(camel_form.results.len(), 1);
        assert_eq!(camel_form.results[0].place, Some(1));

        // The null-ranked game also fails to meet the two-ranked-human gate,
        // so it is absent even when its own row carries a competitive place.
        let gt_only = game_type_stats(&pool, user, false, Some("Camel Up"))
            .await
            .expect("query ok");
        assert_eq!(gt_only[0].games, 1);
        assert_eq!(
            gt_only[0].avg_place_percentile,
            Some(1.0),
            "only the ranked game remains; percentile (2 - 1) / (2 - 1)"
        );
    }

    #[sqlx::test]
    async fn game_type_stats_includes_rating_only_types_and_orders_by_name(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;

        let (gt_zebra, gv_zebra) = make_game_type(&pool, "Zebra Game").await;
        let (gt_camel, _gv_camel) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv_zebra,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        set_game_type_rating(&pool, gt_zebra, user, 1300, 1350).await;
        set_game_type_rating(&pool, gt_camel, user, 1100, 1150).await;

        let stats = game_type_stats(&pool, user, false, None)
            .await
            .expect("query ok");
        assert_eq!(stats.len(), 2);

        assert_eq!(stats[0].game_type_name, "Camel Up");
        assert_eq!(stats[0].games, 0);
        assert_eq!(stats[0].wins, 0);
        assert_eq!(stats[0].rating, Some(1100));
        assert_eq!(stats[0].peak_rating, Some(1150));

        assert_eq!(stats[1].game_type_name, "Zebra Game");
        assert_eq!(stats[1].games, 1);
        assert_eq!(stats[1].wins, 1);
        assert_eq!(stats[1].win_percent, 100.0);
        assert_eq!(stats[1].rating, Some(1300));
        assert_eq!(stats[1].peak_rating, Some(1350));
    }

    /// Competitive denominators: only eligible ranked human rows count. A
    /// user-vs-bots game has n = 1 so no percentile is produced (n >= 2
    /// required), and an all-human game's percentile divides by the ranked
    /// human count, not the seat count.
    #[sqlx::test]
    async fn game_type_stats_computes_avg_place_percentile(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let carol = make_user(&pool, "carol").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), None),
                (None, Some(2), None),
                (None, Some(3), None),
                (None, Some(4), None),
            ],
        )
        .await;

        // Pure bots contribute no ranked-human denominator: n = 1.
        let stats = game_type_stats(&pool, user, true, None)
            .await
            .expect("query ok");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].games, 1);
        assert_eq!(stats[0].wins, 1);
        assert_eq!(
            stats[0].avg_place_percentile, None,
            "single ranked participant yields no percentile"
        );

        // Three ranked humans: percentile uses the ranked-human denominator n = 3.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[
                (Some(user), Some(3), None),
                (Some(bob), Some(2), None),
                (Some(carol), Some(1), None),
            ],
        )
        .await;

        // Tied competitive places: both ranked first count as wins.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[
                (Some(user), Some(1), None),
                (Some(bob), Some(2), None),
                (Some(carol), Some(1), None),
            ],
        )
        .await;

        let stats = game_type_stats(&pool, user, false, None)
            .await
            .expect("query ok");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].games, 2);
        assert_eq!(stats[0].wins, 1, "tied ranked-first game counts as a win");
        // Both ranked games: (3 - 3) / 2 = 0.0 and (3 - 1) / 2 = 1.0.
        assert_eq!(stats[0].avg_place_percentile, Some(0.5));
    }

    #[sqlx::test]
    async fn game_type_stats_does_not_leak_other_users_ratings(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let other = make_user(&pool, "bob").await;

        let (_gt_shared, gv_shared) = make_game_type(&pool, "Camel Up").await;
        insert_finished_game(
            &pool,
            gv_shared,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(other), Some(2), None)],
        )
        .await;

        let (gt_other, _gv_other) = make_game_type(&pool, "Zebra Game").await;
        set_game_type_rating(&pool, gt_other, other, 1400, 1450).await;

        let stats = game_type_stats(&pool, user, false, None)
            .await
            .expect("query ok");

        assert!(
            !stats.iter().any(|s| s.game_type_name == "Zebra Game"),
            "other user's game_type_users row leaked into this user's stats: {stats:?}"
        );
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].game_type_name, "Camel Up");
    }

    /// F-151 / wd F48: with an explicit game-type filter, `game_type_stats`
    /// must return ONLY that type and that type's own rating. The user holds
    /// rating rows for two alphabetically-ordered distinct types ("Acquire"
    /// sorts before "Zebra Game"); requesting the later one must not surface
    /// the earlier one's rating via the unfiltered side of the FULL OUTER JOIN.
    /// Pre-fix this fails: the `gtu` side is filtered by user only, so the
    /// alphabetically-first "Acquire" row leaks in and orders first.
    #[sqlx::test]
    async fn game_type_stats_explicit_filter_returns_only_requested_type_and_rating(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;

        let (gt_acquire, _gv_acquire) = make_game_type(&pool, "Acquire").await;
        let (gt_zebra, gv_zebra) = make_game_type(&pool, "Zebra Game").await;

        insert_finished_game(
            &pool,
            gv_zebra,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        set_game_type_rating(&pool, gt_acquire, user, 1300, 1350).await;
        set_game_type_rating(&pool, gt_zebra, user, 1400, 1450).await;

        let stats = game_type_stats(&pool, user, false, Some("Zebra Game"))
            .await
            .expect("query ok");

        assert_eq!(
            stats.len(),
            1,
            "only the requested game type may be returned: {stats:?}"
        );
        assert_eq!(stats[0].game_type_name, "Zebra Game");
        assert_eq!(stats[0].games, 1);
        assert_eq!(stats[0].wins, 1);
        assert_eq!(
            stats[0].rating,
            Some(1400),
            "rating must be the requested type's, not Acquire's 1300"
        );
        assert_eq!(stats[0].peak_rating, Some(1450));
    }

    #[sqlx::test]
    async fn rating_series_reconstruction_matches_game_type_users_rating(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), Some(16)),
                (Some(opponent), Some(2), Some(-16)),
            ],
        )
        .await;

        // Bot game interleaved between rated games; rating_change NULL, must
        // not appear in the series.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[
                (Some(user), Some(2), Some(-8)),
                (Some(opponent), Some(1), Some(8)),
            ],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[
                (Some(user), Some(1), Some(20)),
                (Some(opponent), Some(2), Some(-20)),
            ],
        )
        .await;

        set_game_type_rating(&pool, gt, user, 1228, 1228).await;

        let series = rating_series(&pool, user, "Camel Up")
            .await
            .expect("query ok");

        assert_eq!(series.len(), 3);
        assert_eq!(series[0].finished_at, datetime!(2026-01-01 00:00:00));
        assert_eq!(series[0].rating, 1216);
        assert_eq!(series[1].finished_at, datetime!(2026-01-03 00:00:00));
        assert_eq!(series[1].rating, 1208);
        assert_eq!(series[2].finished_at, datetime!(2026-01-04 00:00:00));
        assert_eq!(series[2].rating, 1228);

        let final_row = game_type_stats(&pool, user, false, None)
            .await
            .expect("query ok")
            .into_iter()
            .find(|s| s.game_type_name == "Camel Up")
            .expect("camel up present");
        assert_eq!(final_row.rating, Some(series[2].rating));
    }

    #[sqlx::test]
    async fn finished_games_returns_opponents_and_respects_limit_and_type_filter(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt_camel, gv_camel) = make_game_type(&pool, "Camel Up").await;
        let (_gt_duel, gv_duel) = make_game_type(&pool, "Duel").await;

        let game1 = insert_finished_game(
            &pool,
            gv_camel,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), Some(16)),
                (Some(opponent), Some(2), Some(-16)),
            ],
        )
        .await;

        // Single-human + bot game: only visible with include_single_human.
        let game2 = insert_finished_game(
            &pool,
            gv_camel,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;

        let game3 = insert_finished_game(
            &pool,
            gv_duel,
            datetime!(2026-01-03 00:00:00),
            &[
                (Some(user), Some(2), Some(-8)),
                (Some(opponent), Some(1), Some(8)),
            ],
        )
        .await;

        let all = finished_games(&pool, user, None, true, None, None)
            .await
            .expect("query ok");
        assert_eq!(all.len(), 3);
        // DESC order: newest first.
        assert_eq!(all[0].game_id, game3);
        assert_eq!(all[1].game_id, game2);
        assert_eq!(all[2].game_id, game1);

        let row1 = all
            .iter()
            .find(|r| r.game_id == game1)
            .expect("game1 present");
        assert_eq!(row1.player_count, 2);
        assert_eq!(row1.ranked_placing, Some(1));
        assert_eq!(row1.rating_change, Some(16));
        assert_eq!(row1.opponents.len(), 1);
        assert_eq!(row1.opponents[0].user_id, Some(opponent));
        assert_eq!(row1.opponents[0].name, "bob");

        let row2 = all
            .iter()
            .find(|r| r.game_id == game2)
            .expect("game2 present");
        assert_eq!(row2.opponents.len(), 1);
        assert_eq!(row2.opponents[0].user_id, None);
        assert_eq!(row2.opponents[0].name, "bot-1");

        let excluding_single = finished_games(&pool, user, None, false, None, None)
            .await
            .expect("query ok");
        assert!(!excluding_single.iter().any(|r| r.game_id == game2));
        assert_eq!(excluding_single.len(), 2);

        let limited = finished_games(&pool, user, None, true, Some(1), None)
            .await
            .expect("query ok");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].game_id, game3);

        let camel_only = finished_games(&pool, user, Some("Camel Up"), true, None, None)
            .await
            .expect("query ok");
        assert_eq!(camel_only.len(), 2);
        assert!(camel_only.iter().all(|r| r.game_type_name == "Camel Up"));
    }

    /// `finished_games` reports competitive placement from `ranked_placing`
    /// alone - never the authoritative `place`. The user's authoritative seat
    /// is 1 but their competitive placing is 2; the row must carry 2 and a
    /// ranked-human player count of 2.
    #[sqlx::test]
    async fn finished_games_uses_competitive_placing_not_authoritative_place(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        sqlx::query(
            "UPDATE game_players SET ranked_placing = $1 WHERE game_id = $2 AND user_id = $3",
        )
        .bind(2)
        .bind(game)
        .bind(user)
        .execute(&pool)
        .await
        .expect("override user ranked placing");

        let rows = finished_games(&pool, user, None, false, None, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].ranked_placing, Some(2),
            "ranked_placing must come from ranked_placing, not authoritative place"
        );
        assert_eq!(rows[0].player_count, 2);
    }

    /// `player_count` and the inclusion gate count eligible ranked human seats
    /// only: pure bots contribute neither to the participant count nor to
    /// meeting the include_single_human rule.
    #[sqlx::test]
    async fn finished_games_player_count_counts_ranked_humans_only(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), None),
                (Some(opponent), Some(2), None),
                (None, Some(3), None),
                (None, Some(4), None),
            ],
        )
        .await;

        let rows = finished_games(&pool, user, None, true, None, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].player_count, 2, "two ranked humans, not four seats");
    }

    /// A historical null-ranked row (finished authoritative `place` but no
    /// competitive `ranked_placing`) never surfaces in `finished_games`: the
    /// user's own row is filtered out and it contributes nothing to the
    /// ranked-human eligibility denominator.
    #[sqlx::test]
    async fn finished_games_excludes_historical_null_ranked_games(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(opponent), Some(1), None)],
        )
        .await;
        sqlx::query("UPDATE game_players SET ranked_placing = NULL WHERE game_id = $1")
            .bind(legacy)
            .execute(&pool)
            .await
            .expect("null ranked placing");

        let rows = finished_games(&pool, user, None, false, None, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 1);
        assert!(
            !rows.iter().any(|r| r.game_id == legacy),
            "null-ranked game must be excluded: {rows:?}"
        );
        assert_eq!(rows[0].player_count, 2);
    }

    /// A replaced human keeps its `user_id` alongside a replacement bot and
    /// remains an eligible competitive participant: it counts in `player_count`
    /// and toward the ranked-human inclusion gate, and still appears as an
    /// opponent under its human identity.
    #[sqlx::test]
    async fn finished_games_includes_replaced_humans_as_participants(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let replaced = make_user(&pool, "dave").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(replaced), Some(2), None)],
        )
        .await;
        let bot_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO game_bots (id, game_id, name, bot_name)
               VALUES (uuid_generate_v4(), $1, 'replacement-bot', 'medium')
               RETURNING id"#,
        )
        .bind(game)
        .fetch_one(&pool)
        .await
        .expect("insert replacement bot");
        sqlx::query(
            "UPDATE game_players SET game_bot_id = $1 WHERE game_id = $2 AND user_id = $3",
        )
        .bind(bot_id)
        .bind(game)
        .bind(replaced)
        .execute(&pool)
        .await
        .expect("attach replacement bot seat");

        let rows = finished_games(&pool, user, None, false, None, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].player_count, 2);
        assert_eq!(rows[0].opponents.len(), 1);
        assert_eq!(rows[0].opponents[0].user_id, Some(replaced));
        assert_eq!(rows[0].opponents[0].name, "dave");
    }

    /// Tied competitive placements: both tied ranked-first humans carry the
    /// same non-null `ranked_placing` in their finished-games rows and each
    /// still sees the other in the ranked-human count.
    #[sqlx::test]
    async fn finished_games_reports_tied_competitive_places(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(1), None)],
        )
        .await;

        let for_user = finished_games(&pool, user, None, false, None, None)
            .await
            .expect("query ok");
        assert_eq!(for_user.len(), 1);
        assert_eq!(for_user[0].ranked_placing, Some(1));
        assert_eq!(for_user[0].player_count, 2);

        let for_opponent = finished_games(&pool, opponent, None, false, None, None)
            .await
            .expect("query ok");
        assert_eq!(for_opponent.len(), 1);
        assert_eq!(for_opponent[0].ranked_placing, Some(1));
        assert_eq!(for_opponent[0].player_count, 2);
    }

    /// wd F51: the per-game rating aggregate (the `LEFT JOIN LATERAL` that
    /// produces `match_elo`) must ignore NULL `rating_before` seats, and the
    /// game-type filter must keep another game type's ratings out entirely.
    /// Calls `game_history` directly (the old body queried raw SQL and would
    /// have kept passing even if the lateral were deleted). The Camel Up game
    /// has one rated seat (1200) and one NULL seat, so the aggregate collapses
    /// to 1200/1200/1200 while `player_count` still counts the NULL seat; the
    /// Duel game carries a distinctive 2000 rating that must not appear.
    #[sqlx::test]
    async fn rating_before_aggregates_exclude_nulls(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (_gt_camel, gv_camel) = make_game_type(&pool, "Camel Up").await;
        let (_gt_duel, gv_duel) = make_game_type(&pool, "Duel").await;

        let camel = insert_finished_game(
            &pool,
            gv_camel,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(alice), Some(1), Some(16)),
                (Some(bob), Some(2), Some(-16)),
            ],
        )
        .await;

        // alice rated, bob left NULL: the aggregate must ignore bob's NULL.
        sqlx::query(
            "UPDATE game_players SET rating_before = 1200 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(camel)
        .bind(alice)
        .execute(&pool)
        .await
        .expect("set alice rating_before");

        let duel = insert_finished_game(
            &pool,
            gv_duel,
            datetime!(2026-01-02 00:00:00),
            &[
                (Some(alice), Some(1), Some(16)),
                (Some(bob), Some(2), Some(-16)),
            ],
        )
        .await;
        sqlx::query(
            "UPDATE game_players SET rating_before = 2000 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(duel)
        .bind(alice)
        .execute(&pool)
        .await
        .expect("set duel alice rating_before");
        sqlx::query(
            "UPDATE game_players SET rating_before = 2000 WHERE game_id = $1 AND user_id = $2",
        )
        .bind(duel)
        .bind(bob)
        .execute(&pool)
        .await
        .expect("set duel bob rating_before");

        let rows = game_history(&pool, alice, None, Some("Camel Up"), true, 50, 0, None)
            .await
            .expect("query ok");

        assert_eq!(
            rows.len(),
            1,
            "game-type filter must exclude the Duel game: {rows:?}"
        );
        assert_eq!(rows[0].game_id, camel);
        assert_eq!(rows[0].game_type_name, "Camel Up");
        assert_eq!(rows[0].player_count, 2, "count(*) still counts the NULL seat");

        let elo = rows[0]
            .match_elo
            .as_ref()
            .expect("match_elo present for a game with a rated seat");
        assert_eq!(elo.min, 1200, "NULL seat must not lower the min");
        assert_eq!(elo.max, 1200, "NULL seat must not raise the max");
        assert_eq!(elo.avg, 1200, "NULL seat must be excluded from the avg");
    }

    #[sqlx::test]
    async fn active_games_lists_unfinished_with_opponents(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let unfinished = insert_unfinished_game(
            &pool,
            gv,
            &[(Some(user), None, None), (Some(opponent), None, None)],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let active = active_games(&pool, user, None).await.expect("query ok");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].game_id, unfinished);
        assert!(!active[0].is_turn);
        assert_eq!(active[0].opponents.len(), 1);
        assert_eq!(active[0].opponents[0].user_id, Some(opponent));
        assert_eq!(active[0].opponents[0].name, "bob");
    }

    #[sqlx::test]
    async fn head_to_head_counts_wins_losses_ties(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;
        let (_gt2, gv2) = make_game_type(&pool, "Duel").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(opponent), Some(1), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(1), None)],
        )
        .await;
        // Different game type: excluded.
        insert_finished_game(
            &pool,
            gv2,
            datetime!(2026-01-04 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let h2h = head_to_head(&pool, user, "Camel Up", false, None)
            .await
            .expect("query ok");
        assert_eq!(h2h.len(), 1);
        assert_eq!(h2h[0].user_id, Some(opponent));
        assert_eq!(h2h[0].name, "bob");
        assert_eq!(h2h[0].games, 3);
        assert_eq!(h2h[0].wins, 1);
        assert_eq!(h2h[0].losses, 1);
        assert_eq!(h2h[0].ties, 1);
    }

    #[sqlx::test]
    async fn head_to_head_excludes_bots(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;

        let h2h = head_to_head(&pool, user, "Camel Up", true, None)
            .await
            .expect("query ok");
        assert!(h2h.is_empty());
    }

    /// Head-to-head requires a non-null competitive placement for both human
    /// participants: a game where the opponent's `ranked_placing` is null must
    /// not count toward the pair, even though the pair passed the ranked-human
    /// eligibility gate.
    #[sqlx::test]
    async fn head_to_head_requires_competitive_placement_for_both(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        sqlx::query(
            "UPDATE game_players SET ranked_placing = NULL WHERE game_id = $1 AND user_id = $2",
        )
        .bind(legacy)
        .bind(opponent)
        .execute(&pool)
        .await
        .expect("null opponent ranked placing");

        let h2h = head_to_head(&pool, user, "Camel Up", true, None)
            .await
            .expect("query ok");
        assert_eq!(h2h.len(), 1);
        assert_eq!(h2h[0].user_id, Some(opponent));
        assert_eq!(h2h[0].games, 1);
        assert_eq!(h2h[0].wins, 1);
        assert_eq!(h2h[0].losses, 0);
        assert_eq!(h2h[0].ties, 0);
    }

    /// A replaced human retains its human identity (user_id) alongside a
    /// replacement bot seat and remains an eligible competitive participant:
    /// it counts toward the eligibility denominator and head-to-head.
    #[sqlx::test]
    async fn competitive_queries_include_replaced_humans_as_participants(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let replaced = make_user(&pool, "dave").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(replaced), Some(2), None)],
        )
        .await;
        let bot_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO game_bots (id, game_id, name, bot_name)
               VALUES (uuid_generate_v4(), $1, 'replacement-bot', 'medium')
               RETURNING id"#,
        )
        .bind(game)
        .fetch_one(&pool)
        .await
        .expect("insert replacement bot");
        sqlx::query(
            "UPDATE game_players SET game_bot_id = $1 WHERE game_id = $2 AND user_id = $3",
        )
        .bind(bot_id)
        .bind(game)
        .bind(replaced)
        .execute(&pool)
        .await
        .expect("attach replacement bot seat");

        let totals = overall_totals(&pool, user, false).await.expect("query ok");
        assert_eq!(totals.finished_games, 1);
        assert_eq!(totals.wins, 1);

        let h2h = head_to_head(&pool, user, "Camel Up", false, None)
            .await
            .expect("query ok");
        assert_eq!(h2h.len(), 1);
        assert_eq!(h2h[0].user_id, Some(replaced));
        assert_eq!(h2h[0].name, "dave");
        assert_eq!(h2h[0].games, 1);
        assert_eq!(h2h[0].wins, 1);
    }

    #[sqlx::test]
    async fn recent_form_returns_last_n_chronological(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let (_gt1, gv1) = make_game_type(&pool, "Camel Up").await;
        let (_gt2, gv2) = make_game_type(&pool, "Duel").await;

        insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (None, Some(1), None)],
        )
        .await;
        let g3 = insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-03 00:00:00),
            &[(Some(user), Some(1), Some(16)), (None, Some(2), None)],
        )
        .await;
        let g4 = insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-04 00:00:00),
            &[(Some(user), Some(3), None), (None, Some(1), None)],
        )
        .await;

        let g5 = insert_finished_game(
            &pool,
            gv2,
            datetime!(2026-01-05 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;

        let forms = recent_form(&pool, user, 3, true).await.expect("query ok");
        assert_eq!(forms.len(), 2);

        let camel = forms
            .iter()
            .find(|f| f.game_type_name == "Camel Up")
            .expect("camel up present");
        // g1 (place 1) dropped as oldest; remaining last 3 in chronological order.
        assert_eq!(camel.results.len(), 3);
        assert_eq!(camel.results[0].game_id, g2);
        assert_eq!(camel.results[0].place, Some(2));
        assert_eq!(camel.results[1].game_id, g3);
        assert_eq!(camel.results[1].place, Some(1));
        assert_eq!(camel.results[1].rating_change, Some(16));
        assert_eq!(camel.results[0].rating_change, None);
        assert_eq!(camel.results[2].game_id, g4);
        assert_eq!(camel.results[2].place, Some(3));

        let duel = forms
            .iter()
            .find(|f| f.game_type_name == "Duel")
            .expect("duel present");
        assert_eq!(duel.results.len(), 1);
        assert_eq!(duel.results[0].game_id, g5);
    }

    /// A historical null-ranked game must be filtered inside the qualifying
    /// CTE before `row_number()`, so it can neither enter the window nor
    /// displace a genuinely recent ranked game.
    #[sqlx::test]
    async fn recent_form_null_ranked_does_not_displace_recent(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let g1 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(make_user(&pool, "bob").await), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(make_user(&pool, "carol").await), Some(1), None)],
        )
        .await;
        let g3 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(user), Some(1), None), (Some(make_user(&pool, "dave").await), Some(2), None)],
        )
        .await;

        // Newest by finished_at but missing the user's competitive placing.
        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[
                (Some(user), Some(3), None),
                (Some(make_user(&pool, "erin").await), Some(1), None),
                (Some(make_user(&pool, "frank").await), Some(2), None),
            ],
        )
        .await;
        sqlx::query(
            "UPDATE game_players SET ranked_placing = NULL WHERE game_id = $1 AND user_id = $2",
        )
        .bind(legacy)
        .bind(user)
        .execute(&pool)
        .await
        .expect("null user ranked placing");

        let forms = recent_form(&pool, user, 3, false).await.expect("query ok");
        let camel = forms
            .iter()
            .find(|f| f.game_type_name == "Camel Up")
            .expect("camel up present");
        let ids: Vec<Uuid> = camel.results.iter().map(|r| r.game_id).collect();
        assert!(
            !ids.contains(&legacy),
            "null-ranked game displaced a recent game: {ids:?}"
        );
        assert_eq!(ids, vec![g1, g2, g3], "window holds the ranked games, oldest first");
    }

    /// Form `player_count` is a competitive participant count: pure bots and
    /// historical null-ranked rows do not contribute.
    #[sqlx::test]
    async fn recent_form_player_count_counts_ranked_humans_only(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), None),
                (Some(opponent), Some(2), None),
                (None, Some(3), None),
                (None, Some(4), None),
            ],
        )
        .await;

        let forms = recent_form(&pool, user, 1, true).await.expect("query ok");
        let camel = forms
            .iter()
            .find(|f| f.game_type_name == "Camel Up")
            .expect("camel up present");
        assert_eq!(camel.results.len(), 1);
        assert_eq!(camel.results[0].place, Some(1));
        assert_eq!(
            camel.results[0].player_count, 2,
            "two ranked humans, not four seats"
        );
    }

    // Reconstructed-final == rating drift is already covered at the fixture
    // level by rating_series_reconstruction_matches_game_type_users_rating
    // above; this test covers the #29 backfill migration itself (peak
    // correction, idempotency, never lowering an already-correct peak).
    #[sqlx::test]
    async fn peak_rating_backfill_corrects_historical_peaks(pool: PgPool) {
        const MIGRATION: &str = include_str!("../../migrations/011_peak_rating_backfill.sql");

        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        // Rating goes up, up, then down: peak (1236) occurs mid-history,
        // final (1206) is lower than peak.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), Some(16)),
                (Some(opponent), Some(2), Some(-16)),
            ],
        )
        .await;

        // Bot game interleaved, rating_change NULL, must not affect peak.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (None, Some(2), None)],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[
                (Some(user), Some(1), Some(20)),
                (Some(opponent), Some(2), Some(-20)),
            ],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[
                (Some(user), Some(2), Some(-30)),
                (Some(opponent), Some(1), Some(30)),
            ],
        )
        .await;

        // Historical-wrong state: peak never updated by legacy code.
        set_game_type_rating(&pool, gt, user, 1206, 1200).await;

        // A second user whose peak is already correctly above the
        // reconstruction: must not be lowered.
        let other = make_user(&pool, "carol").await;
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(other), Some(1), Some(50)),
                (Some(opponent), Some(2), Some(-50)),
            ],
        )
        .await;
        set_game_type_rating(&pool, gt, other, 1250, 1300).await;

        sqlx::raw_sql(MIGRATION)
            .execute(&pool)
            .await
            .expect("run migration 011");

        let (rating, peak): (i32, i32) = sqlx::query_as(
            r#"SELECT rating, peak_rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2"#,
        )
        .bind(gt)
        .bind(user)
        .fetch_one(&pool)
        .await
        .expect("query ok");
        assert_eq!(rating, 1206);
        assert_eq!(peak, 1236);

        let (other_rating, other_peak): (i32, i32) = sqlx::query_as(
            r#"SELECT rating, peak_rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2"#,
        )
        .bind(gt)
        .bind(other)
        .fetch_one(&pool)
        .await
        .expect("query ok");
        assert_eq!(other_rating, 1250);
        assert_eq!(
            other_peak, 1300,
            "already-correct higher peak must not be lowered"
        );

        // Idempotency: running again is a no-op.
        sqlx::raw_sql(MIGRATION)
            .execute(&pool)
            .await
            .expect("run migration 011 again");

        let (rating2, peak2): (i32, i32) = sqlx::query_as(
            r#"SELECT rating, peak_rating FROM game_type_users WHERE game_type_id = $1 AND user_id = $2"#,
        )
        .bind(gt)
        .bind(user)
        .fetch_one(&pool)
        .await
        .expect("query ok");
        assert_eq!(rating2, 1206);
        assert_eq!(peak2, 1236);
    }

    #[sqlx::test]
    async fn rating_before_backfill_computes_pre_game_ratings(pool: PgPool) {
        const MIGRATION: &str = include_str!("../../migrations/017_game_player_rating_before.sql");

        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(alice), Some(1), Some(16)),
                (Some(bob), Some(2), Some(-16)),
            ],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[
                (Some(alice), Some(1), Some(20)),
                (Some(bob), Some(2), Some(-20)),
            ],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[
                (Some(alice), Some(2), Some(-30)),
                (Some(bob), Some(1), Some(30)),
            ],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[(Some(alice), Some(1), None), (None, Some(2), None)],
        )
        .await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-05 00:00:00),
            &[(Some(alice), Some(1), None), (None, Some(2), None)],
        )
        .await;

        sqlx::raw_sql(MIGRATION)
            .execute(&pool)
            .await
            .expect("run migration 017");

        let alice_rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT gp.rating_before FROM game_players gp JOIN games g ON g.id = gp.game_id WHERE gp.user_id = $1 ORDER BY g.finished_at, g.id",
        )
        .bind(alice)
        .fetch_all(&pool)
        .await
        .expect("query ok");

        assert_eq!(alice_rows[0], (Some(1200),));
        assert_eq!(alice_rows[1], (Some(1216),));
        assert_eq!(alice_rows[2], (Some(1236),));
        assert_eq!(alice_rows[3], (None,));
        assert_eq!(alice_rows[4], (None,));

        let bob_rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT gp.rating_before FROM game_players gp JOIN games g ON g.id = gp.game_id WHERE gp.user_id = $1 ORDER BY g.finished_at, g.id",
        )
        .bind(bob)
        .fetch_all(&pool)
        .await
        .expect("query ok");

        assert_eq!(bob_rows[0], (Some(1200),));
        assert_eq!(bob_rows[1], (Some(1184),));
        assert_eq!(bob_rows[2], (Some(1164),));

        sqlx::raw_sql(MIGRATION)
            .execute(&pool)
            .await
            .expect("run migration 017 again");

        let alice_rows2: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT gp.rating_before FROM game_players gp JOIN games g ON g.id = gp.game_id WHERE gp.user_id = $1 ORDER BY g.finished_at, g.id",
        )
        .bind(alice)
        .fetch_all(&pool)
        .await
        .expect("query ok");
        assert_eq!(alice_rows2, alice_rows);

        sqlx::query(
            "UPDATE game_players SET rating_before = 9999 WHERE id = (SELECT gp.id FROM game_players gp JOIN games g ON g.id = gp.game_id WHERE gp.user_id = $1 ORDER BY g.finished_at, g.id LIMIT 1)",
        )
        .bind(alice)
        .execute(&pool)
        .await
        .expect("update ok");

        sqlx::raw_sql(MIGRATION)
            .execute(&pool)
            .await
            .expect("run migration 017 third time");

        let alice_rows3: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT gp.rating_before FROM game_players gp JOIN games g ON g.id = gp.game_id WHERE gp.user_id = $1 ORDER BY g.finished_at, g.id",
        )
        .bind(alice)
        .fetch_all(&pool)
        .await
        .expect("query ok");
        assert_eq!(alice_rows3[0], (Some(9999),));
    }

    #[sqlx::test]
    async fn recent_form_for_game_type_keys_by_user_oldest_to_newest(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        let g1 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;

        // Bot-only-humans game (single human): excluded entirely.
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(alice), Some(1), None), (None, Some(2), None)],
        )
        .await;

        let form = recent_form_for_game_type(&pool, &[alice, bob], gt, 10)
            .await
            .expect("query ok");

        let alice_results = form.get(&alice).expect("alice present");
        assert_eq!(alice_results.len(), 2);
        assert_eq!(alice_results[0].game_id, g1);
        assert_eq!(alice_results[1].game_id, g2);

        let bob_results = form.get(&bob).expect("bob present");
        assert_eq!(bob_results.len(), 2);
        assert_eq!(bob_results[0].game_id, g1);
        assert_eq!(bob_results[1].game_id, g2);
    }

    #[sqlx::test]
    async fn recent_form_for_game_type_respects_per_user_limit_and_type_scope(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt1, gv1) = make_game_type(&pool, "Camel Up").await;
        let (_gt2, gv2) = make_game_type(&pool, "Duel").await;

        insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-02 00:00:00),
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;

        // Different game type: excluded from Camel Up results.
        insert_finished_game(
            &pool,
            gv2,
            datetime!(2026-01-03 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        let form = recent_form_for_game_type(&pool, &[alice, bob], gt1, 1)
            .await
            .expect("query ok");

        let alice_results = form.get(&alice).expect("alice present");
        assert_eq!(alice_results.len(), 1);
        assert_eq!(alice_results[0].game_id, g2);
    }

    /// F-152 / wd F55: a finished legacy game with a NULL `finished_at` must
    /// not displace a genuinely recent game from the per-user form window.
    /// PostgreSQL defaults `DESC` to `NULLS FIRST`, so without `NULLS LAST` the
    /// legacy row sorts to `rn = 1` and pushes the oldest dated game out of a
    /// 3-game window. Pre-fix this fails: the legacy row is in the window.
    #[sqlx::test]
    async fn recent_form_for_game_type_null_finished_at_does_not_displace_recent(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        let g1 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;
        let g3 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        // Legacy finished game: is_finished = true but finished_at IS NULL.
        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;
        sqlx::query("UPDATE games SET finished_at = NULL WHERE id = $1")
            .bind(legacy)
            .execute(&pool)
            .await
            .expect("null the legacy finished_at");

        let form = recent_form_for_game_type(&pool, &[alice], gt, 3)
            .await
            .expect("query ok");
        let results = form.get(&alice).expect("alice present");

        let ids: Vec<Uuid> = results.iter().map(|r| r.game_id).collect();
        assert!(
            !ids.contains(&legacy),
            "NULL-finished_at legacy game displaced a recent game: {ids:?}"
        );
        assert_eq!(ids, vec![g1, g2, g3], "window holds the dated games, oldest first");
    }

    /// A historical null-ranked row must be filtered from the qualifying CTE
    /// before `row_number()`, so it cannot displace a valid recent game from a
    /// per-user form window even when it is the newest by `finished_at`.
    #[sqlx::test]
    async fn recent_form_for_game_type_null_ranked_does_not_displace_recent(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        let g1 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;
        let g3 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        // Newest by finished_at but the target user lacks a competitive placing.
        let legacy = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-04 00:00:00),
            &[
                (Some(alice), Some(2), None),
                (Some(bob), Some(1), None),
                (Some(make_user(&pool, "carol").await), Some(3), None),
            ],
        )
        .await;
        sqlx::query(
            "UPDATE game_players SET ranked_placing = NULL WHERE game_id = $1 AND user_id = $2",
        )
        .bind(legacy)
        .bind(alice)
        .execute(&pool)
        .await
        .expect("null alice ranked placing");

        let form = recent_form_for_game_type(&pool, &[alice], gt, 3)
            .await
            .expect("query ok");
        let results = form.get(&alice).expect("alice present");

        let ids: Vec<Uuid> = results.iter().map(|r| r.game_id).collect();
        assert!(
            !ids.contains(&legacy),
            "NULL-ranked legacy game displaced a recent game: {ids:?}"
        );
        assert_eq!(ids, vec![g1, g2, g3], "window holds the ranked games, oldest first");
    }

    #[sqlx::test]
    async fn game_history_pages_by_created_at_desc(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let g1 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        let g2 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(opponent), Some(1), None)],
        )
        .await;
        let g3 = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-03 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        sqlx::query("UPDATE games SET created_at = $1 WHERE id = $2")
            .bind(datetime!(2026-01-01 00:00:00))
            .bind(g1)
            .execute(&pool)
            .await
            .expect("set g1 created_at");
        sqlx::query("UPDATE games SET created_at = $1 WHERE id = $2")
            .bind(datetime!(2026-01-02 00:00:00))
            .bind(g2)
            .execute(&pool)
            .await
            .expect("set g2 created_at");
        sqlx::query("UPDATE games SET created_at = $1 WHERE id = $2")
            .bind(datetime!(2026-01-03 00:00:00))
            .bind(g3)
            .execute(&pool)
            .await
            .expect("set g3 created_at");

        let page1 = game_history(&pool, user, None, None, true, 2, 0, None)
            .await
            .expect("query ok");
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].game_id, g3);
        assert_eq!(page1[1].game_id, g2);

        let page2 = game_history(&pool, user, None, None, true, 2, 2, None)
            .await
            .expect("query ok");
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].game_id, g1);

        let mut all_ids: Vec<Uuid> = page1
            .iter()
            .chain(page2.iter())
            .map(|r| r.game_id)
            .collect();
        all_ids.sort();
        let mut expected = vec![g1, g2, g3];
        expected.sort();
        assert_eq!(all_ids, expected);
    }

    #[sqlx::test]
    async fn game_history_status_filter(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let finished = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        let active = insert_unfinished_game(
            &pool,
            gv,
            &[(Some(user), None, None), (Some(opponent), None, None)],
        )
        .await;

        let only_finished = game_history(&pool, user, Some(true), None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(only_finished.len(), 1);
        assert_eq!(only_finished[0].game_id, finished);
        assert!(only_finished[0].is_finished);

        let only_active = game_history(&pool, user, Some(false), None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(only_active.len(), 1);
        assert_eq!(only_active[0].game_id, active);
        assert!(!only_active[0].is_finished);

        let all = game_history(&pool, user, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn game_history_game_type_filter(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt1, gv1) = make_game_type(&pool, "Camel Up").await;
        let (_gt2, gv2) = make_game_type(&pool, "Duel").await;

        let camel_game = insert_finished_game(
            &pool,
            gv1,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv2,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let camel_only = game_history(&pool, user, None, Some("Camel Up"), true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(camel_only.len(), 1);
        assert_eq!(camel_only[0].game_id, camel_game);
        assert_eq!(camel_only[0].game_type_name, "Camel Up");

        let all = game_history(&pool, user, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn game_history_count_matches_rows(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(2), None), (Some(opponent), Some(1), None)],
        )
        .await;
        insert_unfinished_game(
            &pool,
            gv,
            &[(Some(user), None, None), (Some(opponent), None, None)],
        )
        .await;

        let count_all = game_history_count(&pool, user, None, None, true)
            .await
            .expect("query ok");
        assert_eq!(count_all, 3);

        let count_finished = game_history_count(&pool, user, Some(true), None, true)
            .await
            .expect("query ok");
        assert_eq!(count_finished, 2);

        let rows = game_history(&pool, user, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len() as i64, count_all);
    }

    #[sqlx::test]
    async fn game_history_include_single_human_filter(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let bot_game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(user), Some(1), Some(16)), (None, Some(2), Some(-16))],
        )
        .await;
        let human_game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(user), Some(1), None), (Some(opponent), Some(2), None)],
        )
        .await;

        let excluding = game_history(&pool, user, None, None, false, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(excluding.len(), 1);
        assert_eq!(excluding[0].game_id, human_game);

        let including = game_history(&pool, user, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(including.len(), 2);
        assert!(including.iter().any(|r| r.game_id == bot_game));
    }

    #[sqlx::test]
    async fn game_history_opponents_carry_placing(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
        let opponent = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(user), Some(1), Some(16)),
                (Some(opponent), Some(2), Some(-16)),
            ],
        )
        .await;

        let rows = game_history(&pool, user, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].opponents.len(), 1);
        assert_eq!(rows[0].opponents[0].user_id, Some(opponent));
        assert_eq!(rows[0].opponents[0].name, "bob");
        assert_eq!(rows[0].opponents[0].place, Some(2));
    }

    #[sqlx::test]
    async fn game_history_match_elo_aggregate(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        let rated_game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[
                (Some(alice), Some(1), Some(16)),
                (Some(bob), Some(2), Some(-16)),
            ],
        )
        .await;

        sqlx::query(
            "UPDATE game_players SET rating_before = $1 WHERE game_id = $2 AND user_id = $3",
        )
        .bind(1200)
        .bind(rated_game)
        .bind(alice)
        .execute(&pool)
        .await
        .expect("set alice rating_before");
        sqlx::query(
            "UPDATE game_players SET rating_before = $1 WHERE game_id = $2 AND user_id = $3",
        )
        .bind(1300)
        .bind(rated_game)
        .bind(bob)
        .execute(&pool)
        .await
        .expect("set bob rating_before");

        let unrated_game = insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        let rows = game_history(&pool, alice, None, None, true, 50, 0, None)
            .await
            .expect("query ok");
        assert_eq!(rows.len(), 2);

        let rated_row = rows
            .iter()
            .find(|r| r.game_id == rated_game)
            .expect("rated");
        let elo = rated_row.match_elo.as_ref().expect("match_elo present");
        assert_eq!(elo.min, 1200);
        assert_eq!(elo.max, 1300);
        assert_eq!(elo.avg, 1250);

        let unrated_row = rows
            .iter()
            .find(|r| r.game_id == unrated_game)
            .expect("unrated");
        assert_eq!(unrated_row.match_elo, None);
    }

    #[sqlx::test]
    async fn opponents_by_game_masks_private_opponent(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let stranger = make_user(&pool, "stranger").await;
        let friend = make_user(&pool, "friend").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        crate::db::set_game_visibility(&pool, bob, "private")
            .await
            .unwrap();

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        // Anonymous viewer: bob masked
        let rows = finished_games(&pool, alice, None, false, None, None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].opponents.len(), 1);
        assert_eq!(rows[0].opponents[0].user_id, None);
        assert_eq!(rows[0].opponents[0].name, "Anonymous");

        // Stranger viewer: bob masked
        let rows = finished_games(&pool, alice, None, false, None, Some(stranger))
            .await
            .unwrap();
        assert_eq!(rows[0].opponents[0].user_id, None);
        assert_eq!(rows[0].opponents[0].name, "Anonymous");

        // Self (alice viewing own game): bob is the opponent, alice is the
        // subject - bob is still private to alice. But alice is the player
        // whose games we're listing, so the opponent visibility applies.
        // bob is private, alice is not bob, so bob is masked for alice too.
        let rows = finished_games(&pool, alice, None, false, None, Some(alice))
            .await
            .unwrap();
        assert_eq!(rows[0].opponents[0].user_id, None);
        assert_eq!(rows[0].opponents[0].name, "Anonymous");

        // Now set bob to 'friends' and make friend accepted
        crate::db::set_game_visibility(&pool, bob, "friends")
            .await
            .unwrap();
        crate::db::test_support::accept_friends(&pool, bob, friend).await;

        // Friend viewer: bob visible
        let rows = finished_games(&pool, alice, None, false, None, Some(friend))
            .await
            .unwrap();
        assert_eq!(rows[0].opponents[0].user_id, Some(bob));
        assert_eq!(rows[0].opponents[0].name, "bob");

        // Stranger still masked under 'friends'
        let rows = finished_games(&pool, alice, None, false, None, Some(stranger))
            .await
            .unwrap();
        assert_eq!(rows[0].opponents[0].user_id, None);
        assert_eq!(rows[0].opponents[0].name, "Anonymous");
    }

    #[sqlx::test]
    async fn head_to_head_masks_private_opponent(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let stranger = make_user(&pool, "stranger").await;
        let friend = make_user(&pool, "friend").await;
        let (_gt, gv) = make_game_type(&pool, "Camel Up").await;

        crate::db::set_game_visibility(&pool, bob, "private")
            .await
            .unwrap();

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-01 00:00:00),
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;

        // Anonymous viewer: bob masked but row still present
        let h2h = head_to_head(&pool, alice, "Camel Up", false, None)
            .await
            .unwrap();
        assert_eq!(h2h.len(), 1);
        assert_eq!(h2h[0].user_id, None);
        assert_eq!(h2h[0].name, "Anonymous");
        assert_eq!(h2h[0].games, 1);
        assert_eq!(h2h[0].wins, 1);

        // Stranger: masked
        let h2h = head_to_head(&pool, alice, "Camel Up", false, Some(stranger))
            .await
            .unwrap();
        assert_eq!(h2h[0].user_id, None);
        assert_eq!(h2h[0].name, "Anonymous");

        // Set bob to friends, add friend
        crate::db::set_game_visibility(&pool, bob, "friends")
            .await
            .unwrap();
        crate::db::test_support::accept_friends(&pool, bob, friend).await;

        // Friend: visible
        let h2h = head_to_head(&pool, alice, "Camel Up", false, Some(friend))
            .await
            .unwrap();
        assert_eq!(h2h[0].user_id, Some(bob));
        assert_eq!(h2h[0].name, "bob");

        // Stranger still masked
        let h2h = head_to_head(&pool, alice, "Camel Up", false, Some(stranger))
            .await
            .unwrap();
        assert_eq!(h2h[0].user_id, None);
        assert_eq!(h2h[0].name, "Anonymous");
    }
}
