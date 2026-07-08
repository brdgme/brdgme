# 28: Abuse Protection (bots, scripted clients, DoS) - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/28-abuse-protection.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Goal:** Harden brdgme's login/confirm endpoints and the edge against
scripted abuse via WP1-WP4: login flow rework + email send caps (app,
pre-cutover), global HTTP hygiene middleware (app, pre-cutover),
client-IP trust fix (app, pre-cutover), and a Cloudflare edge
(infra/Tofu, post-cutover + 1-week gate).

**Spec:** `docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md`

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

> **Update 2026-07-08:** the #16 Cilium PROXY-protocol flip referenced
> throughout this section was attempted live and dropped the same day -
> DOKS's managed reconciler owns `cilium-config` and reverts the flag, so
> it can never be set persistently. The "after the flip" bullet below will
> not happen; the "today (pre-PROXY-flip)" state is the permanent state.
> See D6 in `docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md`.

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
   is within CF ranges). **Update 2026-07-08:** the PROXY-protocol
   fallback in this parenthetical will never apply - the flip was
   attempted and dropped permanently (see WP3 note above and D6 in the
   design spec). Safety here rests solely on the origin-lockdown
   `loadBalancerSourceRanges`/CF-IP-range restriction in step 4 (or, if
   that never works cleanly, on accepting the direct-to-LB bypass risk
   with WP1's caps as the backstop, as already noted in step 4).
7. Update `docs/superpowers/specs/2026-07-08-20-external-dns-design.md` note: its "no second NS
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
