# 22: Email via Resend - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/22-email-via-resend.md`. Task granularity is
> work-package level; run superpowers:writing-plans against the paired spec
> before execution if bite-sized steps are needed.

**Spec:** `docs/superpowers/specs/2026-07-05-22-email-via-resend-design.md`

## 22a: Outbound via Resend API [high priority - run before Phase 16]

- [x] Create the Resend account; verify `brdg.me` as the sending domain:
      add the SPF, DKIM, and DMARC DNS records. Done 2026-07-05: account
      created, all five records (DKIM, `send` MX + SPF, DMARC, apex
      receiving MX) added to `infra/dns.tf`. **Decision 2026-07-05:** the
      apex receiving MX was added now despite 22b planning inbound on
      `play.brdg.me` - all inbound `@brdg.me` mail now routes to Resend,
      superseding the legacy Linode server's A-record-fallback receipt
      (legacy play-by-email replies stop working; no webhook until 22b).
      Records applied and domain verified (the 2026-07-05 live-inbox check
      passing DKIM `d=brdg.me` proves both).
- [x] Replace `lettre` with `resend-rs` in `rust/web`:
      `send_login_email` sends via the Resend client (`resend_rs::Resend`,
      held in `AppState` alongside the existing shared `reqwest::Client`).
      Env: `RESEND_API_KEY` (unset = log fallback, replacing the
      `SMTP_HOST`-unset fallback) and `EMAIL_FROM` (default
      `login@brdg.me`). Removed `SMTP_HOST`/`SMTP_PORT`/`SMTP_FROM`
      handling, the `lettre` dependency, and updated `.env.template`.
- [x] Prod config (SealedSecret once Phase 15 lands; plain Secret before):
      `RESEND_API_KEY`, `EMAIL_FROM`. Done 2026-07-05: `email-config`
      Secret created in the `brdgme` namespace on the new DOKS cluster
      (item 21 stage-2). Not yet referenced by any Deployment - the app
      isn't deployed to this cluster until items 15/16.
- [x] Delete `k8s/base/smtp/` from the new-system overlays and the
      Tiltfile. Checked whether the legacy stack (`rust/api`) sends
      through the in-cluster `smtp` Service: it does not (`Mail::Relay`/
      `Mail::Smtp` in `rust/api/src/config.rs` is constructed but never
      read - the one call site in `controller/auth.rs` is commented out),
      so `k8s/base/smtp/` was deleted outright rather than moved to a
      legacy overlay.
- [x] Verify: a prod login email lands in a real Gmail inbox - not spam -
      with SPF, DKIM, and DMARC all passing (inspect the received headers).
      Done 2026-07-05, ahead of app deployment: sent directly via the
      Resend HTTP API (`login@brdg.me` -> Gmail) since the app isn't on
      the new cluster yet. Landed in inbox; headers show
      `spf=pass`, `dkim=pass` (both `d=brdg.me s=resend` and
      `d=amazonses.com`), `dmarc=pass`. The full in-app login-email check
      (via `resend-rs` in the running web service) stays on the item 16
      cutover checklist.
- [x] Rate-limit the login endpoint (the only email-sending route today)
      with `tower_governor` per client IP (added 2026-07-03 final pass):
      without it, anyone hammering the login form drains the 100/day Resend
      quota and locks every player out of logging in. Implementation note:
      `Login` is a Leptos server function auto-mounted by `leptos_axum`
      alongside every other server fn/page route in one opaque `Router`
      build step, so a `GovernorLayer` can't be scoped to just that route
      without either rate-limiting the whole app or all of `/api`. Instead
      `auth/rate_limit.rs` builds the same `governor` rate limiter
      `tower_governor` uses internally and checks it directly inside the
      `login()` handler body, keyed by `SmartIpKeyExtractor` (falls back
      through `X-Forwarded-For`/`X-Real-Ip`/`Forwarded` to the TCP peer
      address). Burst 5, replenishes 1 every 20s, per IP. Caveat carried
      forward unresolved: verify real client IPs survive the DO LB +
      Cilium Gateway path (externalTrafficPolicy/PROXY protocol - a known
      DOKS consideration; fold into Phase 14's LB prerequisite check). If
      source IPs are not preserved, the limiter keys on the LB address and
      throttles everyone collectively - configure the LB first.

## 22b: Play-by-email (turn notifications + inbound replies)

**Tasks:**

- [ ] Confirm the live Resend inbound payload schema and signature scheme
      against their docs/account before delegating the endpoint work (the
      shapes above are from 2026-07 documentation, not verified in anger).
- [ ] Migration: `game_players.email_token`, `processed_webhook_events`,
      `users.turn_emails_enabled`.
- [ ] Resend receiving config for `play.brdg.me` (MX records via
      tofu/manual per the 22a note; webhook URL + secret). *(human/infra)*
- [ ] `email_render` module: markup → `text`/`html` bodies
      (`brdgme_markup::plain`/`html` both exist - verified; the bot's
      `markup_resolve_players` is reusable), status header, log digest,
      command usages, footer, threading headers.
- [ ] `notify_turn_emails` alongside `trigger_bot_turns`; elimination and
      finish mails at the same call sites.
- [ ] Webhook endpoint: signature verification + parser + server-command
      matcher + executor + response email.
- [ ] Tests (Phase 11 patterns): parser unit tests across quoting styles
      (Gmail, Outlook, plain `>`); endpoint integration tests with JSON
      fixtures and a fixed webhook secret (`#[sqlx::test]` + mock game
      service); sender-auth rejection cases (bad token, From mismatch,
      From matching an unverified address, bad signature);
      duplicate-event idempotency; server-command handling (`concede`,
      `unsubscribe`); opt-out suppresses notifications but never command
      responses.
- [ ] Quota guard: count outbound sends (extend the Phase 18 send-counter
      metric); alert via a Grafana Cloud rule as volume approaches 100/day
      or 3k/mo. Note 22b/22c multiply volume:
      each turn in an N-player game can trigger several mails - revisit
      the Pro-tier decision when enabling.

## 22c: Turn reminders (new feature, added 2026-07-04)

**Tasks:**

- [ ] Migration + `is_turn` transition reset.
- [ ] Sweep task + send path.
- [ ] Tests: due/not-due boundaries, no double-send, reset on turn change,
      opt-out and bot exclusion, SKIP LOCKED behaviour under two
      concurrent sweeps.

## 22d: Multiple emails per account + active-address switching (new feature, added 2026-07-04)

**Tasks:**

- [ ] Migration: `verified_at`, partial unique index on primary; backfill.
- [ ] Server fns: list/add (send code)/confirm/remove/switch; login lookup
      across verified addresses (check what the current login server fn
      does with `user_emails` today and align).
- [ ] Settings UI section (Leptos): address list, add + confirm flow,
      make-active, remove.
- [ ] Switch → outstanding-turn digest send.
- [ ] 22b integration: From matching across verified addresses; all sends
      target the primary address at send time (look up per send, don't
      cache).
- [ ] Tests: verification flow, single-primary invariant under concurrent
      switches, unverified address rejected for From match and switch,
      digest sent on switch (capped), login via secondary address.

**Sequencing within 22:** 22b → 22c (reuses the notification body and
sweep pattern) → 22d (touches auth; largest UI surface). 22c and 22d are
independent of each other; either can slot in when convenient after 22b.
