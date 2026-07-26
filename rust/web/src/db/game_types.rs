#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use uuid::Uuid;

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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use crate::db::test_support::*;
    use crate::db::*;
    use sqlx::postgres::PgPool;
    use uuid::Uuid;

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

    /// ws F35: five untested lookups, batched. The only one with real logic is
    /// `find_latest_non_deprecated_game_version`, which must skip deprecated
    /// rows.
    #[sqlx::test]
    async fn game_and_version_lookups(pool: PgPool) {
        let (game_type_id, gv) = make_game_type_and_version(&pool).await;
        let a = make_user(&pool, "alice").await;
        let game = make_game_with_players(&pool, gv, a.id, &[], 1, &[0]).await;

        // find_game
        let found = find_game(&pool, game.id).await.unwrap().unwrap();
        assert_eq!(found.id, game.id);
        assert_eq!(found.game_version_id, gv);
        assert!(find_game(&pool, Uuid::new_v4()).await.unwrap().is_none());

        // find_game_version
        let version = find_game_version(&pool, gv).await.unwrap().unwrap();
        assert_eq!(version.id, gv);
        assert_eq!(version.game_type_id, game_type_id);
        assert!(!version.is_deprecated);
        assert!(
            find_game_version(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_none()
        );

        // rules default to '' (migrations/004) and round-trip
        assert_eq!(
            find_game_version_rules(&pool, gv).await.unwrap(),
            Some(String::new())
        );
        sqlx::query("UPDATE game_versions SET rules = $1 WHERE id = $2")
            .bind("how to play")
            .bind(gv)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            find_game_version_rules(&pool, gv).await.unwrap(),
            Some("how to play".to_string())
        );
        assert!(
            find_game_version_rules(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_none()
        );

        // render meta
        let (uri, name, iface) = find_game_version_render_meta(&pool, gv)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(uri, "http://localhost:0/mock");
        assert_eq!(name, "1.0.0");
        assert!(
            iface >= 1,
            "interface_version should have a sane default, got {iface}"
        );
        assert!(
            find_game_version_render_meta(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_none()
        );

        // latest non-deprecated must skip a deprecated newer row
        let newer: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, '2.0.0', 'http://localhost:0/mock2', true, true) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let latest = find_latest_non_deprecated_game_version(&pool, game_type_id)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(
            latest.id, newer,
            "a deprecated version must never be chosen"
        );
        assert_eq!(latest.id, gv);
        assert!(
            find_latest_non_deprecated_game_version(&pool, Uuid::new_v4())
                .await
                .unwrap()
                .is_none()
        );
    }
}
