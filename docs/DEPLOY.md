# Deployment Guide

Production deploys are driven by ArgoCD watching the `brdgme-config` repo.
The `brdgme` repo contains k8s manifests and application code; `brdgme-config`
pins which commit of `brdgme` to render and which image tags to run.

## Prerequisites

- CI green on the target commit (the `e2e` job is allowed to fail)
- `build-rust` and/or `build-go-games` jobs completed (they push images to
  `ghcr.io/brdgme/brdgme/<name>:sha-<short-sha>`)

## Deploy steps

All steps happen in the `brdgme-config` repo (`prod/kustomization.yaml`).

### 1. Bump the manifest ref

The `resources:` entry pins which commit of `brdgme` ArgoCD renders k8s
manifests from. This must match the commit whose images you are deploying:

```yaml
resources:
- https://github.com/brdgme/brdgme//k8s/prod?ref=<full-40-char-sha>
```

Get the full SHA:
```bash
git -C ../brdgme rev-parse HEAD
```

### 2. Bump image tags

Update `newTag:` for every image. Tags use the format `sha-<7-char-sha>`.

**Rust images** (from the `build-rust` CI job): `migrate`, `web`, `bot`,
`operator`, and all game images built by the Rust workspace (e.g.
`acquire-1`, `lost-cities-1`, `lost-cities-2`, `battleship-2`, `tic-tac-toe-2`,
etc.).

**Go images** (from the `build-go-games` CI job): legacy Go game
implementations (e.g. `age-of-war-1`, `battleship-1`, `category-5-1`, etc.).

If only Rust code changed, only the `build-rust` job runs and Go image tags
stay at their previous value (check the prior CI run for the Go tag).

To find which images a CI run built:
```bash
gh run view <run-id> --log --job <job-id> \
  | grep -oP 'ghcr\.io/brdgme/brdgme/[a-z0-9-]+:sha-[a-f0-9]+' | sort -u
```

### 3. Commit and push

```bash
git add prod/kustomization.yaml
git commit -m "Deploy: bump to sha-<short-sha>"
git push
```

ArgoCD auto-syncs (`prune: true`, `selfHeal: true`). Migrations run as a
pre-sync Job (`k8s/base/web/migrate-job.yaml`, sync-wave 1) before the web
Deployment rolls out (sync-wave 2).

### 4. Verify

```bash
argocd app get brdgme
kubectl rollout status deployment/web -n brdgme --timeout=120s
```

## Key rules

- **ref and image tags must always be bumped together.** The ref determines
  which manifests ArgoCD renders; the tags determine which containers run.
  They must point at the same commit.
- **Never edit applied migrations.** See AGENTS.md "Database migrations".
- **Migration 005 rollout window:** old pods may briefly error on dropped
  columns (~30-60s). Accepted; self-heals once rollout completes.

## Rollback

Revert `prod/kustomization.yaml` to the previous ref and tags, commit, push.
ArgoCD prunes and rolls back automatically.
