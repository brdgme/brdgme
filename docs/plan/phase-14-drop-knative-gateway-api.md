# Phase 14: Drop Knative - Plain Deployments + Gateway API

**Status:** Dev complete (landed fc7cb3f); prod prerequisites pending

**Implementation (2026-07-03):** all delegable items done. Gateway exposure in
Kind uses a NodePort pin on Cilium's per-Gateway LoadBalancer Service (see
the `gateway-nodeport` Tilt resource) plus a `ctlptl.yaml` `extraPortMappings`
entry (host 8080 -> node 31080) - the originally planned `kubectl
port-forward` fallback (`legacy-gateway` resource) turned out to be
unworkable: Cilium's per-Gateway Service has no selector (it programs
endpoints itself, not via backing pods), so `kubectl port-forward` on it can
never connect. `k8s/kind-config.yaml` was folded into the new `ctlptl.yaml`
and deleted. GatewayClass name `cilium` assumed for prod - confirm on DOKS
(Phase 16 prerequisite).

**Verification (2026-07-04):** live Kind verification completed for all three
dev modes - hybrid (default), `LEGACY=1`, and `WEB_IN_CLUSTER=1`. See the
Verification section below for details. Two dev-environment bugs found and
fixed during verification, both orthogonal to the Gateway/Deployment change
itself: the `legacy-gateway` port-forward approach described above, and an
`operator` local_resource startup race against the `postgres` port-forward
(now retries with `pg_isready` before connecting). A third issue was found
and fixed to complete the `WEB_IN_CLUSTER=1` checklist item: the `operator`
local_resource runs as a plain host process and cannot resolve
`*.svc.cluster.local` names needed to reconcile `GameVersion` HTTP calls to
game services - it is now wrapped in `mirrord exec` (same pattern as `web`
and `bot`) via a new `rust/operator/.mirrord/mirrord.json`. Remaining: Prod
prerequisites (operator-verified, see below).

**Decision (2026-07-03):** remove Knative Serving and Kourier entirely.
Knative is healthy as a project (CNCF graduated 2025-09, quarterly releases -
this is not a Rocket-style maintenance risk), but it is a poor fit at brdgme's
scale:

- The Serving control plane requests ~630m CPU / ~400Mi (activator,
  autoscaler, autoscaler-hpa, controller, webhook) plus Kourier plus a
  queue-proxy sidecar in every pod. The workloads it scales to zero - ~20
  idle Go/Rust game services at roughly 5-25Mi RSS each - cost less to just
  run always-on. Scale-to-zero is negative-value at this scale.
- Turn-based ASCII games have no load spikes worth request-based autoscaling.
- DOKS now provides a managed Gateway API implementation on Cilium
  (pre-installed on clusters >= 1.33 with VPC-native networking,
  auto-provisions the DO load balancer, no controller to run). This replaces
  Kourier + DomainMapping with standard `Gateway`/`HTTPRoute` resources.
- Removes dev-environment complexity that exists only for Knative: the local
  registry digest-resolution requirement, the `k8s_kind('Service', ...)` Tilt
  workaround, the Kourier NodePort patch, and the Knative install in
  `setup-kind-cluster.sh`.
- Makes the bot an always-on Deployment, which resolves the Phase 13
  scale-to-zero vs NATS-subscriber conflict.

Alternatives considered: KEDA + HTTP add-on (add-on still beta v0.15.x with
an interceptor proxy in the request path; saves nothing vs always-on here;
KEDA's JetStream scaler remains the right tool if a genuinely heavy
scale-to-zero consumer ever appears, e.g. in-cluster LLM inference); FaaS
frameworks (wrong packaging model). Eventing is unaffected: NATS Core was
already chosen over Knative Eventing.

**Sequencing:** run this phase before Phase 13 (NATS bot eventing), Phase 15
(ArgoCD), and Phase 16 (cutover), so manifests are rewritten once and the
ArgoCD config repo + cutover/rollback procedures are written against the
final infrastructure.

**Current Knative surface (audited 2026-07-03):**
- ksvc manifests: `k8s/base/web/service.yaml` (minScale 1),
  `k8s/base/bot/service.yaml`, and the legacy trio `k8s/base/web-legacy/`,
  `k8s/base/api/`, `k8s/base/websocket/`.
- `k8s/base/domain-mapping/` - 4 DomainMappings with
  `networking.knative.dev/certificate-class: cert-manager.io` annotations.
- `k8s/prod/knative-serving/` - config-domain, config-certmanager,
  config-network patches.
- `k8s/base/cert-manager/cluster-issuer.yaml` - HTTP01 solver bound to the
  Kourier ingress class.
- `scripts/setup-kind-cluster.sh` - Knative Serving + Kourier install, webhook
  waits, Kourier NodePort 31080 patch.
- `Tiltfile` - `k8s_kind('Service', api_version='serving.knative.dev/v1', ...)`,
  mode comments, `*.brdgme.lvh.me:8080` links that route through Kourier.
- Game services are already plain Deployments + NodePort Services - unaffected
  by this phase apart from optional Service-type cleanup.

### Manifests

- [x] `k8s/base/web/`: replace ksvc with a Deployment (target `replicas: 2`
      to match the multi-replica vision; `replicas: 1` acceptable while the
      cluster is small - Redis/NATS fan-out already handles multi-replica)
      plus a ClusterIP Service on port 3000. Preserve the existing env/secret
      wiring.
- [x] `k8s/base/bot/`: replace ksvc with a Deployment (`replicas: 1`,
      always-on) plus a ClusterIP Service on port 4000. Preserve
      `postgres-config`/`bot-config` secret refs and `LISTEN_ADDR`.
- [x] Legacy trio (`web-legacy`, `api`, `websocket`): ksvc → Deployment +
      Service. Temporary manifests - deleted at Phase 16 decommission - so
      minimal effort, no replicas tuning.
- [x] Delete `k8s/base/domain-mapping/`. Create `k8s/base/gateway/`: one
      `Gateway` with an HTTPS listener per hostname (`brdg.me`,
      `legacy.brdg.me`, `api.brdg.me`, `ws.brdg.me`) and one `HTTPRoute` per
      hostname routing to the matching Service. Update
      `k8s/prod/app/kustomization.yaml` (currently includes
      `../../base/domain-mapping`).
- [x] Delete `k8s/prod/knative-serving/` and remove it from
      `k8s/prod/kustomization.yaml`.
- [x] cert-manager for Gateway API: enable the Gateway API feature
      (`config.enableGatewayAPI: true` / `--enable-gateway-api`), annotate the
      `Gateway` with `cert-manager.io/cluster-issuer`, switch the
      ClusterIssuer HTTP01 solver from the Kourier ingress class to
      `gatewayHTTPRoute` solvers referencing the Gateway.
- [x] Optional cleanup: game service Services from NodePort → ClusterIP (only
      the monolith calls them in-cluster).

### Dev environment (Kind)

- [x] `scripts/setup-kind-cluster.sh`: remove the Knative Serving + Kourier
      install blocks, webhook waits, and Kourier NodePort patch. Keep Cilium
      (CNI), the GameVersion CRD, and the Kind cluster/registry logic.
- [x] Enable Cilium Gateway API in Kind: install the Gateway API CRDs and set
      `gatewayAPI.enabled=true` in the Cilium install values. Expose the
      Gateway via NodePort 31080 (already mapped to host 8080 in
      `k8s/kind-config.yaml` `extraPortMappings`) to preserve the
      `{service}.brdgme.lvh.me:8080` dev URLs. If Cilium's Gateway NodePort
      exposure proves awkward in Kind, fall back to Tilt port-forwards and
      update DEV.md accordingly - decide during implementation.
- [x] `Tiltfile`: remove the `k8s_kind('Service', ...)` registration; update
      the mode comments; verify `WEB_IN_CLUSTER=1` and `LEGACY=1` modes
      deploy and route correctly.
- [x] Local registry: no longer mandatory (the digest-resolution requirement
      was Knative's) but kept - it is faster than `kind load`. Replace the
      hand-rolled cluster + registry bootstrap (the docker run / network
      connect / KEP-1755 ConfigMap blocks in `setup-kind-cluster.sh`) with
      ctlptl (Tilt-team tool; decided 2026-07-03): a committed `ctlptl.yaml`
      declares the Kind cluster (default CNI disabled) and `kind-registry`.
      Add `ctlptl` to `devenv.nix`. The script shrinks to: `ctlptl apply` →
      Cilium install (Gateway API enabled) → GameVersion CRD.
- [x] Verify hybrid mode is unaffected (web/bot run as local processes;
      game services and backing services unchanged).

### Prod prerequisites

- [ ] Verify the DOKS cluster is >= Kubernetes 1.33 with VPC-native
      networking so the managed Gateway API (GatewayClass) is available;
      plan a cluster upgrade if not.
- [ ] Confirm behaviour of the auto-provisioned DO load balancer for
      WebSockets: long-lived connection support and idle timeout
      configuration (the monolith holds a WS per connected client).
- [ ] Remove the Knative/net-certmanager one-time `kubectl apply`
      prerequisites from the Phase 16 notes; cert-manager alone remains.

### Verification

- [x] Kind, full-cluster mode (`WEB_IN_CLUSTER=1`): web reachable through the
      Gateway; login, game creation, command flow, and a WebSocket session
      all work through the Gateway route.
- [x] Kind, `LEGACY=1`: legacy trio reachable via their hostnames.
- [ ] Prod TLS issuance (HTTP01 through the Gateway) is verified as part of
      the Phase 16 cutover checklist, not here.

### Docs

- [x] Update `docs/DEV.md`: Kourier/Knative references, lvh.me routing
      explanation, setup script description.
- [x] `docs/VISION.md` and `docs/ARCHITECTURE.md` already reflect the target
      state (updated 2026-07-03).
- [x] Update the operator long-term goal wherever stated: it manages
      Deployment/Service lifecycle per game version, not Knative Services.

