# 16: Production Cutover (hard cutover + break-glass rollback) - Design

> Extracted 2026-07-08 from `docs/plan/16-production-cutover-validation.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Pending

**Revised 2026-07-04:** the original side-by-side plan (legacy stack deployed
to prod at `legacy.brdg.me`, 4-week validation window, cross-system WS
compatibility) is replaced by a **hard cutover**. Rationale: solo operator,
small user base, both systems share the database so rollback is cheap either
way, and the parallel period's main cost was keeping the fat-payload WS
compat system alive in production - which blocked Phase 17. With Phase 17
now running pre-cutover, go-live happens on the final architecture.

**Data model of the cutover (clarified 2026-07-05):** old prod (Linode) and
the new stack (DOKS) never run against a shared database and never serve
users simultaneously. The sequence is: beta period on an isolated throwaway
database → stop legacy → dump/restore prod data into CNPG (Phase 19
procedure) → point DNS at the new stack. Anything written to the old system
after the dump is lost by design; the freeze step in the cutover plan exists
to make that window zero.

## Image naming

The old React frontend (`web/Dockerfile`) and the new Leptos SSR app
(`rust/Dockerfile` `web` target) previously shared the image tag `brdgme/web`.
The new Leptos app keeps `brdgme/web`. The old React frontend is renamed to
`brdgme/web-legacy`.

## Legacy stack: dev + break-glass only

The legacy manifests and image builds (see the implementation plan) were
completed for the original side-by-side plan. They are retained, but their
role changes: `LEGACY=1` dev mode in Kind, and the break-glass rollback
bundle for prod. They are **not** included in the prod kustomization and get
no prod hostnames or DNS records (`legacy.brdg.me`/`api.brdg.me`/`ws.brdg.me`
are not created in `infra/dns.tf`).

## Beta period (added 2026-07-05: isolated database, pre-cutover)

Michael wants a short beta on the production cluster before real traffic,
against a **completely isolated database** - the fresh CNPG database the
cluster boots with, wiped at cutover by the Phase 19 import. No legacy
data, no shared state with old prod, which keeps serving users untouched.

## Rollback procedure

**None (decided 2026-07-08, recorded in #31):** no simultaneous
deployments and no supported rollback paths. Solo side project,
friends-only user base - downtime is acceptable and operator effort is
the scarce resource. If cutover goes badly, fix forward. Incidentally,
the Linode box still exists until its decommission and could be revived
by hand (apex DNS TTL is 300 at cutover), but this is not a maintained
or tested procedure; anything written to the new system since cutover
would be lost. (The previous two-path break-glass design - Linode revert
+ a `k8s/prod-rollback/` legacy-in-DOKS overlay - is superseded; the
overlay was never built.)
