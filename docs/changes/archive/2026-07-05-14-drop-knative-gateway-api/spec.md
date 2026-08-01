# 14: Drop Knative - Plain Deployments + Gateway API - Design

> Extracted 2026-07-08 from `docs/plan/14-drop-knative-gateway-api.md`
> (superpowers layout migration). Content dates from 2026-07-05; this is a
> point-in-time decision record, not a living document.

**Status:** Dev complete (landed fc7cb3f); cluster version/VPC-native/
GatewayClass prerequisite verified 2026-07-05 (item 21 stage-2); WS
idle-timeout and Knative-cleanup-notes prerequisites resolved 2026-07-05;
client-IP/PROXY-protocol prerequisite researched 2026-07-05 but intentionally
deferred - it needs a live edit to a DOKS-managed ConfigMap with no dry-run
value before a Gateway exists to use it, so it's done at Phase 16 cutover
instead (see prerequisites section in the implementation plan)

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
