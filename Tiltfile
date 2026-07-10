# Tiltfile
#
# Default (hybrid) mode:
#   Builds game images locally, deploys backing services and game microservices
#   to Kind. Runs `cargo leptos watch` locally on port 3000 for fast iteration.
#
# Full-cluster mode (WEB_IN_CLUSTER=1):
#   Also builds brdgme/web and deploys it as a Deployment + ClusterIP Service
#   in Kind.
#
# Legacy side-by-side mode (LEGACY=1):
#   Also builds and deploys web-legacy, api, and websocket as Deployments +
#   ClusterIP Services, plus a dev-only Gateway/HTTPRoute set routing by
#   *.brdgme.lvh.me hostname. Reachable via `kubectl port-forward` (see the
#   "legacy-gateway" resource below) on port 8080.
#
# Prerequisites: run scripts/setup-kind-cluster.sh once before `tilt up`.

WEB_IN_CLUSTER = os.getenv("WEB_IN_CLUSTER", "") == "1"
LEGACY = os.getenv("LEGACY", "") == "1"

# Dev environment only - credentials are not sensitive here.
secret_settings(disable_scrub=True)

# --- Game image builds ---

# Rust games
for game in ["acquire-1", "battleship-2", "category-5-2", "farkle-2", "for-sale-2", "greed-2", "jaipur-2", "liars-dice-2", "lost-cities-1", "lost-cities-2", "no-thanks-2", "sushi-go-2", "sushizock-2", "tic-tac-toe-2", "zombie-dice-2"]:
    docker_build("brdgme/" + game, ".", dockerfile="rust/Dockerfile", target=game, only=["rust/"])

# Go games
for game in [
    "age-of-war-1",
    "battleship-1",
    "category-5-1",
    "cathedral-1",
    "farkle-1",
    "for-sale-1",
    "greed-1",
    "liars-dice-1",
    "love-letter-1",
    "modern-art-1",
    "no-thanks-1",
    "roll-through-the-ages-1",
    "splendor-1",
    "sushi-go-1",
    "sushizock-1",
    "texas-holdem-1",
    "zombie-dice-1",
]:
    docker_build("brdgme/" + game, ".", dockerfile="brdgme-go/Dockerfile", target=game, only=["brdgme-go/", "go.mod"])

# Push images to the local Kind registry. The host pushes to localhost:5000
# (port-mapped from the kind-registry container); manifests reference
# kind-registry:5000 which resolves inside the cluster via Docker network DNS.
# Not strictly required now that everything is a plain Deployment (`kind load
# docker-image` would work too), but kept - pushing to a registry is faster
# than `kind load` for repeated iteration.
default_registry('localhost:5000', host_from_cluster='kind-registry:5000')

# --- Kubernetes resources ---

# Create the brdgme namespace (kustomize sets it on resources but does not
# create the namespace object itself).
k8s_yaml(blob("""
apiVersion: v1
kind: Namespace
metadata:
  name: brdgme
"""))

# Dev postgres credentials. In production this secret is managed outside Tilt.
k8s_yaml(blob("""
apiVersion: v1
kind: Secret
metadata:
  name: postgres-config
  namespace: brdgme
stringData:
  POSTGRES_USER: brdgme_user
  POSTGRES_PASSWORD: brdgme_password
  POSTGRES_DB: brdgme
  DATABASE_URL: postgres://brdgme_user:brdgme_password@postgres-rw/brdgme
"""))

# CNPG's bootstrap.initdb.secret requires a kubernetes.io/basic-auth secret
# (username/password keys) - postgres-config above doesn't qualify, so this
# is a separate secret referenced by the Cluster CR (k8s/base/postgres).
k8s_yaml(blob("""
apiVersion: v1
kind: Secret
metadata:
  name: postgres-user
  namespace: brdgme
type: kubernetes.io/basic-auth
stringData:
  username: brdgme_user
  password: brdgme_password
"""))

# Track the CNPG Cluster CR as a Tilt resource (the operator creates its pods).
k8s_kind('Cluster', api_version='postgresql.cnpg.io/v1', pod_readiness='wait')

if WEB_IN_CLUSTER:
    docker_build(
        "brdgme/web",
        ".",
        dockerfile="rust/Dockerfile",
        target="web",
    )
    docker_build(
        "brdgme/bot",
        ".",
        dockerfile="rust/Dockerfile",
        target="bot",
        only=["rust/"],
    )
    k8s_yaml(kustomize("k8s/dev"))
    k8s_yaml(blob("""
apiVersion: v1
kind: Secret
metadata:
  name: bot-config
  namespace: brdgme
stringData:
  LLM_URL: {llm_url}
  LLM_API_KEY: {llm_api_key}
  BOT_MODEL: {bot_model}
""".format(
        llm_url=os.getenv("LLM_URL", "http://localhost:11434"),
        llm_api_key=os.getenv("LLM_API_KEY", ""),
        bot_model=os.getenv("BOT_MODEL", "qwen3:4b"),
    )))
else:
    k8s_yaml(kustomize("k8s/dev-without-web"))
    local_resource(
        "web",
        serve_cmd="cd rust/web && SQLX_OFFLINE=true DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme NATS_URL=nats://localhost:4222 mirrord exec -f .mirrord/mirrord.json --target pod/nats-0 --target-namespace brdgme -- cargo leptos watch",
        links=["http://localhost:3000"],
        resource_deps=["postgres", "nats"],
    )
    local_resource(
        "bot",
        serve_cmd="set -a && [ -f .env ] && . ./.env; set +a && cd rust/bot && RUST_LOG=${RUST_LOG:-info} DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme NATS_URL=nats://localhost:4222 mirrord exec -f .mirrord/mirrord.json --target pod/nats-0 --target-namespace brdgme -- cargo run",
        resource_deps=["postgres", "nats"],
    )

k8s_resource("postgres", port_forwards=["5432:5432"])
k8s_resource("nats", port_forwards=["4222:4222"])

# Run migrations manually. Trigger from the Tilt UI or via:
#   cd rust/web && sqlx migrate run
# After running migrations, regenerate SQLx query metadata:
#   cd rust/web && cargo sqlx prepare -- --features ssr
local_resource(
    "migrate",
    cmd="cd rust/web && sqlx migrate run",
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    resource_deps=["postgres"],
)

local_resource(
    "crd-ready",
    cmd="kubectl wait --for=condition=established --timeout=60s crd/gameversions.brdgme.com",
)

local_resource(
    "operator",
    serve_cmd="for i in $(seq 60); do pg_isready -h localhost -p 5432 >/dev/null 2>&1 && break; sleep 1; done; cd rust && DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme RUST_LOG=info mirrord exec -f operator/.mirrord/mirrord.json --target pod/nats-0 --target-namespace brdgme -- cargo run -p brdgme-operator",
    resource_deps=["postgres", "nats", "crd-ready"],
)

if LEGACY:
    docker_build("brdgme/web-legacy", ".", dockerfile="web/Dockerfile", target="web", only=["web/"])
    docker_build("brdgme/websocket", ".", dockerfile="websocket/Dockerfile", target="websocket", only=["websocket/"])
    docker_build("brdgme/api", ".", dockerfile="rust/api/Dockerfile", target="api", only=["rust/"])
    k8s_yaml(kustomize("k8s/dev-legacy"))
    k8s_resource("web-legacy", links=["http://web-legacy.brdgme.lvh.me:8080"])
    k8s_resource("api", links=["http://api.brdgme.lvh.me:8080"])
    k8s_resource("websocket", links=["http://websocket.brdgme.lvh.me:8080"])
    k8s_resource("redis", port_forwards=["6379:6379"])

# Dev-only Gateway/HTTPRoute set routing by *.brdgme.lvh.me hostname.
# k8s/base/gateway/ (real hostnames, HTTPS, cert-manager) is prod-only -
# this dev equivalent is plain HTTP and lives only in the Tiltfile, per
# the "dev workarounds belong in the Tiltfile, not k8s/" convention.
#
# Cilium provisions a per-Gateway LoadBalancer Service (named
# cilium-gateway-<gateway-name>) which stays <pending> in Kind - it has no
# selector (Cilium programs endpoints itself, not via backing pods), so
# `kubectl port-forward` on it can never work. Instead, this pins the
# Service's NodePort to 31080 (Cilium allocates NodePorts on LoadBalancer
# Services since nodePort.enabled=true is set in setup-kind-cluster.sh),
# which lines up with the extraPortMappings entry in ctlptl.yaml
# (hostPort 8080 -> containerPort 31080) to make each service reachable at
# `{service}.brdgme.lvh.me:8080`. The Service is created dynamically by the
# Gateway controller, not by an applied manifest, so this can't be a
# setup-script patch - it has to wait for the Service to exist first.
#
# Created whenever a Deployment exists for it to route to: the "web" route
# under WEB_IN_CLUSTER=1 (in-cluster web), the legacy trio's routes under
# LEGACY=1.
if WEB_IN_CLUSTER or LEGACY:
    gateway_routes = ""
    gateway_deps = []
    if WEB_IN_CLUSTER:
        gateway_routes += """
---
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: web
  namespace: brdgme
spec:
  parentRefs:
    - name: brdgme-dev
  hostnames:
    - web.brdgme.lvh.me
  rules:
    - backendRefs:
        - name: web
          port: 3000
"""
        gateway_deps.append("web")
    if LEGACY:
        gateway_routes += """
---
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: web-legacy
  namespace: brdgme
spec:
  parentRefs:
    - name: brdgme-dev
  hostnames:
    - web-legacy.brdgme.lvh.me
  rules:
    - backendRefs:
        - name: web-legacy
          port: 80
---
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: api
  namespace: brdgme
spec:
  parentRefs:
    - name: brdgme-dev
  hostnames:
    - api.brdgme.lvh.me
  rules:
    - backendRefs:
        - name: api
          port: 8000
---
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: websocket
  namespace: brdgme
spec:
  parentRefs:
    - name: brdgme-dev
  hostnames:
    - websocket.brdgme.lvh.me
  rules:
    - backendRefs:
        - name: websocket
          port: 80
"""
        gateway_deps += ["web-legacy", "api", "websocket"]

    k8s_yaml(blob("""
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: brdgme-dev
  namespace: brdgme
spec:
  gatewayClassName: cilium
  listeners:
    - name: http
      port: 80
      protocol: HTTP
      allowedRoutes:
        namespaces:
          from: Same
""" + gateway_routes))
    local_resource(
        "gateway-nodeport",
        cmd="for i in $(seq 60); do kubectl get svc -n brdgme cilium-gateway-brdgme-dev >/dev/null 2>&1 && break; sleep 1; done; " +
            "kubectl patch svc -n brdgme cilium-gateway-brdgme-dev --type=merge -p '{\"spec\":{\"ports\":[{\"name\":\"port-80\",\"port\":80,\"protocol\":\"TCP\",\"targetPort\":80,\"nodePort\":31080}]}}'",
        resource_deps=gateway_deps,
    )
