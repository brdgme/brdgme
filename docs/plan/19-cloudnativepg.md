# 19: CloudNativePG

**Status:** Dev complete (Kind); prod pending

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

- [x] Install the CNPG operator in `setup-kind-cluster.sh` (same manifest
      install as prod).
- [x] Replace `k8s/base/postgres/` with a `Cluster` CR: `instances: 1`,
      `imageName: ghcr.io/cloudnative-pg/postgresql:18`, small storage
      request, `bootstrap.initdb` creating the `brdgme` database and owner
      role with the current `postgres-config` credentials (via
      `bootstrap.initdb.secret`) so app config is unchanged.
- [x] Update service references: CNPG exposes `<cluster>-rw`/`<cluster>-ro`
      Services - `DATABASE_URL`/`postgres-config` host changes accordingly.
- [x] Tiltfile: update the Postgres port-forward target and the mirrord
      target (`pod/postgres-0` → the CNPG instance pod, e.g.
      `pod/<cluster>-1`).
- [x] Dev data is disposable: recreate the cluster, run migrations, no
      import needed.

**Decision (2026-07-05):** use `imageName: ghcr.io/cloudnative-pg/postgresql:18`,
not `:17` as originally written above - the StatefulSet being replaced already
ran postgres:18, and switching to CNPG shouldn't also downgrade the major
version.

**Implementation notes:** the CNPG-required bootstrap secret must be
`kubernetes.io/basic-auth` (username/password keys), which the existing
`postgres-config` secret's shape doesn't satisfy - added a second secret
(`postgres-user`) for this rather than reshaping `postgres-config`, since
`postgres-config` (envFrom on the migrate Job, `web`, and `bot` in-cluster
Deployments) still needs its `POSTGRES_*`/`DATABASE_URL` key names. Its
`DATABASE_URL` host changed from `postgres` to `postgres-rw`.
`k8s_kind('Cluster', api_version='postgresql.cnpg.io/v1', pod_readiness='wait')`
is enough for Tilt to discover the operator-created instance pod and attach
the existing `k8s_resource("postgres", port_forwards=["5432:5432"])` -
no fallback `local_resource` port-forward was needed. mirrord in the
Tiltfile (web, bot, operator local_resources) previously targeted
`pod/postgres-0`, a stable StatefulSet pod; CNPG instance pods
(`postgres-1`, ...) are not stable across recreations, so mirrord now
targets `pod/nats-0` instead (still a StatefulSet pod, present in both dev
modes) purely for cluster DNS resolution. Since mirrord's default env
stealing from the target pod no longer carries `DATABASE_URL` (the CNPG
pod has no `postgres-config` envFrom), `DATABASE_URL` is now set explicitly
in each serve_cmd (`postgres://brdgme_user:brdgme_password@localhost:5432/brdgme`,
via the port-forward, `ignore_localhost` in mirrord.json bypasses mirrord
for it) rather than relied on implicitly - mirroring the pattern the
`operator` local_resource already used.

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
- [ ] Delete the old `k8s/base/postgres/` StatefulSet manifests (already
      done - see Dev section above; `k8s/base/postgres/` is shared by dev
      and prod).
- [ ] The prod `postgres-config` secret is managed externally (outside
      Tilt/kustomize) - at cutover, its `DATABASE_URL` host must change from
      `postgres` to `postgres-rw` to match the CNPG Service naming, same as
      the dev Tiltfile secret.

