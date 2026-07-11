# 31: Rust-Only Repository (eliminate Go and TS/JS) - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/31-rust-only-repo.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Goal:** the repository contains only Rust application code. Non-Rust
support files remain by design: docs, k8s YAML, Tiltfile (Starlark), CI
YAML, OpenTofu HCL, Dockerfiles, devenv.nix, shell scripts.

**Spec:** `docs/superpowers/specs/2026-07-08-31-rust-only-repo-design.md`

## WP1: Delete the legacy stack (pre-cutover, unblocked)

Supersedes the matching lines of the #16 decommission list; #16 keeps the
Linode decommission and its human steps. Single commit, tagged beforehand.

- [x] Tag `legacy-final` on master; record the last GHCR sha tags for
      `web-legacy`, `websocket`, `api` in this file.
      Tagged `legacy-final` = f83eced (2026-07-12). Last GHCR tags:
      `web-legacy:sha-ae83392`, `websocket:sha-ae83392`, `api:sha-ae83392`
      (ae83392 was the last commit touching the CI `legacy` path filter).
- [x] Delete source: `web/`, `websocket/`, `rust/api/` (includes
      `rust/api/migrations` - the migrate image uses `rust/web/migrations`,
      self-sufficient since `001_initial_schema.sql`).
- [x] Remove `"api"` from `rust/Cargo.toml` workspace members.
- [x] Delete k8s: `k8s/base/legacy/`, `k8s/base/web-legacy/`,
      `k8s/base/api/`, `k8s/base/websocket/`, `k8s/base/redis/` (kept only
      for break-glass + LEGACY dev; monolith dropped Redis in #17),
      `k8s/dev-legacy/`; remove the `web-legacy`/`websocket`/`api` image
      entries from `k8s/prod/app/kustomization.yaml`.
      (Also removed the `api` entry from `k8s/prod/app/sync-wave-patch.yaml`
      and the three image entries + ref bump in brdgme-config.)
- [x] Tiltfile: remove `LEGACY=1` mode (image builds, k8s_yaml, gateway
      legacy routes/listeners).
- [x] CI: remove the `build-legacy` job from `.github/workflows/ci.yml`.
- [x] Delete stale root artifacts (pulled forward from #16): `WORKSPACE`,
      `build.sh`, `test.sh`, `docker-compose.yml` - verify nothing
      references them first.
- [x] Update DEV.md (LEGACY mode docs) and ARCHITECTURE.md. (#16 already
      updated 2026-07-08: overlay superseded, rollback section replaced by
      the no-rollback decision.)

Payoff: smaller workspace (clippy/deny/test/Renovate stop covering the
Rocket-era api crate), three CI image builds gone, #16 loses its one
unwritten deliverable.

## WP2: Finish Track B ports (ongoing; tracked in #23)

The Go-elimination critical path is the 7 remaining `-2` conversions:
age-of-war, cathedral, love-letter, modern-art, roll-through-the-ages,
splendor, texas-holdem. Library prerequisites first where blocking:

- [ ] cost/permutation module in `rust/lib` (blocks splendor-2)
- [ ] poker hand evaluation (blocks texas-holdem-2)
- [ ] 7 conversions, each per the #23 recipe (crate + 1:1 tests, workspace,
      Dockerfile, CI matrix, Tiltfile, k8s manifests, `-1` marked
      `isDeprecated: true`)

Track A ports are net-new content, not required for Go elimination -
interleave at will.

## WP3: Remove the Go stack (after all 17 `-2` games are deployed)

The `-1` GameVersions keep serving existing games from pinned images; the
repo stops carrying Go. Single commit, tagged beforehand.

- [ ] Tag `go-final`; wait for master CI to push the final
      `sha-<short>` Go game images; record the digests in this file.
- [ ] Pin every `-1` Deployment in `k8s/prod/app/kustomization.yaml` to the
      recorded GHCR digest (immutable `@sha256:` pins, not `latest`).
- [ ] Drop `-1` games from the dev environment (Tiltfile Go builds and dev
      overlay entries) - dev has no legacy games to serve; `-1` k8s base
      manifests stay for prod until WP4 shelves each game.
- [ ] Delete `brdgme-go/` and root `go.mod`.
- [ ] CI: remove `test-go` and `build-go-games` jobs.
- [ ] Update ARCHITECTURE.md/DEV.md; note in #23 that the "retire
      brdgme-go" line is done.

## WP5: Lift `rust/` to the repository root (last)

Only after WP1 and WP3 - fewer moving paths. Mechanical, one commit, `git
mv` to preserve history.

- [ ] Move `rust/*` to root: workspace `Cargo.toml`/`Cargo.lock`,
      `rust-toolchain.toml`, `deny.toml`, `Dockerfile`, crate dirs,
      `.sqlx` if present.
- [ ] Fix paths: `rust/Dockerfile` COPY paths, `docker-bake.hcl`
      (`dockerfile = "rust/Dockerfile"`), Tiltfile `only=["rust/"]`
      filters and dockerfile args, CI `working-directory: rust` +
      `Swatinem/rust-cache` `workspaces: rust`, devenv.nix, `.gitignore`
      (`rust/target` -> `target`).
- [ ] Sweep docs for `rust/` path references (CODING.md, DEV.md,
      ARCHITECTURE.md, GAME_PORTING*.md, plan files).

## Sequencing

- WP1: any time - pre-cutover is *better* (simplifies #16). Does not block
  or depend on cutover.
- WP2: ongoing, post-go-live priority per PLAN.md (#23).
- WP3: gated on WP2 complete (17/17 `-2` deployed).
- WP4: independent feature work, post-go-live; unblocks deleting `-1`
  Deployments that WP3 pinned.
- WP5: gated on WP1 + WP3 (no `web/`, `websocket/`, `brdgme-go/`, `go.mod`
  left at root).

End state: application code is 100% Rust at the repo root; games ship as
versioned editions with a deprecate -> shelve lifecycle; the only
non-Rust files are docs, manifests, CI, infra, and (pending decision 2)
the Playwright e2e suite.
