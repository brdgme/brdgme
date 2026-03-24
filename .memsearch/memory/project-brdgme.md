# brdgme Project

## Overview

brdgme is a lo-fi multiplayer board gaming platform, 10+ years old, with real users. ASCII rendering, text commands, play via web or email. Always open source.

Active branch: `leptos` - Rust/Leptos rewrite of all frontend + backend.

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
- `docs/` - project documentation

## Documentation Files

- `docs/VISION.md` - timeless goals and principles
- `docs/ARCHITECTURE.md` - system design, components, JSON contract, DB schema
- `docs/PLAN.md` - phase-by-phase migration plan, source of truth for next tasks
- `docs/DEV.md` - setup, daily workflow, SQLx, Rust conventions, gotchas

## Migration Plan Status (as of 2026-03-24)

- Phases 1-5.5: complete
- Phase 5.6: functionally complete - one open bug: restart 500 (diagnostics added 2026-03-22, raw response will be captured on next live restart attempt)
- Phase 6: complete (2026-03-22):
  - Redis pub/sub replaces tokio broadcast
  - Server publishes full legacy-format ShowResponse to `game.{id}` and `user.{token_id}` for web-legacy React compat
  - Server publishes `BrdgmeUpdate` (GameViewData + logs) to `ws.{user_id}` for rust/web Leptos direct signal updates (no re-fetch)
  - `/ws` handler is session-aware; subscribes to `game.*` + `ws.{user_id}`
  - `GameRestarted` WS event removed - regular BrdgmeUpdate for old game carries `restarted_game_id` (same as legacy always did)
- Phase 6.5: ArgoCD production CD (next)
- Phase 7: side-by-side validation then legacy decommission

## Production Status

Production is NOT migrated - old system still running. Do NOT deploy the `leptos` branch to prod without restoring legacy services to the prod kustomization. The `leptos` branch kustomization was modified for dev/migration work.

## WebSocket Architecture

Uses Redis pub/sub. Three channels:
- `game.{id}` - legacy ShowResponse format (public, for web-legacy compat)
- `user.{auth_token_id}` - legacy ShowResponse with private data (for web-legacy)
- `ws.{user_id}` - `BrdgmeUpdate` (GameViewData + logs) for rust/web Leptos

`/ws` handler is session-aware; subscribes to `game.*` + `ws.{user_id}`. Multi-replica safe. NATS replaces Redis in Phase 8 (after legacy decommission).
