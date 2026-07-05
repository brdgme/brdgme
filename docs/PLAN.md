# brdgme Product Backlog

This file is the backlog index. Each work item lives in `docs/plan/NN-*.md`;
the number is a **permanent ID in assignment order** - it never changes and
never implies execution order. Priority lives in the ordered list below
(historically these items were called "phases"; prose saying "Phase NN"
means item `NN`).

**Priority order (hard-cutover resequencing 2026-07-04):**
Restart 500 error → 3-player render → #21 OpenTofu (first: encodes the
cluster prereqs, DNS zone, buckets) → #22a Resend outbound remaining
steps (domain verification via tofu, prod secret, live-inbox check) →
#14 prod prerequisites → #13 NATS bot eventing (JetStream) →
#17 NATS WS migration (now pre-cutover) → #19 CloudNativePG →
#15 ArgoCD + sealed-secrets → #20 external-dns → #18
hardening (VictoriaLogs, now pre-cutover) → #16 hard cutover + 1-week
validation gate → decommission.
**Post-go-live:** #22b-d (play-by-email, reminders, multi-email) →
#24 game invites → #25 rules rendering → #26 theming/dark mode →
#23 Rust game ports (ongoing).

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

| # | Title | Status | Link |
|---|---|---|---|
| 1 | Foundation & Shared Logic | Complete | [01-foundation.md](plan/01-foundation.md) |
| 2 | Database Layer | Complete | [02-database-layer.md](plan/02-database-layer.md) |
| 3 | Backend (Axum Core) | Complete | [03-backend-axum-core.md](plan/03-backend-axum-core.md) |
| 4 | WebSocket Integration | Complete | [04-websocket-integration.md](plan/04-websocket-integration.md) |
| 5 | Frontend (Leptos UI) | Complete | [05-frontend-leptos-ui.md](plan/05-frontend-leptos-ui.md) |
| 6 | Dev Environment Migration | Complete | [06-dev-environment-migration.md](plan/06-dev-environment-migration.md) |
| 7 | Pre-Cutover Fixes | Complete | [07-pre-cutover-fixes.md](plan/07-pre-cutover-fixes.md) |
| 8 | Redis pub/sub + web-legacy WS compatibility | Complete | [08-redis-pubsub-web-legacy-ws-compat.md](plan/08-redis-pubsub-web-legacy-ws-compat.md) |
| 9 | LLM Bots | Complete | [09-llm-bots.md](plan/09-llm-bots.md) |
| 10 | Eliminate runtime panics in rust/web | Complete | [10-eliminate-runtime-panics.md](plan/10-eliminate-runtime-panics.md) |
| 11 | Testing Foundation | Complete (completed 2026-07-04) | [11-testing-foundation.md](plan/11-testing-foundation.md) |
| 12 | ELO Ratings | Complete | [12-elo-ratings.md](plan/12-elo-ratings.md) |
| 13 | NATS Bot Eventing | Complete (2026-07-05) | [13-nats-bot-eventing.md](plan/13-nats-bot-eventing.md) |
| 14 | Drop Knative - Plain Deployments + Gateway API | Dev complete (landed fc7cb3f); prod prerequisites pending | [14-drop-knative-gateway-api.md](plan/14-drop-knative-gateway-api.md) |
| 15 | Production CD (ArgoCD) | Pending | [15-production-cd-argocd.md](plan/15-production-cd-argocd.md) |
| 16 | Production Cutover (hard cutover + break-glass rollback; revised 2026-07-04) | Pending | [16-production-cutover-validation.md](plan/16-production-cutover-validation.md) |
| 17 | NATS Migration + WS simplification | Pending - resequenced pre-cutover 2026-07-04 | [17-nats-migration-ws-simplification.md](plan/17-nats-migration-ws-simplification.md) |
| 18 | Production Hardening | Pending - resequenced pre-cutover 2026-07-04; probes & observability section added 2026-07-05 | [18-production-hardening.md](plan/18-production-hardening.md) |
| 19 | CloudNativePG | Pending | [19-cloudnativepg.md](plan/19-cloudnativepg.md) |
| 20 | external-dns | Pending | [20-external-dns.md](plan/20-external-dns.md) |
| 21 | OpenTofu Infrastructure as Code | Pending - human-paced | [21-opentofu-iac.md](plan/21-opentofu-iac.md) |
| 22 | Email via Resend | 22a code complete (landed 77a2092); human/infra steps + 22b-22d pending - high priority; 22c reminders + 22d multi-email added 2026-07-04 | [22-email-via-resend.md](plan/22-email-via-resend.md) |
| 23 | Rust Game Ports | Pending | [23-rust-game-ports.md](plan/23-rust-game-ports.md) |
| 24 | Game Invites | Pending - post-go-live, non-blocking | [24-game-invites.md](plan/24-game-invites.md) |
| 25 | Rules Rendering for Humans (Web UI + Email) | Pending - post-go-live, non-blocking | [25-rules-rendering.md](plan/25-rules-rendering.md) |
| 26 | Theming / Dark Mode (Web UI + Email) | Pending - post-go-live, non-blocking | [26-theming.md](plan/26-theming.md) |
| Bug fixes | Bug fixes | Partially resolved | [bugs.md](plan/bugs.md) |
| Review findings 2026-07-04 | Review findings 2026-07-04 | Resolved 2026-07-04 | [review-findings-2026-07-04.md](plan/review-findings-2026-07-04.md) |
| Quick wins | Quick wins (added 2026-07-03) | Complete | [quick-wins.md](plan/quick-wins.md) |
| Development Workflow | Development Workflow | Superseded by [DEV.md](DEV.md) | [development-workflow.md](plan/development-workflow.md) |

---

## History

Items are numbered in assignment order, not execution order - see the
priority order at the top. Items 1-12 are complete; 14 is dev-complete
with prod prerequisites pending; 13 and 15-24 remain pending.
(2026-07-04: files renamed `phase-NN-*.md` → `NN-*.md` and this file
reframed as the backlog - reprioritising was fighting the "phase" naming.)
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
"Review findings 2026-07-04" section - the HIGH items block prod cutover.
2026-07-04: Phase 22 expanded with 22c turn reminders and 22d multi-email
switching; Phase 24 game invites added - all post-go-live, non-blocking.
2026-07-04 (later): hard-cutover decision - Phase 16 rewritten from
side-by-side validation to hard cutover with a break-glass rollback overlay
and a 1-week gate; Phases 17 and 18 resequenced pre-cutover; Phase 21
moved to the front of the pre-go-live sequence.)

2026-07-04: Phase 25 rules rendering added (single-source RULES.md,
render-time specialization; web UI post-go-live, email folded after 22b).

**2026-07-04:** restructured this file into a KEP-style layout: a thin index
(this file) plus one file per phase/work-stream under `docs/plan/`. The
"Delegation Readiness (assessed 2026-07-02)" section was deleted rather than
moved - it was stale, and every delegation-gap note it referenced already
exists inline in the relevant phase file.
