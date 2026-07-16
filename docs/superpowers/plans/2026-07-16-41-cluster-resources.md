# 41 - Cluster resource improvements (2026-07-16)

Investigation + preparation session. NOTHING in this session deploys to or
mutates the production cluster; all cluster access was read-only. Prepared
changes live on LOCAL branches only (never pushed); the deploy step is a
separate, operator-approved action - see the Deploy runbook at the bottom.

Requirements record: docs/BACKLOG.md row 41 (added 2026-07-16).

## Repo/deploy topology (surveyed this session)

- brdgme-config is the GitOps repo. ArgoCD watches its prod/ directory only.
  prod/kustomization.yaml uses a kustomize remote base pinned to a brdgme
  source-repo commit: https://github.com/brdgme/brdgme//k8s/prod?ref=SHA.
  So changes under k8s/ in the brdgme repo deploy via a ref bump in
  brdgme-config (normally done by CI on master merge).
- Cluster-scoped addons (argocd/, cert-manager/, cnpg-operator/,
  sealed-secrets/ in brdgme-config) are MANUAL-APPLY pinned kustomizations
  (kubectl apply -k <dir>), not ArgoCD-managed. metrics-server follows this
  pattern.
- Alloy is ArgoCD-managed: k8s/prod/alloy/ in the brdgme repo, pulled in via
  k8s/prod/app/kustomization.yaml.
- Sealed secrets live in brdgme-config/sealed-secrets/secrets/ and are
  included in the ArgoCD app via prod/kustomization.yaml.

## Task plan

| # | Sub-item | Where the change lands | Status |
|---|----------|------------------------|--------|
| 1 | (b) verify alloy OOM + stuck Tempo exporter (read-only) | evidence only | done - confirmed, live |
| 2 | (e) restart-loop diagnosis (read-only) | findings only | done - historic, diagnosed |
| 3 | (f) topology data gathering (read-only) | analysis only | done - no change proposed |
| 4 | (a) metrics-server GitOps prep | brdgme-config branch 41-cluster-resources | done - b0cbe74 |
| 5 | (b) alloy traces disable + OTEL env removal | brdgme branch 41-cluster-resources (k8s/prod) | prepared (see runbook) |
| 6 | (c) requests/limits for BestEffort pods | both repos | prepared (see runbook) |
| 7 | (d) GHCR pull secret prep + operator instructions | brdgme-config branch + brdgme branch | prepared (see runbook) |
| 8 | Findings/proposals written up here | this file | done |

## Findings (read-only evidence, gathered 2026-07-16)

Cluster: 2x s-2vcpu-4gb DOKS nodes (allocatable 1900m CPU / ~3003Mi each),
server v1.36.0. Raw evidence dumps were written to the session scratchpad
(evidence/ dir, 34 files); key figures are inlined below.

### (a) metrics-server absence - confirmed

No metrics.k8s.io APIService registered at all; `kubectl top nodes` returns
"error: Metrics API not available". Right-sizing is blind without it.

### (b) Alloy verification - confirmed, and LIVE, not historic

- Pod QoS Burstable (requests 100m/128Mi, limit 256Mi memory).
- restartCount 4; lastState OOMKilled exit 137 at 2026-07-15T17:44:53Z.
- Current workingSet at observation time: ~234MiB of the 256Mi limit
  (~91%) and climbing - another OOMKill is likely without intervention.
- Current logs (200-line tail): 197/200 lines are
  otelcol.exporter.otlp.grafana_cloud errors "last resolver error:
  produced zero addresses", continuous retry with periodic "Dropping
  data" (queue exhausted). The pre-OOM previous-container log tail shows
  the identical loop plus "sending queue is full" / rejected_items
  immediately before death.
- Conclusion: the stuck Tempo exporter (#32) backs up its retry queue and
  drives alloy into its memory limit. The traces pipeline currently
  delivers zero value (all data dropped) at real memory cost. Disabling it
  (not raising the limit) is the right first move, per the 2026-07-16
  decision.

### (e) Restart-loop diagnosis - historic, control-plane reachability

All affected pods run on node brdgme-pool-3cv8wq (the infra node).

| workload | QoS | restarts | last crash | stable since |
|---|---|---|---|---|
| cert-manager-controller | BestEffort | 167 | 2026-07-08 04:05Z exit 1 | 8 days |
| cert-manager-cainjector | BestEffort | 165 | 2026-07-08 04:05Z exit 1 | 8 days |
| cnpg-controller-manager | Burstable (100m/100Mi, limit 200Mi) | 245 | 2026-07-11 04:37Z exit 1 | 5 days |
| barman-cloud | BestEffort | 4 | 2026-07-10 exit 1 | 6 days |

Previous-container logs give the causes:

- cnpg-controller-manager: "Failed to renew lease ... context deadline
  exceeded" -> "leader election lost" -> exit 1. Leader-election lease
  renewal timing out.
- cert-manager + cainjector: crashed at startup with
  `Get "https://10.127.32.1:443/...": EOF` while probing CRDs/API groups -
  API-server connectivity failure at boot; both died in the same
   04:05:23-33Z window on 2026-07-08.

Diagnosis: both failure modes are the same family - transient
control-plane (API server / konnectivity) unreachability. Exit-1 crash ->
restart is these controllers' designed behaviour when leadership or the
API connection is lost; the loops are historic (all pods stable 5-8 days,
current logs clean) and the counts accumulated over past incident windows.
Cluster events have fully expired, so node-pressure timing could not be
corroborated, but note these are (mostly) BestEffort pods on the node that
also runs the whole argocd stack. Sub-item (c)'s requests will not fix an
API-server-side blip, but they reduce the local contribution (CPU
starvation delaying lease renewal, memory pressure) and make the pods less
likely to be evicted first. No fix beyond (c) is proposed; if loops resume
after (c) + metrics-server land, investigate DOKS control-plane health
(konnectivity) instead.

### (c) Usage evidence for BestEffort pods (kubelet Summary API)

metrics-server is absent; numbers below are point-in-time workingSet from
`kubectl get --raw /api/v1/nodes/<node>/proxy/stats/summary` (best
available evidence, single sample - treat as indicative, revisit once
metrics-server lands).

| pod | workingSet MiB | cpu m | QoS today |
|---|---|---|---|
| postgres-1 (brdgme) | 189.4 | 7 | BestEffort |
| argocd-application-controller-0 | 478.8 | 6 | BestEffort |
| argocd-server | 63.5 | 3 | BestEffort |
| argocd-repo-server | 48.0 | <1 | BestEffort |
| argocd-applicationset-controller | 37.3 | <1 | BestEffort |
| argocd-notifications-controller | 35.7 | <1 | BestEffort |
| argocd-dex-server | 39.3 | <1 | BestEffort |
| argocd-redis | 9.3 | 7 | BestEffort |
| cert-manager-controller | 45.9 | <1 | BestEffort |
| cert-manager-cainjector | 67.0 | <1 | BestEffort |
| cert-manager-webhook | 27.4 | <1 | BestEffort |
| cnpg-controller-manager | 57.1 | 5 | Burstable |
| barman-cloud | 27.3 | 1 | BestEffort |
| sealed-secrets-controller | 16.1 | <1 | BestEffort |

The production database (postgres-1) has zero resource protection and
shares a node with the unbounded 479MiB argocd application-controller.

Sizing decisions (prepared this session). Basis: single-sample workingSet
above + headroom; NONE of these are load-tested numbers - revisit all of
them once metrics-server (a) has a week of data. Requests are deliberately
close to observed usage (scheduling truth + eviction protection); limits
are 2-4x observed so we do not introduce new OOM restarts on
leader-elected controllers.

| workload | cpu req | mem req | mem limit | basis |
|---|---|---|---|---|
| postgres (CNPG Cluster spec) | 250m | 512Mi | 1Gi | observed 189Mi; DB gets the most headroom; CNPG default shared_buffers fits well inside |
| migrate job (prod patch) | 25m | 64Mi | 256Mi | short-lived sqlx migrator, no sample available |
| argocd-application-controller | 100m | 384Mi | 768Mi | observed 479Mi workingSet (request intentionally below single-sample peak; limit 1.6x) |
| argocd-server | 25m | 64Mi | 192Mi | observed 64Mi |
| argocd-repo-server | 25m | 96Mi | 512Mi | observed 48Mi; manifest-gen spikes are the known failure mode, generous limit |
| argocd-applicationset-controller | 10m | 48Mi | 128Mi | observed 37Mi |
| argocd-notifications-controller | 10m | 48Mi | 128Mi | observed 36Mi |
| argocd-dex-server | 10m | 48Mi | 128Mi | observed 39Mi |
| argocd-redis | 10m | 32Mi | 128Mi | observed 9Mi |
| cert-manager-controller | 10m | 64Mi | 192Mi | observed 46Mi |
| cert-manager-cainjector | 10m | 96Mi | 256Mi | observed 67Mi |
| cert-manager-webhook | 10m | 48Mi | 128Mi | observed 27Mi |
| sealed-secrets-controller | 10m | 32Mi | 128Mi | observed 16Mi |
| barman-cloud | 10m | 48Mi | 128Mi | observed 27Mi |
| cnpg-controller-manager | unchanged | unchanged | unchanged | already Burstable 100m/100Mi req, 200Mi limit; observed 57Mi |

Consequence, stated deliberately: the infra node's memory requests rise
from ~38% to ~87% of allocatable. That is intentional truth-telling - the
node's REAL usage is already ~69% - and it stops the scheduler treating
the infra node as roomy. CNPG Guaranteed QoS (requests=limits) was
considered for postgres and deferred: too rigid before metrics-server
data exists on this tight fleet.

### (f) Topology spread analysis

Data:

- Node 3c0il9 ("app node"): 49 pods - all 39 game workers, alloy, bot,
  operator, 1x web, kube-system daemonsets. Requests: cpu 57%, mem 70%;
  limits: mem 151% (overcommitted). Real workingSet: ~2073MiB (~53% of
  physical).
- Node 3cv8wq ("infra node"): 26 pods - argocd (7), cert-manager (3),
  cnpg-system (2), nats-0, postgres-1, 1x web, kube-system control pods.
  Requests: cpu 46%, mem 38%; limits: mem 80%. Real workingSet: ~2688MiB
  (~69% of physical) - the infra node is under MORE real memory pressure
  than the game node, driven by argocd-application-controller (479MiB) and
  postgres-1 (189MiB), both currently BestEffort.

Analysis / proposal:

1. The headline "all games on node1" skew is a REQUESTS skew, not a real
   usage problem: 39 game workers x 32Mi requests = ~1.2Gi requested but
   real usage is far lower (node total ~53% used). The infra node is the
   one closer to real memory pressure.
2. Backlog #42 (approved 2026-07-16) will scale non-latest game versions
   to zero via KEDA HTTP add-on - that removes ~17 idle deployments'
   requests (~550Mi) from the picture. Adding topology spread constraints
   to game workers NOW would fight #42: spreading idle deployments that
   are about to stop existing as running pods, and KEDA-managed scale
   from zero does not benefit from spread on 1-replica deployments anyway
   (a 1-replica deployment cannot spread).
3. Recommendation: do NOT add topology spread constraints to game workers.
   Instead:
   - Let (c) land requests on the infra stack so the scheduler sees the
     infra node's true cost (today it looks 38% requested while really
     69% used, which is why the scheduler kept stacking onto it looking
     attractive for new pods).
   - Let #42 remove the idle -1 edition replicas.
   - Re-evaluate after both land, with metrics-server data. If node1
     requests % is still high, the cheap lever is `replicas: 1` -> keep
     (already 1) and rely on #42; the next lever would be a
     topologySpreadConstraint on the web deployment only (2 replicas,
     already split 1/1 across nodes today by default scheduling - no
     change needed unless that drifts).
4. No change is prepared for (f) this session by design - analysis only.

### (d) GHCR pull secret - design and operator instructions for Michael

Evidence status: all cluster events have expired, so the historic
`pull QPS exceeded` / ImagePullBackOff churn could not be re-confirmed
this session; no pods are currently in ImagePullBackOff. Game containers
use imagePullPolicy IfNotPresent, so pull storms happen on deploys (39+
images repulled per node on a ref bump), which is exactly when anonymous
GHCR QPS limits bite. Authenticated pulls get much higher limits.

Design (survey result): no imagePullSecrets exist anywhere today (default
ServiceAccount in brdgme is bare, deployments set none). Wiring goes on
the `default` ServiceAccount in the brdgme namespace via a prod-only
manifest in k8s/prod/app (brdgme repo) - one place, covers all 39 game
deployments + web/bot/operator Deployments, the migrate Job, and any
future #42 KEDA-scaled pods, without touching 40+ base manifests (dev
keeps using local images and needs no secret). A pod whose
imagePullSecrets references a not-yet-existing secret still pulls
anonymously (warning event only), so the wiring is safe to deploy before
or after the secret - but the intended order is secret first.

Operator steps (Michael) - all in brdgme-config, inside `devenv shell`,
kubectl context do-syd1-brdgme:

1. Create a GitHub Personal Access Token (classic) with ONLY the
   `read:packages` scope. Packages live under the brdgme org, so if org
   SSO/approval applies, authorize the token for the org. (Fine-grained
   PATs do not support registry auth for org packages in all cases;
   classic + read:packages is the documented-safe choice.)
2. Seal it (do NOT create the secret directly in the cluster):

   kubectl create secret docker-registry ghcr-pull -n brdgme \
     --docker-server=ghcr.io \
     --docker-username=<github-username> \
     --docker-password=<the-PAT> \
     --dry-run=client -o yaml \
   | kubeseal --format yaml > sealed-secrets/secrets/ghcr-pull.yaml

3. Add `ghcr-pull.yaml` to the resources list in
   sealed-secrets/secrets/kustomization.yaml (the prepared branch carries
   a skeleton/instructions file showing exactly this).
4. Commit and push brdgme-config; ArgoCD syncs the SealedSecret and the
   controller materializes secret `ghcr-pull` in namespace brdgme.
   Verify: kubectl get secret ghcr-pull -n brdgme
5. Deploy the ServiceAccount wiring (the prepared brdgme commit - see the
   deploy runbook) via the normal ref-bump path.
6. Verify on the next image-bump deploy: no `pull QPS exceeded` events;
   kubectl get events -n brdgme | grep -i pull

Revert: remove the imagePullSecrets entry from the ServiceAccount
manifest (or revert the wiring commit); delete
sealed-secrets/secrets/ghcr-pull.yaml and its kustomization entry;
revoke the PAT on GitHub.

## Deploy runbook

Nothing below has been applied to the cluster. Branches are LOCAL ONLY
(never pushed). Suggested order: 1 (metrics-server) any time; 2 (alloy
trim) ASAP - alloy is at ~91% of its limit; 3+4 together via one ref
bump; GHCR secret (operator steps in (d) above) before or after the
wiring, either is safe.

### 1. metrics-server addon (#41a)

- Repo: brdgme-config, branch `41-cluster-resources`, commit `b0cbe74`
  "feat: add metrics-server addon (backlog #41a)".
- Adds `metrics-server/kustomization.yaml` pinned to upstream v0.9.0
  components.yaml (compat matrix covers k8s 1.34+; cluster is 1.36.0). No
  `--kubelet-insecure-tls` - DigitalOcean's own marketplace stack keeps
  secure TLS defaults on DOKS. No args patches needed (upstream already
  ships `--kubelet-preferred-address-types=InternalIP,ExternalIP,Hostname`).
  README.md gains bootstrap step 9.
- Deploy (manual-apply addon, NOT ArgoCD): merge/push the branch, then
  `kubectl apply -k metrics-server/` from the repo root (devenv shell).
- Verify: `kubectl -n kube-system rollout status deploy/metrics-server`,
  then after ~1 min `kubectl top nodes` and `kubectl top pods -A` return
  data; `kubectl get apiservice v1beta1.metrics.k8s.io` shows Available.
- Revert: `kubectl delete -k metrics-server/` + git revert.

### 2. alloy traces disable (#41b)

- Repo: brdgme, branch `41-cluster-resources`, commit `77ab35c`
  "feat: disable alloy traces pipeline while Tempo exporter is broken (#41b)".
- Changes: k8s/prod/alloy/configmap.yaml - Job 3 (otelcol receiver/auth/
  exporter) removed, replaced by a disable-note comment; Jobs 1 (Loki
  logs) and 2 (Prometheus metrics incl. CNPG backup series) untouched.
  k8s/prod/app/web-patch.yaml - OTEL_EXPORTER_OTLP_ENDPOINT removed
  (OTEL_TRACES_SAMPLER_ARG kept, inert). alloy deployment.yaml/service.yaml
  deliberately untouched (dead 4317/4318 ports are harmless, keeps the
  revert trivial). The 256Mi limit is NOT raised, per the 2026-07-16
  decision.
- Verify after sync: alloy pod restarts stop accruing; logs no longer
  show otelcol resolver errors; workingSet drops well below 256Mi
  (kubectl top pod -n brdgme once metrics-server is in); Loki still
  receiving logs and Grafana Cloud still receiving cnpg_collector_* and
  web metrics; web pods restarted without the OTEL endpoint env.
- Revert (re-enable traces once #32/quota is resolved):
  `git revert 77ab35c` in brdgme + normal deploy.

### 3. postgres + migrate resources (#41c, brdgme half)

- Repo: brdgme, branch `41-cluster-resources`, commit `771359b`
  "feat: set resources for postgres and migrate job (#41c)".
- Changes: postgres CNPG Cluster gains requests 250m/512Mi + limit 1Gi
  (k8s/prod/app/postgres-patch.yaml); new k8s/prod/app/migrate-patch.yaml
  gives the migrate Job 25m/64Mi requests + 256Mi limit; registered in
  the app kustomization patches list.
- CAUTION: changing the CNPG Cluster resources triggers a rolling restart
  of postgres-1 (single instance = brief DB outage). Deploy in a quiet
  window.
- Verify after sync: `kubectl get pod postgres-1 -n brdgme -o
  jsonpath='{.status.qosClass}'` returns Burstable (was BestEffort);
  cluster healthy (`kubectl get cluster -n brdgme`); next migrate Job run
  completes normally.
- Revert: `git revert 771359b` + deploy (another postgres restart).

### 4. GHCR pull-secret wiring (#41d, brdgme half)

- Repo: brdgme, branch `41-cluster-resources`, commit `9f89ab8`
  "feat: wire ghcr-pull imagePullSecrets on prod service accounts (#41d)".
- Changes: new k8s/prod/app/default-serviceaccount.yaml (default SA,
  namespace brdgme, imagePullSecrets ghcr-pull) + new
  k8s/prod/operator-sa-patch.yaml for the brdgme-operator SA; registered
  in the respective kustomizations. Safe to deploy before the secret
  exists (missing pull secret = anonymous fallback + warning event).
- Verify after sync: `kubectl get sa default brdgme-operator -n brdgme -o
  yaml` shows imagePullSecrets; on the next image bump, pods pull without
  QPS errors (needs the sealed secret from runbook entry 6 to be live for
  the authenticated path).
- Revert: `git revert 9f89ab8` + deploy.

### 5. addon requests/limits (#41c, brdgme-config half)

- Repo: brdgme-config, branch `41-cluster-resources`, commit `d581709`
  "feat: set resources for argocd, cert-manager, sealed-secrets,
  barman-cloud (#41c)".
- Changes: JSON patches in argocd/, cert-manager/, sealed-secrets/, and
  cnpg-operator/barman-cloud-plugin/ kustomizations applying the sizing
  table above (11 workloads; no cpu limits; cert-manager's existing
  --enable-gateway-api / leader-election args verified intact in the
  render). cnpg-controller-manager untouched (already sized).
- Deploy (manual-apply addons, NOT ArgoCD): merge/push the branch, then
  per directory: `kubectl apply -k argocd/`, `kubectl apply -k
  cert-manager/`, `kubectl apply -k sealed-secrets/`, `kubectl apply -k
  cnpg-operator/barman-cloud-plugin/`. Each triggers rolling restarts of
  the patched controllers (brief; argocd application-controller restart
  pauses sync momentarily, cert-manager restart is safe outside a cert
  renewal window).
- Verify: `kubectl get pods -n argocd -o
  jsonpath='{range .items[*]}{.metadata.name}{" "}{.status.qosClass}{"\n"}{end}'`
  shows Burstable (repeat for cert-manager, kube-system
  sealed-secrets-controller, cnpg-system barman-cloud); all pods Ready;
  no new OOMKills over the following days (`kubectl get pods -A` restart
  columns; `kubectl top` once metrics-server is in).
- Revert: `git revert d581709` + re-apply the same four dirs.

### 6. GHCR sealed-secret skeleton (#41d, brdgme-config half)

- Repo: brdgme-config, branch `41-cluster-resources`, commit `2eacf90`
  "docs: add ghcr-pull sealed-secret skeleton and operator instructions
  (#41d)".
- Changes: sealed-secrets/secrets/ghcr-pull.yaml.example (operator steps +
  commented SealedSecret shape; deliberately NOT in the secrets
  kustomization, so it can never sync as a placeholder) + a 3-line README
  pointer in bootstrap step 3.
- Deploy: nothing to deploy directly - Michael follows the steps in the
  .example file / the (d) section above (PAT -> kubeseal -> register
  ghcr-pull.yaml -> push).
- Revert: delete the .example + README lines (docs-only).

Note on deploy mechanics for 2-4: these live in the brdgme repo's k8s/prod
tree, which reaches prod via brdgme-config's prod/kustomization.yaml remote
base `?ref=`. Normal path: merge the brdgme branch to master and let CI
push the ref bump to brdgme-config; ArgoCD then syncs. Verify/revert steps
per entry below.

## Deployed 2026-07-16

All runbook entries were applied to prod later the same day (separate,
operator-approved deploy session). Verified results:

1. metrics-server live - metrics.k8s.io Available, `kubectl top` returns
   node and pod data.
2. Alloy traces trim - 0 restarts since, otelcol resolver-error loop gone
   from logs, memory 234Mi -> 162Mi at verification time.
3. Addon resources - argocd, cert-manager, sealed-secrets and barman-cloud
   pods all Burstable QoS and Ready. Benign note: `kubectl apply -k
   argocd/` exits 1 on a pre-existing applicationsets CRD annotation-size
   limit; use server-side apply for that directory later.
4. Postgres + migrate resources - CNPG switchover took ~2.5 min, cluster
   healthy, web clean afterwards.
5. ghcr-pull - sealed secret unsealed; both ServiceAccounts (default,
   brdgme-operator) carry the imagePullSecrets wiring; no pull errors.

ArgoCD Synced + Healthy at brdgme-config a97fba1; prod ref = brdgme
9f89ab8. Zero unhealthy pods cluster-wide at verification.

### Post-deploy baseline (kubectl top, captured 2026-07-16)

| node | cpu | cpu % | memory | mem % |
|---|---|---|---|---|
| brdgme-pool-3c0il9 | 288m | 15% | 2449Mi | 81% |
| brdgme-pool-3cv8wq | 267m | 14% | 2305Mi | 76% |

Key pods from the sizing table (all Ready; 0 restarts since the deploy -
cnpg-controller-manager's 245 count is the historic figure from (e), last
restart 5d before the deploy):

| pod | cpu m | memory MiB |
|---|---|---|
| postgres-1 (brdgme) | 27 | 113 |
| argocd-application-controller-0 | 8 | 312 |
| argocd-server | 1 | 95 |
| argocd-repo-server | 1 | 89 |
| argocd-applicationset-controller | 1 | 28 |
| argocd-notifications-controller | 1 | 70 |
| argocd-dex-server | 1 | 95 |
| argocd-redis | 7 | 10 |
| cert-manager-controller | 1 | 79 |
| cert-manager-cainjector | 1 | 42 |
| cert-manager-webhook | 1 | 19 |
| cnpg-controller-manager | 6 | 58 |
| barman-cloud | 1 | 19 |
| sealed-secrets-controller | 1 | 15 |
| alloy (brdgme) | 9 | 185 |

Notes: argocd-dex-server (95Mi) and argocd-notifications-controller
(70Mi) run above their 48Mi requests but within their 128Mi limits;
alloy at 185Mi sits comfortably under its unchanged 256Mi limit with
traces disabled. Revisit all sizings after ~a week of metrics-server
data, per (c).
