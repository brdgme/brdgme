# Cloudflare zone for brdg.me (item 28 WP4). The zone was created by hand
# in the CF dashboard 2026-07-10 (free plan) and the registrar NS were cut
# over the same day; tofu ADOPTED it via import blocks rather than
# creating it - see the plan
# docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md.
# Records: 8 legacy/Resend records are DNS-only (proxied = false) so the
# legacy Linode site and Resend mail flow are untouched; beta is proxied
# (orange-cloud) through the CF edge. The apex flips to proxied-new-LB on
# cutover day (#16 runbook, spec W8).
#
# All records use ttl = 1 (Cloudflare "automatic") to match the live zone
# exactly (confirmed via the Task 1 API listing: every one of the 9 live
# records already has ttl = 1, not 3600 - matching live avoids a spurious
# in-place diff on adoption).
resource "cloudflare_zone" "brdgme" {
  name = var.domain_name
  type = "full"
  account = {
    id = var.cloudflare_account_id
  }
}

# Legacy prod records (Linode host) - DNS-only until cutover (item 16).
resource "cloudflare_dns_record" "legacy_apex_a" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "A"
  name    = "brdg.me"
  content = "172.105.164.158"
  proxied = false
  ttl     = 1
}

resource "cloudflare_dns_record" "legacy_mail_a" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "A"
  name    = "mail.brdg.me"
  content = "172.105.164.158"
  proxied = false
  ttl     = 1
}

resource "cloudflare_dns_record" "legacy_apex_spf" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "TXT"
  name    = "brdg.me"
  content = "\"v=spf1 a:mail.brdg.me ip4:172.105.254.59 ip4:194.195.125.83 ip4:194.195.125.116 ~all\""
  proxied = false
  ttl     = 1
}

# Resend records (item 22a) - MUST stay DNS-only (mail).
resource "cloudflare_dns_record" "resend_dkim" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "TXT"
  name    = "resend._domainkey.brdg.me"
  content = "\"p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDDEjCLF0TFjsPbLJbJwRz8GMZK+vRuDcBlD905bPDCPLAlddAG2Sk9ykytATElN1uJGCF0hdeM2kIeSGjZgJtTuFjupwK1AOrhBs3FJKockXmKicXBBhTWGKjhCk95LSHvYYIj/gE6A88dWD0YsBpM3Yikrg6pUU/J1n50Y28v4QIDAQAB\""
  proxied = false
  ttl     = 1
}

resource "cloudflare_dns_record" "resend_send_mx" {
  zone_id  = cloudflare_zone.brdgme.id
  type     = "MX"
  name     = "send.brdg.me"
  content  = "feedback-smtp.us-east-1.amazonses.com"
  priority = 10
  proxied  = false
  ttl      = 1
}

resource "cloudflare_dns_record" "resend_send_spf" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "TXT"
  name    = "send.brdg.me"
  content = "\"v=spf1 include:amazonses.com ~all\""
  proxied = false
  ttl     = 1
}

resource "cloudflare_dns_record" "resend_dmarc" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "TXT"
  name    = "_dmarc.brdg.me"
  content = "\"v=DMARC1; p=none;\""
  proxied = false
  ttl     = 1
}

# Apex receiving MX -> Resend inbound (decision 2026-07-05, see the old
# dns.tf comment: routes ALL @brdg.me inbound mail to Resend; replies to
# legacy play-by-email are dropped until 22b).
resource "cloudflare_dns_record" "resend_inbound_mx" {
  zone_id  = cloudflare_zone.brdgme.id
  type     = "MX"
  name     = "brdg.me"
  content  = "inbound-smtp.us-east-1.amazonaws.com"
  priority = 10
  proxied  = false
  ttl      = 1
}

# Pre-cutover validation subdomain (item 16 beta) - proxied through the
# CF edge (orange-cloud). Proxied records require ttl = 1 (automatic).
resource "cloudflare_dns_record" "beta_a" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "A"
  name    = "beta.brdg.me"
  content = "170.64.251.15"
  proxied = true
  ttl     = 1
}
