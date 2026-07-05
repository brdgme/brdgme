# 22: Email via Resend

**Status:** 22a complete 2026-07-05 (code landed 77a2092; DNS records
applied, domain verified, prod secret created, live-inbox
SPF/DKIM/DMARC check passed; only the in-app send check remains, on the
Phase 16 beta checklist). 22b-22d pending, post-cutover.

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

### Learnings from legacy Go brdg.me (reviewed 2026-07-04)

The old `~/Development/brdg.me` Go server (`server/email/`,
`server/game/game.go`, `server/scommand/`) implemented full play-by-email.
What to keep, and what its bespoke implementation got wrong:

**Keep (feature parity targets):**
- **Dual-format bodies:** every mail was `multipart/alternative` - plain
  text (terminal render) + HTML: the whole body inside a dark
  terminal-styled `<pre>` (`color:white;background:#1d1f21;
  font-family:DejaVu Sans Mono,monospace;white-space:pre-wrap`). Cheap,
  distinctive, and renders board ASCII perfectly. Reproduce with
  `brdgme_markup::plain` + `brdgme_markup::html`; Resend takes `text` +
  `html` fields natively - no hand-rolled MIME.
- **Thread per game:** initial mail set `Message-Id: <{game_id}@brdg.me>`;
  every later mail set `In-Reply-To` to it, so one game = one conversation
  in the client. Reproduce via Resend custom headers (also add
  `References` for better client compat).
- **Rich, actionable body:** header (whose turn / "the game is over, the
  winners were..."), a `Since last time:` digest of game-log entries new
  to that player, a `You can:` list of currently legal command usages, the
  player's board render, and a "continue playing in your browser" link.
  The command-usage list is what made email play usable - port it from the
  command spec.
- **Every recipient class:** on each state change the old server mailed
  (a) the acting player with output/errors, (b) players whose turn it now
  is, (c) current-turn players with new logs, (d) newly eliminated
  players, (e) everyone uncontacted at game end. Parity target, with (c)
  approximated via `last_turn_at` (no per-player read markers in the new
  schema; `is_read` covers the acting player's own view).
- **Unsubscribe:** footer text + `unsubscribe`/`subscribe` as commands,
  checked per user before any send.
- **Email as a full interface:** server-level commands worked by mail with
  no game id: `new`, `join`, `list`, `say`, `concede` (+vote), `restart`,
  `subscribe`/`unsubscribe`; unknown senders got a welcome mail with
  usage. 22b scopes this down (see below) - in-game commands + a small
  server-command set on reply; starting games by email is out of scope
  until demand exists.

**Do differently (the mess to avoid):**
- Hand-rolled MIME assembly and parsing: regex quoted-printable decoding
  (panicked on bad input), regex HTML tag stripping, manual multipart
  recursion. Replaced by Resend's parsed webhook payload, with
  `mail-parser` as the raw-MIME fallback.
- **From-header-only sender auth** - trivially spoofable; anyone could
  play as anyone. Replaced by the per-player `email_token` in Reply-To;
  From match is demoted to defence in depth.
- **Game id parsed out of the Subject line** (regex UUID) - fragile when
  clients rewrite subjects. The Reply-To token replaces it.
- Local Postfix relay + hardcoded SMTP password + an unauthenticated
  inbound HTTP listener on :9999. All gone with Resend (22a) + svix-signed
  webhooks.
- Commands executed with no idempotency/replay protection.

### 22b: Play-by-email (turn notifications + inbound replies)

Delivers the founding VISION principle: a full game playable from an email
client. Sequencing: after the Phase 16 cutover (additive feature; fine to
build during the validation window). Depends on 22a.

**Design:**

- **Inbound domain:** dedicated subdomain `play.brdg.me` with MX records
  pointing at Resend receiving - keeps `brdg.me`'s own MX untouched.
- **Reply addressing / sender auth:** each `game_players` row gets a random
  unique `email_token` (migration). Notification emails set
  `Reply-To: g-{email_token}@play.brdg.me`. On receipt the token maps to
  (game, player), and the From address must also match one of that
  player's user's **verified** `user_emails` rows (not just the primary -
  see 22d) - the token authorises, the From check is defence in depth.
  From alone is never trusted (trivially spoofable).
- **Message format:** Resend `text` + `html` fields. `text` from
  `brdgme_markup::plain`; `html` from `brdgme_markup::html` wrapped in the
  legacy dark terminal `<pre>` style (see learnings above). One shared
  `email_render` module used by every outbound game mail (notifications,
  reminders, command responses, digests).
- **Threading:** first mail for a game carries
  `Message-Id: <game-{game_id}@brdg.me>`; subsequent mails set
  `In-Reply-To`/`References` to it via Resend custom headers. Subject:
  `{Game type} with {opponent names}` - stable across the thread, no game
  id needed in it.
- **Body content (parity with legacy):** status header (whose turn /
  result), `Since last time:` log digest (game_logs newer than the
  recipient's `last_turn_at`, filtered by log targets), board render,
  `You can:` command usages from the command spec, browser link,
  unsubscribe footer. Also send `List-Unsubscribe` +
  `List-Unsubscribe-Post` headers (one-click unsubscribe; Gmail/Yahoo
  now require it for bulk senders).
- **Recipient classes (parity with legacy):** turn notification when
  `is_turn` transitions to true (same call sites as `trigger_bot_turns`:
  execute_command; create/undo/concede/restart handlers); elimination
  mail on `is_eliminated` transition; game-finished mail (with placings
  and rating changes) to all human players at finish; command
  response/error mail to an inbound sender. Bot players always skipped;
  unsubscribed users always skipped.
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
- **Server commands in replies:** besides game commands, honour
  `concede`, `undo`, `restart`, and `unsubscribe`/`subscribe` (scoped to
  the token's game where applicable; un/subscribe is account-wide) -
  matched case-insensitively before falling through to `execute_command`.
  This is the pared-down legacy `scommand` set; `new`/`join`/`list`/`say`
  are not ported (no email-initiated games).
- **Response email:** every inbound reply gets one: the updated render on
  success (move confirmed), or the validation error + current render on
  failure. This closes the loop - inbox-only play. Unknown token or
  failed From check: no response at all (don't create a bounce oracle).
- **Opt-out:** `users.turn_emails_enabled boolean NOT NULL DEFAULT true`
  (legacy `rust/api` schema has no such column - confirmed; the old Go
  server used a MongoDB `Unsubscribed` flag). Toggled by the
  unsubscribe/subscribe commands, the List-Unsubscribe endpoint, and a
  settings UI checkbox.
- **Idempotency:** the provider retries webhooks on non-2xx. Store
  processed webhook event ids (small table, unique constraint, periodic
  cleanup); a duplicate id returns 200 without re-executing.

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

### 22c: Turn reminders (new feature, added 2026-07-04)

If a player has held the turn for a threshold time, nudge them by email.
`game_players.is_turn_at` already records when the turn started - no
schema change needed for detection, only for send-tracking.

**Design:**

- Migration: `game_players.turn_reminder_sent_at timestamp NULL`. Reset to
  NULL whenever `is_turn` transitions (both directions).
- A `tokio` interval task in the monolith (single-flight guard: `FOR
  UPDATE SKIP LOCKED` on the candidate rows so multi-replica deploys don't
  double-send) sweeps every ~15 min for
  `is_turn AND NOT is_eliminated AND turn_reminder_sent_at IS NULL AND
  is_turn_at < now() - interval '24 hours'`, joined against
  `turn_emails_enabled` and bot exclusion.
- Reminder mail reuses the 22b turn-notification body (render + usages +
  reply instructions) with a "still your turn" header; same game thread
  headers. One reminder per turn (v1); escalation/repeat cadence and a
  per-user threshold preference are future options, not built now.
- Threshold via env (`TURN_REMINDER_AFTER`, default 24h) so dev can test
  with minutes.

**Tasks:**

- [ ] Migration + `is_turn` transition reset.
- [ ] Sweep task + send path.
- [ ] Tests: due/not-due boundaries, no double-send, reset on turn change,
      opt-out and bot exclusion, SKIP LOCKED behaviour under two
      concurrent sweeps.

### 22d: Multiple emails per account + active-address switching (new feature, added 2026-07-04)

The `user_emails` table (multiple rows per user, `is_primary`) already
exists in the schema and legacy data - the login flow just only ever uses
one. Feature: a user links several addresses (e.g. personal + work) and
switches the active one on the fly; switching re-sends outstanding-turn
emails to the newly active address so their playable games follow them.

**Design:**

- **Model:** `is_primary` = the active address. All outbound game email
  for a user goes to the primary address only. Inbound From matching
  accepts any **verified** address on the account.
- **Verification:** adding an address sends a confirmation code to the new
  address (reuse the login-code machinery); it becomes usable
  (switchable-to, From-matchable) only once confirmed. Migration:
  `user_emails.verified_at timestamp NULL`; backfill existing rows as
  verified (they predate the feature and were login-used). Unverified
  addresses expire after ~24h (cleanup in the 22c sweep task).
- **Switching:** settings UI lists addresses with add/remove/"make
  active". Server fn sets `is_primary` in one transaction (exactly one
  primary per user - enforce with a partial unique index
  `ON user_emails(user_id) WHERE is_primary`). Cannot remove the primary;
  cannot switch to unverified.
- **Switch triggers outstanding-turn digest:** on switch, find all games
  where the user `is_turn AND NOT is_finished` and send each game's turn
  notification (22b body) to the new address, so the inbox they just
  moved to contains every actionable game. Cap at the first ~20 games by
  `is_turn_at` (quota protection). Reset `turn_reminder_sent_at` is NOT
  done - reminders track the turn, not the address.
- **Login:** login accepts any verified address (code goes to the address
  entered), resolving to the owning user. Signup path unchanged (creates
  user + primary).
- **Out of scope (v1):** per-address schedules/auto-switching by time of
  day - the manual switch is the feature; automation can come later if
  wanted.

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

