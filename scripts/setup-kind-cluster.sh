#!/usr/bin/env bash
# Sets up a local Kind cluster with Cilium (CNI + Gateway API).
#
# Prerequisites: ctlptl, cilium-cli, kubectl, docker (all provided via
# devenv.nix).
#
# IDEMPOTENT: safe to re-run against an existing cluster.
# Exception: Kind cluster config changes (ctlptl.yaml) require a manual
# `kind delete cluster` followed by re-running this script.
#
# Version pins:
#   Gateway API CRDs: https://github.com/kubernetes-sigs/gateway-api/releases
#   Must be >= what Cilium's cilium-operator vendors (check its go.mod for
#   sigs.k8s.io/gateway-api). Cilium 1.18.5 vendors ~v1.3.1-dev; installing
#   older CRDs (e.g. v1.2.0) makes cilium-operator fail with
#   "no kind is registered for the type v1.Gateway in scheme" and the
#   Gateway/GatewayClass never leave "Waiting for controller".
GATEWAY_API_VERSION="1.3.0"

set -euo pipefail

# --- Kind cluster + local registry ---
echo "==> Applying ctlptl.yaml (Kind cluster + kind-registry)..."
ctlptl apply -f ctlptl.yaml

kubectl config use-context kind-kind

# --- Registry access via containerd certs.d (hosts.toml) ---
# The node image's containerd already sets `config_path`, and containerd
# rejects config that also sets `registry.mirrors` alongside it - so registry
# access is configured per kind's local-registry docs instead:
# https://kind.sigs.k8s.io/docs/user/local-registry/
echo "==> Configuring containerd registry access for kind-registry:5000..."
REGISTRY_DIR="/etc/containerd/certs.d/kind-registry:5000"
for node in $(kind get nodes --name kind); do
  docker exec "${node}" mkdir -p "${REGISTRY_DIR}"
  cat <<EOF | docker exec -i "${node}" cp /dev/stdin "${REGISTRY_DIR}/hosts.toml"
[host."http://kind-registry:5000"]
EOF
done

# Disable auto-restart so Kind does not start on boot. The cluster should only
# run when explicitly started for development (via `docker start kind-control-plane`
# or `tilt up` after starting the cluster manually).
echo "==> Setting Kind cluster restart policy to 'no'..."
docker update --restart=no kind-control-plane

# --- Gateway API CRDs ---
# Cilium's Helm chart does not install these; they must exist before Cilium
# is installed with gatewayAPI.enabled=true.
echo "==> Installing Gateway API CRDs ${GATEWAY_API_VERSION}..."
kubectl apply -f "https://github.com/kubernetes-sigs/gateway-api/releases/download/v${GATEWAY_API_VERSION}/standard-install.yaml"

# --- Cilium (CNI + Gateway API) ---
# nodePort.enabled=true: Cilium's Gateway API controller refuses to reconcile
# without either kube-proxy-replacement or node-port support enabled ("Gateway
# API support requires either kube-proxy-replacement or enable-node-port
# enabled"), and this Kind cluster keeps kube-proxy running rather than doing
# a full kube-proxy replacement.
# gatewayAPI.secretsNamespace.sync=false: the Gateway TLS-secret-sync watcher
# hits the same "no kind is registered for the type v1.Gateway in scheme"
# failure as the unfixed CRD version above, independent of the CRD/node-port
# fixes. Dev Gateways here are HTTP-only (no TLS listeners), so secret sync is
# unused - disable it rather than debug further.
echo "==> Installing Cilium (CNI + Gateway API)..."
cilium install \
  --set gatewayAPI.enabled=true \
  --set nodePort.enabled=true \
  --set gatewayAPI.secretsNamespace.sync=false

echo "==> Waiting for Cilium to be ready..."
cilium status --wait

# --- brdgme CRD ---
# Installed here rather than managed by Tilt to avoid a deadlock: Tilt would
# need to delete the CRD before re-applying it, but the CRD cannot be deleted
# while GameVersion custom resources have operator finalizers on them, and the
# operator only runs after Tilt has finished applying manifests.
echo "==> Installing brdgme GameVersion CRD..."
kubectl apply -f k8s/base/operator/crd.yaml

echo ""
echo "==> Cluster ready. Run 'tilt up' to start the dev environment."
