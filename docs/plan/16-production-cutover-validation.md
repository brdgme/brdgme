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
Phase 19 (CNPG), Phase 15 (ArgoCD + sealed-secrets), Phase 20
(external-dns), Phase 18 (VictoriaLogs + alerting).

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
(`legacy.brdg.me`/`api.brdg.me`/`ws.brdg.me` are not created; external-dns
has nothing to manage for them).

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
- [ ] Break-glass overlay: a `k8s/prod-rollback/` kustomization that deploys
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

### Cutover steps

- [ ] Before creating the Gateway: flip `kube-system/cilium-config`'s
      `enable-gateway-api-proxy-protocol` to `"true"` on the prod cluster,
      restart the `cilium` DaemonSet, and confirm DOKS's reconciler doesn't
      revert it. Only then uncomment
      `do-loadbalancer-enable-proxy-protocol: "true"` in
      `k8s/base/gateway/gateway.yaml` - wrong order sends PROXY-protocol
      bytes to an Envoy not yet expecting them and breaks all traffic. See
      docs/plan/14-drop-knative-gateway-api.md prod-prerequisites section.
- [ ] Deploy the full new stack to prod via ArgoCD (Phase 15) onto CNPG
      (Phase 19), NATS (13/17), Gateway API (14), with external-dns (20)
      managing `brdg.me` from the HTTPRoute.
- [ ] Verify TLS issuance (HTTP01 through the Gateway) for `brdg.me`.
- [ ] Point `brdg.me` at the new system (external-dns/HTTPRoute - a git
      operation after Phase 20).
- [ ] Smoke-test immediately (see validation criteria) with VictoriaLogs
      (Phase 18) open.

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
      window (checked via VictoriaLogs; restart 500 bug must be fixed or
      explained first).
- [ ] Bots complete turns reliably in production (no stuck bot turns needing
      manual bumps; JetStream redelivery observed working on at least one
      induced failure or verified via consumer metrics).
- [ ] Backups healthy in production (added 2026-07-05 - closes the
      no-backup gap in current prod): daily CNPG base backups and WAL
      archiving observed landing in the Spaces bucket during the window,
      and the Phase 19 PITR restore into a scratch `Cluster` has been
      verified against production data (not just dev).

### Rollback procedure (break-glass)

Both systems share the database; rollback is redeploy + routing, no data
migration. Sessions differ (cookie vs Bearer token) so users re-login.

- Apply `k8s/prod-rollback/` (legacy trio + Redis + routes), verify the
  legacy stack is serving, then point apex at it (HTTPRoute/external-dns
  git operation). Minutes, not seconds - acceptable for this project.
- Games created or finished via the new system remain valid for legacy
  (same schema); no cleanup needed.
- Note: ELO ratings columns and other new-system-only writes are ignored by
  legacy; safe.

### Decommission (once the validation gate passes)

- [ ] Delete `k8s/prod-rollback/`, `k8s/base/legacy/`, `k8s/base/web-legacy/`,
      and the `api`/`websocket` manifests.
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds and `LEGACY=1` mode from the Tiltfile;
      update DEV.md.
- [ ] Delete `k8s/base/redis/` (kept until now only for break-glass -
      the monolith stopped using Redis in Phase 17).
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
