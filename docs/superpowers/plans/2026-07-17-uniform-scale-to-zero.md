# Uniform Scale-to-Zero Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `scaleToZero` flag; every game version routes through the KEDA HTTP interceptor and scales to zero.

**Architecture:** The operator (rust/operator) stops branching on `spec.scaleToZero` and always writes the interceptor URI to `game_versions.uri`. All 39 game dirs under `k8s/base/game/` get an `HTTPScaledObject` (min 0 / max 1, `scaledownPeriod: 1800`). DB cutover happens via operator reconciles, not SQL.

**Tech Stack:** Rust (kube-rs controller, sqlx), Kustomize manifests, KEDA HTTP add-on.

**Spec:** docs/superpowers/specs/2026-07-17-uniform-scale-to-zero-design.md

## Global Constraints

- RAM-starved dev machine: one shell at a time; always `cargo <cmd> -p operator`, never workspace-wide builds. Never start tilt or kind.
- Agents must NOT: commit/push brdgme-config, mutate the cluster, run SQL writes, or poll CI/ArgoCD. Michael does those; give him copy-pasteable commands.
- brdgme repo commits/pushes and read-only kubectl (`--kubeconfig ~/.kube/brdgme-kubeconfig.yaml`) are allowed.
- ROLLOUT ORDERING: Task 2 must not be pushed until Michael confirms the Task 1 operator image is live in prod (old binary would revert the 19 interceptor rows to direct URLs on the generation bump).

---

### Task 1: Operator - remove scaleToZero

**Files:**
- Modify: `rust/operator/src/crd.rs` (remove field)
- Modify: `rust/operator/src/controller.rs:119-127` (URI branch), plus new test module
- Modify: `k8s/base/operator/crd.yaml:30-33` (remove scaleToZero property)

**Interfaces:**
- Produces: `controller::interceptor_uri() -> String` (module-private; used only within controller.rs)
- The reconcile loop behavior consumed by Task 3: on generation change, upserts `game_versions.uri` = interceptor URI for every CR.

- [ ] **Step 1: Write the failing test**

Append to `rust/operator/src/controller.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interceptor_uri_defaults_to_keda_proxy() {
        // INTERCEPTOR_URI is not set in the test environment.
        assert_eq!(
            interceptor_uri(),
            "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p operator`
Expected: FAIL to compile with "cannot find function `interceptor_uri`"

- [ ] **Step 3: Implement**

In `rust/operator/src/controller.rs`, replace the URI branch (lines 119-127):

```rust
    let uri = interceptor_uri();
```

and add above `reconcile` (near `requeue_with_jitter`):

```rust
fn interceptor_uri() -> String {
    std::env::var("INTERCEPTOR_URI").unwrap_or_else(|_| {
        "http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080".to_string()
    })
}
```

In `rust/operator/src/crd.rs`, delete:

```rust
    #[serde(default)]
    pub scale_to_zero: bool,
```

In `k8s/base/operator/crd.yaml`, delete the four `scaleToZero` lines (30-33):

```yaml
              scaleToZero:
                type: boolean
                default: false
                description: "Whether the game version's deployment should scale to zero replicas when idle."
```

- [ ] **Step 4: Verify**

Run: `cargo fmt -p operator -- --check && cargo clippy -p operator -- -D warnings && cargo test -p operator`
Expected: all pass; tests include `interceptor_uri_defaults_to_keda_proxy` and `healthz_returns_ok`.

- [ ] **Step 5: Commit and push**

```bash
git add rust/operator/src/crd.rs rust/operator/src/controller.rs k8s/base/operator/crd.yaml
git commit -m "feat: route all game versions through KEDA interceptor, drop scaleToZero

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
git push
```

This is a rust/ change, so CI builds and publishes `sha-<short>` images.

---

### GATE after Task 1 (Michael, manual)

Report the new commit sha to Michael with these commands; wait for his confirmation that the operator is live before starting Task 2.

```bash
# after CI is green for the Task 1 commit:
cd ~/Development/brdgme-config
# edit prod/kustomization.yaml: operator newTag -> sha-<short-of-task1-commit>
git add prod/kustomization.yaml && git commit -m "deploy: operator uniform interceptor routing" && git push
argocd app sync brdgme   # if auto-sync doesn't pick it up
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml rollout status deploy/operator -n brdgme
```

---

### Task 2: Manifests - HSOs everywhere, flag removed

**Files:**
- Modify: 19 `k8s/base/game/*/game-version.yaml` (remove `scaleToZero: true` line): age-of-war-1, battleship-1, category-5-1, cathedral-1, farkle-1, for-sale-1, greed-1, liars-dice-1, lost-cities-1, love-letter-1, modern-art-1, no-thanks-1, roll-through-the-ages-1, splendor-1, sushi-go-1, sushizock-1, texas-holdem-1, tic-tac-toe-2, zombie-dice-1
- Modify: the same 19 dirs' `http-scaled-object.yaml` (`scaledownPeriod: 300` -> `1800`)
- Create: `http-scaled-object.yaml` in 20 dirs: acquire-1, age-of-war-2, battleship-2, category-5-2, cathedral-2, farkle-2, for-sale-2, greed-2, jaipur-2, liars-dice-2, lost-cities-2, love-letter-2, modern-art-2, no-thanks-2, roll-through-the-ages-2, splendor-2, sushi-go-2, sushizock-2, texas-holdem-2, zombie-dice-2
- Modify: those 20 dirs' `kustomization.yaml` (add `- http-scaled-object.yaml`)

**Interfaces:**
- Consumes: nothing from Task 1 at build time (independent files), but MUST NOT be pushed until the Task 1 gate has passed.
- Produces: 39 uniform game dirs, each with deployment/service/game-version/http-scaled-object.

- [ ] **Step 1: Remove the flag and bump scaledownPeriod in the 19 existing dirs**

```bash
cd ~/Development/brdgme
existing="age-of-war-1 battleship-1 category-5-1 cathedral-1 farkle-1 for-sale-1 greed-1 liars-dice-1 lost-cities-1 love-letter-1 modern-art-1 no-thanks-1 roll-through-the-ages-1 splendor-1 sushi-go-1 sushizock-1 texas-holdem-1 tic-tac-toe-2 zombie-dice-1"
for n in $existing; do
  sed -i '/^  scaleToZero: true$/d' "k8s/base/game/$n/game-version.yaml"
  sed -i 's/^  scaledownPeriod: 300$/  scaledownPeriod: 1800/' "k8s/base/game/$n/http-scaled-object.yaml"
done
grep -rn scaleToZero k8s/base/game/ ; grep -rln 'scaledownPeriod: 300' k8s/base/game/
```

Expected: both greps print nothing.

- [ ] **Step 2: Create HSOs in the 20 remaining dirs and register them**

```bash
new="acquire-1 age-of-war-2 battleship-2 category-5-2 cathedral-2 farkle-2 for-sale-2 greed-2 jaipur-2 liars-dice-2 lost-cities-2 love-letter-2 modern-art-2 no-thanks-2 roll-through-the-ages-2 splendor-2 sushi-go-2 sushizock-2 texas-holdem-2 zombie-dice-2"
for n in $new; do
  cat > "k8s/base/game/$n/http-scaled-object.yaml" <<EOF
apiVersion: http.keda.sh/v1alpha1
kind: HTTPScaledObject
metadata:
  name: $n
spec:
  hosts:
  - $n.games.internal
  scaleTargetRef:
    name: $n
    kind: Deployment
    apiVersion: apps/v1
    service: $n
    port: 80
  replicas:
    min: 0
    max: 1
  scaledownPeriod: 1800
EOF
  grep -q http-scaled-object "k8s/base/game/$n/kustomization.yaml" || \
    printf -- '- http-scaled-object.yaml\n' >> "k8s/base/game/$n/kustomization.yaml"
done
```

Note: some `kustomization.yaml` files lack a trailing newline (e.g. age-of-war-2); verify each ends with a well-formed resource list afterwards: `for n in $new; do tail -c 80 "k8s/base/game/$n/kustomization.yaml"; echo; done` - every file must list all four resources on separate lines. Fix any run-together lines by hand.

- [ ] **Step 3: Verify the overlay builds**

Run: `kubectl kustomize k8s/prod > /dev/null && kubectl kustomize k8s/prod | grep -c 'kind: HTTPScaledObject'`
Expected: no error; count is 39.

- [ ] **Step 4: Verify no game dir is missing anything**

Run: `for d in k8s/base/game/*/; do [ -f "$d/http-scaled-object.yaml" ] || echo "MISSING $d"; done`
Expected: no output.

- [ ] **Step 5: Commit and push (ONLY after Task 1 gate confirmed)**

```bash
git add k8s/base/game/
git commit -m "feat: uniform scale-to-zero - HTTPScaledObjects for all 39 game versions

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
git push
```

k8s-only commit: CI will not build images (expected).

---

### GATE after Task 2 (Michael, manual)

```bash
cd ~/Development/brdgme-config
# edit prod/kustomization.yaml: ref -> <full sha of Task 2 commit>
git add prod/kustomization.yaml && git commit -m "deploy: uniform scale-to-zero manifests" && git push
argocd app sync brdgme

# then force the 20 unbumped CRs to reconcile (runs over all 39; extra 19 are no-op upserts):
for v in $(kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get gameversions -n brdgme -o name); do
  kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml patch "$v" -n brdgme \
    --subresource=status --type=merge -p '{"status":{"observedGeneration":null}}'
done
```

---

### Task 3: Verification (read-only) and docs

**Files:**
- Modify: `docs/superpowers/plans/2026-07-17-uniform-scale-to-zero.md` (record evidence)

**Interfaces:**
- Consumes: live cluster state after both gates.

- [ ] **Step 1: DB rows**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec -n brdgme postgres-1 -c postgres -- psql -d brdgme -c "SELECT uri, count(*) FROM game_versions WHERE is_public GROUP BY uri"`
Expected: a single interceptor URI row covering all public versions (no direct `*.brdgme.svc.cluster.local` URIs).

- [ ] **Step 2: HSOs and CR status**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get httpscaledobjects -n brdgme | grep -vc True; kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get gameversions -n brdgme -o jsonpath='{range .items[*]}{.metadata.name} {.metadata.generation} {.status.observedGeneration}{"\n"}{end}'`
Expected: 39 HSOs all READY (grep -vc True prints 1, the header line); every CR has observedGeneration == generation.

- [ ] **Step 3: Operator logs**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml logs deploy/operator -n brdgme --tail=100`
Expected: `Upserting game version` lines with the interceptor URI, no errors.

- [ ] **Step 4: Cold-start spot check on a previously always-on version**

Run: `kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml exec -n brdgme nats-0 -- wget -S -O /dev/null --header 'Host: acquire-1.games.internal' http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080/ 2>&1 | tail -5; kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml get deploy acquire-1 -n brdgme`
Expected: HTTP 405 (GET on JSON-RPC endpoint proves wake); deployment 1/1. (Only meaningful once acquire-1 has scaled to 0 - 1800s idle; if still 1/1 from rollout, note it and let Michael confirm scale-down later.)

- [ ] **Step 5: Record evidence and commit**

Append a "Verification record" section to this plan doc with the actual outputs, then:

```bash
git add docs/superpowers/plans/2026-07-17-uniform-scale-to-zero.md
git commit -m "docs: uniform scale-to-zero verification record

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
git push
```

## Verification record (2026-07-17)

All checks read-only, `--kubeconfig ~/.kube/brdgme-kubeconfig.yaml`.

**Step 1: DB rows** - PASS

```
SELECT uri, count(*) FROM game_versions WHERE is_public GROUP BY uri
                                  uri                                   | count
------------------------------------------------------------------------+-------
 http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080 |    39
(1 row)
```

Single interceptor URI covering all 39 public versions, no direct `*.brdgme.svc.cluster.local` rows.

**Step 2: HSOs and CR status** - PASS

`get httpscaledobjects -n brdgme | grep -vc True` -> `1` (header line only; all HSOs READY=True).

`gameversions` generation vs observedGeneration (39/39 rows, all equal):

```
acquire-1 1 1
age-of-war-1 3 3
age-of-war-2 1 1
battleship-1 3 3
battleship-2 1 1
category-5-1 3 3
category-5-2 1 1
cathedral-1 3 3
cathedral-2 1 1
farkle-1 2 2
farkle-2 1 1
for-sale-1 3 3
for-sale-2 1 1
greed-1 2 2
greed-2 1 1
jaipur-2 1 1
liars-dice-1 2 2
liars-dice-2 1 1
lost-cities-1 2 2
lost-cities-2 1 1
love-letter-1 3 3
love-letter-2 1 1
modern-art-1 3 3
modern-art-2 1 1
no-thanks-1 2 2
no-thanks-2 1 1
roll-through-the-ages-1 3 3
roll-through-the-ages-2 1 1
splendor-1 3 3
splendor-2 1 1
sushi-go-1 3 3
sushi-go-2 1 1
sushizock-1 3 3
sushizock-2 1 1
texas-holdem-1 3 3
texas-holdem-2 1 1
tic-tac-toe-2 2 2
zombie-dice-1 3 3
zombie-dice-2 1 1
```

**Step 3: Operator logs** - PASS

`logs deploy/operator --tail=100`: 39 `Upserting game version` lines, one per CR, each with `uri="http://keda-add-ons-http-interceptor-proxy.keda.svc.cluster.local:8080"` (sample: `acquire-1`, `age-of-war-1`, `age-of-war-2`). Remaining lines are `Spec unchanged since last reconcile, skipping` (steady-state re-reconciles). No error/warn lines in the tail.

**Step 4: Cold-start spot check** - SKIPPED (not applicable yet)

```
kubectl get deploy acquire-1 -n brdgme
NAME        READY   UP-TO-DATE   AVAILABLE   AGE
acquire-1   1/1     1            1           10d
```

acquire-1 is still 1/1, so the wget cold-start probe would not be meaningful (nothing to wake). Scale-down pending 1800s idle - Michael to confirm later.
