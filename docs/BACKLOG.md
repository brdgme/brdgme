# brdgme Product Backlog

This file is the backlog index. Each work item's design/decisions live in
`docs/superpowers/specs/YYYY-MM-DD-NN-*-design.md` and its tasks/runbook live
in `docs/superpowers/plans/YYYY-MM-DD-NN-*.md` (either may be absent if the
item has no content of that kind); `NN` is a **permanent ID in assignment
order** - it never changes and never implies execution order. Priority lives
in the ordered list below (historically these items were called "phases";
prose saying "Phase NN" means item `NN`).

When an item is fully done/cancelled/superseded, move its row out of the
Status table below into [`docs/archive/BACKLOG.md`](archive/BACKLOG.md),
adding a Resolution and the date resolved - that file is append-only and
keeps this one from filling up with closed work.

**Priority order (updated 2026-07-10; hard-cutover resequencing
2026-07-04):** done so far: restart-500 + 3-player-render bugs, #21
OpenTofu (complete 2026-07-06), #22a Resend
outbound, #14 dev + prod prereqs (fully done - client-IP/PROXY-protocol
attempted and dropped 2026-07-08),
#13 NATS bot eventing, #17 NATS WS migration, #19 prod provisioning +
#15 ArgoCD + sealed-secrets (live, first fully-green sync 2026-07-08;
their beta-window tails - CI deploy job, sync-failure drill, PITR verify,
import rehearsal - remain), #28 WP1-3 app hardening (promoted 2026-07-08,
complete 2026-07-10), #28 WP4 Cloudflare edge (complete 2026-07-11).
**Remaining pre-go-live:** #34 admin functions + #35 user settings
(added 2026-07-11, wanted in place for the beta) -> #33 pre-go-live UI/UX
polish batch -> #16 beta period (isolated DB) -> hard cutover + 1-week
validation gate -> decommission. #32 demoted to post-go-live 2026-07-10
(Michael: the Grafana Cloud quota must reset anyway - not a go-live
blocker). (#20 external-dns retired 2026-07-05 - no viable DO
provider; see 20-external-dns.md.)
**Post-go-live:** #32 Alloy OTLP export investigation (demoted from
pre-go-live 2026-07-10), #22b-d (play-by-email, reminders, multi-email) →
#24 game invites → #25 rules rendering → #26 theming/dark mode →
#23 Rust game ports (ongoing) → (bottom of backlog) #36 Web Push turn
notifications. #31 (Rust-only repo) spans both: WP1
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

Fully done/resolved/superseded items (1-14, 17, 18, 20, 21, 28, Quick wins,
Review findings 2026-07-04, Development Workflow) have been moved to
[`docs/archive/BACKLOG.md`](archive/BACKLOG.md).

| # | Title | Status | Spec | Plan |
|---|---|---|---|---|
| 15 | Production CD (ArgoCD) | Live 2026-07-08 - ArgoCD + sealed-secrets running in prod, first fully-green sync at brdgme@851e23c; remaining: CI deploy job, delete stale k8s/argocd/, admin-password rotation, sync-failure drill (#16 beta) | [spec](superpowers/specs/2026-07-08-15-production-cd-argocd-design.md) | [plan](superpowers/plans/2026-07-08-15-production-cd-argocd.md) |
| 16 | Production Cutover (hard cutover + break-glass rollback; beta period on isolated DB + freeze/TTL runbook added 2026-07-05) | Pending | [spec](superpowers/specs/2026-07-08-16-production-cutover-validation-design.md) | [plan](superpowers/plans/2026-07-08-16-production-cutover-validation.md) |
| 19 | CloudNativePG | Dev complete; prod Cluster + Barman Cloud backups running under ArgoCD (green 2026-07-08); remaining: PITR verify + import rehearsal (#16 beta), real import at cutover | [spec](superpowers/specs/2026-07-08-19-cloudnativepg-design.md) | [plan](superpowers/plans/2026-07-08-19-cloudnativepg.md) |
| 22 | Email via Resend | 22a complete (code landed 77a2092; prod secret + live-inbox SPF/DKIM/DMARC check done 2026-07-05); 22b-22d pending | [spec](superpowers/specs/2026-07-05-22-email-via-resend-design.md) | [plan](superpowers/plans/2026-07-05-22-email-via-resend.md) |
| 23 | Rust Game Ports | Pending | [spec](superpowers/specs/2026-07-04-23-rust-game-ports-design.md) | [plan](superpowers/plans/2026-07-04-23-rust-game-ports.md) |
| 24 | Game Invites | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-04-24-game-invites-design.md) | [plan](superpowers/plans/2026-07-04-24-game-invites.md) |
| 25 | Rules Rendering for Humans (Web UI + Email) | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-05-25-rules-rendering-design.md) | [plan](superpowers/plans/2026-07-05-25-rules-rendering.md) |
| 26 | Theming / Dark Mode (Web UI + Email) | Pending - post-go-live, non-blocking | [spec](superpowers/specs/2026-07-05-26-theming-design.md) | [plan](superpowers/plans/2026-07-05-26-theming.md) |
| 27 | rust/web Simplification (skinny queries, WS signal merge; 5 WPs, added 2026-07-05) | Pending | [spec](superpowers/specs/2026-07-07-27-web-simplification-design.md) | [plan](superpowers/plans/2026-07-07-27-web-simplification.md) |
| 29 | Player Stats and Historical Reports (profiles, ELO charts, form strips; zero-dep SSR SVG charting) | Draft 2026-07-08 - post-go-live, non-blocking; no schema changes for v1 | [spec](superpowers/specs/2026-07-08-29-stats-reports-design.md) | [plan](superpowers/plans/2026-07-08-29-stats-reports.md) |
| 30 | Friends (requests, invite policy, picker suggestions, dashboard summaries; reuses the dormant 2017 `friends` table) | Draft 2026-07-08 - post-go-live, non-blocking; independent of #24 but shares its picker/policy surfaces | [spec](superpowers/specs/2026-07-08-30-friends-design.md) | [plan](superpowers/plans/2026-07-08-30-friends.md) |
| 31 | Rust-Only Repository (delete legacy trio + brdgme-go, game shelving lifecycle, lift `rust/` to root) | Ready 2026-07-08 - no-rollback decision made, WP1 runnable pre-cutover; WP3-5 gated on #23 Track B | [spec](superpowers/specs/2026-07-08-31-rust-only-repo-design.md) | [plan](superpowers/plans/2026-07-08-31-rust-only-repo.md) |
| 32 | Alloy `otelcol.exporter.otlp.grafana_cloud` export failure (Tempo traces) | Pending - demoted to post-go-live 2026-07-10 (Michael: the Grafana Cloud quota must reset anyway, not a go-live blocker; was promoted pre-go-live 2026-07-09). Observed 2026-07-09 in prod alloy pod logs - the OTLP exporter (Tempo traces endpoint, Grafana Cloud) is stuck in a retry loop with `resolver error: produced zero addresses`; traces are not being exported | - | - |
| 33 | Pre-go-live UI/UX polish batch (minor jank collected as found, e.g. login submit loading state) | Plan written 2026-07-11 - ready to execute; the collection doc ([docs/pre-go-live-polish.md](pre-go-live-polish.md)) is the requirements record (9 entries), the plan is the batch fix | - | [plan](superpowers/plans/2026-07-11-33-pre-go-live-polish.md) |
| 34 | Admin functions (`is_admin` flag, force-delete game, game JSON export + dev import CLI) | Decided 2026-07-11 - pre-beta | [spec](superpowers/specs/2026-07-11-34-admin-functions-design.md) | - |
| 35 | User settings page (unique display names 1-16 `[a-zA-Z0-9_-]`, petname-generated defaults, ordered 3-colour prefs wired into game creation) | Decided 2026-07-11 - pre-beta | [spec](superpowers/specs/2026-07-11-35-user-settings-design.md) | - |
| 36 | Web Push turn notifications (service worker, VAPID keys, push subscriptions in Postgres, server-side push on turn change, settings toggle, graceful permission-denied handling) | Pending - post-go-live, bottom of backlog (scoped 2026-07-11; sits alongside #22c turn-reminder emails; no spec yet) | - | - |
| Bug fixes | Bug fixes | Partially resolved | - | [plan](superpowers/plans/2026-07-05-bugs.md) |

---

## Human tasks (operator-only, in rough execution order)

Everything below needs Michael (accounts, credentials, production access);
tasks are also marked *(human)* inline in their phase files. Added
2026-07-05.

1. **#15:** ~~mostly done 2026-07-06/08~~ - repo created, deploy key
   provisioned, sealed-secrets + ArgoCD installed, secrets sealed, fully
   green. Outstanding: rotate the admin password + delete
   `argocd-initial-admin-secret` (still present 2026-07-08); confirm the
   sealing-key pair is backed up offline.
2. **#16 beta:** drive the beta checklist (test games, Grafana
   verification). (The cilium PROXY-protocol ConfigMap flip is dropped -
   see History 2026-07-08; the `beta.brdg.me` record is already applied
   and resolving.)
3. **#19 (during beta):** test import - `pg_dump` live Linode prod,
   restore into a scratch CNPG database, record timings/fixes; verify a
   PITR restore from the Spaces backups.
4. **#16 cutover:** lower TTLs (`tofu apply`); announce downtime; stop
   the Linode stack; real `pg_dump`/restore + migrations; repoint apex
   DNS (`tofu apply`); smoke test; flip the uptime monitor to apex.
   (The `postgres-config`/`postgres-rw` host is handled at Phase 15
   sealing time, not cutover - revised 2026-07-06.)
5. **#16 decommission (after the validation week):** decommission the
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

2026-07-08: moved fully done/resolved/superseded items (1-13, 17, 21,
Quick wins, Review findings 2026-07-04, Development Workflow) out of the
Status table into the new append-only `docs/archive/BACKLOG.md`, so the
table only tracks work still in flight; also dropped the now-stale
"#21: done in full" line from Human tasks. Going forward, close items by
appending to the archive rather than deleting rows outright.

2026-07-08 (later): the #14/#16 client-IP/PROXY-protocol flip was
attempted live on the `brdgme` prod cluster - `enable-gateway-api-proxy-protocol`
patched to `"true"` in `kube-system/cilium-config` and the cilium
DaemonSet restarted successfully, but DOKS's managed addon reconciler
(fieldManager `manager`) rewrote the ConfigMap back to `"false"` at
13:09:20Z, ~15 minutes later - it owns `cilium-config` and the flag
cannot be set persistently by the cluster operator. The matching DO-LB
annotation commit briefly deployed via ArgoCD and was reverted the same
hour (`brdgme` f31be4b, `brdgme-config` 8333793); prod is back to the
pre-flip state and `beta.brdg.me` stayed up throughout. Decision
(Michael): drop the client-IP/PROXY-protocol work entirely - no DO
support ticket, no retry planned; real client IPs are simply not
available to the app on this platform, so per-IP app-level limits stay
one collective bucket (keyed on the LB SNAT address) and XFF-spoofable
permanently. With this dropped, #14 has no remaining work and moves to
the archive as fully done. #28 WP1-3 (app-level hardening: DB-backed
send caps + per-code attempt caps, IP-independent) is promoted to
pre-go-live priority as the effective protection in place of the flip;
WP4 (Cloudflare edge, which sees real client IPs) stays post-cutover.
See `docs/superpowers/plans/2026-07-05-14-drop-knative-gateway-api.md`,
`docs/superpowers/plans/2026-07-08-16-production-cutover-validation.md`,
and `docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md`
for detail.

2026-07-09: #18 hardening closed - full Grafana Cloud observability
(Alloy log/metric/trace shipping with volume cuts, OTLP tracing at 10%
sampling, /metrics, probes incl. operator /healthz) implemented in-tree;
WASM source maps descoped (toolchain blocker); contact point
(beefsack@gmail.com) + external uptime monitor done. Moved to the
archive as fully done; remaining rollout (deploy, quota window,
alert-rule creation) removed from the backlog - Michael tracks it
separately. #20 (external-dns, superseded 2026-07-05) also moved to the
archive - no remaining work was ever tracked against it. Dropped the
now-stale '#18' line from Human tasks, same as the '#21' line was
dropped 2026-07-08.

2026-07-09: #32 added - Alloy's OTLP exporter to Grafana Cloud (Tempo
traces) observed stuck in a retry loop with `resolver error: produced
zero addresses` in prod alloy pod logs; no traces are being exported.
Promoted to pre-go-live priority - needs investigation before go-live.

2026-07-10: #28 WP1-3 completed. WP2 (commits 666e35b..0093291) added
global HTTP hygiene middleware to `build_router` - 256 KiB request body
limit + 30s timeout - plus a live-websocket >30s survival test; task
review approved. WP3 (commit 6e53681) switched rate-limit keying to
`PeerIpKeyExtractor` (socket peer address only), with forwarding headers
proven ignored, permanent per D6; task review approved (the dead
`headers` param on `extract_client_ip` is kept intentionally, to be
stripped in WP4's signature revisit). A final whole-branch review over
WP1-3 found no Critical issues and two Important findings, both resolved
by user decisions: the login/confirm rate-limit governor was loosened for
the shared SNAT bucket (login burst 30/+1 per 2s, confirm burst 60/+1 per
1s, with a D6 comment explaining why; WP4 will re-tighten per-IP via
`CF-Connecting-IP`), and the migration-005 `DROP COLUMN` deploy window was
accepted and documented (SQL comment plus a #16 beta-checklist line); a
reviewer-recommended accepted-race comment was also added in
`confirm_login_inner`. Fix commit 5a7bb85; re-review approved. Separately,
Michael initially considered and rejected pulling WP4 (Cloudflare) ahead
of go-live: it would stay post-cutover + 1-week gate per D1, since
bringing it forward would entangle nameserver migration with the cutover
itself, and the app-level DB caps are mandatory regardless (Cloudflare
would still be bypassed by traffic hitting the load balancer directly).

2026-07-10 (later, same day): that call was reversed - WP4 is promoted to
pre-go-live, superseding D1's post-cutover scheduling. Rationale: CF
proxy/WS/rate-limit behaviour is far easier to validate against
beta.brdg.me while still in beta than after going live; the nameserver
move happens well before cutover week, with legacy apex records ported
DNS-only (unproxied) so the live Linode site is untouched until cutover
day. The WP4 plan section needs a resequencing pass, since it was written
assuming Phase 16 (cutover) was already complete; the design spec is a
point-in-time record and is not being edited. Remaining pre-go-live order
is now #32 investigation → #28 WP4 (Cloudflare edge) → #16 beta →
cutover.

2026-07-10 (later still): #28 WP4 redesigned for pre-go-live and specced
(`docs/superpowers/specs/2026-07-10-28-wp4-cloudflare-pre-golive-design.md`,
plan `docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md`).
Single-stage migration: Michael created the CF zone (free plan, existing
account), CF copied the DO records at zone creation, and the registrar
nameservers were cut over to Cloudflare the same day - so the Tofu work is
adoption/import of the live zone, not creation, and beta.brdg.me is
already proxied. Key redesign call (spec W6): once the CF edge rate-limit
rule is proven on beta, the in-app per-IP rate limiting is DELETED
(`rate_limit.rs`, governor deps, `extract_client_ip`) rather than
re-tightened via a `CF-Connecting-IP` carve-out - WP1's DB-backed caps
remain the backstop for direct-to-LB traffic, and WP2's hygiene middleware
stays (W9). The old plan's WP4 section is superseded in place. Separately,
#32 (Alloy OTLP export) demoted to post-go-live (Michael: the Grafana
Cloud quota must reset anyway - not a go-live blocker); remaining
pre-go-live order is now #28 WP4 -> #16 beta -> cutover.

2026-07-11 (pre-beta planning): #34 admin functions and #35 user settings
added (both pre-beta, specs written same day); #36 Web Push turn
notifications added at the bottom of the post-go-live backlog (scoped
only - full service-worker/VAPID subsystem judged too large for now; an
in-tab-only Notification API variant was considered and rejected in
favour of doing Web Push properly later). Four new jank entries appended
to docs/pre-go-live-polish.md under #33 (inert sidebar Menu button,
missing autofocus set, white flash on command submit - a regression of
the Suspense->Transition fix recorded in 2026-07-05-bugs.md, reactive
title with my-turn count). Bot model configuration (multi-provider
routing/failover, runtime model switching) was discussed and deliberately
PARKED without a backlog item - to be revisited in a future session; the
sealed-secret reseal workflow stands for now.

2026-07-11: #28 WP4 (Cloudflare edge) completed, commits e34b8cf..0ef55d6:
brdg.me zone adopted into tofu (import, free plan); SSL Full-strict + WS +
edge rate-limit rule (60 req/10s on `/api/`, flood-proven 60 pass/40 429);
TLS switched HTTP01 -> DNS01, DO DNS resources deleted; in-app per-IP rate
limiting deleted per spec W6 (WP1 DB caps + WP2 hygiene middleware remain
the app-side backstop); Bot Fight Mode on (enable_js required), verified
against WS + login; origin lockdown spike REJECTED - DO LB allow-rules
annotation rejected by the controller, direct-to-LB bypass accepted and
documented (spec W7, DB caps backstop); docs updated (infra README
migration record, external-dns spec cross-ref). With WP1-4 all done, #28
is fully done and moved to the archive.

2026-07-11: #33 entry 5's secondary "Also investigate" item - whether
Rust build caching (Swatinem/rust-cache CI jobs, the docker-bake
registry-backed layer cache / cargo-chef stages) is as good as it can be,
since Rust builds are still often really long - was deliberately deferred
by operator decision: recorded here as an unscheduled backlog note rather
than a #33 plan task. #33 Task 2 (CI path-gating via dorny/paths-filter,
commit 8120ee3) already removed the cost of Rust builds for non-Rust
changes, so this caching investigation only affects CI runs that
genuinely touch Rust.
