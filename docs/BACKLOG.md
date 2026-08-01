# brdgme Product Backlog

This file is the backlog index. Each work item's design/decisions live in
`docs/changes/<active-change>/spec.md` and its tasks/runbook live in
`docs/changes/<active-change>/plan.md` (either may be absent if the item has
no content of that kind); `NN` is a **permanent ID in assignment
order** - it never changes and never implies execution order. Priority lives
in the ordered list below (historically these items were called "phases";
prose saying "Phase NN" means item `NN`).

When an item is fully done/cancelled/superseded, move its row out of the
Status table below into [`docs/archive/BACKLOG.md`](archive/BACKLOG.md),
adding a Resolution and the date resolved - that file is append-only and
keeps this one from filling up with closed work.

**Priority order (updated 2026-07-26):**
**Immediate next:** #31 Rust-only repository (delete legacy trio + brdgme-go,
lift `rust/` to root; #23 game ports now complete so WP3-5 unblocked).
**Then:** #52 managed Postgres migration (CNPG to DO Managed Database, frees
~600Mi cluster memory), #50 dev environment reassessment (make local dev viable
again - kubernetes/kind/tilt too heavy; consider docker compose or partial
deployments), #15 tails (CI deploy job, delete stale k8s/argocd/, admin-password
rotation), #54 maximum-performance fuzzer (after #31, which reworks the workspace
layout the fuzz bins sit on; a faster fuzzer makes every subsequent game port and
remediation package cheaper to validate).
**Unscheduled post-go-live:** #27 remainder (WebSocketTrigger deletion, flaky
NATS tests), #36 Web Push, #37 game verification, #38 "new version" notification,
#40 DB tests opt-in, #46 turn timer, #48 moderation, #49 sqlx macros (review first),
#51 bot sqlx queries, #55 dependency/toolchain currency pass.

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

Fully done/resolved/superseded items (1-14, 17-26, 28-30,
32-34, 41-45, 47, Quick wins, Review findings 2026-07-04, Development Workflow)
have been moved to [`docs/archive/BACKLOG.md`](archive/BACKLOG.md).

| # | Title | Status | Spec | Plan |
|---|---|---|---|---|
| 15 | Production CD (ArgoCD) | Live 2026-07-08 - ArgoCD + sealed-secrets running in prod, first fully-green sync at brdgme@851e23c; remaining: CI deploy job, delete stale k8s/argocd/, admin-password rotation | [spec](changes/15-production-cd-argocd/spec.md) | [plan](changes/15-production-cd-argocd/plan.md) |
| 27 | rust/web Simplification (remainder) | WP1/2/3b/4/5 done; remaining: (a) WP3a delete WebSocketTrigger - dual-signal duplication at every update site, needs re-scoping since trigger now carries proposal/friend updates and public index keys on it; (b) harden 2 flaky NATS-timing tests still `#[ignore]`d (`game/mod.rs:1078`, `websocket.rs:191`) | [spec](changes/27-web-simplification/spec.md) | [plan](changes/27-web-simplification/plan.md) |
| 31 | Rust-Only Repository (delete legacy trio + brdgme-go, game shelving lifecycle, lift `rust/` to root) | Ready 2026-07-08 - no-rollback decision made, WP1 runnable pre-cutover; WP3-5 gated on #23 Track B | [spec](changes/31-rust-only-repo/spec.md) | [plan](changes/31-rust-only-repo/plan.md) |
| 36 | Web Push turn notifications (service worker, VAPID keys, push subscriptions in Postgres, server-side push on turn change, settings toggle, graceful permission-denied handling) | Pending - post-go-live, bottom of backlog (scoped 2026-07-11; sits alongside #22c turn-reminder emails; no spec yet) | - | - |
| 37 | Rust game port verification testing (operator gameplay pass over all converted Rust games; some observed misbehaving 2026-07-11 - see History for the full game list) | Pending - downgraded 2026-07-17 (operator: games seem okay, does not block go-live) | - | - |
| 38 | "New version released" notification (detect when a new version has gone live and show a message to users with the site open so they know to refresh; cache busting itself handled by Cloudflare) | Pending - unscheduled; scope narrowed 2026-07-24 to detection + user-facing message only; must include viability/suitability review first to confirm implementation is not prohibitively complex | - | - |
| 40 | DB tests run (and fail) by default (every local/agent test run hits DB test failures, repeatedly surprising agents; investigate whether DB-dependent tests should be opt-in - e.g. feature/env gated - instead of opt-out, or made to pass by default) | Pending - unscheduled; added 2026-07-15. Addendum 2026-07-16 resolved: the `cargo sqlx prepare` failure was the `User` struct lacking `theme`/`is_admin` (migration 007/008) - fixed by 212f1db, which added both fields and regenerated `.sqlx`. The related `invite_policy` concern is also resolved: `User` queries now list columns explicitly instead of `SELECT *`/`RETURNING *`, so `invite_policy` deliberately staying off the `User` struct (see `get_invite_policy` in `rust/web/src/db.rs`) is safe and won't silently break `cargo sqlx prepare` again. Addendum 2026-07-29 (2026-07-23 Rust review, T3-B7 bo F9): the bot's nullable-column decode pattern (`try_get::<Option<T>,_>(..).context(..)?` in `rust/bot/src/config.rs` and `rust/bot/src/main.rs`) is correct and landed but has no non-DB unit test - the decode path only runs against a live `PgPool`. A sqlx-mock / row-fixture harness for the bot would close this; same underlying problem as the DB-tests-opt-in question above. | - | - |
| 46 | Turn timer to prevent dead games (idea sketch: some timer before the easy bot makes a play on the player's behalf? 3 strikes before forced concede?) | Captured 2026-07-17 - post-go-live, needs brainstorming; interacts with #47 bot replacement and #43 bot configs | - | - |
| 48 | Basic moderation - initially just usernames, may extend to more later | Captured 2026-07-17 - post-go-live, needs brainstorming | - | - |
| 49 | Convert #30 friends plain sqlx queries to compile-time-checked macros now that the .sqlx prepare workflow is healthy (the friends-related queries in `rust/web/src/db.rs` currently use plain (runtime-checked) `sqlx::query`/`sqlx::query_as` calls by old convention; convert them to `query!`/`query_as!` macros and add cache entries via `cargo sqlx prepare` per docs/DEV.md) | Captured 2026-07-18 - unscheduled; **note 2026-07-24: review first to ensure direction is correct and appropriate before executing** | - | - |
| 51 | Bot dynamic sqlx queries mask schema drift (`rust/bot/src/main.rs` + `config.rs`: `row.try_get(..).unwrap_or(..)` turns query bugs into silent wrong behaviour - e.g. broken `is_turn` column makes bot silently never play; switch to checked `sqlx::query!` macros or replace `unwrap_or` defaults with `.context(..)?`) | Captured 2026-07-24 (split from Bug fixes plan); unscheduled | - | [plan](changes/51-bot-sqlx-queries/plan.md) |
| 50 | Dev environment reassessment - review the whole dev env with a view to making it viable to run locally again; the kind/tilt kubernetes setup is too heavy now (host crashes, 32GB RAM minimum per AGENTS.md); consider docker compose or allowing partial deployments | Captured 2026-07-19, prioritised 2026-07-24 | - | - |
| 52 | Managed Postgres migration - move prod Postgres from in-cluster CloudNativePG to DigitalOcean Managed Database (~$15.15/mo, 1GB/1vCPU/10GB, syd1, private VPC); provision via OpenTofu, load prod data directly into managed instance, then remove CNPG resources (frees ~600Mi cluster memory); dev unaffected (keeps local Postgres) | Approved 2026-07-21, prioritised 2026-07-24; supersedes archived #19 | [spec](changes/52-managed-postgres/spec.md) | [plan](changes/52-managed-postgres/plan.md) |
| 53 | Game rules review (parity park) | **PARKED - awaiting a product decision (per-game rules review).** The 2026-07-23 Rust review found ~30 port-parity findings (code vs official rules vs RULES.md). The global port-parity policy (D-35: official rules authoritative, no gameplay change without per-game sign-off, parked) now lives in [`docs/decisions/PORT_PARITY.md`](decisions/PORT_PARITY.md). The detailed per-rule parity list (D-26..D-32, D-34) was NOT migrated and is retained only in git history under the review's commit range (`f0589894c1937c2c1134cf99523f1fd4e9a8f944..868094a6c8177858dededdd5321ce0c03882ada5`, file `docs/reviews/2026-07-23-rust-review/planning/DECISIONS.md`). The review's surviving summary is [`docs/reviews/2026-07-23-rust-review/SUMMARY.md`](reviews/2026-07-23-rust-review/SUMMARY.md). All of these are PARKED-PENDING-USER-RULES-REVIEW: some RULES.md content was AI-generated and may be wrong, and edition/variation choices are a product decision. **No gameplay change without per-game sign-off.** Packages WP-11, WP-12, WP-16, WP-20, WP-26, WP-30 are BLOCKED-ON-USER-RULES-REVIEW - implementing agents must not pick them up. Rules-content work WP-74 (red7-1 empty-hand elimination sentence) and WP-75 (RULES_AUTHORING strategy-docs rewrite) is also queued behind this park. Three cases (a F1, b F7, e F30) are flagged for immediate fix and are outside the park; b F4 was re-parked and d F37 was rejected as not-a-bug. Liveness fixes WP-15/WP-25 are NOT parked. | - | - |
| 54 | Maximum-performance fuzzer (three independent modes) | Captured 2026-07-26 from the 2026-07-23 Rust review (D-51); scheduled 2026-07-26 into the Then tier after #31 (D-53), nothing measured. Rationale (D-43): "the value of the fuzzer is basically directly correlated by how fast it can run and how many games it can pump through." Three modes on two **independent** axes - renders and serialisation: (1) **game logic only** (default, maximum speed) - game kept live in memory, no serialisation, no rendering, drives `Gamer` directly; (2) **opt-in renders** - pub render plus all private renders after every successful command (stricter than the current loop); (3) **opt-in serialisation** - the "end to end fuzz" exercising the full `api::Request`/`api::Response` path as today. Key findings: the in-process path is **not** serialisation-free - every move already does a full state decode + encode, a pub render, every player's state JSON and N+1 markup renders, of which the loop uses only the acting player's `command_spec` and the opaque state string; `fuzz()` is **already** parallel across `num_cpus::get()` threads, so parallelism is not an available win; two free wins sit in `Fuzzer::command` - a whole-`PlayerRender` clone taken for one field, and a full state-string clone, both per move. Tradeoff: the current loop catches render-panics and serialise-panics for free, and mode 1 gives that up - which is exactly why modes 2 and 3 exist. **Nothing has been measured** (no cargo available in the planning session); the settling commands are in section 6 of the full analysis: [`docs/fuzz-throughput-evaluation.md`](fuzz-throughput-evaluation.md) | - | - |
| 55 | Comprehensive dependency and toolchain currency pass (first run of the recurring process) | Captured 2026-07-29; unscheduled. First execution of the process defined in [`docs/DEPENDENCY-CURRENCY.md`](DEPENDENCY-CURRENCY.md): audit rustc pin (`rust/rust-toolchain.toml`) and tooling vs latest stable, `cargo outdated -R --depth 2` for direct+transitive deps, throwaway-branch breakage assessment (bump everything including majors), then prioritised upgrade commits (rustc pin first, advisories, framework majors, compatible sweep, tail majors) with a duplicate-cluster re-audit at the end. Resolves the parked tower-http/gloo-timers duplication question (2026-07-23 review dp F9) by holding latest and letting the old-version consumers catch up. | - | - |
| 56 | Email escape-hatch verb set (WP-85) | **DEFERRED - awaiting a product decision.** Carved out of the 2026-07-23 Rust review's WP-59 (D-54) and deferred (D-55): D-15 settled the inbound-email dispatch design (game command parser runs first, platform commands are the fallback) but carved out a small hard-reserved set of escape-hatch verbs (`help` and equivalents) that always wins even on the game path; which verbs belong to that set is deliberately undecided. This is a different block from the rules-review park (#53) and must not be folded into it. Accepted cost of the deferral: the acquire-1 / starship-catan-1 top-level `end` move stays unplayable by email until this lands. Surfaced during the 2026-07-23 Rust review. | - | - |
| 57 | unsafe-libyaml still in Cargo.lock (dp-F14 backend half) | Open tech debt from the 2026-07-23 Rust review. The front half of dp-F14 landed (WP-70 migrated bot + game_client from the archived `serde_yaml` to `serde_yaml_ng`, output byte-identical), but the backend half is unresolved: `unsafe-libyaml` 0.2.11 (the C libyaml binding) remains in `Cargo.lock` via `serde_yaml_ng`. Removing it means severing the libyaml-backed parser (e.g. a pure-Rust YAML path). Surfaced during the 2026-07-23 Rust review. | - | - |
| 58 | Non-default build targets are not CI-gated | Open tech debt surfaced during the 2026-07-23 Rust review remediation. Two pre-existing gaps: (a) `cargo test -p web` without the `ssr` feature has never compiled (~323 errors; the integration tests require `ssr`), so the non-ssr web target is untested and ungated; (b) wasm-target clippy with `-D warnings` fails on four pre-existing lints (`unused_unit`, `collapsible_if`) and is never CI-gated (wasm builds via cargo-leptos while clippy runs ssr-only). Either gate these targets in CI or fix-and-gate them. Surfaced during the 2026-07-23 Rust review. Update 2026-07-30: the four (b) wasm-hydrate clippy lints (`unused_unit`, `collapsible_if`) in `websocket_client.rs` were fixed, so `cargo clippy -p web --features hydrate --target wasm32-unknown-unknown -- -D warnings` now passes; gating that target in CI and part (a) remain open. | - | - |

---

## Human tasks (operator-only, in rough execution order)

Everything below needs the operator (accounts, credentials, production access);
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
3. **#16 cutover:** lower TTLs (`tofu apply`); announce downtime; stop
   the Linode stack; real `pg_dump`/restore + migrations; repoint apex
   DNS (`tofu apply`); smoke test; flip the uptime monitor to apex.
   (The `postgres-config`/`postgres-rw` host is handled at Phase 15
   sealing time, not cutover - revised 2026-07-06.)
4. **#16 decommission (after the validation week):** decommission the
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

2026-07-08: `docs/plan/` retired in favor of per-change spec/plan documents -
each item's design/decisions moved to `docs/changes/<active-change>/spec.md`,
its tasks/runbook to `docs/changes/<active-change>/plan.md` (point-in-time
records, not living documents).

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
(operator decision): drop the client-IP/PROXY-protocol work entirely - no DO
support ticket, no retry planned; real client IPs are simply not
available to the app on this platform, so per-IP app-level limits stay
one collective bucket (keyed on the LB SNAT address) and XFF-spoofable
permanently. With this dropped, #14 has no remaining work and moves to
the archive as fully done. #28 WP1-3 (app-level hardening: DB-backed
send caps + per-code attempt caps, IP-independent) is promoted to
pre-go-live priority as the effective protection in place of the flip;
WP4 (Cloudflare edge, which sees real client IPs) stays post-cutover.
See `docs/changes/archive/2026-07-05-14-drop-knative-gateway-api/plan.md`,
`docs/changes/archive/2026-07-08-16-production-cutover-validation/plan.md`,
and `docs/changes/archive/2026-07-08-28-abuse-protection/spec.md`
for detail.

2026-07-09: #18 hardening closed - full Grafana Cloud observability
(Alloy log/metric/trace shipping with volume cuts, OTLP tracing at 10%
sampling, /metrics, probes incl. operator /healthz) implemented in-tree;
WASM source maps descoped (toolchain blocker); contact point
(the site admin) + external uptime monitor done. Moved to the
archive as fully done; remaining rollout (deploy, quota window,
alert-rule creation) removed from the backlog - the operator tracks it
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
The operator initially considered and rejected pulling WP4 (Cloudflare) ahead
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
(`docs/changes/archive/2026-07-10-28-wp4-cloudflare-pre-golive/spec.md`,
plan `docs/changes/archive/2026-07-10-28-wp4-cloudflare-pre-golive/plan.md`).
Single-stage migration: the operator created the CF zone (free plan, existing
account), CF copied the DO records at zone creation, and the registrar
nameservers were cut over to Cloudflare the same day - so the Tofu work is
adoption/import of the live zone, not creation, and beta.brdg.me is
already proxied. Key redesign call (spec W6): once the CF edge rate-limit
rule is proven on beta, the in-app per-IP rate limiting is DELETED
(`rate_limit.rs`, governor deps, `extract_client_ip`) rather than
re-tightened via a `CF-Connecting-IP` carve-out - WP1's DB-backed caps
remain the backstop for direct-to-LB traffic, and WP2's hygiene middleware
stays (W9). The old plan's WP4 section is superseded in place. Separately,
#32 (Alloy OTLP export) demoted to post-go-live (operator: the Grafana
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

2026-07-11 (beta testing): #37 added - the operator reports some of the games
ported to Rust appear to have problems, from a beta testing pass on the
deployed #33 batch (deploy sha-48686c8). Item is a checklist to do a full
operator gameplay pass over every already-converted Rust game. Authoritative
list compiled from the `rust/Cargo.toml` workspace members and the Tiltfile
"Rust games" `docker_build` loop (both deployed via `k8s/base/game/`),
excluding `lords-of-vegas-1` (implemented but intentionally not deployed,
see Out of Scope above) - 15 games: acquire-1, battleship-2, category-5-2,
farkle-2, for-sale-2, greed-2, jaipur-2, liars-dice-2, lost-cities-1,
lost-cities-2, no-thanks-2, sushi-go-2, sushizock-2, tic-tac-toe-2,
zombie-dice-2. Note `acquire-1` and `lost-cities-1` are native Rust `-1`
editions (no Go predecessor), not Go-replacement `-2` conversions - both
still count as Rust games in scope for this testing pass. Same testing
pass also produced four new jank entries appended to
docs/pre-go-live-polish.md: favicon grey too light (`#606060` fix already
in the operator's working tree), game log sections (recent-logs panel + sidebar)
still flashing on command submit, a reusable centered loading spinner
needed for initial game page load, and disabling the command input/send
button while a command is submitting. These are recorded for a future #33
continuation session, not actioned now.

2026-07-13: #26 theming core implemented end-to-end (28-phase serial
run, plan `2026-07-13-26-theming-semantic-colors.md`): 12-slot palette +
`soften`/`contrast` transforms (THEMING.md revised), all 23 games on
named colours, `ColType::RGB` removed from the AST, semantic-class web
renderer with per-theme CSS custom properties, brdgme light/dark +
Dracula themes with a contrast gate test, system-theme default with
instant client-side switching, migrations 006 (player colour palette)
and 007 (user theme) written but not yet run. Decisions D1-D15 in the
plan need operator review. **Web chrome theming is the immediate next
work item** (operator decision 2026-07-13: critical for wrapping up
#26) - `main.scss` still hardcodes ~20 chrome colours. Also found:
lords-of-vegas-1 `shuffled_deck` iterates HashMap keys pre-shuffle, so
seeded starts are non-deterministic across processes (pre-existing bug,
unscheduled).

2026-07-13 (later): web chrome theming shipped (f185ae5, plan
`2026-07-13-26-web-chrome-theming.md`), closing D11. Follow-up noted
(operator, not for this session): `THEME_BOOT_SCRIPT` - the inline
minified pre-paint cookie-reading script in `rust/web/src/app.rs` -
reads like a malicious injection at first glance even though review
shows it is fine. Find a cleaner approach (e.g. readable source
minified/embedded at build time, an external same-origin script file,
or an SSR-set attribute from the cookie on the request) that keeps the
no-flash-before-first-paint behaviour.

2026-07-15: #40 added - every local/agent test run produces DB test
failures (DB-dependent tests fail without a database), which repeatedly
surprises agents mid-task. Investigate whether DB tests should be opt-in
rather than opt-out, or made to pass by default. An agent-facing warning
was added to AGENTS.md (Working style) the same day.

2026-07-11: #38 added - investigate frontend cache busting when a new
version is bumped in brdgme-config: browsers may keep serving stale
WASM/JS/assets after a deploy. Candidate approaches, simplest-first: force
a reload in clients whenever a new version is deployed, or surface a "new
version released, please reload" message to the user; the underlying
cache-busting story (hashed asset filenames / cache headers) should be
investigated as part of the same item.

2026-07-17: post-go-live one-liner captures from the operator, recorded for
later brainstorming (not designed or scheduled): #44 "New game" screen
usability, #45 rich index page for new visitors, #46 turn timer against
dead games, #47 concede in >2-player games with bot replacement, #48
basic moderation (usernames first). Of the same list, four were already
captured and got no new item: full email availability folded into #22's
scope as a 2026-07-17 note (22b must cover game creation and player
settings, not just plays), game history and stats = #29, add-friend =
#30, game invites instead of auto-starting = #24.

2026-07-17: #43 bot efficacy added (post-go-live, slotted after #25 rules
rendering since the per-game doc split must coordinate with #25's
single-source RULES.md design). Motivation (operator): bots have trouble
parsing the full game render, and it is a large noisy prompt payload -
lean on player and public data instead for more signal and less noise;
reduced AI API spend is a welcome side benefit but bot quality is the
main goal. Scope: documented player/public data contracts per game
(player data = exactly what the player sees, public data = exactly what
a spectator sees - no more, no less); RULES.md split into pure rules /
optional render-explanation doc (EXAMPLES.md, need unconfirmed) /
player-public data doc / human-accessible strategy doc; bots prompted
with only the rules, the data documentation, and both data payloads (both
so the bot can tell public from hidden information); difficulties become
complete bot configs rather than prompt wording (Easy deepseek-v4-flash
no thinking no strategy doc / Medium high thinking no strategy doc /
Hard high thinking + strategy doc, full per-difficulty config so
providers can vary per bot); future work flagged for admin GUI bot-config
management (add/remove/switch, priority failover, shared-priority load
balancing, enable/disable). This unparks the bot model configuration
topic parked 2026-07-11.

2026-07-17: second #33 polish batch recorded as the immediate next tasks -
9 entries appended to docs/pre-go-live-polish.md from the operator's beta pass
over the new settings page and gameplay: settings page should scroll only
the main content (sidebar static, app-shell pattern); non-game content
pages widened to ~1220px (Wikipedia-style, fits 3 theme columns); selected
theme indicated by a thicker highlight-colour border instead of name
highlight; theme previews become 2x5 text-free swatch blocks (5 spaces
each, NamedColor::ALL accent order, fg/bg excluded); username shows stale
value when returning to settings after a change (must not be cached
anywhere); ELO rating change rendered next to the rating at game end
(match legacy brdg.me); command input sometimes self-clears while typing
(suspected websocket update on the open game or a sidebar game -
investigate/reproduce/fix); sub menu button still not showing on mobile
game pages despite the 2026-07-11 fixes (should be U+22EE, right-aligned
in the title bar - check for regression or unshipped fix);
invalid-command rejections surface as "error running server function:
<msg>" with HTTP 500 (`ServerError|expected buy or done` observed on
beta 2026-07-16) - should render as "Invalid command: <msg>" and be a
4xx typed user-input error, not a generic ServerFnError. Documented
only - not actioned; a #33 continuation plan is needed before execution.

2026-07-16: #35 settings page implemented end-to-end (spec + plan same
day, all uncommitted). #34 partial - migration 008 adds `users.is_admin`,
bump-bot-to-play made admin-only. Preferred colours now honoured at game
creation (`choose_colors` in `rust/web/src/db.rs`, with legacy
Amber->Orange / BlueGrey->Cyan normalization). CSS-404 asset-caching fix:
`<HashedStylesheet>` replaces the hardcoded `/pkg/web.css` link, and the
immutable cache header is now only set on successful `/pkg/` responses -
a Cloudflare-cached 404 for the hashed CSS was the symptom; see
[docs/decisions/ASSET_CACHING.md](decisions/ASSET_CACHING.md). Game-page
command-input auto-focus loosened - typing only skips focusing the
command input when a text-entry element is focused, Space stays
BODY-only. Also discovered `cargo sqlx prepare` fails on the missing
`User.theme` field (recorded under #40).

2026-07-17 (later): second #33 polish batch implemented end-to-end, all
9 tasks in `docs/changes/archive/2026-07-17-33-pre-go-live-polish-2-plan/plan.md`
committed to master: header sub menu button now shows below 60em (missing
CSS override added); settings/content pages scroll only the main content
pane (app-shell pattern) and widened to a shared ~1220px centered layout;
selected theme indicated by a thicker highlight-colour border instead of
name highlight; theme previews are now 2x5 text-free swatch blocks
(`NamedColor::ALL` accent order, fg/bg excluded); username no longer goes
stale after a change (session copy refreshed in `set_username`, fresh read
in `get_settings`); ELO rating change now renders next to the rating at
game end (legacy-parity icon/sign); invalid-command rejections now return
a typed `UserError` and render "Invalid command: <msg>" instead of a raw
`ServerFnError`/HTTP 500; command input no longer self-clears on
background game updates (root cause was a global `game_update` signal
re-keying the `<Transition>` and remounting `GameCommandInput` on
unrelated-game updates - fixed by hoisting command text above the
remounting closure). Only Task 9 Step 6 (manual reproduction/verification
on the deployed beta) is left, deliberately, for the operator.

2026-07-17 (later still): third #33 polish batch implemented end-to-end,
all 3 tasks in
`docs/changes/archive/2026-07-17-33-pre-go-live-polish-3-plan/plan.md` committed
to master: a reusable `Spinner` component extracted from the login form
and used by `GamePage`, whose `<Transition>` is now remounted via a
deduped game-id memo so a centered spinner shows on initial load and on
game-to-game navigation but never on seq-only WS/command refetches of the
already-visible game; the command input and Send button are disabled
while a command is submitting, refocus the input on both success and
error, and keep the typed text on error instead of clearing it; and the
autocomplete suggestion click handler now refocuses the command input
after inserting the word. Manual browser verification of all three,
including the initial-load/navigation spinner behaviour, the
disabled-while-submitting state, and the refocus-on-click behaviour, is
left for the operator.

2026-07-20: #43 bot efficacy implemented end-to-end. Migration 013 adds
bots/llm_providers/bot_providers tables (three-layer enable gate,
round-robin + failover routing, AES-256-GCM credential encryption);
game_bots.difficulty renamed to bot_name (no longer constrained to
easy/medium/hard); game_versions.interface_version added. Game interface
V2: DataDocs/BasicStrategy/AdvancedStrategy endpoints, Gamer trait
methods with default empty impls, game_client fetch_game_data()
abstracts V1/V2 (callers never see versions). Bot crate restructured:
DB config load with env fallback, ProviderRouter, structured YAML
prompt (render removed entirely, static system + dynamic user split for
KV cache). All 27 Rust games upgraded to V2 with per-game DATA_DOCS.md,
BASIC_STRATEGY.md, ADVANCED_STRATEGY.md. Operator reads
interfaceVersion from GameVersion CRD. Game creation UI reads bot
options from bots table.

2026-07-20: #23 four more games ported from Go: red7-1 (49-card palette
game, 2-4p), alhambra-1 (tile-laying with currency, 2-6p, incl. Dirk
bot), starship-catan-1 (2p space Catan - fixes Go Winners() bug
(returned same player for both branches) and die-range off-by-one
(%3+1 = {1,2,3} not {1,2,3,4})), seven-wonders-1 (card drafting civ,
3-7p, simultaneous turns). New shared crate brdgme_cost (generic
Cost<K> + can_afford_perm; fixes Go Cost.Drop bug - iterated index not
values). splendor-2 keeps its inline cost.rs (accepted, two coexisting
impls).

2026-07-20: polish batch - sidebar user link (username links to
profile), profile colour ribbon, random pref colours on signup, inline
Save button, add-friend visibility (hidden for existing friends/pending
requests), duplicate player error message, 9th player colour (Yellow),
Texas Holdem bet display (Current bet + Your bet), bot rating exclusion
fixed (apply_rating_changes skipped ENTIRE game if any bot present -
humans in bot games never rated; now excludes only bots from rated
set). Discoveries: reciprocal friend request auto-accept was already
implemented (no change needed); bot was receiving full render but
structured player_state/pub_state already available in Status response
(motivated #43).

2026-07-21: #22 (email via Resend) fully complete - 22b play-by-email
(inbound email parsing, game creation/plays/settings via email), 22c
turn-reminder emails, and 22d multi-email switching all implemented.
#24 game invites complete (invite flow with accept/decline, policy
enforcement). #25 rules rendering complete (web UI + email,
single-source RULES.md with render-time specialization).
