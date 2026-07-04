# 15: Production CD (ArgoCD)

**Status:** Pending

**Goal:** Replace manual `kubectl apply -k k8s/prod` with ArgoCD for GitOps
continuous delivery. GitHub Actions handles CI (build + push to GHCR). ArgoCD
handles CD (sync cluster state to Git). Database migrations run as an ArgoCD
PreSync hook so a failed migration halts the sync before any pods are replaced.

**Delegation gap:** most of this phase needs production cluster credentials
and judgement calls - treat it as human-operated with agent assistance, not
delegable. Before delegating even the assistable parts, specify:
- **`brdgme-config` repo layout:** exact directory structure, what is copied
  from `k8s/prod`, and how per-service image tags are pinned/edited.
- **GitHub Actions deploy step:** the workflow changes (job YAML), which
  secrets/deploy keys exist and how they are provisioned.
- **ArgoCD exposure:** LoadBalancer vs Ingress vs port-forward-only admin
  access, domain, and TLS.
- **PreSync verification procedure:** concrete steps to prove a failing
  migration halts the sync (e.g. a deliberately broken migration in a
  throwaway branch) - "verify" currently has no procedure.

### ArgoCD installation (production cluster)

- [ ] Install ArgoCD into the production cluster via the official manifest:
      `kubectl apply -n argocd -f
      https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml`
- [ ] Expose the ArgoCD API server (LoadBalancer or Ingress).
- [ ] Store the initial admin password securely and rotate it.

### ArgoCD Application manifest

- [x] Create `k8s/argocd/brdgme-app.yaml`: an `Application` resource pointing
      to this repo, `k8s/prod` kustomize path, auto-sync enabled, prune
      enabled, self-heal enabled.
- [x] Commit the `Application` manifest to the repo so ArgoCD manages itself
      (app-of-apps pattern is not needed at this scale - a single Application
      is sufficient).

### Database migration PreSync hook

- [x] Create `k8s/base/migrate/job.yaml`: a `Job` that runs
      `sqlx migrate run` using the `brdgme/migrate` image (dedicated
      Dockerfile target in `rust/Dockerfile`) and the `postgres-config` secret.
      Annotate with:
      - `argocd.argoproj.io/hook: PreSync`
      - `argocd.argoproj.io/hook-delete-policy: BeforeHookCreation`
- [x] Add `k8s/base/migrate/` to `k8s/base/brdgme/kustomization.yaml`.
- [ ] Verify: a failed migration halts the ArgoCD sync and leaves the running
      pods untouched.

### Secrets management: sealed-secrets (added 2026-07-03)

GitOps makes the config repo the source of truth, but the app secrets
(`postgres-config`, `bot-config`, `INTERNAL_API_KEY`, and later the
external-dns DO token) currently exist only as manually created cluster
Secrets - previously unaddressed. Decision: bitnami-labs/sealed-secrets.
Asymmetric encryption; `SealedSecret` CRs are safe to commit; no external
store. (External Secrets Operator rejected - no external secret store
exists to back it; SOPS+age rejected - key distribution and editor
integration overhead for a solo operator.)

- [ ] Install the sealed-secrets controller into the prod cluster via
      kustomize in `brdgme-config` (ArgoCD-managed like everything else).
- [ ] Add `kubeseal` to `devenv.nix`.
- [ ] Convert each prod secret to a `SealedSecret` committed to
      `brdgme-config`; delete the manually created Secrets once the
      controller has unsealed replacements.
- [ ] Back up the controller's sealing key pair to the same offline store as
      other cluster credentials - losing it means re-sealing everything.
- [ ] Dev unaffected: Tilt continues creating plain Secrets in Kind.

### Image update flow (separate config repo)

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

- [ ] Create a `brdgme-config` repository containing the `k8s/prod` kustomize
      manifests (copy from this repo). This becomes the single source of truth
      for what is running in production.
- [ ] Update the ArgoCD `Application` to point to `brdgme-config` instead of
      this repo.
- [ ] Add a GitHub Actions deploy step: after pushing images to GHCR, clone
      `brdgme-config`, run `kustomize edit set image` for each updated image,
      commit, and push. ArgoCD auto-sync picks up the change.
- [ ] Grant the GitHub Actions bot write access to `brdgme-config` via a
      deploy key or fine-grained PAT scoped to that repo only.
- [ ] To roll back: revert the relevant commit in `brdgme-config`. ArgoCD
      syncs to the previous tag. No tooling changes required.

### Notes

- Migrations are forward-only. A migration that removes a column still read
  by the running version will break live traffic. Use expand/contract: add the
  new column in one deploy, remove the old column in a later deploy after all
  pods are on the new version.
- ArgoCD does not replace Tilt for local dev. Tilt remains the dev environment
  tool; ArgoCD is production-only.

