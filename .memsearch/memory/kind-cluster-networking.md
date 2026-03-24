# Kind Cluster Networking Research

Research notes for migrating local dev to Cilium CNI with Cilium Gateway API as the Knative networking layer.

## Current Setup (as of 2026-03-24)

- Kind cluster: `kindest/node:v1.34.0`, `disableDefaultCNI: true`, `kubeProxyMode: none`
- Cilium 1.19.1 with `kubeProxyReplacement=true`, `gatewayAPI.enabled=true`
- Gateway API CRDs v1.4.1
- Knative Serving 1.21.1 + net-gateway-api 1.21.0
- Two Cilium Gateways (`knative-ingress-gateway`, `knative-local-gateway`) in `knative-serving`
- Workaround: patch `cilium-gateway-knative-ingress-gateway` service from `LoadBalancer` to `NodePort 30080`
- Kind `extraPortMappings`: `containerPort: 30080 -> hostPort: 8080`
- Domain: `lvh.me` (`*.lvh.me -> 127.0.0.1` via public DNS)

The cluster was not working reliably - services inaccessible and/or Knative routes stuck `Uninitialized`.

## Root Cause: Six Problems with Cilium in Kind

### Problem 1: NodePort unreachable from host with Cilium eBPF (Critical)

When `kubeProxyReplacement=true`, Cilium eBPF handles all packet routing. In Kind this breaks NodePort services accessed from the host machine. eBPF socket intercept and Kind's container networking do not cooperate.

- Cilium issues #25479, #42997 - both open, structural incompatibility
- Multiple reports over several years, no resolution in Cilium 1.19.x
- The entire previous approach was built on this broken foundation

### Problem 2: Host network mode broken (Cilium 1.18.3+)

`gatewayAPI.hostNetwork.enabled=true` is broken in Cilium 1.18.3+ including 1.19.1 (issue #42786, OPEN). Gateway stays `PROGRAMMED: False` ("Gateway waiting for address") because the code checks for a LoadBalancer address but host network mode creates a ClusterIP service with no external address.

### Problem 3: L2 Announcements + Gateway API broken

External traffic does not reach Gateway API backends when L2 Announcements are enabled. Cilium issue #43819, OPEN, affects v1.18.5-v1.19.0+. BPF load balancer entries show `[LoadBalancer, l7-load-balancer]` without endpoints.

### Problem 4: Cilium not officially supported by net-gateway-api

net-gateway-api officially tests and supports Istio, Contour, and Envoy Gateway. Cilium is not on that list. Conformance testing request (issue #553) open since 2023.

### Problem 5: KIngress stuck Uninitialized

The net-gateway-api controller determines route readiness by probing the service named in `config-gateway`. With Cilium, Gateway service may not have expected addresses, causing all KIngress objects to remain `Uninitialized` forever. net-gateway-api issue #817.

### Problem 6: ClusterIP not supported for local gateway

Cilium only supports `LoadBalancer` and `NodePort` for Gateway services. ClusterIP support being discussed (CFP #44113, open) but not implemented. Using NodePort for local gateway unnecessarily exposes internal cluster ports.

## Approaches Considered

### A. Current approach: patch Gateway to NodePort (BROKEN)
Fundamentally broken by Problem 1. Cilium eBPF prevents NodePort access from host in Kind.

### B. Host network mode (BROKEN)
Broken by Problem 2. `PROGRAMMED: False` regression in Cilium 1.18.3+ unresolved.

### C. Cilium L2 Announcements / LB IPAM (BROKEN)
Broken by Problem 3. Gateway API backends unreachable with L2 Announcements enabled.

### D. cloud-provider-kind (Secondary option, untested)

`cloud-provider-kind` (sigs.k8s.io/cloud-provider-kind) watches LoadBalancer services and provisions real IPs from a routable address space.

- On Linux, assigned IPs are reachable from the host (no tunnel needed)
- Cilium creates a LoadBalancer service for each Gateway; cloud-provider-kind assigns it an IP
- Avoids NodePort entirely
- Uses sslip.io DNS: service hostname embeds the IP, e.g. `app.default.172.18.255.200.sslip.io`
- Risks: described as "very alpha"; requires elevated privileges; unknown interaction with Cilium Gateway API; Problem 3 may not apply since cloud-provider-kind is not L2 announcements

### E. Drop Cilium in dev, use Kourier (RECOMMENDED)

Kourier is the only officially supported and documented Knative ingress for Kind. Every official Knative guide and `kn quickstart kind` tool uses Kourier.

Eliminates all four critical open issues (Problems 1-4). Production continues using Cilium.

Changes to `scripts/setup-kind-cluster.sh`:
- Remove Cilium helm install, Gateway API CRDs install, and gateway configuration
- Remove `kubeProxyMode: none` and `disableDefaultCNI: true` from kind-config.yaml (use default CNI)
- Install Kourier: `kubectl apply -f .../net-kourier.yaml`
- Patch Knative to use Kourier: `kubectl patch configmap/config-network -n knative-serving --type merge -p '{"data":{"ingress-class":"kourier.ingress.networking.knative.dev"}}'`
- Expose Kourier via NodePort on port 30080, mapped to host 8080 via extraPortMappings
- Domain: keep lvh.me

Kourier's service can be patched to a fixed NodePort without Cilium eBPF disrupting the traffic path: `host:8080 -> containerPort:30080 -> NodePort:30080 -> Kourier`.

Cost: dev and prod use different networking layers; cannot test Cilium-specific Gateway API features locally.

### F. Use Envoy Gateway instead of Cilium Gateway

Envoy Gateway is one of the three officially tested net-gateway-api implementations. Install as standalone ingress controller (CNI can remain Cilium). Fully supported by net-gateway-api. LoadBalancer services also need cloud-provider-kind in Kind. More complex than Kourier but more production-like.

## DNS Options for Local Dev

### lvh.me (current)
`*.lvh.me -> 127.0.0.1` via public DNS. No configuration needed. Best for NodePort/hostPort at 127.0.0.1. No SSL. Offline dependency.

### sslip.io / nip.io
Embeds IP in hostname: `app.default.172.18.255.200.sslip.io -> 172.18.255.200`. Required when Gateway has an IP other than 127.0.0.1 (cloud-provider-kind approach).

### traefik.me / localtest.me
Alternative wildcard `-> 127.0.0.1`. Drop-in replacements for lvh.me.

### local.brdg.me (not yet set up)
Add wildcard A record `*.local.brdg.me -> 127.0.0.1` to brdg.me DNS zone. Advantages: custom domain, future TLS via wildcard Let's Encrypt cert with DNS-01 challenge. Requires DNS zone access, does not work offline.

### Knative magic DNS job
`kubectl apply -f https://github.com/knative/serving/releases/download/knative-v1.21.1/serving-default-domain.yaml` - detects ingress IP, patches `config-domain` to use sslip.io with embedded IP.

### DNS Recommendation
For Kourier: keep lvh.me. For future TLS: add `*.local.brdg.me -> 127.0.0.1` wildcard to brdg.me zone.

## Component Versions (as of 2026-03-24)

| Component | Version | Notes |
|-----------|---------|-------|
| Cilium | 1.19.1 | No Gateway API fixes in 1.19.2 patch |
| Gateway API CRDs | v1.4.1 | Current stable |
| Knative Serving | 1.21.1 | Current |
| net-gateway-api | 1.21.0 | Current |
| kindest/node | v1.34.0 | Kubernetes 1.34 |

## Open Questions (as of 2026-03-24)

1. Does cloud-provider-kind work reliably with Cilium Gateway API in Kind on Linux? (untested)
2. Is the Cilium host network `PROGRAMMED: False` bug fixed in 1.19.2 or any 1.20.x release? (Cilium issue #42786)
3. Is the Gateway API + L2 Announcements traffic bug fixed in 1.19.x? (Cilium issue #43819)
4. Does brdg.me DNS allow adding `*.local.brdg.me -> 127.0.0.1`?

## Sources

- Cilium Kind Installation: https://docs.cilium.io/en/stable/installation/kind/
- Cilium Gateway API: https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/gateway-api/
- Cilium issue #25479 - NodePort unreachable in Kind+kubeproxyless
- Cilium issue #42786 - host network PROGRAMMED: False
- Cilium issue #42997 - NodePort unreachable from host in Kind
- Cilium issue #43819 - L2 + Gateway API no external traffic
- Cilium issue #44113 - ClusterIP internal gateway CFP
- net-gateway-api issue #553 - Cilium conformance
- net-gateway-api issue #817 - KIngress Uninitialized with Cilium
- cloud-provider-kind: https://github.com/kubernetes-sigs/cloud-provider-kind
- Tilt Knative extension: https://github.com/tilt-dev/tilt-extensions/blob/master/knative/Tiltfile
- sslip.io: https://sslip.io/
