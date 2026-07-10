# 28 WP4: Cloudflare Pre-Go-Live Edge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt the already-live Cloudflare zone for brdg.me into Tofu
(import, not create - the zone exists and the registrar NS are already cut
over), configure the edge (SSL Full strict, WebSockets, one free
rate-limiting rule, Bot Fight Mode as a separately-verified toggle), switch
TLS issuance to DNS01, remove the DO zone, and - once the CF rate-limit
rule is proven on beta - delete the in-app per-IP rate limiting entirely
(spec W6). Investigate origin lockdown as a timeboxed spike.

**Architecture:** Cloudflare fronts beta.brdg.me (proxied A record) with
the 8 legacy/Resend records DNS-only; cert-manager issues via DNS01 using a
zone-scoped CF API token (sealed in brdgme-config); the app drops all
IP-keyed limiting - WP1's DB-backed caps remain the backstop for
direct-to-LB traffic, and WP2's hygiene middleware stays (spec W9).

**Tech Stack:** OpenTofu (cloudflare provider ~> 5, digitalocean ~> 2.49),
cert-manager v1.20.3 (DNS01 cloudflare solver), sealed-secrets +
ArgoCD (brdgme-config), Rust (axum/leptos web crate).

**Spec:** docs/superpowers/specs/2026-07-10-28-wp4-cloudflare-pre-golive-design.md

## Global Constraints

- ASCII-only in all docs and comments written by this plan (no em dashes,
  no smart quotes).
- `tofu` runs from `infra/` with `CLOUDFLARE_API_TOKEN` /
  `CLOUDFLARE_ACCOUNT_ID` sourced from `.env` (devenv's `dotenv.enable`
  exports them on shell entry). The cloudflare provider reads
  `CLOUDFLARE_API_TOKEN` natively; the account ID is a tofu variable with a
  committed default (it is not secret - visible in dashboard URLs).
- Every `tofu apply` step is preceded by a `tofu plan` step whose expected
  summary is stated; **any planned destroy of a cloudflare resource is a
  stop-and-report condition** (Tasks 2-3, 8; Task 5 expects DO-only
  destroys). Applies touch live DNS/edge config and run with Michael's env.
- web crate tests: `SQLX_OFFLINE=true cargo test -p web --features ssr`
  (run from `rust/`).
- `gh run watch` is forbidden in background agents - poll
  `gh run view <run-id> --json status,conclusion` instead. No `kubectl -w`
  watch modes, no unbounded loops; poll with bounded `for` loops + `sleep`.
- Prod deploys flow through ArgoCD watching `brdgme-config` `prod/`, which
  remote-bases this repo's `k8s/prod` at a pinned `?ref=`. Deploying a
  k8s manifest change = merge to master here, then bump the `?ref=` (and
  image tags when the app image changed) in
  `/home/beefsack/Development/brdgme-config/prod/kustomization.yaml`, commit
  and push there; ArgoCD auto-syncs.
- Sealed secrets live in the sibling repo
  `/home/beefsack/Development/brdgme-config` under `sealed-secrets/secrets/`,
  kubeseal pattern per the "Bootstrap order" section (step 3) of
  `docs/superpowers/plans/2026-07-08-15-production-cd-argocd.md`. `kubeseal`
  is in that repo's devenv, and sealing requires the prod kubectl context.
- Task ordering is load-bearing: Task 6 (CF rate-limit rule proven) gates
  Task 7 (in-app limiter deletion); Task 4 verification gates Task 5 (DO
  zone removal); Task 8 (Bot Fight Mode) runs only after the proxy path is
  otherwise proven.
- W8 (cutover-day apex flip to proxied + Gateway apex listeners) is NOT in
  this plan - it stays in the #16 cutover runbook.
- Steps marked **(operator-verify)** need Michael to manually confirm
  (email receipt, playing a game session); everything else is
  agent-executable.

### Known values (discovered 2026-07-10 during planning)

- CF zone ID for brdg.me: `a1efe9aa5ee2d537028b7a0e03794784` (re-derived in
  Task 1 anyway).
- CF account ID: `cada680352b729d5b0c87470b05c55f7`.
- Assigned nameservers: `seth.ns.cloudflare.com`, `sue.ns.cloudflare.com` -
  registrar already cut over (verified via `dig NS`).
- beta.brdg.me is ALREADY proxied: resolves to CF anycast
  (172.67.213.245 / 104.21.23.220) and serves with a `cf-ray` header.
- Leptos server-fn mount prefix: **`/api/`**. Each server fn mounts at
  `/api/<snake_case_name><xxh64-hash>`, e.g.
  `/api/login16822034172302558962` and
  `/api/confirm_login16822034172302558962` (hashes read from the built WASM;
  they change if the fn moves in source, so the CF rule matches the `/api/`
  prefix, not exact paths).
- Planning-time WARNING: with the current `.env` token,
  `GET /zones?name=brdg.me` succeeded but
  `GET /zones/<id>/dns_records` returned `10000 Authentication error` and
  `/user/tokens/verify` returned `1000 Invalid API Token`. The token may
  lack Zone.DNS scope or the `.env` value may be stale. Task 1 re-checks
  and stops if unresolved.

### Task 1: Edge state audit (read-only, no code)

**Files:**
- Test (report only, no repo changes).

**Interfaces:**
- Produces: zone ID, the live DNS record listing (IDs, types, names,
  contents, proxied flags, TTLs) consumed by Task 2's import blocks and
  record resources; confirmation the proxy/WS/email path already works.
- Consumes: `CLOUDFLARE_API_TOKEN` from the devenv shell.

**Steps:**

- [ ] **Step 1: Verify the API token before anything else.**
  ```sh
  curl -s https://api.cloudflare.com/client/v4/user/tokens/verify \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" | jq '.success, .result.status, .errors'
  ```
  Expected: `true`, `"active"`, `[]`. If it returns
  `1000 Invalid API Token` or the Step 5 dns_records listing fails with
  `10000 Authentication error` (both observed at planning time), STOP and
  report: Michael must confirm/re-issue the token with Zone.DNS Edit +
  Zone.Zone Settings Edit + Zone.Zone Read on brdg.me and update `.env`
  (then re-enter the devenv shell so dotenv re-exports it).
- [ ] **Step 2: Confirm NS cutover and beta proxying.**
  ```sh
  dig NS brdg.me +short
  dig A beta.brdg.me +short
  dig A brdg.me +short
  dig A mail.brdg.me +short
  ```
  Expected: NS lines end in `.ns.cloudflare.com.` (currently `seth`/`sue`);
  beta resolves to CF anycast IPs (currently 172.67.213.245 /
  104.21.23.220), NOT 170.64.251.15; apex and mail still resolve to
  172.105.164.158 (legacy Linode, DNS-only). Any deviation on apex/mail is
  a stop-and-report condition (legacy site must stay untouched).
- [ ] **Step 3: Confirm the proxy serves beta.**
  ```sh
  curl -sI https://beta.brdg.me | grep -iE "^(HTTP|cf-ray|server)"
  ```
  Expected shape: `HTTP/2 200`, `server: cloudflare`, a `cf-ray: ...-SYD`
  line.
- [ ] **Step 4: Get the zone ID.**
  ```sh
  curl -s "https://api.cloudflare.com/client/v4/zones?name=brdg.me" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    | jq -r '.result[0] | .id, .status'
  ```
  Expected: `a1efe9aa5ee2d537028b7a0e03794784` and `active`.
- [ ] **Step 5: List every DNS record with IDs (Task 2 input).**
  ```sh
  ZONE_ID=a1efe9aa5ee2d537028b7a0e03794784
  curl -s "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/dns_records?per_page=100" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    | jq -r '.result[] | [.id, .type, .name, .content, (.proxied|tostring), (.ttl|tostring), (.priority|tostring)] | @tsv' \
    | column -t -s "$(printf '\t')"
  ```
  Output shape: one row per record,
  `<32-hex-id>  A  beta.brdg.me  170.64.251.15  true  1  null`.
  Record the full table in the task report. Compare against `infra/dns.tf`'s
  9 records: apex A, mail A, apex SPF TXT, resend._domainkey TXT, send MX
  (prio 10), send SPF TXT, _dmarc TXT, apex inbound MX (prio 10), beta A.
  Note explicitly, per record: proxied flag vs the spec's W4 intent (the 8
  legacy/Resend records DNS-only i.e. `proxied=false`, beta `proxied=true`)
  - CF's zone-creation copy may have guessed proxied status - plus any
  extra or missing records vs dns.tf. Extra records are a stop-and-report
  condition (Michael decides adopt-vs-delete); proxied-flag drift is NOT a
  stop - Task 2 reconciles it.
- [ ] **Step 6: Verify a websocket connects through the proxy right now.**
  ```sh
  curl -si --max-time 15 --http1.1 \
    -H "Connection: Upgrade" -H "Upgrade: websocket" \
    -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
    -H "Sec-WebSocket-Version: 13" \
    https://beta.brdg.me/ws | head -3
  ```
  Expected: first line `HTTP/1.1 101 Switching Protocols` (curl then holds
  the socket until `--max-time` expires - exit code 28 after printing the
  101 is success). If it returns a non-101 HTTP status, report it -
  WebSockets may need the Task 3 zone setting before this passes; re-run
  after Task 3 in that case.
- [ ] **Step 7 (operator-verify): Login email still delivers.** Michael:
  request a login code on https://beta.brdg.me and confirm the email
  arrives (Resend DNS records intact through the CF zone copy). Record
  pass/fail in the task report.

### Task 2: Tofu cloudflare provider + zone/record adoption

**Files:**
- Modify: `infra/versions.tf` (add cloudflare to required_providers)
- Modify: `infra/provider.tf` (cloudflare provider block)
- Modify: `infra/variables.tf` (add `cloudflare_account_id`)
- Create: `infra/cloudflare.tf` (zone + 9 dns records)
- Create: `infra/imports.tf` (import blocks; deleted at end of this task)

**Interfaces:**
- Consumes: record IDs + live contents/TTLs/proxied flags from Task 1
  Step 5; `CLOUDFLARE_API_TOKEN` env (provider auth).
- Produces: `cloudflare_zone.brdgme` (its `.id` is referenced by Tasks 3
  and 8) and one `cloudflare_dns_record` per record, in state, with a clean
  plan. DNS ownership in Tofu moves to Cloudflare; Task 5 removes the DO
  side.

**Steps:**

- [ ] **Step 1: Add the provider requirement.** In `infra/versions.tf`,
  inside the existing `required_providers` block after `digitalocean`, add:
  ```hcl
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5"
    }
  ```
  (v5 is current - 5.21.x as of 2026-07-10. v5 renamed `cloudflare_record`
  to `cloudflare_dns_record`, moved zone settings to
  `cloudflare_zone_setting`, and `cloudflare_zone` takes an `account`
  attribute object - the HCL below is written against v5; do not use v4
  shapes.)
- [ ] **Step 2: Provider + account variable.** Append to
  `infra/provider.tf`:
  ```hcl
  # Auth via the native CLOUDFLARE_API_TOKEN env var (set in .env,
  # exported by devenv's dotenv integration on shell entry) - never a tofu
  # variable, never committed. Scoped to the brdg.me zone: Zone.DNS Edit +
  # Zone.Zone Settings Edit + Zone.Zone Read (spec W2).
  provider "cloudflare" {}
  ```
  Append to `infra/variables.tf`:
  ```hcl
  variable "cloudflare_account_id" {
    description = "Cloudflare account ID owning the brdg.me zone. Not secret (it appears in dashboard URLs); also in .env as CLOUDFLARE_ACCOUNT_ID for direct API calls outside tofu."
    type        = string
    default     = "cada680352b729d5b0c87470b05c55f7"
  }
  ```
- [ ] **Step 3: Write `infra/cloudflare.tf`.** Full content (reconcile the
  `content`/`ttl` values below against Task 1 Step 5's listing before
  `tofu plan` - the live API values are authoritative for content/TTL; the
  `proxied` flags below are authoritative per spec W4 and override whatever
  CF guessed at zone creation. v5 returns TXT `content` wrapped in double
  quotes - if Task 1's listing shows quoted content, quote-escape it below
  to match, e.g. `"\"v=spf1 ...\""`):
  ```hcl
  # Cloudflare zone for brdg.me (item 28 WP4). The zone was created by hand
  # in the CF dashboard 2026-07-10 (free plan) and the registrar NS were cut
  # over the same day; tofu ADOPTED it via import blocks rather than
  # creating it - see the plan
  # docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md.
  # Records: 8 legacy/Resend records are DNS-only (proxied = false) so the
  # legacy Linode site and Resend mail flow are untouched; beta is proxied
  # (orange-cloud) through the CF edge. The apex flips to proxied-new-LB on
  # cutover day (#16 runbook, spec W8).
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
    ttl     = 3600
  }

  resource "cloudflare_dns_record" "legacy_mail_a" {
    zone_id = cloudflare_zone.brdgme.id
    type    = "A"
    name    = "mail.brdg.me"
    content = "172.105.164.158"
    proxied = false
    ttl     = 3600
  }

  resource "cloudflare_dns_record" "legacy_apex_spf" {
    zone_id = cloudflare_zone.brdgme.id
    type    = "TXT"
    name    = "brdg.me"
    content = "\"v=spf1 a:mail.brdg.me ip4:172.105.254.59 ip4:194.195.125.83 ip4:194.195.125.116 ~all\""
    proxied = false
    ttl     = 3600
  }

  # Resend records (item 22a) - MUST stay DNS-only (mail).
  resource "cloudflare_dns_record" "resend_dkim" {
    zone_id = cloudflare_zone.brdgme.id
    type    = "TXT"
    name    = "resend._domainkey.brdg.me"
    content = "\"p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDDEjCLF0TFjsPbLJbJwRz8GMZK+vRuDcBlD905bPDCPLAlddAG2Sk9ykytATElN1uJGCF0hdeM2kIeSGjZgJtTuFjupwK1AOrhBs3FJKockXmKicXBBhTWGKjhCk95LSHvYYIj/gE6A88dWD0YsBpM3Yikrg6pUU/J1n50Y28v4QIDAQAB\""
    proxied = false
    ttl     = 3600
  }

  resource "cloudflare_dns_record" "resend_send_mx" {
    zone_id  = cloudflare_zone.brdgme.id
    type     = "MX"
    name     = "send.brdg.me"
    content  = "feedback-smtp.us-east-1.amazonses.com"
    priority = 10
    proxied  = false
    ttl      = 3600
  }

  resource "cloudflare_dns_record" "resend_send_spf" {
    zone_id = cloudflare_zone.brdgme.id
    type    = "TXT"
    name    = "send.brdg.me"
    content = "\"v=spf1 include:amazonses.com ~all\""
    proxied = false
    ttl     = 3600
  }

  resource "cloudflare_dns_record" "resend_dmarc" {
    zone_id = cloudflare_zone.brdgme.id
    type    = "TXT"
    name    = "_dmarc.brdg.me"
    content = "\"v=DMARC1; p=none;\""
    proxied = false
    ttl     = 3600
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
    ttl      = 3600
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
  ```
- [ ] **Step 4: Write `infra/imports.tf`** using Task 1 Step 5's record IDs
  (dns_record import ID format is `<zone_id>/<record_id>`; zone import ID
  is the bare zone ID):
  ```hcl
  # One-shot import blocks adopting the hand-created CF zone + records into
  # state (plan 2026-07-10-28-wp4-cloudflare-pre-golive.md Task 2). DELETE
  # this file once `tofu plan` is clean after the import apply.
  import {
    to = cloudflare_zone.brdgme
    id = "a1efe9aa5ee2d537028b7a0e03794784"
  }

  import {
    to = cloudflare_dns_record.legacy_apex_a
    id = "a1efe9aa5ee2d537028b7a0e03794784/<record id of the apex A row from Task 1 Step 5>"
  }
  ```
  ...and one `import` block per remaining resource
  (`legacy_mail_a`, `legacy_apex_spf`, `resend_dkim`, `resend_send_mx`,
  `resend_send_spf`, `resend_dmarc`, `resend_inbound_mx`, `beta_a`), each
  `id = "a1efe9aa5ee2d537028b7a0e03794784/<matching record id>"`. Match
  rows to resources by type+name+content from the Task 1 table - every
  one of the 9 resources MUST have an import block (a missing one shows up
  as a create in the next step).
- [ ] **Step 5: Init and plan.**
  ```sh
  cd /home/beefsack/Development/brdgme/infra
  tofu init
  tofu plan
  ```
  `tofu init` expected output includes
  `Installing cloudflare/cloudflare v5.x...` and
  `OpenTofu has been successfully initialized!`.
  `tofu plan` expected summary: `Plan: 10 to import, 0 to add, N to change,
  0 to destroy.` where the N in-place changes are only proxied/TTL/content
  reconciliations on imported records (e.g. flipping a wrongly-proxied
  Resend record to `proxied = false`). **Any destroy, any create, or any
  change touching the DO cluster/VPC/buckets is a stop-and-report
  condition.** If a TXT record shows a content diff that is only
  quoting, fix the HCL to match the API-returned form and re-plan.
- [ ] **Step 6: Apply and confirm clean.**
  ```sh
  tofu apply -auto-approve
  tofu plan
  ```
  Apply expected: `Apply complete! Resources: 10 imported, 0 added,
  N changed, 0 destroyed.` The follow-up `tofu plan` MUST print
  `No changes. Your infrastructure matches the configuration.` If the
  apply changed any proxied flag, re-run Task 1 Steps 2-3 (beta still
  proxied, apex/mail still 172.105.164.158).
- [ ] **Step 7: Delete `infra/imports.tf`** (import blocks are one-shot;
  state now owns the resources), then `tofu plan` again expecting
  `No changes.`
- [ ] **Step 8: Commit.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/versions.tf infra/provider.tf infra/variables.tf infra/cloudflare.tf
  git commit -m "Infra #28 WP4: adopt Cloudflare zone + records into tofu (import, no create)

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```

### Task 3: CF settings via Tofu (SSL strict, WebSockets, rate-limit rule)

**Files:**
- Modify: `infra/cloudflare.tf` (append zone settings + ruleset)

**Interfaces:**
- Consumes: `cloudflare_zone.brdgme.id` from Task 2.
- Produces: SSL mode Full (strict), WebSockets on, and the single free-tier
  rate-limiting rule on the `/api/` server-fn prefix - the rule Task 6
  verifies and Task 7's deletion depends on. Bot Fight Mode is explicitly
  NOT set here: per spec W5 it is a separately-verified toggle, deferred to
  Task 8 (free tier has no BFM exceptions, so it must be flipped and
  verified in isolation after everything else is proven).

**Steps:**

- [ ] **Step 1: Append settings + ruleset to `infra/cloudflare.tf`:**
  ```hcl
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
  ```
  (If the API rejects the ratelimit constants - e.g. a 400 naming `period`
  or `mitigation_timeout` - the free-tier allowed values changed; verify
  the current allowed values against the provider/API error message at
  execution and adjust ONLY those two numbers. This is the plan's single
  permitted deferral.)
- [ ] **Step 2: Plan.**
  ```sh
  cd /home/beefsack/Development/brdgme/infra
  tofu plan
  ```
  Expected: `Plan: 3 to add, 0 to change, 0 to destroy.` (two
  `cloudflare_zone_setting`, one `cloudflare_ruleset`). Zone settings
  pre-exist server-side with defaults, but `cloudflare_zone_setting` is a
  PATCH-style resource - "add" here only asserts the value, it does not
  create anything destructive. Any destroy: stop and report.
- [ ] **Step 3: Apply, verify, re-plan.**
  ```sh
  tofu apply -auto-approve
  tofu plan
  ```
  Expected: `Apply complete! Resources: 3 added, 0 changed, 0 destroyed.`
  then `No changes.` Verify live:
  ```sh
  ZONE_ID=a1efe9aa5ee2d537028b7a0e03794784
  curl -s "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/settings/ssl" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" | jq -r '.result.value'
  curl -s "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/settings/websockets" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" | jq -r '.result.value'
  ```
  Expected: `strict` then `on`. Then re-run Task 1 Step 3 (beta still 200
  through the proxy - `strict` requires a valid origin cert, which
  beta-brdg-me-tls already is) and Task 1 Step 6 (websocket 101). A 52x
  from CF after enabling `strict` means origin TLS validation failed:
  stop, set `value = "full"` temporarily, apply, and report.
- [ ] **Step 4: Commit.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/cloudflare.tf
  git commit -m "Infra #28 WP4: CF SSL strict + websockets + /api/ rate-limit rule via tofu

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```

### Task 4: TLS DNS01 switch (sealed CF token + ClusterIssuer)

**Files:**
- Create (in `/home/beefsack/Development/brdgme-config`):
  `sealed-secrets/secrets/cloudflare-api-token.yaml`
- Modify (brdgme-config): `sealed-secrets/secrets/kustomization.yaml`
- Modify (this repo): `k8s/base/cert-manager/cluster-issuer.yaml`
- Modify (brdgme-config): `prod/kustomization.yaml` (`?ref=` bump to deploy)

**Interfaces:**
- Consumes: `CLOUDFLARE_API_TOKEN` (the same single token, spec W2); the
  prod kubectl context + kubeseal (brdgme-config devenv); ArgoCD sync.
- Produces: cert-manager issues/renews via DNS01 through Cloudflare. Gates
  Task 5 (DO zone removal must not happen until issuance no longer depends
  on anything but the CF zone).

The ClusterIssuer is cluster-scoped, so cert-manager reads
`apiTokenSecretRef` from its cluster-resource namespace - the `cert-manager`
namespace (where the controller runs; see
`brdgme-config/cert-manager/kustomization.yaml`, `namespace: cert-manager`;
the default `--cluster-resource-namespace` is the controller's own
namespace and is not overridden in the deployment args).

**Steps:**

- [ ] **Step 1: Seal the token into brdgme-config.** From a
  brdgme-config devenv shell with the prod kubectl context active and
  `CLOUDFLARE_API_TOKEN` exported (source the brdgme repo's `.env` if
  needed):
  ```sh
  cd /home/beefsack/Development/brdgme-config
  kubectl create secret generic cloudflare-api-token \
    -n cert-manager \
    --from-literal=api-token="$CLOUDFLARE_API_TOKEN" \
    --dry-run=client -o yaml \
    | kubeseal --format yaml > sealed-secrets/secrets/cloudflare-api-token.yaml
  ```
  Output shape: a `kind: SealedSecret` YAML with
  `metadata.name: cloudflare-api-token`, `metadata.namespace: cert-manager`,
  and `spec.encryptedData.api-token: Ag...` (long base64). kubeseal fetches
  the controller's cert from the live cluster, so this must run against
  prod.
- [ ] **Step 2: Register it in the kustomization.** In
  `brdgme-config/sealed-secrets/secrets/kustomization.yaml`, add
  `- cloudflare-api-token.yaml` to `resources:` (alphabetical position,
  after `bot-config.yaml`).
- [ ] **Step 3: Commit + push brdgme-config; poll the sync.**
  ```sh
  cd /home/beefsack/Development/brdgme-config
  git add sealed-secrets/secrets/cloudflare-api-token.yaml sealed-secrets/secrets/kustomization.yaml
  git commit -m "Seal cloudflare-api-token for cert-manager DNS01 (brdgme #28 WP4)"
  git push
  for i in $(seq 1 30); do
    kubectl -n argocd get application brdgme \
      -o jsonpath='{.status.sync.status} {.status.health.status}{"\n"}'
    kubectl -n cert-manager get secret cloudflare-api-token >/dev/null 2>&1 && break
    sleep 10
  done
  kubectl -n cert-manager get secret cloudflare-api-token
  ```
  Expected final output: `Synced Healthy` and the secret listed with
  `TYPE Opaque, DATA 1`. (ArgoCD's app targets the brdgme namespace but the
  SealedSecret carries its own explicit `cert-manager` namespace; the
  default AppProject allows all destinations. If the sync reports a
  namespace permission error, report it - do not widen the project
  yourself.)
- [ ] **Step 4: Rewrite the ClusterIssuer solver.** Replace the full
  contents of `k8s/base/cert-manager/cluster-issuer.yaml` (this repo) with:
  ```yaml
  ---
  # ClusterIssuer for Let's Encrypt production certificates.
  # DNS01 via Cloudflare (item 28 WP4, spec W3): HTTP01 through the CF
  # proxy is fragile (challenge caching, Always-Use-HTTPS interference),
  # and DNS01 also derisks cutover-day apex issuance. The token Secret
  # lives in the cert-manager namespace (cert-manager's cluster-resource
  # namespace for ClusterIssuer secret refs) as a SealedSecret in
  # brdgme-config (sealed-secrets/secrets/cloudflare-api-token.yaml),
  # scoped to the brdg.me zone (Zone.DNS Edit + Zone Settings Edit +
  # Zone Read).
  #
  # Prerequisite (one-time, not in kustomize - cluster infrastructure):
  # cert-manager installed with the Gateway API feature enabled
  # (--enable-gateway-api) - still required, because the Gateway
  # annotation cert-manager.io/cluster-issuer on
  # k8s/base/gateway/gateway.yaml is what creates the Certificate
  # resources; only the ACME challenge solver moved off the Gateway.
  #
  # Rollback: revert this file to the http01 gatewayHTTPRoute solver
  # (git history) - but note HTTP01 through the Cloudflare proxy is
  # unreliable, so a real rollback also means grey-clouding beta
  # (proxied = false on cloudflare_dns_record.beta_a in infra/) while
  # HTTP01 challenges run.
  apiVersion: cert-manager.io/v1
  kind: ClusterIssuer
  metadata:
    name: letsencrypt
  spec:
    acme:
      server: https://acme-v02.api.letsencrypt.org/directory
      email: admin@brdg.me
      privateKeySecretRef:
        name: letsencrypt-account-key
      solvers:
      - dns01:
          cloudflare:
            apiTokenSecretRef:
              name: cloudflare-api-token
              key: api-token
  ```
- [ ] **Step 5: Commit (this repo) and deploy via ArgoCD ref bump.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add k8s/base/cert-manager/cluster-issuer.yaml
  git commit -m "k8s #28 WP4: switch letsencrypt ClusterIssuer from HTTP01 to Cloudflare DNS01

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  git push
  NEW_SHA=$(git rev-parse master)
  cd /home/beefsack/Development/brdgme-config
  sed -i "s|?ref=[0-9a-f]*|?ref=${NEW_SHA}|" prod/kustomization.yaml
  git add prod/kustomization.yaml
  git commit -m "deploy: brdgme ${NEW_SHA:0:7} (ClusterIssuer DNS01)"
  git push
  ```
  Note: the ref bump deploys everything merged to `k8s/prod` since the
  previous pinned sha, per the GitOps contract. Image tags are left
  unchanged (no app image change in this task). Poll the sync as in
  Step 3's bounded loop until `Synced Healthy`.
- [ ] **Step 6: Verify the issuer is Ready.**
  ```sh
  kubectl get clusterissuer letsencrypt \
    -o jsonpath='{.status.conditions[?(@.type=="Ready")].status} {.status.conditions[?(@.type=="Ready")].message}{"\n"}'
  ```
  Expected: `True The ACME account was registered with the ACME server`.
- [ ] **Step 7: Force a renewal through DNS01 and watch it go Ready.** The
  beta cert is created by the Gateway annotation
  (`cert-manager.io/cluster-issuer: letsencrypt` on Gateway `brdgme` in
  namespace `brdgme`), so deleting its Secret makes cert-manager re-issue
  through the new solver:
  ```sh
  kubectl -n brdgme get secret beta-brdg-me-tls \
    -o jsonpath='{.data.tls\.crt}' | base64 -d | openssl x509 -noout -dates
  kubectl -n brdgme delete secret beta-brdg-me-tls
  for i in $(seq 1 60); do
    READY=$(kubectl -n brdgme get certificate beta-brdg-me-tls \
      -o jsonpath='{.status.conditions[?(@.type=="Ready")].status}' 2>/dev/null)
    echo "attempt $i: Ready=$READY"
    [ "$READY" = "True" ] && break
    sleep 10
  done
  kubectl -n brdgme get secret beta-brdg-me-tls \
    -o jsonpath='{.data.tls\.crt}' | base64 -d | openssl x509 -noout -dates
  ```
  Expected: the loop reaches `Ready=True` (DNS01 typically 1-5 min;
  propagation checks dominate) and the second `notBefore` is today's date,
  strictly newer than the first printout. If it stays `False` for the full
  10 minutes: `kubectl -n brdgme describe certificate beta-brdg-me-tls`
  and `kubectl -n brdgme get challenges` for the failing challenge, then
  stop and report (rollback per the comment in Step 4's YAML).
  Finally confirm the site serves the fresh cert through the proxy - note
  CF terminates the public edge cert, so verify at the origin-facing
  level via cert-manager state, and that beta still serves:
  ```sh
  curl -sI https://beta.brdg.me | head -1
  ```
  Expected: `HTTP/2 200`.

### Task 5: Remove the DO zone from Tofu (gate: Task 4 verified)

**Files:**
- Delete: `infra/dns.tf`

**Interfaces:**
- Consumes: Task 4's successful DNS01 renewal (proof nothing depends on the
  DO zone) and Task 2's clean CF adoption.
- Produces: single-owner DNS (Cloudflare, in Tofu). `var.domain_name` stays
  (now consumed by `cloudflare_zone.brdgme`).

**Steps:**

- [ ] **Step 1: Confirm the gate.** Task 4 Step 7 passed (fresh notBefore
  via DNS01). Do not proceed otherwise.
- [ ] **Step 2: Delete `infra/dns.tf`** (the whole file:
  `digitalocean_domain.brdgme` + all 9 `digitalocean_record` resources -
  the NS no longer point at DO, so these serve nothing).
- [ ] **Step 3: Plan - DO-only destroys.**
  ```sh
  cd /home/beefsack/Development/brdgme/infra
  tofu plan
  ```
  Expected: `Plan: 0 to add, 0 to change, 10 to destroy.` and every
  destroyed address starts with `digitalocean_domain.` or
  `digitalocean_record.`. **Zero cloudflare_* changes** - any cloudflare
  change or non-DNS DO destroy is a stop-and-report condition.
- [ ] **Step 4: Apply and verify resolution is unaffected.**
  ```sh
  tofu apply -auto-approve
  tofu plan
  dig A beta.brdg.me +short
  dig A brdg.me +short
  dig TXT resend._domainkey.brdg.me +short | head -c 40
  ```
  Expected: `Apply complete! Resources: 0 added, 0 changed, 10 destroyed.`,
  `No changes.`, beta still CF anycast, apex still 172.105.164.158, DKIM
  TXT still returns `"p=MIGfMA0GCSqGSIb3...` (Cloudflare is authoritative;
  the DO zone was dead weight).
- [ ] **Step 5: Commit.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/dns.tf
  git commit -m "Infra #28 WP4: remove dead DO zone/records (NS on Cloudflare, DNS01 proven)

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```

### Task 6: CF rate-limit rule verification (GATE for Task 7)

**Files:**
- Modify (only if tuning needed): `infra/cloudflare.tf`
  (`requests_per_period`)
- Test: live against https://beta.brdg.me

**Interfaces:**
- Consumes: Task 3's ruleset.
- Produces: a proven edge rate limit - the explicit precondition spec W6
  sets for deleting the in-app limiters (Task 7 MUST NOT start until this
  task's report says the rule trips under flood and never under normal
  use). Final constants recorded in the checklist at the bottom of this
  plan.

**Steps:**

- [ ] **Step 1: Trip the rule with a parallel curl flood.** 100 requests,
  10 in flight, all inside one 10s window:
  ```sh
  seq 1 100 | xargs -P 10 -I{} curl -s -o /dev/null -w "%{http_code}\n" \
    "https://beta.brdg.me/api/rl_probe" | sort | uniq -c
  ```
  Expected shape (counts approximate):
  ```
       60 200
       40 429
  ```
  (Non-429 lines may be another status depending on how the Leptos
  fallback answers an unknown /api path - what matters is roughly the
  first 60 pass and the rest are 429.) Then confirm a mitigated response
  comes from Cloudflare:
  ```sh
  seq 1 70 | xargs -P 10 -I{} curl -s -o /dev/null "https://beta.brdg.me/api/rl_probe"
  curl -si "https://beta.brdg.me/api/rl_probe" | grep -iE "^(HTTP|cf-ray|server)"
  ```
  Expected: `HTTP/2 429`, `server: cloudflare`, a `cf-ray:` line. Wait
  ~15s afterwards (mitigation_timeout 10s) and confirm a single request is
  no longer blocked.
- [ ] **Step 2 (operator-verify): Normal use never trips it.** Michael:
  log in on beta, browse the game list, and play a real game session
  (moves + websocket updates) for a few minutes. No 429s / "blocked"
  interstitials may appear. Report pass/fail.
- [ ] **Step 3 (conditional): Tune.** Only if Step 2 tripped: raise
  `requests_per_period` in `infra/cloudflare.tf` (60 -> 100; server-fn
  calls are user-action-driven, so a legitimate trip means the number is
  simply too low), then `tofu plan` (expect `1 to change`),
  `tofu apply -auto-approve`, re-run Steps 1-2, and commit:
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/cloudflare.tf
  git commit -m "Infra #28 WP4: tune CF rate-limit threshold after beta verification

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```
- [ ] **Step 4: Record the verified constants** in this plan's "Verified
  edge constants" checklist section (bottom of this file) and in the task
  report: rule path prefix `/api/`, final requests_per_period, period 10s,
  mitigation_timeout 10s, action block. This is the gate artifact for
  Task 7.

### Task 7: Delete in-app per-IP rate limiting (spec W6; gated on Task 6)

**Files:**
- Delete: `rust/web/src/auth/rate_limit.rs`
- Modify: `rust/web/src/auth/mod.rs`
- Modify: `rust/web/src/auth/server.rs`
- Modify: `rust/web/src/state.rs`
- Modify: `rust/web/src/router.rs`
- Modify: `rust/web/src/main.rs`
- Modify: `rust/web/Cargo.toml`
- Modify: `rust/web/tests/ssr_pages.rs`
- Modify: `rust/web/tests/websocket_hygiene.rs`

**Interfaces:**
- Consumes: Task 6's gate (CF rule proven).
- Produces: an app with NO IP-keyed limiting. Keeps untouched (spec W9):
  WP1's DB-backed caps in `auth/server.rs` (per-email cooldown/cap, global
  Resend cap, per-code attempt cap) and WP2's hygiene middleware in
  `router.rs` (`RequestBodyLimitLayer` 256 KiB + `TimeoutLayer` 30s). This
  also supersedes the old WP4 step 6 CF-Connecting-IP carve-out: nothing
  in-app keys on IP anymore, so no carve-out is needed.

TDD inversion: this task deletes behaviour, so first prove the suite green
and identify which tests exist only to exercise the limiter, then delete
code + those tests together, then prove green again.

**Steps:**

- [ ] **Step 1: Baseline test run.**
  ```sh
  cd /home/beefsack/Development/brdgme/rust
  SQLX_OFFLINE=true cargo test -p web --features ssr
  ```
  Expected: all green. The limiter-only tests that will be deleted are all
  inside `rust/web/src/auth/rate_limit.rs`'s `mod tests` (7 tests:
  `allows_up_to_burst_size_then_rejects`,
  `rate_limits_are_tracked_independently_per_ip`,
  `spoofed_forwarding_headers_do_not_select_the_key`,
  `extracts_ip_from_peer_addr`, `returns_none_when_nothing_to_extract_from`,
  `returns_none_when_only_spoofed_headers_present_and_no_peer_addr`,
  `confirm_limiter_allows_up_to_burst_size_then_rejects`) - they vanish
  with the file. No test elsewhere asserts limiter behaviour (verified by
  grep 2026-07-10); the other files below only construct/provide the
  limiters.
- [ ] **Step 2: Delete the module.**
  ```sh
  git rm rust/web/src/auth/rate_limit.rs
  ```
  In `rust/web/src/auth/mod.rs`, delete the line `pub mod rate_limit;`.
- [ ] **Step 3: `rust/web/src/state.rs`.** Delete line 1
  (`use crate::auth::rate_limit::{ConfirmRateLimiter, LoginRateLimiter};`),
  the now-unused `use std::sync::Arc;`, and the two fields:
  ```rust
      pub login_rate_limiter: Arc<LoginRateLimiter>,
      pub confirm_rate_limiter: Arc<ConfirmRateLimiter>,
  ```
- [ ] **Step 4: `rust/web/src/auth/server.rs`.** Four removals:
  1. The whole `login_client_ip` helper (the `#[cfg(feature = "ssr")]
     async fn login_client_ip()` block, currently lines 96-104) - it was
     the only consumer of `extract_client_ip`.
  2. In `login()`: the limiter lookup + check (currently lines 115-127) -
     from `let login_rate_limiter =` through the closing `}` of the
     `if let Some(ip) = login_client_ip().await ...` block returning the
     "Too many login attempts" response.
  3. In `confirm_login()`: the equivalent block (currently lines 252-264),
     from `let confirm_rate_limiter =` through the `}` returning the
     "Too many attempts" error.
  4. In the `#[cfg(test)] mod tests` helper `with_pool_context`: the two
     lines
     `provide_context(crate::auth::rate_limit::build_login_rate_limiter());`
     and
     `provide_context(crate::auth::rate_limit::build_confirm_rate_limiter());`.
  Also reword the comment near the global-cap query (currently line ~193)
  that says the DB caps are replica-safe "unlike the in-process governor
  above" - the governor is gone; say instead: unlike edge/per-process
  limits, these caps live in Postgres so they hold across replicas and
  deploys (the per-IP edge limit is Cloudflare's, see the 2026-07-10 WP4
  spec W6).
- [ ] **Step 5: `rust/web/src/router.rs`.** In `build_router`: delete the
  two clones (`let login_rate_limiter = state.login_rate_limiter.clone();`,
  `let confirm_rate_limiter = state.confirm_rate_limiter.clone();`) and the
  two `provide_context(...)` lines for them. Rewrite the WP2 hygiene
  comment above `RequestBodyLimitLayer` (currently lines 110-122, which
  opens with "Global HTTP hygiene, not abuse-proofing: `tower_governor`'s
  login/confirm limiters...") to drop the governor mention, e.g.:
  ```rust
      // Global HTTP hygiene, not abuse-proofing (kept deliberately, spec
      // W9 of the 2026-07-10 #28 WP4 design): these two layers stop a
      // stray oversized POST or a wedged handler from tying up a
      // worker/connection, and still cover direct-to-LB traffic that
      // bypasses Cloudflare. Hard abuse quotas are the WP1 DB-backed send
      // caps (`login()`'s cooldown/per-email/global caps in
      // `auth/server.rs`) - replica-safe and restart-proof because they
      // live in Postgres; per-IP rate limiting happens at the Cloudflare
      // edge, not in-app. Added after `/healthz` (like `TraceLayer`
      // below) so both apply to it too, which is harmless since the probe
      // is bodyless and returns immediately. Placed before `TraceLayer`
      // so it stays the outermost layer and still records a span (with
      // e.g. a 413/408 status) for requests these reject.
  ```
- [ ] **Step 6: `rust/web/src/main.rs`.** Delete the two builder lines
  (`let login_rate_limiter = web::auth::rate_limit::build_login_rate_limiter();`
  and the confirm equivalent) and the two `AppState` fields
  (`login_rate_limiter: login_rate_limiter.clone(),`,
  `confirm_rate_limiter: confirm_rate_limiter.clone(),`).
- [ ] **Step 7: `rust/web/Cargo.toml`.** Delete the dependency block:
  ```toml
  # Login rate limiting
  tower_governor = { version = "0.8", optional = true }
  governor = { version = "0.10", optional = true }
  ```
  and the two feature entries `"dep:tower_governor",` and
  `"dep:governor",` from the `ssr` feature list.
- [ ] **Step 8: Integration test state.** In `rust/web/tests/ssr_pages.rs`
  and `rust/web/tests/websocket_hygiene.rs`, delete the two `AppState`
  fields in each:
  ```rust
          login_rate_limiter: web::auth::rate_limit::build_login_rate_limiter(),
          confirm_rate_limiter: web::auth::rate_limit::build_confirm_rate_limiter(),
  ```
- [ ] **Step 9: Full verification.**
  ```sh
  cd /home/beefsack/Development/brdgme/rust
  SQLX_OFFLINE=true cargo test -p web --features ssr
  SQLX_OFFLINE=true cargo clippy -p web --all-features
  grep -rn "governor\|rate_limit\|extract_client_ip" web/src web/tests web/Cargo.toml
  ```
  Expected: tests green (7 fewer tests than Step 1's run), clippy clean
  (no warnings), and the grep returns nothing (checked in `web/` only -
  the string may legitimately appear in docs elsewhere).
- [ ] **Step 10: Commit and deploy to beta.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add -A rust/web
  git commit -m "web #28 WP4: delete in-app per-IP rate limiting (spec W6, CF edge rule proven)

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  git push
  ```
  Then poll CI for the image build (no `gh run watch`):
  ```sh
  RUN_ID=$(gh run list --branch master --limit 1 --json databaseId -q '.[0].databaseId')
  for i in $(seq 1 60); do
    gh run view "$RUN_ID" --json status,conclusion -q '.status + " " + (.conclusion // "-")'
    gh run view "$RUN_ID" --json status -q .status | grep -q completed && break
    sleep 30
  done
  ```
  Expected final line: `completed success`. Then deploy the new image:
  in `/home/beefsack/Development/brdgme-config/prod/kustomization.yaml`,
  bump `?ref=` to the new master sha and set the `web` (and `migrate`)
  image `newTag:` to the CI-produced tag (`sha-<short7>` per the existing
  entries), commit `deploy: brdgme <short7> (delete in-app rate limiting)`,
  push, and poll the ArgoCD app (Task 4 Step 3 loop) until
  `Synced Healthy`.
- [ ] **Step 11 (operator-verify): Login flow on beta post-deploy.**
  Michael: request a code, receive the email, confirm, session works.

### Task 8: Bot Fight Mode toggle + verification (spec W5)

**Files:**
- Modify: `infra/cloudflare.tf`

**Interfaces:**
- Consumes: everything previously proven (proxy, WS, login, rate limit) -
  BFM is flipped last so any breakage is unambiguously attributable.
- Produces: BFM on (or a documented decision to leave it off - the spec's
  own fallback).

**Steps:**

- [ ] **Step 1: Append to `infra/cloudflare.tf`:**
  ```hcl
  # Bot Fight Mode (spec W5) - flipped as the LAST edge toggle and
  # verified in isolation: the free tier has no BFM exceptions, and the
  # documented fallback is fight_mode = false if it breaks websockets or
  # login (spec's beta validation checklist).
  resource "cloudflare_bot_management" "brdgme" {
    zone_id    = cloudflare_zone.brdgme.id
    fight_mode = true
  }
  ```
  Bot management is a per-zone singleton that already exists server-side;
  if `tofu apply` fails with a conflict/"already exists" error, adopt it
  instead: add `import { to = cloudflare_bot_management.brdgme,
  id = "a1efe9aa5ee2d537028b7a0e03794784" }` to a temporary
  `infra/imports.tf`, apply, delete the import file, re-plan clean (same
  one-shot pattern as Task 2).
- [ ] **Step 2: Plan + apply.**
  ```sh
  cd /home/beefsack/Development/brdgme/infra
  tofu plan
  tofu apply -auto-approve
  tofu plan
  ```
  Expected: `Plan: 1 to add, 0 to change, 0 to destroy.`, apply succeeds,
  then `No changes.`
- [ ] **Step 3: Re-verify the websocket through the proxy.** Run Task 1
  Step 6's curl (expect the `101`). Then the >30s idle survival:
  **(operator-verify)** Michael plays a game session on beta and leaves
  the tab idle for over a minute - live updates must still arrive
  afterwards (WP2's 30s app-side pings keep it alive; BFM must not kill
  the upgraded connection).
- [ ] **Step 4 (operator-verify): Login flow with BFM on.** Request code,
  receive email, confirm - BFM must not challenge/block the login
  server-fn POSTs.
- [ ] **Step 5 (conditional fallback): If Step 3 or 4 breaks,** set
  `fight_mode = false` in `infra/cloudflare.tf`, run
  `tofu plan` (expect `1 to change`), `tofu apply -auto-approve`, re-verify
  the broken flow now works, and record the outcome (BFM off + why) in the
  checklist below and the commit message.
- [ ] **Step 6: Commit.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/cloudflare.tf
  git commit -m "Infra #28 WP4: Bot Fight Mode toggle (verified against WS + login on beta)

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```

### Task 9: Origin lockdown investigation (spike, timeboxed 1 hour; spec W7)

**Files:**
- Modify: `k8s/base/gateway/gateway.yaml` (annotation attempt; kept only on
  success)
- Modify: `docs/superpowers/plans/2026-07-10-28-wp4-cloudflare-pre-golive.md`
  (checklist outcome note, either way)

**Interfaces:**
- Consumes: the proven proxy path (Tasks 1-8).
- Produces: either a locked-down LB (only CF ranges reach the origin) or a
  documented accepted direct-to-LB bypass (WP1 DB caps backstop). Success
  or failure, the outcome is committed.

Background: `loadBalancerSourceRanges` is a Service spec field, and the
cilium Gateway only exposes `spec.infrastructure.annotations`/`labels` to
the generated Service - so the only Gateway-compatible attempt is the DO
CCM annotation `service.beta.kubernetes.io/do-loadbalancer-allow-rules`
(comma-separated `cidr:<range>` entries; ignored if
`loadBalancerSourceRanges` were somehow set, which it is not here). It has
reported reliability issues - hence a timeboxed spike, not a committed
feature.

**Steps:**

- [ ] **Step 1: Build the allow-rules value from Cloudflare's published
  ranges (fetch, do not hardcode - the list changes):**
  ```sh
  ALLOW=$(curl -s https://www.cloudflare.com/ips-v4 https://www.cloudflare.com/ips-v6 \
    | sed 's/^/cidr:/' | paste -sd, -)
  echo "$ALLOW"
  ```
  Output shape: `cidr:173.245.48.0/20,cidr:103.21.244.0/22,...,cidr:2400:cb00::/32,...`
  (~15 v4 + ~7 v6 entries, one line).
- [ ] **Step 2: Add the annotation.** In `k8s/base/gateway/gateway.yaml`
  under `spec.infrastructure.annotations`, alongside the existing
  idle-timeout annotation, add (pasting Step 1's actual output as the
  value):
  ```yaml
        # Spike (28 WP4 W7): restrict the DO LB to Cloudflare's published
        # ranges so direct-to-origin traffic can't bypass the edge.
        # Value generated from https://www.cloudflare.com/ips-v4 + ips-v6.
        service.beta.kubernetes.io/do-loadbalancer-allow-rules: "cidr:173.245.48.0/20,...paste full Step 1 output..."
  ```
- [ ] **Step 3: Deploy and test.** Commit on a branch or directly to master
  (small, revertible), push, bump the brdgme-config `?ref=` (Task 4 Step 5
  pattern, commit message `deploy: brdgme <short7> (LB allow-rules spike)`),
  poll to `Synced Healthy`, give the DO CCM ~2 minutes to reconcile the LB,
  then:
  ```sh
  LB_IP=$(kubectl -n brdgme get gateway brdgme -o jsonpath='{.status.addresses[0].value}')
  echo "$LB_IP"   # expect 170.64.251.15
  curl -sk --connect-timeout 10 --resolve "beta.brdg.me:443:${LB_IP}" \
    "https://beta.brdg.me/healthz" ; echo "direct exit: $?"
  curl -s -o /dev/null -w "%{http_code}\n" https://beta.brdg.me/healthz
  ```
  Success = the direct curl times out (`direct exit: 28`) while the
  proxied curl prints `200`. Also re-check the websocket through the proxy
  (Task 1 Step 6).
- [ ] **Step 4a (success): Keep it.** Amend the gateway.yaml comment to
  record it works and that the range list needs occasional manual refresh
  from cloudflare.com/ips; update this plan's checklist section
  ("origin lockdown: ENABLED"); commit:
  `k8s #28 WP4: restrict DO LB to Cloudflare ranges (allow-rules spike succeeded)`.
- [ ] **Step 4b (failure - annotation ignored, LB flaps, or proxied traffic
  breaks): Revert.** `git revert` the Step 3 commit (and re-bump the
  brdgme-config ref to the revert sha), confirm beta serves again, then
  document the accepted bypass in BOTH places: a comment in
  `k8s/base/gateway/gateway.yaml` (next to `infrastructure.annotations`:
  allow-rules attempted 2026-07-10 and failed <symptom>; direct-to-LB
  bypass accepted, WP1 DB caps are the backstop, per spec W7) and this
  plan's checklist ("origin lockdown: REJECTED - accepted bypass").
  Commit the doc note:
  `docs #28 WP4: record origin-lockdown spike outcome (accepted direct-to-LB bypass)`.
  Timebox: if the spike exceeds 1 hour of wall-clock investigation, take
  the 4b path.

### Task 10: Docs (infra README + external-dns spec note)

**Files:**
- Modify: `infra/README.md`
- Modify: `docs/superpowers/specs/2026-07-08-20-external-dns-design.md`

**Interfaces:**
- Consumes: outcomes of Tasks 2-5.
- Produces: the migration recorded where the Route53 one is; the
  external-dns spec's stale cross-reference updated.

**Steps:**

- [ ] **Step 1: `infra/README.md`.** Two edits. (a) In the Prerequisites
  section, append a bullet:
  ```markdown
  - A Cloudflare API token scoped to the brdg.me zone (Zone.DNS Edit +
    Zone.Zone Settings Edit + Zone.Zone Read), exported as
    `CLOUDFLARE_API_TOKEN` (the cloudflare provider reads it natively; see
    `.env.example`). The account ID has a committed default in
    `variables.tf`.
  ```
  (b) After the "DNS migration from Route53" section, add:
  ```markdown
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
  ```
  Also update the intro paragraph's "the `brdg.me` DNS zone (plus legacy
  records until cutover)" to "the `brdg.me` DNS zone (on Cloudflare since
  2026-07-10, plus legacy records until cutover)".
- [ ] **Step 2: external-dns spec note.**
  `docs/superpowers/specs/2026-07-08-20-external-dns-design.md` already
  carries a 2026-07-08 update note about the Cloudflare move (its "no
  second NS cutover" rationale is already recorded as superseded) - do NOT
  add a duplicate. Amend that existing parenthetical (the one reading
  "(Update 2026-07-08: item 28 decided to move the zone to Cloudflare
  anyway, post-cutover, ...)"): change "anyway, post-cutover, for the free
  WAF/DoS/CDN edge" to "anyway - resequenced 2026-07-10 to pre-go-live,
  single-stage - for the free WAF/DoS/CDN edge", and change the trailing
  cross-reference "See docs/superpowers/plans/2026-07-08-28-abuse-protection.md
  WP4." to "See docs/superpowers/specs/2026-07-10-28-wp4-cloudflare-pre-golive-design.md."
- [ ] **Step 3: Commit.**
  ```sh
  cd /home/beefsack/Development/brdgme
  git add infra/README.md docs/superpowers/specs/2026-07-08-20-external-dns-design.md
  git commit -m "Docs #28 WP4: record Cloudflare DNS migration in infra README + external-dns spec

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
  ```

## Verified edge constants (filled in during execution)

Task 6/8/9 record their outcomes here so the final state is readable
without digging through task reports:

- [ ] Rate-limit rule: path prefix `/api/`, requests_per_period = ___
  (proposed 60), period 10s, mitigation_timeout 10s, action block,
  characteristics ip.src (+ cf.colo.id, API-required).
- [ ] Bot Fight Mode: on / off because ___.
- [ ] Origin lockdown: ENABLED via do-loadbalancer-allow-rules / REJECTED
  (accepted direct-to-LB bypass, WP1 DB caps backstop) because ___.

## Spec decision -> task map (review aid)

- W1 single-stage migration: already live (post-approval state); Tasks 1-5
  adopt and finish it.
- W2 single scoped token: Task 2 (provider env) + Task 4 (sealed for
  cert-manager).
- W3 DNS01: Task 4.
- W4 record port (8 DNS-only + beta proxied), prompt DO removal: Tasks 1
  (audit), 2 (adoption + proxied reconciliation), 5 (DO removal).
- W5 CF config via Tofu (SSL strict, WS on, rate-limit rule, BFM as
  separate toggle): Tasks 3 + 8.
- W6 delete in-app per-IP limiting after the CF rule is proven: Tasks 6
  (proof gate) + 7 (deletion).
- W7 origin lockdown investigation: Task 9.
- W8 cutover-day delta: out of scope here, stays in the #16 runbook
  (noted in Global Constraints).
- W9 keep WP2 hygiene middleware: Task 7 explicitly preserves it and
  rewrites its comment.
- Post-approval state (zone live, NS cut over, adoption-not-creation,
  proxied-flag audit, early validation): Tasks 1-2 embody it.

