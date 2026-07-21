# Managed Postgres (DigitalOcean) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. NOTE: this plan is scheduled LATER than the Sentry and Alloy-removal work, and several steps are human/operator-operated (tofu apply with real credentials, the prod data load, brdgme-config secret changes). Do NOT run mutating `tofu apply` / `kubectl` against prod without operator approval.

**Goal:** Move production Postgres from in-cluster CloudNativePG to a DigitalOcean Managed Database (single basic node, ~$15.15/mo), loading the prod data directly into it, then remove the CNPG resources.

**Architecture:** Provision the managed cluster + database via OpenTofu (`infra/database.tf`) in the existing VPC; repoint `web`/`bot`/`migrate` by replacing the `DATABASE_URL` in the `postgres-config` SealedSecret (brdgme-config); run migrations and load prod data into the managed DB; verify; then delete the CNPG `Cluster`, Barman `ObjectStore`/`ScheduledBackup`, backup CronJob, and the CNPG/barman operators + their secrets. Dev is untouched.

**Tech Stack:** OpenTofu (digitalocean provider), DigitalOcean Managed PostgreSQL, sqlx (TLS), kustomize/ArgoCD, kubectl.

## Global Constraints

- Prod-only. Dev keeps its existing local/Kind Postgres and its own `DATABASE_URL`; no dev change.
- Single basic node `db-s-1vcpu-1gb` (~$15.15/mo), region `syd1` (`var.region`), inside `digitalocean_vpc.brdgme`. No standby now.
- PostgreSQL major version 18 (to match the current CNPG `imageName`), or the latest DO-supported major if 18 is unavailable at apply time.
- The managed DB REQUIRES SSL; the app connects with `?sslmode=require` over the PRIVATE (VPC) URI. sqlx must have a TLS backend enabled.
- Load prod data DIRECTLY into the managed DB - do not load into CNPG and migrate again.
- Teardown of CNPG happens ONLY after the managed DB is verified serving prod traffic (rollback = repoint `DATABASE_URL` back to CNPG until then).
- The separate missing `database-encryption-key` secret (stuck web/bot rollout) is its own issue, not part of this plan.
- Mutating prod steps (tofu apply, data load, secret swap, CNPG deletion) are operator-approved and human-operated.

## File Structure

- `infra/database.tf` - NEW: managed DB cluster + database resource.
- `infra/outputs.tf` (or inline output) - expose the private connection attributes for sealing.
- `rust/web/Cargo.toml` - ensure sqlx TLS feature enabled (verify; modify only if missing).
- `k8s/prod/app/postgres-patch.yaml` - DELETE (CNPG prod overrides).
- `k8s/prod/app/postgres-backup.yaml` - DELETE (Barman ObjectStore + ScheduledBackup).
- `k8s/base/postgres/` - DELETE the prod Cluster base once dev no longer needs it (see Task 8 note).
- `k8s/prod/app/kustomization.yaml` - remove the postgres base resource, the `postgres-patch.yaml` patch entry, and the `postgres-backup.yaml` resource entry.
- brdgme-config (out of repo): replace `postgres-config` `DATABASE_URL`; remove `postgres-user` + `barman-cloud-creds` secrets; uninstall CNPG/barman operators.

---

### Task 1: Add the managed DB to OpenTofu

**Files:**
- Create: `infra/database.tf`

**Interfaces:**
- Produces: `digitalocean_database_cluster.brdgme` + `digitalocean_database_db.brdgme`; private connection attributes for the secret step.

- [ ] **Step 1: Write `infra/database.tf`**

```hcl
# Production Postgres as a DigitalOcean Managed Database (decided 2026-07-21,
# docs/superpowers/specs/2026-07-21-managed-postgres-design.md). Replaces the
# in-cluster CloudNativePG Cluster. Single basic node, private networking in
# the brdgme VPC; SSL is required on all connections.
resource "digitalocean_database_cluster" "brdgme" {
  name                 = "brdgme-db"
  engine               = "pg"
  version              = "18"
  size                 = "db-s-1vcpu-1gb"
  region               = var.region
  node_count           = 1
  private_network_uuid = digitalocean_vpc.brdgme.id

  lifecycle {
    # Mirror the cluster's node_pool posture: scaling/size changes are a
    # manual human decision, not driven through tofu.
    ignore_changes = [node_count, size]
  }
}

resource "digitalocean_database_db" "brdgme" {
  cluster_id = digitalocean_database_cluster.brdgme.id
  name       = "brdgme"
}

output "db_private_host" {
  value     = digitalocean_database_cluster.brdgme.private_host
  sensitive = true
}
output "db_port" {
  value = digitalocean_database_cluster.brdgme.port
}
output "db_user" {
  value     = digitalocean_database_cluster.brdgme.user
  sensitive = true
}
output "db_password" {
  value     = digitalocean_database_cluster.brdgme.password
  sensitive = true
}
```

- [ ] **Step 2: Validate the config without applying**

Run: `tofu -chdir=infra plan` (operator, with DO credentials)
Expected: a plan showing the new `digitalocean_database_cluster` + `digitalocean_database_db` to be created, no changes to the existing cluster/vpc/spaces. Do NOT apply yet.

- [ ] **Step 3: Commit**

```bash
git add infra/database.tf
git commit -m "feat(infra): add DigitalOcean managed Postgres cluster (db-s-1vcpu-1gb)"
```

### Task 2: Provision the managed DB (operator)

**Files:** none (tofu state)

- [ ] **Step 1: Apply**

Run: `tofu -chdir=infra apply` (operator-approved)
Expected: cluster + database created; outputs `db_private_host`/`db_port`/`db_user`/`db_password` populated.

- [ ] **Step 2: Record the private connection details securely**

Construct the app `DATABASE_URL`:
`postgres://<db_user>:<db_password>@<db_private_host>:<db_port>/brdgme?sslmode=require`
(Use the private host, database name `brdgme`, and `sslmode=require`.) Handle these credentials out-of-band - do NOT print them into a transcript or commit them.

### Task 3: Ensure sqlx has a TLS backend

**Files:**
- Modify (only if needed): `rust/web/Cargo.toml`

- [ ] **Step 1: Check the sqlx features**

Run: `grep -n "sqlx" rust/web/Cargo.toml`
Expected: shows the sqlx dependency + features. Determine whether a TLS backend (`rustls`/`tls`) is enabled.

- [ ] **Step 2: Enable TLS if missing**

If no TLS feature is present, add `rustls` (or `tls`) to the sqlx features in `rust/web/Cargo.toml`, e.g.:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "macros"] }
```

(Adjust to the existing feature list; only ADD the TLS feature. Verify the exact feature name against the pinned sqlx version.)

- [ ] **Step 3: Build + clippy**

Run: `cargo build -p web --features ssr` and `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean.

- [ ] **Step 4: Commit (only if a change was made)**

```bash
git add rust/web/Cargo.toml rust/Cargo.lock
git commit -m "feat(web): enable sqlx TLS for SSL-required managed Postgres"
```

### Task 4: Repoint `DATABASE_URL` (brdgme-config, operator)

**Files:** none in this repo (brdgme-config)

- [ ] **Step 1: Seal + apply the new `DATABASE_URL`**

In `brdgme-config`, update the `postgres-config` SealedSecret so `DATABASE_URL` is the managed-DB private URI from Task 2 (drop the now-unused `POSTGRES_*` keys). Seal and apply/sync so the `postgres-config` Secret in the cluster carries the new URL. (Out of this repo; operator action.)

- [ ] **Step 2: Confirm the secret updated**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get secret postgres-config -n brdgme -o jsonpath='{.data.DATABASE_URL}' | base64 -d | sed 's/:[^:@]*@/:****@/'`
Expected: the managed-DB private host with `sslmode=require` (password masked).

### Task 5: Migrate + load prod data (human-operated)

**Files:** none

- [ ] **Step 1: Run migrations against the managed DB**

Trigger the `migrate` Job on the new `DATABASE_URL` (ArgoCD sync-wave hook, or run `sqlx migrate run` pointed at the managed URI from a job pod). Confirm all migrations apply cleanly.

- [ ] **Step 2: Load the production data directly into the managed DB**

Run the established prod-data load (pg_dump/restore or the import process) pointed at the managed URI. This is the single, direct load - no CNPG round-trip. Verify row counts / sanity-check key tables.

### Task 6: Cutover - repoint web/bot and verify

**Files:** none (ArgoCD sync)

- [ ] **Step 1: Sync web/bot onto the new `DATABASE_URL`**

Let ArgoCD roll `web`/`bot` (they read `DATABASE_URL` via envFrom from `postgres-config`). Watch the rollout complete.

- [ ] **Step 2: Verify the app serves against the managed DB**

Smoke-test beta.brdg.me: login, load a game page, create/play a game. Confirm no DB connection errors in `kubectl logs` and that data reads/writes hit the managed DB.

- [ ] **Step 3: Hold before teardown**

Do NOT proceed to Task 7 until the managed DB is confirmed serving prod traffic. Until CNPG is deleted, rollback is repointing `DATABASE_URL` back to `postgres-rw`.

### Task 7: Remove CNPG resources (this repo)

**Files:**
- Delete: `k8s/prod/app/postgres-patch.yaml`, `k8s/prod/app/postgres-backup.yaml`
- Modify: `k8s/prod/app/kustomization.yaml` (remove the postgres base resource, the `postgres-patch.yaml` patch entry, the `postgres-backup.yaml` resource entry)
- Delete (conditionally): `k8s/base/postgres/` (see note)

- [ ] **Step 1: Remove the prod CNPG manifests + references**

Delete `k8s/prod/app/postgres-patch.yaml` and `k8s/prod/app/postgres-backup.yaml`. In `k8s/prod/app/kustomization.yaml`, remove: the `../../base/postgres` resource (if present), the `- path: postgres-patch.yaml` patch entry, and the `- postgres-backup.yaml` resource entry. KEEP `migrate-patch.yaml` (the migrate Job stays) and the `images:` block.

Note on `k8s/base/postgres/`: the base `Cluster` CR is dev/prod-shared. If dev still runs CNPG in Kind, leave the base in place and only remove it from the PROD kustomization. Delete `k8s/base/postgres/` entirely only once dev has also moved off CNPG (a separate later decision).

- [ ] **Step 2: Verify the prod kustomization builds without CNPG**

Run: `kubectl kustomize k8s/prod/app`
Expected: renders with NO CNPG `Cluster`, `ObjectStore`, or `ScheduledBackup`; web/bot/operator/migrate still present.

- [ ] **Step 3: Commit**

```bash
git add -A k8s/prod/app k8s/base/postgres
git commit -m "chore(k8s): remove CloudNativePG cluster + Barman backup resources (prod on managed DB)"
```

### Task 8: Operator teardown - CNPG operators + secrets (brdgme-config)

**Files:** none in this repo (brdgme-config + cluster)

- [ ] **Step 1: Confirm no CNPG workloads remain**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get pods -n brdgme | grep -E "postgres|barman"` and `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get cluster.postgresql.cnpg.io -A`
Expected: no CNPG instance pods, no `Cluster` CRs (after ArgoCD prunes).

- [ ] **Step 2: Uninstall the CNPG + Barman operators and remove their secrets (operator, brdgme-config)**

In `brdgme-config`, delete the `cnpg-operator/` manual-apply kustomization (CNPG operator + Barman Cloud plugin) via `kubectl delete -k cnpg-operator/`, and remove the `postgres-user` (CNPG bootstrap basic-auth) and `barman-cloud-creds` SealedSecrets. Optionally retire the `brdgme-cnpg-backups` DO Spaces bucket (`infra/spaces.tf`) once old backups are confirmed unneeded.

- [ ] **Step 3: Confirm the operators + secrets are gone**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get pods -n cnpg-system`
Expected: no running CNPG/barman pods (namespace may be removed).

### Task 9: Final verification

- [ ] **Step 1: App health on managed DB**

Confirm web/bot/operator are healthy and serving prod traffic against the managed DB; `kubectl top nodes` shows the recovered memory (~240Mi working set + the 512Mi request / 1Gi limit gone).

- [ ] **Step 2: Dev unaffected**

Confirm the dev stack (Tilt/Kind) still boots against its own Postgres - no dev `DATABASE_URL` change.

- [ ] **Step 3: Rust gate (if Task 3 changed code)**

Run: `cargo fmt --all -- --check`, `cargo clippy -p web --all-targets --features ssr -- -D warnings`
Expected: clean.

- [ ] **Step 4: Update the spec Status line**

Mark `docs/superpowers/specs/2026-07-21-managed-postgres-design.md` Status as complete with the date.

```bash
git add docs/superpowers/specs/2026-07-21-managed-postgres-design.md
git commit -m "docs: mark managed Postgres migration complete"
```
