use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brdgme_cmd::api::{Request, Response};
use futures::StreamExt;
use kube::{
    Api, Client, ResourceExt,
    api::{Patch, PatchParams},
    runtime::{
        Controller,
        controller::Action,
        finalizer::{Event, finalizer},
        watcher,
    },
};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::crd::{GameVersion, GameVersionStatus};

const FINALIZER: &str = "brdgme.com/game-version";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),
    #[error("Database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Game service error: {0}")]
    GameService(String),
    #[error("Finalizer error: {0}")]
    Finalizer(Box<kube::runtime::finalizer::Error<Error>>),
}

pub struct Ctx {
    pub client: Client,
    pub pool: PgPool,
    pub http: reqwest::Client,
}

// Requeue interval plus jitter to avoid a thundering herd of reconciles all
// firing at once. No `rand` dependency in this crate, so derive the jitter
// from the current time instead of pulling one in for this alone.
fn requeue_with_jitter() -> Action {
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 % 901)
        .unwrap_or(0);
    Action::requeue(Duration::from_secs(3600 + jitter))
}

async fn game_service_request(
    client: &reqwest::Client,
    uri: &str,
    name: &str,
    request: &Request,
) -> Result<Response, Error> {
    brdgme_game_client::request(client, uri, name, request)
        .await
        .map_err(|e| Error::GameService(format!("{e:#}")))
}

fn interceptor_uri(env: Option<String>) -> String {
    env.unwrap_or_else(|| {
        "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080".to_string()
    })
}

async fn reconcile(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = obj.name_any();
    let ns = obj.namespace().unwrap_or_else(|| "brdgme".to_string());
    let api: Api<GameVersion> = Api::namespaced(ctx.client.clone(), &ns);
    let generation = obj.metadata.generation;

    match finalizer(&api, FINALIZER, obj, |event| async {
        match event {
            Event::Apply(obj) => apply(obj, ctx).await,
            Event::Cleanup(obj) => cleanup(obj, ctx).await,
        }
    })
    .await
    {
        Ok(action) => Ok(action),
        Err(err) => {
            let status = GameVersionStatus {
                ready: false,
                message: Some(err.to_string()),
                observed_generation: generation,
            };
            if let Err(e) = api
                .patch_status(
                    &name,
                    &PatchParams::default(),
                    &Patch::Merge(json!({ "status": status })),
                )
                .await
            {
                error!(name, error = %e, "Failed to patch failure status");
            }
            Err(Error::Finalizer(Box::new(err)))
        }
    }
}

async fn apply(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = obj.name_any();
    let generation = obj.metadata.generation;
    let observed_generation = obj.status.as_ref().and_then(|s| s.observed_generation);
    if generation.is_some() && generation == observed_generation {
        info!(name, "Spec unchanged since last reconcile, skipping");
        return Ok(requeue_with_jitter());
    }

    let uri = interceptor_uri(std::env::var("INTERCEPTOR_URI").ok());
    info!(name, uri, "Upserting game version");

    let player_counts =
        match game_service_request(&ctx.http, &uri, &name, &Request::PlayerCounts).await? {
            Response::PlayerCounts { player_counts } => player_counts
                .into_iter()
                .map(|c| c as i32)
                .collect::<Vec<_>>(),
            other => {
                return Err(Error::GameService(format!(
                    "unexpected response to PlayerCounts: {:?}",
                    other
                )));
            }
        };

    let rules = match game_service_request(&ctx.http, &uri, &name, &Request::Rules).await? {
        Response::Rules { rules } => rules,
        other => {
            return Err(Error::GameService(format!(
                "unexpected response to Rules: {:?}",
                other
            )));
        }
    };

    upsert_game_type_and_version(
        &ctx.pool,
        &obj.spec.type_name,
        &player_counts,
        obj.spec.weight,
        &obj.spec.blurb,
        &name,
        &uri,
        obj.spec.is_deprecated,
        obj.spec.interface_version,
        &rules,
    )
    .await?;

    let ns = obj.namespace().unwrap_or_else(|| "brdgme".to_string());
    let api: Api<GameVersion> = Api::namespaced(ctx.client.clone(), &ns);
    let status = GameVersionStatus {
        ready: true,
        message: None,
        observed_generation: generation,
    };
    api.patch_status(
        &name,
        &PatchParams::default(),
        &Patch::Merge(json!({ "status": status })),
    )
    .await?;

    Ok(requeue_with_jitter())
}

async fn cleanup(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = obj.name_any();
    info!(name, "Marking game version unavailable");
    sqlx::query(
        "UPDATE game_versions SET is_public = false, updated_at = NOW() \
         WHERE name = $1 AND game_type_id = (SELECT id FROM game_types WHERE name = $2)",
    )
    .bind(&name)
    .bind(&obj.spec.type_name)
    .execute(&ctx.pool)
    .await?;
    Ok(Action::await_change())
}

// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
async fn upsert_game_type_and_version(
    pool: &PgPool,
    type_name: &str,
    player_counts: &[i32],
    weight: f32,
    blurb: &str,
    version_name: &str,
    uri: &str,
    is_deprecated: bool,
    interface_version: i32,
    rules: &str,
) -> Result<(), sqlx::Error> {
    let game_type_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO game_types (name, player_counts, weight, blurb)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (name) DO UPDATE
            SET updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(type_name)
    .bind(player_counts)
    .bind(weight)
    .bind(blurb)
    .fetch_one(pool)
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
    .bind(version_name)
    .bind(uri)
    .bind(is_deprecated)
    .bind(interface_version)
    .bind(rules)
    .execute(pool)
    .await?;

    if !is_deprecated {
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
        .bind(player_counts)
        .bind(weight)
        .bind(blurb)
        .bind(version_name)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn error_policy(obj: Arc<GameVersion>, err: &Error, _ctx: Arc<Ctx>) -> Action {
    error!(name = obj.name_any(), error = %err, "Reconcile error");
    Action::requeue(Duration::from_secs(30))
}

pub async fn run(client: Client, pool: PgPool) {
    let api: Api<GameVersion> = Api::all(client.clone());
    let http = reqwest::Client::new();
    let ctx = Arc::new(Ctx { client, pool, http });
    Controller::new(api, watcher::Config::default())
        .shutdown_on_signal()
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            if let Err(e) = res {
                error!("Controller error: {:?}", e);
            }
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interceptor_uri_fallback_and_override() {
        assert_eq!(
            interceptor_uri(None),
            "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080"
        );
        assert_eq!(interceptor_uri(Some("x".to_string())), "x");
    }

    #[test]
    fn status_and_spec_serde_shape() {
        use crate::crd::{GameVersionSpec, GameVersionStatus};

        let status = GameVersionStatus {
            ready: true,
            message: None,
            observed_generation: Some(2),
        };
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({"ready": true, "observedGeneration": 2})
        );

        let status_msg = GameVersionStatus {
            ready: true,
            message: Some("boom".to_string()),
            observed_generation: Some(2),
        };
        let val = serde_json::to_value(&status_msg).unwrap();
        assert_eq!(val["message"], json!("boom"));
        assert_eq!(val["ready"], json!(true));
        assert_eq!(val["observedGeneration"], json!(2));

        let spec: GameVersionSpec =
            serde_json::from_value(json!({"typeName": "X", "interfaceVersion": 2})).unwrap();
        assert_eq!(spec.interface_version, 2);

        let spec_default: GameVersionSpec =
            serde_json::from_value(json!({"typeName": "X"})).unwrap();
        assert_eq!(spec_default.interface_version, 1);
    }

    // Applies the web crate's migrations so the schema matches production.
    // The operator itself never runs migrations (docs/DEV.md).
    #[sqlx::test(migrations = "../web/migrations")]
    async fn upsert_writes_weight_and_blurb(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            2.7,
            "A test blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            1,
            "rules text",
        )
        .await
        .unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Test Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 2.7f32);
        assert_eq!(blurb, "A test blurb.");

        // Upsert path: a second reconcile updates the existing row in place.
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            3.0,
            "New blurb.",
            "test-game-1",
            "http://localhost:0/mock",
            false,
            1,
            "rules text",
        )
        .await
        .unwrap();

        let (weight, blurb): (f32, String) =
            sqlx::query_as("SELECT weight, blurb FROM game_types WHERE name = 'Test Game'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(weight, 3.0);
        assert_eq!(blurb, "New blurb.");
        let versions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM game_versions WHERE name = 'test-game-1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(versions, 1);
    }

    #[sqlx::test(migrations = "../web/migrations")]
    async fn authoritative_version_wins_regardless_of_order_deprecated_first(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Lost Cities",
            &[2],
            1.0,
            "old blurb",
            "lost-cities-1",
            "http://localhost:0/mock",
            true,
            1,
            "rules text",
        )
        .await
        .unwrap();

        upsert_game_type_and_version(
            &pool,
            "Lost Cities",
            &[2, 3],
            2.0,
            "new blurb",
            "lost-cities-2",
            "http://localhost:0/mock",
            false,
            1,
            "rules text",
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

        upsert_game_type_and_version(
            &pool,
            "Lost Cities",
            &[2],
            1.0,
            "old blurb",
            "lost-cities-1",
            "http://localhost:0/mock",
            true,
            1,
            "rules text",
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

    #[sqlx::test(migrations = "../web/migrations")]
    async fn authoritative_version_wins_regardless_of_order_non_deprecated_first(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Lost Cities",
            &[2, 3],
            2.0,
            "new blurb",
            "lost-cities-2",
            "http://localhost:0/mock",
            false,
            1,
            "rules text",
        )
        .await
        .unwrap();

        upsert_game_type_and_version(
            &pool,
            "Lost Cities",
            &[2],
            1.0,
            "old blurb",
            "lost-cities-1",
            "http://localhost:0/mock",
            true,
            1,
            "rules text",
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

    #[sqlx::test(migrations = "../web/migrations")]
    async fn first_write_deprecated_only_still_writes_values(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Solo Game",
            &[1],
            0.5,
            "solo blurb",
            "solo-game-1",
            "http://localhost:0/mock",
            true,
            1,
            "rules text",
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
}
