# brdgme-go Extraction (deprecated Go games repo) - Design

**Status:** Approved 2026-07-10. Execution blocked on the #31 "retire the Go
stack" milestone (all 17 Track B ports deployed, `-1` versions deprecated).

## Problem

When the Go stack is removed from this repo (rust-only repo, #31), the 17
deprecated Go `-1` game services keep running in k8s for an expected 3-6
months while in-flight games drain. Platform API changes can still require
updating them during that window (precedent: the "rules" game API addition
needed Go-side updates). Git-history-only resurrection would mean reviving a
deleted toolchain under pressure; keeping the Go code in-repo but out of CI
contradicts the rust-only goal and rots invisibly.

## Decision

Extract the Go games to a new standalone repository with its own CI, at the
Go-retirement milestone. Deployment ownership stays in this repo.

## New repository: `brdgme/brdgme-go`

- Contents: the `brdgme-go/` tree hoisted to the repo root, `go.mod`/`go.sum`,
  the existing `brdgme-go/Dockerfile`, and a short README stating the repo's
  purpose (deprecated Go games kept buildable during drain; archive after
  shelving) and the fix-deploy procedure.
- History: preserve via `git filter-repo` on the `brdgme-go/` path so
  blame/archaeology survive. Fallback to a plain copy only if filter-repo
  proves more hassle than it is worth.
- Out of scope: the `websocket` Go service. It is already replaced and is
  deleted outright under #31; it does not move.

## CI (GitHub Actions, single workflow)

- On PR/push: `go test ./...` and `go vet ./...`.
- On push to main: build the Dockerfile and push per-game images to GHCR,
  tagged by commit SHA. Confirm exact image naming at implementation time so
  that updating this repo's `k8s/` manifests is a digest/tag bump only.

## Deploy flow for a rare fix

1. Patch and merge in `brdgme/brdgme-go`; CI publishes images.
2. Bump the pinned image ref in this repo's `k8s/base/game/<game>-1`
   manifests. All Deployments, Services, and GameVersion CRs stay here.

## End of life

When WP4 shelving (see 2026-07-08-31-rust-only-repo-design.md) removes the
last `-1` Deployment, archive `brdgme/brdgme-go` (GitHub read-only). This
caps API-drift exposure; the drain window is the only period the repo must
stay updatable.
