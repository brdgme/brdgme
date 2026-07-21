use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn game_info_header(pool: &PgPool, name: &str) -> Result<Option<(Uuid, String, String)>> {
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, name, blurb FROM game_types WHERE lower(name) = lower($1)")
            .bind(name)
            .fetch_optional(pool)
            .await?;
    Ok(row)
}

pub async fn game_info_rules_version_id(pool: &PgPool, game_type_id: Uuid) -> Result<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM game_versions
         WHERE game_type_id = $1 AND is_public = true AND is_deprecated = false
         ORDER BY name LIMIT 1",
    )
    .bind(game_type_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

pub async fn game_info_total_games(pool: &PgPool, game_type_id: Uuid) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         WHERE gv.game_type_id = $1 AND g.is_finished = true",
    )
    .bind(game_type_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn game_info_active_today(pool: &PgPool, game_type_id: Uuid) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM games g
         JOIN game_versions gv ON gv.id = g.game_version_id
         WHERE gv.game_type_id = $1
           AND g.updated_at >= date_trunc('day', now() AT TIME ZONE 'utc')",
    )
    .bind(game_type_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn game_info_distinct_players(pool: &PgPool, game_type_id: Uuid) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT gp.user_id) FROM game_players gp
         JOIN games g ON g.id = gp.game_id
         JOIN game_versions gv ON gv.id = g.game_version_id
         WHERE gv.game_type_id = $1 AND gp.user_id IS NOT NULL AND g.is_finished = true",
    )
    .bind(game_type_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn game_info_top_ranking(
    pool: &PgPool,
    game_type_id: Uuid,
) -> Result<Vec<(Uuid, String, i32, i32)>> {
    sqlx::query_as::<_, (Uuid, String, i32, i32)>(
        "SELECT gtu.user_id, u.name, gtu.rating, gtu.peak_rating
         FROM game_type_users gtu
         JOIN users u ON u.id = gtu.user_id
         WHERE gtu.game_type_id = $1
         ORDER BY gtu.rating DESC, u.name
         LIMIT 10",
    )
    .bind(game_type_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::PrimitiveDateTime;
    use time::macros::datetime;

    async fn make_user(pool: &PgPool, name: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO users (id, name, pref_colors)
             VALUES (uuid_generate_v4(), $1, '{}') RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("insert user")
    }

    async fn make_game_type(pool: &PgPool, name: &str) -> (Uuid, Uuid) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (id, name, player_counts)
             VALUES (uuid_generate_v4(), $1, '{2,3,4}') RETURNING id",
        )
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("insert game_type");

        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (id, game_type_id, name, uri, is_public, is_deprecated)
             VALUES (uuid_generate_v4(), $1, '1.0.0', 'http://localhost:0/mock', true, false)
             RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(pool)
        .await
        .expect("insert game_version");

        (game_type_id, game_version_id)
    }

    async fn insert_finished_game(
        pool: &PgPool,
        game_version_id: Uuid,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        insert_game(pool, game_version_id, true, players).await
    }

    async fn insert_unfinished_game(
        pool: &PgPool,
        game_version_id: Uuid,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        insert_game(pool, game_version_id, false, players).await
    }

    async fn insert_game(
        pool: &PgPool,
        game_version_id: Uuid,
        is_finished: bool,
        players: &[(Option<Uuid>, Option<i32>, Option<i32>)],
    ) -> Uuid {
        let game_id: Uuid = sqlx::query_scalar(
            "INSERT INTO games (id, game_version_id, is_finished, finished_at, game_state)
             VALUES (uuid_generate_v4(), $1, $2, CASE WHEN $2 THEN now() ELSE NULL END, '')
             RETURNING id",
        )
        .bind(game_version_id)
        .bind(is_finished)
        .fetch_one(pool)
        .await
        .expect("insert game");

        const COLORS: [&str; 8] = [
            "Green", "Red", "Blue", "Orange", "Purple", "Brown", "Cyan", "Pink",
        ];
        for (i, (user_id, place, rating_change)) in players.iter().enumerate() {
            let game_bot_id: Option<Uuid> = match user_id {
                Some(_) => None,
                None => Some(
                    sqlx::query_scalar(
                        "INSERT INTO game_bots (id, game_id, name, bot_name)
                         VALUES (uuid_generate_v4(), $1, $2, 'medium') RETURNING id",
                    )
                    .bind(game_id)
                    .bind(format!("bot-{i}"))
                    .fetch_one(pool)
                    .await
                    .expect("insert game_bot"),
                ),
            };

            sqlx::query(
                r#"INSERT INTO game_players
                    (id, game_id, user_id, game_bot_id, "position", color, has_accepted,
                     is_turn, is_turn_at, last_turn_at, is_eliminated, is_read, place, rating_change)
                   VALUES (uuid_generate_v4(), $1, $2, $3, $4, $5, true, false, now(), now(), false, true, $6, $7)"#,
            )
            .bind(game_id)
            .bind(*user_id)
            .bind(game_bot_id)
            .bind(i as i32)
            .bind(COLORS[i % COLORS.len()])
            .bind(*place)
            .bind(*rating_change)
            .execute(pool)
            .await
            .expect("insert game_player");
        }

        game_id
    }

    async fn set_game_type_rating(
        pool: &PgPool,
        game_type_id: Uuid,
        user_id: Uuid,
        rating: i32,
        peak: i32,
    ) {
        sqlx::query(
            "INSERT INTO game_type_users (id, game_type_id, user_id, rating, peak_rating)
             VALUES (uuid_generate_v4(), $1, $2, $3, $4)
             ON CONFLICT (game_type_id, user_id) DO UPDATE SET rating = $3, peak_rating = $4",
        )
        .bind(game_type_id)
        .bind(user_id)
        .bind(rating)
        .bind(peak)
        .execute(pool)
        .await
        .expect("upsert game_type_users");
    }

    async fn backdate_updated_at(pool: &PgPool, game_id: Uuid, to: PrimitiveDateTime) {
        sqlx::query("ALTER TABLE games DISABLE TRIGGER update_games_updated_at")
            .execute(pool)
            .await
            .expect("disable trigger");
        sqlx::query("UPDATE games SET updated_at = $1 WHERE id = $2")
            .bind(to)
            .bind(game_id)
            .execute(pool)
            .await
            .expect("backdate updated_at");
        sqlx::query("ALTER TABLE games ENABLE TRIGGER update_games_updated_at")
            .execute(pool)
            .await
            .expect("enable trigger");
    }

    #[sqlx::test]
    async fn total_games_counts_only_finished(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv,
            &[(Some(alice), Some(2), None), (Some(bob), Some(1), None)],
        )
        .await;
        insert_unfinished_game(
            &pool,
            gv,
            &[(Some(alice), None, None), (Some(bob), None, None)],
        )
        .await;

        assert_eq!(game_info_total_games(&pool, gt).await.expect("query ok"), 2);
    }

    #[sqlx::test]
    async fn active_today_counts_only_games_updated_today(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        let finished = insert_finished_game(
            &pool,
            gv,
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        insert_unfinished_game(
            &pool,
            gv,
            &[(Some(alice), None, None), (Some(bob), None, None)],
        )
        .await;

        assert_eq!(
            game_info_active_today(&pool, gt).await.expect("query ok"),
            2
        );

        backdate_updated_at(&pool, finished, datetime!(2020-01-01 00:00:00)).await;

        assert_eq!(
            game_info_active_today(&pool, gt).await.expect("query ok"),
            1
        );
    }

    #[sqlx::test]
    async fn distinct_players_counts_distinct_humans_only(pool: PgPool) {
        let alice = make_user(&pool, "alice").await;
        let bob = make_user(&pool, "bob").await;
        let (gt, gv) = make_game_type(&pool, "Camel Up").await;

        insert_finished_game(
            &pool,
            gv,
            &[(Some(alice), Some(1), None), (Some(bob), Some(2), None)],
        )
        .await;
        insert_finished_game(
            &pool,
            gv,
            &[(Some(alice), Some(1), None), (None, Some(2), None)],
        )
        .await;
        insert_unfinished_game(&pool, gv, &[(Some(bob), None, None), (None, None, None)]).await;

        assert_eq!(
            game_info_distinct_players(&pool, gt)
                .await
                .expect("query ok"),
            2
        );
    }

    #[sqlx::test]
    async fn top_ranking_orders_by_rating_desc_and_limits_to_10(pool: PgPool) {
        let (gt, _gv) = make_game_type(&pool, "Camel Up").await;

        for i in 0..11 {
            let user = make_user(&pool, &format!("player-{i:02}")).await;
            set_game_type_rating(&pool, gt, user, 1200 + i, 1200 + i).await;
        }

        let ranking = game_info_top_ranking(&pool, gt).await.expect("query ok");
        assert_eq!(ranking.len(), 10);
        assert_eq!(ranking[0].1, "player-10");
        assert_eq!(ranking[0].2, 1210);
        assert_eq!(ranking[9].1, "player-01");
        assert_eq!(ranking[9].2, 1201);
    }
}
