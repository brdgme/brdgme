use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brdgme_cmd::api::{Request, Response};
use brdgme_registration::{Registration, set_public, upsert};
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

use crate::crd::{GameVersion, GameVersionSpec, GameVersionStatus};

const FINALIZER: &str = "brdgme.com/game-version";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes error: {0}")]
    Kube(#[from] kube::Error),
    #[error("Database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Registration error: {0}")]
    Registration(#[from] brdgme_registration::RegistrationError),
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

    upsert(
        &ctx.pool,
        &registration_from_spec(&obj.spec, &name, &uri, player_counts, rules),
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
    set_public(&ctx.pool, &name, &obj.spec.type_name, false).await?;
    Ok(Action::await_change())
}

fn registration_from_spec(
    spec: &GameVersionSpec,
    version_name: &str,
    uri: &str,
    player_counts: Vec<i32>,
    rules: String,
) -> Registration {
    Registration {
        type_name: spec.type_name.clone(),
        version_name: version_name.to_string(),
        weight: spec.weight,
        blurb: spec.blurb.clone(),
        is_deprecated: spec.is_deprecated,
        interface_version: spec.interface_version,
        player_counts,
        uri: uri.to_string(),
        rules,
    }
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

    #[test]
    fn registration_from_spec_maps_crd_fields() {
        use crate::crd::GameVersionSpec;

        let spec = GameVersionSpec {
            type_name: "Tic-tac-toe".to_string(),
            weight: 1.0,
            blurb: "A blurb.".to_string(),
            is_deprecated: false,
            interface_version: 2,
        };
        let registration = registration_from_spec(
            &spec,
            "tic-tac-toe-2",
            "http://interceptor:8080",
            vec![2],
            "rules text".to_string(),
        );
        assert_eq!(registration.type_name, "Tic-tac-toe");
        assert_eq!(registration.version_name, "tic-tac-toe-2");
        assert_eq!(registration.weight, 1.0);
        assert_eq!(registration.blurb, "A blurb.");
        assert!(!registration.is_deprecated);
        assert_eq!(registration.interface_version, 2);
        assert_eq!(registration.player_counts, vec![2]);
        assert_eq!(registration.uri, "http://interceptor:8080");
        assert_eq!(registration.rules, "rules text");
    }
}
