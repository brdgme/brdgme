# Session Memory

## Project Overview
brdgme: lo-fi multiplayer board gaming platform, 10+ years old, real users.
Play via web or email, ASCII rendering, text commands, bot support.
All open source, always.

## Repo Structure
- `rust/web` - new Axum+Leptos monolith (`leptos` branch)
- `rust/operator` - kube-rs operator watching `GameVersion` CRDs
- `rust/api` - old Rocket API (kept alive for side-by-side validation)
- `web` - old React/Redux/Webpack frontend (kept alive, renamed brdgme/web-legacy)
- `websocket` - old Node.js WebSocket service (kept alive for side-by-side)
- `brdgme-go` - Go game implementations (~20 games)
- `rust/game` - Rust game implementations (Acquire, Lords of Vegas, Lost Cities x2)
- `rust/lib` - shared Rust libraries (brdgme_cmd, brdgme_game, brdgme_color, brdgme_markup)
- `k8s/` - Kubernetes manifests
- `docs/` - VISION.md, ARCHITECTURE.md, PLAN.md, REVIEW.md, MEMORY.md

## Docs Structure
- `docs/VISION.md` - timeless goals, no status
- `docs/ARCHITECTURE.md` - target arch + stable game JSON contract + Mermaid diagrams
- `docs/PLAN.md` - migration phases, current source of truth for what to work on next
- `docs/adr/` - empty, ready for ADRs

## Target Architecture (agreed)
- **Platform**: DigitalOcean Kubernetes, Sydney (SYD1)
- **Dev cluster**: Kind + Knative/Kourier (replacing minikube + skaffold; Cilium removed)
- **Dev tooling**: Tilt (replacing skaffold); mirrord for local web → cluster DNS access
- **Always-on core**: rust/web as Knative Service (minScale: 1, not scale-to-zero)
- **WebSocket fan-out**: NATS Core in-cluster (replaces tokio::sync::broadcast)
- **Game services**: plain Deployments now, Knative Serving long-term
- **Operator**: kube-rs operator watching `GameVersion` CRDs, running as Tilt local_resource
- **Database**: PostgreSQL
- **Ingress**: Kourier (Knative default) for dev; production ingress TBD
- **No Redis** (replaced by NATS), **No Node.js**, **No Rocket** (after decommission)
- NATS Core→JetStream upgrade path: single config flag, needs volume for persistence

## Operator (`rust/operator`)
- Watches `GameVersion` CRDs (`gameversions.brdgme.com/v1`)
- Each CR represents one deployed game version (e.g. `acquire-1`, `lost-cities-2`)
- Upserts `game_types` and `game_versions` rows in PostgreSQL on reconcile
- Uses finalizers (`brdgme.com/game-version`) to set `is_public = false` on deletion
- `is_deprecated: true` on a CR means the version is kept running for in-progress games
  but excluded from new game creation (e.g. `lost-cities-1`)
- `GameVersion` CR YAML files live alongside each game: `k8s/base/game/{name}/game-version.yaml`
- CRD and RBAC in `k8s/base/operator/`
- Runs locally via Tilt with `RUST_LOG=info`; `crd-ready` resource gates startup

## Image Naming
- `brdgme/web` - new Leptos SSR app (rust/Dockerfile, web target)
- `brdgme/web-legacy` - old React frontend (web/Dockerfile, renamed)
- Both share PostgreSQL and game microservices during side-by-side validation

## Production Status
- Production NOT yet migrated - old system still running in prod
- Must restore legacy services to prod kustomization before any production deploy

## Plan Status (docs/PLAN.md)
- Phases 1-5.5: Complete
- Phase 5.6: In progress - all 13 blockers done, all 4 missing endpoints done,
  new-game UI done, operator done; frontend gaps (game logs, action buttons,
  whose-turn display, etc.) and several code quality items remain
- Phase 6: NATS integration (replaces tokio::sync::broadcast, unblocks Redis removal)
- Phase 6.5: Production CD - ArgoCD + separate brdgme-config repo for image tags
- Phase 7: Side-by-side validation (old + new live together, then decommission)

## Key Decisions
- Old system kept alive (side-by-side) until rust/web proven in prod, then decommissioned
- Tilt replaces skaffold for all local dev; skaffold.yaml/.travis.yml deleted in Phase 5.5
- All services (rust/web + legacy api/websocket/web-legacy) deployed as Knative Services, minScale: 1
- `LEGACY=1 tilt up` enables side-by-side dev (old React at localhost:3001)
- NATS in scope as Phase 6 (not post-cutover)
- web/Dockerfile bumped to node:22 (was node:14.7.0 EOL); webpack -p → --mode production
- rust/api/Dockerfile: new proper multi-stage build replacing binary-only deploy artifact
- Production CD: GitHub Actions (CI/build/push to GHCR) + ArgoCD watching separate brdgme-config repo
- ArgoCD Image Updater rejected - rollback override bug (issue #1249); separate config repo used instead
- CI: .github/workflows/ci.yml - tests on all branches, image push to GHCR on master only
- SQLx offline metadata in `rust/web/.sqlx/` - run `cargo sqlx prepare -- --features ssr`
  from `rust/web/` after any query change; verify with `SQLX_OFFLINE=true cargo check --features ssr`
- mirrord used in Tilt hybrid mode: wraps `cargo leptos watch` to give the local web server
  cluster DNS access (NixOS-compatible; no /etc/hosts modification, no root required)
- Operator CRD kind is `GameVersion` (not `GameType`) - one CR per deployed game service version
- `secret_settings(disable_scrub=True)` in Tiltfile: prevents "brdgme" from being redacted in logs

## Long-term (out of scope now)
- **Email**: third-party provider (Mailgun/Postmark), inbound via webhook→Knative Service
- **Bots**: LLM-based, Knative-invoked
- **Knative for game services**: currently plain Deployments, Knative migration deferred
- **No Kafka/RabbitMQ** (NATS if ever needed)
