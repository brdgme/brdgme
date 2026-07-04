# Phase 18: Production Hardening

**Status:** Pending

**Goal:** Ensure errors are visible and diagnosable in production, where
optimised WASM strips debug info and panics are otherwise silent.

**Delegation gap:** every open item here needs a decision or spec first:
- **`ErrorBoundary`:** which components get boundaries (list them), what each
  fallback renders, and whether recovery (retry/reload) is offered.
- **WASM source maps:** currently an investigation ("check the option") - do
  the research, then write the resulting config task.
- **Log aggregation:** decided 2026-07-03 - VictoriaLogs (see the task
  below). Remaining spec work: exact manifests, retention figure, and which
  structured fields become stream fields.
- **Alerting:** thresholds, alert destinations (email? something else?), and
  what tool evaluates the rules.

### WASM client

- [x] **`console_error_panic_hook`**: already installed in `lib.rs::hydrate()`.
      Panics write the message and location to the browser console before
      aborting, even in release builds.

- [ ] **`ErrorBoundary`**: wrap key page sections (`GamePage`, `DashboardPage`)
      in Leptos `<ErrorBoundary>` components so a component error renders a
      fallback instead of silently breaking the UI. Without this, a panic or
      unhandled error in the game view leaves the user with a blank or frozen
      component and no indication of what happened.

- [ ] **WASM source maps**: configure `cargo-leptos` to emit source maps in
      release builds. This makes browser console stack traces show Rust source
      locations rather than raw WASM offsets. Check `Cargo.toml`
      `[package.metadata.leptos]` for the `source-map` option when evaluating.

### Server (SSR / Axum)

- [ ] **Structured log aggregation (VictoriaLogs - decided 2026-07-03)**:
      single-node VictoriaLogs Deployment + PVC (~10Gi, ~30d retention) and
      a Vector DaemonSet shipping container stdout with Kubernetes metadata.
      The JSON structured fields already emitted (trace_id, game_id, etc.)
      map directly onto VictoriaLogs fields; query via its built-in web UI
      (Grafana datasource optional later). Chosen over Loki (roughly 5-10x
      heavier at rest in published benchmarks - does not fit 2GB nodes) and
      Datadog (not open source). This is infrastructure config, not code.

- [ ] **Error rate alerting**: alert on elevated `tracing::error!` rate or
      HTTP 5xx rate from the monolith, via vmalert evaluating LogsQL queries
      against VictoriaLogs. Alert destination still undecided.

### Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.

