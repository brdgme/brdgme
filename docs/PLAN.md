# Monolith Migration Plan

**Current focus (in order):**
Restart 500 error → 3-player render → Phase 22a Resend outbound remaining
steps (domain verification, prod secret, live-inbox check) → Phase 14 prod
prerequisites →
Phase 13 NATS bot eventing (JetStream) → Phase 19 CloudNativePG → Phase 15
ArgoCD + sealed-secrets → Phase 20 external-dns → Phase 16 cutover +
validation → Phase 22b play-by-email → Phase 17 NATS WS migration → Phase 18
hardening (VictoriaLogs). Phase 21 OpenTofu is human-paced and independent;
highest value if started before Phase 14's prod prerequisites.

## Objective

Consolidate the `brdgme` platform into a single Rust-based monolithic
application using Axum (backend) and Leptos (frontend/WASM). This replaces the
Rocket API, Node.js WebSocket service, and TypeScript/React frontend.

## Strategy

Build the new system in `rust/web` in parallel with the existing services. The
old services (`rust/api`, `web`, `websocket`) remain untouched until cutover.

## Out of Scope (decided 2026-07-02)

- **Go game services**: the 17 Go games under `brdgme-go/` remain in production
  indefinitely behind the stable game HTTP contract. They are not part of this
  migration and there is no plan to port them to Rust. The contract is
  language-agnostic; Go and Rust games are built and deployed identically.
- **Chat**: legacy chat tables/queries (`rust/api` chat queries, `games.chat_id`)
  are not ported. Future work, not scheduled.
- **lords-of-vegas-1**: implemented in `rust/game/` but intentionally not
  deployed (no Tiltfile entry, no k8s manifests). Future work, not scheduled.
- **Play-by-email**: not part of the cutover itself, but now planned as
  Phase 22b (post-cutover). Outbound email moves to Resend pre-cutover
  (Phase 22a).

---

## Status

| Phase/Stream | Title | Status | Link |
|---|---|---|---|
| Phase 1 | Foundation & Shared Logic | Complete | [phase-01-foundation.md](plan/phase-01-foundation.md) |
| Phase 2 | Database Layer | Complete | [phase-02-database-layer.md](plan/phase-02-database-layer.md) |
| Phase 3 | Backend (Axum Core) | Complete | [phase-03-backend-axum-core.md](plan/phase-03-backend-axum-core.md) |
| Phase 4 | WebSocket Integration | Complete | [phase-04-websocket-integration.md](plan/phase-04-websocket-integration.md) |
| Phase 5 | Frontend (Leptos UI) | Complete | [phase-05-frontend-leptos-ui.md](plan/phase-05-frontend-leptos-ui.md) |
| Phase 6 | Dev Environment Migration | Complete | [phase-06-dev-environment-migration.md](plan/phase-06-dev-environment-migration.md) |
| Phase 7 | Pre-Cutover Fixes | Complete | [phase-07-pre-cutover-fixes.md](plan/phase-07-pre-cutover-fixes.md) |
| Phase 8 | Redis pub/sub + web-legacy WS compatibility | Complete | [phase-08-redis-pubsub-web-legacy-ws-compat.md](plan/phase-08-redis-pubsub-web-legacy-ws-compat.md) |
| Phase 9 | LLM Bots | Complete | [phase-09-llm-bots.md](plan/phase-09-llm-bots.md) |
| Phase 10 | Eliminate runtime panics in rust/web | Complete | [phase-10-eliminate-runtime-panics.md](plan/phase-10-eliminate-runtime-panics.md) |
| Phase 11 | Testing Foundation | Complete (completed 2026-07-04) | [phase-11-testing-foundation.md](plan/phase-11-testing-foundation.md) |
| Phase 12 | ELO Ratings | Complete | [phase-12-elo-ratings.md](plan/phase-12-elo-ratings.md) |
| Phase 13 | NATS Bot Eventing | Pending | [phase-13-nats-bot-eventing.md](plan/phase-13-nats-bot-eventing.md) |
| Phase 14 | Drop Knative - Plain Deployments + Gateway API | Dev complete (landed fc7cb3f); prod prerequisites pending | [phase-14-drop-knative-gateway-api.md](plan/phase-14-drop-knative-gateway-api.md) |
| Phase 15 | Production CD (ArgoCD) | Pending | [phase-15-production-cd-argocd.md](plan/phase-15-production-cd-argocd.md) |
| Phase 16 | Production Cutover & Side-by-Side Validation | Pending | [phase-16-production-cutover-validation.md](plan/phase-16-production-cutover-validation.md) |
| Phase 17 | NATS Migration + WS simplification | Pending | [phase-17-nats-migration-ws-simplification.md](plan/phase-17-nats-migration-ws-simplification.md) |
| Phase 18 | Production Hardening | Pending | [phase-18-production-hardening.md](plan/phase-18-production-hardening.md) |
| Phase 19 | CloudNativePG | Pending | [phase-19-cloudnativepg.md](plan/phase-19-cloudnativepg.md) |
| Phase 20 | external-dns | Pending | [phase-20-external-dns.md](plan/phase-20-external-dns.md) |
| Phase 21 | OpenTofu Infrastructure as Code | Pending - human-paced | [phase-21-opentofu-iac.md](plan/phase-21-opentofu-iac.md) |
| Phase 22 | Email via Resend | 22a code complete (landed 77a2092); human/infra steps + 22b pending - high priority | [phase-22-email-via-resend.md](plan/phase-22-email-via-resend.md) |
| Phase 23 | Rust Game Ports | Pending | [phase-23-rust-game-ports.md](plan/phase-23-rust-game-ports.md) |
| Bug fixes | Bug fixes | Partially resolved | [bugs.md](plan/bugs.md) |
| Review findings 2026-07-04 | Review findings 2026-07-04 | Resolved 2026-07-04 | [review-findings-2026-07-04.md](plan/review-findings-2026-07-04.md) |
| Quick wins | Quick wins (added 2026-07-03) | Complete | [quick-wins.md](plan/quick-wins.md) |
| Development Workflow | Development Workflow | Superseded by [DEV.md](DEV.md) | [development-workflow.md](plan/development-workflow.md) |

---

## History

Phases are numbered in assignment order, not execution order - see the focus
line for execution order. Phases 1-12 are complete; Phase 14 is dev-complete
with prod prerequisites pending; Phase 13 and Phases 15-23 remain pending.
(Renumbered 2026-07-02: 5.5→6, 5.6→7, old 6→8, 5.7→10, 6.5→ArgoCD, old
7→cutover, old 8→NATS WS; ELO and NATS bot eventing split out of Phase 9
into Phases 12 and 13. 2026-07-03: Phase 14 'Drop Knative' inserted; ArgoCD
14→15, cutover 15→16, NATS WS 16→17, hardening 17→18. 2026-07-03 tech
review: Quick wins section and Phases 19-21 added; JetStream, ctlptl,
sealed-secrets, and VictoriaLogs decisions folded into Phases 13/14/15/18.
2026-07-03: Phase 22 'Email via Resend' added, split 22a outbound /
22b play-by-email; 22a revised same day to the Resend HTTP API - DO blocks
outbound SMTP - superseding the Mailpit quick win. 2026-07-03 final pass:
Renovate/cargo-deny/kubeconform quick win, leptos-use in Phase 17,
tower_governor in 22a, stale root artifacts in the Phase 16 decommission.
2026-07-03: Phase 10 runtime panics completed. 2026-07-04: comprehensive
review completed (docs/REVIEW-2026-07-04.md); findings added as the
"Review findings 2026-07-04" section - the HIGH items block prod cutover.)

**2026-07-04:** restructured this file into a KEP-style layout: a thin index
(this file) plus one file per phase/work-stream under `docs/plan/`. The
"Delegation Readiness (assessed 2026-07-02)" section was deleted rather than
moved - it was stale, and every delegation-gap note it referenced already
exists inline in the relevant phase file.
