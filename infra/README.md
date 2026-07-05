# infra/ - OpenTofu

Describes the DigitalOcean account infrastructure that Kubernetes cannot
self-describe: the DOKS cluster, the VPC, the `brdg.me` DNS zone, the Spaces
bucket for CloudNativePG (Phase 19) backups, and the Spaces bucket for this
configuration's own state. See `docs/plan/21-opentofu-iac.md` for the
decision record.

The Gateway-provisioned load balancer is **not** managed here - DOKS owns
it.

## Prerequisites

- `opentofu` (`tofu`) - available via `devenv.nix`.
- A DigitalOcean API token with read/write access, exported as
  `DIGITALOCEAN_TOKEN`.
- A Spaces access key/secret pair (generated separately from the API token,
  under "API > Spaces Keys" in the DO control panel), exported as
  `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` for the S3-compatible backend.

## Before doing anything: fix the placeholders

Every value in `variables.tf` marked "must match reality" is a guess and
**will not match the live account**. Before running `tofu import`, check the
real values and update the variable defaults (or pass `-var` overrides):

- `cluster_name`, `node_pool_name`, `node_pool_size`,
  `node_pool_node_count`: `doctl kubernetes cluster list`,
  `doctl kubernetes cluster node-pool list <cluster>`.
- The exact cluster `version` in `cluster.tf`:
  `doctl kubernetes cluster get <cluster> --format Version`.
- `vpc_name` and the VPC `ip_range` in `vpc.tf`: `doctl vpcs list`.
- `region` if the account isn't in SYD1.

## Bootstrapping the state bucket

The state bucket (`digitalocean_spaces_bucket.tofu_state` in `spaces.tf`) is
also the backend target in `versions.tf`, so on a brand new setup it can't
be created through the same `tofu apply` that uses it as a backend. Create
it once out of band (via the DO console, `doctl`, or a throwaway local
backend), then proceed with `tofu init` against the S3 backend and import it
below so ongoing management goes through tofu.

## Import

Run these once the variables above match reality. Each import must be
followed by `tofu plan` showing **no changes** - if it doesn't, the
resource's arguments in the `.tf` files don't match the live resource, and
must be corrected (not the other way around: do not let tofu recreate
existing infrastructure).

```sh
cd infra
tofu init

tofu import digitalocean_vpc.brdgme <vpc-uuid>
tofu import digitalocean_kubernetes_cluster.brdgme <cluster-uuid>
tofu import digitalocean_domain.brdgme <domain-name>   # e.g. brdg.me

# Only needed for the state bucket bootstrapped out of band above, so its
# ongoing management goes through tofu too. Import ID is "<region>,<name>" -
# verify against the digitalocean_spaces_bucket provider docs, the exact
# argument order has changed between provider versions.
tofu import digitalocean_spaces_bucket.tofu_state syd1,brdgme-tofu-state

tofu plan   # must show no changes
```

## New resources

`digitalocean_spaces_bucket.cnpg_backups` is created by tofu (`tofu apply`)
after the import above shows a clean plan - it is not imported, it doesn't
exist yet.
