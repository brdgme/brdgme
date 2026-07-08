# 21: OpenTofu Infrastructure as Code - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/21-opentofu-iac.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete 2026-07-06 (stage 1 + stage-2 applied 2026-07-05;
state-bucket versioning applied and Route53 zone deleted 2026-07-06). The
`beta` record has since been added (`digitalocean_record.beta_a` in
`infra/dns.tf`); apex repoint remains part of the Phase 16 runbook, not
this item. See docs/superpowers/plans/2026-07-08-16-production-cutover-validation.md "Beta
period".

**Spec:** `docs/superpowers/specs/2026-07-06-21-opentofu-iac-design.md`

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
- [x] Delete the Route53 zone to stop its charge. Done 2026-07-06. (Originally
      "after ~a week / ~2026-07-12"; brought forward 2026-07-06 after
      verifying the `.me` TLD delegation TTL is only 3600s and the live
      delegation + answers already match `infra/dns.tf` - the propagation
      risk the week was buffering has passed.) Steps:
      1. AWS console → Route53 → Hosted zones → `brdg.me` → delete every
         record EXCEPT the NS and SOA (the console requires this before
         zone deletion).
      2. Delete the hosted zone itself.
      3. There is nothing to change in tofu - the zone was never in state.
- [x] Stage-2 apply (the DOKS cluster). Done 2026-07-05: `brdgme` cluster
      created in `syd1`, version `1.36.0-do.2`, `s-2vcpu-4gb` single node,
      `ha=false`, VPC-native. Recreated once after the first apply landed
      with `ha=true` (DOKS 1.36.0 default-flip, see cost posture note
      in the spec) - `tofu plan` now shows no changes.
- [x] Encode the Phase 14 prerequisite: cluster version >= 1.33 with
      VPC-native networking. Verified live: see
      `docs/superpowers/plans/2026-07-05-14-drop-knative-gateway-api.md`.
- [x] Create new resources (CNPG backup bucket for Phase 19, state bucket)
      via tofu from the start. Both exist and are in state.
- [ ] Cutover hostname records (revised 2026-07-05 to match the Phase 16
      beta-then-hard-cutover flow; supersedes Phase 20's external-dns
      controller plan, see docs/superpowers/specs/2026-07-08-20-external-dns-design.md): at beta start,
      add a `beta.brdg.me` A record pointing at the Gateway LB IP; at
      cutover, repoint the apex `brdg.me` A record from Linode to the LB
      IP (and lower legacy TTLs to 300 beforehand - see the Phase 16
      runbook). `legacy.brdg.me`/`api.brdg.me`/`ws.brdg.me` are NOT
      created unless the break-glass overlay is invoked. Human-operated
      (needs the live LB IP).
- [x] Enable versioning on the `brdgme-tofu-state` Spaces bucket (agreed
      2026-07-05; guards against a corrupted/clobbered state file).
      Applied 2026-07-05: `versioning { enabled = true }` in
      `infra/spaces.tf`, clean single-change apply.
