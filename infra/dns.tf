# The zone belongs to tofu; records within it will belong to external-dns
# (Phase 20) once it runs. Until cutover, the legacy records below keep the
# current Linode-hosted prod working when the nameservers move from Route53
# to DO - see README.md "DNS migration from Route53".
resource "digitalocean_domain" "brdgme" {
  name = var.domain_name
}

# Legacy prod records, copied from the Route53 zone (discovered via DNS
# queries 2026-07-05 - verify against Route53 itself before the NS switch,
# queries cannot enumerate a zone). Remove at decommission (item 16).
resource "digitalocean_record" "legacy_apex_a" {
  domain = digitalocean_domain.brdgme.id
  type   = "A"
  name   = "@"
  value  = "172.105.164.158"
  ttl    = 3600
}

resource "digitalocean_record" "legacy_mail_a" {
  domain = digitalocean_domain.brdgme.id
  type   = "A"
  name   = "mail"
  value  = "172.105.164.158"
  ttl    = 3600
}

resource "digitalocean_record" "legacy_apex_spf" {
  domain = digitalocean_domain.brdgme.id
  type   = "TXT"
  name   = "@"
  value  = "v=spf1 a:mail.brdg.me ip4:172.105.254.59 ip4:194.195.125.83 ip4:194.195.125.116 ~all"
  ttl    = 3600
}
