# CNPG operator (cluster-scoped, manually applied)

Same pattern as `k8s/argocd/` (see `docs/plan/15-production-cd-argocd.md`):
cluster-scoped controllers are applied manually with `kubectl apply -k`,
not managed by the app's ArgoCD `Application`, because controller upgrades
are rare, deliberate events for a solo operator.

This would normally live in the separate `brdgme-config` repo alongside
`argocd/` and `sealed-secrets/` (per the Phase 15 layout), but that repo
doesn't exist yet, so it's authored here for now under `k8s/cnpg-operator/`
until Phase 15 creates `brdgme-config` and it can move.

## What's here

- `kustomization.yaml` - the CloudNativePG operator itself, via a kustomize
  remote base pinned to the same `CNPG_VERSION` (1.30.0) dev installs in
  `scripts/setup-kind-cluster.sh`. Installs into `cnpg-system`.
- `barman-cloud-plugin/` - the Barman Cloud CNPG-I plugin (backup/WAL
  archiving to S3-compatible object storage), required by the prod
  `Cluster`'s `.spec.plugins` (see `k8s/prod/app/postgres-patch.yaml`).
  Must be installed into the same namespace as the CNPG operator
  (`cnpg-system`) and requires cert-manager's controller already running in
  the cluster (already a prod prerequisite - see
  `docs/plan/14-drop-knative-gateway-api.md`).

## Apply manually

```sh
kubectl apply -k k8s/cnpg-operator/
kubectl rollout status deployment -n cnpg-system cnpg-controller-manager

kubectl apply -k k8s/cnpg-operator/barman-cloud-plugin/
kubectl rollout status deployment -n cnpg-system barman-cloud
```
