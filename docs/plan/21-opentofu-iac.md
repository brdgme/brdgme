# 21: OpenTofu Infrastructure as Code

**Status:** Stage 1 applied, Route53 records verified, NS switched to DO
(all 2026-07-05); remaining: Route53 zone deletion (~2026-07-12), then
stage-2 apply (cluster) when ready to spend - human-paced

**Decision (2026-07-03 tech review):** describe the DigitalOcean account
infrastructure in OpenTofu (Linux Foundation Terraform fork; open source,
matching project principles). Scope is only what Kubernetes cannot
self-describe: the DOKS cluster, the VPC, the `brdg.me` DNS zone (the zone
belongs to tofu, records to external-dns), the Spaces bucket for CNPG
backups, and the Spaces bucket for tofu state. The Gateway-provisioned load
balancer is NOT managed here - DOKS owns it.

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

- No HA control plane (`ha` stays unset/false - HA is $40/mo).
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

- [x] Add `opentofu` to `devenv.nix`.
- [x] `infra/` directory: DO provider, S3 backend against a Spaces bucket.
- [x] ~~`tofu import` the existing resources~~ Revised 2026-07-05: the DO
      account was confirmed **empty** (current prod is Linode + Route53
      DNS), so everything is created by tofu, not imported. `infra/`
      rewritten accordingly; the only import is the bootstrapped state
      bucket. Legacy DNS records (apex A, `mail` A, apex SPF TXT →
      Linode) are carried in `infra/dns.tf` so prod survives the
      Route53 → DO nameserver switch; see `infra/README.md`.
- [x] Bootstrap `brdgme-tofu-state` Spaces bucket (console), set Spaces
      keys, `tofu init` + import the bucket, stage-1 apply (VPC, zone +
      legacy records, CNPG bucket). Done 2026-07-05; the `brdgme` DO
      project was also created manually and imported. State confirmed:
      VPC, project, domain, 3 legacy records, both buckets. `tofu plan`
      shows only the expected stage-2 adds (cluster, project-resource
      assignment) plus a trivial in-place `acl: private` on the imported
      state bucket.
- [x] Verify records against Route53. Done 2026-07-05 via a console
      export of the zone: it contains only apex A (172.105.164.158),
      `mail` A (172.105.164.158), and the apex SPF TXT, plus NS/SOA
      (zone-internal, never migrated) - `infra/dns.tf` carries all of
      them with identical values. Only difference: Route53 TTLs are 300s
      vs 3600s in tofu - cosmetic (slower propagation of future edits),
      not a correctness issue.
- [x] Switch nameservers at the registrar to `ns1-3.digitalocean.com`.
      Done 2026-07-05.
- [ ] Delete the Route53 zone after ~a week (kept as fallback until
      ~2026-07-12) to stop its charge.
- [ ] Stage-2 apply (the DOKS cluster) when ready to deploy - billing
      starts at creation.
- [x] Encode the Phase 14 prerequisite: cluster version >= 1.33 with
      VPC-native networking.
- [x] Create new resources (CNPG backup bucket for Phase 19, state bucket)
      via tofu from the start. Both exist and are in state.

