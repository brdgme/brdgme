# 42: Image Size Reduction and Scale-to-Zero Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut game-worker image size (~64%, zero build changes) via distroless base images, add retry-with-backoff resilience to web's game client, then scale non-latest game-version deployments to zero via core KEDA with a metrics-api/external scaler, backed by a web-exposed per-version demand signal.

**Spec:** docs/superpowers/specs/2026-07-16-42-image-scale-to-zero-viability.md

**Status:** Approved direction, including owner decision 2026-07-16: Phase 3 uses the KEDA HTTP add-on (pinned v0.15.x) as the primary scale-to-zero mechanism for non-latest game versions, with core-KEDA metrics-api and a bespoke operator shim as documented fallbacks. Phase 3 now has a PoC acceptance gate (not a KEDA-vs-shim decision gate) before fleet rollout.

**Architecture:** Phase 1 swaps the `FROM debian:bookworm-slim` runtime stage in `rust/Dockerfile` to a digest-pinned `gcr.io/distroless/cc-debian12`, adds a non-root numeric `USER`, and moves the listen port from 80 to 8080 across the ~44 k8s manifest files under `k8s/base/game/*/` (non-root cannot bind port 80 without `CAP_NET_BIND_SERVICE`). Phase 2 adds bounded exponential-backoff retries to `rust/web/src/game/client.rs`. Phase 3 is now HTTP add-on based: installs KEDA core + the HTTP add-on (pinned), creates an `HTTPScaledObject` per non-latest game-version Deployment, and re-wires non-latest traffic through the add-on's interceptor proxy Service so idle versions scale to zero with zero worker code changes; latest versions keep the direct Service path. Phase 4 measures results and decides on further follow-ups (musl/scratch, port-80 cleanup).

**Tech Stack:** Rust 2024 (existing game/web binaries, no new Rust dependencies for Phase 1-2), Docker/`docker-bake.hcl` (unchanged structure, base image only), Kubernetes/Kustomize (DOKS), KEDA core (Phase 3, pinned version).

## Global Constraints

- **Non-goals (explicit):**
  - No NATS rearchitecture of game RPC (spec Option 5: doesn't fit brdgme's synchronous single-caller RPC pattern; net-new surface across ~22 binaries for no functional gain).
  - No bespoke demand-tracking/metrics-api scaler and no operator-shim activation logic (owner decision 2026-07-16: prefer official upstream component; alternatives documented in the spec as fallbacks if the add-on proves unstable).
  - No musl/scratch images in this plan (later option only, if GHCR pull pressure persists after Phase 1; needs a musl cross-toolchain added to the shared chef/builder stage that also serves the web build - separate risk).
  - No in-cluster Prometheus (spec: +200-500Mi on a memory-tight cluster, defeats the purpose of freeing memory).
  - Alloy/traces-pipeline changes are handled separately under backlog #41b (and #32) and do NOT gate this plan - the chosen KEDA metrics-api path (variant c) has no Prometheus/Grafana Cloud dependency in its activation path.
- Base image pin: `gcr.io/distroless/cc-debian12@sha256:<digest>` - pin by digest, not tag, per spec (GCR read shutdown doesn't affect Artifact Registry-served `gcr.io/distroless/*`, but digest pinning is still the reproducible-build norm here).
- `docker-bake.hcl` is unchanged (only the runtime `FROM` line and `USER` in `rust/Dockerfile` change; the 26-target matrix and cargo-chef builder stage are untouched).
- CMD is already exec-form and probes are already `tcpSocket` (verified in the spec) - Phase 1 only needs to re-point the port, not the probe mechanism.
- Non-root numeric USER: use `USER 65532` (the conventional distroless nonroot UID) - no shell exists in the distroless image to create a named user, so numeric UID is required regardless.
- Run only ONE cargo build/test at a time. Docker builds via `docker buildx bake` as normal.
- Manifests live under `k8s/base/game/<game>/` (`deployment.yaml` + `service.yaml`); brdgme-config (a separate repo/dir per prior commits, e.g. `/home/beefsack/Development/brdgme-config`) holds the per-env rollout config referenced for canarying.
- Games memory requests/limits (32Mi/10m requests, 64Mi limits) are unchanged by Phase 1/2 - image size reduction does not reduce container RSS (spec's explicit honesty note).

---

### Phase 1: Distroless image swap + non-root + port 8080

**Files:**
- Modify: `/home/beefsack/Development/brdgme/rust/Dockerfile` (all 22 game runtime stages)
- Modify: `k8s/base/game/<game>/deployment.yaml` and `service.yaml` for all 22 games (~44 files: containerPort, tcpSocket probe port, `ADDR` env, Service targetPort)
- No change: `rust/docker-bake.hcl`

**Interfaces:**
- Consumes: existing `ADDR` env var support in each game binary's entrypoint (`rust/lib/cmd/src/http.rs`, defaults `0.0.0.0:80`); existing exec-form CMD and tcpSocket probes.
- Produces: each of the 22 game images runs as `gcr.io/distroless/cc-debian12@sha256:<digest>`, non-root `USER 65532`, listening on `0.0.0.0:8080` (set via `ADDR` env in the Deployment), with matching containerPort/probe/Service targetPort of 8080.

- [x] **Step 1: Resolve and record the distroless digest**

Run: `docker pull gcr.io/distroless/cc-debian12 && docker inspect --format='{{index .RepoDigests 0}}' gcr.io/distroless/cc-debian12`

Record the resulting `gcr.io/distroless/cc-debian12@sha256:...` digest for use in the Dockerfile.

- [x] **Step 2: Swap the runtime base image and add non-root USER in rust/Dockerfile**

For each of the 22 game runtime stages, replace:

```dockerfile
FROM debian:bookworm-slim
```

with:

```dockerfile
FROM gcr.io/distroless/cc-debian12@sha256:<digest>
```

Add `USER 65532` after the binary `COPY` and before the exec-form `CMD` in each stage. Remove the stale GLIBC_2.38 builder/runtime-drift comment if present (distroless/cc uses the same glibc family as the chef builder - the drift class this comment warns about no longer applies once base and builder track compatible glibc versions; verify this at build time in Step 4, don't just delete on faith - keep the comment if the builder/base glibc versions still diverge).

- [x] **Step 3: Update ADDR and port wiring in the k8s manifests**

Across `k8s/base/game/<game>/deployment.yaml` (all ~22 games):
- Set/confirm `ADDR=0.0.0.0:8080` env var.
- Change `containerPort` from 80 to 8080.
- Change the `tcpSocket.port` in readiness/liveness probes from 80 to 8080.

Across `k8s/base/game/<game>/service.yaml`:
- Change `targetPort` from 80 to 8080 (the Service's external `port` can stay 80 if other manifests reference it by that port - confirm by grepping for the Service port consumers before deciding whether to renumber both).

Grep for any other port-80 references before considering this step done:

```bash
grep -rln ":80\b\|port: 80\|containerPort: 80" k8s/base/game/
```

- [x] **Step 4: Build and verify locally**

Run: `docker buildx bake <one or two game targets>` (pick two games with different characteristics, e.g. one simple and one with more dependencies).

Run the built image locally, confirm:
- Non-root: `docker inspect --format='{{.Config.User}}' <image>` shows `65532`.
- Listens on 8080: `docker run -p 8080:8080 <image>` then `curl` or POST a request against it and confirm a valid game response.
- No shell present (expected) - if debugging is needed, use `kubectl debug` ephemeral containers against a running pod instead of `docker exec`.

- [x] **Step 5: Canary rollout** (done 2026-07-16: canary tic-tac-toe-2 on sha-7f65580 via brdgme-config 3f81c60; fleet rollout awaits soak)

Roll out one game's new image + manifests via brdgme-config's canary/staging path first. Confirm the pod starts, passes probes, and serves a real request end-to-end (submit a move against that game in a test game). Only after this passes, roll out the remaining 21 games.

Rollback plan: revert the image tag/digest and the manifest port/ADDR changes for the affected game(s) via the same brdgme-config path; no data-layer changes are involved so rollback is a plain manifest revert.

- [x] **Step 6: Fleet rollout** (done 2026-07-16: two-wave rollout. Wave 1: brdgme-config commit 3f34945 (by owner) pinned ref to 7f65580 and all newTags to sha-7f65580; all Rust-built images (all -2 games, acquire-1, lost-cities-1, web, bot, operator, migrate) rolled out clean. Gap found: the 17 Go-built -1 games went ImagePullBackOff - no sha-7f65580 image existed for them because CI path gating never built Go game images on that commit; no outage, old sha-90b0764 pods kept serving. Forward fix: brdgme master 328bd3a added a stdlib HTTP-on-ADDR server (new cmd.Serve HTTP wrapper, default 0.0.0.0:8080, replacing the webify base) plus distroless base to the 17 Go games (CI green); brdgme master cebcc12 fixed their k8s manifests (8080/ADDR/probes/targetPort, same pattern as 77c81cb). Wave 2: owner committed/pushed the final brdgme-config deploy (ref -> cebcc120d602c9aaaced54c5b4e9c8adcc6ea9ad, the 17 Go -1 newTags -> sha-328bd3a); ArgoCD sync successful. Verification evidence (2026-07-16, single read-only kubectl pass): all game pods in namespace brdgme 1/1 Running, 0 restarts, no ImagePullBackOff/CrashLoopBackOff; all 17 Go -1 games on sha-328bd3a; Rust-built pods (including acquire-1, lost-cities-1) on sha-7f65580 as expected; no sha-90b0764 pods remain; spot-checks of splendor-1/greed-1/category-5-1 confirmed image tag, containerPort 8080 and ADDR=0.0.0.0:8080 (UID 65532 comes from the image's USER directive, no explicit k8s securityContext - not independently verifiable from spec alone); node memory 74% and 81%, both under the 90% threshold. Known caveat: pre-existing vet errors in libcard/deck_test.go will fail `go test` if the Go games' Dockerfile Go version (currently 1.17.1) is ever bumped, since newer Go runs vet as part of test.)

Once canary is confirmed healthy for a full day (covering typical traffic patterns), roll out the remaining games' Dockerfile stages + manifests via brdgme-config to the full fleet.

---

### Phase 2: Web game-client retry-with-backoff

**Files:**
- Modify: `rust/web/src/game/client.rs`

**Interfaces:**
- Consumes: existing `reqwest` client with `connect_timeout` 5s / `timeout` 10s set in `rust/web/src/main.rs`; existing synchronous call path used during a user's submit-move request.
- Produces: bounded retries with exponential backoff + jitter on connect errors and timeouts, within an overall deadline compatible with the user-facing submit-move request (must not make a move submission hang far longer than today's single-attempt 10s timeout budget - size the retry loop's total deadline accordingly, e.g. a small number of attempts with capped backoff so the worst case stays in the few-seconds-to-low-tens-of-seconds range, not open-ended).

Note: game requests are stateless computations (JSON in, JSON out, no DB/side effects per the spec's current-state section) - retrying a failed or timed-out request is safe; there is no double-apply risk.

This phase is valuable standalone (today's client has zero retry logic per the spec) and is also the named prerequisite for Phase 3's scale-to-zero (a cold 0->1 activation needs the caller to tolerate the activation window; this also covers transient interceptor hiccups once Phase 3's HTTP add-on is in the request path for non-latest versions).

- [x] **Step 1: Write failing unit tests for the retry policy**

Add unit tests in `rust/web/src/game/client.rs` covering: retries on connect-refused/timeout errors, does not retry on a valid non-2xx game response (an actual game-logic error, not a transport error), respects a max-attempts/max-elapsed bound, and backoff intervals grow (with jitter) rather than being fixed.

- [x] **Step 2: Implement retry-with-backoff**

Wrap the existing request call with a bounded retry loop (e.g. 2-3 retries, exponential backoff with jitter, capped total elapsed time). Keep the existing per-attempt `connect_timeout`/`timeout` values from `rust/web/src/main.rs` unchanged; the retry loop governs attempts, not per-attempt timeouts.

- [x] **Step 3: Run tests**

Run: `SQLX_OFFLINE=true cargo test -p web --features ssr --lib game::client -j 2`
Expected: PASS.

- [x] **Step 4: Manual verification against a scaled-to-zero worker** (closed 2026-07-17 by the Phase 3 PoC: tic-tac-toe-2 served live web traffic through the interceptor, scaled to 0 after the idle window, and a real game request through web triggered a 0->1 cold start that succeeded end-to-end - the retry/activation path was exercised against a genuine 0-replica target. Earlier partial verification 2026-07-16: no unauthenticated web trigger found quickly against a leptos SSR route tree, so ran the reduced test instead - scaled tic-tac-toe-2 to 0, confirmed pod/endpoints fully removed, restored replicas=1, confirmed Ready and a direct POST against the Service works again. Owner separately confirmed manual end-to-end play through the web frontend post-deploy works. Did not directly observe web's retry/backoff logs against a live 0-endpoint target - that would need either an authenticated game session or a scratch debug pod in-cluster, the latter declined as out of scope for this pass)

Manually scale one game deployment to 0 (`kubectl scale deployment/<game> --replicas=0`), submit a move in a test game against that version, confirm the web request either recovers once the deployment is manually scaled back up within the retry window, or fails gracefully with a clear error rather than hanging indefinitely. Scale the deployment back to 1 afterward.

---

### Phase 3: Scale-to-zero for non-latest game versions via the KEDA HTTP add-on

**Owner decision (2026-07-16):** the KEDA HTTP add-on (pinned v0.15.x) is the chosen mechanism, not a decision-gated choice. Michael prefers an officially-documented, actively-maintained upstream component over hand-rolled activation semantics; the metrics-api path requires bespoke demand-tracking/endpoint code in web, which is exactly what he wants to avoid. Risk is confined to non-latest versions - latest games are never routed through the interceptor and keep the direct Service path. Core-KEDA metrics-api and the bespoke operator shim (spec Options 4/6) remain documented fallbacks only, to be used if the add-on proves unstable during the PoC below - they are not built in this plan.

**Files:**
- New: KEDA core + HTTP add-on install manifests (Helm chart or raw manifests, both pinned; add-on v0.15.x)
- New: `HTTPScaledObject` manifest for each non-latest game-version Deployment
- Modify: the mechanism that registers `game_versions.uri` for non-latest versions (operator, `rust/operator/src/controller.rs`) OR `rust/web/src/game/client.rs` (Host header override) - see routing step below for which

**Interfaces:**
- Consumes: `game_versions.uri` / `find_latest_non_deprecated_game_version` in `rust/web/src/db.rs` (distinguishes latest vs. non-latest versions); Phase 2's retry-with-backoff (buffers requests during a 0->1 activation window, including the add-on's own polling-floor latency).
- Produces: KEDA core + HTTP add-on installed (pinned; estimated ~100-150Mi combined interceptor + external-scaler + operator footprint); an `HTTPScaledObject` per non-latest game-version Deployment with `scaleTargetRef` pointing at that Deployment/Service, `spec.hosts` matching a per-version host value, implicit `minReplicaCount: 0`, and `scaledownPeriod` 300s+; non-latest traffic routed through the interceptor proxy Service instead of directly at the game Service.

- [x] **Step 1: Install KEDA core + HTTP add-on** (done 2026-07-16/17: core v2.18.3 + add-on v0.15.0, applied manually with `kubectl apply --server-side --force-conflicts -k` per brdgme-config `keda/README.md`; all 6 deployments healthy; interceptor proxy Service confirmed at `keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080`)

Install KEDA core, then the HTTP add-on Helm chart pinned to v0.15.x (spec estimates ~100-150Mi combined interceptor + scaler + operator overhead). Verify CRDs install cleanly (`HTTPScaledObject`) and the interceptor, external-scaler, and operator pods come up healthy. Confirm the interceptor's proxy Service exists (default `keda-add-ons-http-interceptor-proxy` in the `keda` namespace, port 8080 per the 0.15 install defaults - confirm exact name/namespace/port against the actual Helm release values used, since chart defaults can be overridden) (keda.sh http-add-on 0.15 docs, 2026).

- [ ] **Step 2: Create HTTPScaledObjects for non-latest versions**

For each non-latest game-version Deployment (spec: ~17 candidates today), create an `HTTPScaledObject` with:
- `scaleTargetRef` naming that Deployment's Service and port
- `spec.hosts`: a distinct host value per game version (e.g. `<game>-<version>.games.internal` or similar internal-only convention - does not need to be a real DNS name, only a value the interceptor matches on)
- `minReplicaCount` left at the add-on's implicit 0 (scale-to-zero)
- `scaledownPeriod`: 300s+ to start (tune during the PoC - this is the number of seconds after the last active request before scaling to 0; recently-played versions stay warm through this window) (keda.sh http-add-on 0.15 docs, 2026)

Exclude every game's latest version entirely - no `HTTPScaledObject`, direct Service path only, never proxied through the interceptor.

- [x] **Step 3: Wire routing - non-latest requests must flow through the interceptor with the matching Host** (done 2026-07-17: option (b) implemented - web sends `Host: {game_versions.name}.games.internal` on every game server request as of brdgme `c9c5d94`, deployed as web `sha-6ec07fa`; per-version host derived from the name, no schema change; routing for a given version is flipped purely by its `game_versions.uri` value)

Verified against the 0.15 docs (keda.sh http-add-on 0.15 docs, 2026): the interceptor matches incoming requests against each `HTTPScaledObject`'s `spec.hosts` using the HTTP `Host` header (not the URL path by default - `pathPrefixes` is a separate, additional match dimension this plan does not need). Traffic must physically be sent to the interceptor's proxy Service address, with the `Host` header set to the value declared in that version's `spec.hosts`. These are two different things: the interceptor's own Service DNS name (e.g. `keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080`) is where the TCP connection goes; the per-version `spec.hosts` value is a separate Host-header match key, not required to be a real routable name.

Concrete brdgme wiring: the game URI web actually calls comes from `game_versions.uri` in Postgres, registered by the operator (`rust/operator/src/controller.rs`). Two options, pick one based on what's simpler to implement:
  - **(a) URI swap only, no web code change** - only works if the URI's host portion can simultaneously (i) resolve/route to the interceptor's proxy Service and (ii) be usable as the `Host` header value matched by `spec.hosts`. This is NOT generally true: reqwest sends the `Host` header derived from the request URL's authority, and the interceptor's proxy Service DNS name is a fixed cluster-internal name (e.g. `keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local`) - it is not itself a per-version match key. Setting `game_versions.uri` to the interceptor's DNS name alone would send that same fixed Host header for every non-latest version, which the interceptor cannot use to distinguish versions.
  - **(b) URI swap + explicit Host header override (this is the one that works)** - register `game_versions.uri` for non-latest versions as `http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080` (connection target, same for all non-latest versions) plus a per-version Host-match value (e.g. store it in a new column, or derive it deterministically from game+version so no schema change is needed - decide during implementation). Web's HTTP client (`rust/web/src/game/client.rs`) needs a small code change: when calling a non-latest version, set an explicit `Host` request header to that version's `spec.hosts` value (reqwest supports overriding the `Host` header explicitly; this does not change the connection target, only the header sent). **Do NOT assume (a) is sufficient - confirm during Step 1/2 implementation that (b)'s explicit Host header override is required, per the docs finding above.**

Latest versions: `game_versions.uri` keeps pointing directly at that game's Service, unchanged, never through the interceptor.

**PoC record (Michael, 2026-07-17):**

- **Target:** tic-tac-toe-2 - the LATEST tic-tac-toe version, deliberately deviating from the "non-latest only" framing above for the PoC (it is the only tic-tac-toe version in `game_versions`).
- **Artifacts (committed and deployed 2026-07-17):** `HTTPScaledObject` at `k8s/base/game/tic-tac-toe-2/http-scaled-object.yaml` (host `tic-tac-toe-2.games.internal`, `scaleTargetRef` Deployment/Service `tic-tac-toe-2` port 80, replicas min 0 max 1, `scaledownPeriod` 300), wired into that game's `kustomization.yaml` - brdgme commit `093918f`; brdgme-config `prod/kustomization.yaml` bumped to web `sha-6ec07fa` / ref `093918f4b12a96d02636cf5556b58b3bab1c3693`, synced by ArgoCD.
- **Web client Host header:** web sends `Host: {game_versions.name}.games.internal` on every game server request as of brdgme commit `c9c5d94`.
- **`game_versions` row:** id `076f4633-ebf5-43da-bcd6-34c12eef6654`, name `tic-tac-toe-2`, current (old) uri `http://tic-tac-toe-2.brdgme.svc.cluster.local`.
- **Cutover SQL:**
  ```sql
  UPDATE game_versions SET uri = 'http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080' WHERE id = '076f4633-ebf5-43da-bcd6-34c12eef6654';
  ```
- **Revert SQL:**
  ```sql
  UPDATE game_versions SET uri = 'http://tic-tac-toe-2.brdgme.svc.cluster.local' WHERE id = '076f4633-ebf5-43da-bcd6-34c12eef6654';
  ```
- **SQL access path:** `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec -n brdgme postgres-1 -c postgres -- psql -d brdgme -c "..."`
- **Cutover executed:** 2026-07-17 by Michael (`UPDATE 1`). **Verification (2026-07-17):** HTTPScaledObject Ready; new game created and played through beta.brdg.me via the interceptor path (working); deployment scaled 1->0 at 04:16:12Z (~300s after last activity); next UI request triggered 0->1 at 04:17:22Z, pod Ready at 04:17:27Z (~5s cold start; ~7s click-to-UI-response observed by Michael); no interceptor/scaler errors. Day-1 gate evidence - the multi-day stability window of Step 4 continues from here.

- [ ] **Step 4: PoC acceptance gate on one non-latest version**

Before any fleet rollout: put exactly one non-latest game-version Deployment behind the interceptor (Steps 2-3 for that one version only). Submit moves against it and measure cold-start (0-replica) latency against the budget from the open questions below. Run it for several days under normal idle/active cycling and confirm: no interceptor errors, no interceptor/scaler/operator OOMs, the deployment reliably scales to 0 after `scaledownPeriod` and reliably reactivates on the next request, and the corresponding latest-version deployment never gets routed through the interceptor during this window. Only proceed to fleet rollout once this gate passes.

Rollback (if the PoC fails or fleet rollout needs to be undone): point the affected `game_versions.uri` rows back at the direct Service, remove the `HTTPScaledObject`(s), and uninstall the add-on/KEDA core if abandoning the approach entirely. No data-layer changes beyond the `uri` column, so rollback is a plain revert.

- [ ] **Step 5: Fleet rollout**

Once the PoC gate passes, create `HTTPScaledObject`s and the corresponding `game_versions.uri` + Host-header wiring for the remaining non-latest versions, and roll out via brdgme-config.

---

### Phase 4: Measure and iterate

**Depends on:** metrics-server availability (backlog #41a) for accurate cluster-level memory measurement.

**Files:** none expected (measurement + a written decision, not code changes).

- [ ] **Step 1: Measure freed memory**

Compare actual freed memory requests against the spec's baseline (~1.25Gi current total, ~544Mi estimated freed from ~17 non-latest deployments scaling to 0, netting to roughly ~0.7Gi warm "latest" deployments + KEDA's ~70-160Mi overhead). Record actual vs. estimated.

- [ ] **Step 2: Measure GHCR pull behavior and cold-start latency**

Record image pull times/cache-hit rates post-distroless-swap, and cold-start latencies for 0->1 activations under the HTTP add-on (Phase 3), across a representative sample of non-latest versions. Check whether the interceptor exposes its own metrics (request counts, activation latency) that are cheaper to read than end-to-end timing from web.

- [ ] **Step 3: Decide on follow-ups**

Based on measured data, decide whether to pursue:
- The musl/scratch second image-size step (spec: further ~8MiB/image estimate) - only if GHCR pull pressure still matters after the distroless swap.
- Cleaning up any remaining port-80 references or hardening left over from Phase 1.

---

## Open Questions

- **Cold-start latency budget** (Michael): acceptable worst-case latency for a move against an old (non-latest) version. Spec estimate: ~0.5-2s cache-hit, ~5-10s on an image-pull miss.
- **PoC acceptance criteria** (Michael, gate before Phase 3 fleet rollout): what cold-start latency threshold and stability window (N days, zero interceptor errors/OOMs) must the single-version PoC clear before proceeding to the remaining non-latest versions?
- **`scaledownPeriod` tuning value** (`HTTPScaledObject` field): spec suggests 300s+ as a starting point - confirm/tune during the Phase 3 PoC.
- **Confirm interceptor Host-routing wiring vs. `game_versions.uri` registration**: this plan's Step 3 concludes an explicit Host-header override in web's client is required (not a URI swap alone) - reconfirm against the exact pinned add-on release's docs/behavior during Phase 3 Step 1-3 implementation before finalizing the schema/code change.
