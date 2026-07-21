# Managed Postgres (DigitalOcean) - Design

Date: 2026-07-21
Status: Approved (design). To be executed LATER than the Sentry and
Alloy-removal workstreams. Replaces the CloudNativePG setup designed in
`docs/superpowers/specs/2026-07-08-19-cloudnativepg-design.md`.

## Problem

Postgres runs in-cluster under CloudNativePG (CNPG): a `Cluster` CR
(`k8s/base/postgres/cluster.yaml` + `k8s/prod/app/postgres-patch.yaml`),
WAL-archiving + daily base backups to DO Spaces via the Barman Cloud plugin
(`k8s/prod/app/postgres-backup.yaml`), and the CNPG + barman-cloud operators
installed in `cnpg-system`. The owner is not comfortable running stateful
workloads in the cluster when a managed alternative exists. A managed
database buys automated backups + point-in-time recovery, automated upgrades
and failover behaviour, and scaling on demand - and removes a meaningful
amount of in-cluster state, operators, and backup plumbing.

It also frees cluster memory: the `postgres` instance pod runs at ~159Mi
working set but reserves 512Mi request / 1Gi limit (the single largest
overcommit on the nodes), and the `cnpg-controller-manager` (~40Mi) and
`barman-cloud` (~40Mi) pods sit alongside it.

## Decision

Move PRODUCTION Postgres to a DigitalOcean Managed Database, provisioned via
the existing OpenTofu IaC (`infra/`). Single basic node, ~$15.15/month
(1GB RAM / 1 vCPU / 10GB), region `syd1` to match the cluster, private
networking within the existing VPC. Load the imminent production data
DIRECTLY into the managed instance (do not load into CNPG and migrate again).
Then remove the CNPG resources.

Dev is UNAFFECTED: it keeps its existing local/Kind Postgres; the connection
change is prod-only (a sealed secret). This deliberately breaks dev/prod
database-infrastructure parity (which the CNPG design valued) in exchange for
not running stateful infra in prod - an accepted trade-off.

## Provisioning (OpenTofu, `infra/`)

Add `infra/database.tf`:

- `digitalocean_database_cluster.brdgme`:
  - `engine = "pg"`, `version` = PostgreSQL 18 (to match the current CNPG
    `imageName: ghcr.io/cloudnative-pg/postgresql:18`; use the latest
    DO-supported major if 18 is unavailable at apply time).
  - `region = var.region` (syd1), `size = "db-s-1vcpu-1gb"` (the $15.15/mo
    basic single node), `node_count = 1`.
  - `private_network_uuid = digitalocean_vpc.brdgme.id` so the cluster pods
    reach it over the VPC (private URI), not the public internet.
  - `tags` consistent with the rest of `infra/`.
- `digitalocean_database_db.brdgme`: database name `brdgme`.
- The cluster's default user (`doadmin`) and its private `uri` are exposed as
  outputs/attributes; a dedicated app user may be created via
  `digitalocean_database_user` if least-privilege is preferred (decision at
  implementation time; default user is acceptable for beta).
- Add the cluster to the existing `digitalocean_project` (see `project.tf`)
  for consistency.

DO managed databases REQUIRE SSL on connections; the app connects with
`?sslmode=require` over the private URI.

## Connection / secrets

- The app reads `DATABASE_URL` from the `postgres-config` secret (envFrom on
  the `web`, `bot`, and `migrate` workloads). The current value points at the
  CNPG `postgres-rw` service.
- Create/replace the prod `DATABASE_URL` with the managed DB private URI
  (`postgres://<user>:<pass>@<private-host>:25060/brdgme?sslmode=require`) as
  a SealedSecret in `brdgme-config` (operator action, out of this repo -
  recorded as a step). The remaining `POSTGRES_*` keys in `postgres-config`
  become unused and can be dropped.
- **sqlx TLS:** the managed DB requires SSL, so `rust/web`'s sqlx must be
  built with a TLS backend. Verify/enable the sqlx `tls`/`rustls` feature
  (the in-cluster CNPG connection used no TLS). This is the one likely
  application-code touch; confirm at implementation time whether sqlx already
  has a TLS feature enabled.

## Repointing + data load

1. `tofu apply` provisions the managed cluster + database.
2. Seal + apply the new `DATABASE_URL` secret (brdgme-config).
3. Run migrations against the managed DB (the `migrate` Job runs on the new
   `DATABASE_URL`, or run `sqlx migrate run` pointed at it).
4. Load the production data DIRECTLY into the managed DB (the owner's
   imminent prod-data load - `pg_dump`/restore or the established load
   process, pointed at the managed URI). This is human-operated.
5. Sync `web`/`bot` (ArgoCD) onto the new `DATABASE_URL`; verify the app
   serves against the managed DB.

## Removal (after cutover verified)

This repo (ArgoCD-managed):
- `k8s/base/postgres/` - the `Cluster` CR + kustomization (dev/prod-shared
  base). Removing it from prod is via the prod kustomization; the base may be
  deleted entirely once dev no longer uses CNPG (dev decision - see below).
- `k8s/prod/app/postgres-patch.yaml` - prod CNPG overrides.
- `k8s/prod/app/postgres-backup.yaml` - the Barman `ObjectStore` +
  `ScheduledBackup`.
- Their references in `k8s/prod/app/kustomization.yaml`.

brdgme-config / operator actions (out of this repo, recorded as steps):
- Uninstall the CNPG operator + Barman Cloud plugin (`cnpg-operator/`
  manual-apply kustomization).
- Delete the `postgres-user` (CNPG bootstrap basic-auth) and
  `barman-cloud-creds` secrets.
- Optionally retire the `brdgme-cnpg-backups` DO Spaces bucket
  (`infra/spaces.tf`) once old backups are confirmed unneeded.

## Dev environment

Unaffected by the prod cutover. Dev keeps its existing Postgres (CNPG in Kind
or the local Tilt Postgres) and its own `DATABASE_URL`. If/when dev is also
moved off CNPG (e.g. to a plain local Postgres container), the
`k8s/base/postgres/` base can be retired - but that is a separate, later
decision and NOT part of this prod migration.

## Cutover sequencing / risk

- Prod-only; human-operated data load. No data is migrated twice (the prod
  load goes straight to the managed DB).
- Rollback before teardown: until the CNPG `Cluster` is deleted, the old
  in-cluster DB still exists; repointing `DATABASE_URL` back reverts. Teardown
  happens only after the managed DB is verified serving prod traffic.
- The managed DB single node has no standby; brief unavailability during a DO
  maintenance window or node failure is possible. Accepted for beta; a standby
  can be added later (node_count/size change) without a re-architecture.

## Out of scope (rejected / deferred)

- A standby/HA managed node now (cost; beta doesn't need it; add later).
- Migrating dev off CNPG (separate later decision).
- Changing the app's data model or queries (only the connection changes, plus
  the possible sqlx TLS feature).
- Keeping CNPG running alongside the managed DB long-term (removed after
  cutover).
- The separate missing `database-encryption-key` secret (the cause of the
  stuck web/bot rollout observed this session) - tracked on its own, not part
  of this migration.

## Success criteria

1. `tofu plan`/`apply` creates the DO managed Postgres cluster + database in
   the VPC at the $15.15/mo size.
2. `web`/`bot`/`migrate` connect to the managed DB over the private URI with
   SSL; migrations applied; prod data loaded; the app serves prod traffic
   against it.
3. No CNPG `Cluster`, `ObjectStore`, `ScheduledBackup`, or backup CronJob
   remains; the CNPG/barman operators and their secrets are removed; the
   ArgoCD app syncs green.
4. The `postgres`/`cnpg-controller`/`barman-cloud` pods are gone and their
   ~240Mi working set + 512Mi request / 1Gi limit no longer reserve node
   resources.
5. Dev still runs against its own Postgres, unaffected.
