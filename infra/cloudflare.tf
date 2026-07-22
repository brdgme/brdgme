# Cloudflare zone for brdg.me (item 28 WP4). The zone was created by hand
# in the CF dashboard 2026-07-10 (free plan) and the registrar NS were cut
# over the same day; tofu ADOPTED it via import blocks rather than
# creating it - see the plan
# docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md.
# Post-cutover (#16): the apex A record points at the DOKS Gateway LB and
# is proxied (orange-cloud) through the CF edge; the Resend mail records
# stay DNS-only. The legacy Linode mail/SPF records and the beta validation
# subdomain were removed at cutover.
#
# All records use ttl = 1 (Cloudflare "automatic").
resource "cloudflare_zone" "brdgme" {
  name = var.domain_name
  type = "full"
  account = {
    id = var.cloudflare_account_id
  }
}

# Apex -> the DOKS Gateway load balancer (the IP beta used during
# validation), proxied (orange-cloud) through the CF edge.
resource "cloudflare_dns_record" "apex_a" {
  zone_id = cloudflare_zone.brdgme.id
  type    = "A"
  name    = "brdg.me"
  content = "170.64.251.15"
  proxied = true
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

# Zone settings (spec W5). SSL "strict" = dashboard "Full (strict)":
# CF connects to the origin over TLS and validates the origin cert
# (cert-manager's Let's Encrypt cert on the Gateway). WebSockets must be
# "on" for /ws through the proxy. Bot Fight Mode is deliberately NOT
# managed here yet - it lands in a later, separately-verified task of
# the 2026-07-10 WP4 plan because the free tier has no BFM exceptions
# and it can break websockets/login.
resource "cloudflare_zone_setting" "ssl" {
  zone_id    = cloudflare_zone.brdgme.id
  setting_id = "ssl"
  value      = "strict"
}

resource "cloudflare_zone_setting" "websockets" {
  zone_id    = cloudflare_zone.brdgme.id
  setting_id = "websockets"
  value      = "on"
}

# The one free-tier rate-limiting rule (spec W5/W6): per-IP, scoped to
# the Leptos server-fn prefix /api/ (fns mount at
# /api/<name><hash>, e.g. /api/login..., /api/confirm_login...).
# Free tier constraints: period and mitigation_timeout are fixed at 10s,
# action "block", and characteristics must include cf.colo.id alongside
# ip.src. 60 req/10s/IP is deliberately generous - server-fn bursts from
# one page load are far below it; a curl flood is far above. Tuned
# during beta verification (plan Task 6) before the in-app limiters are
# deleted (spec W6).
resource "cloudflare_ruleset" "rate_limit" {
  zone_id = cloudflare_zone.brdgme.id
  name    = "brdgme rate limiting"
  kind    = "zone"
  phase   = "http_ratelimit"

  rules = [{
    ref         = "api_per_ip"
    description = "Per-IP limit on Leptos server fns (/api/ prefix)"
    expression  = "(starts_with(http.request.uri.path, \"/api/\"))"
    action      = "block"
    enabled     = true
    ratelimit = {
      characteristics     = ["cf.colo.id", "ip.src"]
      period              = 10
      requests_per_period = 60
      mitigation_timeout  = 10
    }
  }]
}

# Cache rule for hashed /pkg/ assets. Root cause of the post-deploy
# wasm-bindgen LinkError this fixes: Cloudflare's default cached-extension
# list includes .js/.css but not .wasm, so the edge could serve a cached JS
# glue file alongside an evicted-then-refetched, newer wasm. This rule makes
# all of /pkg/ cache-eligible and defers TTL to origin Cache-Control - the
# app serves /pkg/* as `public, max-age=31536000, immutable` (see
# rust/web/src/router.rs's `set_cache_control` middleware). See
# docs/decisions/ASSET_CACHING.md.
#
# ORDERING CONSTRAINT: apply this rule ONLY AFTER the app deploy shipping
# hashed /pkg/ filenames (hash-files in web/Cargo.toml) is live. Applying it
# while /pkg/web.wasm is still a stable, unhashed URL would edge-cache that
# wasm long-term and reproduce the very bug this fixes.
resource "cloudflare_ruleset" "cache_rules" {
  zone_id = cloudflare_zone.brdgme.id
  name    = "brdgme cache rules"
  kind    = "zone"
  phase   = "http_request_cache_settings"

  rules = [{
    ref         = "pkg_immutable_assets"
    description = "Edge-cache hashed /pkg/ assets, respect origin Cache-Control"
    expression  = "(starts_with(http.request.uri.path, \"/pkg/\"))"
    action      = "set_cache_settings"
    enabled     = true
    action_parameters = {
      cache = true
      edge_ttl = {
        mode = "respect_origin"
      }
    }
  }]
}

# Bot Fight Mode (spec W5) - flipped as the LAST edge toggle and
# verified in isolation: the free tier has no BFM exceptions, and the
# documented fallback is fight_mode = false if it breaks websockets or
# login (spec's beta validation checklist).
# JS detections must be on for fight mode (API rejects fight_mode
# without enable_js, found at apply 2026-07-11).
resource "cloudflare_bot_management" "brdgme" {
  zone_id    = cloudflare_zone.brdgme.id
  enable_js  = true
  fight_mode = true
}

# Renamed at cutover (was legacy_apex_a). The moved block keeps the apex A
# record an in-place update rather than a destroy/create.
moved {
  from = cloudflare_dns_record.legacy_apex_a
  to   = cloudflare_dns_record.apex_a
}
