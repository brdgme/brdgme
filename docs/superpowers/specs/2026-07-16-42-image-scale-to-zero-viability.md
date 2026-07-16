# Backlog #42: Image Size Reduction and Scale-to-Zero Viability

One-line summary: from-scratch/distroless game images are viable now (-64% size, zero build changes); scale-to-zero for non-latest game versions will use the KEDA HTTP add-on (chosen 2026-07-16 by Michael - see verdict), with core-KEDA metrics-api as second choice and a bespoke operator shim as third; a NATS-event-driven design does not fit brdgme's actual RPC pattern.

Date: 2026-07-16
Status: research/viability only, no changes made.

Implementation plan: docs/superpowers/plans/2026-07-16-42-image-scale-to-zero.md

## 1. Current State (Grounded)

- Cluster: 2-node DOKS, s-2vcpu-4gb each (~8Gi total memory).
- ~39 game-worker Deployments (multiple versions across 22 games), 1 replica each, requests 32Mi/10m, limits 64Mi. Measured: ~1.25Gi memory requests reserved by games while mostly idle (task brief's ~1.6Gi estimate is not what measurement shows).
- Game workers are Rust `warp` HTTP servers, stateless, JSON request/response, no DB/TLS/outbound calls. Listen address from `ADDR` env, default `0.0.0.0:80` (e.g. `/home/beefsack/Development/brdgme/rust/game/no-thanks-2/src/bin/no_thanks_2_http.rs`). Graceful SIGTERM shutdown already implemented via `tokio::signal` in `/home/beefsack/Development/brdgme/rust/lib/cmd/src/http.rs`.
- Web calls workers synchronously during the user's submit-move request: `/home/beefsack/Development/brdgme/rust/web/src/game/client.rs` (reqwest, connect_timeout 5s / timeout 10s set in `rust/web/src/main.rs`, **no retry logic**). Target URI comes from `game_versions.uri` in Postgres (`rust/web/src/db.rs`: `find_game_version` / `find_latest_non_deprecated_game_version`).
- Old-version deployments still receive live traffic: `games.game_version_id` is fixed at game creation; only new games use the latest version. Old versions go idle only once their games finish.
- NATS + JetStream is deployed (`k8s/base/nats/stateful-set.yaml`, 20m/64Mi) but used only for web<->bot eventing (`/home/beefsack/Development/brdgme/rust/web/src/nats.rs`: JetStream stream `BOT`, subjects `bot.>`, WorkQueue retention, durable pull consumers `bot-turn`/`bot-command`, ack_wait 5m, max_deliver 3) plus core-NATS `game.{id}` pub/sub for websocket UI updates. It has no involvement in game-worker RPC today.
- The custom operator (`/home/beefsack/Development/brdgme/rust/operator`, `controller.rs`) registers game versions into Postgres; it does not manage Deployments. Deployments are static manifests under `/home/beefsack/Development/brdgme/k8s/base/game/<game>/` (`deployment.yaml` + `service.yaml`, tcpSocket probes on port 80) plus brdgme-config.
- Images: single shared `/home/beefsack/Development/brdgme/rust/Dockerfile` (cargo-chef, `lukemathwalker/cargo-chef:0.1.77-rust-1.97.0-bookworm` builder), 26-target `docker-bake.hcl` matrix. All 22 game runtime stages: `FROM debian:bookworm-slim` + COPY binary + exec-form CMD. Measured ~30.3MiB compressed per game image; the shared `bookworm-slim` base layer is ~28.2MiB of that (93%), binary layer ~2MiB. Registry is GHCR, which has a known pull-QPS problem. The Dockerfile carries a comment warning of a past GLIBC_2.38 builder/runtime drift crash.

## 2. Q1: From-Scratch / Distroless Images

**Verdict: sensible. Adopt `gcr.io/distroless/cc-debian12` as step one.**

- Binaries are glibc-dynamic (bookworm builder, default gnu target) - plain `scratch` fails without libc. Two viable paths: (a) `distroless/cc-debian12` (keeps glibc, zero Rust build changes), or (b) musl static build + `scratch`/`distroless/static`.
- Runtime needs on a minimal base: none beyond libc. No TLS/ca-certs (no outbound calls), no DNS, no tzdata (no `chrono`/`time` in any `*_http` binary - only `rand_bot` uses chrono and it isn't in an HTTP binary), no `/tmp` usage, CMD already exec-form, tcpSocket probes need no shell, SIGTERM handling already in-binary (no tini needed).
- Measured image sizes (docker manifest inspect, amd64 compressed):

| Base | Compressed size |
|---|---|
| `debian:bookworm-slim` (current) | ~28.2 MiB |
| `distroless/cc-debian12` | ~8.75 MiB |
| `distroless/static-debian12` (+musl) | ~0.67 MiB |
| `scratch` (+musl) | 0 |

- Per-image outcome: current ~30.3MiB -> distroless/cc ~10.8MiB (-64%, zero build changes) -> distroless/static+musl ~2.7MiB (-91%) -> scratch+musl ~2MiB (-93%).
- Fleet effect (estimate): 22 images x 20-28MiB saved = ~450-620MiB less to pull/cache per node; helps GHCR pull-QPS and node disk.
- Honesty note: base image size does not reduce container RSS. Savings are pull time, disk, and page cache only - not memory requests/limits.
- Hardening add-ons (optional, separable): numeric non-root USER; binding port 80 as non-root needs `CAP_NET_BIND_SERVICE` or moving to 8080 (ADDR env already supports this but touches ~44 manifest files - containerPort, tcpSocket probes, service targetPort). Treat as a follow-up, not required for the base swap.
- musl route costs: needs a musl target + cross toolchain added to the shared chef/builder stage (which also serves the web build - risk of breaking that path); musl's mallocng has an unbenchmarked perf risk under multithreaded tokio (likely low impact here). `distroless/cc` eliminates the GLIBC-drift bug class the Dockerfile already warns about, with zero build changes.
- Other risks: no shell -> use kubectl debug ephemeral containers (supported on this cluster's k8s version); vulnerability scanning is fine on distroless (Trivy/Grype support it, less CVE noise than a full Debian userland).
- **Maturity and ecosystem (verified 2026-07):**
  - `GoogleContainerTools/distroless`: ~22.7k GitHub stars, actively maintained, automated base-update PRs for CVE patches (SUPPORT_POLICY.md); Debian 12 LTS support through ~2028; images cosign-signed; no deprecation signals (GoogleContainerTools/distroless, 2026-07).
  - Registry: legacy GCR shut down reads on 2025-06-03, but `gcr.io/distroless/*` is served via Artifact Registry and explicitly unaffected (Google Cloud, 2025-06); pin by digest anyway.
  - Alternatives check: Chainguard's free tier has been restricted to `:latest` tags since 2024-11-21 (pinned tags require a paid subscription; a limited "Catalog Starter" free tier was added 2026-03) - worse for reproducible pinned builds than distroless (Chainguard, 2026-03). Ubuntu chiseled images are newer with less adoption. Alpine has a musl ABI mismatch for glibc binaries. `distroless/cc` remains the mainstream default for glibc-dynamic binaries.

**Recommendation:** swap all 22 game runtime stages to `distroless/cc-debian12` + numeric non-root USER now. musl+scratch is a valid *later* step only if GHCR pull pressure persists after this (further ~8MiB/image estimate).

## 3. Q2: Scale-to-Zero

| Option | Idle overhead (estimate) | Fits current RPC pattern? | Maturity | Verdict |
|---|---|---|---|---|
| 1. KEDA core + JetStream scaler | ~70-160Mi (operator + metrics adapter) | No - needs a queue; game RPC is synchronous HTTP | GA (KEDA core) | Not standalone-viable; would require the full NATS migration (Option 5) just for a scaling signal |
| 2. KEDA HTTP add-on | ~100-150Mi (interceptor + scaler + operator) | Yes - traffic is already plain in-cluster HTTP, one Service per game version, zero worker code changes | Beta/experimental, long-standing (through 2025-2026) | Chosen (owner decision 2026-07-16) - see verdict |
| 3. Knative Serving | ~300-500Mi (activator + autoscaler + controller + webhook) = 15-25% of one node's memory before any workload | Yes, architecturally (activator holds requests) | GA but heavy operational surface (CRDs, webhook, net-* ingress plugins) | Overkill for this problem size |
| 4. Bespoke shim in existing operator | ~0 extra platform memory (reuses existing operator process/service account) | Yes | Owned code, narrow RBAC | Third choice |
| 5. NATS-event-driven wake-up | N/A (see below) | No | N/A | Does not fit; documented cost below |
| 6. KEDA core + metrics-api/external scaler | ~70-160Mi (operator + metrics adapter) | Yes - web exposes a per-version demand signal, retry-with-backoff buffers the request | GA (KEDA core, CNCF graduated) | Second choice - see core-KEDA section below |

### Option 2 detail - Maturity (verified 2026-07)
- KEDA core graduated CNCF on 2023-08-22 (CNCF, 2023-08). The HTTP add-on is a separate community sub-project, NOT covered by that graduation, and is still labeled beta - its README states "We can't yet recommend it for production usage" and points to Kedify's commercial HTTP scaler as the supported alternative (kedacore/http-add-on, 2026-07).
- Health: latest release v0.15.0 (2026-06-15), repo active (last commit 2026-07-13), 541 stars / 165 forks, 16 open issues - active but small (kedacore/http-add-on, 2026-07).
- Adoption: thin public evidence; the main documented production case is Choreo/WSO2 (4000+ services, scale-to-zero on low-traffic services, ~7-node reduction) (Choreo/WSO2, 2026). Otherwise tutorial-level content - niche, not mainstream.
- Activation latency: the interceptor proxies 100% of traffic (~1-5ms steady-state overhead); the KEDA scaler's default 15s polling interval adds a floor before a 0->1 trigger fires, on top of pod start time; open issue #219 acknowledges cold-start notification latency as unresolved, and issue #1443 discusses adding Envoy for hot paths (kedacore/http-add-on issues, 2026-07). With brdgme's tiny images, expect low-end latency but budget ~2-20s worst case for a cold 0->1 including the polling floor.
- Keep-warm: `HTTPScaledObject.spec.scaledownPeriod` is the number of seconds after the last active request before scaling to 0 (examples commonly use 300s; the exact default is unconfirmed - verify against the CRD schema of whatever add-on version gets pinned). Recently-played game versions stay at 1 replica for this window, so popular ones effectively stay warm.

### Core KEDA instead of the HTTP add-on (verified 2026-07)
- **How KEDA is normally used:** core KEDA graduated CNCF on 2023-08-22, with 45+ organizations running it in production (FedEx, Grafana Labs, Reddit, Xbox) and 60+ built-in scalers. The dominant patterns are queue/stream lag (Kafka, RabbitMQ, SQS, Azure Service Bus) and the Prometheus scaler on custom metrics; cron is used for predictable schedules. All of these rely on a durable, externally observable backlog signal (keda.sh docs / CNCF announcement, 2023-2026).
- **The fundamental catch:** core KEDA never sits in the data path or buffers requests. With `minReplicaCount: 0` the Service has no endpoints, so a synchronous request just fails outright. Activation is driven by `IsActive` on each `pollingInterval` (default 30s), with `cooldownPeriod` default 300s before scaling back to zero (keda.sh ScaledObject spec, 2026). The HTTP add-on's interceptor exists precisely to hold the request while a scale-up happens - core KEDA has no equivalent.
- **Verified brdgme plumbing:** web already exposes Prometheus metrics via `axum-prometheus` on `:9090/metrics` (`rust/web/src/main.rs`); this is a low-cardinality HTTP layer with no per-game-version counter today. Alloy scrapes annotated pods and remote-writes to Grafana Cloud's hosted Prometheus (`k8s/prod/alloy/configmap.yaml`, no scrape-interval override so ~60s default). There is no in-cluster Prometheus and no KEDA installed today.
- **Core-KEDA variants assessed:**

| Variant | Assessment |
|---|---|
| a. Prometheus scaler, min=1 | Safe but no scale-to-zero - keeps the 32Mi floor; does not meet the goal. |
| b. Prometheus scaler, min=0, via Grafana Cloud | Activation chain = ~60s Alloy scrape + remote-write lag + KEDA poll + pod start = 30-120s+ worst case; also couples scale-up to Grafana Cloud availability. Not viable for synchronous submit-move. |
| b'. In-cluster Prometheus to fix the lag | +200-500Mi on a memory-tight cluster - defeats the purpose. |
| c. metrics-api or external/external-push scaler, min=0 | Web tracks per-version demand (in-flight/recent requests) and exposes a tiny endpoint per version; KEDA polls it directly (`pollingInterval` settable 1-5s) or `external-push` removes the poll wait entirely. Latency = seconds + pod start. No Prometheus/Grafana Cloud in the loop. Strongest core-KEDA candidate (keda.sh metrics-api and external-scalers docs, 2026). |
| d. cron scaler | No predictable windows for game traffic - irrelevant. |

- **Honest convergence note:** web needs retry-with-backoff and per-version demand tracking under BOTH the metrics-api path and the bespoke shim - the only difference is whether "watch demand -> patch replicas" lives in KEDA's battle-tested controller (cost: KEDA install ~70-160Mi, 39 ScaledObjects, CRDs) or in owned operator code (cost: own the scale logic + RBAC). The HTTP add-on is the only option that avoids web-side demand code, at the price of beta software proxying every request.

### Option 4 detail (bespoke shim)
On connection-refused/timeout, patch Deployment 0->1, retry HTTP with backoff until Service endpoints are ready (existing tcpSocket probe gates readiness). Scale-down: annotate Deployment with last-request timestamp, sweep from the existing operator process for idle > N minutes. RBAC: grant the operator (not web) patch rights on Deployments. Concurrent 0->1 patches are idempotent/harmless; the idle sweeper must check recent activity to avoid killing a pod mid-request (a risk shared by every option here). Estimate: a few days of effort, mostly retry/backoff plus idle-timer correctness. Web client needs retry-with-backoff added regardless (it has none today).

### Option 5 detail - NATS-event-driven wake-up (first-class treatment)
- **Core NATS request-reply:** no persistence - a request published while the worker is scaled to zero is simply lost. Only fixable by caller retry plus an activator subscribed to the subjects to trigger scale-up - functionally identical to Option 4 with NATS as the trigger transport instead of the HTTP error itself. Adds a hop, no correctness gain.
- **JetStream:** persistence closes the request-side race (the message waits; a KEDA JetStream scaler can wake on pending count). But the reply path for a synchronous caller is awkward: replying over core NATS reintroduces the race on the return leg, or a reply-stream + correlation IDs + subscribe loop in web adds real latency/complexity versus today's single `post().await`.
- **Migration cost:** every game binary's entrypoint (`rust/lib/cmd/src/http.rs`) needs a new NATS-consumer mode; `rust/web/src/game/client.rs` gets rewritten to publish/await-reply; a new subject scheme (e.g. `game.<name>.<version>.request`) has to stay in sync with the operator's `game_versions.uri` registration. This is net-new surface across ~22 binaries for zero functional gain over Options 2/4.
- **Why it doesn't fit:** the existing NATS usage (bot turns) is a genuine async multi-actor problem. Game RPC is single-caller, synchronous, request/response - it never needed a bus. Recommendation is to state this cost explicitly rather than adopt NATS for this purpose merely because it's already deployed.

### Cross-cutting for all scale-to-zero options
Keep the *latest* version per game warm (minReplicas 1, or exclude it from scaling) - it's the one live players hit continuously. The ~17 non-latest deployments are the real scale-to-zero candidates: ~544Mi requests freed (estimate), ~1.1Gi limits freed. Tiny images from Q1 make cache-hit cold starts sub-2s regardless of which mechanism is chosen.

**Verdict, in order (owner decision 2026-07-16):**
1. **KEDA HTTP add-on, pinned v0.15.x** - chosen by Michael. Rationale (recorded verbatim-ish): he prefers an officially-documented, actively-maintained upstream component over hand-rolled activation semantics; the metrics-api path requires bespoke demand-tracking/endpoint code in web, which is exactly what he wants to avoid. Plan: PoC one non-latest game version behind the interceptor first, generous `scaledownPeriod` (300s+) to start, mitigations from the risk assessment below retained (pin release, confined blast radius, rollback plan). The interceptor only proxies non-latest-version hosts - latest games keep the direct Service path, never routed through the interceptor, so the beta component's blast radius is confined to already-idle old versions. Honest beta caveat unchanged: the add-on's own README states "We can't yet recommend it for production usage" (kedacore/http-add-on, 2026-07) - this is a deliberate, informed tradeoff, not a gap in the research.
2. **KEDA core + metrics-api/external scaler** - viable, battle-hardened KEDA core with no beta component, but requires bespoke demand-tracking code in web (the per-version demand endpoint) - exactly the owned-code surface the owner wants to avoid by picking the add-on instead.
3. **Bespoke operator shim** - same web-side work as (2), fewer moving parts (no KEDA install/CRDs), but fully owned scale logic end-to-end.

Ruled out: Prometheus-via-Grafana-Cloud (too slow), in-cluster Prometheus (footprint), cron (no fit), NATS redesign (section above).

## 4. Recommended Path

Sequencing:

1. **Base image swap** to `distroless/cc-debian12` + non-root USER across all 22 game Dockerfile stages. Low risk, zero Rust build changes, -64% image size, helps GHCR pull pressure and cold-start latency. musl+scratch stays optional/later.
2. **Add retry-with-backoff** to web's game client (`rust/web/src/game/client.rs`). Prerequisite for any scale-to-zero option, and an improvement to today's resilience regardless (currently zero retries).
3. **Scale-to-zero for non-latest versions**: KEDA HTTP add-on, pinned v0.15.x (owner decision 2026-07-16; core-KEDA metrics-api and the bespoke shim remain documented fallbacks if the add-on proves unstable). Latest version per game stays on the direct Service path (never routed through the interceptor).

Combined resource picture (estimates):

| State | Games memory requests | Platform overhead | Notes |
|---|---|---|---|
| Current | ~1.25Gi (39 deployments x 32Mi, 1 replica each) | ~0 (NATS unrelated to this path) | Measured |
| After step 3 | ~0.7Gi (22 warm "latest" deployments x 32Mi) | +100-150Mi (HTTP add-on: interceptor + scaler + operator) | Net ~350-400Mi requests freed |

The bigger structural win is not the one-time freed memory: today every new game version adds a *permanent* 32Mi reservation forever (old versions never scale down). After step 3, marginal reservation cost per new version drops to zero at steady state (only the latest version per game stays warm), so version churn and future game ports stop accumulating reservation indefinitely.

Note: a separate Alloy assessment (trim the traces pipeline, backlog #41b/#32) does NOT gate this work - the chosen HTTP add-on path also has no Prometheus/Grafana dependency (the interceptor and its scaler operate entirely on request traffic, independent of Alloy/Grafana Cloud).

### Risk assessment (bottom line)
- **Low-risk/mainstream:** the `distroless/cc-debian12` swap - proceed with normal confidence (pin by digest).
- **The one risky bet is the beta add-on, again:** the HTTP add-on's own README still says "we can't yet recommend it for production usage." This is unchanged from the earlier analysis - what changed is that Michael weighed this explicitly against the cost of owning demand-tracking/activation code in web, and chose the upstream component anyway (recorded decision, 2026-07-16), not an oversight.
- **Mitigations (retained):** pin the release (v0.15.x, digest where possible); PoC on one non-latest game version before fleet rollout; generous `scaledownPeriod` (300s+ to start) so recently-played versions stay warm; blast radius confined to non-latest versions only - latest games always use the direct Service path, never the interceptor; keep core-KEDA metrics-api and the bespoke operator shim (Options 4/6) as documented fallbacks if the add-on proves unstable.
- **Bottom line:** `distroless/cc` swap is within mainstream practice - proceed with normal confidence. The HTTP add-on carries genuine beta risk, confined by design to non-latest versions, with a PoC gate and rollback plan before fleet rollout; this is a deliberate, recorded owner tradeoff (official upstream component vs. owned activation code), not an unmitigated risk. Platform overhead ~100-150Mi (interceptor + scaler + operator).

## 5. Open Questions for Michael

- Acceptable worst-case latency for a move against an old (non-latest) version? Cold start is ~0.5-2s on a cache-hit node (estimate), but up to ~5-10s on an image pull miss (estimate).
- PoC acceptance gate (see plan): what cold-start latency threshold and stability window (N days, no interceptor errors/OOMs) must the one-version PoC clear before fleet rollout of the HTTP add-on?
- Verify `scaledownPeriod` tuning (HTTPScaledObject field) on the chosen path during the PoC.
- Move games from port 80 to 8080 (44 manifest edits) as part of hardening now, or defer as a separate follow-up?
- Any appetite for the musl/scratch second step (further ~8MiB/image, estimate), or is `distroless/cc` sufficient?
- Should the NATS-based design (Option 5) be dropped per this analysis, or is architectural consistency with the bot pipeline valued enough to justify its migration cost anyway?
