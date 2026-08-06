use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brdgme_cmd::api::{Request, Response};
use brdgme_registration::{Registration, set_public, upsert};
use futures::{FutureExt, StreamExt, future::BoxFuture};
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

// Boxed rather than a plain `async fn`: with a plain `async fn` here, proving
// the `Send` bound `Controller::run` requires on the returned future hits a
// known rustc trait-solver limitation around opaque async-fn return types
// that borrow from their own locals (rust-lang/rust#134997), reported as
// "implementation of `Send`/`Acquire` is not general enough". Returning a
// concrete `BoxFuture` sidesteps the buggy generic leak-check.
fn reconcile(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> BoxFuture<'static, Result<Action, Error>> {
    async move {
        let name = obj.name_any();
        let ns = obj.namespace().unwrap_or_else(|| "brdgme".to_string());
        let api: Api<GameVersion> = Api::namespaced(ctx.client.clone(), &ns);
        let generation = obj.metadata.generation;

        match finalizer(&api, FINALIZER, obj, |event| {
            async move {
                match event {
                    Event::Apply(obj) => apply(obj, ctx).await,
                    Event::Cleanup(obj) => cleanup(obj, ctx).await,
                }
            }
            .boxed()
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
    .boxed()
}

fn apply(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> BoxFuture<'static, Result<Action, Error>> {
    async move {
        let name = obj.name_any();
        let generation = obj.metadata.generation;
        let observed_generation = obj.status.as_ref().and_then(|s| s.observed_generation);
        // Rows persisted before the snapshot columns existed (and first
        // reconciles) have incomplete snapshots, so keep reconciling until all
        // three are persisted even when the generation is otherwise unchanged.
        // Only then is `observedGeneration` written and the normal guard resumes.
        let snapshots_complete = snapshots_complete(&ctx.pool, &name, &obj.spec.type_name).await?;
        if should_skip_reconcile(generation, observed_generation, snapshots_complete) {
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
    .boxed()
}

/// Whether the stored `game_versions` row for a version has all three snapshot
/// columns populated. A missing row counts as incomplete so a never-registered
/// version is reconciled even when the CR generation is unchanged, backfilling
/// its snapshots before `observedGeneration` is written (R-51/F-196).
async fn snapshots_complete(
    pool: &PgPool,
    version_name: &str,
    type_name: &str,
) -> Result<bool, Error> {
    let row = sqlx::query_as::<_, (Option<Vec<i32>>, Option<f32>, Option<String>)>(
        r#"
        SELECT snapshot_player_counts, snapshot_weight, snapshot_blurb
        FROM game_versions
        WHERE name = $1
          AND game_type_id = (SELECT id FROM game_types WHERE name = $2)
        "#,
    )
    .bind(version_name)
    .bind(type_name)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|(player_counts, weight, blurb)| {
            snapshots_complete_from_row(player_counts, weight, blurb)
        })
        .unwrap_or(false))
}

fn snapshots_complete_from_row(
    player_counts: Option<Vec<i32>>,
    weight: Option<f32>,
    blurb: Option<String>,
) -> bool {
    player_counts.is_some() && weight.is_some() && blurb.is_some()
}

/// Whether a reconcile can skip the service round-trip entirely: the CR
/// generation matches the last observed one and the stored row already has
/// complete snapshots. Missing or incomplete snapshots force a reconcile even
/// on an unchanged generation so the row is backfilled first (R-51/F-196).
fn should_skip_reconcile(
    generation: Option<i64>,
    observed_generation: Option<i64>,
    snapshots_complete: bool,
) -> bool {
    generation.is_some() && generation == observed_generation && snapshots_complete
}

fn cleanup(obj: Arc<GameVersion>, ctx: Arc<Ctx>) -> BoxFuture<'static, Result<Action, Error>> {
    async move {
        let name = obj.name_any();
        info!(name, "Marking game version unavailable");
        set_public(&ctx.pool, &name, &obj.spec.type_name, false).await?;
        Ok(Action::await_change())
    }
    .boxed()
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
    use axum::{
        Json, Router,
        extract::Path,
        routing::{patch, post},
    };
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use std::sync::Mutex;
    use tokio::net::TcpListener;

    // The apply tests each write INTERCEPTOR_URI, which the real apply reads
    // from the process environment. Serialize them so the write cannot race
    // with the sibling test's read.
    static APPLY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    // Sets INTERCEPTOR_URI for the duration of a test and restores the exact
    // previous value (including absent) on drop, which also runs during panic
    // unwinds so a failing test cannot leak its override into siblings.
    // Serialized by APPLY_LOCK.
    struct InterceptorUriGuard(Option<String>);

    impl InterceptorUriGuard {
        fn set(uri: &str) -> Self {
            let previous = std::env::var("INTERCEPTOR_URI").ok();
            unsafe { std::env::set_var("INTERCEPTOR_URI", uri) };
            Self(previous)
        }
    }

    impl Drop for InterceptorUriGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(uri) => unsafe { std::env::set_var("INTERCEPTOR_URI", uri) },
                None => unsafe { std::env::remove_var("INTERCEPTOR_URI") },
            }
        }
    }

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
    fn should_skip_reconcile_requires_matching_generation_and_complete_snapshots() {
        assert!(should_skip_reconcile(Some(2), Some(2), true));
        assert!(!should_skip_reconcile(Some(2), Some(2), false));
        assert!(!should_skip_reconcile(Some(2), Some(3), true));
        assert!(!should_skip_reconcile(None, None, true));
        assert!(!should_skip_reconcile(Some(2), None, true));
    }

    #[test]
    fn snapshots_complete_from_row_requires_all_three_columns() {
        assert!(snapshots_complete_from_row(
            Some(vec![2, 3]),
            Some(1.5),
            Some("blurb".to_string()),
        ));
        assert!(!snapshots_complete_from_row(
            None,
            Some(1.5),
            Some("blurb".to_string()),
        ));
        assert!(!snapshots_complete_from_row(
            Some(vec![2, 3]),
            None,
            Some("blurb".to_string()),
        ));
        assert!(!snapshots_complete_from_row(
            Some(vec![2, 3]),
            Some(1.5),
            None,
        ));
        assert!(!snapshots_complete_from_row(None, None, None));
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

    async fn set_created_at(pool: &PgPool, type_name: &str, version_name: &str, created_at: &str) {
        sqlx::query(
            "UPDATE game_versions SET created_at = $1::timestamp \
             WHERE name = $2 AND game_type_id = (SELECT id FROM game_types WHERE name = $3)",
        )
        .bind(created_at)
        .bind(version_name)
        .bind(type_name)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn descriptor_values(pool: &PgPool, type_name: &str) -> (Vec<i32>, f32, String) {
        sqlx::query_as("SELECT player_counts, weight, blurb FROM game_types WHERE name = $1")
            .bind(type_name)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    // Tests that never talk to the kube API pass a dead address; the apply
    // tests pass an in-process mock server. The workspace enables both rustls
    // backends, so the process default must be installed before the client
    // builds its TLS stack (same setup main() performs).
    fn kube_client(server: &str) -> kube::Client {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        use kube::config::{Cluster, Context, Kubeconfig, NamedCluster, NamedContext};
        kube::Client::try_from(Kubeconfig {
            current_context: Some("dummy".to_string()),
            clusters: vec![NamedCluster {
                name: "dummy".to_string(),
                cluster: Some(Cluster {
                    server: Some(server.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            contexts: vec![NamedContext {
                name: "dummy".to_string(),
                context: Some(Context {
                    cluster: "dummy".to_string(),
                    ..Default::default()
                }),
                other: Default::default(),
            }],
            ..Default::default()
        })
        .unwrap()
    }

    #[sqlx::test(migrations = "../web/migrations")]
    async fn snapshots_complete_detects_complete_incomplete_and_missing_rows(pool: PgPool) {
        let type_name = "Snapshot Check".to_string();
        let version_name = "snapshot-check-1".to_string();
        upsert(
            &pool,
            &Registration {
                type_name: type_name.clone(),
                version_name: version_name.clone(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(
            snapshots_complete(&pool, &version_name, &type_name)
                .await
                .unwrap()
        );

        sqlx::query("UPDATE game_versions SET snapshot_weight = NULL WHERE name = $1")
            .bind(&version_name)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            !snapshots_complete(&pool, &version_name, &type_name)
                .await
                .unwrap()
        );

        assert!(
            !snapshots_complete(&pool, "never-registered-version", &type_name)
                .await
                .unwrap()
        );
    }

    #[sqlx::test(migrations = "../web/migrations")]
    async fn cleanup_marks_version_non_public_and_repoints_descriptors(pool: PgPool) {
        let type_name = "Cleanup Game".to_string();
        let fallback = "cleanup-game-1".to_string();
        let newest = "cleanup-game-2".to_string();
        let registration =
            |version: &str, player_counts: Vec<i32>, weight: f32, blurb: &str| Registration {
                type_name: type_name.clone(),
                version_name: version.to_string(),
                weight,
                blurb: blurb.to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts,
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules".to_string(),
            };
        upsert(
            &pool,
            &registration(&fallback, vec![2], 1.0, "fallback blurb"),
        )
        .await
        .unwrap();
        set_created_at(&pool, &type_name, &fallback, "2026-01-01 00:00:00").await;
        upsert(
            &pool,
            &registration(&newest, vec![2, 3, 4], 3.0, "newest blurb"),
        )
        .await
        .unwrap();
        set_created_at(&pool, &type_name, &newest, "2026-01-02 00:00:00").await;
        assert_eq!(
            descriptor_values(&pool, &type_name).await,
            (vec![2, 3, 4], 3.0, "newest blurb".to_string())
        );

        let obj = Arc::new(GameVersion {
            metadata: ObjectMeta {
                name: Some(newest.clone()),
                ..Default::default()
            },
            spec: GameVersionSpec {
                type_name: type_name.clone(),
                weight: 3.0,
                blurb: "newest blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
            },
            status: None,
        });
        let ctx = Arc::new(Ctx {
            client: kube_client("http://127.0.0.1:1"),
            pool: pool.clone(),
            http: reqwest::Client::new(),
        });
        cleanup(obj, ctx).await.unwrap();

        let is_public: bool =
            sqlx::query_scalar("SELECT is_public FROM game_versions WHERE name = $1")
                .bind(&newest)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!is_public, "cleanup must mark the version non-public");
        assert_eq!(
            descriptor_values(&pool, &type_name).await,
            (vec![2], 1.0, "fallback blurb".to_string())
        );
    }

    /// In-process stand-in for the game service behind the KEDA interceptor.
    /// Records the request kind of every request it serves.
    async fn start_game_service() -> (String, Arc<Mutex<Vec<&'static str>>>) {
        let seen = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let seen2 = Arc::clone(&seen);
        let app = Router::new().route(
            "/",
            post(move |Json(payload): Json<Request>| async move {
                let kind = match payload {
                    Request::PlayerCounts => "PlayerCounts",
                    Request::Rules => "Rules",
                    other => {
                        return Json(Response::SystemError {
                            message: format!("unsupported in mock: {other:?}"),
                        });
                    }
                };
                seen2.lock().unwrap().push(kind);
                Json(match kind {
                    "PlayerCounts" => Response::PlayerCounts {
                        player_counts: vec![2, 3, 4],
                    },
                    _ => Response::Rules {
                        rules: "rules text".to_string(),
                    },
                })
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), seen)
    }

    /// In-process stand-in for the Kubernetes API's GameVersion status
    /// subresource, recording every patch body it receives and echoing a
    /// valid GameVersion back (the real kube client deserializes the reply).
    async fn start_kube_api() -> (String, Arc<Mutex<Vec<serde_json::Value>>>) {
        let patches = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let patches2 = Arc::clone(&patches);
        let app = Router::new().route(
            "/apis/brdgme.com/v1/namespaces/{namespace}/gameversions/{name}/status",
            patch(
                move |Path((namespace, name)): Path<(String, String)>,
                      Json(body): Json<serde_json::Value>| async move {
                    patches2.lock().unwrap().push(body.clone());
                    Json(json!({
                        "apiVersion": "brdgme.com/v1",
                        "kind": "GameVersion",
                        "metadata": { "name": name, "namespace": namespace },
                        "spec": { "typeName": "Snapshot Apply Game", "interfaceVersion": 2 },
                        "status": body.get("status").cloned().unwrap_or(serde_json::Value::Null),
                    }))
                },
            ),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), patches)
    }

    // R-51/F-196 acceptance criterion 4: with unchanged observedGeneration and
    // NULL snapshots, apply bypasses the generation guard, fetches PlayerCounts
    // and Rules, persists the three-field snapshot, re-points the descriptors,
    // and writes observedGeneration through the status subresource.
    #[sqlx::test(migrations = "../web/migrations")]
    async fn apply_backfills_incomplete_snapshots_on_unchanged_generation(pool: PgPool) {
        let type_name = "Snapshot Apply Game".to_string();
        let fallback = "snapshot-apply-game-1".to_string();
        let newest = "snapshot-apply-game-2".to_string();
        let registration =
            |version: &str, player_counts: Vec<i32>, weight: f32, blurb: &str| Registration {
                type_name: type_name.clone(),
                version_name: version.to_string(),
                weight,
                blurb: blurb.to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts,
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules".to_string(),
            };
        upsert(
            &pool,
            &registration(&fallback, vec![2], 1.0, "fallback blurb"),
        )
        .await
        .unwrap();
        set_created_at(&pool, &type_name, &fallback, "2026-01-01 00:00:00").await;
        upsert(
            &pool,
            &registration(&newest, vec![2, 3, 4], 3.0, "newest blurb"),
        )
        .await
        .unwrap();
        set_created_at(&pool, &type_name, &newest, "2026-01-02 00:00:00").await;

        // Simulate a row persisted before the snapshot columns existed: the
        // newest authoritative version has NULL snapshots, so the descriptors
        // reconcile back onto the fallback.
        sqlx::query(
            "UPDATE game_versions \
             SET snapshot_player_counts = NULL, snapshot_weight = NULL, snapshot_blurb = NULL \
             WHERE name = $1",
        )
        .bind(&newest)
        .execute(&pool)
        .await
        .unwrap();
        let type_id: sqlx::types::Uuid =
            sqlx::query_scalar("SELECT id FROM game_types WHERE name = $1")
                .bind(&type_name)
                .fetch_one(&pool)
                .await
                .unwrap();
        brdgme_registration::reconcile_game_type_descriptors(&pool, type_id)
            .await
            .unwrap();
        assert_eq!(
            descriptor_values(&pool, &type_name).await,
            (vec![2], 1.0, "fallback blurb".to_string())
        );

        let _guard = APPLY_LOCK.lock().await;
        let (game_uri, seen) = start_game_service().await;
        let (kube_uri, patches) = start_kube_api().await;
        let _interceptor_uri = InterceptorUriGuard::set(&game_uri);

        let obj = Arc::new(GameVersion {
            metadata: ObjectMeta {
                name: Some(newest.clone()),
                namespace: Some("brdgme".to_string()),
                generation: Some(2),
                ..Default::default()
            },
            spec: GameVersionSpec {
                type_name: type_name.clone(),
                weight: 3.0,
                blurb: "newest blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
            },
            status: Some(GameVersionStatus {
                ready: true,
                message: None,
                observed_generation: Some(2),
            }),
        });
        let ctx = Arc::new(Ctx {
            client: kube_client(&kube_uri),
            pool: pool.clone(),
            http: reqwest::Client::new(),
        });
        apply(obj, ctx).await.unwrap();

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &["PlayerCounts", "Rules"],
            "equal generation with NULL snapshots must still hit the game service"
        );
        let snapshots: (Vec<i32>, f32, String) = sqlx::query_as(
            "SELECT snapshot_player_counts, snapshot_weight, snapshot_blurb \
             FROM game_versions WHERE name = $1",
        )
        .bind(&newest)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(snapshots, (vec![2, 3, 4], 3.0, "newest blurb".to_string()));
        assert_eq!(
            descriptor_values(&pool, &type_name).await,
            (vec![2, 3, 4], 3.0, "newest blurb".to_string())
        );
        let patches = patches.lock().unwrap();
        assert_eq!(patches.len(), 1, "expected exactly one status patch");
        assert_eq!(patches[0]["status"]["observedGeneration"], json!(2));
        assert_eq!(patches[0]["status"]["ready"], json!(true));
    }

    // R-51/F-196 acceptance criterion 4 paired assertion: a completed snapshot
    // on the same generation takes the normal no-request generation guard, so
    // neither the game service nor the status subresource is touched.
    #[sqlx::test(migrations = "../web/migrations")]
    async fn apply_skips_service_and_status_on_complete_snapshots_unchanged_generation(
        pool: PgPool,
    ) {
        let type_name = "Snapshot Skip Game".to_string();
        let version = "snapshot-skip-game-1".to_string();
        upsert(
            &pool,
            &Registration {
                type_name: type_name.clone(),
                version_name: version.clone(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
                player_counts: vec![2],
                uri: "http://localhost:0/mock".to_string(),
                rules: "rules".to_string(),
            },
        )
        .await
        .unwrap();

        let _guard = APPLY_LOCK.lock().await;
        let (game_uri, seen) = start_game_service().await;
        let (kube_uri, patches) = start_kube_api().await;
        let _interceptor_uri = InterceptorUriGuard::set(&game_uri);

        let obj = Arc::new(GameVersion {
            metadata: ObjectMeta {
                name: Some(version.clone()),
                namespace: Some("brdgme".to_string()),
                generation: Some(2),
                ..Default::default()
            },
            spec: GameVersionSpec {
                type_name: type_name.clone(),
                weight: 1.0,
                blurb: "blurb".to_string(),
                is_deprecated: false,
                interface_version: 2,
            },
            status: Some(GameVersionStatus {
                ready: true,
                message: None,
                observed_generation: Some(2),
            }),
        });
        let ctx = Arc::new(Ctx {
            client: kube_client(&kube_uri),
            pool: pool.clone(),
            http: reqwest::Client::new(),
        });
        apply(obj, ctx).await.unwrap();

        assert!(
            seen.lock().unwrap().is_empty(),
            "complete snapshots on an unchanged generation must not hit the game service"
        );
        assert!(
            patches.lock().unwrap().is_empty(),
            "complete snapshots on an unchanged generation must not patch status"
        );
    }
}
