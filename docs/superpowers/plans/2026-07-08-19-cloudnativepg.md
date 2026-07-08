# 19: CloudNativePG - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/19-cloudnativepg.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.
>
> Note: the remaining unchecked tasks are marked *(human)* - they are
> human-operated per the sequencing decision in the spec.

**Spec:** `docs/superpowers/specs/2026-07-08-19-cloudnativepg-design.md`

## Dev (Kind)

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

## Prod (DOKS)

- [x] Install the CNPG operator via kustomize, applied manually from
      `brdgme-config` - not ArgoCD-managed (controller upgrades are rare,
      deliberate events for a solo operator).
- [x] `Cluster` CR: `instances: 1` initially (matches today's posture;
      `instances: 2` + automated failover is a later config change), storage
      on DO block storage.
- [x] Backups: Barman Cloud plugin → DO Spaces bucket (endpoint
      `https://syd1.digitaloceanspaces.com`), daily scheduled base backup +
      continuous WAL archiving (manifests agent-delegable).
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
- [x] The prod `postgres-config` secret's `DATABASE_URL` host must be
      `postgres-rw` (CNPG Service naming), same as the dev Tiltfile secret.
      Done at Phase 15 sealing time (one re-seal needed - the first attempt
      had an empty password, fixed in brdgme-config d0847d4); the web pod
      connects in prod (app healthy in the 2026-07-08 green sync). Nothing
      to do at cutover.
