# Session Memory

## Project Overview
brdgme: lo-fi multiplayer board gaming platform, 10+ years old, real users.
Play via web or email, ASCII rendering, text commands, bot support.
All open source, always.

## Repo Structure
- `rust/web` - new Axum+Leptos monolith (defective, `leptos` branch)
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
- `docs/REVIEW.md` - comprehensive parity review of rust/web vs old system (complete)
- `docs/adr/` - empty, ready for ADRs

## Target Architecture (agreed)
- **Platform**: DigitalOcean Kubernetes, Sydney (SYD1)
- **Dev cluster**: Kind + Cilium + Knative (replacing minikube + skaffold)
- **Dev tooling**: Tilt (replacing skaffold)
- **CNI**: Cilium
- **Always-on core**: rust/web as Knative Service (minScale: 1, not scale-to-zero)
- **WebSocket fan-out**: NATS Core in-cluster (replaces tokio::sync::broadcast)
- **Game services**: plain Deployments now, Knative Serving long-term
- **Operator**: kube-rs operator watching GameType CRDs (post-cutover)
- **Database**: PostgreSQL
- **Ingress**: Cilium Gateway API + single load balancer
- **No Redis** (replaced by NATS), **No Node.js**, **No Rocket** (after decommission)
- NATS Core→JetStream upgrade path: single config flag, needs volume for persistence

## Image Naming
- `brdgme/web` - new Leptos SSR app (rust/Dockerfile, web target)
- `brdgme/web-legacy` - old React frontend (web/Dockerfile, renamed)
- Both share PostgreSQL and game microservices during side-by-side validation

## Production Status
- Production NOT yet migrated - old system still running in prod
- prod kustomization currently resolves to k8s/base/brdgme (which has dropped api/websocket)
- Must restore legacy services to prod kustomization before any production deploy

## Plan Status (docs/PLAN.md)
- Phases 1-4: Complete
- Phase 5: Defective (login broken, auth broken, stubs throughout - see Phase 5 notes)
- Phase 5.5: Dev environment migration [Mostly Complete] - manifests/scripts/Tiltfile done; manual cluster verify still needed
- Phase 5.6: Pre-cutover fixes (13 blockers including email sending, + parity gaps) [Next]
- Phase 6: NATS integration (replaces tokio::sync::broadcast, unblocks Redis removal)
- Phase 7: Side-by-Side validation (old + new live together, then decommission)

## REVIEW.md Status (COMPLETE)
38 items: 12 blockers, 26 known gaps.
All items extracted into PLAN.md Phase 5.6.
Item 38: Token parser false-positive autocomplete - add CommandSpec::suggest().

## Key Decisions
- Old system kept alive (side-by-side) until rust/web proven in prod, then decommissioned
- Tilt replaces skaffold for all local dev
- rust/web deployed as Knative Service (not plain Deployment), minScale: 1
- NATS in scope as Phase 6 (not post-cutover)
- Email sending is a blocker for cutover (existing functionality, not new)
- web/Dockerfile pins node:14.7.0 (EOL) - noted as risk in Phase 7
- Production builds: docker build/push + kubectl apply -k k8s/prod (no Tilt/skaffold)

## Long-term (out of scope now)
- **Email**: third-party provider (Mailgun/Postmark), inbound via webhook→Knative Service
- **Bots**: LLM-based, Knative-invoked
- **Knative for game services**: currently plain Deployments, Knative migration deferred
- **kube-rs operator**: post-cutover
- **No Kafka/RabbitMQ** (NATS if ever needed)
