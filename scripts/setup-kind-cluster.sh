#!/usr/bin/env bash
# Sets up a local Kind cluster with Knative Serving and Kourier ingress.
#
# Prerequisites: kind, kubectl, docker (all provided via devenv.nix).
#
# IDEMPOTENT: safe to re-run against an existing cluster.
# Exception: Kind cluster config changes (kind-config.yaml) require a manual
# `kind delete cluster` followed by re-running this script.
#
# Version pins:
#   Knative Serving: https://github.com/knative/serving/releases
#   Kourier:         https://github.com/knative-extensions/net-kourier/releases
KNATIVE_VERSION="1.21.1"
KOURIER_VERSION="1.21.0"

set -euo pipefail


# --- Local registry ---
# Knative resolves image digests from the registry before scheduling pods, so
# kind load docker-image is insufficient. A registry reachable from within the
# cluster is required.
echo "==> Starting local registry (kind-registry:5000)..."
if [ "$(docker inspect -f '{{.State.Running}}' kind-registry 2>/dev/null)" == "true" ]; then
  echo "    Registry already running, skipping."
elif docker inspect kind-registry &>/dev/null; then
  docker start kind-registry
else
  docker run --detach --restart=no --name kind-registry \
    --publish "127.0.0.1:5000:5000" registry:2
fi

# --- Kind cluster ---
echo "==> Creating Kind cluster..."
if ! kind get clusters 2>/dev/null | grep -q '^kind$'; then
  kind create cluster --config k8s/kind-config.yaml
else
  echo "    Cluster already exists, skipping creation."
  if [ "$(docker inspect -f '{{.State.Running}}' kind-control-plane 2>/dev/null)" != "true" ]; then
    echo "    Control plane container is stopped, starting..."
    docker start kind-control-plane
    echo "    Waiting for API server to become ready..."
    until kubectl --context kind-kind cluster-info &>/dev/null; do
      sleep 2
    done
  fi
  kubectl config use-context kind-kind
fi

# Disable auto-restart so Kind does not start on boot. The cluster should only
# run when explicitly started for development (via `docker start kind-control-plane`
# or `tilt up` after starting the cluster manually).
echo "==> Setting Kind cluster restart policy to 'no'..."
docker update --restart=no kind-control-plane

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
# additional time to register its endpoint with the API server.
echo "==> Waiting for Knative webhook endpoint to register..."
until kubectl -n knative-serving get endpoints webhook \
      -o jsonpath='{.subsets[*].addresses[*].ip}' 2>/dev/null | grep -qE '[0-9]+'; do
    sleep 2
done

# --- Kourier ingress ---
echo "==> Installing Kourier ${KOURIER_VERSION}..."
kubectl apply -f "https://github.com/knative-extensions/net-kourier/releases/download/knative-v${KOURIER_VERSION}/kourier.yaml"

echo "==> Waiting for Kourier to be ready..."
kubectl -n knative-serving rollout status deployment/net-kourier-controller --timeout=120s
kubectl -n kourier-system rollout status deployment/3scale-kourier-gateway --timeout=120s

# Kourier creates a LoadBalancer service which stays <pending> in Kind.
# Patch it to NodePort 31080, which maps to host port 8080 via
# extraPortMappings in kind-config.yaml.
echo "==> Patching Kourier service to NodePort 31080..."
kubectl patch svc kourier \
  -n kourier-system \
  --type merge \
  --patch '{"spec":{"type":"NodePort","ports":[{"name":"http2","port":80,"protocol":"TCP","targetPort":8080,"nodePort":31080},{"name":"https","port":443,"protocol":"TCP","targetPort":8443,"nodePort":31443}]}}'

retry kubectl patch configmap/config-network \
  --namespace knative-serving \
  --type merge \
  --patch '{"data":{"ingress-class":"kourier.ingress.networking.knative.dev"}}'

# Skip image digest resolution for the local registry.
echo "==> Configuring Knative to skip digest resolution for kind-registry:5000..."
retry kubectl patch configmap/config-deployment \
  -n knative-serving \
  --type merge \
  --patch '{"data":{"registries-skipping-tag-resolving":"kind.local,ko.local,dev.local,kind-registry:5000"}}'

# --- Knative domain ---
# Use lvh.me as the base domain - *.lvh.me resolves to 127.0.0.1, so
# services are reachable at {service}.{namespace}.lvh.me:8080 without
# any /etc/hosts changes (useful on NixOS where /etc/hosts is read-only).
echo "==> Configuring Knative domain (lvh.me)..."
retry kubectl patch configmap/config-domain \
  --namespace knative-serving \
  --type merge \
  --patch '{"data":{"lvh.me":"","example.com":null}}'

# --- brdgme CRD ---
# Installed here rather than managed by Tilt to avoid a deadlock: Tilt would
# need to delete the CRD before re-applying it, but the CRD cannot be deleted
# while GameVersion custom resources have operator finalizers on them, and the
# operator only runs after Tilt has finished applying manifests.
echo "==> Installing brdgme GameVersion CRD..."
kubectl apply -f k8s/base/operator/crd.yaml

echo ""
echo "==> Cluster ready. Run 'tilt up' to start the dev environment."
echo "    Legacy services available at http://{service}.brdgme.lvh.me:8080"
