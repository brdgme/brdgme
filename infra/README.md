# infra/ - OpenTofu

Describes the DigitalOcean account infrastructure that Kubernetes cannot
self-describe: the DOKS cluster, the VPC, the `brdg.me` DNS zone (on
Cloudflare since 2026-07-10, plus legacy records until cutover), the
Spaces bucket for CloudNativePG (Phase 19) backups, and the Spaces bucket
for this configuration's own state. See
`docs/superpowers/specs/2026-07-06-21-opentofu-iac-design.md` for the
decision record and cost posture.

The DO account was confirmed empty on 2026-07-05 (current prod runs on
Linode with Route53 DNS), so everything here is **created** by tofu; the
only import is the bootstrapped state bucket. The Gateway-provisioned load
balancer is **not** managed here - DOKS owns it.

## Prerequisites

- `opentofu` (`tofu`) and `doctl` - available via `devenv.nix`.
- A DigitalOcean API token with read/write access, exported as
  `DIGITALOCEAN_TOKEN`.
- A Spaces access key/secret pair (generated separately from the API token,
  under "API > Spaces Keys" in the DO control panel), exported as
  `SPACES_ACCESS_KEY_ID` / `SPACES_SECRET_ACCESS_KEY` for the digitalocean
  provider's `digitalocean_spaces_bucket` resources. The S3-compatible state
  backend only reads the `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` names;
  `devenv.nix` exports those as aliases of the `SPACES_*` pair on shell entry,
  so only the `SPACES_*` pair needs to be set (see `.env.example`).
- A Cloudflare API token scoped to the brdg.me zone (Zone.DNS Edit +
  Zone.Zone Settings Edit + Zone.Zone Read), exported as
  `CLOUDFLARE_API_TOKEN` (the cloudflare provider reads it natively; see
  `.env.example`). The account ID has a committed default in
  `variables.tf`.

## Bootstrapping the state bucket

The state bucket (`digitalocean_spaces_bucket.tofu_state` in `spaces.tf`) is
also the backend target in `versions.tf`, so it can't be created through the
same `tofu apply` that uses it as a backend. Create `brdgme-tofu-state` once
in SYD1 via the DO console, then import it (below) so ongoing management
goes through tofu.

## Standing everything up

```sh
cd infra
tofu init

# Adopt the bootstrapped state bucket. Import ID is "<region>,<name>".
tofu import digitalocean_spaces_bucket.tofu_state syd1,brdgme-tofu-state

# Stage 1 - near-free resources (VPC, CNPG bucket). DNS is no longer
# created here - brdg.me now lives on Cloudflare (`cloudflare.tf`,
# adopted via import blocks); see "DNS migration to Cloudflare" below.
tofu apply -target=digitalocean_vpc.brdgme \
  -target=digitalocean_spaces_bucket.cnpg_backups

# Stage 2 - the cluster. ~$24/mo starts accruing the moment it exists;
# defer until ready to deploy. The Gateway LB (~$12/mo) appears later,
# when the first Gateway resource is created in the cluster.
tofu apply
```

`tofu plan` after both stages must show no changes.

## DNS migration from Route53

`brdg.me` is currently served by Route53 (the registrar still points NS
there); prod is a Linode host. The DO zone created here carries copies of
the legacy records (`dns.tf`) so existing prod keeps working when the
nameservers switch. Before switching NS at the registrar:

1. Verify the legacy record set against Route53 itself (console or
   `aws route53 list-resource-record-sets`) - the records in `dns.tf` were
   discovered via DNS queries on 2026-07-05 (apex A, `mail` A, apex SPF
   TXT), and queries cannot enumerate a zone. Add anything missed.
2. `tofu apply` stage 1 so the DO zone is fully populated.
3. Switch the nameservers at the registrar to `ns1-3.digitalocean.com`.
   Zero downtime if the record sets match. Keep the Route53 zone around
   (unchanged) for a week as a fallback, then delete it to stop its charge.

The NS switch is required before item 22a's Resend records (SPF/DKIM/DMARC)
can take effect, and before external-dns (Phase 20) manages records here.
The legacy records are removed at decommission (item 16).

## DNS migration to Cloudflare (2026-07-10)

`brdg.me` moved from DO nameservers to Cloudflare for item 28 WP4 (free
WAF/rate-limiting/proxy edge in front of beta, later the apex). Unlike
the Route53 move, this one was done manually ahead of Tofu adoption:
Michael created the zone in the CF dashboard (free plan), CF copied the
DO records at zone creation, and the registrar NS were cut over the same
day - `cloudflare.tf` then ADOPTED the live zone and records via import
blocks (no resources created), reconciling proxied flags to the design
(8 legacy/Resend records DNS-only, `beta` proxied). The DO zone
(`dns.tf`) was removed once DNS01 issuance through Cloudflare was
verified. TLS moved from HTTP01 to DNS01 at the same time
(`k8s/base/cert-manager/cluster-issuer.yaml`), with the token sealed for
cert-manager in `brdgme-config`. See
`docs/superpowers/specs/2026-07-10-28-wp4-cloudflare-pre-golive-design.md`
and the matching plan for the full decision record.
