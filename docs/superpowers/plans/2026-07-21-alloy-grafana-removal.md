# Alloy / Grafana Cloud Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Grafana Alloy and the Grafana Cloud dependency from the app and the cluster, recovering ~232Mi and operational surface, while keeping metrics-server, stdout logs, and Sentry.

**Architecture:** Delete the ArgoCD-managed `k8s/prod/alloy/` manifests and their kustomization reference; remove the now-dead OpenTelemetry export wiring from `rust/web` (the OTLP layer was inert since #41b removed `OTEL_EXPORTER_OTLP_ENDPOINT`); drop the inert `OTEL_TRACES_SAMPLER_ARG` env var. The `grafana-cloud` SealedSecret lives in the separate `brdgme-config` repo and is removed there as an operator step.

**Tech Stack:** kustomize/ArgoCD manifests, Rust (tracing_subscriber, tracing-opentelemetry removal), OpenTofu/kubectl for the out-of-repo secret.

## Global Constraints

- Keep metrics-server (kube-system addon) - do NOT touch it.
- Keep the `tracing_subscriber` JSON fmt layer and the `sentry_tracing::layer()` in `rust/web` - only the OTLP export path is removed.
- Keep the Sentry DSN env vars (`SENTRY_DSN_WEB`/`SENTRY_DSN_SERVER`) in `k8s/prod/app/web-patch.yaml` - only `OTEL_TRACES_SAMPLER_ARG` is removed.
- Sentry must still receive events after removal (it sends direct to SaaS, independent of Alloy).
- The `grafana-cloud` secret removal happens in `brdgme-config` (operator action), only AFTER Alloy is gone and nothing references it.
- Best landed after/alongside Sentry Phase 1 (so app observability exists before the Grafana dashboards go). Not strictly required.
- Rust gate before commit (AGENTS.md): `cargo fmt --all -- --check`; `cargo clippy -p web --all-targets --features ssr -- -D warnings`.

## File Structure

- `rust/web/src/main.rs` - rewrite `init_tracing` to drop the OTLP layer + provider; update the caller.
- `rust/web/src/router.rs` - drop the OTel `trace_id` correlation from `make_root_span` + the OTel imports.
- `rust/web/Cargo.toml` - remove the OpenTelemetry dependencies.
- `k8s/prod/app/web-patch.yaml` - remove `OTEL_TRACES_SAMPLER_ARG`.
- `k8s/prod/alloy/` - delete the whole directory.
- `k8s/prod/app/kustomization.yaml` - remove the `- ../alloy` reference.

---

### Task 1: Remove the OTLP export layer from `rust/web`

**Files:**
- Modify: `rust/web/src/main.rs:15` (caller) and `rust/web/src/main.rs:108-211` (init_tracing)

**Interfaces:**
- Produces: `init_tracing()` returning `()` (no provider), registry retaining only env_filter + fmt + sentry layers.

- [ ] **Step 1: Update the caller in `main`**

In `rust/web/src/main.rs`, change line 15 from:

```rust
    let _tracer_provider = init_tracing();
```

to:

```rust
    init_tracing();
```

- [ ] **Step 2: Rewrite `init_tracing` without the OTLP layer**

Replace the whole `init_tracing` function (the doc comment + body, ~lines 108-211) with:

```rust
/// Sets up the `tracing_subscriber` registry: JSON logs to stdout always, plus
/// the Sentry tracing layer (errors -> Sentry events, warn/info -> breadcrumbs,
/// tracing spans -> Sentry spans). The former OTLP trace-export layer was
/// removed 2026-07-21 with Alloy/Grafana Cloud
/// (docs/superpowers/specs/2026-07-21-alloy-grafana-removal-design.md).
#[cfg(feature = "ssr")]
fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false);

    // Governs the fmt layer (RUST_LOG unset -> "info" default).
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        // Unconditional: forwards `error`-level events as Sentry events and
        // `warn`/`info` as breadcrumbs (sentry-tracing's default filter);
        // no-ops with no client initialized (dev/Tilt/CI).
        .with(sentry_tracing::layer())
        .init();
}
```

- [ ] **Step 3: Build to confirm the OTEL references are gone from main.rs**

Run: `cargo build -p web --features ssr`
Expected: FAILS only if `router.rs` / `Cargo.toml` still reference OTel (handled in Tasks 2-3) - proceed; this confirms main.rs itself is clean once those land.

- [ ] **Step 4: Commit (after Tasks 2-3 make the build pass)**

```bash
git add rust/web/src/main.rs
git commit -m "refactor(web): remove inert OTLP trace-export layer from init_tracing"
```

### Task 2: Remove the OTel `trace_id` correlation from `router.rs`

**Files:**
- Modify: `rust/web/src/router.rs:15,21` (imports) and `rust/web/src/router.rs:35-63` (make_root_span)

**Interfaces:**
- Consumes: nothing OTel after this change.
- Produces: `http_request` span with `method`/`route`/`status`/`latency_ms` only.

- [ ] **Step 1: Remove the OTel imports**

In `rust/web/src/router.rs`, delete line 15 and line 21:

```rust
use opentelemetry::trace::{TraceContextExt, TraceId};
```
```rust
use tracing_opentelemetry::OpenTelemetrySpanExt;
```

- [ ] **Step 2: Simplify `make_root_span`**

Replace `make_root_span` and its doc comment (~lines 35-63) with:

```rust
/// Root span for every HTTP request, carrying route (matched path, not raw
/// path - low-cardinality), status, and latency.
fn make_root_span(request: &Request<axum::body::Body>) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path());
    tracing::info_span!(
        "http_request",
        method = %request.method(),
        route = %route,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}
```

- [ ] **Step 3: Build**

Run: `cargo build -p web --features ssr`
Expected: compiles once Task 3 removes the now-unused deps.

- [ ] **Step 4: Commit (after Task 3)**

```bash
git add rust/web/src/router.rs
git commit -m "refactor(web): drop OTel trace_id correlation from http_request span"
```

### Task 3: Remove the OpenTelemetry dependencies

**Files:**
- Modify: `rust/web/Cargo.toml`

- [ ] **Step 1: Remove the OTel crates**

In `rust/web/Cargo.toml`, delete the dependencies that are now unreferenced: `opentelemetry`, `opentelemetry_sdk` (`opentelemetry-sdk`), `opentelemetry-otlp`, and `tracing-opentelemetry`. (Confirm exact names/lines by searching the file; these are the four crates `init_tracing`/`router.rs` used.)

- [ ] **Step 2: Build + clippy + fmt**

Run: `cargo build -p web --features ssr`
Expected: compiles, no unresolved imports.

Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean (no unused-dependency or unused-import warnings).

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 3: Confirm no OTel references remain in web**

Run: `grep -rn "opentelemetry\|tracing_opentelemetry\|OTEL_" rust/web/src`
Expected: no matches (the `OTEL_` env reads are gone; Sentry references remain).

- [ ] **Step 4: Commit**

```bash
git add rust/web/Cargo.toml rust/Cargo.lock
git commit -m "chore(web): remove OpenTelemetry dependencies (Alloy/Grafana removal)"
```

### Task 4: Remove the inert `OTEL_TRACES_SAMPLER_ARG` env var

**Files:**
- Modify: `k8s/prod/app/web-patch.yaml`

- [ ] **Step 1: Delete the OTEL env entry, keep the Sentry DSNs**

In `k8s/prod/app/web-patch.yaml`, remove the `OTEL_TRACES_SAMPLER_ARG` env entry (and update the surrounding comment, which currently explains the inert OTEL var). KEEP the `SENTRY_DSN_WEB` and `SENTRY_DSN_SERVER` entries. The `env:` list should retain only the two Sentry DSNs.

- [ ] **Step 2: Validate the manifest still parses**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml apply --dry-run=client -f k8s/prod/app/web-patch.yaml` is NOT applicable (it is a kustomize patch); instead build the app kustomization:
Run: `kubectl kustomize k8s/prod/app` (or `kustomize build k8s/prod/app`)
Expected: renders without error; the `web` Deployment env shows the two Sentry DSNs and no `OTEL_TRACES_SAMPLER_ARG`.

- [ ] **Step 3: Commit**

```bash
git add k8s/prod/app/web-patch.yaml
git commit -m "chore(k8s): remove inert OTEL_TRACES_SAMPLER_ARG from web patch"
```

### Task 5: Delete the Alloy manifests + kustomization reference

**Files:**
- Delete: `k8s/prod/alloy/` (deployment.yaml, configmap.yaml, rbac.yaml, service.yaml, serviceaccount.yaml, kustomization.yaml)
- Modify: `k8s/prod/app/kustomization.yaml:5` (remove `- ../alloy`)

- [ ] **Step 1: Remove the `../alloy` resource reference**

In `k8s/prod/app/kustomization.yaml`, delete the `- ../alloy` line from the `resources:` list.

- [ ] **Step 2: Delete the alloy directory**

Run: `git rm -r k8s/prod/alloy/`
Expected: the six alloy manifests are staged for deletion.

- [ ] **Step 3: Verify the app kustomization builds without Alloy**

Run: `kubectl kustomize k8s/prod/app`
Expected: renders with NO Alloy Deployment/ConfigMap/Service/ServiceAccount/RBAC; web/bot/operator/postgres/etc. still present.

- [ ] **Step 4: Commit**

```bash
git add k8s/prod/app/kustomization.yaml
git commit -m "chore(k8s): remove Alloy collector and Grafana Cloud telemetry pipeline"
```

### Task 6: Operator step - remove the `grafana-cloud` secret (brdgme-config)

**Files:** none in this repo (brdgme-config + cluster)

- [ ] **Step 1: Confirm nothing references the secret**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get deploy -n brdgme -o json | jq -r '.items[].spec.template.spec.containers[].envFrom[]?.secretRef?.name' | sort -u`
Expected: `grafana-cloud` is NOT listed (Alloy was its only consumer).

- [ ] **Step 2: Delete the SealedSecret + Secret in brdgme-config (operator, separate repo)**

In the `brdgme-config` repo, remove the `grafana-cloud` SealedSecret manifest from `sealed-secrets/secrets/` and its reference in the prod kustomization; apply/sync so ArgoCD prunes the `grafana-cloud` Secret from the cluster. (Out of this repo; recorded here for the commit session/operator.)

- [ ] **Step 3: Confirm the secret is gone from the cluster**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get secret -n brdgme grafana-cloud`
Expected: `NotFound`.

### Task 7: Final verification

- [ ] **Step 1: Rust gate**

Run: `cargo fmt --all -- --check`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean.

- [ ] **Step 2: Cluster state**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get all -n brdgme | grep -i alloy`
Expected: no output (after ArgoCD syncs the ref bump in brdgme-config).

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml top nodes`
Expected: still works (metrics-server intact); node memory usage drops by ~Alloy's working set after sync.

- [ ] **Step 3: Sentry still receiving**

Confirm Sentry (errors, and Phase-1 transactions if landed) still receives events - proving Sentry is independent of Alloy.

- [ ] **Step 4: Update the spec Status line**

Mark `docs/superpowers/specs/2026-07-21-alloy-grafana-removal-design.md` Status as complete with the date.

```bash
git add docs/superpowers/specs/2026-07-21-alloy-grafana-removal-design.md
git commit -m "docs: mark Alloy/Grafana Cloud removal complete"
```
