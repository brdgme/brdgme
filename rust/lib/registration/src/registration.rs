use sqlx::{Acquire, PgPool};
use thiserror::Error;
use uuid::Uuid;

use crate::GameVersionManifest;

#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    pub type_name: String,
    pub version_name: String,
    pub weight: f32,
    pub blurb: String,
    pub is_deprecated: bool,
    pub interface_version: i32,
    pub player_counts: Vec<i32>,
    pub uri: String,
    pub rules: String,
}

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("database error: {0}")]
    Sql(#[from] sqlx::Error),
}

/// Outcome of a set reconciliation: how many versions were made public and how
/// many stored versions were demoted to non-public.
#[derive(Debug, Clone, PartialEq)]
pub struct SetStats {
    pub registered: usize,
    pub demoted: u64,
}

impl Registration {
    /// Builds the registration for a canonical manifest, filling in the
    /// runtime-observed values (direct URI, player counts, rules) that the
    /// manifest itself does not carry.
    pub fn from_manifest(
        manifest: &GameVersionManifest,
        uri: String,
        player_counts: Vec<i32>,
        rules: String,
    ) -> Self {
        Self {
            type_name: manifest.spec.type_name.clone(),
            version_name: manifest.metadata.name.clone(),
            weight: manifest.spec.weight,
            blurb: manifest.spec.blurb.clone(),
            is_deprecated: manifest.spec.is_deprecated,
            interface_version: manifest.spec.interface_version,
            player_counts,
            uri,
            rules,
        }
    }
}

/// Upserts the `game_types` and `game_versions` rows for one game version and
/// returns the `game_types.id` for its type. The operator reconciler (`apply`)
/// and the local registration CLI both call this so the two paths persist
/// identical rows. Accepts anything that can acquire a connection or borrow a
/// transaction, so the set path reuses it inside its transaction.
pub async fn upsert<'c, A>(acquire: A, reg: &Registration) -> Result<Uuid, RegistrationError>
where
    A: Acquire<'c, Database = sqlx::Postgres>,
{
    let mut conn = acquire.acquire().await?;
    let game_type_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO game_types (name, player_counts, weight, blurb)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (name) DO UPDATE
            SET updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(&reg.type_name)
    .bind(&reg.player_counts)
    .bind(reg.weight)
    .bind(&reg.blurb)
    .fetch_one(&mut *conn)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated, interface_version, rules)
        VALUES ($1, $2, $3, true, $4, $5, $6)
        ON CONFLICT (game_type_id, name) DO UPDATE
            SET uri               = EXCLUDED.uri,
                is_public         = true,
                is_deprecated     = EXCLUDED.is_deprecated,
                interface_version = EXCLUDED.interface_version,
                rules             = EXCLUDED.rules,
                updated_at        = NOW()
        "#,
    )
    .bind(game_type_id)
    .bind(&reg.version_name)
    .bind(&reg.uri)
    .bind(reg.is_deprecated)
    .bind(reg.interface_version)
    .bind(&reg.rules)
    .execute(&mut *conn)
    .await?;

    if !reg.is_deprecated {
        sqlx::query(
            r#"
            UPDATE game_types
            SET player_counts = $2,
                weight        = $3,
                blurb         = $4,
                updated_at    = NOW()
            WHERE id = $1
              AND NOT EXISTS (
                  SELECT 1
                  FROM game_versions newer
                  WHERE newer.game_type_id = $1
                    AND newer.is_deprecated = false
                    AND (newer.created_at, newer.name) > (
                        SELECT cur.created_at, cur.name
                        FROM game_versions cur
                        WHERE cur.game_type_id = $1
                          AND cur.name = $5
                    )
              )
            "#,
        )
        .bind(game_type_id)
        .bind(&reg.player_counts)
        .bind(reg.weight)
        .bind(&reg.blurb)
        .bind(&reg.version_name)
        .execute(&mut *conn)
        .await?;
    }

    Ok(game_type_id)
}

/// Idempotently reconciles the stored set to exactly `regs`: every requested
/// exact (game type, version) is upserted public with its persisted URI and
/// every stored version outside the set is demoted non-public. All upserts
/// and the demotion run in one transaction, so any database error rolls the
/// whole set back. Rows are never deleted. The set must already be
/// deduplicated on (game type, version); callers validate input before
/// calling.
pub async fn bulk_set(pool: &PgPool, regs: &[Registration]) -> Result<SetStats, RegistrationError> {
    let mut tx = pool.begin().await?;
    let mut game_type_ids = Vec::with_capacity(regs.len());
    let mut version_names = Vec::with_capacity(regs.len());
    for reg in regs {
        game_type_ids.push(upsert(&mut tx, reg).await?);
        version_names.push(reg.version_name.clone());
    }
    let demoted = sqlx::query(
        r#"
        UPDATE game_versions
        SET is_public = false, updated_at = NOW()
        WHERE NOT EXISTS (
            SELECT 1
            FROM unnest($1::uuid[], $2::text[]) AS selected(game_type_id, name)
            WHERE selected.game_type_id = game_versions.game_type_id
              AND selected.name = game_versions.name
        )
        "#,
    )
    .bind(&game_type_ids)
    .bind(&version_names)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(SetStats {
        registered: regs.len(),
        demoted,
    })
}

/// Flips `is_public` for one game version. The operator's finalizer cleanup
/// uses it to hide a deleted version; the local CLI uses it when demoting a
/// non-selected version.
pub async fn set_public(
    pool: &PgPool,
    version_name: &str,
    type_name: &str,
    is_public: bool,
) -> Result<(), RegistrationError> {
    sqlx::query(
        "UPDATE game_versions SET is_public = $3, updated_at = NOW() \
         WHERE name = $1 AND game_type_id = (SELECT id FROM game_types WHERE name = $2)",
    )
    .bind(version_name)
    .bind(type_name)
    .bind(is_public)
    .execute(pool)
    .await?;
    Ok(())
}

/// Marks every stored game version except `keep_version_name` non-public.
/// The Compose lane registers all 27 deployable games as peer services in one
/// bulk set, so a version is public only while it is in that set. Rows are
/// never deleted. Returns the number of rows demoted.
/// Accepts anything that can acquire a connection or borrow a transaction, so
/// callers that need the demotion to be atomic with a preceding upsert run it
/// inside the same transaction.
pub async fn mark_others_non_public<'c, A>(
    acquire: A,
    keep_version_name: &str,
) -> Result<u64, RegistrationError>
where
    A: Acquire<'c, Database = sqlx::Postgres>,
{
    let mut conn = acquire.acquire().await?;
    Ok(sqlx::query(
        "UPDATE game_versions SET is_public = false, updated_at = NOW() WHERE name <> $1",
    )
    .bind(keep_version_name)
    .execute(&mut *conn)
    .await?
    .rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIC_TAC_TOE_MANIFEST: &str =
        include_str!("../../../../k8s/base/game/tic-tac-toe-2/game-version.yaml");

    fn tic_tac_toe_registration(uri: &str) -> Registration {
        Registration {
            type_name: "Tic-tac-toe".to_string(),
            version_name: "tic-tac-toe-2".to_string(),
            weight: 1.0,
            blurb: "Take turns marking the grid and make three in a row before your opponent does. The old classic, for one quick game or ten.".to_string(),
            is_deprecated: false,
            interface_version: 2,
            player_counts: vec![2],
            uri: uri.to_string(),
            rules: "rules text".to_string(),
        }
    }

    #[test]
    fn manifest_parses_canonical_fields() {
        let manifest: GameVersionManifest = TIC_TAC_TOE_MANIFEST.parse().unwrap();
        assert_eq!(manifest.metadata.name, "tic-tac-toe-2");
        assert_eq!(manifest.spec.type_name, "Tic-tac-toe");
        assert_eq!(manifest.spec.weight, 1.0);
        assert_eq!(manifest.spec.interface_version, 2);
        assert!(!manifest.spec.is_deprecated);
    }

    #[test]
    fn manifest_defaults_apply() {
        let manifest: GameVersionManifest = r#"
            apiVersion: brdgme.com/v1
            kind: GameVersion
            metadata:
              name: some-game-1
            spec:
              typeName: Some Game
            "#
        .parse()
        .unwrap();
        assert_eq!(manifest.spec.interface_version, 1);
        assert_eq!(manifest.spec.weight, 0.0);
        assert_eq!(manifest.spec.blurb, "");
        assert!(!manifest.spec.is_deprecated);
    }

    #[test]
    fn from_manifest_maps_canonical_metadata() {
        let manifest = TIC_TAC_TOE_MANIFEST.parse().unwrap();
        let registration = Registration::from_manifest(
            &manifest,
            "http://127.0.0.1:8080".to_string(),
            vec![2],
            "rules".to_string(),
        );
        assert_eq!(registration.type_name, "Tic-tac-toe");
        assert_eq!(registration.version_name, "tic-tac-toe-2");
        assert_eq!(registration.interface_version, 2);
        assert_eq!(registration.uri, "http://127.0.0.1:8080");
    }

    // Applies the web crate's migrations so the schema matches production.
    // The operator itself never runs migrations (docs/DEV.md).
    #[sqlx::test(migrations = "../../web/migrations")]
    async fn local_and_operator_paths_persist_equivalent_rows(pool: PgPool) {
        let manifest = TIC_TAC_TOE_MANIFEST.parse().unwrap();

        // Local path: canonical manifest + direct localhost URL + values
        // queried from the running host game.
        let local = Registration::from_manifest(
            &manifest,
            "http://127.0.0.1:8080".to_string(),
            vec![2],
            "rules text".to_string(),
        );
        upsert(&pool, &local).await.unwrap();

        // Operator path: the reconciler maps the same canonical spec fields
        // (identical serde shape to the GameVersion CRD) plus the in-cluster
        // URI and the same queried values, then calls the same shared upsert.
        let operator = Registration {
            type_name: manifest.spec.type_name.clone(),
            version_name: manifest.metadata.name.clone(),
            weight: manifest.spec.weight,
            blurb: manifest.spec.blurb.clone(),
            is_deprecated: manifest.spec.is_deprecated,
            interface_version: manifest.spec.interface_version,
            player_counts: vec![2],
            uri: "http://interceptor:8080".to_string(),
            rules: "rules text".to_string(),
        };
        upsert(&pool, &operator).await.unwrap();

        // Same version name + type: the shared upsert updates in place rather
        // than duplicating rows.
        let types: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_types WHERE name = 'Tic-tac-toe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(types, 1);
        let versions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(versions, 1);

        let (weight, blurb, player_counts): (f32, String, Vec<i32>) = sqlx::query_as(
            "SELECT weight, blurb, player_counts FROM game_types WHERE name = 'Tic-tac-toe'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(weight, manifest.spec.weight);
        assert_eq!(blurb, manifest.spec.blurb);
        assert_eq!(player_counts, vec![2]);

        let (uri, is_public, is_deprecated, interface_version, rules): (
            String,
            bool,
            bool,
            i32,
            String,
        ) = sqlx::query_as(
            "SELECT uri, is_public, is_deprecated, interface_version, rules \
                 FROM game_versions WHERE name = 'tic-tac-toe-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(uri, "http://interceptor:8080");
        assert!(is_public);
        assert_eq!(is_deprecated, manifest.spec.is_deprecated);
        assert_eq!(interface_version, manifest.spec.interface_version);
        assert_eq!(rules, "rules text");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn upsert_writes_weight_and_blurb(pool: PgPool) {
        upsert(&pool, &tic_tac_toe_registration("http://localhost:0/mock"))
            .await
            .unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Tic-tac-toe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 1.0f32);
        assert_eq!(
            blurb,
            tic_tac_toe_registration("http://localhost:0/mock").blurb
        );

        // Upsert path: a second reconcile updates the existing row in place.
        let mut updated = tic_tac_toe_registration("http://localhost:0/mock");
        updated.weight = 3.0;
        updated.blurb = "New blurb.".to_string();
        upsert(&pool, &updated).await.unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Tic-tac-toe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 3.0);
        assert_eq!(blurb, "New blurb.");
        let versions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(versions, 1);
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn authoritative_version_wins_regardless_of_order_deprecated_first(pool: PgPool) {
        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-1".to_string(),
                weight: 1.0,
                blurb: "old blurb".to_string(),
                is_deprecated: true,
                interface_version: 1,
                player_counts: vec![2],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-2".to_string(),
                weight: 2.0,
                blurb: "new blurb".to_string(),
                is_deprecated: false,
                interface_version: 1,
                player_counts: vec![2, 3],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        let player_counts: Vec<i32> =
            sqlx::query_scalar("SELECT player_counts FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(player_counts, vec![2, 3]);
        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 2.0f32);
        assert_eq!(blurb, "new blurb");

        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-1".to_string(),
                weight: 1.0,
                blurb: "old blurb".to_string(),
                is_deprecated: true,
                interface_version: 1,
                player_counts: vec![2],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        let player_counts: Vec<i32> =
            sqlx::query_scalar("SELECT player_counts FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(player_counts, vec![2, 3]);
        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 2.0f32);
        assert_eq!(blurb, "new blurb");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn authoritative_version_wins_regardless_of_order_non_deprecated_first(pool: PgPool) {
        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-2".to_string(),
                weight: 2.0,
                blurb: "new blurb".to_string(),
                is_deprecated: false,
                interface_version: 1,
                player_counts: vec![2, 3],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-1".to_string(),
                weight: 1.0,
                blurb: "old blurb".to_string(),
                is_deprecated: true,
                interface_version: 1,
                player_counts: vec![2],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        let player_counts: Vec<i32> =
            sqlx::query_scalar("SELECT player_counts FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(player_counts, vec![2, 3]);
        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 2.0f32);
        assert_eq!(blurb, "new blurb");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn first_write_deprecated_only_still_writes_values(pool: PgPool) {
        upsert(
            &pool,
            &Registration {
                type_name: "Solo Game".to_string(),
                version_name: "solo-game-1".to_string(),
                weight: 0.5,
                blurb: "solo blurb".to_string(),
                is_deprecated: true,
                interface_version: 1,
                player_counts: vec![1],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        let player_counts: Vec<i32> =
            sqlx::query_scalar("SELECT player_counts FROM game_types WHERE name = 'Solo Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(player_counts, vec![1]);
        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Solo Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 0.5f32);
        assert_eq!(blurb, "solo blurb");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn set_public_flips_is_public(pool: PgPool) {
        upsert(&pool, &tic_tac_toe_registration("http://localhost:0/mock"))
            .await
            .unwrap();

        set_public(&pool, "tic-tac-toe-2", "Tic-tac-toe", false)
            .await
            .unwrap();
        let is_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!is_public);

        set_public(&pool, "tic-tac-toe-2", "Tic-tac-toe", true)
            .await
            .unwrap();
        let is_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'tic-tac-toe-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(is_public);
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn mark_others_non_public_demotes_without_deleting(pool: PgPool) {
        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-2".to_string(),
                weight: 1.5,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8080".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();
        upsert(
            &pool,
            &Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-1".to_string(),
                weight: 1.5,
                blurb: "blurb".to_string(),
                is_deprecated: true,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8080".to_string(),
                rules: "rules text".to_string(),
            },
        )
        .await
        .unwrap();

        let demoted = mark_others_non_public(&pool, "lost-cities-2")
            .await
            .unwrap();
        assert_eq!(demoted, 1);

        let lc2_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'lost-cities-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let lc1_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'lost-cities-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(lc2_public);
        assert!(!lc1_public);

        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 2);
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn bulk_set_persists_persisted_uri_and_demotes_unselected(pool: PgPool) {
        // Baseline: a third game is public before the set runs.
        upsert(
            &pool,
            &Registration {
                type_name: "Zombie Dice".to_string(),
                version_name: "zombie-dice-2".to_string(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8080".to_string(),
                rules: "rules".to_string(),
            },
        )
        .await
        .unwrap();

        let selected = [
            Registration {
                type_name: "Tic-tac-toe".to_string(),
                version_name: "tic-tac-toe-2".to_string(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8081".to_string(),
                rules: "rules one".to_string(),
            },
            Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-2".to_string(),
                weight: 1.5,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2, 3],
                uri: "http://127.0.0.1:8082".to_string(),
                rules: "rules two".to_string(),
            },
        ];

        let stats = bulk_set(&pool, &selected).await.unwrap();
        assert_eq!(stats.registered, 2);
        assert_eq!(stats.demoted, 1);

        let (uri, rules, is_public): (String, String, bool) = sqlx::query_as(
            "SELECT uri, rules, is_public FROM game_versions WHERE name = 'tic-tac-toe-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(uri, "http://127.0.0.1:8081");
        assert_eq!(rules, "rules one");
        assert!(is_public);

        let (uri, is_public): (String, bool) =
            sqlx::query_as("SELECT uri, is_public FROM game_versions WHERE name = 'lost-cities-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(uri, "http://127.0.0.1:8082");
        assert!(is_public);

        let is_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'zombie-dice-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!is_public, "non-selected version must be demoted");

        let versions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(versions, 3, "demotion must not delete rows");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn bulk_set_demotes_exact_identity_not_whole_type(pool: PgPool) {
        // Same game type with two versions: only the exact selected version
        // stays public, the sibling version is demoted.
        for (version, deprecated) in [("lost-cities-2", false), ("lost-cities-1", true)] {
            upsert(
                &pool,
                &Registration {
                    type_name: "Lost Cities".to_string(),
                    version_name: version.to_string(),
                    weight: 1.5,
                    blurb: "blurb".to_string(),
                    is_deprecated: deprecated,
                    interface_version: 1,
                    player_counts: vec![2],
                    uri: "http://127.0.0.1:8080".to_string(),
                    rules: "rules".to_string(),
                },
            )
            .await
            .unwrap();
        }
        upsert(
            &pool,
            &Registration {
                type_name: "Tic-tac-toe".to_string(),
                version_name: "tic-tac-toe-2".to_string(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8080".to_string(),
                rules: "rules".to_string(),
            },
        )
        .await
        .unwrap();

        let selected = [Registration {
            type_name: "Lost Cities".to_string(),
            version_name: "lost-cities-2".to_string(),
            weight: 1.5,
            blurb: "blurb".to_string(),
            is_deprecated: false,
            interface_version: 2,
            player_counts: vec![2, 3],
            uri: "http://127.0.0.1:8081".to_string(),
            rules: "rules".to_string(),
        }];
        bulk_set(&pool, &selected).await.unwrap();

        let (lc2_public, lc1_public, ttt_public): (bool, bool, bool) = sqlx::query_as(
            "SELECT \
                (SELECT is_public FROM game_versions WHERE name = 'lost-cities-2'), \
                (SELECT is_public FROM game_versions WHERE name = 'lost-cities-1'), \
                (SELECT is_public FROM game_versions WHERE name = 'tic-tac-toe-2')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(lc2_public);
        assert!(!lc1_public, "same-type sibling version must be demoted");
        assert!(!ttt_public, "other-type version must be demoted");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn bulk_set_is_idempotent(pool: PgPool) {
        let selected = [tic_tac_toe_registration("http://127.0.0.1:8081")];

        bulk_set(&pool, &selected).await.unwrap();
        let first: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_versions")
            .fetch_one(&pool)
            .await
            .unwrap();
        let stats = bulk_set(&pool, &selected).await.unwrap();
        assert_eq!(stats.registered, 1);
        assert_eq!(stats.demoted, 0);

        let (count, is_public, uri): (i64, bool, String) = sqlx::query_as(
            "SELECT COUNT(*) OVER (), is_public, uri FROM game_versions \
             WHERE name = 'tic-tac-toe-2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, first, "second run must not duplicate rows");
        assert!(is_public);
        assert_eq!(uri, "http://127.0.0.1:8081");
    }

    #[sqlx::test(migrations = "../../web/migrations")]
    async fn bulk_set_rolls_back_entire_set_on_db_error(pool: PgPool) {
        upsert(
            &pool,
            &Registration {
                type_name: "Zombie Dice".to_string(),
                version_name: "zombie-dice-2".to_string(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8080".to_string(),
                rules: "rules".to_string(),
            },
        )
        .await
        .unwrap();

        sqlx::query(
            "CREATE OR REPLACE FUNCTION fail_game_version_insert() RETURNS trigger AS \
             $$ BEGIN RAISE EXCEPTION 'forced failure'; END $$ LANGUAGE plpgsql",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TRIGGER fail_game_version_insert \
             BEFORE INSERT ON game_versions FOR EACH ROW EXECUTE FUNCTION fail_game_version_insert()",
        )
        .execute(&pool)
        .await
        .unwrap();

        let selected = [
            tic_tac_toe_registration("http://127.0.0.1:8081"),
            Registration {
                type_name: "Lost Cities".to_string(),
                version_name: "lost-cities-2".to_string(),
                weight: 1.5,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://127.0.0.1:8082".to_string(),
                rules: "rules".to_string(),
            },
        ];
        assert!(bulk_set(&pool, &selected).await.is_err());

        // Nothing from the failed set may persist, including the type rows
        // upserted before the first failing version insert.
        let ttt_types: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_types WHERE name = 'Tic-tac-toe'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ttt_types, 0);
        let lc_types: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_types WHERE name = 'Lost Cities'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lc_types, 0);
        let zombie_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = 'zombie-dice-2'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            zombie_public,
            "baseline row must be untouched by the rollback"
        );
    }

    // The 27 delivered Rust games each have a canonical manifest and together
    // form a valid set: no duplicate (game type, version) identity. The
    // bulk/set path keys on exactly that identity, so the delivered set must
    // satisfy it.
    #[test]
    fn all_delivered_game_manifests_have_unique_identity() {
        let manifests = [
            include_str!("../../../../k8s/base/game/acquire-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/age-of-war-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/alhambra-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/battleship-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/category-5-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/cathedral-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/farkle-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/for-sale-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/greed-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/hanamikoji-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/jaipur-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/liars-dice-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/lost-cities-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/lost-cities-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/love-letter-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/modern-art-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/no-thanks-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/red7-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/roll-through-the-ages-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/seven-wonders-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/splendor-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/starship-catan-1/game-version.yaml"),
            include_str!("../../../../k8s/base/game/sushi-go-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/sushizock-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/texas-holdem-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/tic-tac-toe-2/game-version.yaml"),
            include_str!("../../../../k8s/base/game/zombie-dice-2/game-version.yaml"),
        ];
        assert_eq!(manifests.len(), 27);

        let mut identities = std::collections::HashSet::new();
        for content in manifests {
            let manifest: GameVersionManifest = content.parse().unwrap();
            let identity = (manifest.spec.type_name, manifest.metadata.name);
            assert!(
                identities.insert(identity.clone()),
                "duplicate game type+version identity: {identity:?}"
            );
        }
        assert_eq!(identities.len(), 27);
    }
}
