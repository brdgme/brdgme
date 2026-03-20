# Development Guide

## Prerequisites

- NixOS with `devenv` - run `devenv shell` to get all tools
- Docker (for Kind)
- One-time cluster setup: `bash scripts/setup-kind-cluster.sh`

The setup script is idempotent - safe to re-run. Kind cluster config changes
require delete + recreate (`kind delete cluster` then re-run the script).

## Daily Workflow

```
tilt up
```

Starts in hybrid mode: backing services + game microservices in Kind, web
server running locally via `cargo leptos watch` on port 3000.

```
WEB_IN_CLUSTER=1 tilt up
```

Builds and deploys `brdgme/web` as a Knative Service inside Kind. Slow
iteration - use only for cluster integration testing.

```
LEGACY=1 tilt up
```

Also deploys legacy stack (old React frontend at localhost:3001).

## Hybrid Mode Networking

The local web server cannot resolve `*.brdgme.svc.cluster.local`. mirrord is
installed via devenv and wraps `cargo leptos watch` in the Tiltfile, giving
the local process transparent access to cluster DNS and services.

Target pod: `postgres-0` (stable StatefulSet pod, always running).

If mirrord behaves unexpectedly, check that postgres is healthy first.

On NixOS `/etc/hosts` is managed and read-only - kubefwd is not a viable
alternative on this system.

## Login Codes in Dev

SMTP is not configured in dev. Login codes are printed directly to the Tilt
`web` resource log output:

```
==> LOGIN CODE for user@example.com: 123456
```

## SQLx Offline Mode

All SQL queries require cached metadata in `rust/web/.sqlx/`. After adding or
changing any query:

```bash
cd rust/web
sqlx migrate run          # must be applied first
cargo sqlx prepare -- --features ssr
```

Verify with:
```bash
SQLX_OFFLINE=true cargo check --features ssr
```

The `.sqlx/` directory is committed. The `operator` crate uses dynamic queries
(`sqlx::query(...)` not macros) and has no `.sqlx/` metadata requirement.

## Rust Conventions

- Edition: `2024` for all crates
- Error handling: `thiserror` for library/service crates, `Box<dyn std::error::Error>` for binary entry points
- Key dependency versions when adding operator-style crates:
  - `kube = "3"`, `k8s-openapi = { version = "0.27", features = ["latest"] }`
  - `schemars = "1"` (kube 3.x requires schemars 1.x, not 0.8)

## Database

Connection string (also in `devenv.nix` as `DATABASE_URL`):
```
postgres://brdgme_user:brdgme_password@localhost:5432/brdgme
```

Migrations live in `rust/web/migrations/`. SQLx runs them automatically on
web server startup via `create_pool()`. The operator does not run migrations.

## Game Types in Dev

Game types are populated by the operator reconciling `GameVersion` CRs. If the
new game page shows no games, check that the `operator` Tilt resource is
healthy and has logged "Upserting game version" for each game.

## Tilt Resource Notes

- `crd-ready` gates the operator on CRD establishment - do not remove this dependency
- Tilt scrubbing disabled (`secret_settings(disable_scrub=True)`) so "brdgme" appears in logs
- Game `GameVersion` CRs live alongside each game: `k8s/base/game/{name}/game-version.yaml`
