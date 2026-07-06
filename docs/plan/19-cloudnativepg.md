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

- [x] Install the CNPG operator via kustomize (ArgoCD-managed once Phase 15
      lands).
- [x] `Cluster` CR: `instances: 1` initially (matches today's posture;
      `instances: 2` + automated failover is a later config change), storage
      on DO block storage.
- [x] Backups: Barman Cloud plugin → DO Spaces bucket (endpoint
      `https://syd1.digitaloceanspaces.com`), daily scheduled base backup +
      continuous WAL archiving (manifests agent-delegable).

**Implementation notes (2026-07-06):** CNPG 1.30 deprecated the in-tree
`.spec.backup.barmanObjectStore` field in favour of the Barman Cloud CNPG-I
plugin (`plugin-barman-cloud`, verified against its v0.13.0 release manifest
and docs at `cloudnative-pg.github.io/plugin-barman-cloud`) - a separate
plugin operator/sidecar with its own CRD, `objectstores.barmancloud.cnpg.io`
(kind `ObjectStore`, group `barmancloud.cnpg.io`), installed into the same
namespace as the CNPG operator (`cnpg-system`) and requiring cert-manager
(already a prod prerequisite since Phase 14). It ships as a plain manifest
(no Helm-only path), so it fits the same kustomize-remote-base pattern as
the CNPG operator itself.

Operator install (CNPG + the Barman Cloud plugin) lives in this repo under
`k8s/cnpg-operator/` (`kustomization.yaml` + `barman-cloud-plugin/`) rather
than in `brdgme-config` per the Phase 15 pattern, since that repo doesn't
exist yet - it's a manually-applied unit (`kubectl apply -k
k8s/cnpg-operator/`), same as `k8s/argocd/brdgme-app.yaml`, not referenced
by `k8s/prod/kustomization.yaml` or managed by the app's ArgoCD
`Application`. Move it into `brdgme-config` alongside `argocd/` and
`sealed-secrets/` once that repo is created.

The prod-only `Cluster` overrides (storage) and the new backup resources
(`ObjectStore`, `ScheduledBackup`) live under `k8s/prod/app/`
(`postgres-patch.yaml`, `postgres-backup.yaml`), wired in via `resources:`/
`patches:` in `k8s/prod/app/kustomization.yaml` - not in
`k8s/base/postgres/`, which stays dev/prod-shared and untouched (2Gi, no
storage class). `k8s/prod/app` is the layer where `namespace: brdgme` and
image tags are already set for the concrete prod resources, so it's the
right layer to patch rather than `k8s/prod/kustomization.yaml`.

Storage: `storageClass: do-block-storage` (DigitalOcean's standard block
storage class on DOKS) at `10Gi`, up from the base's `2Gi`. There's no real
traffic yet pre-cutover (Phase 16 hasn't happened), so 10Gi is a
comfortably-sized guess rather than a measured figure - CNPG supports
online PVC resizing later if needed, so this isn't a one-way door.

The `ObjectStore` (`postgres-backup`) points at the DO Spaces bucket
`brdgme-cnpg-backups` (created via `infra/spaces.tf` in Phase 21) at
`https://syd1.digitaloceanspaces.com`, and references a `barman-cloud-creds`
`Secret` (`ACCESS_KEY_ID`/`ACCESS_SECRET_KEY` keys) that isn't committed
with real values - it becomes a `SealedSecret` once Phase 15/18's
sealed-secrets controller is bootstrapped, same as the other prod secrets.
The `Cluster`'s `.spec.plugins` references this store with
`isWALArchiver: true` for continuous WAL archiving, and a `ScheduledBackup`
(`postgres-daily`, `method: plugin`) runs a daily base backup at 03:00 UTC.
- [ ] Verify a PITR restore into a scratch `Cluster` before relying on the
      backups *(human - runs during the Phase 16 beta window)*. Steps:
      1. Confirm at least one base backup exists:
         `kubectl get backups -n brdgme` shows `completed`, and the Spaces
         bucket contains `base/` + `wals/` objects (DO console or
         `s3cmd ls`).
      2. Write a marker row via psql through the `postgres-rw`
         port-forward (e.g. a throwaway row in a scratch table), note the
         timestamp, wait ~10 minutes for WAL archiving to ship it.
      3. Apply a scratch `Cluster` manifest (e.g. `postgres-restore-test`)
         with `bootstrap.recovery` from the same `barmanObjectStore` as an
         `externalCluster`, `recoveryTarget.targetTime` set to just after
         the marker timestamp. Do NOT point it at the live cluster's
         serverName for writes - recovery is read-from-bucket only.
      4. When the scratch instance is ready, port-forward its `-rw`
         Service and confirm the marker row exists and normal tables look
         sane.
      5. Delete the scratch `Cluster` and its PVC; record the restore
         wall time here.
- [ ] **Test import (rehearsal, during the Phase 16 beta period)** *(human)*:
      run the full procedure below end-to-end against a **scratch database**
      (`CREATE DATABASE brdgme_import_test` in the CNPG instance - not the
      beta database), using a real `pg_dump` of live Linode prod. A plain
      `pg_dump` is read-only and safe against the live server; no freeze
      needed. Success criteria:
      - `pg_dump` and `pg_restore` complete without errors (or every error
        is understood and its fix recorded in the procedure below - e.g.
        extension/locale/ownership quirks).
      - `sqlx migrate run` applies cleanly on top, answering the open
        question in step 4 (record the answer there; add a baseline step
        to the procedure if one turns out to be needed).
      - Step-5 verification passes (row counts vs source, login works
        against the scratch DB, one historical game renders - point a dev
        `DATABASE_URL` at it through the port-forward).
      - Record the measured dump+restore wall time in the Phase 16 runbook
        so the cutover freeze window is announced honestly.
      Drop the scratch database afterwards. Update the procedure below
      with anything learned - at cutover it must be executable verbatim.
- [ ] Data import **(revised 2026-07-05: workstation pg_dump/restore, not
      `bootstrap.initdb.import`)**. The original spec assumed the old
      StatefulSet lived in the same cluster; current prod is on Linode, and
      a live cross-provider connection for CNPG's `externalCluster` import
      is more plumbing than the data warrants. Michael decided:
      dump/restore through the workstation. Also note the beta-period
      interaction (Phase 16): the CNPG cluster is created **before** the
      import with a fresh, isolated database for beta testing; the import
      replaces that beta data at cutover. *(human)* Procedure - run for
      real inside the Phase 16 cutover window (legacy stack already
      stopped - see the Phase 16 runbook), after the test import below has
      passed:
      1. Tunnel to the Linode Postgres (SSH tunnel or `kubectl
         port-forward`, whichever matches the legacy deployment) and
         `pg_dump -Fc -d brdgme -f brdgme-prod.dump`. Keep this dump - it
         is also the rollback snapshot.
      2. `kubectl port-forward svc/postgres-rw 5433:5432 -n brdgme`, then
         recreate the target database inside the CNPG instance (drop the
         beta contents): `psql -c 'DROP DATABASE brdgme' -c 'CREATE
         DATABASE brdgme OWNER brdgme_user'` via the superuser (enable
         `enableSuperuserAccess` temporarily or use the operator's
         `postgres` credentials).
      3. `pg_restore --no-owner --role=brdgme_user -d brdgme
         brdgme-prod.dump` through the same port-forward.
      4. Run the new system's migrations (`sqlx migrate run`) on top and
         confirm they apply cleanly - the legacy dump has no
         `_sqlx_migrations` table, so **the rehearsal must verify** the
         migration set is additive over the legacy schema (or a documented
         baseline step is needed; determine during rehearsal and record
         here).
      5. Verify: row counts for `users`, `games`, `game_players`,
         `game_logs` match the source; app login with a real account;
         one historical game renders.
- [x] Delete the old `k8s/base/postgres/` StatefulSet manifests - done in
      the Dev section above (`k8s/base/postgres/` now holds the CNPG
      `cluster.yaml`; the directory is shared by dev and prod).
- [ ] The prod `postgres-config` secret's `DATABASE_URL` host must be
      `postgres-rw` (CNPG Service naming), same as the dev Tiltfile secret.
      *(human)* Revised 2026-07-06: with the Phase 16 beta plan the app
      runs against CNPG from beta start, so this is NOT a cutover edit -
      get it right when first sealing `postgres-config` during the
      Phase 15 bootstrap (step 3 of the bootstrap runbook), and verify the
      web pod connects during beta. Nothing to do at cutover.

