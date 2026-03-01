#!/usr/bin/env bash
# Sets up a local Kind cluster with Cilium CNI, Knative Serving, and
# Cilium Gateway API as the Knative networking layer.
#
# Prerequisites: kind, helm, kubectl, docker installed (all provided via devenv.nix).
#
# Run once per workstation or after `kind delete cluster`.
#
# Version pins match official documentation:
#   Cilium:           https://docs.cilium.io/en/stable/installation/kind/
#   Gateway API CRDs: https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/gateway-api/
#   Knative Serving:  https://knative.dev/docs/install/yaml-install/serving/install-serving-with-yaml/
#   net-gateway-api:  https://github.com/knative-extensions/net-gateway-api/releases
CILIUM_VERSION="1.19.1"
GATEWAY_API_VERSION="v1.4.1"
KNATIVE_SERVING_VERSION="1.21.1"
NET_GATEWAY_API_VERSION="1.21.0"

set -euo pipefail

# --- Local registry ---
# Knative resolves image digests from the registry before scheduling pods, so
# Kind's native image loading (kind load docker-image) is not sufficient.
# A local registry reachable from within the cluster is required.
echo "==> Starting local registry (kind-registry:5000)..."
if [ "$(docker inspect -f '{{.State.Running}}' kind-registry 2>/dev/null)" != "true" ]; then
  docker run --detach --restart=always --name kind-registry \
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
    host: "kind-registry:5000"
    help: "https://kind.sigs.k8s.io/docs/user/local-registry/"
EOF

# --- Gateway API CRDs (required by Cilium Gateway API support) ---
# Version per https://docs.cilium.io/en/stable/network/servicemesh/gateway-api/gateway-api/
echo "==> Installing Gateway API CRDs ${GATEWAY_API_VERSION}..."
kubectl apply -f "https://github.com/kubernetes-sigs/gateway-api/releases/download/${GATEWAY_API_VERSION}/standard-install.yaml"

# --- Cilium ---
# kubeProxyReplacement=true is required for Gateway API support and because
# kube-proxy is disabled in kind-config.yaml. When kube-proxy is not running,
# Cilium cannot reach the API server via its ClusterIP (10.96.0.1) during
# bootstrap, so we point it directly at the control plane node IP.
# Ref: https://docs.cilium.io/en/stable/network/kubernetes/kubeproxy-free/
API_SERVER_IP=$(docker inspect kind-control-plane \
  --format='{{.NetworkSettings.Networks.kind.IPAddress}}')

echo "==> Preloading Cilium image ${CILIUM_VERSION}..."
docker pull "quay.io/cilium/cilium:v${CILIUM_VERSION}"
kind load docker-image "quay.io/cilium/cilium:v${CILIUM_VERSION}"

echo "==> Installing Cilium ${CILIUM_VERSION} (API server: ${API_SERVER_IP}:6443)..."
helm repo add cilium https://helm.cilium.io/ --force-update
helm install cilium cilium/cilium \
  --version "${CILIUM_VERSION}" \
  --namespace kube-system \
  --set image.pullPolicy=IfNotPresent \
  --set ipam.mode=kubernetes \
  --set kubeProxyReplacement=true \
  --set k8sServiceHost="${API_SERVER_IP}" \
  --set k8sServicePort=6443 \
  --set gatewayAPI.enabled=true

echo "==> Waiting for Cilium to be ready..."
kubectl -n kube-system rollout status daemonset/cilium --timeout=120s

# --- Knative Serving ---
echo "==> Installing Knative Serving ${KNATIVE_SERVING_VERSION}..."
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_SERVING_VERSION}/serving-crds.yaml"
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_SERVING_VERSION}/serving-core.yaml"

echo "==> Waiting for Knative Serving to be ready..."
kubectl -n knative-serving rollout status deployment/controller --timeout=120s
kubectl -n knative-serving rollout status deployment/webhook --timeout=120s

echo "==> Configuring DNS (sslip.io)..."
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_SERVING_VERSION}/serving-default-domain.yaml"

# Skip image digest resolution for the local registry. Knative's controller
# resolves tags to digests via HTTP before scheduling pods. For a plain-HTTP
# local registry the controller would fail without this exemption.
echo "==> Configuring Knative to skip digest resolution for kind-registry:5000..."
kubectl patch configmap/config-deployment \
  -n knative-serving \
  --type merge \
  --patch '{"data":{"registries-skipping-tag-resolving":"kind.local,ko.local,dev.local,kind-registry:5000"}}'

# --- net-gateway-api (Cilium as Knative networking layer) ---
echo "==> Installing net-gateway-api ${NET_GATEWAY_API_VERSION}..."
kubectl apply -f "https://github.com/knative-extensions/net-gateway-api/releases/download/knative-v${NET_GATEWAY_API_VERSION}/release.yaml"

# Create a Gateway for Knative to use (backed by Cilium's GatewayClass).
echo "==> Creating Knative gateways using Cilium GatewayClass..."
kubectl apply -f - <<'EOF'
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: knative-ingress-gateway
  namespace: knative-serving
spec:
  gatewayClassName: cilium
  listeners:
    - name: http
      port: 80
      protocol: HTTP
      allowedRoutes:
        namespaces:
          from: All
---
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: knative-local-gateway
  namespace: knative-serving
spec:
  gatewayClassName: cilium
  listeners:
    - name: http
      port: 80
      protocol: HTTP
      allowedRoutes:
        namespaces:
          from: All
EOF

# Configure Knative to use the gateway-api ingress class and the Cilium gateways.
kubectl patch configmap/config-network \
  --namespace knative-serving \
  --type merge \
  --patch '{"data":{"ingress-class":"gateway-api.ingress.knative.dev"}}'

kubectl patch configmap/config-gateway \
  --namespace knative-serving \
  --type merge \
  --patch '{
    "data": {
      "external-gateways": "- class: cilium\n  gateway: knative-serving/knative-ingress-gateway\n",
      "local-gateways": "- class: cilium\n  gateway: knative-serving/knative-local-gateway\n"
    }
  }'

echo ""
echo "==> Cluster ready. Run 'tilt up' to start the dev environment."
