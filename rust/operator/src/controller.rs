use std::sync::Arc;
use std::time::Duration;

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
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Game service error: {0}")]
    GameService(String),
}

pub struct Ctx {
    pub client: Client,
    pub pool: PgPool,
    pub http: reqwest::Client,
}

async fn game_service_request(
    client: &reqwest::Client,
    uri: &str,
    request: &Request,
) -> Result<Response, Error> {
    let resp = client.post(uri).json(request).send().await?;
    let response: Response = resp.json().await?;
    match response {
        Response::SystemError { message } => Err(Error::GameService(message)),
        other => Ok(other),
    }
}

async fn reconcile(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = obj.name_any();
    let ns = obj.namespace().unwrap_or_else(|| "brdgme".to_string());
    let api: Api<GameVersion> = Api::namespaced(ctx.client.clone(), &ns);

    if obj.metadata.deletion_timestamp.is_some() {
        info!(name, "Marking game version unavailable");
        sqlx::query(
            "UPDATE game_versions SET is_public = false, updated_at = NOW() WHERE name = $1",
        )
        .bind(&name)
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

    let uri = std::env::var("GAME_SERVICE_URI_TEMPLATE")
        .map(|t| t.replace("{name}", &name).replace("{ns}", &ns))
        .unwrap_or_else(|_| format!("http://{}.{}.svc.cluster.local", name, ns));
    info!(name, uri, "Upserting game version");

    let player_counts = match game_service_request(&ctx.http, &uri, &Request::PlayerCounts).await? {
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

    let rules = match game_service_request(&ctx.http, &uri, &Request::Rules).await? {
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
        &name,
        &uri,
        obj.spec.is_deprecated,
        &rules,
    )
    .await?;

    Ok(Action::requeue(Duration::from_secs(3600)))
}

// Splitting these into a params struct would be a larger refactor than warranted here.
#[allow(clippy::too_many_arguments)]
async fn upsert_game_type_and_version(
    pool: &PgPool,
    type_name: &str,
    player_counts: &[i32],
    weight: f32,
    version_name: &str,
    uri: &str,
    is_deprecated: bool,
    rules: &str,
) -> Result<(), sqlx::Error> {
    let game_type_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO game_types (name, player_counts, weight)
        VALUES ($1, $2, $3)
        ON CONFLICT (name) DO UPDATE
            SET player_counts = EXCLUDED.player_counts,
                weight        = EXCLUDED.weight,
                updated_at    = NOW()
        RETURNING id
        "#,
    )
    .bind(type_name)
    .bind(player_counts)
    .bind(weight as f64)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated, rules)
        VALUES ($1, $2, $3, true, $4, $5)
        ON CONFLICT (game_type_id, name) DO UPDATE
            SET uri           = EXCLUDED.uri,
                is_public     = true,
                is_deprecated = EXCLUDED.is_deprecated,
                rules         = EXCLUDED.rules,
                updated_at    = NOW()
        "#,
    )
    .bind(game_type_id)
    .bind(version_name)
    .bind(uri)
    .bind(is_deprecated)
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
