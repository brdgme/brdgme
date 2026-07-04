# Phase 22: Email via Resend

**Status:** 22a code complete (landed 77a2092); human/infra steps + 22b pending - high priority

**Decision (2026-07-03):** all platform email moves to Resend (resend.com).
Outbound replaces the self-managed SMTP relay path (spam-filter /
deliverability pain); inbound (play-by-email replies) uses Resend receiving,
which POSTs parsed email to a webhook. Chosen for: inbound webhooks on every
tier including free (3,000 emails/mo combined sent+received, 100/day cap),
an official Rust SDK (`resend-rs`, built on the `svix` crate that also
verifies its webhooks), and the strongest hobby/OSS adoption of the current
providers. Rejected: Postmark (inbound locked to Pro tier), Mailgun
(1 inbound route on Basic; full routing $35/mo), SES (cheapest at scale but
S3/SNS plumbing + AWS account overhead - revisit only if sustained volume
exceeds ~10k/mo, where SES is ~$1 vs Resend's $20).

**Split:** 22a (outbound swap - small, independent of the k8s phases, run
early) and 22b (turn notifications + inbound replies - a new feature, after
cutover). Free-tier watch item: the 100/day combined cap is the binding
constraint; the escape hatch is Pro at $20/mo.

### 22a: Outbound via Resend API [high priority - run before Phase 16]

**Revised 2026-07-03 (same day):** send via the Resend HTTP API with
`resend-rs`, not SMTP. DigitalOcean blocks outbound SMTP ports (25/465/587)
by default and unblocking is a discretionary support request; the API over
443 sidesteps the whole problem class. This drops `lettre` entirely and
deletes the in-cluster smtp relay, and the same `resend-rs`/`svix` stack
verifies the 22b inbound webhooks - a net dependency reduction.
(Supersedes the Mailpit quick win and the earlier SMTP-transport version of
this section.)

**Dev story:** no email infrastructure in dev at all. `RESEND_API_KEY`
unset → the existing log fallback prints the email (already how login codes
are read in dev). For work on real email content (22b templates), set a
Resend test-mode API key in `.env` - the same pattern the bot uses for
`LLM_API_KEY`.

- [ ] Create the Resend account; verify `brdg.me` as the sending domain:
      add the SPF, DKIM, and DMARC DNS records. Zone-level records belong
      to OpenTofu (Phase 21) once it exists; if 22a runs first, add them
      manually and note them for import. *(human/infra - not done here)*
- [x] Replace `lettre` with `resend-rs` in `rust/web`:
      `send_login_email` sends via the Resend client (`resend_rs::Resend`,
      held in `AppState` alongside the existing shared `reqwest::Client`).
      Env: `RESEND_API_KEY` (unset = log fallback, replacing the
      `SMTP_HOST`-unset fallback) and `EMAIL_FROM` (default
      `login@brdg.me`). Removed `SMTP_HOST`/`SMTP_PORT`/`SMTP_FROM`
      handling, the `lettre` dependency, and updated `.env.template`.
- [ ] Prod config (SealedSecret once Phase 15 lands; plain Secret before):
      `RESEND_API_KEY`, `EMAIL_FROM`. *(human/infra - not done here)*
- [x] Delete `k8s/base/smtp/` from the new-system overlays and the
      Tiltfile. Checked whether the legacy stack (`rust/api`) sends
      through the in-cluster `smtp` Service: it does not (`Mail::Relay`/
      `Mail::Smtp` in `rust/api/src/config.rs` is constructed but never
      read - the one call site in `controller/auth.rs` is commented out),
      so `k8s/base/smtp/` was deleted outright rather than moved to a
      legacy overlay.
- [ ] Verify: a prod login email lands in a real Gmail inbox - not spam -
      with SPF, DKIM, and DMARC all passing (inspect the received headers).
      *(human/infra - not done here)*
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

Delivers the founding VISION principle: a full game playable from an email
client. Sequencing: after the Phase 16 cutover (additive feature; fine to
build during the validation window). Depends on 22a.

**Design:**

- **Inbound domain:** dedicated subdomain `play.brdg.me` with MX records
  pointing at Resend receiving - keeps `brdg.me`'s own MX untouched.
- **Reply addressing / sender auth:** each `game_players` row gets a random
  unique `email_token` (migration). Notification emails set
  `Reply-To: g-{email_token}@play.brdg.me`. On receipt the token maps to
  (game, player), and the From address must also match that player's user
  email - the token authorises, the From check is defence in depth. From
  alone is never trusted (trivially spoofable).
- **Turn notification email:** sent when a player's `is_turn` transitions
  to true - same call sites as `trigger_bot_turns` (execute_command;
  create/undo/concede/restart handlers). Content: game type + opponents,
  the player's render converted markup → plain text, command examples from
  the command spec, and reply instructions. Bot players skipped.
- **Webhook endpoint:** `POST /api/webhooks/resend` on the monolith.
  Resend webhooks are svix-signed: verify with the `svix` crate (the same
  library `resend-rs` builds on) against the raw request body; secret via
  env/SealedSecret; 401 on failure. Svix verification also rejects
  timestamps more than 5 minutes old (replay protection).
- **Reply parsing:** take the text/plain part; drop quoted lines (`>`
  prefix), everything from the first `On ... wrote:` line onward, and
  anything after a `-- ` signature marker. Remaining non-empty lines are
  commands, executed in order via `execute_command`; stop at the first
  error. If Resend delivers raw MIME rather than parsed JSON, use the
  `mail-parser` crate for MIME handling rather than hand-rolling it;
  quote-stripping stays bespoke either way (a few lines, no well-maintained
  crate covers it).
- **Response email:** every inbound reply gets one: the updated render on
  success (move confirmed), or the validation error + current render on
  failure. This closes the loop - inbox-only play.
- **Idempotency:** the provider retries webhooks on non-2xx. Store
  processed webhook event ids (small table, unique constraint, periodic
  cleanup); a duplicate id returns 200 without re-executing.

**Tasks:**

- [ ] Confirm the live Resend inbound payload schema and signature scheme
      against their docs/account before delegating the endpoint work (the
      shapes above are from 2026-07 documentation, not verified in anger).
- [ ] Migration: `game_players.email_token` + `processed_webhook_events`.
- [ ] Resend receiving config for `play.brdg.me` (MX records via
      tofu/manual per the 22a note; webhook URL + secret).
- [ ] Markup → plain-text render path: check what `brdgme_markup` already
      provides; the bot's `markup_resolve_players` is reusable.
- [ ] `notify_turn_emails` alongside `trigger_bot_turns`. Check whether the
      legacy `users` table already has an email-notification pref column
      before adding an opt-out flag.
- [ ] Webhook endpoint: signature verification + parser + executor +
      response email.
- [ ] Tests (Phase 11 patterns): parser unit tests across quoting styles
      (Gmail, Outlook, plain `>`); endpoint integration tests with JSON
      fixtures and a fixed webhook secret (`#[sqlx::test]` + mock game
      service); sender-auth rejection cases (bad token, From mismatch,
      bad signature); duplicate-event idempotency.
- [ ] Quota guard: count outbound sends; alert via Phase 18 vmalert as
      volume approaches 100/day or 3k/mo.

