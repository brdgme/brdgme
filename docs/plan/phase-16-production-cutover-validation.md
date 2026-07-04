# Phase 16: Production Cutover & Side-by-Side Validation

**Status:** Pending

**Goal:** Run old and new systems simultaneously against the same database so
they can be compared directly before committing to cutover. Legacy services
(`rust/api`, `web`, `websocket`) are kept alive until `rust/web` is proven in
production.

**Delegation note:** this phase is operator-driven by nature (production
deploys, DNS, live verification) - not agent-delegable. The two
agent-delegable subtasks are the `http.ts` apex-domain verification (in the
rollback section below, ready now) and the final source/manifest deletion in
the decommission list (ready once the validation gate passes).

Both systems share PostgreSQL, Redis, and the game microservices. Auth
mechanisms are different (Bearer token vs session cookie) so each requires a
separate login - this is acceptable for testing. Both systems publish to Redis
`game.{id}` and `user.{token_id}` channels, so a move in either UI triggers
correct real-time WebSocket updates for clients on the other system.

**Note:** If a move is made via the legacy `rust/api`, the rust/web Leptos
frontend will not receive a `ws.{user_id}` update (the old api does not publish
to that channel). The game page will show stale state until manual refresh.
This is acceptable for the validation period.

### Risks

- `web/Dockerfile` bumped to `node:20` (was `node:14.7.0`, EOL). Build
  verified working.

### Image naming

The old React frontend (`web/Dockerfile`) and the new Leptos SSR app
(`rust/Dockerfile` `web` target) previously shared the image tag `brdgme/web`.
The new Leptos app keeps `brdgme/web`. The old React frontend is renamed to
`brdgme/web-legacy`.

### Infra changes needed

**Superseded note (2026-07-03):** the checked items below referencing Knative
(`DomainMapping`, `config-domain`, Kourier TLS, `net-certmanager`) were built
before the Phase 14 decision to drop Knative. Phase 14 replaces them with
plain Deployments + Gateway API `HTTPRoute`s + cert-manager Gateway
integration. The hostname table and the validation/rollback/decommission
sections below remain correct; only the routing mechanism changed.

- [x] New Leptos app: `rust/Dockerfile` `web` target → `brdgme/web`. k8s
      manifests in `k8s/base/web/` unchanged.
- [x] Add `brdgme/web-legacy` image build to the Tiltfile (from
      `web/Dockerfile`, final stage `web`, tagged `brdgme/web-legacy`).
- [x] Add `brdgme/api` and `brdgme/websocket` image builds to the Tiltfile.
- [x] Create `k8s/base/web-legacy/` manifests (Deployment + Service) using
      `image: brdgme/web-legacy`. Mirror the structure of `k8s/base/web/` but
      with `name: web-legacy`.
- [x] Create `k8s/base/legacy/kustomization.yaml` grouping `web-legacy`, `api`,
      and `websocket` as the legacy stack.
- [x] Restore `api` and `websocket` manifests to an active kustomization overlay
      alongside the legacy frontend. (`k8s/base/brdgme` now includes `../legacy`)
- [x] Configure Knative domain to `brdg.me` (patch `config-domain` in
      `knative-serving`). (`k8s/prod/knative-serving/config-domain.yaml`)
- [x] Create Knative `DomainMapping` resources (one per service) to assign
      custom hostnames. All services are already Knative Services, so Kourier
      routes by hostname automatically:
      - `brdg.me` → `web`
      - `legacy.brdg.me` → `web-legacy`
      - `api.brdg.me` → `api`
      - `ws.brdg.me` → `websocket`
      (`k8s/base/domain-mapping/`, included in `k8s/prod/app/`)
- [x] Remove `k8s/base/ingress/` (nginx Ingress) from `k8s/base/brdgme` -
      Kourier is the sole external entry point via DomainMappings.
- [x] TLS: cert-manager with per-DomainMapping certificates via
      `networking.knative.dev/certificate-class: cert-manager.io` annotation.
      `k8s/base/cert-manager/cluster-issuer.yaml`: Let's Encrypt `ClusterIssuer`
      using HTTP01 solver with `kourier.ingress.networking.knative.dev` ingress
      class. `k8s/prod/knative-serving/`: `config-certmanager.yaml` (issuer ref)
      and `config-network.yaml` (auto-tls: enabled, http-protocol: redirected).
      Prerequisites (one-time, not in kustomize - cluster infrastructure):
        kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.17.2/cert-manager.yaml
        kubectl apply -f https://github.com/knative/net-certmanager/releases/download/knative-v1.21.0/release.yaml
- [x] Verify the old React frontend's API base URL is configured to point to
      the `api` service - confirmed: `http.ts` derives URL by replacing first
      subdomain with `api` (`legacy.brdg.me` → `api.brdg.me`).

### Validation criteria (gate for decommission)

Note this phase is cutover-first: `brdg.me` points at the new system
immediately; the legacy stack on `legacy.brdg.me` is the fallback. "Proven in
production" means all of the following, over a validation window of at least
4 weeks:

- [ ] Every user-facing flow exercised on the new system in production: login
      (email + code), game creation (human opponents and bot slots), command
      submission with autocomplete, undo, concede, restart, mark-read, game
      logs, sidebar active games, live WebSocket updates.
- [ ] At least one game of each deployed game type (Rust and Go) played to
      completion via the new UI.
- [ ] Ratings update correctly on game finish and concede (requires the ELO
      pre-cutover task).
- [ ] Cross-system WS updates verified: a move made in the new UI appears live
      in a legacy React client on the same game, and vice versa.
- [ ] No unexplained monolith 5xx responses or WASM client panics in the
      window (restart 500 bug must be fixed or explained first).
- [ ] Bots complete turns reliably in production (no stuck bot turns needing
      manual bumps).

### Rollback procedure

Both systems share the database, so rollback is routing-only; no data
migration in either direction. Sessions are separate (cookie vs Bearer token)
so users re-login after a swap.

- [ ] Verify before relying on it: the legacy React frontend derives its API
      URL by replacing the first subdomain with `api` (`web/src/.../http.ts`).
      Confirm it produces `api.brdg.me` when served from the apex `brdg.me`,
      not only from `legacy.brdg.me`. If it does not, rollback requires a
      frontend config change - test this while legacy is still deployed.
- To roll back: edit the `brdg.me` `HTTPRoute` (`k8s/base/gateway/`, after
  Phase 14) to point its `backendRef` at the `web-legacy` Service instead of
  `web`, apply, and verify. The TLS certificate is bound to the Gateway
  listener, not the backend, so no re-issue is needed. Keep the
  `legacy.brdg.me` route intact.
- Games created or finished via the new system remain valid for legacy (same
  schema); no cleanup needed.

### Decommission (once validation criteria above are met)

Remove the legacy stack in this order:

- [ ] Remove `api`, `websocket`, and `web-legacy` from the kustomization and
      delete their k8s manifests.
- [ ] Delete `rust/api/`, `web/`, and `websocket/` source directories.
- [ ] Remove legacy image builds from the Tiltfile.
- [ ] Delete stale root build artifacts (added 2026-07-03 final pass):
      `WORKSPACE` (Bazel era), `build.sh`/`test.sh` (docker builds of
      legacy targets), `docker-compose.yml` (pre-Kind dev environment).
      Verify nothing references them (CI, docs) before deleting.

Redis remains after this step - it is still used by `rust/web`. Removal
happens in Phase 17.

**Notes (Build & Dev Environment):**
- Switched to `cargo-binstall` in Dockerfile to avoid `serde` compilation
  errors when installing `cargo-leptos`.
- Fixed `dart-sass` path handling in Dockerfile.
- Isolated `cargo chef cook` for the `web` crate to prevent non-WASM
  dependencies (`mio`, `socket2`) from breaking the WASM build graph.
- Implemented `SQLX_OFFLINE=true` support via `.sqlx` metadata. Added
  Skaffold port-forwarding to allow local builds to verify queries against
  the K8s Postgres instance.
- Refactored `skaffold.yaml`: default profile deploys only backing services
  (Postgres, Redis, game services), skipping the slow `web` build. Use
  `skaffold dev -p with-web` for a full cluster test.

**2025-12-22: Fixed database connection pool in server functions**
- Server functions were failing with "Database pool not found" errors.
- Root cause: `leptos_axum::extract()` with `State<AppState>` had no state
  context in the server function scope.
- Fix: switched to Leptos context-based dependency injection.
  - `leptos_routes_with_context()` instead of `leptos_routes()`.
  - `PgPool` and `GameBroadcaster` provided via `provide_context()`.
  - Server functions use `use_context::<PgPool>()` instead of Axum state
    extraction.
  - `use_context()` with error handling instead of `expect_context()`.

