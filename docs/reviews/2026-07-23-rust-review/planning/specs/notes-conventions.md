# Repo conventions - notes for spec writers

Compiled 2026-07-25 from live repo files. Cite-checked; paths are absolute.

## Layout

- Rust workspace root: `/home/beefsack/Development/brdgme/rust` (own
  `Cargo.toml`, `Cargo.lock`, `deny.toml`, `Dockerfile`).
- Members: `web/` (Axum + Leptos monolith), `bot/` (LLM bot NATS consumer),
  `operator/` (k8s operator), `lib/` (cmd, color, cost, game, game_client,
  markup, rand_bot), `game/` (27 game crates, e.g. `alhambra-1`,
  `tic-tac-toe-2`), `tools/` (fuzz etc.).
- Toolchain: `/home/beefsack/Development/brdgme/rust/rust-toolchain.toml` -
  channel `1.97.0`, components rustfmt + clippy, target
  `wasm32-unknown-unknown`.

## Authoritative instruction files

- `/home/beefsack/Development/brdgme/AGENTS.md` - session bootstrap, cargo
  and resource rules, migration immutability, working style. No CLAUDE.md
  anywhere in the repo.
- `/home/beefsack/Development/brdgme/docs/CODING.md` - code style, error
  handling, Leptos SSR/hydration rules, dependency strategy, testing
  conventions.

## Build / test commands

- Run from `/home/beefsack/Development/brdgme/rust`.
- ALWAYS target single packages: `cargo build/check/clippy/test -p <crate>`
  (AGENTS.md "Resource constraints"). Never workspace-wide builds/tests on
  dev machines - links ~30 binaries, spikes RAM/disk.
- `web` is feature-gated: server code needs `--features ssr`
  (`cargo test -p web --features ssr`,
  `cargo clippy -p web --all-targets --features ssr -- -D warnings`).
- Full pre-commit suite: `/home/beefsack/Development/brdgme/scripts/rust-test.sh`
  - spins up throwaway Postgres 18 (port 15432) and NATS 2.11 (14222), sets
  `DATABASE_URL`, `NATS_URL`, `SQLX_OFFLINE=true`, `RUST_MIN_STACK=8388608`,
  runs migrations then `scripts/rust-ci-commands.sh`. AGENTS.md requires
  this passes before committing any Rust change.
- CI command sequence (`/home/beefsack/Development/brdgme/scripts/rust-ci-commands.sh`):
  1. `sqlx migrate run --source web/migrations`
  2. `cargo fmt --all -- --check`
  3. `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
  4. `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  5. `(cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets)`
  6. `cargo test --workspace --exclude web`
  7. `cargo test -p web --features ssr`
- Known condition: DB-backed tests fail in a plain local/agent run without
  the containers; pre-existing, do not report as regression (AGENTS.md;
  backlog #40).
- Tooling comes from the devenv/nix shell; never install on the host.
  Never start `tilt`/kind on a <32GB machine.

## Test conventions (docs/CODING.md "Testing Conventions" + observed)

- Mandatory tests: changes to `rust/web/src/db.rs`, `rust/web/src/game/mod.rs`,
  `rust/web/src/auth/` must land with tests; reviewers reject otherwise.
- DB tests use `#[sqlx::test]` (per-test isolated migrated database); no
  shared fixtures or ordering dependence.
- Never call the real game service or LLM in tests; mock the game service
  with an in-process Axum server returning canned JSON (pattern in
  `/home/beefsack/Development/brdgme/rust/web/src/game/client.rs`).
- Page coverage: in-process SSR tests (`#[sqlx::test]` +
  `tower::ServiceExt::oneshot` via `web::router::build_router`) in
  `/home/beefsack/Development/brdgme/rust/web/tests/ssr_pages.rs` are the
  default layer; Playwright is a single hydration smoke spec only
  (`rust/web/end2end/tests/page-loads.spec.ts`), kept under ~1 min.
- Turn-order assertions: never `assert_ne!` on player index in cascade-back
  games; assert on emitted log content instead.
- Test placement observed:
  - `rust/lib/game`: inline `#[cfg(test)]` modules only (e.g.
    `src/command/parser/mod.rs`, `src/command/suggest.rs`, `src/game.rs`,
    `src/rng.rs`); no `tests/` dir.
  - `rust/game/alhambra-1`: inline `#[cfg(test)]` in `src/lib.rs` plus an
    integration test `tests/contract.rs` (game crates carry a contract
    test).
  - `rust/web`: integration tests in `tests/` (`ssr_pages.rs`,
    `nats_bot_eventing.rs`, `websocket_hygiene.rs`) plus inline module
    tests; all behind `--features ssr`.
- Tests backdating `updated_at` must `SET LOCAL`-style adjust within the
  `#[sqlx::test]` per-test database (CODING.md).

## Lint / format

- `cargo fmt --all -- --check` (note CODING.md warns it can flip-flop if
  local toolchain drifts; toolchain is pinned via rust-toolchain.toml).
- Clippy at `-D warnings`, split as in CI above (workspace-minus-web, then
  web with ssr).
- `deny.toml` at `/home/beefsack/Development/brdgme/rust/deny.toml`
  (currently warn-level; see review dependencies unit).

## k8s / deploy layout

- k8s manifests live at repo root, NOT under rust/:
  `/home/beefsack/Development/brdgme/k8s/` with `argocd/`, `base/`, `dev/`,
  `dev-without-web/`, `prod/` (kustomize overlays).
- `base/` contains: `bot/`, `brdgme/`, `cert-manager/`, `game/` (per-game
  dirs like `acquire-1/deployment.yaml`), `gateway/`, `ingress/`,
  `ingress-nginx/`, `migrate/`, `nats/`, `operator/`, `postgres/`, `web/`.
- The review's "k8s/base/web/deployment.yaml" is
  `/home/beefsack/Development/brdgme/k8s/base/web/deployment.yaml`
  (alongside `kustomization.yaml`, `service.yaml`).
- Deploys: ArgoCD; migrate Job at sync-wave 1, web Deployment at wave 2
  (AGENTS.md migration notes). Images `ghcr.io/brdgme/brdgme/<image>`; org
  is `brdgme`, never `beefsack`.
- Migrations `rust/web/migrations/` are immutable once applied (sqlx
  checksums); new work = new numbered migration.
- Dev environment: Tilt + kind (`Tiltfile`, `ctlptl.yaml` at repo root);
  see `docs/DEV.md` / `docs/DEPLOY.md`.
