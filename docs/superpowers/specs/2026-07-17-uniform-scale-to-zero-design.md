# Uniform scale-to-zero for all game versions

Date: 2026-07-17
Status: approved (Michael, 2026-07-17)
Follow-up to: #42 (docs/superpowers/plans/2026-07-16-42-image-scale-to-zero.md)

## Decision

Remove the `scaleToZero` flag and the config-driven routing split introduced in
#42 Phase 3. Every game version routes through the KEDA HTTP interceptor and
scales to zero. There is no per-version CRD knob; the only knob is
`replicas.min` in each version's `HTTPScaledObject` manifest, which stays at 0
for all versions.

## Rationale

- The flag's cost is not the if/else in the operator; it is two routing paths,
  DB `uri` rows that diverge by class, HTTPScaledObjects for only 19 of 41
  versions, and an operator that reverts DB rows when config and manifests
  drift (the class of bug hit during the #42 rollout).
- KEDA request-based scaling means a live match with slow-moving players will
  scale down between moves and pay a cold start (~1.7s pod-wake, ~3s
  click-to-render, measured). Accepted for closed beta; mitigated by raising
  `scaledownPeriod` from 300s to 1800s fleet-wide.
- Escape hatch if a version ever needs to be always-warm: set `replicas.min: 1`
  in its `http-scaled-object.yaml`. No code change.

## Design

### Operator (`rust/operator`)

- Delete `scale_to_zero` from `GameVersionSpec` (`crd.rs`) and from the CRD
  YAML (`k8s/base/operator/`).
- Delete the URI branch in `controller.rs` (currently lines 119-127): the URI
  is always `INTERCEPTOR_URI`, defaulting to
  `http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080`.
- Delete `GAME_SERVICE_URI_TEMPLATE` handling.
- Everything else (Host header, observedGeneration skip, jittered requeue,
  upsert) is unchanged.

### Manifests (`k8s/base/game/`)

- Remove `scaleToZero: true` from the 19 `game-version.yaml` files that have
  it.
- Add `http-scaled-object.yaml` to the 22 game dirs that lack one (min 0,
  max 1, host `{name}.games.internal`, port 80), and register it in each
  `kustomization.yaml`.
- Set `scaledownPeriod: 1800` in all 41 HSOs (19 existing files currently say
  300).

### Database

No manual SQL. Removing `scaleToZero: true` bumps the generation of the 19
CRs that carry it; the operator reconciles those and upserts the interceptor
URI. The other 22 CRs get no spec change, so the observedGeneration skip
would leave their rows on direct URLs; Michael clears their status to force a
reconcile (watch event fires immediately, no SQL needed):

```bash
for v in $(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get gameversions -n brdgme -o name); do
  kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml patch "$v" -n brdgme \
    --subresource=status --type=merge -p '{"status":{"observedGeneration":null}}'
done
```

(Run over all 41 for simplicity; re-reconciling the 19 is a harmless no-op
upsert.)

### Rollout ordering (critical)

The new operator image must be live before the manifest changes apply. The old
binary defaults an absent `scaleToZero` to `false` and would revert the 19
cutover rows to direct service URLs.

1. Land and build the operator change (rust/ change, so CI publishes an
   image).
2. Michael bumps the operator tag in brdgme-config and syncs; new operator
   live.
3. Land the manifest changes (k8s-only commit); Michael bumps the
   brdgme-config ref and syncs; ArgoCD applies CR + HSO changes; the operator
   reconciles the 19 generation-bumped CRs.
4. Michael runs the status-clear loop (Database section) so the remaining 22
   CRs reconcile onto the interceptor URI.

Between steps 2 and 3 the new operator reconciles existing CRs: 19 have
`scaleToZero: true` (now an unknown field, ignored) and 22 have direct URLs
in the DB. The new binary writes the interceptor URI for every version it
reconciles, but only on generation change - rows stay as-is until steps 3-4.
The 22 direct-URL rows keep working throughout (their Services remain).

### Out of scope

- Operator ownership of HTTPScaledObjects/Deployments/Services (option A;
  revisit only if the operator ever grows child-resource management).
- Dev overlay KEDA support: `k8s/base` already carries HSOs and dev has no
  KEDA; unchanged by this work.
- Removing the `game_versions.uri` column (all rows now identical) - possible
  later simplification, not now.

## Verification

- `cargo fmt --check`, `cargo clippy -p operator`, `cargo test -p operator`.
- After rollout: all 41 `game_versions.uri` rows equal the interceptor URL;
  41 HSOs READY; operator logs clean; spot-check cold-start wake on a
  previously non-scale-to-zero version (e.g. acquire-1); confirm scale-down
  after 1800s idle.
