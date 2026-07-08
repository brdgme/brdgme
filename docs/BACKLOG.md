# brdgme Product Backlog

This file is the backlog index. Each work item's design/decisions live in
`docs/superpowers/specs/YYYY-MM-DD-NN-*-design.md` and its tasks/runbook live
in `docs/superpowers/plans/YYYY-MM-DD-NN-*.md` (either may be absent if the
item has no content of that kind); `NN` is a **permanent ID in assignment
order** - it never changes and never implies execution order. Priority lives
in the ordered list below (historically these items were called "phases";
prose saying "Phase NN" means item `NN`).

**Priority order (updated 2026-07-08; hard-cutover resequencing
2026-07-04):** done so far: restart-500 + 3-player-render bugs, #21
OpenTofu (complete 2026-07-06), #22a Resend
outbound, #14 dev + prod prereqs (client-IP flip deferred to #16 beta),
#13 NATS bot eventing, #17 NATS WS migration, #19 prod provisioning +
#15 ArgoCD + sealed-secrets (live, first fully-green sync 2026-07-08;
their beta-window tails - CI deploy job, sync-failure drill, PITR verify,
import rehearsal - remain). **Remaining pre-go-live:** #18 hardening
(Grafana Cloud observability + APM, decided 2026-07-05, superseding
VictoriaLogs) → #16 beta period (isolated DB) → hard cutover + 1-week
validation gate → decommission. (#20 external-dns retired 2026-07-05 -
no viable DO provider; see 20-external-dns.md.)
**Post-go-live:** #22b-d (play-by-email, reminders, multi-email) →
#24 game invites → #25 rules rendering → #26 theming/dark mode →
#23 Rust game ports (ongoing). #31 (Rust-only repo) spans both: WP1
legacy-stack deletion can run pre-cutover (unblocked 2026-07-08 by the
no-rollback decision; it simplifies #16); the rest follows #23.

## Objective

Consolidate the `brdgme` platform into a single Rust-based monolithic
application using Axum (backend) and Leptos (frontend/WASM). This replaces the
Rocket API, Node.js WebSocket service, and TypeScript/React frontend.

## Strategy

Build the new system in `rust/web` in parallel with the existing services. The
old services (`rust/api`, `web`, `websocket`) remain untouched until cutover.

## Out of Scope (decided 2026-07-02)

- **Go game services**: not part of the *cutover* migration - they keep
  running behind the stable, language-agnostic game HTTP contract
  throughout. (The 2026-07-02 "never ported" call was superseded 2026-07-04
  and 2026-07-08: all 17 are being converted to Rust `-2` editions under
  #23, and the Go stack is removed once conversions finish - see #31.)
- **Chat**: legacy chat tables/queries (`rust/api` chat queries, `games.chat_id`)
  are not ported. Future work, not scheduled.
- **lords-of-vegas-1**: implemented in `rust/game/` but intentionally not
  deployed (no Tiltfile entry, no k8s manifests). Future work, not scheduled.
- **Play-by-email**: not part of the cutover itself, but now planned as
  Phase 22b (post-cutover). Outbound email moves to Resend pre-cutover
  (Phase 22a).

---

## Status

| # | Title | Status | Spec | Plan |
|---|---|---|---|---|
| 1 | Foundation & Shared Logic | Complete | - | [plan](superpowers/plans/2026-07-08-01-foundation.md) |
| 2 | Database Layer | Complete | - | [plan](superpowers/plans/2026-07-08-02-database-layer.md) |
| 3 | Backend (Axum Core) | Complete | - | [plan](superpowers/plans/2026-07-08-03-backend-axum-core.md) |
| 4 | WebSocket Integration | Complete | - | [plan](superpowers/plans/2026-07-08-04-websocket-integration.md) |
| 5 | Frontend (Leptos UI) | Complete | - | [plan](superpowers/plans/2026-07-08-05-frontend-leptos-ui.md) |
| 6 | Dev Environment Migration | Complete | - | [plan](superpowers/plans/2026-07-08-06-dev-environment-migration.md) |
| 7 | Pre-Cutover Fixes | Complete | - | [plan](superpowers/plans/2026-07-02-07-pre-cutover-fixes.md) |
| 8 | Redis pub/sub + web-legacy WS compatibility | Complete | [spec](superpowers/specs/2026-07-08-08-redis-pubsub-web-legacy-ws-compat-design.md) | [plan](superpowers/plans/2026-07-08-08-redis-pubsub-web-legacy-ws-compat.md) |
| 9 | LLM Bots | Complete | [spec](superpowers/specs/2026-07-08-09-llm-bots-design.md) | [plan](superpowers/plans/2026-07-08-09-llm-bots.md) |
| 10 | Eliminate runtime panics in rust/web | Complete | [spec](superpowers/specs/2026-07-08-10-eliminate-runtime-panics-design.md) | [plan](superpowers/plans/2026-07-08-10-eliminate-runtime-panics.md) |
| 11 | Testing Foundation | Complete (completed 2026-07-04) | [spec](superpowers/specs/2026-07-04-11-testing-foundation-design.md) | [plan](superpowers/plans/2026-07-04-11-testing-foundation.md) |
| 12 | ELO Ratings | Complete | [spec](superpowers/specs/2026-07-04-12-elo-ratings-design.md) | [plan](superpowers/plans/2026-07-04-12-elo-ratings.md) |
| 13 | NATS Bot Eventing | Complete (2026-07-05) | [spec](superpowers/specs/2026-07-05-13-nats-bot-eventing-design.md) | [plan](superpowers/plans/2026-07-05-13-nats-bot-eventing.md) |
| 14 | Drop Knative - Plain Deployments + Gateway API | Dev complete (landed fc7cb3f); all prod prereqs resolved 2026-07-05 except client-IP/PROXY-protocol, intentionally deferred live to Phase 16 (needs a DOKS-managed ConfigMap flip with no dry-run value pre-Gateway) | [spec](superpowers/specs/2026-07-05-14-drop-knative-gateway-api-design.md) | [plan](superpowers/plans/2026-07-05-14-drop-knative-gateway-api.md) |
| 15 | Production CD (ArgoCD) | Live 2026-07-08 - ArgoCD + sealed-secrets running in prod, first fully-green sync at brdgme@851e23c; remaining: CI deploy job, delete stale k8s/argocd/, admin-password rotation, sync-failure drill (#16 beta) | [spec](superpowers/specs/2026-07-08-15-production-cd-argocd-design.md) | [plan](superpowers/plans/2026-07-08-15-production-cd-argocd.md) |
| 16 | Production Cutover (hard cutover + break-glass rollback; beta period on isolated DB + freeze/TTL runbook added 2026-07-05) | Pending | [spec](superpowers/specs/2026-07-08-16-production-cutover-validation-design.md) | [plan](superpowers/plans/2026-07-08-16-production-cutover-validation.md) |
| 17 | NATS Migration + WS simplification | Complete (2026-07-05) | [spec](superpowers/specs/2026-07-05-17-nats-migration-ws-simplification-design.md) | [plan](superpowers/plans/2026-07-05-17-nats-migration-ws-simplification.md) |
| 18 | Production Hardening | Pending - fully specced 2026-07-05: all-in Grafana Cloud (logs/metrics/traces/alerting, supersedes VictoriaLogs), APM via OTLP, probes, external uptime monitor, capacity check | [spec](superpowers/specs/2026-07-07-18-production-hardening-design.md) | [plan](superpowers/plans/2026-07-07-18-production-hardening.md) |
| 19 | CloudNativePG | Dev complete; prod Cluster + Barman Cloud backups running under ArgoCD (green 2026-07-08); remaining: PITR verify + import rehearsal (#16 beta), real import at cutover | [spec](superpowers/specs/2026-07-08-19-cloudnativepg-design.md) | [plan](superpowers/plans/2026-07-08-19-cloudnativepg.md) |
| 20 | external-dns | Superseded 2026-07-05 - folded into Phases 16/21 | [spec](superpowers/specs/2026-07-08-20-external-dns-design.md) | - |
| 21 | OpenTofu Infrastructure as Code | Complete 2026-07-06 (stages applied 2026-07-05; state-bucket versioning + Route53 zone deletion 2026-07-06) | [spec](superpowers/specs/2026-07-06-21-opentofu-iac-design.md) | [plan](superpowers/plans/2026-07-06-21-opentofu-iac.md) |
| 22 | Email via Resend | 22a complete (code landed 77a2092; prod secret + live-inbox SPF/DKIM/DMARC check done 2026-07-05); 22b-22d pending | [spec](superpowers/specs/2026-07-05-22-email-via-resend-design.md) | [plan](superpowers/plans/2026-07-05-22-email-via-resend.md) |
| 23 | Rust Game Ports | Pending | [spec](superpowers/specs/2026-07-04-23-rust-game-ports-design.md) | [plan](superpowers/plans/2026-07-04-23-rust-game-ports.md) |
| 24 | Game Invites | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-04-24-game-invites-design.md) | [plan](superpowers/plans/2026-07-04-24-game-invites.md) |
| 25 | Rules Rendering for Humans (Web UI + Email) | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-05-25-rules-rendering-design.md) | [plan](superpowers/plans/2026-07-05-25-rules-rendering.md) |
| 26 | Theming / Dark Mode (Web UI + Email) | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-05-26-theming-design.md) | [plan](superpowers/plans/2026-07-05-26-theming.md) |
| 27 | rust/web Simplification (skinny queries, WS signal merge; 5 WPs, added 2026-07-05) | Pending | [spec](superpowers/specs/2026-07-07-27-web-simplification-design.md) | [plan](superpowers/plans/2026-07-07-27-web-simplification.md) |
| 28 | Abuse Protection (bots, scripted clients, DoS) - login rework + send caps + Cloudflare edge | Decided 2026-07-08, ready to implement (WP1-3 pre-cutover app work, WP4 Cloudflare post-cutover) | [spec](superpowers/specs/2026-07-08-28-abuse-protection-design.md) | [plan](superpowers/plans/2026-07-08-28-abuse-protection.md) |
| 29 | Player Stats and Historical Reports (profiles, ELO charts, form strips; zero-dep SSR SVG charting) | Draft 2026-07-08 - post-go-live, non-blocking; no schema changes for v1 | [spec](superpowers/specs/2026-07-08-29-stats-reports-design.md) | [plan](superpowers/plans/2026-07-08-29-stats-reports.md) |
| 30 | Friends (requests, invite policy, picker suggestions, dashboard summaries; reuses the dormant 2017 `friends` table) | Draft 2026-07-08 - post-go-live, non-blocking; independent of #24 but shares its picker/policy surfaces | [spec](superpowers/specs/2026-07-08-30-friends-design.md) | [plan](superpowers/plans/2026-07-08-30-friends.md) |
| 31 | Rust-Only Repository (delete legacy trio + brdgme-go, game shelving lifecycle, lift `rust/` to root) | Ready 2026-07-08 - no-rollback decision made, WP1 runnable pre-cutover; WP3-5 gated on #23 Track B | [spec](superpowers/specs/2026-07-08-31-rust-only-repo-design.md) | [plan](superpowers/plans/2026-07-08-31-rust-only-repo.md) |
| Bug fixes | Bug fixes | Partially resolved | - | [plan](superpowers/plans/2026-07-05-bugs.md) |
| Review findings 2026-07-04 | Review findings 2026-07-04 | Resolved 2026-07-04 | - | [plan](superpowers/plans/2026-07-04-review-findings-2026-07-04.md) |
| Quick wins | Quick wins (added 2026-07-03) | Complete | - | [plan](superpowers/plans/2026-07-04-quick-wins.md) |
| Development Workflow | Development Workflow | Superseded by [DEV.md](DEV.md) | - | - |

---

## Human tasks (operator-only, in rough execution order)

Everything below needs Michael (accounts, credentials, production access);
tasks are also marked *(human)* inline in their phase files. Added
2026-07-05.

1. **#21:** ~~done in full~~ (state-bucket versioning + Route53 zone
   deletion completed 2026-07-06).
2. **#15:** ~~mostly done 2026-07-06/08~~ - repo created, deploy key
   provisioned, sealed-secrets + ArgoCD installed, secrets sealed, fully
   green. Outstanding: rotate the admin password + delete
   `argocd-initial-admin-secret` (still present 2026-07-08); confirm the
   sealing-key pair is backed up offline.
3. **#18:** ~~stack created + `grafana-cloud` sealed~~ (secret live in the
   cluster; Alloy deployed 2026-07-07). Known issue 2026-07-08: telemetry
   volume exhausted the Grafana Cloud free-tier quota within hours, so
   remote_write is rejected and Alloy OOM-loops buffering the backlog -
   telemetry volume must be cut (and the quota window waited out) before
   #18 can be called shipping. Outstanding: reduce telemetry volume;
   configure the alert rules and email contact point in the Grafana UI;
   set up the external uptime monitor account.
4. **#16 beta:** flip the cilium PROXY-protocol ConfigMap + restart the
   DaemonSet; `tofu apply` the `beta.brdg.me` record; drive the beta
   checklist (test games, Grafana verification).
5. **#19 (during beta):** test import - `pg_dump` live Linode prod,
   restore into a scratch CNPG database, record timings/fixes; verify a
   PITR restore from the Spaces backups.
6. **#16 cutover:** lower TTLs (`tofu apply`); announce downtime; stop
   the Linode stack; real `pg_dump`/restore + migrations; repoint apex
   DNS (`tofu apply`); smoke test; flip the uptime monitor to apex.
   (The `postgres-config`/`postgres-rw` host is handled at Phase 15
   sealing time, not cutover - revised 2026-07-06.)
7. **#16 decommission (after the validation week):** decommission the
   Linode server (archive a final dump); the source/manifest deletion
   itself is agent-delegable.

---

## History

Items are numbered in assignment order, not execution order - see the
priority order at the top. Items 1-13 and 17 are complete; 14 is
dev-complete with the client-IP flip deferred to item 16; 21 and 22a are
done bar small trailing steps; 15, 16, 18, 19-prod, and the post-go-live
items (22b-d, 23-27) remain pending.
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

2026-07-05: Phase 20 (external-dns) retired - DigitalOcean's in-tree
external-dns provider was removed upstream (v0.21.0); the only replacement
is an unreviewed third-party webhook. DNS record management for the
cutover hostnames folds into Phase 21's infra/dns.tf and the Phase 16
cutover runbook instead.

2026-07-05 (plan review): all remaining "not ready" items fully specced
for delegation. Decisions: observability goes all-in on the Grafana Cloud
free tier (logs/metrics/traces/alerting + email delivery; supersedes the
VictoriaLogs/vmalert decisions; single Alloy agent in-cluster; APM via
OTLP traces from the monolith - wanted for cutover week); no in-cluster
alert evaluation (Resend not used for alerts; monolith webhook bridge
documented as fallback only); ArgoCD is port-forward-only with a
remote-base `brdgme-config` repo (no manifest copying); Phase 19 prod
import is workstation pg_dump/restore from Linode (no live cross-provider
link); Phase 16 gains a beta period on an isolated database, a freeze +
TTL-lowering cutover runbook, and a corrected two-path rollback story;
tofu state bucket gets versioning; bot-restart bug specced (bot_slots
pass-through).

2026-07-08: `docs/plan/` retired in favor of the superpowers convention -
each item's design/decisions moved to `docs/superpowers/specs/`, its
tasks/runbook to `docs/superpowers/plans/` (point-in-time records, not
living documents).
