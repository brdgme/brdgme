# 19: CloudNativePG - Design

> Extracted 2026-07-08 from `docs/plan/19-cloudnativepg.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Dev complete (Kind); prod Cluster + Barman Cloud backups running
under ArgoCD (fully green 2026-07-08). Remaining: PITR verify + import
rehearsal (Phase 16 beta), real import at cutover.

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

## Dev (Kind)

**Decision (2026-07-05):** use `imageName: ghcr.io/cloudnative-pg/postgresql:18`,
not `:17` as originally written - the StatefulSet being replaced already
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

## Prod (DOKS)

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

Operator install (CNPG + the Barman Cloud plugin) lives in `brdgme-config`
under `cnpg-operator/` (`kustomization.yaml` + `barman-cloud-plugin/`),
alongside `argocd/` and `sealed-secrets/` - it's a manually-applied unit
(`kubectl apply -k cnpg-operator/`), not referenced by `prod/kustomization.yaml`
or managed by the app's ArgoCD `Application`.

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
