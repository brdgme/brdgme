# 21: OpenTofu Infrastructure as Code - Design

> Extracted 2026-07-08 from `docs/plan/21-opentofu-iac.md` (superpowers layout
> migration). Content dates from 2026-07-06; this is a point-in-time decision
> record, not a living document.

**Status:** Complete 2026-07-06 (stage 1 + stage-2 applied 2026-07-05;
state-bucket versioning applied and Route53 zone deleted 2026-07-06). The
`beta` record has since been added (`digitalocean_record.beta_a` in
`infra/dns.tf`); apex repoint remains part of the Phase 16 runbook, not
this item. See docs/superpowers/plans/2026-07-08-16-production-cutover-validation.md "Beta
period".

**Decision (2026-07-03 tech review):** describe the DigitalOcean account
infrastructure in OpenTofu (Linux Foundation Terraform fork; open source,
matching project principles). Scope is only what Kubernetes cannot
self-describe: the DOKS cluster, the VPC, the `brdg.me` DNS zone and its
records (all managed directly in `infra/dns.tf`), the Spaces bucket for
CNPG backups, and the Spaces bucket for tofu state. The Gateway-provisioned
load balancer is NOT managed here - DOKS owns it.

**Sequencing (revised 2026-07-04):** entirely human-operated (account
credentials), and now scheduled **first** among the pre-go-live infra
phases - Michael wants the account infrastructure described in tofu from
the start. It encodes the Phase 14 prod prerequisite ("cluster >= 1.33,
VPC-native"), owns the DNS zone the 22a Resend records (SPF/DKIM/DMARC)
land in, and creates the Spaces buckets Phase 19 needs.

**Cost posture (decided 2026-07-05):** side project, no income stream -
minimise spend without hacks; prefer managed where free/cheap. Target floor
~$41/mo: one basic node (s-2vcpu-4gb, $24) + one Gateway-provisioned LB
($12) + Spaces flat subscription ($5, covers ALL buckets). Constraints the
tofu config must preserve:

- No HA control plane (`ha = false` explicit in `cluster.tf` - HA is
  $40/mo. Must be explicit, not just unset: since DOKS 1.36.0 (May 2026)
  DO enables HA by default when the field is left unset, and HA cannot be
  disabled after creation - discovered 2026-07-05 when the first stage-2
  apply landed with `ha: true`, requiring a destroy/recreate of the
  cluster to fix).
- Single node pool, basic (shared CPU) tier, no cluster autoscaling. Node
  scaling is a **manual human decision** - `ignore_changes = [node_pool]`
  on the cluster resource is deliberate and must stay.
- Exactly ONE Gateway in the cluster - each Gateway provisions its own DO
  LB at $12/mo, so a second Gateway silently doubles LB cost.
- Container images live on GHCR (free, public packages), not DOCR - no
  registry resources belong in this config.
- In-cluster state over managed services where backup discipline covers
  the risk: CNPG instead of DO Managed Postgres ($15+/mo) - revisit only
  if Phase 19 restore verification proves shaky.
