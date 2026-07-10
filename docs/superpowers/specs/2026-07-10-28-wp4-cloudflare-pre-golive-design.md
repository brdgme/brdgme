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
- WP3 (peer-only rate-limit keying, commit 6e53681) proved forwarding
  headers are ignored at the app, documenting the trust model that the
  W6 limiter deletion relies on.
- Michael's simplifying call 2026-07-10: the current brdg.me
  (Linode-hosted legacy site) is not actively used right now, so the
  migration does not need zero-downtime ceremony around the legacy
  records - keep it simple, single-stage.
- #32 (Alloy OTLP) demoted to post-go-live 2026-07-10 (Grafana Cloud
  quota needs to reset anyway); WP4 is now the sole remaining pre-go-live
  item before #16 beta.
- Explicit goal: this pivot is also about using Cloudflare to remove
  custom security complexity from the app wherever CF now provides the
  equivalent, not just adding an edge in front of it. Rate limiting is
  the primary case.

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
  identical to email-config, see the "Bootstrap order" section, step 3,
  of plan `docs/superpowers/plans/2026-07-08-15-production-cd-argocd.md`).
  Rejected: split tokens per consumer - over-engineering for this
  platform.
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
- **W6 - Delete in-app per-IP rate limiting instead of extending it.**
  Once the CF rate-limiting rule (real client IPs at the edge) is
  verified on beta, remove the tower_governor login/confirm limiters
  from the app entirely: `rust/web/src/auth/rate_limit.rs`, the
  governor dependency, `extract_client_ip` and its tests, and the
  previously planned CF-Connecting-IP carve-out (now unnecessary -
  nothing in-app keys on IP anymore). Rationale: per D6 the in-app
  limiter was already one collective SNAT bucket (coarse damping only);
  WP1's DB-backed caps (per-email cooldown/cap, global Resend cap,
  per-code attempt cap) are IP-independent and remain the backstop for
  direct-to-LB traffic. WP3's peer-only keying work is not wasted - it
  proved forwarding headers are ignored, which documented the trust
  model this deletion relies on. This supersedes the "re-tighten per-IP
  in WP4" intent recorded in the D6 comment in `rate_limit.rs` and in
  commit 5a7bb85's loosening rationale - the limiter is deleted, not
  re-tightened.
- **W7 - Origin lockdown investigation**, unchanged from the old WP4
  step 4 (`loadBalancerSourceRanges` vs DO LB annotations via the
  cilium Gateway `spec.infrastructure`), timed after the proxy is
  proven; if neither works cleanly, accept direct-to-LB bypass risk
  (WP1 DB caps are the backstop).
- **W8 - Cutover-day delta** stays in the #16 runbook: apex flips from
  DNS-only-Linode to proxied-new-LB, Gateway apex listeners added, DNS01
  already proven by then.
- **W9 - Keep WP2's hygiene middleware (body limit 256 KiB + 30s
  timeout).** Explicitly considered for removal and kept: Cloudflare
  free tier's own limits are far looser (100 MB request bodies, ~100s
  proxy timeout), the middleware still protects the direct-to-LB path,
  and it is one small layer in `build_router` with near-zero complexity,
  unlike the limiter machinery. Overridable at spec review.

## Human vs agent split (runbook order)

1. Human: add brdg.me zone to the existing CF account (free plan),
   create the scoped API token, record the assigned nameservers.
2. Agent: all Tofu changes (provider + zone + records + settings),
   ClusterIssuer DNS01 change, sealed CF token in `brdgme-config`; `tofu
   apply` populates the CF zone (inert until the NS flip).
3. Gate: Michael's explicit sign-off.
4. Human: registrar NS flip to Cloudflare's nameservers.
5. Validation checklist (below), then remove DO zone resources.

## Post-approval state (2026-07-10)

Approved by Michael 2026-07-10, with the following facts superseding the
runbook order above:

- The CF zone for brdg.me already exists; the scoped API token is
  created. Local env has `CLOUDFLARE_API_TOKEN` and
  `CLOUDFLARE_ACCOUNT_ID` set in `.env` (gitignored); both documented in
  `.env.example`.
- All existing DO records were copied to the CF zone at zone creation,
  and the registrar nameservers are already cut over to Cloudflare.
  Runbook steps 1, 3 and 4 are therefore done; the NS-flip sign-off gate
  is moot.
- Consequence for W4: the Tofu work is now adoption, not creation - the
  cloudflare provider gets added and the existing zone + records are
  imported into state (import blocks or `tofu import`), then reconciled
  so `tofu plan` is clean. The proxied/DNS-only status of each imported
  record must be audited against W4's intent (8 records DNS-only, beta
  proxied) - CF's zone-creation import may have guessed proxied status.
- W9 (keep WP2 hygiene middleware) confirmed by Michael, conditional on:
  the code stays tiny (it is - one layer stack in `build_router`), no
  maintenance burden, and no websocket interference (already proven by
  WP2's live >30s WS survival test).
- Validation checklist items that can run immediately (NS propagation,
  cf-ray on beta, WS through proxy, login email) should be exercised
  early in implementation since the proxy may already be live.

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
  browsing. Must be verified before the W6 in-app limiter deletion
  lands (CF rule proven first, then delete).
- Bot Fight Mode verified against websockets + login; toggle off if it
  interferes.

## Non-goals

Unchanged from the 2026-07-08 spec: Turnstile trigger-based only, no
gameplay rate limiting, no paid tier.
