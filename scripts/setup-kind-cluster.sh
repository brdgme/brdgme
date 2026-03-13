#!/usr/bin/env bash
# Sets up a local Kind cluster with Knative Serving and Kourier ingress.
#
# Prerequisites: kind, kubectl, docker (all provided via devenv.nix).
#
# Run once per workstation or after `kind delete cluster`.
#
# Version pin: https://github.com/knative/serving/releases
KNATIVE_VERSION="1.21.1"

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
kind create cluster --config k8s/kind-config.yaml

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

# --- Kourier ingress ---
echo "==> Installing Kourier ${KNATIVE_VERSION}..."
kubectl apply -f "https://github.com/knative/net-kourier/releases/download/knative-v${KNATIVE_VERSION}/kourier.yaml"
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
