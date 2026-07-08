# 28: Abuse Protection (bots, scripted clients, DoS) - Design

> Extracted 2026-07-08 from `docs/plan/28-abuse-protection.md` (superpowers layout
> migration). Content dates from 2026-07-08; this is a point-in-time decision
> record, not a living document.

**Status:** Decided 2026-07-08 - scoped into WP1-WP4 below, ready to
implement. WP1-WP3 are pre-cutover app work; WP4 (Cloudflare) is
post-cutover infra work.

**Problem:** the login endpoint (and every server function) is a plain
HTTP API - anyone can drive it with curl. That is inherent to how the web
works and not a vulnerability by itself, but it means the only thing
standing between the platform and scripted abuse is whatever server-side
protection exists. Today that is thin.

## Current position (audited 2026-07-08)

- **Login**: per-IP rate limit (burst 5, +1/20s) in
  `rust/web/src/auth/rate_limit.rs`, sized to protect the Resend
  free-tier 100/day cap. `login()` also **auto-creates a user row** for
  any unknown email, so a scripted client can grow the `users` /
  `user_emails` tables and burn Resend quota within the rate limit.
- **Confirm code**: per-IP rate limit (burst 10, +1/10s) against
  brute-forcing the 6-digit code space. No per-code attempt cap - per-IP
  limiting alone is the wrong control here (botnets and XFF spoofing
  evade it; a cap on attempts against a single code does not care where
  attempts come from).
- **Client IP trust**: `SmartIpKeyExtractor` prefers `X-Forwarded-For` /
  `X-Real-Ip` / `Forwarded` headers, which nothing in the current chain
  (DO LB -> cilium Gateway -> app) strips or sets authoritatively until
  the PROXY-protocol flip (deferred to #16 beta) lands - so per-IP
  limits are currently **evadable by spoofing XFF**.
  **Update 2026-07-08:** the PROXY-protocol flip was attempted live and
  dropped the same day - DOKS's managed reconciler owns `cilium-config`
  and reverts the flag, so it cannot be set persistently (see D6 below).
  Per-IP app-level limits therefore stay collective/spoofable
  permanently, not just until a flip lands; the design's weight rests on
  D2 (IP-independent app caps) and D1 (Cloudflare edge) as already
  decided.
- **Replica blind spot**: web runs `replicas: 2` and the governor
  limiters are in-process, so every per-IP quota is effectively doubled
  and resets on deploy. Fine for coarse abuse-damping; not fine as the
  mechanism protecting a hard external quota (Resend 100/day).
- **Everything else**: no rate limiting on the other server fns (all
  require a session), no global request/body-size limits beyond axum
  defaults, no bot detection, no CAPTCHA, no WAF/CDN. DO gives L3/4
  DDoS protection on the LB; nothing exists at L7.
- **Exposure is already live**: `beta.brdg.me` points at the cluster,
  so the login endpoint and the (shared) Resend quota are reachable
  today, during the beta period, before the apex cutover.

## Threats, ranked for this platform

1. **Resend quota burn + user-table spam** (live today via beta): the
   cheapest attack with the most annoying blast radius (login emails
   stop for everyone at 100/day).
2. **Confirm-code brute force**: 1M code space, currently only
   IP-limited.
3. **Volumetric / L7 DoS**: low probability for a hobby platform, but
   the only $0 mitigation (Cloudflare free) needs DNS lead time, so it
   is a decision, not an incident response.
4. **Scripted gameplay**: session-gated, low-value target. Explicit
   non-goal until observed.

## Decisions (2026-07-08)

- **D1 - Cloudflare free in front of brdg.me, post-cutover.** Adopt
  Cloudflare's free tier as the edge: proxied (orange-cloud) web
  hostnames, free managed WAF ruleset, Bot Fight Mode, one
  rate-limiting rule (IP-keyed, 10s window - verified free-tier limits
  2026-07-08), unmetered L3-L7 DDoS mitigation, CDN caching for the
  wasm bundle, WebSockets supported. Requires Cloudflare as
  authoritative DNS, i.e. a second nameserver move (DO -> Cloudflare).
  Accepted because the zone is already 100% Tofu-managed (`infra/dns.tf`)
  so the move is a provider swap, not a process change; the Terraform/
  OpenTofu `cloudflare` provider is first-class. Scheduled
  **after the Phase 16 cutover + 1-week gate** so it cannot entangle
  the cutover. This supersedes the "avoid a second NS cutover"
  reasoning in docs/superpowers/specs/2026-07-08-20-external-dns-design.md - that reasoning weighed
  the move against external-dns convenience only; DoS/WAF/CDN is a
  materially better return on the same cost.
- **D2 - stop auto-creating users on login.** Replace the
  `users.login_confirmation` / `login_confirmation_at` columns with a
  `login_confirmations` table keyed by email; the user row is created
  only on successful code confirmation. Kills user-table spam, gives a
  natural place for per-code attempt caps and DB-backed (replica-safe)
  send caps, and net-deletes columns from `users`.
- **D3 - app-level hardening lands pre-cutover** (during beta): the
  endpoint is already exposed and it is pure app code with no Phase 16
  entanglement. **Update 2026-07-08:** promoted further, to pre-go-live
  priority ahead of #16 beta rather than merely "during beta" - see D6.
- **D4 - Turnstile is trigger-based, not now.** Adopt only if abuse is
  observed (login_emails_sent_total spiking, user-table spam). D1 makes
  it a small step later (Turnstile is free and integrates at the CF
  layer we will already have).
- **D5 - no gameplay/server-fn rate limiting for now.** Session-gated,
  low value; revisit only on observed abuse.
- **D6 (2026-07-08) - no real client IPs at the app, ever, on DOKS.** The
  #14/#16 PROXY-protocol flip was attempted live: `enable-gateway-api-proxy-protocol`
  was patched to `"true"` in `kube-system/cilium-config` and the cilium
  DaemonSet restarted successfully, but DOKS's managed addon reconciler
  reverted the ConfigMap back to `"false"` at 13:09:20Z, ~15 minutes
  later - it owns `cilium-config` and the flag cannot be set
  persistently by the cluster operator. The matching DO-LB annotation
  deploy was reverted the same hour (`brdgme` f31be4b, `brdgme-config`
  8333793). Decision (Michael): drop the client-IP/PROXY-protocol work
  entirely - no DO support ticket, no retry. WP1-3 (app-level hardening)
  are promoted to pre-go-live priority as the resulting effective
  protection; WP4 (Cloudflare, which sees real client IPs at the edge)
  stays post-cutover. Implication for WP1: since per-IP limiting is
  permanently a single collective bucket (all clients share the LB SNAT
  address), WP1 should consider loosening the login rate limiter (burst
  5, +1/20s, see "Current position" above) so legitimate concurrent
  users sharing that bucket don't get throttled by each other.

Defense-in-depth logic: the edge (WP4) absorbs volume but is bypassable
(direct-to-LB) and per-IP; the app caps (WP1) are the backstop that
protect the hard quotas regardless of where traffic enters.

## Non-goals

Turnstile/CAPTCHA (trigger-based, D4), gameplay/server-fn
rate limiting (D5), any paid WAF/CDN tier, in-cluster rate-limit
machinery (Envoy/CiliumEnvoyConfig) - the CF free tier + app caps cover
the realistic threat set at $0.
