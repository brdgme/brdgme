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

- [ ] **Grafana Alloy manifests** (`k8s/prod/alloy/` - prod-only, NOT in
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

- [ ] **Heartbeat / no-data alert**: Alloy self-monitoring metrics (`up`,
      `alloy_build_info`) are already shipped by the scrape config. Add a
      Grafana Cloud alert rule: fire when `up` reports no data for 10
      minutes - this is the "cluster stopped shipping telemetry" alarm that
      replaces any in-cluster dead-man's-switch.

### APM / distributed tracing (added 2026-07-05 - wanted for cutover week)

- [ ] **OTLP trace export from the monolith** (`rust/web`, ssr feature
      only): add `tracing-opentelemetry`, `opentelemetry`,
      `opentelemetry-otlp`, `opentelemetry_sdk` and layer them into the
      existing `tracing_subscriber` registry in `main.rs`. Config via
      standard env: `OTEL_EXPORTER_OTLP_ENDPOINT` (prod:
      `http://alloy.<ns>.svc:4317`; unset = layer not installed - dev needs
      no collector), `OTEL_SERVICE_NAME=web`. Sampling: parent-based ratio,
      default 1.0 (traffic is tiny; make it `OTEL_TRACES_SAMPLER_ARG`-
      tunable so it can be dialed down if the 50GB tier ever matters).
      All exported gated behind the ssr feature; MUST NOT enter the WASM
      build (same cfg discipline as the rest of `main.rs`).
- [ ] **Span coverage**: HTTP server spans via `tower-http`'s `TraceLayer`
      (or `axum-tracing-opentelemetry` if the hand-wiring is awkward -
      implementer's choice, note which) so every request gets a root span
      with route, status, and latency; game-service calls in
      `game/client.rs` wrapped in a client span carrying `game.uri` so slow
      game services are visible per-request; DB work is NOT individually
      instrumented in v1 (sqlx span-per-query is noisy - revisit only if a
      real incident needs it). The `trace_id` already emitted in JSON logs
      must be the OTel trace id (use the otel layer's ids, not a homegrown
      one) so Grafana links logs ↔ traces.
- [ ] **Verify in dev**: run Alloy + a local Grafana Cloud (or just Alloy's
      debug exporter) once, confirm a request produces a root span with the
      game-service child span, then verify the same end-to-end in prod
      during the Phase 16 beta period.

### Metrics from the app

- [ ] **Monolith `/metrics`**: add `metrics` + `metrics-exporter-prometheus`
      (or `axum-prometheus`, implementer's choice) to `rust/web` (ssr only),
      exposing Prometheus text format on the existing port at `/metrics`
      (internal - the Gateway HTTPRoutes must NOT route it; verify it is
      unreachable via `brdg.me/metrics`). Minimum series: HTTP request
      count/duration histogram by (route, status), WS connection gauge,
      login-email send counter (feeds the Resend quota alert below).
      Annotate the web Deployment for Alloy scraping.
- [ ] **Bot health endpoint (+ optional metrics)**: the bot lost its HTTP
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

- [ ] **web**: add readiness + liveness HTTP probes hitting a new `/healthz`
      route in `router.rs` that returns a plain 200 without touching the
      database (a DB outage must not restart or de-endpoint web pods - they
      serve error pages and the WS layer independently; document this
      rationale in a comment on the route). Initial delay ~5s, period 10s.
- [ ] **bot**: liveness probe on the new `/healthz` (above) - NATS
      disconnected long enough to fail the probe means restart is the right
      remedy. No readiness probe needed (no Service traffic to gate;
      keep the Service for the port anyway or drop it - implementer's
      choice, note which).
- [ ] **NATS**: confirm `k8s/base/nats` probes the monitoring port
      (`/healthz` on 8222 is the official pattern); add if missing.
- [ ] **CNPG / migrate Job**: operator-managed and Job respectively - no
      action, listed so the audit is complete.

### Capacity check (added 2026-07-05 - gate before cutover)

- [ ] Ensure every prod workload has explicit resource **requests** (web,
      bot, game services, NATS, Alloy, CNPG via its `resources` stanza,
      ArgoCD components via its kustomize patch if defaults are too big -
      ArgoCD is the heaviest add; its defaults total ~1GB requests and can
      be patched down for a single-app install).
- [ ] Sum requests vs the s-2vcpu-4gb node's allocatable (~2.5GiB after
      DOKS reservations) and record the table in this file. If the sum
      exceeds ~80% of allocatable, flag it - the decision to add a second
      node is Michael's and is deliberately deferred until the numbers
      force it; the deliverable here is the honest number, not the node.

### Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.

