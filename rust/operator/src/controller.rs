use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brdgme_cmd::api::{Request, Response};
use futures::StreamExt;
use kube::{
    Api, Client, ResourceExt,
    api::{Patch, PatchParams},
    runtime::{Controller, controller::Action, watcher},
};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::crd::GameVersion;

const FINALIZER: &str = "brdgme.com/game-version";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),
    #[error("Database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Game service error: {0}")]
    GameService(String),
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

fn interceptor_uri() -> String {
    std::env::var("INTERCEPTOR_URI").unwrap_or_else(|_| {
        "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080".to_string()
    })
}

async fn reconcile(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = obj.name_any();
    let ns = obj.namespace().unwrap_or_else(|| "brdgme".to_string());
    let api: Api<GameVersion> = Api::namespaced(ctx.client.clone(), &ns);

    if obj.metadata.deletion_timestamp.is_some() {
        info!(name, "Marking game version unavailable");
        sqlx::query(
            "UPDATE game_versions SET is_public = false, updated_at = NOW() \
             WHERE name = $1 AND game_type_id = (SELECT id FROM game_types WHERE name = $2)",
        )
        .bind(&name)
        .bind(&obj.spec.type_name)
        .execute(&ctx.pool)
        .await?;

        let finalizers: Vec<String> = obj
            .finalizers()
            .iter()
            .filter(|f| f.as_str() != FINALIZER)
            .cloned()
            .collect();
        api.patch(
            &name,
            &PatchParams::default(),
            &Patch::Merge(json!({ "metadata": { "finalizers": finalizers } })),
        )
        .await?;

        return Ok(Action::await_change());
    }

    if !obj.finalizers().contains(&FINALIZER.to_string()) {
        let mut finalizers = obj.finalizers().to_vec();
        finalizers.push(FINALIZER.to_string());
        api.patch(
            &name,
            &PatchParams::default(),
            &Patch::Merge(json!({ "metadata": { "finalizers": finalizers } })),
        )
        .await?;
    }

    let generation = obj.metadata.generation;
    let observed_generation = obj.status.as_ref().and_then(|s| s.observed_generation);
    if generation.is_some() && generation == observed_generation {
        info!(name, "Spec unchanged since last reconcile, skipping");
        return Ok(requeue_with_jitter());
    }

    let uri = interceptor_uri();
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

    api.patch_status(
        &name,
        &PatchParams::default(),
        &Patch::Merge(json!({ "status": { "ready": true, "observedGeneration": generation } })),
    )
    .await?;

    Ok(requeue_with_jitter())
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
            SET player_counts = EXCLUDED.player_counts,
                weight        = EXCLUDED.weight,
                blurb         = EXCLUDED.blurb,
                updated_at    = NOW()
        RETURNING id
        "#,
    )
    .bind(type_name)
    .bind(player_counts)
    .bind(weight as f64)
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
    fn interceptor_uri_defaults_to_keda_proxy() {
        // INTERCEPTOR_URI is not set in the test environment.
        assert_eq!(
            interceptor_uri(),
            "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080"
        );
    }

    // Applies the web crate's migrations so the schema matches production.
    // The operator itself never runs migrations (docs/DEV.md).
    #[sqlx::test(migrations = "../web/migrations")]
    async fn upsert_writes_weight_and_blurb(pool: PgPool) {
        upsert_game_type_and_version(
            &pool,
            "Test Game",
            &[2, 3],
            2.5,
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
        assert_eq!(weight, 2.5);
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
}
