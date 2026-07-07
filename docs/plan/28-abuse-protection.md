# 28: Abuse Protection (bots, scripted clients, DoS)

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
  reasoning in docs/plan/20-external-dns.md - that reasoning weighed
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
  entanglement.
- **D4 - Turnstile is trigger-based, not now.** Adopt only if abuse is
  observed (login_emails_sent_total spiking, user-table spam). D1 makes
  it a small step later (Turnstile is free and integrates at the CF
  layer we will already have).
- **D5 - no gameplay/server-fn rate limiting for now.** Session-gated,
  low value; revisit only on observed abuse.

Defense-in-depth logic: the edge (WP4) absorbs volume but is bypassable
(direct-to-LB) and per-IP; the app caps (WP1) are the backstop that
protect the hard quotas regardless of where traffic enters.

---

## WP1: login flow rework + email send caps (pre-cutover, app)

**Files to read first:** `rust/web/src/auth/server.rs`,
`rust/web/src/auth/rate_limit.rs`, `rust/web/src/models/user.rs`,
`rust/web/migrations/` (latest schema), `rust/web/src/router.rs`
(context provisioning).

**Migration** (`005_login_confirmations.sql`):

- New table `login_confirmations`:
  - `email text PRIMARY KEY` (one active confirmation per email;
    `login()` upserts),
  - `code char(6) NOT NULL`,
  - `created_at timestamptz NOT NULL DEFAULT now()` (code validity
    window: 1 hour, unchanged),
  - `attempts int NOT NULL DEFAULT 0` (failed confirm attempts against
    this code),
  - `sent_count int NOT NULL DEFAULT 0`, `last_sent_at timestamptz`
    (per-email + global send caps).
- Drop `users.login_confirmation` and `users.login_confirmation_at`.
- Rows are short-lived operational state; delete opportunistically
  (expired rows on upsert, the row itself on successful confirm). No
  cron/job needed.

**`login()` changes:**

1. Validate email as today.
2. Keep the per-IP governor check (coarse damping; see WP3 for the IP
   it keys on).
3. **No user creation.** Look up `login_confirmations` by email:
   - **Cooldown:** if `last_sent_at` < 60s ago, return the same
     generic success response *without sending* (idempotent resend
     shield; response must be indistinguishable so it is not an
     enumeration/behaviour oracle).
   - **Per-email cap:** if `sent_count` >= 5 within the validity
     window, return generic success without sending.
   - **Global cap (protects the Resend 100/day quota, replica-safe):**
     `SELECT coalesce(sum(sent_count),0) FROM login_confirmations
     WHERE last_sent_at > now() - interval '24 hours'` - if >= 50,
     refuse with an honest "logins are temporarily limited, try again
     later" message (this one affects legit users, so do not pretend
     success) and increment a `login_email_cap_hit_total` counter.
4. Upsert the row (fresh code if expired, else re-send the existing
   code), bump `sent_count`/`last_sent_at`, send the email.

**`confirm_login()` changes:**

1. Keep the per-IP governor check.
2. Look up `login_confirmations` by email. Missing row, expired row, or
   `attempts >= 10` => the existing generic "Invalid or expired token"
   error. On code mismatch, increment `attempts` (this is the real
   brute-force control: 10 attempts per code, independent of source
   IP) and return the same generic error.
3. On match, in one transaction: create the `users` + `user_emails`
   rows **if the email is unknown** (moved here from `login()`),
   create the auth token, delete the `login_confirmations` row, set
   the session. Username derivation (localpart) as today.

**Tests:** update `login_creates_user_and_sets_confirmation_token` (user
must NOT exist until confirm), add: unknown-email login creates no user
row; confirm creates the user exactly once (and repeat-confirm fails);
attempts cap invalidates the code at 10; cooldown suppresses a second
send within 60s with identical response; per-email and global caps
suppress sends; existing confirm tests (wrong token, expired, right code
wrong email) reworked onto the new table. Per docs/CODING.md, `auth/`
changes must land with tests.

**Acceptance:** `cargo test` in `rust/web` passes; `cargo leptos build`
succeeds; `cargo clippy --all-features` clean; manual dev-flow check
(login -> code in logs -> confirm -> session works; second login within
60s does not print a second code).

## WP2: global HTTP hygiene middleware (pre-cutover, app)

Small, one-place change in `rust/web/src/router.rs::build_router` (plus
`main.rs` if layer ordering demands): add `tower_http`
`RequestBodyLimitLayer` (256 KB - server-fn payloads are small forms)
and `TimeoutLayer` (30s) around the app router. Order them so `/healthz`
and the `/ws` upgrade are unaffected (WS upgrade completes fast, but
verify the timeout layer only bounds the upgrade handshake, not the
long-lived connection - if it kills live sockets, scope it to exclude
`/ws`). Document in a comment that the governor limiters are per-pod
(replicas: 2) and per-deploy, and that hard quotas are protected by the
WP1 DB caps instead.

**Acceptance:** WP1's gate plus: an oversized POST gets 413; a live
websocket survives > 30s idle-with-pings in dev.

## WP3: client-IP trust fix (pre-cutover, app; pairs with the #16 flip)

Replace the `SmartIpKeyExtractor` header-preference in
`rust/web/src/auth/rate_limit.rs::extract_client_ip` with: **use the
socket peer address only; ignore client-supplied forwarding headers**
(add a `cf-connecting-ip` carve-out only in WP4, gated on the peer
being our locked-down edge). Consequences, in order:

- Today (pre-PROXY-flip): peer is the LB/node SNAT address, so per-IP
  collapses to one collective bucket. That is already the effective
  state for honest clients and is strictly better than spoofable
  (fails closed-ish; WP1's caps carry the real protection).
- After the cilium PROXY-protocol flip (#16 beta runbook): peer is the
  real client IP and per-IP limiting becomes trustworthy with no code
  change.
- Dev (Tilt/kind): peer is the real client already; tests that set XFF
  headers get updated to set `ConnectInfo` instead.

**Acceptance:** rate_limit tests updated (spoofed XFF must NOT select
the key); WP1 gate.

## WP4: Cloudflare edge (post-cutover + 1-week gate, infra/Tofu)

**Pre-req:** Phase 16 complete, break-glass window closed, legacy
hostnames (`legacy`/`api`/`ws` per Phase 17/18 state) understood - only
port records that still need to exist.

1. **Tofu provider swap** in `infra/`: add the `cloudflare` provider,
   `cloudflare_zone` for brdg.me, port every live record from `dns.tf`
   (Resend DKIM/SPF/DMARC/MX and `send`/`_dmarc` records stay
   **DNS-only**; apex/`www`-equivalents and any surviving app
   hostnames become **proxied**). Keep the DO zone in place until the
   NS move is verified, then remove `digitalocean_domain` +
   `digitalocean_record` resources.
2. **Registrar NS flip** to Cloudflare's assigned nameservers (manual
   registrar step, like the Phase 21 flip; document in
   `infra/README.md`).
3. **TLS:** switch the cert-manager `ClusterIssuer` from HTTP01 to
   DNS01 with a scoped Cloudflare API token (sealed-secret). Set the
   CF zone SSL mode to **Full (strict)** (Tofu `cloudflare_zone_setting`).
4. **Origin lockdown (best-effort):** restrict the DO LB to
   Cloudflare's published IP ranges. The `do-loadbalancer-allow-rules`
   annotation is deprecated (and has reported reliability issues -
   verify behaviour when applied); `loadBalancerSourceRanges` is the
   supported path but the Gateway-created Service only takes
   annotations via `spec.infrastructure`. Investigate at implementation
   time; if neither works cleanly, accept direct-to-LB bypass risk -
   WP1 caps are the backstop - and note it here.
5. **CF security config via Tofu:** Bot Fight Mode on; free managed
   WAF ruleset (on by default); the one free rate-limiting rule on
   `/api/*` (e.g. >30 req/10s per IP => block 10s - tune so real SSR
   + server-fn bursts never trip it).
6. **App client-IP carve-out:** extend WP3's extractor to prefer
   `CF-Connecting-IP` (safe because either the LB only accepts CF
   ranges, or - without lockdown - only when the PROXY-protocol peer
   is within CF ranges).
7. Update `docs/plan/20-external-dns.md` note: its "no second NS
   cutover" rationale is superseded by this decision; Cloudflare also
   restores an in-tree external-dns provider should that ever be
   wanted.

**Acceptance:** site serves through CF (response headers show
`cf-ray`); websockets work through the proxy; login emails still
deliver (Resend DNS untouched); certs renew via DNS01; `tofu plan`
clean; rate-limit rule verified with a curl loop; real client IPs
appear in the app (rate-limit keys + logs).

## Observability tie-ins (fold into WP1, verify in Phase 18)

- `login_emails_sent_total` already exists and Phase 18 specs a Resend
  quota alert on it - unchanged.
- WP1 adds `login_email_cap_hit_total` (global cap refusals) and a
  counter for confirm-attempt-cap hits; alert on either being nonzero
  (it means someone is probing).

**Non-goals:** Turnstile/CAPTCHA (trigger-based, D4), gameplay/server-fn
rate limiting (D5), any paid WAF/CDN tier, in-cluster rate-limit
machinery (Envoy/CiliumEnvoyConfig) - the CF free tier + app caps cover
the realistic threat set at $0.
