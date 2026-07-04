# Phase 6: Dev Environment Migration

**Status:** Complete

**Goal:** Replace the minikube-based dev environment with Kind + Cilium +
Knative, and replace skaffold with Tilt. This is a prerequisite for the
side-by-side validation phase, since the new `rust/web` service will run as a
Knative Service in production.

### Kind cluster + Cilium

- [x] Write a Kind cluster config with the default CNI disabled
      (`networking.disableDefaultCNI: true`). → `k8s/kind-config.yaml`
- [x] Install Cilium as the CNI into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [x] Verify pod networking and DNS work correctly.

### Knative Serving

- [x] Install Knative Serving into the Kind cluster. (manual: run
      `scripts/setup-kind-cluster.sh`)
- [x] Configure Cilium as the Knative networking layer via `net-gateway-api`
      (Cilium's GatewayClass + Knative Gateway API ingress class). Setup
      automated in `scripts/setup-kind-cluster.sh`.
- [x] Verify a simple Knative Service deploys and is reachable.

### k8s manifests: rust/web as a Knative Service

- [x] Replace `k8s/base/web/deployment.yaml` and `k8s/base/web/service.yaml`
      with a Knative `Service` manifest (`serving.knative.dev/v1`).
- [x] Set `minScale: 1` - the monolith must not scale to zero.
- [x] Game microservices remain as plain Deployments for now.

### Tilt

- [x] Write a `Tiltfile` covering:
  - Deploy backing services (Postgres, Redis, SMTP, game microservices)
    to the Kind cluster via `k8s/dev-without-web` kustomize path.
  - Run `cargo leptos watch` as a local process for fast iteration (default
    hybrid mode).
  - Port-forwarding for Postgres (5432) and Redis (6379).
- [x] Full-cluster mode added (`WEB_IN_CLUSTER=1`): builds `brdgme/web` and
      deploys as Knative Service via `k8s/dev` kustomize path.
- [x] **Migrate CI to GitHub Actions**: `.github/workflows/ci.yml` created.
      Builds all service images and pushes to GHCR on master. `k8s/prod` image
      references updated to GHCR via kustomize `images` overlay.
- [x] Remove `skaffold.yaml` and `.travis.yml` once GitHub Actions CI is
      verified working on master.
- [x] **Local registry for Kind + Knative**: `registry:2` container +
      containerd mirror patch + `config-deployment` skip for `kind-registry:5000`
      + Tilt `default_registry`. Requires cluster recreation to take effect.

### Production builds

Production image builds and deploys are driven by GitHub Actions (see item
above). `kubectl apply -k k8s/prod` is still the deploy mechanism.

