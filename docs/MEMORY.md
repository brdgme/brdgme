# Session Memory

## Project
brdgme: lo-fi multiplayer board gaming platform, 10+ years old, real users.
ASCII rendering, text commands, play via web or email. Always open source.
Active branch: `leptos` - Rust/Leptos rewrite of all frontend + backend.

## Docs
- `VISION.md` - timeless goals and principles
- `ARCHITECTURE.md` - system design, components, JSON contract, DB schema
- `PLAN.md` - phase-by-phase migration plan, source of truth for next tasks
- `SCRATCH.md` - deleted; no longer used
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
- Phase 6: complete (2026-03-22 sessions 4-5):
  - Redis pub/sub replaces tokio broadcast
  - Server publishes full legacy-format ShowResponse to `game.{id}` and
    `user.{token_id}` for web-legacy React compat
  - Server publishes `BrdgmeUpdate` (GameViewData + logs) to `ws.{user_id}`
    for rust/web Leptos direct signal updates (no re-fetch)
  - `/ws` handler is session-aware; subscribes to `game.*` + `ws.{user_id}`
  - `GameRestarted` WS event removed - regular BrdgmeUpdate for old game
    carries `restarted_game_id` (same as legacy always did)
- Phase 6.5: ArgoCD production CD (next)
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

**WebSocket:** Uses Redis pub/sub (Phase 6 complete). Three channels:
- `game.{id}` - legacy ShowResponse format (public, for web-legacy compat)
- `user.{auth_token_id}` - legacy ShowResponse with private data (for web-legacy)
- `ws.{user_id}` - `BrdgmeUpdate` (GameViewData + logs) for rust/web Leptos
`/ws` handler is session-aware; subscribes to `game.*` + `ws.{user_id}`.
Multi-replica safe. NATS replaces Redis in Phase 8 (after legacy decommission).
