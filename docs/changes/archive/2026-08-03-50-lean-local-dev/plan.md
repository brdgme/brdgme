# Plan: Lean Local Development (backlog #50)

## Status

Reopened as Expanded work. The Orchestrator approved this correction. All
acceptance criteria have accepted reopened-work evidence.

## Acceptance Evidence Map

| AC | Required evidence | Status |
|---|---|---|
| AC-1 | Default Compose discovery and `docker compose config --quiet`; rendered service, port, and Watch inspection; no wrapper or parallel defaults. | Complete - unit-05 parsed root Compose and rendered 31 services, 27 five-item game Watch lists, and unique published ports. |
| AC-2 | Service/registration inventory, migration evidence, and transactional set assertions for 27 games. | Complete - unit-03 validates canonical-manifest identity and transactional registration; unit-05 confirms the rendered 27-game Compose inventory; unit-07 corrected and regression-tested atomic single-game registration. |
| AC-3 | Direct bounded Compose status/one-shot, persistence, Watch, direct-URI, and cache-reuse observations. | Complete - unit-06 built the full graph (861s current-cache baseline), immediately rebuilt it in 3s with 51 cached layers, confirmed a fully cached scoped-game build, ran the graph and one-shots, reached all 27 direct URIs, and preserved both named volumes through `down` without `-v`. |
| AC-4 | Build context/layer inspection, registry/git cache-mount evidence, and direct Buildx target parse. | Complete - unit-04a inspected cache/context behavior; unit-05 passed the direct `register` Buildx target check. |
| AC-5 | Fresh hydrate and cargo-leptos build evidence. | Complete |
| AC-6 | Direct Compose status/one-shot commands, sampled game request, and host web/hydrate observations; targeted registration and hydrate tests. | Complete - unit-06 verified dependency readiness, successful migration/registration, sampled five game contracts, all 27 URI reachability, and a fresh bounded `cargo leptos build`; unit-06a freshly evaluated the corrected host environment, observed NATS connection, listener readiness, and HTTP 200 from bounded host web process; unit-03 and unit-02 provide the targeted registration/hydrate test evidence. |
| AC-7 | Inspection that production behavior and all `k8s/` paths are unchanged; both Kind entry points reject hosts below 32 GiB. | Complete - no `k8s/` path changed; direct mocked low-memory checks reject both entry points before startup. |
| AC-8 | Documentation review, measurements, and targeted check output. | Complete - unit-05 documentation/static checks and unit-06 bounded build/runtime/resource measurements; unit-06a inspected the development-only host configuration and docs, confirmed exact environment semantics in source, and passed `git diff --check`. |

## Work Units

### unit-01: Reopen Artifacts

Status: complete.

Restore the active change directory and backlog entry, remove only the false
archive row, then correct the reopened specification, plan, log, and state.

### unit-02: Hydrate Fix

Depends on: unit-01.

Fix the SSR-only hydrate feature gate and ignored Leptos metadata. Require fresh
hydrate and cargo-leptos regression evidence.

Status: complete.

### unit-03: Set Registration

Depends on: unit-02.

Implement idempotent, set-aware transactional local registration for every
deployable Rust game while preserving production operator semantics.

Status: complete.

### unit-04a: Docker Build Graph, Dev Tool Targets, Cache, And Context

Depends on: unit-03.

Add the smallest development build target needed by Compose migration and bulk
registration. Improve safe Docker-native cache layering and context boundaries
while preserving production-target behavior.

### unit-04b: Default Compose All-Games Graph, Watch, And Parity

Depends on: unit-04a.

Make root `compose.yaml` the default lane for dependencies, migrations,
registration, and 27 public local game services. Use host `cargo leptos watch`,
native Compose Watch, direct URIs, readiness dependencies, and direct native
Compose validation.

### unit-05: Scripts And Documentation

Depends on: unit-04b.

Status: complete.

Remove low-value lifecycle, smoke, parity, isolation, build-target, and Kind
wrapper scripts with their dedicated tests and fixtures. Retain
`register-wait`, delivery-list, Rust CI, and native cluster setup. Replace CI
wrapper calls with direct Compose and Buildx checks, put the >=32 GiB safety
guard directly in `Tiltfile` and `setup-kind-cluster.sh`, reduce repeated
Compose Watch mappings with standard YAML aliases, and document direct Compose
and Kind workflows. Do not alter any `k8s/` path.

### unit-06: Runtime Validation And Measurement

Depends on: unit-05.

Collect direct bounded Compose status/one-shot, persistence, sampled-game,
host web/hydrate, cache, and measurement evidence without starting Tilt or
Kind. Do not run an elaborate all-game/login/SSE smoke workflow.

Status: complete.

### unit-06a: Host Web Environment Correction

Depends on: unit-06.

Set the development-only host environment needed by `cargo leptos watch` and
document its Compose-hostname distinction and shell reload behavior. Collect a
newly evaluated `devenv` environment observation and, where it can run without
disturbing the user's Compose graph, bounded host-web NATS/listening evidence.
If reevaluation is unavailable, record the exact manual command and leave the
runtime evidence pending.

Status: complete.

### unit-07: Independent Review

Depends on: unit-06.

Review actual scoped changes and evidence for registration correctness,
production isolation, cache/watch behavior, acceptance coverage, and omissions.

Status: complete.

## Checkpoints

- The Lead appends verified checkpoints to `log.md` and updates `state.md`.
- No source commits are checkpoints.
- Do not execute Tilt or Kind on this host.

## Residual Risks

- Compose, Docker, or Cargo availability can block runtime evidence and must not
  prompt installation or host reconfiguration.
- Local DB-dependent tests can fail without their database environment; record
  that known limitation distinctly from a change failure.
- Native Compose Watch event delivery was not run because it is an unbounded
  foreground process. Rendered Watch configuration and a fully cached scoped
  manual game rebuild provide bounded evidence instead.
- The 861s full build is a current-cache baseline, not a manufactured cold
  measurement; `COMPOSE_PARALLEL_LIMIT` may be inert under Bake while
  invocation-only `CARGO_BUILD_JOBS=4` was the effective resource bound.
- A pre-existing orphaned host `web` process holds ports 3000 and 9090. It was
  not stopped; unit-06a used port 3002 for its bounded verification process.

## Final Outcome

- Accepted for completion 2026-08-03: AC-1 through AC-8 are complete. Evidence
  covers root Compose configuration and inventory, transactional registration,
  Docker build/cache behavior, hydrate and host-web checks, direct runtime
  observations, retained Kind guards, and focused formatting, clippy, and test
  gates.
- Residual Watch limitation: native Compose Watch event delivery was not run
  because it is an unbounded foreground process. Rendered Watch configuration
  and a fully cached scoped manual game rebuild remain the bounded evidence.
