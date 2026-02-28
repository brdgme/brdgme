# Tiltfile
#
# Default (hybrid) mode:
#   Builds game images locally, deploys backing services and game microservices
#   to Kind. Runs `cargo leptos watch` locally on port 3000 for fast iteration.
#
# Full-cluster mode (WEB_IN_CLUSTER=1):
#   Also builds brdgme/web and deploys it as a Knative Service in Kind.
#
# Prerequisites: run scripts/setup-kind-cluster.sh once before `tilt up`.

WEB_IN_CLUSTER = os.getenv("WEB_IN_CLUSTER", "") == "1"

# --- Game image builds ---

# Rust games
for game in ["acquire-1", "lost-cities-1", "lost-cities-2"]:
    docker_build("brdgme/" + game, ".", dockerfile="rust/Dockerfile", target=game)

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
    docker_build("brdgme/" + game, ".", dockerfile="brdgme-go/Dockerfile", target=game)

# --- Kubernetes resources ---

# Create the brdgme namespace (kustomize sets it on resources but does not
# create the namespace object itself).
k8s_yaml(blob("""
apiVersion: v1
kind: Namespace
metadata:
  name: brdgme
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
        serve_cmd="cd rust/web && cargo leptos watch",
        links=["http://localhost:3000"],
    )

k8s_resource("postgres", port_forwards=["5432:5432"])
k8s_resource("redis", port_forwards=["6379:6379"])
