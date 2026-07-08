# Quick Wins - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/quick-wins.md`. This is a catch-all
> list of independent quick-win items, not a single cohesive design - each
> entry below is its own decision/task with its own rationale (and, where
> applicable, its own resolution notes). There is no shared spec because the
> entries are unrelated to one another. This work is complete/closed;
> retained as an execution record.

**Status:** Complete

### Mailpit replaces namshi/smtp [Superseded 2026-07-03 - do not implement]

Superseded the same day by the Phase 22a revision: outbound email sends via
the Resend HTTP API, not SMTP (DigitalOcean blocks outbound SMTP ports
25/465/587 by default; unblocking is a discretionary support request). With
no SMTP in the app at all, dev needs no SMTP catcher: the existing log
fallback prints emails when `RESEND_API_KEY` is unset, and `k8s/base/smtp/`
is deleted in Phase 22a rather than upgraded.

### Update all dependencies to latest (added 2026-07-04, immediate)

Do this before the Renovate rollout below so the first Renovate run isn't a
wall of PRs. Motivated by the 2026-07-04 lint-gate work: CI's stable clippy
(1.96) was newer than local toolchains and dependabot reports 63
vulnerabilities on the default branch.

- [x] **Rust crates**: bump every version requirement in the `rust/`
      workspace `Cargo.toml`s (root + member crates) to the latest published
      release, then `cargo update`. Fix any breaking-change fallout. If any
      `sqlx::query!` strings change, regenerate `rust/web/.sqlx`
      (`cargo sqlx prepare -- --features ssr --all-targets` from `rust/web`
      against a migrated Postgres).
- [x] **Rust toolchain**: pin the workspace toolchain (add
      `rust-toolchain.toml` at `rust/`) to current stable so local, CI, and
      Docker builds agree; update `rust/Dockerfile` base images to match.
- [x] **CI runners/actions** (`.github/workflows/ci.yml`): bump all action
      versions (`actions/checkout`, `dtolnay/rust-toolchain`,
      `Swatinem/rust-cache`, `docker/*`) and service images (`postgres`,
      `redis`) to latest.
- [x] Gate: full `test-rust` job green locally (fmt, both clippy
      invocations, both test invocations) and in CI.
      Resolved 2026-07-04 (commits d1ade77, b0696ff): toolchain pinned to
      1.94.0 via `rust/rust-toolchain.toml`, wasm-bindgen bumped to 0.2.121,
      `bot`/`operator` moved to sqlx 0.9, workspace crates moved to edition
      2024, and CI actions/service images bumped.

Out of scope: legacy `web/` npm tree (deleted at Phase 16), `brdgme-go`
modules, `devenv.lock` (manual `devenv update`).

### Dependency automation + CI hygiene (added 2026-07-03 final pass)

Independent and delegable. Reduces ongoing maintenance cost with off-the-shelf
tooling; no phase dependencies.

- [x] Renovate (Mend GitHub App, free for open source; `renovate.json` with
      `config:recommended`): automated dependency-update PRs across Cargo,
      Go modules, Dockerfiles, GitHub Actions, and kustomize image tags.
      `ignorePaths` the legacy `web/` npm tree (deleted at Phase 16 anyway).
      `devenv.lock` is not supported - `devenv update` stays manual.
      Resolved 2026-07-04: added root `renovate.json` extending
      `config:recommended` with `ignorePaths` covering `web/**` (plus the
      preset's own node_modules/bower_components defaults, since
      `ignorePaths` isn't a mergeable option). Cargo, gomod, Dockerfile,
      github-actions and kustomize managers are all enabled by default, so
      no further config was needed. Validated with
      `npx --package renovate -- renovate-config-validator`. Installing the
      Mend GitHub App on the repo is a separate, human, one-time step.
- [x] cargo-deny in CI (`deny.toml`): RustSec advisories, license
      compliance (aligns with the everything-open-source principle), and
      duplicate-dependency checks.
      Resolved 2026-07-04: added `rust/deny.toml` (advisories, licenses,
      bans, sources) and a `cargo-deny` CI job
      (`taiki-e/install-action` + `cargo deny check`). License allow-list
      covers the permissive licenses actually in the tree (MIT, Apache-2.0,
      BSD-2/3-Clause, ISC, Zlib, BSL-1.0, Unicode-3.0, CC0-1.0, MIT-0,
      CDLA-Permissive-2.0, Unlicense); workspace crates marked
      `publish = false` and excluded via `licenses.private.ignore = true`
      since none of them are published or license-tagged. 7 pre-existing
      RustSec advisories (3 diesel CVEs + `encoding`, only reachable via
      legacy `rust/api`; `paste`/`proc-macro-error2` transitive via leptos;
      `term_size` via `lib/cmd`) are documented `ignore` entries in
      `deny.toml` pending a real dependency bump. `bans.multiple-versions`
      left at `warn` - the tree has ~30 unavoidable duplicate-version
      crates today, no `skip` entries needed.
- [x] kubeconform in CI: validate `kustomize build` output for `k8s/dev`
      and `k8s/prod` so manifest breakage is caught before apply.
      Resolved 2026-07-04: added a `kubeconform` CI job that installs pinned
      `kustomize` v5.8.1 and `kubeconform` v0.8.0 binaries and validates
      both overlays, using the datree CRDs-catalog as a second
      `-schema-location` for the Gateway API and cert-manager CRDs, with
      `-ignore-missing-schemas` covering only the project's own
      `GameVersion` CRD (`brdgme.com/v1`, no published schema anywhere).
