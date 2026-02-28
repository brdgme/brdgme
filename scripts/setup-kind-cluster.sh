#!/usr/bin/env bash
# Sets up a local Kind cluster with Cilium CNI, Knative Serving, and
# Cilium Gateway API as the Knative networking layer.
#
# Prerequisites: kind, helm, kubectl installed (all provided via devenv.nix).
#
# Run once per workstation or after `kind delete cluster`.
#
# Version pins - check for newer releases before running:
#   Cilium:           https://github.com/cilium/cilium/releases
#   Knative Serving:  https://github.com/knative/serving/releases
#   net-gateway-api:  https://github.com/knative-extensions/net-gateway-api/releases
#   Gateway API CRDs: https://github.com/kubernetes-sigs/gateway-api/releases
CILIUM_VERSION="1.16.0"
KNATIVE_VERSION="1.16.0"

set -euo pipefail

# --- Kind cluster ---
echo "==> Creating Kind cluster..."
kind create cluster --config k8s/kind-config.yaml

# --- Gateway API CRDs (required by Cilium Gateway API support) ---
echo "==> Installing Gateway API CRDs..."
kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.2.0/standard-install.yaml

# --- Cilium ---
# kubeProxyReplacement=true means kube-proxy is not running, so Cilium cannot
# reach the API server via its ClusterIP (10.96.0.1) during bootstrap. We
# resolve this by pointing Cilium directly at the control plane node IP.
API_SERVER_IP=$(docker inspect kind-control-plane \
  --format='{{.NetworkSettings.Networks.kind.IPAddress}}')

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
echo "==> Installing Knative Serving ${KNATIVE_VERSION}..."
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_VERSION}/serving-crds.yaml"
kubectl apply -f "https://github.com/knative/serving/releases/download/knative-v${KNATIVE_VERSION}/serving-core.yaml"

echo "==> Waiting for Knative Serving to be ready..."
kubectl -n knative-serving rollout status deployment/controller --timeout=120s
kubectl -n knative-serving rollout status deployment/webhook --timeout=120s

# --- net-gateway-api (Cilium as Knative networking layer) ---
echo "==> Installing net-gateway-api ${KNATIVE_VERSION}..."
kubectl apply -f "https://github.com/knative-extensions/net-gateway-api/releases/download/knative-v${KNATIVE_VERSION}/release.yaml"

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
apiVersion: gateway.networking.k8s.io/v1beta1
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
      "external-gateways": "knative-serving/knative-ingress-gateway",
      "local-gateways": "knative-serving/knative-local-gateway"
    }
  }'

echo ""
echo "==> Cluster ready. Run 'tilt up' to start the dev environment."
