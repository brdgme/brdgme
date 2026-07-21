# Development Guide

## Prerequisites

- NixOS with `devenv` - run `devenv shell` to get all tools
- Docker (for Kind)
- One-time cluster setup: `bash scripts/setup-kind-cluster.sh`

The setup script is idempotent - safe to re-run. Kind cluster config changes
(`ctlptl.yaml`) require delete + recreate (`kind delete cluster` then re-run
the script).

## Daily Workflow

```
tilt up
```

Starts in hybrid mode: backing services + game microservices in Kind, web
server running locally via `cargo leptos watch` on port 3000.

```
WEB_IN_CLUSTER=1 tilt up
```

Builds and deploys `brdgme/web` as a Deployment + ClusterIP Service inside
Kind. Slow iteration - use only for cluster integration testing.

In `WEB_IN_CLUSTER=1` mode the web service is accessible via domain-based
routing through a dev-only Gateway API `Gateway`/`HTTPRoute` set (Cilium),
forwarded to host port 8080 (all `*.lvh.me` subdomains resolve to
127.0.0.1): `http://web.brdgme.lvh.me:8080`.

## Hybrid Mode Networking

The local web server cannot resolve `*.brdgme.svc.cluster.local`. mirrord is
installed via devenv and wraps `cargo leptos watch` in the Tiltfile, giving
the local process transparent access to cluster DNS and services.

Target pod: `nats-0` (stable StatefulSet pod, always running in both dev
modes). Postgres is CloudNativePG-managed (see below) - its pods
(`postgres-1`, ...) are not stable across recreations, so mirrord targets
`nats` instead. `DATABASE_URL` is passed explicitly in the Tiltfile serve
commands (`postgres-rw` via the `kubectl port-forward` on localhost:5432)
rather than relying on mirrord's env stealing from the target pod.

If mirrord behaves unexpectedly, check that postgres and nats are healthy
first.

On NixOS `/etc/hosts` is managed and read-only - kubefwd is not a viable
alternative on this system.

## Login Codes in Dev

`RESEND_API_KEY` is not set in dev, so no real email is sent. Login codes
are printed directly to the Tilt `web` resource log output:

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

**If the long-lived local dev database's migration checksum has drifted**
(`sqlx migrate run` fails with "migration N was previously applied but has
been modified"), do not fight it - regenerate `.sqlx` via a disposable scratch
database instead of migrating the real one:

```bash
createdb -h localhost -U brdgme_user brdgme_sqlx_prepare
DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_sqlx_prepare \
  sqlx migrate run --source rust/web/migrations
cd rust/web && DATABASE_URL=postgres://brdgme_user:brdgme_password@localhost:5432/brdgme_sqlx_prepare \
  cargo sqlx prepare -- --tests --features ssr --all-targets
dropdb -h localhost -U brdgme_user brdgme_sqlx_prepare
```

Use `--tests --features ssr --all-targets` - omitting `--tests` misses
integration-test queries and `SQLX_OFFLINE=true cargo check --all-targets`
will fail even though `cargo check --features ssr` passes. If the dev DB
itself is unusable, `cargo sqlx database reset -y --source web/migrations`
recreates it from scratch.

## Build and Test Gotchas

**Plain `cargo check --workspace` and `cargo test -p web` fail by design.**
The `web` crate (Leptos) has no default features, so its default target
excludes the ssr/hydrate feature gates - expect E0433 "cannot find
brdgme_game" and missing ssr-gated deps (sqlx, tokio, etc.). This is not a
build breakage; it just means `web` always needs an explicit `--features`
flag.

The canonical verification commands are the ones CI runs
(`.github/workflows/ci.yml`), all with `SQLX_OFFLINE=true` (sqlx
compile-time query checks work offline; without it, compilation tries to
reach a live DB):

```bash
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test --workspace --exclude web
cargo test -p web --features ssr
```

`cargo test -p web --features ssr` additionally needs a running Postgres at
`DATABASE_URL` to pass at runtime (CI uses a `postgres:18` service
container; locally devenv sets `DATABASE_URL` to the connection string in
the Database section below and expects that DB to exist externally - devenv
defines no postgres service). Without it the crate compiles but ~41 DB tests
fail with connection timeouts.

## Full Local Test Run

`scripts/rust-test.sh` spins up temporary Postgres and NATS containers, sets
environment variables, then delegates to `scripts/rust-ci-commands.sh` - the
same script CI calls in `.github/workflows/ci.yml`. This guarantees the cargo
commands (fmt, clippy, sqlx prepare check, tests) are identical between local
and CI; changes only need to be made in one place.

It takes several minutes (compilation + test time). It's optional but
recommended before pushing, since it catches everything CI would catch without
needing to wait for GitHub Actions.

Usage (from repo root):
```bash
bash scripts/rust-test.sh
```

Containers are cleaned up automatically on exit (success or failure). Uses
non-standard ports (15432, 14222) so it won't conflict with a running tilt dev
environment.

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

Postgres runs as a CloudNativePG `Cluster` CR (`postgres`); the operator
creates and names instance pods itself (`postgres-1`, ...), and the name is
not stable across recreations. Find the current primary pod first:
```bash
kubectl get pods -n brdgme -l cnpg.io/cluster=postgres
```
The commands below use `$POD` for that pod name.

**Backup** (writes to a file inside the pod, then copies it out to avoid kubectl
streaming issues with large dumps):
```bash
POD=$(kubectl get pods -n brdgme -l cnpg.io/cluster=postgres -o jsonpath='{.items[0].metadata.name}')
kubectl exec -n brdgme "$POD" -- pg_dump -U brdgme_user -Fc -f /tmp/backup.dump brdgme
kubectl cp "brdgme/$POD:/tmp/backup.dump" backup.dump
kubectl exec -n brdgme "$POD" -- rm /tmp/backup.dump
```

Verify the dump is complete:
```bash
pg_restore --list backup.dump
```

**Drop and recreate the database** (useful when restoring a backup):
```bash
kubectl exec -n brdgme "$POD" -- psql -U brdgme_user -d postgres \
  -c "DROP DATABASE IF EXISTS brdgme WITH (FORCE);" \
  -c "CREATE DATABASE brdgme OWNER brdgme_user;"
```

**Restore from backup** (copy the file into the pod first - piping binary data
through `kubectl exec -i` is unreliable and produces "end of file" errors):
```bash
kubectl cp backup.dump "brdgme/$POD:/tmp/restore.dump"
kubectl exec -n brdgme "$POD" -- pg_restore --no-owner --no-acl \
  -U brdgme_user -d brdgme /tmp/restore.dump
kubectl exec -n brdgme "$POD" -- rm /tmp/restore.dump
```

After restore, run migrations before starting the web server:
```bash
cd rust/web && sqlx migrate run
```
Migrations are idempotent and additive - safe to run on top of a restored
production schema.

### peak_rating backfill and rating drift check (#29)

`game_type_users.peak_rating` was historically wrong: legacy code never
maintained it (rows sit at the `1200` default), and current code only ever
raises it as new games finish. `rust/web/migrations/011_peak_rating_backfill.sql`
reconstructs true rating history as `1200 + running cumulative sum of
game_players.rating_change` per `(user_id, game_type_id)`, ordered by
`games.finished_at` then `games.id` (matches `rating_series()` in
`rust/web/src/stats/queries.rs`), and raises `peak_rating` to
`GREATEST(1200, max running value)` wherever the stored value is lower. It
never lowers a peak, and the `peak_rating <` guard makes re-running the
migration file a no-op - safe to apply more than once.

Drift-check query to sanity-check the reconstruction against prod before
trusting the backfill (any row returned means the stored `rating` doesn't
match the reconstructed final rating and warrants investigation):

```sql
WITH series AS (
    SELECT
        gp.user_id,
        gv.game_type_id,
        1200 + sum(gp.rating_change) OVER (
            PARTITION BY gp.user_id, gv.game_type_id
            ORDER BY g.finished_at, g.id
        ) AS running_rating,
        row_number() OVER (
            PARTITION BY gp.user_id, gv.game_type_id
            ORDER BY g.finished_at DESC, g.id DESC
        ) AS rn
    FROM game_players gp
    JOIN games g ON g.id = gp.game_id
    JOIN game_versions gv ON gv.id = g.game_version_id
    WHERE gp.user_id IS NOT NULL
      AND gp.rating_change IS NOT NULL
      AND g.finished_at IS NOT NULL
),
finals AS (
    SELECT user_id, game_type_id, running_rating AS reconstructed_rating
    FROM series WHERE rn = 1
)
SELECT u.name, gt.name AS game_type, f.reconstructed_rating, gtu.rating AS stored_rating
FROM finals f
JOIN game_type_users gtu ON gtu.user_id = f.user_id AND gtu.game_type_id = f.game_type_id
JOIN users u ON u.id = f.user_id
JOIN game_types gt ON gt.id = f.game_type_id
WHERE f.reconstructed_rating <> gtu.rating;
```

Fixture-level coverage lives in `rust/web/src/stats/queries.rs`:
`rating_series_reconstruction_matches_game_type_users_rating` covers the
reconstructed-final-equals-`rating` invariant, and
`peak_rating_backfill_corrects_historical_peaks` covers the migration itself
(peak correction, idempotency on re-run, and that an already-correct higher
peak is never lowered).

## Bot / LLM Configuration

The bot reads LLM settings from `.env` in the project root (loaded by the Tilt
`bot` resource via `set -a && . ./.env`). Current dev config:

```
LLM_URL=https://openrouter.ai/api
LLM_API_KEY=<openrouter key>
BOT_MODEL=openai/gpt-5-nano
REASONING_EFFORT=medium
```

`LLM_API_KEY` is not committed. Set it in `.env` locally.

`RUST_LOG` is deliberately not kept in `.env` - set it ad hoc in the shell
when extra detail is needed (e.g. `RUST_LOG=info,bot=trace`). Keeping it in
`.env` adds noise to normal runs.

## Game Types in Dev

Game types are populated by the operator reconciling `GameVersion` CRs. If the
new game page shows no games, check that the `operator` Tilt resource is
healthy and has logged "Upserting game version" for each game.

## Dev vs Prod Configuration

k8s manifests under `k8s/` reflect how the system runs in production. They are
not modified for dev convenience. Dev-specific workarounds (port-forwarding,
local process substitutions) belong in the Tiltfile only.

The dev-only Gateway/HTTPRoute set is created by the Tiltfile under
`WEB_IN_CLUSTER=1` (not committed to `k8s/`, since it uses
plain HTTP and `lvh.me` hostnames that only make sense in dev - see the
comment above the `gateway-nodeport` Tilt resource). Cilium provisions a
per-Gateway LoadBalancer Service with no selector (Cilium programs endpoints
itself, not via backing pods), so `kubectl port-forward` can never work on
it. Instead, the `gateway-nodeport` Tilt resource waits for the Service to
exist and patches it to pin its NodePort to 31080, which lines up with the
`extraPortMappings` entry in `ctlptl.yaml` (hostPort 8080 -> containerPort
31080). `*.lvh.me` resolves to 127.0.0.1 via public DNS, so each routed
service is reachable at `{service}.brdgme.lvh.me:8080` without any
`/etc/hosts` changes.

## Tilt Resource Notes

- `crd-ready` gates the operator on CRD establishment - do not remove this dependency
- Tilt scrubbing disabled (`secret_settings(disable_scrub=True)`) so "brdgme" appears in logs
- Game `GameVersion` CRs live alongside each game: `k8s/base/game/{name}/game-version.yaml`
- The `gameversions.brdgme.com` CRD is installed by `setup-kind-cluster.sh`, not by Tilt.
  Tilt must never own the CRD: it cannot delete it safely while GameVersion CRs have operator
  finalizers and the operator isn't yet running.

## Troubleshooting

**Certificates stuck `False`/pending on cert-manager + Gateway API: check DNS
before assuming a config bug.** cert-manager's HTTP-01 self-check hits
whatever the hostname currently resolves to. If a challenge fails with a
plain wrong-status-code error (e.g. 404) or "no such host", the ClusterIssuer,
Gateway listeners, HTTPRoutes, and solver pods are very likely all correctly
configured already - verify DNS actually points at the new load balancer
before debugging Gateway/cert-manager manifests.

Related: with cert-manager's Gateway API integration
(`cert-manager.io/cluster-issuer` annotation on the `Gateway`), adding an
HTTPS listener with `tls.certificateRefs: [name: X]` is enough - cert-manager
auto-creates the `Certificate`/`Secret` X and solves HTTP-01 via a temporary
`cm-acme-http-solver-*` `HTTPRoute` it manages itself. Each hostname needs its
own HTTP (port 80) listener on the `Gateway` for that solver route to attach
to, alongside the HTTPS (443) listener.

**sealed-secrets: annotating a `SealedSecret`'s metadata does not trigger
reconcile.** The controller only re-reconciles on spec changes; if a Secret
was manually adopted (`sealedsecrets.bitnami.com/managed=true`) but the
`SealedSecret` still reports a stale `Synced=False`, restart the controller
(`kubectl rollout restart deployment sealed-secrets-controller -n
kube-system`) to force a full resync. ArgoCD has a built-in health check for
`bitnami.com/SealedSecret` that reads this `Synced` condition.

**Stale e2e processes on ports 8100/3010 can make `run.sh` report a false
green.** Its readiness polling connects to whatever is already listening on
those ports, including leftover binaries from an earlier interrupted run.
Kill any listeners on 8100/3010 before trusting a run.

## Recovery: CRD stuck in terminating state

If `gameversions.brdgme.com` gets stuck deleting (Tilt reports "timeout waiting for delete"):

```bash
kubectl get gameversions -A -o name | xargs -I{} kubectl patch {} -n brdgme --type=merge -p '{"metadata":{"finalizers":[]}}'
```

This strips the operator finalizers so Kubernetes can complete the deletion. Re-apply the CRD
afterwards with:

```bash
kubectl apply -f k8s/base/operator/crd.yaml
```
