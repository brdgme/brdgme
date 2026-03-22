# Tiltfile
#
# Default (hybrid) mode:
#   Builds game images locally, deploys backing services and game microservices
#   to Kind. Runs `cargo leptos watch` locally on port 3000 for fast iteration.
#
# Full-cluster mode (WEB_IN_CLUSTER=1):
#   Also builds brdgme/web and deploys it as a Knative Service in Kind.
#
# Legacy side-by-side mode (LEGACY=1):
#   Also builds and deploys web-legacy, api, and websocket as Knative Services.
#   Services are accessible via lvh.me domain routing on port 8080.
#
# Prerequisites: run scripts/setup-kind-cluster.sh once before `tilt up`.

WEB_IN_CLUSTER = os.getenv("WEB_IN_CLUSTER", "") == "1"
LEGACY = os.getenv("LEGACY", "") == "1"

# Dev environment only - credentials are not sensitive here.
secret_settings(disable_scrub=True)

# --- Game image builds ---

# Rust games
for game in ["acquire-1", "lost-cities-1", "lost-cities-2"]:
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
# Required for Knative: it resolves image digests from the registry directly,
# so kind load docker-image is insufficient.
default_registry('localhost:5000', host_from_cluster='kind-registry:5000')

# Register Knative Service as a workload type so Tilt can track it.
k8s_kind('Service', api_version='serving.knative.dev/v1', image_json_path='{.spec.template.spec.containers[0].image}')

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
  DATABASE_URL: postgres://brdgme_user:brdgme_password@postgres/brdgme
"""))

if WEB_IN_CLUSTER:
    docker_build(
        "brdgme/web",
        ".",
        dockerfile="rust/Dockerfile",
        target="web",
    )
    k8s_yaml(kustomize("k8s/dev"))
else:
    k8s_yaml(kustomize("k8s/dev-without-web"))
    local_resource(
        "web",
        serve_cmd="cd rust/web && SQLX_OFFLINE=true mirrord exec --target pod/postgres-0 --target-namespace brdgme -- cargo leptos watch",
        links=["http://localhost:3000"],
        resource_deps=["postgres"],
    )

k8s_resource("postgres", port_forwards=["5432:5432"])
k8s_resource("redis", port_forwards=["6379:6379"])

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
    serve_cmd="cd rust && DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme RUST_LOG=info cargo run -p brdgme-operator",
    resource_deps=["postgres", "crd-ready"],
)

if LEGACY:
    docker_build("brdgme/web-legacy", ".", dockerfile="web/Dockerfile", target="web", only=["web/"])
    docker_build("brdgme/websocket", ".", dockerfile="websocket/Dockerfile", target="websocket", only=["websocket/"])
    docker_build("brdgme/api", ".", dockerfile="rust/api/Dockerfile", target="api", only=["rust/"])
    k8s_yaml(kustomize("k8s/dev-legacy"))
    k8s_resource("web-legacy", links=["http://web-legacy.brdgme.lvh.me:8080"])
    k8s_resource("api", links=["http://api.brdgme.lvh.me:8080"])
    k8s_resource("websocket", links=["http://websocket.brdgme.lvh.me:8080"])
