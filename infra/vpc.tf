# Existing resource - to be imported, not created. Values must match the
# live VPC exactly before `tofu import`; see infra/README.md.
resource "digitalocean_vpc" "brdgme" {
  name     = var.vpc_name
  region   = var.region
  ip_range = "10.10.0.0/16" # placeholder - confirm actual CIDR with `doctl vpcs list` before import
}
