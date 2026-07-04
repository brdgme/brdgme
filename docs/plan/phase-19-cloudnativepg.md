# Phase 19: CloudNativePG

**Status:** Pending

**Decision (2026-07-03 tech review):** replace the hand-rolled Postgres
StatefulSet (`k8s/base/postgres/`) with CloudNativePG (CNCF operator, the de
facto standard for Postgres on Kubernetes - adopted as the recommended
pattern by major clouds). Gains: declarative provisioning, scheduled backups
+ WAL archiving + PITR to DO Spaces (S3-compatible) via the Barman Cloud
plugin, a config-change path to replicas + automated failover, and identical
database infrastructure in dev and prod (same operator, same `Cluster` CR in
Kind and DOKS).

**Sequencing:** after Phase 14 (manifests rewritten once, against final
infrastructure), before Phase 15 (so ArgoCD manages the final shape from day
one) and Phase 16 (cutover happens onto CNPG, not migrated again after).
Dev-side work is delegable; the prod data import is human-operated.

### Dev (Kind)

- [ ] Install the CNPG operator in `setup-kind-cluster.sh` (same manifest
      install as prod).
- [ ] Replace `k8s/base/postgres/` with a `Cluster` CR: `instances: 1`,
      `imageName: ghcr.io/cloudnative-pg/postgresql:17`, small storage
      request, `bootstrap.initdb` creating the `brdgme` database and owner
      role with the current `postgres-config` credentials (via
      `bootstrap.initdb.secret`) so app config is unchanged.
- [ ] Update service references: CNPG exposes `<cluster>-rw`/`<cluster>-ro`
      Services - `DATABASE_URL`/`postgres-config` host changes accordingly.
- [ ] Tiltfile: update the Postgres port-forward target and the mirrord
      target (`pod/postgres-0` → the CNPG instance pod, e.g.
      `pod/<cluster>-1`).
- [ ] Dev data is disposable: recreate the cluster, run migrations, no
      import needed.

### Prod (DOKS)

- [ ] Install the CNPG operator via kustomize (ArgoCD-managed once Phase 15
      lands).
- [ ] `Cluster` CR: `instances: 1` initially (matches today's posture;
      `instances: 2` + automated failover is a later config change), storage
      on DO block storage.
- [ ] Backups: Barman Cloud plugin → DO Spaces bucket (endpoint
      `https://syd1.digitaloceanspaces.com`), daily scheduled base backup +
      continuous WAL archiving. Verify a PITR restore into a scratch
      `Cluster` before relying on it.
- [ ] Data import: `bootstrap.initdb.import` (CNPG logical import) from the
      existing StatefulSet instance during a maintenance window - no shared
      PVC. Verify row counts and app login, then retire the StatefulSet.
- [ ] Delete the old `k8s/base/postgres/` StatefulSet manifests.

