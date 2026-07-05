resource "digitalocean_vpc" "brdgme" {
  name     = var.vpc_name
  region   = var.region
  ip_range = "10.10.0.0/16"
}
