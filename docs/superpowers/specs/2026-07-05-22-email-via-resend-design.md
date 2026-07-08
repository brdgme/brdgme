# 22: Email via Resend - Design

> Extracted 2026-07-08 from `docs/plan/22-email-via-resend.md` (superpowers layout
> migration). Content dates from 2026-07-05; this is a point-in-time decision
> record, not a living document.

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

## 22a: Outbound via Resend API [high priority - run before Phase 16]

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

## Learnings from legacy Go brdg.me (reviewed 2026-07-04)

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

## 22b: Play-by-email (turn notifications + inbound replies)

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

## 22c: Turn reminders (new feature, added 2026-07-04)

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

## 22d: Multiple emails per account + active-address switching (new feature, added 2026-07-04)

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

**Sequencing within 22:** 22b → 22c (reuses the notification body and
sweep pattern) → 22d (touches auth; largest UI surface). 22c and 22d are
independent of each other; either can slot in when convenient after 22b.
