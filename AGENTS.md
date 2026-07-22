# AGENTS.md

Session-bootstrapping instructions for agents working in this repo. Code-style
and dependency-strategy rules live in `docs/CODING.md`, not here.

## Superpowers

This repo assumes the [superpowers](https://github.com/obra/superpowers)
plugin/skill system is available. If it is not installed, alert the user
before proceeding.

## Read the top-level docs first

At the start of every session, read the following files in full:

- `docs/ARCHITECTURE.md`
- `docs/BACKLOG.md`
- `docs/CODING.md`
- `docs/DEV.md`
- `docs/DEPLOY.md`
- `docs/VISION.md`

Everything else under `docs/` is out of scope for this bootstrap step - read
on demand when the task at hand references it. That includes the
subdirectories (`docs/decisions/`, `docs/porting/`, `docs/authoring/`,
`docs/superpowers/`) and the on-demand references directly under `docs/`,
notably `docs/hydration.md` (Leptos SSR hydration mechanics, hazards, and
troubleshooting - read it before touching Suspense/Transition/resource
structure in `rust/web` or when debugging a hydration panic) and
`docs/email.md` (outbound email rendering mechanics, the Gmail
`font-size:0`/foster-parenting hazard, and a headless-Chromium verification
playbook - read it before touching `rust/web/src/email/` or debugging a
"the email looks wrong" report).

## Reference facts agents get wrong

- GitHub org is `brdgme`, not `beefsack` (that's the developer's personal
  account). Use `ghcr.io/brdgme/brdgme/<image>` and
  `https://github.com/brdgme/brdgme.git`, never `beefsack`.
- Never type or infer a git commit SHA by extending a shortened form seen
  earlier in output (e.g. completing a 7-char prefix into 40 hex chars from
  memory). Fetch the exact value with `git rev-parse` immediately before
  writing it into a file (kustomize refs, pinned images) or a commit, and
  verify the written content afterward.
- If the user's `tilt up` is already running on a non-default port, all
  `tilt` CLI invocations need the matching `--port=<N>` or they fail with
  "No tilt apiserver found". Never block on unbounded/streaming tilt or
  kubectl output (`tilt logs -f`, foreground `port-forward`,
  `kubectl wait` without `--timeout`); poll bounded snapshots instead.

## Database migrations

- Migration files under `rust/web/migrations/` are immutable once applied to
  any environment - never edit them, not even comments. sqlx checksums the
  file contents against what was recorded as applied; any edit (including
  comment-only changes) breaks the checksum and fails the prod migrate Job.
  This happened with migration 005 on 2026-07-11.
- New work goes in a new numbered migration, never a change to an existing
  one.
- Commentary about an already-applied migration belongs in docs or
  AGENTS.md, not the `.sql` file.
- Note on migration 005 (`login_confirmations`): ArgoCD runs the migrate Job
  at sync-wave 1 and the web Deployment at sync-wave 2, but old-image pods
  keep serving traffic until their rollout completes. This leaves a brief
  (~30-60s) window where old pods hard-error `SELECT`ing the
  `login_confirmation` / `login_confirmation_at` columns that 005 drops.
  Accepted: the app self-heals once the rollout finishes, and beta traffic
  is low.

## Resource constraints

- Never start the `tilt` dev environment (or the kind cluster it manages) on
  a machine with less than 32GB RAM - it exhausts host memory. If a task
  seems to need it, stop and ask the user instead.
- Target single packages for all cargo work: `cargo build/check/clippy/test
  -p <crate>` (e.g. `-p web`). Never run workspace-wide builds or test runs;
  they link ~30 binaries and spike RAM and disk.

## Working style

- Never install packages on the host machine - all tooling comes from the
  `devenv`/nix shell. If something's missing, report it rather than
  installing around it.
- Before committing any change that includes Rust code, run
  `scripts/rust-test.sh` and ensure it passes. This script spins up temporary
  Postgres and NATS containers, runs migrations, and executes the full CI
  check suite (fmt, clippy, tests). It takes several minutes but catches
  issues before the slow CI loop. See `docs/DEV.md` for details.
- If you cannot run the full script, at minimum run:
  - `cargo fmt --all -- --check`
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
  Never commit or push with outstanding fmt or clippy errors.
- Running the test suite locally always produces DB test failures - tests
  that need a database fail in a plain local/agent run. This is a known,
  pre-existing condition, not caused by your change; do not chase it or
  report it as a regression. Whether DB tests should be opt-in instead of
  on-by-default is tracked as backlog #40.
- Do not background-poll CI runs waiting for completion - the user watches
  CI himself.
- Do not run commands that would print secret material (sealed-secrets
  sealing keys, decrypted Secrets, tokens) into the session transcript;
  ask the user to run those directly in their own terminal instead.
