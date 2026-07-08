# 15: Production CD (ArgoCD) - Design

> Extracted 2026-07-08 from `docs/plan/15-production-cd-argocd.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Live - ArgoCD + sealed-secrets running in prod; first fully-green
sync 2026-07-08 at brdgme@851e23c. Remaining: the GitHub Actions deploy job
(deploys are currently manual one-commit bumps in `brdgme-config`), deleting
the stale `k8s/argocd/` from this repo, rotating the ArgoCD admin password
(the initial-admin secret still exists in the cluster), confirming the
sealing-key offline backup, and the sync-failure drill (by design during the
Phase 16 beta).

**Delegation gaps resolved 2026-07-05** - the four open decisions (config
repo layout, Actions deploy step, exposure, PreSync verification) are now
specified inline in the implementation plan. Cluster-touching steps still need
production credentials and are marked *(human)*; everything else is delegable.

## `brdgme-config` repo layout (decided 2026-07-05)

A new private GitHub repo `brdgme-config`. **Manifests are NOT copied** -
`prod/kustomization.yaml` uses a kustomize **remote base** pinned to a
source-repo commit, and holds only the deploy-time state (ref + image
tags + sealed secrets). This avoids the copy's drift problem: manifest
changes keep living in the source repo; a deploy is a one-commit bump of
`ref` + image tags in `brdgme-config`.

```
brdgme-config/
├── README.md          # bootstrap runbook (below) + rollback one-liner
├── argocd/
│   ├── kustomization.yaml   # remote base: ArgoCD official install
│   │                        #   manifest, pinned to a release tag
│   └── brdgme-app.yaml      # the Application (moved from source-repo
│                            #   k8s/argocd/ - delete it there)
├── sealed-secrets/
│   ├── kustomization.yaml   # remote base: sealed-secrets controller
│   │                        #   manifest, pinned to a release tag
│   └── secrets/             # SealedSecret CRs: postgres-config,
│                            #   postgres-user, bot-config, email-config,
│                            #   internal-api-key, grafana-cloud
└── prod/
    └── kustomization.yaml   # resources:
                             #   - https://github.com/<owner>/brdgme//k8s/prod?ref=<sha>
                             # images: [ghcr.io tag overrides per image]
                             # also references ../sealed-secrets/secrets
```

Scope split: the single ArgoCD `Application` manages `prod/` only.
`argocd/` and `sealed-secrets/` (the controllers themselves) are applied
manually with `kubectl apply -k` from this repo - controller upgrades are
rare, deliberate events for a solo operator; self-managing ArgoCD via
app-of-apps is overkill at this scale (reaffirming the earlier decision).

## Secrets management: sealed-secrets (added 2026-07-03)

GitOps makes the config repo the source of truth, but the app secrets
(`postgres-config`, `postgres-user`, `bot-config`, `email-config`,
`internal-api-key`, `grafana-cloud` - Phase 18) currently exist only as
manually created cluster Secrets - previously unaddressed. Decision: bitnami-labs/sealed-secrets.
Asymmetric encryption; `SealedSecret` CRs are safe to commit; no external
store. (External Secrets Operator rejected - no external secret store
exists to back it; SOPS+age rejected - key distribution and editor
integration overhead for a solo operator.)

## Image update flow (separate config repo)

Image tags are tracked in a dedicated `brdgme-config` repo (separate from this
source repo). GitHub Actions pushes images to GHCR then commits the updated
tags to `brdgme-config`. ArgoCD watches `brdgme-config`, not this repo.

Rationale: committing tags back to the source repo creates CI loop risk and
mixes deployment history with code history. A separate config repo keeps
rollback simple (revert the tag commit, ArgoCD syncs) without any additional
tooling. If a more integrated official mechanism ships with ArgoCD in future it
should be evaluated then. (Evaluated 2026-07-03: ArgoCD Image Updater remains
argoproj-labs, v1.1.x, explicitly not recommended for critical production
workloads, and not merged into core - the custom Actions step stands.)

## Notes

- Migrations are forward-only. A migration that removes a column still read
  by the running version will break live traffic. Use expand/contract: add the
  new column in one deploy, remove the old column in a later deploy after all
  pods are on the new version.
- ArgoCD does not replace Tilt for local dev. Tilt remains the dev environment
  tool; ArgoCD is production-only.
