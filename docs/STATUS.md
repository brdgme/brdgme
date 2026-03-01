# Current Work Status

## Phase 5.5: COMPLETE

## This session: Legacy side-by-side dev environment

### What was done

**1. Legacy services added to Tiltfile (`LEGACY=1 tilt up`)**

New `k8s/base/web-legacy/` manifests:
- `service.yaml` - Knative Service (minScale: 1), nginx container with ConfigMap volume
- `nginx-configmap.yaml` - proxies `/api/` → `http://api/` and `/ws` → `http://websocket/`
- `kustomization.yaml`

`k8s/base/api/` converted to Knative Service:
- Removed `deployment.yaml`, replaced `service.yaml` with `serving.knative.dev/v1` Service
- Retains `envFrom: secretRef: postgres-config` and `ROCKET_ADDRESS: 0.0.0.0`
- `containerPort: 8000`, `minScale: 1`

`k8s/base/websocket/` converted to Knative Service:
- Removed `deployment.yaml`, replaced `service.yaml` with Knative Service
- `REDIS_URL: redis://redis` env var
- `containerPort: 80`, `minScale: 1`

`k8s/dev-legacy/kustomization.yaml` created - groups all three with `namespace: brdgme`.

Tiltfile:
- `LEGACY = os.getenv("LEGACY", "") == "1"` env var
- `k8s_kind('Service', api_version='serving.knative.dev/v1', ...)` - required so Tilt
  recognises Knative Services as workloads (otherwise `k8s_resource` call errors)
- `docker_build` for web-legacy, websocket, api when LEGACY=1
- `k8s_resource("web-legacy", port_forwards=["3001:80"])`

**2. `rust/api/Dockerfile` written** (new file)
- Proper multi-stage build: `rust:slim-bookworm` builder, `debian:bookworm-slim` runtime
- Replaces `rust/api/deploy/Dockerfile` which required a pre-built binary
- CI `build-legacy` job updated to use `rust/api/Dockerfile`

**3. `web/Dockerfile` fixed for Node 22**
- `node:14.7.0` → `node:22` (EOL risk materialised - npm packages require >=18)
- `webpack -p` → `webpack --mode production` (`-p` removed in webpack-cli v4)
- `nginx:1.19.1` → `nginx:stable`
- `web/package-lock.json` regenerated locally with npm 10

**4. `acquire-1` test fix (CI)**
- `board.rs:311-314`: `Loc::default().into()` ambiguous due to
  `serde_json` adding `impl PartialEq<Value> for usize`
- Fixed: `.into()` → `usize::from(...)` (explicit target type)

---

## Local registry for Kind + Knative: RESOLVED

Approach confirmed correct (officially recommended by Kind docs). Three components work together:

1. **`k8s/kind-config.yaml`** - `containerdConfigPatches`: tells Kind nodes' containerd to use HTTP for `kind-registry:5000`
2. **`scripts/setup-kind-cluster.sh`**: starts `registry:2` container, connects it to Kind network, creates `local-registry-hosting` ConfigMap (KEP-1755), patches `config-deployment` to skip digest resolution for `kind-registry:5000`
3. **`Tiltfile`**: `default_registry('localhost:5000', host_from_cluster='kind-registry:5000')`

The missing piece was the `config-deployment` patch - Knative's controller makes its own HTTP calls to resolve image digests and would fail on a plain-HTTP registry without this exemption.

### To test (cluster recreation required for containerd patch)
```bash
kind delete cluster
bash scripts/setup-kind-cluster.sh
# inside rust/web:
sqlx migrate run
# then:
LEGACY=1 tilt up
```

---

## Next major task after local registry is resolved: Phase 5.6

See `docs/PLAN.md` Phase 5.6. Recommended order for blockers:
1. Persistent session store (`tower-sessions-sqlx-store`)
2. Login UI wired to server functions
3. Confirmation token removed from response
4. `with_secure` env-driven
5. Token expiry (30-day)
6. Email sending (SMTP)
7. Auth in Axum handlers (replace `Uuid::nil()`)
8. Authenticate `GET /api/game/{id}`
9. Turn enforcement
10. `GamePlayer` model missing fields
11. `update_game_command_success` writes all fields
12. `find_game_extended` LEFT JOIN for missing `game_type_users`
13. Graceful SIGTERM shutdown
