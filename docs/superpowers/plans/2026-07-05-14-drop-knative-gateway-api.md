# 14: Drop Knative - Plain Deployments + Gateway API - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/14-drop-knative-gateway-api.md`. This
> work is complete/closed (remaining unchecked items are intentionally
> deferred to Phase 16); retained as an execution record.
>
> **Update 2026-07-08 (later):** the deferred item (client-IP/PROXY
> protocol) was attempted and dropped the same day - see the prod
> prerequisites section below. All items are now closed; nothing remains
> deferred.

**Status:** Fully done 2026-07-08. Dev complete (landed fc7cb3f); cluster
version/VPC-native/GatewayClass prerequisite verified 2026-07-05 (item 21
stage-2); WS idle-timeout and Knative-cleanup-notes prerequisites resolved
2026-07-05; client-IP/PROXY-protocol prerequisite attempted live 2026-07-08
and dropped - DOKS's managed reconciler owns `cilium-config` and reverts
the flag, so it cannot be set persistently (see prerequisites section
below for detail; decision recorded in `docs/BACKLOG.md` History
2026-07-08). No remaining work; superseded by #28's IP-independent caps +
Cloudflare edge.

**Spec:** `docs/superpowers/specs/2026-07-05-14-drop-knative-gateway-api-design.md`

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
      `ctlptl.yaml` `extraPortMappings`) to preserve the
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

- [x] Verify the DOKS cluster is >= Kubernetes 1.33 with VPC-native
      networking so the managed Gateway API (GatewayClass) is available;
      plan a cluster upgrade if not. Confirmed 2026-07-05 on the `brdgme`
      cluster (stage-2 tofu apply, item 21): version `1.36.0-do.2`,
      `cluster_subnet`/`service_subnet` set and `vpc_uuid` bound
      (VPC-native), `GatewayClass/cilium` present with
      `controller: io.cilium/gateway-controller`, `ACCEPTED: True`. No
      Gateway resource created yet - deferred to Phase 16 (each Gateway
      provisions a $12/mo DO LB; the cluster must only ever have one).
- [x] Confirm behaviour of the auto-provisioned DO load balancer for
      WebSockets: long-lived connection support and idle timeout
      configuration (the monolith holds a WS per connected client).
      Resolved 2026-07-05 (research, no live Gateway exists yet to test
      against): since the Gateway terminates TLS in-cluster (Envoy), the DO
      LB sits in TCP-passthrough mode ahead of it - DO's documented
      "WebSocket gets a 1-hour idle timeout automatically" behaviour is
      specific to their HTTP/HTTPS LB mode and does not apply here. The only
      tunable is `service.beta.kubernetes.io/do-loadbalancer-http-idle-timeout-seconds`
      (default 60s, range 30-600s), which DO's docs describe as governing
      the LB's general idle timeout regardless of mode. `rust/web` already
      sends a server-side WS ping every 30s specifically to defeat LB idle
      timeouts (`rust/web/src/websocket.rs:74-77`) and the client reconnects
      with `ReconnectLimit::Infinite` (`websocket_client.rs:37`), so this was
      already handled by existing code. Set the annotation explicitly to
      120s (margin above the 30s ping) in `k8s/base/gateway/gateway.yaml`
      rather than relying on the 60s default.
- [x] Confirm real client IPs survive the DO LB + Cilium Gateway path
      (externalTrafficPolicy / PROXY protocol). The 22a login rate limiter
      keys on client IP via `SmartIpKeyExtractor`; if source IPs are not
      preserved it keys on the LB address and throttles all users
      collectively. (Carried in from Phase 22a, 2026-07-03.)
      **Researched, not resolved (2026-07-05):** confirmed via live
      `kubectl get configmap -n kube-system cilium-config` on the `brdgme`
      prod cluster that `enable-gateway-api-proxy-protocol: "false"` is the
      real, currently-disabled control (Cilium requires PROXY protocol on
      the Envoy gateway listener to see the real client IP through a TCP-
      passthrough LB; DO's LB SNATs by default and only adds
      `X-Forwarded-For` in its HTTP/HTTPS mode, which Gateway API doesn't
      use). There is no `doctl`/DO-API-level knob for this flag - it would
      require directly editing the DOKS-managed `cilium-config` ConfigMap
      and restarting the `cilium` DaemonSet, which touches a live
      DO-managed add-on. Decided 2026-07-05: do not test this against the
      live prod cluster now (no Gateway exists yet to actually need it, and
      an unrequested edit to a managed add-on's config is exactly the kind
      of shared-infrastructure change to avoid without a concrete need).
      Deferred to Phase 16: flip the flag, restart `cilium`, confirm it
      isn't reverted by DOKS's reconciler, then set
      `do-loadbalancer-enable-proxy-protocol: "true"` on the Gateway (see
      commented-out annotation in `gateway.yaml`) - in that order, since
      enabling the DO-side annotation first would have the LB send PROXY
      headers to an Envoy not yet expecting them and break all traffic.

      **Resolved 2026-07-08 - attempted and dropped.** Ran the deferred
      steps live on the `brdgme` prod cluster: patched
      `enable-gateway-api-proxy-protocol` to `"true"` in
      `kube-system/cilium-config`, restarted the `cilium` DaemonSet
      successfully, confirmed the flag read back `"true"`. DOKS's managed
      addon reconciler (fieldManager `manager`) rewrote the ConfigMap back
      to `"false"` at 13:09:20Z, ~15 minutes later - confirming DOKS owns
      `cilium-config` and the flag cannot be set persistently by the
      cluster operator, exactly the risk flagged above. The matching
      `do-loadbalancer-enable-proxy-protocol` annotation had briefly
      deployed via ArgoCD in the same window and was reverted the same
      hour (`brdgme` f31be4b, `brdgme-config` 8333793); prod is back to
      the pre-flip state and `beta.brdg.me` stayed up throughout. Decision
      (Michael, 2026-07-08): drop the client-IP/PROXY-protocol work
      entirely rather than open a DO support ticket or retry - real client
      IPs are simply not available to the app on this platform. Per-IP
      app-level limits are therefore a collective bucket (keyed on the LB
      SNAT address) and XFF-spoofable, permanently. The effective
      protections going forward are #28's IP-independent D2 caps
      (DB-backed send caps + per-code attempt caps, promoted to
      pre-go-live priority the same day) and, post-cutover, Cloudflare
      edge per-IP limiting (Cloudflare sees real client IPs). This item
      has no remaining work; see `docs/BACKLOG.md` (#14 archived) and
      `docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md`.
- [x] Remove the Knative/net-certmanager one-time `kubectl apply`
      prerequisites from the Phase 16 notes; cert-manager alone remains.
      Already done: Phase 16's "Superseded, do not do" bullet documents the
      Knative-era items (DomainMappings, config-domain, Kourier TLS) as
      superseded by this phase; no `net-certmanager` references remain
      anywhere in `docs/`.

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
