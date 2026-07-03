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
GATEWAY_API_VERSION="1.2.0"

set -euo pipefail

# --- Kind cluster + local registry ---
echo "==> Applying ctlptl.yaml (Kind cluster + kind-registry)..."
ctlptl apply -f ctlptl.yaml

kubectl config use-context kind-kind

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
echo "==> Installing Cilium (CNI + Gateway API)..."
cilium install --set gatewayAPI.enabled=true

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
