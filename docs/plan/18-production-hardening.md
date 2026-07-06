# 18: Production Hardening

**Status:** Pending (fully specced 2026-07-05; delegable except the marked
human tasks)

**Goal:** Ensure errors are visible and diagnosable in production, where
optimised WASM strips debug info and panics are otherwise silent.

**Resequenced 2026-07-04: pre-go-live.** With the hard-cutover decision
(Phase 16) this phase moves before cutover so error visibility exists from
day one of prod traffic - the Phase 16 validation gate explicitly checks
the log/trace backend for unexplained 5xx/panics.

**Decision (2026-07-05): all-in Grafana Cloud free tier.** Supersedes the
2026-07-03 VictoriaLogs decision and the planned VictoriaMetrics/vmalert
additions. Logs, metrics, traces (APM), dashboards, and alerting (including
email delivery) all go to a Grafana Cloud free-tier stack; the only
in-cluster observability component is a single Grafana Alloy agent.
Rationale: the s-2vcpu-4gb single-node budget cannot comfortably carry
VictoriaLogs + Vector + VictoriaMetrics + vmalert + Alertmanager alongside
the app (~1-1.5GB), Grafana Cloud's free tier (50GB logs, 10k metric
series, 50GB traces, alert rules + email contact points) covers this
project's volume by orders of magnitude, and it adds real APM - wanted for
the hard cutover's first weeks. Exit path if the free tier degrades:
collection stays in-cluster (Alloy speaks Loki/Prometheus/OTLP protocols),
so the backends can be swapped back to self-hosted VictoriaLogs/
VictoriaMetrics by changing Alloy's write endpoints - no app changes.

**Decision (2026-07-05): no in-cluster alert evaluation.** vmalert /
Alertmanager / a webhook-to-Resend bridge are all trimmed - alert rules run
in Grafana Cloud against the shipped telemetry, and Grafana Cloud sends the
emails itself (Resend is NOT used for alerts). The blind spot ("the cluster
stopped shipping telemetry at all") is covered by (a) a Grafana Cloud
no-data alert on a heartbeat metric and (b) the external uptime monitor
below. Documented fallback if in-cluster alerting is ever needed: a
`POST /api/webhooks/alertmanager` endpoint on the monolith sending via the
existing `resend-rs` client - do not build it now.

**Note (2026-07-05):** client-side error *telemetry* - getting production
users' WASM panics to reach the operator at all - remains a known gap. It is
intentionally deferred as a separate future decision (Sentry, a
panic-reporting endpoint, etc. are all out of scope here). The WASM source
maps item below is a local debugging aid only: it makes a panic already
visible in someone's browser console (or reported by a user) resolve to a
real Rust file:line, not a mechanism for getting that panic to the operator.

### WASM client

- [x] **`console_error_panic_hook`**: already installed in `lib.rs::hydrate()`.
      Panics write the message and location to the browser console before
      aborting, even in release builds.

- [x] **ErrorBoundary**: investigated 2026-07-05 — Leptos's `<ErrorBoundary>`
      only catches `Result::Err` rendered in the view, not Rust panics.
      Audited every resource fetch in `rust/web` (GamePage, GameLogs,
      SidebarMenu, DashboardPage) — each already hand-rolls an
      `Err(...) => <fallback>` match arm. `RecentGameLogs` deliberately
      swallows errors since `GameLogs`'s sidebar shows the authoritative
      failure message for the same data. No gap to fill; nothing added.

- [ ] **WASM source maps**: investigated and prototyped end-to-end
      2026-07-05 - **blocked on a toolchain crash, not implemented.**
      Local debugging aid only (see note above), not telemetry.

      Recipe researched and confirmed correct in principle: (1) set
      `debug = true` on the `wasm-release` profile so `rustc` emits DWARF
      into the `wasm32-unknown-unknown` output; (2) `cargo-leptos` 0.3.6
      already has a native `--wasm-debug` flag ("Include debug information
      in Wasm output. Includes source maps and DWARF debug info") that
      passes `--keep-debug`/`--debug` to `wasm-bindgen` so the DWARF
      survives bindgen's walrus-based rewrite (the earlier premise that
      cargo-leptos has no native option was wrong - it does, just not
      documented in the book); (3) a `.wasm.map` would then be produced
      from the DWARF via `llvm-dwarfdump` + a vendored copy of
      Emscripten's `tools/wasm-sourcemap.py` (binaryen does not ship this
      script despite being the more commonly cited source - confirmed by
      inspecting the nixpkgs `binaryen` derivation, which only installs
      compiled tools, no Python scripts; `llvm-dwarfdump` isn't in
      `devenv.nix` yet either, but nixpkgs `llvm` provides it).

      **What actually blocks it:** `cargo-leptos` always runs `wasm-opt`
      on release builds (no supported way to skip it - only
      `wasm-opt-features` to change its flags, not disable it), and the
      pinned toolchain's `wasm-opt` (binaryen 129 via `devenv.nix`, the
      current nixpkgs version) **segfaults/aborts unconditionally on any
      `wasm-bindgen --keep-debug` output that contains a DWARF
      `.debug_info` section** - reproduced twice: once in an isolated
      throwaway crate, once against the real `rust/web` build via
      `cargo leptos build --release --wasm-debug --frontend-only`. The
      crash happens even with zero optimization passes (a plain
      parse-and-rewrite), so no flag combination avoids it - this is a
      hard incompatibility between this rustc's DWARF output and this
      binaryen version, not a tuning problem. Separately, even if the
      crash weren't there, skipping `wasm-opt` entirely to keep DWARF
      inflates the shipped `web.wasm` from ~1.2MB to ~92MB (measured) -
      unacceptable for a render-blocking asset, and cargo-leptos has no
      option to run `wasm-opt` post-strip while keeping DWARF around only
      long enough to snapshot a source map first.

      **What would unblock it:** a `binaryen`/`wasm-opt` release that
      doesn't crash on this DWARF shape (untested - nixpkgs' 129 is
      already close to current), or a `cargo-leptos` release that adopts
      the emscripten pattern of skipping `wasm-opt` specifically for
      debug builds and running it separately on a DWARF-stripped copy
      before the sourcemap snapshot is taken. Revisit if either upstream
      changes; no repo changes were made (Cargo.toml profile edits made
      during the prototype were reverted).

### Grafana Cloud account + Alloy collection

- [ ] **Grafana Cloud stack setup** *(human - account creation)*. Steps:
      1. Sign up at grafana.com (free tier, no card) and create a stack;
         pick the Australia/Sydney region if offered, else the closest.
      2. From the stack's Cloud Portal page, open each of **Loki**,
         **Prometheus**, and **Tempo/OTLP** "Details"/"Send data" pages
         and record: push/remote_write/OTLP endpoint URL + the numeric
         username (tenant/instance id) for each. They differ per signal -
         record all three pairs.
      3. Cloud Portal → Security → **Access Policies** → Create access
         policy: scopes `logs:write`, `metrics:write`, `traces:write` →
         Add token (no expiry or 1y - diarise renewal) → copy the token
         once.
      4. Store all values in the offline credential store, then seal them
         as the `grafana-cloud` Secret (Phase 15 sealing pattern; plain
         Secret if before Phase 15): keys `LOKI_URL`, `LOKI_USER`,
         `PROM_URL`, `PROM_USER`, `TEMPO_URL`, `TEMPO_USER`, `GC_TOKEN`
         (Alloy config consumes these names - keep them in sync with the
         Alloy manifests task).
      5. In the Grafana UI: Alerting → Contact points → edit the default
         email contact point to mick.alexander@gmail.com (or add one and
         set it in the default notification policy). Send the test
         notification and confirm it arrives.

- [x] **Grafana Alloy manifests** (`k8s/prod/alloy/` - prod-only, NOT in
      `k8s/base`; dev keeps reading logs via Tilt): Deployment (single
      replica is fine on a one-node cluster; switch to DaemonSet if a second
      node is added), ServiceAccount + RBAC (read pods/nodes for discovery
      and log tailing), ConfigMap with the Alloy config, and a ClusterIP
      Service exposing OTLP 4317 (grpc) + 4318 (http). Alloy config does
      three jobs:
      1. `loki.source.kubernetes` (or file-based pod log tailing) → Grafana
         Cloud Loki, attaching `namespace`, `pod`, `container`, `app` labels.
         Keep the label set to those four - the JSON structured fields the
         app already emits (trace_id, game_id, etc.) stay in the log body
         and are queryable via LogQL JSON parsing; do not promote them to
         labels (cardinality).
      2. `prometheus.scrape` of pods annotated
         `prometheus.io/scrape: "true"` (+ `port`/`path` annotations) →
         remote_write to Grafana Cloud. Also scrape CNPG (the operator pods
         and instance pods expose `/metrics` on 9187) and NATS (add the
         `prometheus-nats-exporter` sidecar OR scrape NATS's own
         monitoring port 8222 via the `nats` built-in exporter component -
         prefer whichever Alloy supports natively; decide at implementation
         and note it here).
      3. `otelcol.receiver.otlp` → `otelcol.exporter.otlp` to Grafana Cloud
         Tempo, for traces from the monolith (below).
      Credentials come from a `grafana-cloud` Secret (SealedSecret once
      Phase 15 lands; plain Secret during beta). Resource requests/limits
      explicit and small (target requests ~100m/128Mi; measure and adjust).

      **Implementation notes (2026-07-06):** manifests live in
      `k8s/prod/alloy/` (`serviceaccount.yaml`, `rbac.yaml`, `configmap.yaml`,
      `deployment.yaml`, `service.yaml`, `kustomization.yaml`), wired into
      `k8s/prod/app/kustomization.yaml`'s `bases:` as `../alloy`. Validated
      with `kubectl kustomize k8s/prod/app` (renders cleanly; the missing
      `grafana-cloud` Secret is expected and doesn't fail a kustomize build,
      only a real `kubectl apply`/runtime pod would notice). No live Alloy
      or Grafana Cloud stack exists yet, so River syntax is unverified
      against a real `alloy run` / `alloy fmt` - re-check the config with a
      live binary before first apply.

      - **NATS exporter decision:** used Alloy's native
        `prometheus.exporter.nats` component (`server = "http://nats:8222"`,
        NATS's existing monitoring port from `k8s/base/nats/stateful-set.yaml`'s
        `-m 8222` arg) rather than adding a `prometheus-nats-exporter`
        sidecar - one fewer container per NATS pod, and Alloy documents this
        as the native equivalent (it wraps the same exporter code
        in-process). Its `prometheus.scrape` output feeds the same
        `prometheus.remote_write` as everything else.
      - **CNPG scrape discovery assumption:** rather than a CNPG-specific
        job, the same annotation-based `discovery.relabel` used for the web
        pod's `prometheus.io/scrape`/`port`/`path` annotations (9090) is
        generic and will pick up CNPG instance pods automatically **once
        Phase 19's `Cluster` CR sets those same three annotations** (port
        9187) via `.spec.metadata.annotations` on the instance pod template
        - this is an assumption, not yet verified against a live CNPG
        instance (Phase 19 is still in progress); flagging for whoever lands
        Phase 19 to add the annotations and confirm discovery picks the pods
        up. CNPG operator pods (`cnpg-system` namespace) are out of scope
        here (not part of the `brdgme` app namespace's telemetry surface).
      - **RBAC: ClusterRole, not Role:** the kubelet/cadvisor node-metrics
        scrape job (for the "Node pressure" alert) needs cluster-scoped
        `nodes`/`nodes/proxy` access regardless, since nodes aren't
        namespaced - so a namespace-scoped `Role` wouldn't have been
        sufficient even if pod/log access alone could have been trimmed to
        `brdgme`. Rules granted: `pods`/`namespaces`/`nodes` (get/list/watch),
        `pods/log` (get, for `loki.source.kubernetes` tailing), and
        `nodes/proxy` + `nodes/metrics` (get, for the API-server-proxied
        cadvisor scrape - same pattern kube-prometheus-stack uses since
        kubelets aren't otherwise routable from in-cluster pods).
      - Self-monitoring: added a `prometheus.scrape` of Alloy's own
        `localhost:12345` metrics endpoint so `up`/`alloy_build_info` exist
        for the heartbeat/no-data alert (next bullet) without extra wiring.
      - Alloy image pinned to `grafana/alloy:v1.4.3`; bump alongside the
        Grafana Cloud stack setup task if a newer stable tag exists by then.

- [ ] **Heartbeat / no-data alert**: Alloy self-monitoring metrics (`up`,
      `alloy_build_info`) are already shipped by the scrape config. Add a
      Grafana Cloud alert rule: fire when `up` reports no data for 10
      minutes - this is the "cluster stopped shipping telemetry" alarm that
      replaces any in-cluster dead-man's-switch.

### APM / distributed tracing (added 2026-07-05 - wanted for cutover week)

- [x] **OTLP trace export from the monolith** (`rust/web`, ssr feature
      only): added `tracing-opentelemetry` 0.33, `opentelemetry` 0.32,
      `opentelemetry-otlp` 0.32 (`grpc-tonic` + `trace` features only,
      default features off - no reqwest/http transport pulled in),
      `opentelemetry_sdk` 0.32 (`trace` feature only), all `dep:`-gated in
      the `ssr` feature list, none reachable from `hydrate` (verified with
      `cargo tree -p web --no-default-features --features hydrate --target
      wasm32-unknown-unknown -e normal | grep -i otel` - no matches).
      `main.rs`'s `init_tracing()` replaces the old
      `tracing_subscriber::fmt().json().init()` one-liner with a
      `tracing_subscriber::registry().with(fmt_layer).with(otel_layer)
      .init()` composition (`Option<L>` implements `Layer`, so the otel
      layer is simply `None` when unset). Config: `OTEL_EXPORTER_OTLP_ENDPOINT`
      unset -> otel layer not installed at all, no exporter built, no
      connection attempted (dev default); set -> used as the gRPC endpoint
      via `SpanExporter::builder().with_tonic().with_endpoint(...)` (gRPC,
      not HTTP, matching prod's `alloy:4317` target - note
      `opentelemetry-otlp`'s own *cargo feature* default is `http-proto`,
      the crate's cargo default != the OTel spec's protocol default, hence
      the explicit `grpc-tonic` feature and explicit `.with_tonic()` call).
      `OTEL_SERVICE_NAME` (default `web`) sets the `service.name` resource
      attribute via `Resource::builder().with_service_name(...)`. Sampler:
      `Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(ratio)))`,
      `ratio` from `OTEL_TRACES_SAMPLER_ARG` parsed as f64, default/fallback
      1.0 on missing or unparseable input (logged as a `tracing::warn!`
      after the subscriber is initialized, never panics). A failed
      `SpanExporter` build (bad endpoint string) also degrades to no-otel
      with a warning rather than failing process startup.
- [x] **Span coverage**: implemented via a hand-wired `tower_http::trace::
      TraceLayer` in `router.rs` (chose hand-wiring over
      `axum-tracing-opentelemetry`: versions do align cleanly - 0.38 pulls
      in the same opentelemetry/tracing-opentelemetry 0.32/0.33 - but its
      span uses OTel semantic-convention field names and its own
      client-IP/propagation extraction, more surface than this task needs;
      the plain `TraceLayer` + `field::Empty`/`record()` pattern the brief
      already specified for step 3 was simpler to keep self-consistent).
      Root span (`http_request`) is the last `.layer()` call in
      `build_router`, wrapping the whole router including `/healthz`, with
      `route` from `axum::extract::MatchedPath` (fallback to raw path if
      unmatched - same low-cardinality reasoning as the `/metrics` labels),
      `status`/`latency_ms` recorded in `on_response`, and `trace_id`
      recorded immediately after span creation via
      `tracing_opentelemetry::OpenTelemetrySpanExt::context()` (this forces
      the lazy OTel span-context build, assigning a real trace id, without
      needing to explicitly `.enter()` first - `on_new_span`'s builder
      attach and `.context()`'s force-build both run synchronously outside
      of any enter/exit). `game/client.rs::request()` (confirmed via grep to
      be the sole call site all game-service HTTP calls funnel through -
      `pub_render`, `player_render`, and every `Request::New`/`Play`/etc.
      caller in `server_fns.rs`/`game/mod.rs`) is instrumented with
      `#[tracing::instrument(name = "game_service_request", skip(client,
      request), fields(game.uri = %uri))]` (no prior `#[instrument]`
      precedent in the codebase; chose the attribute over manual
      `.instrument()` for brevity). This span nests under the HTTP root
      span automatically whenever a request handler calls into the game
      client, no manual context passing needed. DB work remains
      uninstrumented per the plan text. `trace_id` in JSON logs is the real
      OTel id, not a homegrown one - verified with a throwaway test
      (constructed the same `make_root_span` + registry composition,
      captured JSON output, asserted a 32-hex-char id, then deleted the
      test - see the session report for the captured log line).
- [ ] **Verify in dev**: run Alloy + a local Grafana Cloud (or just Alloy's
      debug exporter) once, confirm a request produces a root span with the
      game-service child span, then verify the same end-to-end in prod
      during the Phase 16 beta period. Not done here - no Alloy/collector
      in this sandbox (no k8s cluster reachable, no local OTLP collector
      binary available). What *was* verified in this sandbox: the
      `trace_id` pipeline end-to-end at the tracing_subscriber level (root
      span -> real OTel trace id -> JSON log field, see the "Span coverage"
      note above); running the built `web` binary with a real but
      unreachable `OTEL_EXPORTER_OTLP_ENDPOINT` and a deliberately-bad
      `OTEL_TRACES_SAMPLER_ARG` produces exactly one clean JSON warn line
      (the sampler fallback) and no otel/exporter error before the process
      moves on to DB pool creation - the tonic gRPC channel to a
      non-listening endpoint is lazy and does not error at startup; running
      it again with the env var unset never calls any otel crate API at all
      (by inspection of `init_tracing`'s `None` branch - the exporter/
      provider/layer are only constructed inside the `Some(endpoint)` arm).
      Needs a real Alloy collector to confirm the root+child span nesting
      arrives in Tempo.

### Metrics from the app

- [x] **Monolith `/metrics`**: added `axum-prometheus` (which pulls in
      `metrics` + `metrics-exporter-prometheus` as its recorder backend) to
      `rust/web` (ssr only). **Deviation from the plan text above:** `/metrics`
      is served on a *second* port (9090), not the existing site port -
      `k8s/base/gateway/httproutes.yaml`'s `web` HTTPRoute has no `matches:`
      stanza, so it forwards every path on the main port to `Service web`;
      putting `/metrics` on that port would make it publicly reachable at
      `brdg.me/metrics`. Port 9090 is bound by its own `axum::serve` task
      spawned in `main.rs`, has no `Service` port or `HTTPRoute` (only
      reachable via direct pod-network/kubelet-style access, same pattern as
      the bot's `/healthz`), and is annotated on the Deployment
      (`prometheus.io/scrape`, `/port`, `/path`) for Alloy to discover later.
      Series implemented: HTTP request count + duration histogram labelled by
      (route via `axum::extract::MatchedPath`, status) via
      `PrometheusMetricLayer`; `ws_connections` gauge (RAII guard in
      `websocket.rs::handle_socket` covers every exit path including the
      early return on NATS subscribe failure); `login_emails_sent_total`
      counter in `send_login_email` (counts only real Resend API calls, not
      the dev-mode log-instead-of-send fallback, since it feeds the Resend
      quota alert below).
- [x] **Bot health endpoint (+ optional metrics)**: the bot lost its HTTP
      port in Phase 13 and has neither probe nor metrics. Add a minimal
      axum listener on `LISTEN_ADDR` (restore port 4000) serving:
      `/healthz` → 200 when the NATS client's `connection_state()` is
      `Connected`, 503 otherwise. `/metrics` optional in v1 (the JetStream
      consumer metrics visible from the NATS side cover queue depth).
      Wire it into `main.rs` alongside the message loop (`tokio::select!`
      or a spawned task). Tests: not meaningfully unit-testable beyond
      compile; verify via the probe in the dev cluster.

### Alert rules (Grafana Cloud - evaluated hosted, delivered by email)

*(human by default - these are configured in the Grafana Cloud UI; only
delegable if an agent is given a Grafana API token to provision them.)*
UI path per rule: Alerting → Alert rules → New alert rule → write the
query against the stack's Prometheus (metrics) or Loki (logs) datasource →
set the evaluation window/threshold from the rule text → leave routing on
the default notification policy (email contact point from the setup task).
After creating each rule, force-fire it once where cheap (e.g. temporarily
set the threshold to 0, wait one evaluation, restore) to prove delivery.
Initial rule set; thresholds are starting points, tune during the Phase 16
beta/validation window:

- [ ] Monolith 5xx: `>0.5%` of requests 5xx over 15m (from the `/metrics`
      histogram), AND an absolute guard: `>10` 5xx in 15m (catches
      low-traffic spikes the percentage misses).
- [ ] Error-log rate: LogQL count of `level=error` from the `web` and `bot`
      containers `>10` in 15m.
- [ ] Heartbeat/no-data (specced above under Alloy).
- [ ] CNPG backup freshness: alert if the newest base backup is older than
      26h or WAL archiving is failing (CNPG exposes
      `cnpg_pg_wal_archive_status`-family and backup metrics via its
      `/metrics`; pick the exact series during implementation and record it
      here).
- [ ] Node pressure: node memory working set >90% of allocatable for 15m;
      any PVC >85% full. (Node/kubelet/cadvisor metrics come with the Alloy
      scrape of the kubelet - include that in the Alloy config task.)
- [ ] Resend quota: login-email counter approaching the 100/day cap
      (>60/day). (22b will extend this when volume multiplies.)

### External uptime monitor (added 2026-07-05)

- [ ] *(human - account creation)* Free external HTTPS check - the only
      monitor that fires when the cluster, LB, DNS, or Grafana Cloud
      shipping is down wholesale. Steps (UptimeRobot free tier or
      equivalent):
      1. Sign up at uptimerobot.com; confirm the account email.
      2. Add New Monitor → type HTTP(s) → URL `https://beta.brdg.me/`
         (during the Phase 16 beta) → interval 5 minutes → alert contact:
         the account email (mick.alexander@gmail.com).
      3. Verify it reports Up, then take the beta stack briefly down (or
         pause a Deployment) once to confirm a Down email actually
         arrives.
      4. At cutover: edit the monitor's URL to `https://brdg.me/` (Phase
         16 runbook step).

### Probes (fleshed out 2026-07-05)

Audited: game services and legacy `api` already have HTTP probes; **web and
bot have none**; NATS/CNPG covered as below.

- [x] **web**: add readiness + liveness HTTP probes hitting a new `/healthz`
      route in `router.rs` that returns a plain 200 without touching the
      database (a DB outage must not restart or de-endpoint web pods - they
      serve error pages and the WS layer independently; document this
      rationale in a comment on the route). Initial delay ~5s, period 10s.
- [x] **bot**: liveness probe on the new `/healthz` (above) - NATS
      disconnected long enough to fail the probe means restart is the right
      remedy. No readiness probe needed (no Service traffic to gate;
      keep the Service for the port anyway or drop it - implementer's
      choice, note which). Chose to drop the Service: the port exists only
      for probe/scrape access from the kubelet, which reaches the pod
      directly without a Service.
- [x] **NATS**: confirm `k8s/base/nats` probes the monitoring port
      (`/healthz` on 8222 is the official pattern); add if missing.
- [ ] **CNPG / migrate Job**: operator-managed and Job respectively - no
      action, listed so the audit is complete.

### Capacity check (added 2026-07-05 - gate before cutover)

- [x] Ensure every prod workload has explicit resource **requests** (web,
      bot, game services, NATS, Alloy, CNPG via its `resources` stanza,
      ArgoCD components via its kustomize patch if defaults are too big -
      ArgoCD is the heaviest add; its defaults total ~1GB requests and can
      be patched down for a single-app install).
- [x] Sum requests vs the s-2vcpu-4gb node's allocatable (~2.5GiB after
      DOKS reservations) and record the table in this file. If the sum
      exceeds ~80% of allocatable, flag it - the decision to add a second
      node is Michael's and is deliberately deferred until the numbers
      force it; the deliverable here is the honest number, not the node.

**Capacity table (measured 2026-07-06 from manifests in this repo):**

| Workload | Replicas | CPU req (per-replica / total) | Mem req (per-replica / total) | Notes |
| --- | --- | --- | --- | --- |
| web | 2 | 50m / 100m | 128Mi / 256Mi | `k8s/base/web/deployment.yaml` |
| bot | 1 | 20m / 20m | 64Mi / 64Mi | `k8s/base/bot/deployment.yaml` |
| nats | 1 | 20m / 20m | 64Mi / 64Mi | StatefulSet, `k8s/base/nats/stateful-set.yaml` |
| alloy | 1 | 100m / 100m | 128Mi / 128Mi | `k8s/prod/alloy/deployment.yaml` |
| Game services (24 identical) | 24 | 10m / 240m | 32Mi / 768Mi | acquire-1, age-of-war-1, battleship-1, category-5-1, cathedral-1, farkle-1, farkle-2, for-sale-1, greed-1, greed-2, liars-dice-1, liars-dice-2, lost-cities-1, lost-cities-2, love-letter-1, modern-art-1, no-thanks-1, no-thanks-2, roll-through-the-ages-1, splendor-1, sushi-go-1, sushizock-1, texas-holdem-1, zombie-dice-1 |
| CNPG Postgres | 1 | not set | not set | `k8s/base/postgres/cluster.yaml` has no `.spec.resources` stanza - falls back to whatever the CNPG operator/container defaults are |
| CNPG operator + Barman Cloud plugin | unknown | unknown | unknown | `k8s/cnpg-operator/` installs unmodified upstream release manifests in `cnpg-system`; this repo does not pin/patch their requests and there's no live cluster to read actual values from |
| ArgoCD controller | not installed | - | ~1GB (estimate) | Controller (`argocd-server`/`repo-server`/`application-controller`/`redis`/`dex-server`) is not yet installed by anything in this repo - `k8s/argocd/brdgme-app.yaml` is only the `Application` CR. The ~1GB figure is upstream ArgoCD's stated default requests total, unverified against this cluster |

**Sum (measured workloads only - web, bot, nats, alloy, game services):**

- Total CPU requests: 480m of the node's 2000m allocatable = **24.0%**
- Total memory requests: 1280Mi (1.25GiB) of the node's ~2.5GiB allocatable = **50.0%**

Neither figure exceeds the ~80% flag threshold, so no callout is required for
the workloads this repo actually specs and controls.

- **Caveat - real headroom is smaller than 24%/50% suggests.** The CNPG
  operator + Barman Cloud plugin (unpatched upstream defaults) and a future
  ArgoCD controller install (~1GB estimate, not yet installed) are excluded
  from the sum above because neither has a measurable request in this repo
  and there's no live cluster to inspect. If ArgoCD's ~1GB estimate alone
  were added to the measured memory total, it would push memory to roughly
  2304Mi of ~2.5GiB (~90%) - over the 80% threshold. The CNPG operator and
  Barman plugin footprint is additional on top of that. This is a real gap
  worth closing before installing ArgoCD (per the existing bullet above,
  patching its kustomize defaults down), but per the task scope that
  decision and any manifest changes are deferred to Michael.

### Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.

