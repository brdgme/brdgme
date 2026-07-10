# 28 WP4: Cloudflare edge, resequenced pre-go-live - Design

> Decided 2026-07-10 (Michael). Supersedes the scheduling and sequencing
> of WP4 in `2026-07-08-28-abuse-protection-design.md` D1 (which stays as
> a point-in-time record); the D1 decision to adopt Cloudflare free tier
> itself is unchanged. Cross-reference the plan at
> `docs/superpowers/plans/2026-07-08-28-abuse-protection.md` (WP4 section
> to be reworked to match this spec).

## Context / rationale

- WP4 promoted from post-cutover to pre-go-live 2026-07-10: CF
  proxy/websocket/rate-limit behaviour is far easier to validate against
  beta.brdg.me during beta than after going live.
- WP3 (peer-only rate-limit keying, commit 6e53681) is the prerequisite
  for the CF-Connecting-IP carve-out.
- Michael's simplifying call 2026-07-10: the current brdg.me
  (Linode-hosted legacy site) is not actively used right now, so the
  migration does not need zero-downtime ceremony around the legacy
  records - keep it simple, single-stage.
- #32 (Alloy OTLP) demoted to post-go-live 2026-07-10 (Grafana Cloud
  quota needs to reset anyway); WP4 is now the sole remaining pre-go-live
  item before #16 beta.

## Decisions

- **W1 - Single-stage migration.** beta.brdg.me goes proxied
  (orange-cloud) from the moment the CF zone goes authoritative; TLS
  DNS01 switch and CF security config land in the same window. Rejected
  alternative: a two-stage grey-cloud-first flip with a week of
  dual-zone fallback - unnecessary given the legacy site is idle.
- **W2 - Single Cloudflare API token**, scoped to the brdg.me zone
  (Zone.DNS Edit + Zone.Zone Settings Edit + Zone.Zone Read). Used by
  Tofu via `CLOUDFLARE_API_TOKEN` env (never committed) and sealed for
  cert-manager as a SealedSecret in `brdgme-config` (kubeseal pattern
  identical to email-config, see plan
  `docs/superpowers/specs/2026-07-08-15-production-cd-argocd-design.md`
  lines 158-177). Rejected: split tokens per consumer - over-engineering
  for this platform.
- **W3 - TLS via DNS01.** ClusterIssuer `letsencrypt`
  (`k8s/base/cert-manager/cluster-issuer.yaml`) switches from the HTTP01
  gatewayHTTPRoute solver to DNS01 with the CF token. HTTP01 through the
  CF proxy is fragile (challenge caching, Always-Use-HTTPS interference)
  and DNS01 also derisks cutover-day apex issuance. Ordering note: DNS01
  can only validate once the CF zone is authoritative, so the issuer
  change deploys before the NS flip and the first renewal is verified
  after it; the existing beta cert stays valid across the gap.
- **W4 - DNS record port.** All 9 records from `infra/dns.tf` move to
  `cloudflare_record` resources: legacy apex A / mail A / apex SPF TXT,
  Resend DKIM TXT / send MX / send SPF TXT / `_dmarc` TXT / apex inbound
  MX (all DNS-only), beta A (proxied). DO zone resources removed
  promptly once the flip is verified (no fallback week).
- **W5 - CF config via Tofu.** SSL mode Full (strict); WebSockets on;
  Bot Fight Mode on but as a separately-verified toggle (free tier has
  no BFM exceptions; documented fallback is turning it off if it breaks
  websockets or login); the one free rate-limiting rule, path verified
  against the actual Leptos server-fn prefix at implementation time,
  tuned so normal SSR/server-fn bursts never trip it.
- **W6 - App carve-out (after edge verified).** `extract_client_ip` in
  `rust/web/src/auth/rate_limit.rs` prefers `CF-Connecting-IP`, dead
  `headers` param stripped, login/confirm limiter constants re-tightened
  per-IP (undoing the shared-SNAT-bucket loosening of 5a7bb85, per the
  existing D6 comment).
- **W7 - Origin lockdown investigation**, unchanged from the old WP4
  step 4 (`loadBalancerSourceRanges` vs DO LB annotations via the
  cilium Gateway `spec.infrastructure`), timed after the proxy is
  proven; if neither works cleanly, accept direct-to-LB bypass risk
  (WP1 DB caps are the backstop).
- **W8 - Cutover-day delta** stays in the #16 runbook: apex flips from
  DNS-only-Linode to proxied-new-LB, Gateway apex listeners added, DNS01
  already proven by then.

## Human vs agent split (runbook order)

1. Human: add brdg.me zone to the existing CF account (free plan),
   create the scoped API token, record the assigned nameservers.
2. Agent: all Tofu changes (provider + zone + records + settings),
   ClusterIssuer DNS01 change, sealed CF token in `brdgme-config`; `tofu
   apply` populates the CF zone (inert until the NS flip).
3. Gate: Michael's explicit sign-off.
4. Human: registrar NS flip to Cloudflare's nameservers.
5. Validation checklist (below), then remove DO zone resources.

## Beta validation checklist

- NS propagation (`dig NS brdg.me`), records resolve.
- beta.brdg.me serves with a `cf-ray` response header.
- Websocket connects through the proxy and survives >30s idle; play a
  real game session.
- Login email delivers (Resend DNS intact - beta logins depend on it).
- Sanity check apex/mail still resolve to 172.105.164.158.
- Forced DNS01 renewal succeeds (e.g. `cmctl renew` / delete the cert
  secret).
- Rate-limiting rule trips under a curl loop, never under normal
  browsing.
- Bot Fight Mode verified against websockets + login; toggle off if it
  interferes.

## Non-goals

Unchanged from the 2026-07-08 spec: Turnstile trigger-based only, no
gameplay rate limiting, no paid tier.
