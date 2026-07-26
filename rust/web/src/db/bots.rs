#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

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
pub async fn validate_bot_slots(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    bot_slots: &[crate::game::server_fns::BotSlot],
) -> Result<Option<String>> {
    if bot_slots.is_empty() {
        return Ok(None);
    }
    let valid_names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM bots WHERE enabled = true ORDER BY display_order")
            .fetch_all(executor)
            .await
            .map_err(|e| anyhow::anyhow!("validate_bot_slots: {e}"))?;
    for slot in bot_slots {
        if slot.name.trim().is_empty() {
            return Ok(Some("Bot display name cannot be empty".to_string()));
        }
        if !valid_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&slot.bot_name))
        {
            return Ok(Some(format!(
                "'{}' is not a valid bot type. Valid bot types: {}",
                slot.bot_name,
                valid_names.join(", ")
            )));
        }
    }
    Ok(None)
}

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
        "INSERT INTO game_bots (game_id, name, bot_name) VALUES ($1, $2, $3) RETURNING id, game_id, name, bot_name",
        game_id,
        name,
        name
    )
    .fetch_one(pool)
    .await?;
    Ok(Some(bot))
}

#[cfg(feature = "ssr")]
pub async fn replacement_bot_available(pool: &PgPool) -> Result<bool> {
    let exists: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM bots WHERE can_replace_humans = true AND enabled = true)",
    )
    .fetch_optional(pool)
    .await?;
    Ok(exists.map(|(b,)| b).unwrap_or(false))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

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

    #[sqlx::test]
    async fn pick_replacement_bot_requires_flag(pool: PgPool) {
        let creator = make_user(&pool, "creator").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, creator.id, &[], 0, &[0]).await;

        assert!(
            pick_replacement_bot(&pool, game.id)
                .await
                .unwrap()
                .is_none()
        );

        sqlx::query("INSERT INTO bots (name, can_replace_humans) VALUES ('Hard', true)")
            .execute(&pool)
            .await
            .unwrap();
        let picked = pick_replacement_bot(&pool, game.id).await.unwrap();
        assert!(picked.is_some());
        assert_eq!(picked.unwrap().bot_name, "Hard");
    }

    /// ws F35: neither bot lookup had a test. `find_enabled_bots` returns only
    /// enabled bots ordered by display_order; `replacement_bot_available`
    /// requires BOTH `enabled = true` AND `can_replace_humans = true`
    /// (column added by migrations/022_concede_bot_replacement.sql:16,
    /// defaulting to false).
    ///
    /// NOTE: migrations/013_bot_efficacy.sql:41-44 seeds three enabled bots
    /// ('easy' 0, 'medium' 1, 'hard' 2), all with can_replace_humans = false,
    /// so the baseline here is NOT an empty table and those three names are
    /// already taken (`bots.name` is UNIQUE).
    #[sqlx::test]
    async fn bot_lookups_respect_enabled_and_can_replace_humans(pool: PgPool) {
        // Seeded baseline.
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec!["easy".to_string(), "medium".to_string(), "hard".to_string()],
            "the three seeded bots, ordered by display_order"
        );
        assert!(
            !replacement_bot_available(&pool).await.unwrap(),
            "no seeded bot has can_replace_humans"
        );

        // A DISABLED bot is excluded from find_enabled_bots and must not make a
        // replacement available even though it can replace humans.
        sqlx::query(
            "INSERT INTO bots (name, display_order, enabled, can_replace_humans)
             VALUES ('offbot', 3, false, true)",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec!["easy".to_string(), "medium".to_string(), "hard".to_string()],
            "a disabled bot must be excluded"
        );
        assert!(
            !replacement_bot_available(&pool).await.unwrap(),
            "can_replace_humans on a DISABLED bot must not count"
        );

        // Enabled AND flagged -> available.
        sqlx::query("UPDATE bots SET can_replace_humans = true WHERE name = 'easy'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(replacement_bot_available(&pool).await.unwrap());

        // Ordering is display_order, not name or insertion order.
        sqlx::query("UPDATE bots SET display_order = 99 WHERE name = 'easy'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            find_enabled_bots(&pool).await.unwrap(),
            vec!["medium".to_string(), "hard".to_string(), "easy".to_string()],
            "ordered by display_order"
        );
    }

    #[sqlx::test]
    async fn validate_bot_slots_accepts_enabled_bot(pool: PgPool) {
        let slots = vec![crate::game::server_fns::BotSlot {
            name: "My Bot".to_string(),
            bot_name: "easy".to_string(),
        }];
        assert_eq!(validate_bot_slots(&pool, &slots).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn validate_bot_slots_accepts_case_mismatch(pool: PgPool) {
        let slots = vec![crate::game::server_fns::BotSlot {
            name: "My Bot".to_string(),
            bot_name: "EASY".to_string(),
        }];
        assert_eq!(validate_bot_slots(&pool, &slots).await.unwrap(), None);
    }

    #[sqlx::test]
    async fn validate_bot_slots_rejects_unknown_type(pool: PgPool) {
        let slots = vec![crate::game::server_fns::BotSlot {
            name: "My Bot".to_string(),
            bot_name: "garbage".to_string(),
        }];
        let result = validate_bot_slots(&pool, &slots).await.unwrap();
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(
            msg.contains("garbage"),
            "message should name the offending bot: {msg}"
        );
        assert!(
            msg.contains("easy"),
            "message should list valid bots: {msg}"
        );
    }

    #[sqlx::test]
    async fn validate_bot_slots_rejects_disabled_bot(pool: PgPool) {
        sqlx::query("UPDATE bots SET enabled = false WHERE name = 'hard'")
            .execute(&pool)
            .await
            .unwrap();
        let slots = vec![crate::game::server_fns::BotSlot {
            name: "My Bot".to_string(),
            bot_name: "hard".to_string(),
        }];
        let result = validate_bot_slots(&pool, &slots).await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("hard"));
    }

    #[sqlx::test]
    async fn validate_bot_slots_rejects_empty_display_name(pool: PgPool) {
        let slots = vec![crate::game::server_fns::BotSlot {
            name: "   ".to_string(),
            bot_name: "easy".to_string(),
        }];
        let result = validate_bot_slots(&pool, &slots).await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("empty"));
    }

    #[sqlx::test]
    async fn validate_bot_slots_accepts_empty_slice(pool: PgPool) {
        assert_eq!(validate_bot_slots(&pool, &[]).await.unwrap(), None);
    }
}
