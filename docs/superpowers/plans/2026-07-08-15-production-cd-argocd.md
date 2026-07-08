# 15: Production CD (ArgoCD) - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/15-production-cd-argocd.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Goal:** Replace manual `kubectl apply -k k8s/prod` with ArgoCD for GitOps
continuous delivery. GitHub Actions handles CI (build + push to GHCR). ArgoCD
handles CD (sync cluster state to Git). Database migrations run as an ArgoCD
PreSync hook so a failed migration halts the sync before any pods are replaced.

**Spec:** `docs/superpowers/specs/2026-07-08-15-production-cd-argocd-design.md`

## ArgoCD installation (production cluster) *(human)*

- [x] `kubectl create ns argocd && kubectl apply -k argocd/` from
      `brdgme-config` (official install manifest via the pinned remote
      base). Done 2026-07-06.
- [x] **Exposure (decided 2026-07-05): port-forward only.** No
      LoadBalancer (a second LB is $12/mo), no public hostname/HTTPRoute
      (a public admin panel to keep patched, for zero benefit to a solo
      operator). Access: `kubectl -n argocd port-forward svc/argocd-server
      8443:443`, UI at `https://localhost:8443`, CLI via
      `argocd login localhost:8443 --grpc-web`. Documented in
      `brdgme-config/README.md`.
- [ ] **Still to do (checked 2026-07-08: `argocd-initial-admin-secret`
      exists in the cluster):** retrieve the initial admin password
      (`argocd admin initial-password -n argocd`), rotate it
      (`argocd account update-password`), store the new one in the same
      offline store as the cluster credentials, and delete the
      `argocd-initial-admin-secret` Secret.

## ArgoCD Application manifest

- [x] Create `k8s/argocd/brdgme-app.yaml`: an `Application` resource pointing
      to this repo, `k8s/prod` kustomize path, auto-sync enabled, prune
      enabled, self-heal enabled.
- [x] Commit the `Application` manifest to the repo so ArgoCD manages itself
      (app-of-apps pattern is not needed at this scale - a single Application
      is sufficient).
- [x] Move `brdgme-app.yaml` into `brdgme-config/argocd/` (per the layout
      in the spec), retargeting `repoURL` to `brdgme-config` and `path` to
      `prod/`. Done 2026-07-06; the private repo is registered in ArgoCD
      via the read-only `argocd-readonly` deploy key.
- [ ] Delete the stale `k8s/argocd/` from the source repo (its
      `brdgme-app.yaml` still points at the source repo and is no longer
      what runs in prod).

## Database migration Sync hook

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

The hook itself is proven in prod: the migrate Job runs during each green
sync (sync-wave ordering fixed in ec06327). The failure path remains to be
drilled:

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

## Secrets management: sealed-secrets

(Decision and rationale in the spec.)

- [x] Install the sealed-secrets controller into the prod cluster via
      kustomize in `brdgme-config`. Done 2026-07-06 (manually applied like
      the other controller units, not ArgoCD-managed).
- [x] Add `kubeseal` to `devenv.nix` - added to `brdgme-config`'s
      `devenv.nix` (where sealing happens), not this repo's.
- [x] Convert each prod secret to a `SealedSecret` committed to
      `brdgme-config`. Done: `postgres-config`, `postgres-user`,
      `bot-config`, `email-config`, `grafana-cloud`, plus
      `barman-cloud-creds` (Phase 19); `internal-api-key` was dropped as
      stale (removed with Phase 13's NATS bot eventing). Verified
      2026-07-08: every app secret in the `brdgme` namespace is
      SealedSecret-owned - no manual Secrets remain.
- [ ] Back up the controller's sealing key pair to the same offline store as
      other cluster credentials - losing it means re-sealing everything.
      *(human - not verifiable from the repo; confirm and tick)*
- [x] Dev unaffected: Tilt continues creating plain Secrets in Kind.

## Image update flow (separate config repo)

(Rationale and repo layout in the spec.)

- [x] Create the `brdgme-config` repository per the layout section in the
      spec. Done 2026-07-06, with two additions to the planned layout:
      `cnpg-operator/` (CNPG + Barman Cloud plugin, Phase 19) and
      `cert-manager/`, both manually-applied controller units alongside
      `argocd/` and `sealed-secrets/`. It is the single source of truth
      for what is running in production.
- [x] Update the ArgoCD `Application` to point to `brdgme-config` instead of
      this repo (covered by the "Move brdgme-app.yaml" task above).
- [ ] **GitHub Actions deploy job (specced 2026-07-05; still pending as of
      2026-07-08 - deploys so far are manual one-commit bumps in
      `brdgme-config`, which proved the flow ArgoCD-side):** append a `deploy`
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
- [x] Provision the deploy key *(human)*. Done 2026-07-06: `ci-deploy`
      read-write deploy key on `brdgme-config` + `CONFIG_REPO_DEPLOY_KEY`
      Actions secret on the source repo.
- [x] Verify GHCR package visibility is **public** for every image the
      cluster pulls (GHCR packages default to private even when the source
      repo is public). Public packages mean no imagePullSecret anywhere; if
      any package must stay private, a `ghcr.io` pull secret is required in
      prod (decided 2026-07-05: images live on GHCR - tied to the build
      platform, not the deployment platform, and free for public packages).
      Verified 2026-07-08: anonymous pull succeeds and the cluster runs
      with no imagePullSecrets.
- [ ] To roll back: revert the relevant commit in `brdgme-config`. ArgoCD
      syncs to the previous tag. No tooling changes required.

## Bootstrap order (one-time, human; record in brdgme-config/README.md)

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
