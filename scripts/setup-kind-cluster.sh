#!/usr/bin/env bash
# Sets up a local Kind cluster with Knative Serving and Kourier ingress.
#
# Prerequisites: kind, kubectl, docker (all provided via devenv.nix).
#
# IDEMPOTENT: this script must remain safe to run against an existing cluster.
# All steps use guards or commands that are no-ops when already applied.
# Exception: Kind has no "apply" equivalent for cluster config - changes to
# k8s/kind-config.yaml (node count, port mappings, CNI settings, etc.) require
# a manual `kind delete cluster` followed by re-running this script. There is
# no way to reconcile Kind cluster config in-place.
#
# Version pins:
#   Serving:  https://github.com/knative/serving/releases
#   Kourier:  https://github.com/knative-extensions/net-kourier/releases
#   (Kourier moved to knative-extensions org and does not publish patch releases)
KNATIVE_VERSION="1.21.1"
KOURIER_VERSION="1.21.0"

set -euo pipefail

# --- Local registry ---
# Knative resolves image digests from the registry before scheduling pods, so
# kind load docker-image is insufficient. A registry reachable from within the
# cluster is required.
echo "==> Starting local registry (kind-registry:5000)..."
if [ "$(docker inspect -f '{{.State.Running}}' kind-registry 2>/dev/null)" != "true" ]; then
  docker run --detach --restart=no --name kind-registry \
    --publish "127.0.0.1:5000:5000" registry:2
fi

# --- Kind cluster ---
echo "==> Creating Kind cluster..."
if ! kind get clusters 2>/dev/null | grep -q '^kind$'; then
  kind create cluster --config k8s/kind-config.yaml
else
  echo "    Cluster already exists, skipping creation."
  kubectl config use-context kind-kind
fi

# Connect registry to Kind network so pods can reach it as "kind-registry:5000".
echo "==> Connecting registry to Kind network..."
docker network connect kind kind-registry 2>/dev/null || true

# KEP-1755: document the local registry so tools can discover it.
kubectl apply -f - <<'EOF'
apiVersion: v1
kind: ConfigMap
metadata:
  name: local-registry-hosting
  namespace: kube-public
data:
  localRegistryHosting.v1: |
    host: "localhost:5000"
    hostFromContainerRuntime: "kind-registry:5000"
    help: "https://kind.sigs.k8s.io/docs/user/local-registry/"
EOF

# --- Knative Serving ---
echo "==> Installing Knative Serving ${KNATIVE_VERSION}..."
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_VERSION}/serving-crds.yaml"
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_VERSION}/serving-core.yaml"

echo "==> Waiting for Knative Serving to be ready..."
kubectl -n knative-serving rollout status deployment/controller --timeout=120s
kubectl -n knative-serving rollout status deployment/webhook --timeout=120s

# rollout status returns as soon as the pod is ready, but the webhook needs
# additional time to register its endpoint with the API server. Poll until
# the webhook service endpoint has a ready address before proceeding.
echo "==> Waiting for Knative webhook endpoint to register..."
until kubectl -n knative-serving get endpoints webhook \
      -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | grep -qE '[0-9]+'; do
    sleep 2
done

# --- Kourier ingress ---
echo "==> Installing Kourier ${KOURIER_VERSION}..."
kubectl apply -f "https://github.com/knative-extensions/net-kourier/releases/download/knative-v${KOURIER_VERSION}/kourier.yaml"
kubectl patch configmap/config-network \
  --namespace knative-serving \
  --type merge \
  --patch '{"data":{"ingress-class":"kourier.ingress.networking.knative.dev"}}'

# --- DNS ---
echo "==> Configuring DNS (sslip.io)..."
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_VERSION}/serving-default-domain.yaml"

# Skip image digest resolution for the local registry. Knative's controller
# resolves tags to digests via HTTP before scheduling pods; plain-HTTP
# registries require this exemption.
echo "==> Configuring Knative to skip digest resolution for kind-registry:5000..."
kubectl patch configmap/config-deployment \
  -n knative-serving \
  --type merge \
  --patch '{"data":{"registries-skipping-tag-resolving":"kind.local,ko.local,dev.local,kind-registry:5000"}}'

echo ""
echo "==> Cluster ready. Run 'tilt up' to start the dev environment."
