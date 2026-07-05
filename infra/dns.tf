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

# Resend records (item 22a), values from the Resend dashboard 2026-07-05.
# Sending uses the send.brdg.me subdomain (MX + SPF) plus DKIM and DMARC.
resource "digitalocean_record" "resend_dkim" {
  domain = digitalocean_domain.brdgme.id
  type   = "TXT"
  name   = "resend._domainkey"
  value  = "p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDDEjCLF0TFjsPbLJbJwRz8GMZK+vRuDcBlD905bPDCPLAlddAG2Sk9ykytATElN1uJGCF0hdeM2kIeSGjZgJtTuFjupwK1AOrhBs3FJKockXmKicXBBhTWGKjhCk95LSHvYYIj/gE6A88dWD0YsBpM3Yikrg6pUU/J1n50Y28v4QIDAQAB"
  ttl    = 3600
}

resource "digitalocean_record" "resend_send_mx" {
  domain   = digitalocean_domain.brdgme.id
  type     = "MX"
  name     = "send"
  value    = "feedback-smtp.us-east-1.amazonses.com."
  priority = 10
  ttl      = 3600
}

resource "digitalocean_record" "resend_send_spf" {
  domain = digitalocean_domain.brdgme.id
  type   = "TXT"
  name   = "send"
  value  = "v=spf1 include:amazonses.com ~all"
  ttl    = 3600
}

resource "digitalocean_record" "resend_dmarc" {
  domain = digitalocean_domain.brdgme.id
  type   = "TXT"
  name   = "_dmarc"
  value  = "v=DMARC1; p=none;"
  ttl    = 3600
}

# Apex receiving MX -> Resend inbound. Decision 2026-07-05: added despite
# the 22b plan putting inbound on play.brdg.me - this routes ALL inbound
# mail for @brdg.me to Resend, which supersedes the legacy Linode server's
# A-record-fallback mail receipt (legacy play-by-email replies stop
# working; no webhook exists until 22b, so replies are dropped).
resource "digitalocean_record" "resend_inbound_mx" {
  domain   = digitalocean_domain.brdgme.id
  type     = "MX"
  name     = "@"
  value    = "inbound-smtp.us-east-1.amazonaws.com."
  priority = 10
  ttl      = 3600
}
