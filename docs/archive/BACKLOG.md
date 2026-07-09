# Backlog Archive

Append-only record of backlog items removed from the `docs/BACKLOG.md`
Status table once they're done/cancelled/superseded, so that file stays
focused on work still in flight. Each row is a copy of the item's last
`docs/BACKLOG.md` row plus a **Resolution** and the date resolved (`-` if
not known). Design/task history for numbered items still lives under their
`NN-*` filename in `docs/superpowers/specs/` and/or `docs/superpowers/plans/`.

**Append new rows to the bottom as items close. Do not edit or reorder
existing rows.**

| # | Title | Resolution | Resolved | Spec | Plan |
|---|---|---|---|---|---|
| 1 | Foundation & Shared Logic | Done | - | - | [plan](../superpowers/plans/2026-07-08-01-foundation.md) |
| 2 | Database Layer | Done | - | - | [plan](../superpowers/plans/2026-07-08-02-database-layer.md) |
| 3 | Backend (Axum Core) | Done | - | - | [plan](../superpowers/plans/2026-07-08-03-backend-axum-core.md) |
| 4 | WebSocket Integration | Done | - | - | [plan](../superpowers/plans/2026-07-08-04-websocket-integration.md) |
| 5 | Frontend (Leptos UI) | Done | - | - | [plan](../superpowers/plans/2026-07-08-05-frontend-leptos-ui.md) |
| 6 | Dev Environment Migration | Done | - | - | [plan](../superpowers/plans/2026-07-08-06-dev-environment-migration.md) |
| 7 | Pre-Cutover Fixes | Done | - | - | [plan](../superpowers/plans/2026-07-02-07-pre-cutover-fixes.md) |
| 8 | Redis pub/sub + web-legacy WS compatibility | Done | - | [spec](../superpowers/specs/2026-07-08-08-redis-pubsub-web-legacy-ws-compat-design.md) | [plan](../superpowers/plans/2026-07-08-08-redis-pubsub-web-legacy-ws-compat.md) |
| 9 | LLM Bots | Done | - | [spec](../superpowers/specs/2026-07-08-09-llm-bots-design.md) | [plan](../superpowers/plans/2026-07-08-09-llm-bots.md) |
| 10 | Eliminate runtime panics in rust/web | Done | - | [spec](../superpowers/specs/2026-07-08-10-eliminate-runtime-panics-design.md) | [plan](../superpowers/plans/2026-07-08-10-eliminate-runtime-panics.md) |
| 11 | Testing Foundation | Done | 2026-07-04 | [spec](../superpowers/specs/2026-07-04-11-testing-foundation-design.md) | [plan](../superpowers/plans/2026-07-04-11-testing-foundation.md) |
| 12 | ELO Ratings | Done | 2026-07-04 | [spec](../superpowers/specs/2026-07-04-12-elo-ratings-design.md) | [plan](../superpowers/plans/2026-07-04-12-elo-ratings.md) |
| 13 | NATS Bot Eventing | Done | 2026-07-05 | [spec](../superpowers/specs/2026-07-05-13-nats-bot-eventing-design.md) | [plan](../superpowers/plans/2026-07-05-13-nats-bot-eventing.md) |
| 17 | NATS Migration + WS simplification | Done | 2026-07-05 | [spec](../superpowers/specs/2026-07-05-17-nats-migration-ws-simplification-design.md) | [plan](../superpowers/plans/2026-07-05-17-nats-migration-ws-simplification.md) |
| 21 | OpenTofu Infrastructure as Code | Done | 2026-07-06 | [spec](../superpowers/specs/2026-07-06-21-opentofu-iac-design.md) | [plan](../superpowers/plans/2026-07-06-21-opentofu-iac.md) |
| Quick wins | Quick wins (added 2026-07-03) | Done | - | - | [plan](../superpowers/plans/2026-07-04-quick-wins.md) |
| Review findings 2026-07-04 | Review findings 2026-07-04 | Resolved | 2026-07-04 | - | [plan](../superpowers/plans/2026-07-04-review-findings-2026-07-04.md) |
| Development Workflow | Development Workflow | Superseded | - | - | [DEV.md](../DEV.md) |
| 14 | Drop Knative - Plain Deployments + Gateway API | Fully done 2026-07-08 - dev landed fc7cb3f; final open item (client-IP/PROXY protocol) attempted and dropped 2026-07-08 (DOKS reconciler owns cilium-config and reverts the flag; superseded by #28's IP-independent caps + Cloudflare edge) | 2026-07-08 | [spec](../superpowers/specs/2026-07-05-14-drop-knative-gateway-api-design.md) | [plan](../superpowers/plans/2026-07-05-14-drop-knative-gateway-api.md) |
| 18 | Production Hardening | Fully done 2026-07-09 - Grafana Cloud observability implemented in full (Alloy log/metric/trace shipping with volume cuts, OTLP tracing at 10% sampling, /metrics, probes incl. operator /healthz added 2026-07-09); WASM source maps descoped 2026-07-09 (toolchain blocker); contact point (beefsack@gmail.com) + external uptime monitor done 2026-07-09; remaining rollout (deploy the in-tree changes, wait out the quota window, create + verify alert rules) removed from the backlog 2026-07-09 - Michael tracks it separately | 2026-07-09 | [spec](../superpowers/specs/2026-07-07-18-production-hardening-design.md) | [plan](../superpowers/plans/2026-07-07-18-production-hardening.md) |
| 20 | external-dns | Superseded 2026-07-05 - folded into Phases 16/21 (DigitalOcean's in-tree external-dns provider removed upstream in v0.21.0; only replacement is an unreviewed third-party webhook) | 2026-07-05 | [spec](../superpowers/specs/2026-07-08-20-external-dns-design.md) | - |
