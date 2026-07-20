use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
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

pub async fn overall_totals(
    pool: &PgPool,
    user_id: Uuid,
    include_single_human: bool,
) -> Result<super::OverallTotals> {
    let row = sqlx::query!(
        r#"
        SELECT
            count(*) AS "finished_games!",
            count(*) FILTER (WHERE gp.place = 1) AS "wins!"
        FROM game_players gp
        JOIN games g ON g.id = gp.game_id
        WHERE gp.user_id = $1
          AND g.is_finished = true
          AND (
              SELECT count(*) FROM game_players gp2
              WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL
          ) >= CASE WHEN $2 THEN 1 ELSE 2 END
        "#,
        user_id,
        include_single_human
    )
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

pub async fn game_type_stats(
    pool: &PgPool,
    user_id: Uuid,
    include_single_human: bool,
) -> Result<Vec<super::GameTypeStats>> {
    let rows = sqlx::query!(
        r#"
        WITH qualifying AS (
            SELECT
                gt.id AS game_type_id,
                gt.name AS game_type_name,
                gp.place,
                (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS n
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL
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
            COALESCE(agg.game_type_name, gt.name) AS "game_type_name!",
            COALESCE(agg.games, 0) AS "games!",
            COALESCE(agg.wins, 0) AS "wins!",
            agg.avg_place_percentile AS avg_place_percentile,
            gtu.rating AS "rating?",
            gtu.peak_rating AS "peak_rating?"
        FROM agg
        FULL OUTER JOIN (
            SELECT game_type_id, rating, peak_rating FROM game_type_users WHERE user_id = $1
        ) gtu ON gtu.game_type_id = agg.game_type_id
        LEFT JOIN game_types gt ON gt.id = gtu.game_type_id
        ORDER BY "game_type_name!"
        "#,
        user_id,
        include_single_human
    )
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

    let mut rating = 1200;
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

/// Other seats (not `user_id`'s own) for each game in `game_ids`, grouped by
/// game id and ordered by seat position within each game.
async fn opponents_by_game(
    pool: &PgPool,
    game_ids: &[Uuid],
    user_id: Uuid,
) -> Result<HashMap<Uuid, Vec<super::Opponent>>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            gp.game_id,
            u.id AS "user_id?",
            COALESCE(u.name, gb.name, 'Bot') AS "name!"
        FROM game_players gp
        LEFT JOIN users u ON u.id = gp.user_id
        LEFT JOIN game_bots gb ON gb.id = gp.game_bot_id
        WHERE gp.game_id = ANY($1) AND gp.user_id IS DISTINCT FROM $2
        ORDER BY gp.game_id, gp.position
        "#,
        game_ids,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut by_game: HashMap<Uuid, Vec<super::Opponent>> = HashMap::new();
    for row in rows {
        by_game
            .entry(row.game_id)
            .or_default()
            .push(super::Opponent {
                user_id: row.user_id,
                name: row.name,
            });
    }
    Ok(by_game)
}

pub async fn finished_games(
    pool: &PgPool,
    user_id: Uuid,
    game_type_name: Option<&str>,
    include_single_human: bool,
    limit: Option<i64>,
) -> Result<Vec<super::FinishedGameRow>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            g.id AS game_id,
            gt.name AS game_type_name,
            g.finished_at,
            gp.place,
            gp.rating_change,
            (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS "player_count!"
        FROM game_players gp
        JOIN games g ON g.id = gp.game_id
        JOIN game_versions gv ON gv.id = g.game_version_id
        JOIN game_types gt ON gt.id = gv.game_type_id
        WHERE gp.user_id = $1
          AND g.is_finished = true
          AND ($3::text IS NULL OR gt.name = $3)
          AND (
              SELECT count(*) FROM game_players gp3
              WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL
          ) >= CASE WHEN $2 THEN 1 ELSE 2 END
        ORDER BY g.finished_at DESC, g.id
        LIMIT $4::bigint
        "#,
        user_id,
        include_single_human,
        game_type_name,
        limit
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let game_ids: Vec<Uuid> = rows.iter().map(|row| row.game_id).collect();
    let mut opponents = opponents_by_game(pool, &game_ids, user_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| super::FinishedGameRow {
            game_id: row.game_id,
            game_type_name: row.game_type_name,
            finished_at: row.finished_at,
            place: row.place,
            player_count: row.player_count,
            rating_change: row.rating_change,
            opponents: opponents.remove(&row.game_id).unwrap_or_default(),
        })
        .collect())
}

pub async fn active_games(pool: &PgPool, user_id: Uuid) -> Result<Vec<super::ActiveGameRow>> {
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
    let mut opponents = opponents_by_game(pool, &game_ids, user_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| super::ActiveGameRow {
            game_id: row.game_id,
            game_type_name: row.game_type_name,
            is_turn: row.is_turn,
            opponents: opponents.remove(&row.game_id).unwrap_or_default(),
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn head_to_head(
    pool: &PgPool,
    user_id: Uuid,
    game_type_name: &str,
    include_single_human: bool,
) -> Result<Vec<super::HeadToHead>> {
    let rows = sqlx::query!(
        r#"
        WITH qualifying AS (
            SELECT g.id AS game_id, gp.place
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND gt.name = $2
              AND (
                  SELECT count(*) FROM game_players gp2
                  WHERE gp2.game_id = g.id AND gp2.user_id IS NOT NULL
              ) >= CASE WHEN $3 THEN 1 ELSE 2 END
        ),
        opponent_rows AS (
            SELECT
                q.place AS my_place,
                gp.user_id AS opp_id,
                u.name AS opp_name,
                gp.place AS opp_place
            FROM qualifying q
            JOIN game_players gp
                ON gp.game_id = q.game_id AND gp.user_id IS NOT NULL AND gp.user_id <> $1
            JOIN users u ON u.id = gp.user_id
        )
        SELECT
            opp_id AS "user_id!",
            opp_name AS "name!",
            count(*) AS "games!",
            count(*) FILTER (
                WHERE my_place IS NOT NULL AND opp_place IS NOT NULL AND my_place < opp_place
            ) AS "wins!",
            count(*) FILTER (
                WHERE my_place IS NOT NULL AND opp_place IS NOT NULL AND my_place > opp_place
            ) AS "losses!",
            count(*) FILTER (
                WHERE my_place IS NOT NULL AND opp_place IS NOT NULL AND my_place = opp_place
            ) AS "ties!"
        FROM opponent_rows
        GROUP BY opp_id, opp_name
        ORDER BY "games!" DESC, opp_name
        "#,
        user_id,
        game_type_name,
        include_single_human
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| super::HeadToHead {
            user_id: row.user_id,
            name: row.name,
            games: row.games,
            wins: row.wins,
            losses: row.losses,
            ties: row.ties,
        })
        .collect())
}

pub async fn recent_form(
    pool: &PgPool,
    user_id: Uuid,
    per_type: i64,
    include_single_human: bool,
) -> Result<Vec<super::GameTypeForm>> {
    let rows = sqlx::query!(
        r#"
        WITH qualifying AS (
            SELECT
                gt.name AS game_type_name,
                g.id AS game_id,
                g.finished_at,
                gp.place,
                gp.rating_change,
                (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS player_count,
                row_number() OVER (
                    PARTITION BY gt.id ORDER BY g.finished_at DESC, g.id
                ) AS rn
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = $1
              AND g.is_finished = true
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL
              ) >= CASE WHEN $3 THEN 1 ELSE 2 END
        )
        SELECT
            game_type_name AS "game_type_name!",
            game_id AS "game_id!",
            finished_at,
            place,
            rating_change,
            player_count AS "player_count!"
        FROM qualifying
        WHERE rn <= $2
        ORDER BY "game_type_name!", finished_at ASC, "game_id!"
        "#,
        user_id,
        per_type,
        include_single_human
    )
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
pub async fn recent_form_for_game_type(
    pool: &PgPool,
    user_ids: &[Uuid],
    game_type_id: Uuid,
    per_user: i64,
) -> Result<HashMap<Uuid, Vec<super::FormResult>>> {
    let rows = sqlx::query!(
        r#"
        WITH qualifying AS (
            SELECT
                gp.user_id AS user_id,
                g.id AS game_id,
                g.finished_at,
                gp.place,
                gp.rating_change,
                (SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id) AS player_count,
                row_number() OVER (
                    PARTITION BY gp.user_id ORDER BY g.finished_at DESC, g.id
                ) AS rn
            FROM game_players gp
            JOIN games g ON g.id = gp.game_id
            JOIN game_versions gv ON gv.id = g.game_version_id
            JOIN game_types gt ON gt.id = gv.game_type_id
            WHERE gp.user_id = ANY($1)
              AND gt.id = $2
              AND g.is_finished = true
              AND (
                  SELECT count(*) FROM game_players gp3
                  WHERE gp3.game_id = g.id AND gp3.user_id IS NOT NULL
              ) >= 2
        )
        SELECT
            user_id AS "user_id!",
            game_id AS "game_id!",
            finished_at,
            place,
            rating_change,
            player_count AS "player_count!"
        FROM qualifying
        WHERE rn <= $3
        ORDER BY user_id, finished_at ASC, "game_id!"
        "#,
        user_ids,
        game_type_id,
        per_user
    )
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

            sqlx::query!(
                r#"INSERT INTO game_players
                    (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
                     is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
                   VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, true, false, now(), now(), false, true, $6, $7)"#,
                game_id,
                *user_id,
                game_bot_id,
                i as i32,
                COLORS[i % COLORS.len()],
                *place,
                *rating_change
            )
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

        let stats = game_type_stats(&pool, user, false).await.expect("query ok");
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

    #[sqlx::test]
    async fn game_type_stats_computes_avg_place_percentile(pool: PgPool) {
        let user = make_user(&pool, "alice").await;
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

        insert_finished_game(
            &pool,
            gv,
            datetime!(2026-01-02 00:00:00),
            &[
                (Some(user), Some(3), None),
                (None, Some(1), None),
                (None, Some(2), None),
            ],
        )
        .await;

        let stats = game_type_stats(&pool, user, true).await.expect("query ok");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].avg_place_percentile, Some(0.5));

        let (_gt2, gv2) = make_game_type(&pool, "Duel").await;
        insert_finished_game(
            &pool,
            gv2,
            datetime!(2026-01-03 00:00:00),
            &[(Some(user), Some(2), None), (None, Some(1), None)],
        )
        .await;

        let stats2 = game_type_stats(&pool, user, true).await.expect("query ok");
        let duel = stats2
            .iter()
            .find(|s| s.game_type_name == "Duel")
            .expect("duel present");
        assert_eq!(duel.avg_place_percentile, Some(0.0));
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

        let stats = game_type_stats(&pool, user, false).await.expect("query ok");

        assert!(
            !stats.iter().any(|s| s.game_type_name == "Zebra Game"),
            "other user's game_type_users row leaked into this user's stats: {stats:?}"
        );
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].game_type_name, "Camel Up");
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

        let final_row = game_type_stats(&pool, user, false)
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

        let all = finished_games(&pool, user, None, true, None)
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
        assert_eq!(row1.place, Some(1));
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

        let excluding_single = finished_games(&pool, user, None, false, None)
            .await
            .expect("query ok");
        assert!(!excluding_single.iter().any(|r| r.game_id == game2));
        assert_eq!(excluding_single.len(), 2);

        let limited = finished_games(&pool, user, None, true, Some(1))
            .await
            .expect("query ok");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].game_id, game3);

        let camel_only = finished_games(&pool, user, Some("Camel Up"), true, None)
            .await
            .expect("query ok");
        assert_eq!(camel_only.len(), 2);
        assert!(camel_only.iter().all(|r| r.game_type_name == "Camel Up"));
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

        let active = active_games(&pool, user).await.expect("query ok");
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

        let h2h = head_to_head(&pool, user, "Camel Up", false)
            .await
            .expect("query ok");
        assert_eq!(h2h.len(), 1);
        assert_eq!(h2h[0].user_id, opponent);
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

        let h2h = head_to_head(&pool, user, "Camel Up", true)
            .await
            .expect("query ok");
        assert!(h2h.is_empty());
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
}
