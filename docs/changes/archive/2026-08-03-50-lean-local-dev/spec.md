# Specification: Lean Local Development (backlog #50)

## Status

Reopened as Expanded work. This corrected specification and plan are approved
by the Orchestrator for autonomous execution.

## Intent

Make root Compose the default, resource-lean local lane while retaining the
production deployment model unchanged.

## Scope

- Root `compose.yaml` owns Postgres, JetStream NATS, migrations, idempotent
  registration, and all 27 deployable Rust games. It excludes the 17 legacy Go
  games and undeployed `lords-of-vegas-1`.
- Every delivered game is public locally at a host-reachable direct URI.
  Registration is set-aware and transactional; production operator semantics do
  not change.
- The host runs `cargo leptos watch`. Remove low-value custom lifecycle,
  smoke, parity, isolation, and build-target wrappers and their fixtures.
  Retain only `scripts/register-wait.sh`, the pre-existing delivery-list and
  Rust CI scripts, and existing cluster setup that native configuration cannot
  replace.
- Use native Compose Watch. `COMPOSE_BAKE`, `COMPOSE_PARALLEL_LIMIT`, and
  `CARGO_BUILD_JOBS` are optional environment-only controls, with no repository
  defaults or limits.
- Improve Docker-native caching through context/layers and registry/git cache
  mounts. Use implicit local BuildKit cache only: no host-cache sharing,
  sccache, local exporter, or builder/daemon configuration.
- Fix the SSR-only hydrate feature gate and ignored Leptos metadata, with fresh
  hydrate and cargo-leptos regression evidence.

## Non-Goals

- Changing production manifests, topology, default image behavior, schema,
  authentication, or scale policy.
- Adding legacy Go games or `lords-of-vegas-1` to local deployment.
- Starting Tilt or Kind on this host.

## Constraints

- Local services must preserve persistent data and keep production operator
  behavior isolated.
- Runtime validation uses concise direct Compose status/one-shot commands, a
  sampled game request, and host hydrate/web observations. Targeted
  transactional-registration and hydrate tests remain required.
- Kind parity remains available, but its >=32 GiB safety guard is direct in
  both `Tiltfile` and `scripts/setup-kind-cluster.sh`; no Kind wrapper remains.
- Do not delete or alter any `k8s/` path for this change. Production-adjacent
  ingress and ArgoCD cleanup remain separate backlog decisions.
- No commits or pushes. All changes remain within this repository.
- Do not run `scripts/rust-test.sh` on this host; use targeted checks and record
  the known local DB-test limitation where it applies.

## Acceptance Criteria

| ID | Criterion |
|---|---|
| AC-1 | Default Compose discovery and `docker compose config` succeed without encoded parallel defaults or custom parity wrappers. |
| AC-2 | Compose defines all 27 deployable Rust game services, Postgres, JetStream NATS, migrations, and idempotent set-aware registration; excluded games remain excluded. |
| AC-3 | Migrations, persistence, native Compose Watch, direct public game URIs, and cache reuse have bounded evidence. |
| AC-4 | Docker context/layer and registry/git cache mounts work with implicit local BuildKit cache only. |
| AC-5 | Fresh hydrate and cargo-leptos builds prove the hydrate gate and Leptos metadata regression fixed. |
| AC-6 | Concise direct runtime validation covers Compose status/one-shots, a sampled game request, and host web/hydrate observations; targeted registration and hydrate tests remain covered. |
| AC-7 | Production topology, manifests, image defaults, schema, auth, and scale policy remain unchanged; Kind parity retains direct entry-point memory guards. |
| AC-8 | Documentation, measurements, and targeted formatting, clippy, and test evidence cover the changed work. |

## Residual Risks

- Compose, Cargo, or Docker availability can block runtime evidence; missing
  tooling is not a reason for installation or host reconfiguration.
- No Tilt or Kind execution is permitted on this host.

## Pending Decisions

None.
