# 16: Production Cutover (hard cutover + break-glass rollback)

**Status:** Pending

**Revised 2026-07-04:** the original side-by-side plan (legacy stack deployed
to prod at `legacy.brdg.me`, 4-week validation window, cross-system WS
compatibility) is replaced by a **hard cutover**. Rationale: solo operator,
small user base, both systems share the database so rollback is cheap either
way, and the parallel period's main cost was keeping the fat-payload WS
compat system alive in production - which blocked Phase 17. With Phase 17
now running pre-cutover, go-live happens on the final architecture.

**Goal:** point `brdg.me` at the new system. The legacy stack (`rust/api`,
`web`, `websocket`) is **never deployed to production** - it remains
buildable in the repo as a break-glass rollback until the validation gate
passes.

**Preconditions (the go-live stack, all pre-cutover):** Phase 21 (OpenTofu),
Phase 22a human steps (Resend domain), Phase 14 prod prerequisites,
Phase 13 (NATS bot eventing), Phase 17 (NATS WS + skinny payloads),
Phase 19 (CNPG), Phase 15 (ArgoCD + sealed-secrets), Phase 18
(Grafana Cloud observability + alerting + external uptime monitor).

**Data model of the cutover (clarified 2026-07-05):** old prod (Linode) and
the new stack (DOKS) never run against a shared database and never serve
users simultaneously. The sequence is: beta period on an isolated throwaway
database → stop legacy → dump/restore prod data into CNPG (Phase 19
procedure) → point DNS at the new stack. Anything written to the old system
after the dump is lost by design; the freeze step below exists to make that
window zero.

**Delegation note:** operator-driven by nature (production deploys, DNS,
live verification) - not agent-delegable. The agent-delegable subtask is the
final source/manifest deletion in the decommission list (ready once the
validation gate passes).

### Image naming

The old React frontend (`web/Dockerfile`) and the new Leptos SSR app
(`rust/Dockerfile` `web` target) previously shared the image tag `brdgme/web`.
The new Leptos app keeps `brdgme/web`. The old React frontend is renamed to
`brdgme/web-legacy`.

### Legacy stack: dev + break-glass only

The legacy manifests and image builds below were completed for the original
side-by-side plan. They are retained, but their role changes: `LEGACY=1` dev
mode in Kind, and the break-glass rollback bundle for prod. They are **not**
included in the prod kustomization and get no prod hostnames or DNS records
(`legacy.brdg.me`/`api.brdg.me`/`ws.brdg.me` are not created in
`infra/dns.tf`).

- [x] New Leptos app: `rust/Dockerfile` `web` target → `brdgme/web`. k8s
      manifests in `k8s/base/web/` unchanged.
- [x] `brdgme/web-legacy` image build in the Tiltfile (from
      `web/Dockerfile`, final stage `web`, tagged `brdgme/web-legacy`).
- [x] `brdgme/api` and `brdgme/websocket` image builds in the Tiltfile.
- [x] `k8s/base/web-legacy/` manifests (Deployment + Service).
- [x] `k8s/base/legacy/kustomization.yaml` grouping `web-legacy`, `api`,
      and `websocket` as the legacy stack.
- [x] Legacy React frontend API URL derivation confirmed: `http.ts` replaces
      the first subdomain with `api` (`legacy.brdg.me` → `api.brdg.me`).
- [ ] **Superseded 2026-07-08 (do not do):** the break-glass overlay below.
      Decided in #31: no simultaneous deployments, no rollback support -
      the legacy stack is deleted pre-cutover (LEGACY=1 dev mode included),
      and the decommission source-deletion items below move to #31 WP1.
      See [31-rust-only-repo.md](31-rust-only-repo.md).
- [ ] ~~Break-glass overlay~~: a `k8s/prod-rollback/` kustomization that deploys
      the legacy trio + Redis + an HTTPRoute set serving the legacy frontend
      from apex `brdg.me` with `api.brdg.me`/`ws.brdg.me` routes (the
      `http.ts` subdomain derivation means apex serving must produce
      `api.brdg.me` - verify this apex case, it was never confirmed; if it
      fails, the overlay must serve from a subdomain and repoint apex DNS).
      Not applied in normal operation; exists so rollback is `kubectl
      apply -k` + route flip, not archaeology. Push the three legacy images
      to GHCR once so the overlay is deployable without a local build.
- [ ] Superseded, do not do: prod deploy of the legacy trio, legacy prod
      hostnames/DNS/TLS, cross-system WS validation. (The Knative-era [x]
      items previously listed here - DomainMappings, config-domain, Kourier
      TLS - were already superseded by Phase 14.)

### Beta period (added 2026-07-05: isolated database, pre-cutover)

Michael wants a short beta on the production cluster before real traffic,
against a **completely isolated database** - the fresh CNPG database the
cluster boots with, wiped at cutover by the Phase 19 import. No legacy
data, no shared state with old prod, which keeps serving users untouched.

- [ ] Before creating the Gateway: enable PROXY protocol on the Cilium
      side FIRST *(human)*. Wrong order (DO-LB annotation before the
      Cilium flip) sends PROXY-protocol bytes to an Envoy not yet
      expecting them and breaks all traffic. See
      docs/plan/14-drop-knative-gateway-api.md prod-prerequisites section.
      (Moved here from the cutover steps - the Gateway is now created at
      beta start, so beta gets to verify the login rate limiter sees real
      client IPs.) Steps:
      1. Backup: `kubectl -n kube-system get configmap cilium-config -o
         yaml > cilium-config-backup.yaml` (keep locally, don't commit).
      2. `kubectl -n kube-system patch configmap cilium-config --type
         merge -p '{"data":{"enable-gateway-api-proxy-protocol":"true"}}'`
      3. `kubectl -n kube-system rollout restart daemonset/cilium &&
         kubectl -n kube-system rollout status daemonset/cilium`
      4. Confirm the value stuck: re-read the ConfigMap now, ~15 minutes
         later, and again the next day (watching for DOKS's reconciler
         reverting it). If it reverts, STOP - do not proceed to the
         annotation - and open a DO support ticket.
      5. Only then uncomment
         `do-loadbalancer-enable-proxy-protocol: "true"` in
         `k8s/base/gateway/gateway.yaml` (commit; ArgoCD syncs it).
- [ ] Deploy the full new stack via ArgoCD (Phase 15) onto CNPG (Phase 19,
      fresh database), NATS (13/17), Gateway API (14).
- [ ] Beta hostname: validates the new cluster end-to-end pre-cutover (DNS,
      Gateway, cert-manager, the app itself) against the new Gateway LB,
      while the `brdg.me` apex stays pointed at the legacy Linode host
      until the cutover steps below run. `beta.brdg.me` HTTP(S) listeners
      + HTTPRoutes added to the Gateway manifests, and the `beta_a`
      `digitalocean_record` added to `infra/dns.tf` pointing at the
      Gateway LB IP (agent-delegable, done). Remaining, *(human)*:
      1. `tofu plan` (expect exactly 1 add) → `tofu apply`.
      2. Verify TLS issuance: `kubectl get certificate -n brdgme` reaches
         `Ready`, and `curl -v https://beta.brdg.me/` serves with a valid
         Let's Encrypt cert - this also proves the issuance path apex will
         use.
- [ ] Point the external uptime monitor (Phase 18) at `beta.brdg.me`.
- [ ] Exercise during beta: login via Resend (in-app path - closes the 22a
      remaining check), a full game vs a bot, a two-account human game with
      live WS, restart/undo/concede; confirm logs, metrics, and traces all
      arrive in Grafana Cloud and the rate limiter keys on distinct client
      IPs (check the peer address the login handler logs).
- [ ] Run the Phase 15 PreSync failure verification and the Phase 19
      dump/restore rehearsal + PITR restore verification while the database
      is still disposable.

### Cutover steps

- [ ] Days ahead: lower TTLs on the legacy apex/`mail` records in
      `infra/dns.tf` from 3600 to 300, `tofu apply` (bounds the split-DNS
      window at the flip to ~5 minutes).
- [ ] Announce the maintenance window to players (email via Resend or a
      notice on the old site - operator's choice).
- [ ] **Freeze:** stop the legacy stack on Linode (downtime begins; users
      see the old site down, not a stale copy taking doomed writes). The
      exact stop commands depend on how the Linode box runs the services
      (systemd units / docker / k8s) - **write them down during the Phase
      19 test-import session** (you'll be on that server anyway) and
      record them here so cutover day is copy-paste. Do NOT stop Postgres
      itself - the dump needs it.
- [ ] Run the Phase 19 dump/restore import into CNPG (drops the beta data),
      apply migrations, verify counts + login.
- [ ] Re-add the apex `brdg.me` listeners (HTTP redirect + HTTPS) and the
      `web` HTTPRoutes to `k8s/base/gateway/` - they were removed 2026-07-08
      (commit 3186371) because their HTTP01 challenges could never complete
      pre-cutover, leaving certs permanently pending. Only apex comes back:
      `legacy`/`api`/`ws` are gone for good (#31). No ClusterIssuer change
      needed - its single solver has no `sectionName` and covers any
      hostname. Commit; ArgoCD syncs it.
- [ ] Repoint apex: update the `brdg.me` A record in `infra/dns.tf` to the
      Gateway LB IP, `tofu apply`. Verify apex TLS issuance (the `brdg.me`
      listener's HTTP01 solve can only complete once DNS points here).
- [ ] Smoke-test immediately (see validation criteria) with Grafana Cloud
      (Phase 18) open; flip the external uptime monitor from
      `beta.brdg.me` to `https://brdg.me/`.
- [ ] Post-cutover tidy: remove the `beta.brdg.me` listener/route/record;
      restore TTLs to 3600 once stable.

### Validation criteria (gate for decommission)

Validation window: **one week** of production traffic (was 4 weeks under
the side-by-side plan; shortened 2026-07-04 with the hard-cutover decision).

- [ ] Every user-facing flow exercised on the new system in production: login
      (email + code, via Resend), game creation (human opponents and bot
      slots), command submission with autocomplete, undo, concede, restart,
      mark-read, game logs, sidebar active games, live WebSocket updates
      (NATS skinny path).
- [ ] At least one game of each deployed game type (Rust and Go) played to
      completion via the new UI.
- [ ] Ratings update correctly on game finish and concede.
- [ ] No unexplained monolith 5xx responses or WASM client panics in the
      window (checked via Grafana Cloud logs + the 5xx alert rule; the
      restart 500 bug was closed could-not-reproduce 2026-07-04 with
      diagnostics improved - if it recurs the error now carries the raw
      payload).
- [ ] Traces arriving for every request class (page load, server fn,
      game-service call) and at least one slow-request trace inspected -
      the APM story works before it is needed in anger.
- [ ] Bots complete turns reliably in production (no stuck bot turns needing
      manual bumps; JetStream redelivery observed working on at least one
      induced failure or verified via consumer metrics).
- [ ] Backups healthy in production (added 2026-07-05 - closes the
      no-backup gap in current prod): daily CNPG base backups and WAL
      archiving observed landing in the Spaces bucket during the window,
      and the Phase 19 PITR restore into a scratch `Cluster` has been
      verified against production data (not just dev).

### Rollback procedure

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

### Decommission (once the validation gate passes)

- [ ] Delete `k8s/prod-rollback/`, `k8s/base/legacy/`, `k8s/base/web-legacy/`,
      and the `api`/`websocket` manifests.
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds and `LEGACY=1` mode from the Tiltfile;
      update DEV.md.
- [ ] Delete `k8s/base/redis/` (kept until now only for break-glass -
      the monolith stopped using Redis in Phase 17).
- [ ] Decommission the Linode server *(human)*. Steps:
      1. Take a final `pg_dump -Fc` from the Linode Postgres and store it
         in the offline archive (belt-and-braces; CNPG backups are now
         the live safety net).
      2. Remove the legacy records from `infra/dns.tf`: the `mail` A
         record and the old apex SPF TXT (the apex A was already
         repointed at cutover). `tofu plan` (expect exactly 2 destroys) →
         `tofu apply`.
      3. Delete the Linode instance in the Linode console (and any
         attached volumes/backups billing).
      4. This also kills the first-hours rollback path - only do it after
         the validation gate passes (which is when this list runs).
- [ ] Delete stale root build artifacts: `WORKSPACE` (Bazel era),
      `build.sh`/`test.sh` (docker builds of legacy targets),
      `docker-compose.yml` (pre-Kind dev environment). Verify nothing
      references them (CI, docs) before deleting.

### Historical notes (build & dev environment, retained from earlier drafts)

- `web/Dockerfile` bumped to `node:20` (was `node:14.7.0`, EOL). Build
  verified working.
- Switched to `cargo-binstall` in Dockerfile to avoid `serde` compilation
  errors when installing `cargo-leptos`; fixed `dart-sass` path handling;
  isolated `cargo chef cook` for the `web` crate to keep non-WASM deps out
  of the WASM build graph; `SQLX_OFFLINE=true` via `.sqlx` metadata.
- 2025-12-22: server-fn DB pool fixed via Leptos context injection
  (`leptos_routes_with_context()` + `provide_context()`/`use_context()`).
