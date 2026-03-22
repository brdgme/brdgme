# Session Memory

## Project
brdgme: lo-fi multiplayer board gaming platform, 10+ years old, real users.
ASCII rendering, text commands, play via web or email. Always open source.
Active branch: `leptos` - Rust/Leptos rewrite of all frontend + backend.

## Docs
- `VISION.md` - timeless goals and principles
- `ARCHITECTURE.md` - system design, components, JSON contract, DB schema
- `PLAN.md` - phase-by-phase migration plan, source of truth for next tasks
- `SCRATCH.md` - current session notes and open issues
- `DEV.md` - setup, daily workflow, SQLx, Rust conventions, gotchas

## Repo Structure
- `rust/web` - Axum + Leptos monolith (the new system)
- `rust/operator` - kube-rs operator watching `GameVersion` CRDs
- `rust/api` - old Rocket API (kept for side-by-side, decommission after cutover)
- `web` - old React frontend (`brdgme/web-legacy`, decommission after cutover)
- `websocket` - old Node.js WebSocket service (decommission after cutover)
- `brdgme-go` - Go game implementations (~17 games)
- `rust/game` - Rust game implementations (Acquire, Lords of Vegas, Lost Cities x2)
- `rust/lib` - shared Rust libs: `brdgme_cmd`, `brdgme_game`, `brdgme_color`, `brdgme_markup`
- `k8s/` - Kubernetes manifests (kustomize)

## Plan Status
- Phases 1-5.5: complete
- Phase 5.6: functionally complete - one open bug: restart 500 (diagnostics
  added 2026-03-22, raw response will be captured on next live restart attempt)
- Bug fixes (2026-03-22 session 3): recent logs is_new logic, width layout,
  timestamps, scroll-to-bottom, page flash, undo log markup, immediate re-fetch
  after actions. See PLAN.md "Bug fixes" section and SCRATCH.md.
- Phase 6: Redis pub/sub (replaces tokio broadcast, required for multi-replica + side-by-side)
- Phase 6.5: ArgoCD production CD
- Phase 7: side-by-side validation then legacy decommission

## Production Status
Production NOT migrated - old system still running. Do not deploy `leptos`
branch to prod without restoring legacy services to the prod kustomization.

## Critical Non-Obvious Constraints

**Rust:** All crates use edition `2024`. New operator-style crates need
`kube = "3"`, `k8s-openapi = { version = "0.27", features = ["latest"] }`,
`schemars = "1"` (kube 3.x is incompatible with schemars 0.8).

**SQLx:** Queries need cached metadata in `rust/web/.sqlx/`. After any query
change: run `sqlx migrate run` then `cargo sqlx prepare -- --features ssr`
from `rust/web/`. The operator uses dynamic queries (no metadata needed).

**Hybrid dev networking:** Local web server cannot resolve `*.svc.cluster.local`.
mirrord wraps `cargo leptos watch` in the Tiltfile targeting `pod/postgres-0`.
On NixOS, `/etc/hosts` is read-only - kubefwd is not viable.

**CRD startup:** The `crd-ready` Tilt resource uses `kubectl wait --for=condition=established`
to gate the operator. Without this the operator fails with "event queue error"
on startup while the API server registers the CRD.

**GameVersion CRs:** One CR per deployed game service version. `is_deprecated: true`
keeps the service running for in-progress games but blocks new game creation.
`lost-cities-1` is deprecated; `lost-cities-2` is current.

**Sessions:** `tower-sessions-sqlx-store` with PostgreSQL. Sessions are
persistent across restarts. `SECURE_COOKIE=true` must be set in production.

**WebSocket:** Currently uses `tokio::sync::broadcast` - single replica only.
NATS replaces this in Phase 6.
