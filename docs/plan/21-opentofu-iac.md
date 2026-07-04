# 21: OpenTofu Infrastructure as Code

**Status:** Pending - human-paced

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

- [ ] Add `opentofu` to `devenv.nix`.
- [ ] `infra/` directory: DO provider, S3 backend against a Spaces bucket.
- [ ] `tofu import` the existing resources (cluster, VPC, domain) - do not
      recreate. `tofu plan` must show no changes after import before
      anything else is done.
- [ ] Encode the Phase 14 prerequisite: cluster version >= 1.33 with
      VPC-native networking.
- [ ] Create new resources (CNPG backup bucket for Phase 19, state bucket)
      via tofu from the start.

