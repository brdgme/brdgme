//! #34 dev-side game import (spec D5): ingests an `ExportBundle` into local
//! Postgres. Dev-only tooling - consumed by the `import-game` binary, never
//! deployed or reachable in prod.
#![cfg(feature = "ssr")]

use crate::game::export::{BUNDLE_SCHEMA_VERSION, ExportBundle};
use anyhow::{Context, anyhow};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub struct ImportOutcome {
    pub game_id: Uuid,
    pub warnings: Vec<String>,
}

pub async fn import_bundle(pool: &PgPool, bundle: &ExportBundle) -> anyhow::Result<ImportOutcome> {
    if bundle.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported bundle schema_version {} (this build supports {})",
            bundle.schema_version,
            BUNDLE_SCHEMA_VERSION
        ));
    }

    let mut warnings = Vec::new();

    // Map the bundle's game type to the local registration by name; the
    // bundle's URI is the exporting environment's and will not resolve here.
    let game_type_id: Uuid = sqlx::query_scalar!(
        "SELECT id FROM game_types WHERE name = $1",
        bundle.game_type_name
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        anyhow!(
            "game type {:?} is not registered locally - start the dev stack so the operator registers it first",
            bundle.game_type_name
        )
    })?;
    let local_version = crate::db::find_latest_non_deprecated_game_version(pool, game_type_id)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "no non-deprecated local game version for {:?}",
                bundle.game_type_name
            )
        })?;
    if local_version.name != bundle.game_version_name {
        warnings.push(format!(
            "bundle was exported from game version {:?} but the local service runs {:?} - the state blob may not load or may behave differently",
            bundle.game_version_name, local_version.name
        ));
    }

    let mut tx = pool.begin().await?;

    let game_id: Uuid = sqlx::query_scalar!(
        "INSERT INTO games (game_version_id, is_finished, finished_at, game_state)
         VALUES ($1, $2, $3, $4) RETURNING id",
        local_version.id,
        bundle.game.is_finished,
        bundle.game.finished_at,
        bundle.game.game_state
    )
    .fetch_one(&mut *tx)
    .await?;

    // Bots: fresh rows keyed by name for the player mapping below.
    let mut bot_ids: HashMap<String, Uuid> = HashMap::new();
    for bot in &bundle.bots {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_bots (game_id, name, difficulty, personality)
             VALUES ($1, $2, $3, $4) RETURNING id",
            game_id,
            bot.name,
            bot.difficulty,
            bot.personality
        )
        .fetch_one(&mut *tx)
        .await?;
        bot_ids.insert(bot.name.clone(), id);
    }

    // Players: placeholder local users for humans (spec D5 - named players,
    // no emails exist in the bundle), bots linked by name.
    let mut player_ids_by_position: HashMap<i32, Uuid> = HashMap::new();
    for player in &bundle.players {
        let (user_id, game_bot_id) = match &player.bot_name {
            Some(bot_name) => (
                None,
                Some(*bot_ids.get(bot_name).ok_or_else(|| {
                    anyhow!(
                        "bundle player {:?} references unknown bot {:?}",
                        player.name,
                        bot_name
                    )
                })?),
            ),
            None => (Some(placeholder_user(&mut tx, &player.name).await?), None),
        };
        let gp_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_players
               (game_id, user_id, game_bot_id, position, color, has_accepted,
                is_turn, is_turn_at, last_turn_at, place, is_eliminated, is_read,
                points, undo_game_state, rating_change)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), $8, $9, true, $10, $11, $12)
             RETURNING id",
            game_id,
            user_id,
            game_bot_id,
            player.position,
            player.color,
            player.has_accepted,
            player.is_turn,
            player.place,
            player.is_eliminated,
            player.points,
            player.undo_game_state,
            player.rating_change
        )
        .fetch_one(&mut *tx)
        .await?;
        player_ids_by_position.insert(player.position, gp_id);

        // Rating rows so game rendering has real game_type_users joins.
        if let Some(user_id) = user_id {
            sqlx::query!(
                "INSERT INTO game_type_users (game_type_id, user_id) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                game_type_id,
                user_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    for log in &bundle.logs {
        let log_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at)
             VALUES ($1, $2, $3, $4) RETURNING id",
            game_id,
            log.body,
            log.is_public,
            log.logged_at
        )
        .fetch_one(&mut *tx)
        .await?;
        for position in &log.target_positions {
            let gp_id = player_ids_by_position.get(position).ok_or_else(|| {
                anyhow!("bundle log targets unknown player position {}", position)
            })?;
            sqlx::query!(
                "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
                log_id,
                gp_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(ImportOutcome { game_id, warnings })
}

/// Uses the bundle's display name when it is a valid, unclaimed username;
/// otherwise generates a fresh one (username rules: migration 009).
async fn placeholder_user(tx: &mut sqlx::PgConnection, name: &str) -> anyhow::Result<Uuid> {
    let taken = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE lower(name) = lower($1)) AS "taken!""#,
        name
    )
    .fetch_one(&mut *tx)
    .await?;
    let final_name = if crate::db::validate_username(name) && !taken {
        name.to_string()
    } else {
        crate::db::generate_unique_username(&mut *tx)
            .await
            .context("generate placeholder username")?
    };
    let id = sqlx::query_scalar!(
        "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        final_name,
        &Vec::<String>::new()
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::export::build_export_bundle;
    use crate::game::server_fns::BotSlot;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn make_exported_game(pool: &PgPool) -> crate::game::export::ExportBundle {
        let creator = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, 'alice', $2)",
            creator,
            &Vec::<String>::new()
        )
        .execute(pool)
        .await
        .unwrap();
        let game_type_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_types (name, player_counts) VALUES ('Lost Cities', $1) RETURNING id",
            &vec![2i32]
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, 'v1', 'http://localhost:0/mock', true, false) RETURNING id",
            game_type_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let game = crate::db::create_game_with_users(
            pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    difficulty: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "prod_state_blob",
            },
        )
        .await
        .unwrap();
        // One public and one private (creator-targeted) log.
        let now = time::OffsetDateTime::now_utc();
        let now_primitive = time::PrimitiveDateTime::new(now.date(), now.time());
        crate::db::insert_game_logs_tx(
            &mut pool.acquire().await.unwrap(),
            game.id,
            vec![
                brdgme_cmd::api::CliLog {
                    content: "public entry".to_string(),
                    at: now_primitive,
                    public: true,
                    to: vec![],
                },
                brdgme_cmd::api::CliLog {
                    content: "private entry".to_string(),
                    at: now_primitive,
                    public: false,
                    to: vec![0],
                },
            ],
        )
        .await
        .unwrap();
        build_export_bundle(pool, game.id).await.unwrap().unwrap()
    }

    #[sqlx::test]
    async fn import_bundle_round_trips_a_game(pool: PgPool) {
        let bundle = make_exported_game(&pool).await;

        let outcome = import_bundle(&pool, &bundle).await.unwrap();
        assert_ne!(outcome.game_id, bundle.game.id);
        // Same local version name as the bundle - no fidelity warning.
        assert!(
            outcome.warnings.is_empty(),
            "warnings: {:?}",
            outcome.warnings
        );

        let ge = crate::db::find_game_extended(&pool, outcome.game_id)
            .await
            .unwrap()
            .expect("imported game exists");
        assert_eq!(ge.game.game_state, "prod_state_blob");
        assert_eq!(ge.game_type.name, "Lost Cities");
        assert_eq!(ge.game_players.len(), 2);
        // Placeholder human user created: "alice" is taken by the original
        // user in this same database, so the import generated a fresh name.
        let human = ge
            .game_players
            .iter()
            .find(|p| p.user.is_some())
            .expect("human seat imported");
        let bot = ge
            .game_players
            .iter()
            .find(|p| p.game_bot.is_some())
            .expect("bot seat imported");
        assert_eq!(bot.game_bot.as_ref().unwrap().name, "Botty");
        assert!(human.user.as_ref().unwrap().id != bundle_original_user_id(&pool).await);

        // Logs and targets came across, remapped to the new player ids.
        let log_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"c!\" FROM game_logs WHERE game_id = $1",
            outcome.game_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(log_count, 2);
        let target_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"c!\" FROM game_log_targets glt
             JOIN game_players gp ON gp.id = glt.game_player_id
             WHERE gp.game_id = $1",
            outcome.game_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(target_count, 1);
    }

    async fn bundle_original_user_id(pool: &PgPool) -> Uuid {
        sqlx::query_scalar!("SELECT id FROM users WHERE name = 'alice'")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn import_bundle_warns_on_version_mismatch(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.game_version_name = "v0-ancient".to_string();

        let outcome = import_bundle(&pool, &bundle).await.unwrap();
        assert!(
            outcome.warnings.iter().any(|w| w.contains("v0-ancient")),
            "expected version-mismatch warning, got {:?}",
            outcome.warnings
        );
    }

    #[sqlx::test]
    async fn import_bundle_errors_when_game_type_missing(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.game_type_name = "No Such Game".to_string();

        let result = import_bundle(&pool, &bundle).await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn import_bundle_rejects_unknown_schema_version(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.schema_version = 999;

        let result = import_bundle(&pool, &bundle).await;
        assert!(result.is_err());
    }
}
