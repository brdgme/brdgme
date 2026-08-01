# Alloy / Grafana Cloud Removal - Design

Date: 2026-07-21
Status: Complete 2026-07-21. Alloy and Grafana Cloud removed from app and cluster. Companion to
`2026-07-21-sentry-performance-tracing-design.md` (which backfills app
observability) and `2026-07-21-managed-postgres-design.md` (which removes the
last workload the Alloy CNPG metrics watched).

## Problem

Grafana Alloy is the cluster's telemetry collector: it tails brdgme-namespace
logs to Loki, scrapes web/CNPG/cAdvisor/kubelet metrics to Prometheus, and
ships everything to Grafana Cloud (config in `k8s/prod/alloy/configmap.yaml`).
It runs at ~232Mi (Burstable, 128Mi request / 256Mi limit) on a memory-bound
2-node cluster, has a history of OOMKills when a remote_write endpoint stalls
(backlog #41b), and - per the owner - is not delivering enough value for the
resources it consumes. The Grafana Cloud dashboards/alerts it feeds are not
being looked at; the traces pipeline was already disabled (#41b) because the
Tempo exporter was stuck. Meanwhile Sentry is being expanded to cover
application errors + performance (companion spec), and `kubectl top` /
metrics-server cover live resource checks.

## Decision

Remove Alloy and Grafana Cloud from the app and the cluster entirely. Keep
metrics-server. Rely on Sentry (errors/perf/uptime/alerts), `kubectl logs`
(ad-hoc log inspection), and DigitalOcean's built-in droplet monitoring
(node-level CPU/memory/disk) for the residual needs.

This is a removal, not a replacement: no lighter collector (Fluent Bit,
vmagent, etc.) is introduced. The owner has judged the residual observability
loss acceptable against the ~232Mi and the operational surface recovered.

## What is removed

### Cluster (this repo, ArgoCD-managed)

- `k8s/prod/alloy/` - the entire directory: `deployment.yaml`,
  `configmap.yaml`, `rbac.yaml`, `service.yaml`, `serviceaccount.yaml`,
  `kustomization.yaml`.
- The `- ../alloy` resource reference in `k8s/prod/app/kustomization.yaml`
  (line 5).

### Cluster (brdgme-config repo, out of this repo - operator action, noted)

- The `grafana-cloud` SealedSecret (in `brdgme-config/sealed-secrets/secrets/`)
  that supplies `LOKI_URL`/`LOKI_USER`/`PROM_URL`/`PROM_USER`/`GC_TOKEN` to
  Alloy. Removed once Alloy is gone. (Not committed from this repo; recorded
  as an operator step.)

### App (`rust/web`)

The OpenTelemetry export wiring that fed Alloy/Tempo (Grafana Cloud traces).
This is already inert - `OTEL_EXPORTER_OTLP_ENDPOINT` was removed in #41b, so
`init_tracing` never installs the OTel layer - but it is dead code that is
part of "the app" and is removed here:

- `rust/web/src/main.rs` `init_tracing`: remove the OTLP exporter /
  `otel_layer` construction (the `OTEL_EXPORTER_OTLP_ENDPOINT`-gated block),
  the `OTEL_TRACES_SAMPLER_ARG` read, and the returned `_tracer_provider`
  plumbing. Keep the `tracing_subscriber` registry with the `fmt` (JSON
  stdout) layer and the `sentry_tracing::layer()`.
- `rust/web/src/router.rs` `make_root_span`: remove the OpenTelemetry
  `trace_id` correlation (`opentelemetry::trace::{TraceContextExt, TraceId}`
  and `tracing_opentelemetry::OpenTelemetrySpanExt` usage). The `http_request`
  span keeps `method`/`route`/`status`/`latency_ms`; the `trace_id` field
  (only ever `TraceId::INVALID` without the OTel layer) is dropped.
- `rust/web/Cargo.toml`: drop the now-unused OpenTelemetry dependencies
  (`opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`,
  `tracing-opentelemetry`) once nothing references them.
- `k8s/prod/app/web-patch.yaml`: remove the inert `OTEL_TRACES_SAMPLER_ARG`
  env var. KEEP the `SENTRY_DSN_WEB` / `SENTRY_DSN_SERVER` vars (Sentry stays).

## What is kept

- **metrics-server** (kube-system, manual addon): powers `kubectl top` and
  any future HPA/VPA. Unrelated to Alloy; untouched.
- **`tracing_subscriber` fmt (JSON) layer** in web/bot/operator: structured
  logs to stdout, inspected via `kubectl logs`. Unchanged.
- **Sentry** (errors + the performance/tracing being added in the companion
  spec): the application-observability path going forward.
- **DigitalOcean droplet monitoring**: built-in node CPU/memory/disk graphs
  and DO's own alerting, covering the node-level view at the provider level.

## What is lost, and the replacement

| Lost | Replacement |
|------|-------------|
| Centralized log search (Loki) | `kubectl logs` (ad-hoc); Sentry for app errors |
| Metrics dashboards over time (Prometheus/Grafana) | Sentry performance (app); DO droplet metrics (node); `kubectl top` (live) |
| Node-pressure alert (cAdvisor `container_memory_working_set_bytes`) | DO droplet monitoring/alerts; `kubectl top nodes` |
| PVC-fullness alert (kubelet `kubelet_volume_stats_*`) | DO managed-DB storage alerts once Postgres is managed (companion spec); manual `kubectl` checks for the remaining PVCs |
| CNPG backup-freshness alert | DO managed-DB automated backups + their own monitoring (companion spec makes this moot) |
| Alloy heartbeat / no-data alert | Sentry uptime monitor (companion spec) |

## Memory / complexity impact

- ~232Mi of working set removed from the cluster (one Burstable pod), plus
  the headroom from its 128Mi request / 256Mi limit no longer being
  reserved/overcommitted.
- One fewer always-on deployment, its RBAC, and an external SaaS dependency
  (Grafana Cloud) and secret (`grafana-cloud`) removed.
- The dead OTEL export path removed from `rust/web` (smaller dependency tree,
  less inert config to reason about).

## Sequencing

Independent of the Sentry spec, but best landed AFTER (or alongside) the
Sentry Phase 1 enablement so app observability exists before the Grafana
dashboards/alerts go away. The managed-Postgres move (companion spec) removes
the last workload the CNPG metrics watched, so it further de-risks the
removal but is not a prerequisite.

## Out of scope (rejected)

- Replacing Alloy with a lighter collector (Fluent Bit for logs, vmagent for
  metrics): the owner wants the function removed, not re-homed. Revisited
  only if centralized log search or metric alerting proves genuinely missed.
- Removing metrics-server: kept deliberately (tiny, powers `kubectl top` /
  future autoscaling).
- Touching ArgoCD: left as-is per the owner's decision this session.

## Success criteria

1. No `alloy` pod, Deployment, ConfigMap, Service, ServiceAccount, or RBAC
   remains in the cluster; the ArgoCD app syncs green without `../alloy`.
2. `rust/web` builds with no OpenTelemetry references; `init_tracing` retains
   only the fmt + sentry layers; `router.rs` has no `trace_id`/OTel imports.
3. `kubectl top nodes`/`pods` still works (metrics-server intact).
4. Sentry continues to receive errors (and, per the companion spec,
   transactions) - confirming Sentry does not depend on Alloy.
5. The `grafana-cloud` secret is deleted from brdgme-config (operator
   confirmed) and no pod references it.
