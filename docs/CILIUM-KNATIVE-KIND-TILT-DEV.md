# Cilium + Knative + Kind + Tilt Dev Environment Research

Research notes for migrating local dev to Cilium CNI with Cilium Gateway API as the Knative
networking layer. This document records findings before implementation to avoid repeating failed
approaches.

---

## Current Setup (as of March 2026)

- Kind cluster: `kindest/node:v1.34.0`, `disableDefaultCNI: true`, `kubeProxyMode: none`
- Cilium 1.19.1 with `kubeProxyReplacement=true`, `gatewayAPI.enabled=true`
- Gateway API CRDs v1.4.1
- Knative Serving 1.21.1 + net-gateway-api 1.21.0
- Two Cilium Gateways (`knative-ingress-gateway`, `knative-local-gateway`) in `knative-serving`
- Workaround: patch `cilium-gateway-knative-ingress-gateway` service from `LoadBalancer` to `NodePort 30080`
- Kind `extraPortMappings`: `containerPort: 30080 -> hostPort: 8080`
- Domain: `lvh.me` (`*.lvh.me -> 127.0.0.1` via public DNS)

The issue: the cluster was not working reliably. Services were inaccessible and/or Knative
routes were stuck `Uninitialized`.

---

## Root Cause Analysis

### Problem 1: NodePort unreachable from host with Cilium eBPF

**Severity: Critical. This is likely the primary cause of failure.**

When `kubeProxyReplacement=true`, Cilium's eBPF data plane handles all packet routing instead
of kube-proxy. In Kind, this breaks NodePort services accessed from the host machine (i.e.,
from outside the Kind Docker network). The eBPF socket intercept and Kind's container
networking do not cooperate cleanly.

- Cilium issues: #25479, #42997 - both open, described as a structural incompatibility
- Reported by multiple users over several years; no resolution in Cilium 1.19.x
- NodePort via `extraPortMappings` is the standard Kind workaround for services, but it does
  not work reliably when Cilium eBPF is the routing layer

**The entire current approach is built on top of this broken foundation.**

### Problem 2: Host network mode broken (Cilium 1.18.3+)

An alternative to NodePort is Cilium's host network mode (`gatewayAPI.hostNetwork.enabled=true`),
which binds Envoy listeners directly on the host network. This would avoid NodePort entirely.

**It is broken in Cilium 1.18.3+ (including 1.19.1): issue #42786 (OPEN)**

The Gateway resource stays `PROGRAMMED: False` with "Gateway waiting for address" because the
code checks for a LoadBalancer address but host network mode creates a ClusterIP service with
no external address.

### Problem 3: L2 Announcements + Gateway API broken

Using Cilium L2 Announcements (LB IPAM) to give the Gateway service a real IP from the Kind
node subnet would avoid NodePort. However:

**External traffic does not reach Gateway API backends when L2 Announcements are enabled:
issue #43819 (OPEN, affects v1.18.5-v1.19.0+)**

BPF load balancer entries show `[LoadBalancer, l7-load-balancer]` without endpoints.

### Problem 4: Cilium not officially supported by net-gateway-api

`net-gateway-api` officially tests and supports Istio, Contour, and Envoy Gateway. Cilium is
not on that list. A conformance testing request (issue #553) has been open since 2023.

### Problem 5: KIngress stuck Uninitialized

The net-gateway-api controller determines route readiness by probing the service named in
`config-gateway`. With Cilium, the Gateway service may not have the expected addresses
(because it is a patched NodePort or an unresolved LoadBalancer), causing all KIngress objects
to remain `Uninitialized` forever.

Issue #817 in knative-extensions/net-gateway-api was closed "not planned" then reopened in
October 2025 - indicating this remains a live problem.

### Problem 6: ClusterIP not supported for local gateway

Knative needs a local gateway for cluster-internal routing. Ideally this uses ClusterIP for
security (not exposed outside the cluster). Cilium only supports `LoadBalancer` and `NodePort`
for Gateway services. ClusterIP support is being discussed upstream (CFP #44113, open) but
not implemented. Using NodePort for the local gateway exposes internal cluster ports on all
nodes unnecessarily.

---

## Approaches Considered

### A. Current approach: patch Gateway to NodePort (BROKEN)

Status: Fundamentally broken by Problem 1. Cilium eBPF prevents NodePort access from host in Kind.

### B. Host network mode (BROKEN)

Status: Broken by Problem 2. PROGRAMMED: False regression in Cilium 1.18.3+ is unresolved.

### C. Cilium L2 Announcements / LB IPAM (BROKEN)

Status: Broken by Problem 3. Gateway API backends unreachable with L2 Announcements enabled.

### D. cloud-provider-kind

`cloud-provider-kind` (sigs.k8s.io/cloud-provider-kind) watches LoadBalancer services and
provisions real IPs from a routable address space, acting as a cloud provider for Kind.

- On Linux, the assigned IPs are reachable from the host (no tunnel needed)
- Cilium creates a LoadBalancer service for each Gateway; cloud-provider-kind assigns it an IP
- Avoids NodePort entirely
- Uses sslip.io DNS: service hostname embeds the IP, e.g. `app.default.172.18.255.200.sslip.io`
- Risks:
  - Described as "very alpha" by maintainers
  - Requires elevated privileges (needs to manage host routes)
  - Unknown interaction with Cilium Gateway API specifically
  - Problem 3 (L2 announcements) may not apply since cloud-provider-kind is not L2 announcements

### E. Drop Cilium for local dev, use Kourier (RECOMMENDED - lowest risk)

Kourier is the only officially supported and documented Knative ingress for Kind. Every
official Knative guide, blog post, and `kn quickstart kind` tool uses Kourier.

The argument for this approach:
- Production continues to use Cilium (separate environments, different config)
- Local dev uses Kourier: well-tested, documented, zero open blockers
- Eliminates all four critical open issues (Problems 1-4)
- Knative in Kind with Kourier: `kn quickstart kind` sets it up automatically
- Domain: keep lvh.me or switch to sslip.io (embedded IP)

The cost:
- Dev and prod use different networking layers (Gateway API class, ingress class differ)
- Cannot test Cilium-specific Gateway API features locally (PROXY protocol, etc.)
- Must maintain two configurations (or use an env var toggle in the setup script)

### F. Use Envoy Gateway instead of Cilium Gateway

Envoy Gateway is one of the three officially tested net-gateway-api implementations. It is a
standalone Gateway API controller backed by Envoy proxy, separate from any CNI.

- Install Envoy Gateway as the ingress controller (CNI remains as default Kind CNI or Cilium)
- Envoy Gateway's LoadBalancer services also need cloud-provider-kind in Kind
- Fully supported by net-gateway-api
- More complex setup than Kourier but more production-like than Kourier
- Knative blog post: https://dev.to/kahirokunn/extending-knative-service-with-envoy-gateway-integration-56ak

---

## DNS Options

### lvh.me (current)

`*.lvh.me -> 127.0.0.1` via public DNS. No configuration needed. Works for any service
accessible on `127.0.0.1`. Best fit for the NodePort/hostPort approach (port 8080 in URL).
No SSL (no CA for lvh.me). Offline dependency on public DNS.

### sslip.io / nip.io

Embeds an IP in the hostname: `app.default.172.18.255.200.sslip.io -> 172.18.255.200`.
Required when the Gateway has an IP other than 127.0.0.1 (cloud-provider-kind approach).
sslip.io now hosts nip.io. Supports IPv6.

### traefik.me / localtest.me

Alternative wildcard `-> 127.0.0.1` services. Drop-in replacements for lvh.me.

### local.brdg.me (custom)

Add a wildcard A record to `brdg.me` DNS: `*.local.brdg.me -> 127.0.0.1`. Works exactly like
lvh.me but on a domain the project controls. Advantages:
- Custom domain can match production hostname patterns
- Future TLS: get a wildcard cert from Let's Encrypt for `*.local.brdg.me` with DNS-01
  challenge; install in the Gateway for HTTPS local dev
Disadvantages:
- Requires DNS zone access (must control brdg.me nameservers)
- Does not work offline

### Knative magic DNS job

```bash
kubectl apply -f https://github.com/knative/serving/releases/download/knative-v1.21.1/serving-default-domain.yaml
```

Runs a job that detects the ingress IP, then patches `config-domain` to use sslip.io with
that IP embedded. With NodePort at 127.0.0.1, the domain becomes `*.default.127.0.0.1.sslip.io`.
Works automatically but requires the ingress to be accessible before running.

---

## Tilt + Knative

An official Tilt extension exists: https://github.com/tilt-dev/tilt-extensions/blob/master/knative/Tiltfile

Key functions:
- `knative_install(version)`: installs Knative CRDs + core, configures dev registries
  (`kind.local`, `ko.local`, `dev.local`), waits for webhook readiness
- `knative_yaml(file)`: processes Knative Service resources; auto-injects
  `autoscaling.knative.dev/minScale: "1"` to prevent scale-to-zero killing Tilt's live update
  target

Usage:
```python
load('ext://knative', 'knative_install', 'knative_yaml')
knative_install()
knative_yaml('path/to/ksvc.yaml')
```

The extension does not handle networking/ingress. That is configured separately.

**Critical:** always set `minScale: 1` on all dev Knative Services. Without it Tilt's live
update target pod is killed when traffic goes to zero.

The brdgme Tiltfile already sets `minScale: 1` manually via annotations on each Knative
Service; the extension could replace that boilerplate, but the manual approach also works.

---

## Version Notes

| Component | Current | Latest | Notes |
|-----------|---------|--------|-------|
| Cilium | 1.19.1 | 1.19.2 | No Gateway API fixes in 1.19.2 patch |
| Gateway API CRDs | v1.4.1 | v1.4.1 | Current stable |
| Knative Serving | 1.21.1 | 1.21.1 | Current |
| net-gateway-api | 1.21.0 | 1.21.0 | Current |
| kindest/node | v1.34.0 | v1.34.0 | Kubernetes 1.34; no known incompatibilities |

---

## Recommendations

### Recommended path: Option E (drop Cilium in dev, use Kourier)

Rationale:
1. All three Cilium-based approaches (NodePort, host network, L2 IPAM) have critical open bugs
   with no near-term resolution. These are not configuration errors - they are known architectural
   incompatibilities between Cilium eBPF and Kind's container networking.
2. Kourier is the official Knative Kind ingress. It has no open blockers for this use case.
3. The main goal (domain-based routing for side-by-side legacy + new Leptos service) is fully
   achievable with Kourier.
4. Production Cilium behavior is not impacted - Cilium stays in production k8s.

**Changes to scripts/setup-kind-cluster.sh:**
- Remove Cilium helm install, Gateway API CRDs install, and gateway configuration
- Remove `kubeProxyMode: none` and `disableDefaultCNI: true` from kind-config.yaml (use default CNI)
- Install Kourier: `kubectl apply -f .../net-kourier.yaml`
- Patch Knative to use Kourier: `kubectl patch configmap/config-network -n knative-serving --type merge -p '{"data":{"ingress-class":"kourier.ingress.networking.knative.dev"}}'`
- Expose Kourier via NodePort on port 30080, mapped to host 8080 via extraPortMappings
- Domain config: keep lvh.me (Kourier + NodePort + extraPortMappings works correctly)

Kourier's service can be patched to a fixed NodePort without Cilium eBPF disrupting the
traffic path, so `host:8080 -> containerPort:30080 -> NodePort:30080 -> Kourier` works.

### Secondary path: Option D (cloud-provider-kind)

If Cilium CNI is required in local dev (e.g., for testing Cilium NetworkPolicy), consider:
1. Keep Cilium as CNI with `kubeProxyReplacement=true`
2. Run `cloud-provider-kind` as a local_resource in the Tiltfile (or as a background process)
3. Let Cilium Gateway get a real IP assigned by cloud-provider-kind
4. Use sslip.io DNS (embed the IP in hostnames)
5. Switch domain from lvh.me to sslip.io-based

Risk: cloud-provider-kind is alpha and interaction with Cilium Gateway API is untested.
If it works, this is a better prod-fidelity dev environment.

### DNS recommendation

For Option E (Kourier): keep lvh.me. It works well for `127.0.0.1`-based access.

For future TLS or more production-like setup: add `*.local.brdg.me -> 127.0.0.1` as a
wildcard DNS record to the brdg.me zone, then migrate domains. This allows DNS-01 wildcard
TLS certs via Let's Encrypt in future.

---

## Open Questions

1. Does cloud-provider-kind work reliably with Cilium Gateway API in Kind on Linux? (untested)
2. Is the Cilium host network PROGRAMMED: False bug fixed in 1.19.2 or any 1.20.x release?
   Check: https://github.com/cilium/cilium/issues/42786
3. Is the Gateway API + L2 Announcements traffic bug fixed in 1.19.x?
   Check: https://github.com/cilium/cilium/issues/43819
4. Does brdg.me DNS allow adding `*.local.brdg.me -> 127.0.0.1`?

---

## Sources

- [Cilium Kind Installation](https://docs.cilium.io/en/stable/installation/kind/)
- [Cilium Gateway API](https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/gateway-api/)
- [Cilium GatewayClass Parameters](https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/parameterized-gatewayclass/)
- [Cilium Gateway API Host Network Mode](https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/host-network-mode/)
- [Cilium kube-proxy free](https://docs.cilium.io/en/stable/network/kubernetes/kubeproxy-free/)
- [Cilium L2 Announcements](https://docs.cilium.io/en/stable/network/l2-announcements/)
- [Cilium issue #25479 - NodePort unreachable in Kind+kubeproxyless](https://github.com/cilium/cilium/issues/25479)
- [Cilium issue #42786 - host network PROGRAMMED: False](https://github.com/cilium/cilium/issues/42786)
- [Cilium issue #42997 - NodePort unreachable from host in Kind](https://github.com/cilium/cilium/issues/42997)
- [Cilium issue #43819 - L2 + Gateway API no external traffic](https://github.com/cilium/cilium/issues/43819)
- [Cilium issue #44113 - ClusterIP internal gateway CFP](https://github.com/cilium/cilium/issues/44113)
- [net-gateway-api issue #553 - Cilium conformance](https://github.com/knative-extensions/net-gateway-api/issues/553)
- [net-gateway-api issue #817 - KIngress Uninitialized with Cilium](https://github.com/knative-extensions/net-gateway-api/issues/817)
- [Knative Kind setup blog](https://knative.dev/blog/articles/set-up-a-local-knative-environment-with-kind/)
- [Tilt Knative extension](https://github.com/tilt-dev/tilt-extensions/blob/master/knative/Tiltfile)
- [cloud-provider-kind](https://github.com/kubernetes-sigs/cloud-provider-kind)
- [Knative + Envoy Gateway blog post](https://dev.to/kahirokunn/extending-knative-service-with-envoy-gateway-integration-56ak)
- [sslip.io](https://sslip.io/)
- [lvh.me wildcard DNS](https://dev.to/nickjj/ngrok-lvhme-and-nipio-a-trilogy-for-local-development-and-testing-5641)
