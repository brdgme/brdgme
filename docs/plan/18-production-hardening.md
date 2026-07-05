# 18: Production Hardening

**Status:** Pending

**Goal:** Ensure errors are visible and diagnosable in production, where
optimised WASM strips debug info and panics are otherwise silent.

**Resequenced 2026-07-04: pre-go-live.** With the hard-cutover decision
(Phase 16) this phase moves before cutover so error visibility exists from
day one of prod traffic - the Phase 16 validation gate explicitly checks
VictoriaLogs for unexplained 5xx/panics.

**Delegation gap:** every open item here needs a decision or spec first:
- **Log aggregation:** decided 2026-07-03 - VictoriaLogs (see the task
  below). Remaining spec work: exact manifests, retention figure, and which
  structured fields become stream fields.
- **Alerting:** thresholds, alert destinations (email? something else?), and
  what tool evaluates the rules.

**Note (2026-07-05):** client-side error *telemetry* - getting production
users' WASM panics to reach the operator at all - remains a known gap. It is
intentionally deferred as a separate future decision (Sentry, a
panic-reporting endpoint, etc. are all out of scope here). The WASM source
maps item below is a local debugging aid only: it makes a panic already
visible in someone's browser console (or reported by a user) resolve to a
real Rust file:line, not a mechanism for getting that panic to the operator.

### WASM client

- [x] **`console_error_panic_hook`**: already installed in `lib.rs::hydrate()`.
      Panics write the message and location to the browser console before
      aborting, even in release builds.

- [x] **ErrorBoundary**: investigated 2026-07-05 — Leptos's `<ErrorBoundary>`
      only catches `Result::Err` rendered in the view, not Rust panics.
      Audited every resource fetch in `rust/web` (GamePage, GameLogs,
      SidebarMenu, DashboardPage) — each already hand-rolls an
      `Err(...) => <fallback>` match arm. `RecentGameLogs` deliberately
      swallows errors since `GameLogs`'s sidebar shows the authoritative
      failure message for the same data. No gap to fill; nothing added.

- [ ] **WASM source maps**: investigated and prototyped end-to-end
      2026-07-05 - **blocked on a toolchain crash, not implemented.**
      Local debugging aid only (see note above), not telemetry.

      Recipe researched and confirmed correct in principle: (1) set
      `debug = true` on the `wasm-release` profile so `rustc` emits DWARF
      into the `wasm32-unknown-unknown` output; (2) `cargo-leptos` 0.3.6
      already has a native `--wasm-debug` flag ("Include debug information
      in Wasm output. Includes source maps and DWARF debug info") that
      passes `--keep-debug`/`--debug` to `wasm-bindgen` so the DWARF
      survives bindgen's walrus-based rewrite (the earlier premise that
      cargo-leptos has no native option was wrong - it does, just not
      documented in the book); (3) a `.wasm.map` would then be produced
      from the DWARF via `llvm-dwarfdump` + a vendored copy of
      Emscripten's `tools/wasm-sourcemap.py` (binaryen does not ship this
      script despite being the more commonly cited source - confirmed by
      inspecting the nixpkgs `binaryen` derivation, which only installs
      compiled tools, no Python scripts; `llvm-dwarfdump` isn't in
      `devenv.nix` yet either, but nixpkgs `llvm` provides it).

      **What actually blocks it:** `cargo-leptos` always runs `wasm-opt`
      on release builds (no supported way to skip it - only
      `wasm-opt-features` to change its flags, not disable it), and the
      pinned toolchain's `wasm-opt` (binaryen 129 via `devenv.nix`, the
      current nixpkgs version) **segfaults/aborts unconditionally on any
      `wasm-bindgen --keep-debug` output that contains a DWARF
      `.debug_info` section** - reproduced twice: once in an isolated
      throwaway crate, once against the real `rust/web` build via
      `cargo leptos build --release --wasm-debug --frontend-only`. The
      crash happens even with zero optimization passes (a plain
      parse-and-rewrite), so no flag combination avoids it - this is a
      hard incompatibility between this rustc's DWARF output and this
      binaryen version, not a tuning problem. Separately, even if the
      crash weren't there, skipping `wasm-opt` entirely to keep DWARF
      inflates the shipped `web.wasm` from ~1.2MB to ~92MB (measured) -
      unacceptable for a render-blocking asset, and cargo-leptos has no
      option to run `wasm-opt` post-strip while keeping DWARF around only
      long enough to snapshot a source map first.

      **What would unblock it:** a `binaryen`/`wasm-opt` release that
      doesn't crash on this DWARF shape (untested - nixpkgs' 129 is
      already close to current), or a `cargo-leptos` release that adopts
      the emscripten pattern of skipping `wasm-opt` specifically for
      debug builds and running it separately on a DWARF-stripped copy
      before the sourcemap snapshot is taken. Revisit if either upstream
      changes; no repo changes were made (Cargo.toml profile edits made
      during the prototype were reverted).

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

### Production readiness: probes & observability (added 2026-07-05)

- [ ] **k8s probe audit for all workloads**: the bot Deployment lost its
      HTTP port in Phase 13 (no more `/trigger` endpoint), so it now has no
      liveness/readiness probe - decide whether to add an exec probe (e.g.
      checking the process is alive / NATS connection is healthy) or another
      approach. Also review probe coverage for web, the game services, NATS,
      Redis, and Postgres to confirm each has an appropriate probe given the
      hard-cutover, no-extended-validation-period posture.

- [ ] **Basic metrics collection**: single-node VictoriaMetrics plus scrape
      configs for the monolith, game services, and bot, pairing with the
      existing vmalert/VictoriaLogs decisions above. Include a minimal
      dashboard/troubleshooting story sufficient for go-live, given there is
      no extended validation period to catch gaps after the fact.

### Not planned

- Client-side error reporting services (Sentry etc.) have immature WASM
  support and add meaningful bundle size. Not worth the trade-off given that
  the SSR layer already captures the important server-side errors and
  `console_error_panic_hook` handles client panics.

