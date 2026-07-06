# 15: Production CD (ArgoCD)

**Status:** Pending

**Goal:** Replace manual `kubectl apply -k k8s/prod` with ArgoCD for GitOps
continuous delivery. GitHub Actions handles CI (build + push to GHCR). ArgoCD
handles CD (sync cluster state to Git). Database migrations run as an ArgoCD
PreSync hook so a failed migration halts the sync before any pods are replaced.

**Delegation gaps resolved 2026-07-05** - the four open decisions (config
repo layout, Actions deploy step, exposure, PreSync verification) are now
specified inline below. Cluster-touching steps still need production
credentials and are marked *(human)*; everything else is delegable.

### `brdgme-config` repo layout (decided 2026-07-05)

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

### ArgoCD installation (production cluster) *(human)*

- [ ] `kubectl create ns argocd && kubectl apply -k argocd/` from
      `brdgme-config` (official install manifest via the pinned remote
      base).
- [ ] **Exposure (decided 2026-07-05): port-forward only.** No
      LoadBalancer (a second LB is $12/mo), no public hostname/HTTPRoute
      (a public admin panel to keep patched, for zero benefit to a solo
      operator). Access: `kubectl -n argocd port-forward svc/argocd-server
      8443:443`, UI at `https://localhost:8443`, CLI via
      `argocd login localhost:8443 --grpc-web`. Document in
      `brdgme-config/README.md`.
- [ ] Retrieve the initial admin password
      (`argocd admin initial-password -n argocd`), rotate it
      (`argocd account update-password`), store the new one in the same
      offline store as the cluster credentials, and delete the
      `argocd-initial-admin-secret` Secret.

### ArgoCD Application manifest

- [x] Create `k8s/argocd/brdgme-app.yaml`: an `Application` resource pointing
      to this repo, `k8s/prod` kustomize path, auto-sync enabled, prune
      enabled, self-heal enabled.
- [x] Commit the `Application` manifest to the repo so ArgoCD manages itself
      (app-of-apps pattern is not needed at this scale - a single Application
      is sufficient).
- [ ] Move `brdgme-app.yaml` into `brdgme-config/argocd/` (per the layout
      above), retargeting `repoURL` to `brdgme-config` and `path` to
      `prod/`; delete `k8s/argocd/` from the source repo. If
      `brdgme-config` is private, register it in ArgoCD with a read-only
      deploy key (`argocd repo add` or a `repository` Secret committed as a
      SealedSecret).

### Database migration Sync hook

- [x] Create `k8s/base/migrate/job.yaml`: a `Job` that runs
      `sqlx migrate run` using the `brdgme/migrate` image (dedicated
      Dockerfile target in `rust/Dockerfile`) and the `postgres-config` secret.
      Annotate with:
      - `argocd.argoproj.io/hook: Sync`
      - `argocd.argoproj.io/hook-delete-policy: BeforeHookCreation`
      - `argocd.argoproj.io/sync-wave: "1"` (`Cluster/postgres` carries
        `sync-wave: "-1"` so it's Ready before this hook runs; hook phase
        always beats sync-wave, so PreSync would run before the Cluster
        exists at all)
      - Note: ArgoCD >= v3.1 has a built-in CNPG Cluster health check, so no
        argocd-cm customization is needed for wave progression to wait on it.
- [x] Add `k8s/base/migrate/` to `k8s/base/brdgme/kustomization.yaml`.
- [ ] Verify: a failed migration halts the ArgoCD sync and leaves the running
      pods untouched. **Procedure (specced 2026-07-05), run once during the
      Phase 16 beta period before any real data exists:**
      1. Record the current web pod name/hash (`kubectl get pods -n brdgme`).
      2. In a `brdgme-config` commit, patch the migrate Job's command to
         `["sh", "-c", "exit 1"]` (kustomize `patches` entry on the Job)
         and simultaneously bump any image tag so the sync has app changes
         to (not) apply.
      3. Observe: the Sync-phase Job runs and fails, the Application reports
         `Sync Failed`/`Degraded`, and the web pods from step 1 are
         untouched (same pod names, old image).
      4. Revert the commit; confirm the sync completes and pods roll.
      This proves the hook mechanics (halt + no partial rollout). A real
      `sqlx migrate run` failure exits non-zero and takes the identical
      path.

### Secrets management: sealed-secrets (added 2026-07-03)

GitOps makes the config repo the source of truth, but the app secrets
(`postgres-config`, `postgres-user`, `bot-config`, `email-config`,
`internal-api-key`, `grafana-cloud` - Phase 18) currently exist only as
manually created cluster Secrets - previously unaddressed. Decision: bitnami-labs/sealed-secrets.
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

- [ ] Create the `brdgme-config` repository per the layout section above
      *(human: create the GitHub repo itself; an agent can author its
      contents)*. Steps: `gh repo create <owner>/brdgme-config --private
      --clone`, then hand to an agent to populate per the layout section.
      (remote-base `prod/kustomization.yaml` pinned to the current source
      ref, image tags matching what CI last built). This becomes the single
      source of truth for what is running in production.
- [ ] Update the ArgoCD `Application` to point to `brdgme-config` instead of
      this repo (covered by the "Move brdgme-app.yaml" task above).
- [ ] **GitHub Actions deploy job (specced 2026-07-05):** append a `deploy`
      job to `.github/workflows/ci.yml`, `needs:` the image-push job,
      running only on `master` pushes:
      1. `actions/checkout` of `brdgme-config` using a deploy key held in
         the source repo's Actions secret `CONFIG_REPO_DEPLOY_KEY`.
      2. In `prod/`: update the remote-base `?ref=` to `${GITHUB_SHA}`
         (`sed -i` on the kustomization - one line, no yq needed), then
         `kustomize edit set image ghcr.io/<owner>/<img>=ghcr.io/<owner>/<img>:${GITHUB_SHA}`
         for each image the workflow built (image tags are the git SHA -
         align the build job's tagging if it differs).
      3. Commit as `deploy: <source repo short-sha>` and push. Add
         `concurrency: { group: deploy, cancel-in-progress: false }` so
         parallel runs cannot race the push.
      ArgoCD auto-sync picks up the commit.
- [ ] Provision the deploy key *(human)*. Steps:
      1. `ssh-keygen -t ed25519 -f deploy-key -N "" -C brdgme-ci-deploy`
      2. `gh repo deploy-key add deploy-key.pub -R <owner>/brdgme-config
         --allow-write --title ci-deploy`
      3. `gh secret set CONFIG_REPO_DEPLOY_KEY -R <owner>/brdgme <
         deploy-key`
      4. `shred -u deploy-key deploy-key.pub`
- [ ] Verify GHCR package visibility is **public** for every image the
      cluster pulls (GHCR packages default to private even when the source
      repo is public). Public packages mean no imagePullSecret anywhere; if
      any package must stay private, a `ghcr.io` pull secret is required in
      prod (decided 2026-07-05: images live on GHCR - tied to the build
      platform, not the deployment platform, and free for public packages).
- [ ] To roll back: revert the relevant commit in `brdgme-config`. ArgoCD
      syncs to the previous tag. No tooling changes required.

### Bootstrap order (one-time, human; record in brdgme-config/README.md)

All commands against the prod cluster (`kubectl config use-context
do-syd1-brdgme` or equivalent), from a `brdgme-config` checkout.

1. Install sealed-secrets: `kubectl apply -k sealed-secrets/`, wait for
   `kubectl -n kube-system rollout status deploy/sealed-secrets-controller`.
2. **Back up the sealing key pair immediately** (losing it means
   re-sealing everything):
   `kubectl -n kube-system get secret -l
   sealedsecrets.bitnami.com/sealed-secrets-key -o yaml >
   sealed-secrets-key-backup.yaml` → store in the same offline store as
   the cluster credentials; do NOT commit it; delete the local copy.
3. Seal each secret (postgres-config, postgres-user, bot-config,
   email-config, internal-api-key, grafana-cloud). Pattern:
   `kubectl create secret generic email-config -n brdgme
   --from-literal=RESEND_API_KEY=... --from-literal=EMAIL_FROM=login@brdg.me
   --dry-run=client -o yaml | kubeseal --format yaml >
   sealed-secrets/secrets/email-config.yaml` - commit the sealed files.
   For secrets that already exist in the cluster (created manually in
   Phase 21/22a), seal from the live values
   (`kubectl get secret <name> -n brdgme -o yaml` to read them), then
   delete the manual Secret AFTER confirming the controller recreated it:
   `kubectl apply -k sealed-secrets/` → check
   `kubectl get secret <name> -n brdgme` shows a fresh creation timestamp.
4. Install ArgoCD: `kubectl create ns argocd && kubectl apply -k argocd/`,
   wait for `kubectl -n argocd rollout status deploy/argocd-server`.
5. Rotate the admin password:
   `kubectl -n argocd port-forward svc/argocd-server 8443:443 &`;
   initial password: `argocd admin initial-password -n argocd`;
   `argocd login localhost:8443 --username admin --grpc-web` (accept the
   self-signed cert); `argocd account update-password`; store the new
   password offline; `kubectl -n argocd delete secret
   argocd-initial-admin-secret`.
6. `kubectl apply -f argocd/brdgme-app.yaml` → watch the first sync in
   the UI (`https://localhost:8443`): PreSync migrate Job runs first,
   then the app resources.
7. Run the PreSync failure verification (procedure above) while the
   database is still disposable (Phase 16 beta).

### Notes

- Migrations are forward-only. A migration that removes a column still read
  by the running version will break live traffic. Use expand/contract: add the
  new column in one deploy, remove the old column in a later deploy after all
  pods are on the new version.
- ArgoCD does not replace Tilt for local dev. Tilt remains the dev environment
  tool; ArgoCD is production-only.

