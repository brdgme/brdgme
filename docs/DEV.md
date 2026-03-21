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

Also deploys legacy stack. Services are accessible via domain-based routing
through Kourier on port 8080 (all `*.lvh.me` subdomains resolve to 127.0.0.1):

- `http://web-legacy.brdgme.lvh.me:8080` - old React frontend
- `http://api.brdgme.lvh.me:8080` - old Rocket API
- `http://websocket.brdgme.lvh.me:8080` - old WebSocket service

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

Migrations live in `rust/web/migrations/`. They are NOT run automatically on
startup. Run them manually via the Tilt UI (`migrate` resource) or:

```bash
cd rust/web && sqlx migrate run
```

The operator does not run migrations.

In production, migrations run as a pre-sync ArgoCD Job (`k8s/base/web/migrate-job.yaml`)
before the new web deployment rolls out.

**Backup** (writes to a file inside the pod, then copies it out to avoid kubectl
streaming issues with large dumps):
```bash
kubectl exec -n brdgme postgres-0 -- pg_dump -U brdgme_user -Fc -f /tmp/backup.dump brdgme
kubectl cp brdgme/postgres-0:/tmp/backup.dump backup.dump
kubectl exec -n brdgme postgres-0 -- rm /tmp/backup.dump
```

Verify the dump is complete:
```bash
pg_restore --list backup.dump
```

**Drop and recreate the database** (useful when restoring a backup):
```bash
kubectl exec -n brdgme postgres-0 -- psql -U brdgme_user -d postgres \
  -c "DROP DATABASE IF EXISTS brdgme WITH (FORCE);" \
  -c "CREATE DATABASE brdgme OWNER brdgme_user;"
```

**Restore from backup** (copy the file into the pod first - piping binary data
through `kubectl exec -i` is unreliable and produces "end of file" errors):
```bash
kubectl cp backup.dump brdgme/postgres-0:/tmp/restore.dump
kubectl exec -n brdgme postgres-0 -- pg_restore --no-owner --no-acl \
  -U brdgme_user -d brdgme /tmp/restore.dump
kubectl exec -n brdgme postgres-0 -- rm /tmp/restore.dump
```

After restore, run migrations before starting the web server:
```bash
cd rust/web && sqlx migrate run
```
Migrations are idempotent and additive - safe to run on top of a restored
production schema.

## Game Types in Dev

Game types are populated by the operator reconciling `GameVersion` CRs. If the
new game page shows no games, check that the `operator` Tilt resource is
healthy and has logged "Upserting game version" for each game.

## Dev vs Prod Configuration

k8s manifests under `k8s/` reflect how the system runs in production. They are
not modified for dev convenience. Dev-specific workarounds (port-forwarding,
local process substitutions) belong in the Tiltfile only.

Knative Services are exposed via Kourier. The Kourier LoadBalancer service is
patched to NodePort 31080, mapped to host port 8080 via `extraPortMappings` in
`k8s/kind-config.yaml`. Knative is configured to use `lvh.me` as its base
domain (`*.lvh.me` resolves to 127.0.0.1 via public DNS), so each service is
reachable at `{service}.{namespace}.lvh.me:8080` without any `/etc/hosts`
changes.

## Tilt Resource Notes

- `crd-ready` gates the operator on CRD establishment - do not remove this dependency
- Tilt scrubbing disabled (`secret_settings(disable_scrub=True)`) so "brdgme" appears in logs
- Game `GameVersion` CRs live alongside each game: `k8s/base/game/{name}/game-version.yaml`
