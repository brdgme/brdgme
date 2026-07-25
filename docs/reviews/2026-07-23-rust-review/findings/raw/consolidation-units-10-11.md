# Consolidation notes: units 10-11

Both units reviewed and lead-verified in one session; curated tallies below are final.
Snapshot for both: f8763a5ba9c0ce3d0e85d61db7133d19a26ed313.

## Unit 10: web-domain (findings/web-domain.md)

### Tallies

critical 1 / major 12 / minor 35 / nit 30 = 78 total. Raw 80 across W1-W6; 2
merged in curation (bot slots W2+W3 -> one major; page-overflow W4+W5 -> one
minor); none rejected.

### Headlines

Critical:
- undo_game on finished game corrupts ratings permanently - no is_finished check;
  db::undo_game reverts state but never rewinds rating_change/game_type_users, and
  apply_rating_changes idempotency guard then blocks re-rating the real outcome.
  game/server_fns.rs:731 (+ db.rs:1407, 1554).

Most notable majors:
- Bot UserError wedges game - rejected bot command is acked, bot.turn never
  re-published, no sweep covers bots; game stuck. game/mod.rs:304-314, 402-410.
- Failed bot.turn publish after DB commit loses the bot turn - warn-only, no
  reconciliation. game/mod.rs:227-242 (also retry-exhaustion wedge at 372-383 and
  consumer spawned once with no restart, main.rs:55-74).
- undo_game has no stale-state guard - can clobber a concurrent move (db.rs
  UPDATE unconditional). game/server_fns.rs:784.
- concede_game TOCTOU - is_finished checked on snapshot, db::concede_game has no
  lock/guard; concurrent concedes or concede-vs-finish leave places and ratings
  contradicting. game/server_fns.rs:827 (db.rs:1292-1321).
- get_game_details ignores game_visibility - is_game_visible_to_user (db.rs:2228)
  is essentially dead code; any authed user sees any game. game/server_fns.rs:231.
- Stats endpoints bypass game_visibility entirely - anonymous profile/history/
  head-to-head name private users, enumerate game ids. stats/mod.rs:174.
- get_proposal serializes every invitee's email_token to any authenticated user -
  the token is the inbound-email auth credential. proposals.rs:78.
- Client-supplied bot slots unvalidated (create_proposal, add_proposal_player,
  restart_core) - bogus bot name creates an unrecoverably wedged game.
  proposals.rs:1163, 1469; game/server_fns.rs:868.
- Auto-decline keyed on proposal created_at not player invite time - late-added
  invitees and roster-reset accepteds are instantly auto-declined (terminal).
  proposals.rs:819.
- Rules-page version picked by ORDER BY name - returns oldest version;
  lexicographic semver wrong anyway. game_info/queries.rs:18.

### Unit state

~14.2k LOC / 19 files of web-crate domain logic: game command pipeline,
server fns, proposals, stats, friends, settings, content pages. Core paths are
solid (optimistic locking on moves, FOR UPDATE in restart_core and proposals,
authz on nearly all server fns, good test coverage); defects cluster at the
edges: the bot-turn NATS pipeline has no recovery path for any wedge mode,
undo/concede skipped the concurrency discipline the move path has, and the
game_visibility privacy model exists in db.rs but was never wired to the read
endpoints. Stats/friends/settings/new_game are mostly quality/consistency
issues (swallowed errors, N+1s, duplication).

### Theme evidence

- Privacy/visibility gates not wired: get_game_details (major), stats endpoints
  (major), get_rendered_rules ignores is_public (minor), export bundle includes
  private logs (minor), email_token serialized to all viewers (major).
- TOCTOU/concurrency guards missing: undo_game (critical + major), concede_game
  (major), cancel_proposal stale roster snapshot (minor), friends cross-request
  race (minor), placeholder_user check-then-insert (nit).
- Bot-slot/player-count validation: unvalidated bot slots x3 entry points
  (major); restart prefill count outside offered counts (nit).
- NATS/queue delivery semantics: bot UserError wedge, retry exhaustion, lost
  publish after commit, consumer never restarted (4 majors), stranded messages
  no term/DLQ (minor).
- Error-swallowing: invite-mailer find_proposal_players (minor), markup parse ->
  blank log lines (minor), mailer tasks Ok(Some) lets (minor),
  cancel_proposal_for_expiry (minor), restart prefill Err dropped (minor),
  friends/settings fire-and-forget mutations (minors), before-snapshot .ok()
  .flatten() (nit).
- Email auth/parsing weaknesses: email_token leak (major), dead Reply-To on
  notification emails with "reply to respond" footer (minor), invite emails
  never trimmed/case-normalized - 3 sites: proposals, settings, new_game
  (minors, one canonicalization policy fixes all).
- Boilerplate duplication: pre-transaction authz block x4 (~60 lines),
  single-human predicate x8 in stats SQL, per-row UPDATE loops.
- Unmaintained/duplicated deps: bespoke percent-encoder vs percent-encoding
  crate (nit).
- Other recurring: unbounded queries/payloads on public endpoints
  (finished_games/rating_series/head_to_head, index O(friends x 10) queries);
  hardcoded constants drifting (1200 base rating, viewBox literals).

### Discussion candidates

- Undo-vs-ratings semantics (the critical): reject undo after finish, or rewind
  ratings atomically? Product call on whether a finishing move is undoable at all.
- game_visibility scope: is the setting meant to gate only the logged-out index
  and friend feeds, or all read endpoints (game details, stats, history)?
  Stats also needs an anonymize-vs-filter decision.
- Export bundle privacy: does spec D4 deliberately accept private logs/hidden
  state in bundles "pasted into issues", or add --redact-private?
- Bot-wedge recovery design: per-error re-publish vs a reconciliation sweep
  ("bot on turn > N min -> re-publish bot.turn") - the sweep fixes all four
  wedge modes at once; pick one mechanism.
- Email canonicalization policy: trim+lowercase at boundaries vs enforcing
  lowercased storage globally (touches unique constraint and existing rows).
- Rules/game-info public posture: auth-gate rules or make them public; currently
  inconsistent both directions.

## Unit 11: web-frontend-email (findings/web-frontend-email.md)

### Tallies

critical 2 / major 12 / minor 28 / nit 18 = 60 total. One cross-worker
duplicate merged (RFC 8058/unsubscribe header defect, inbound.rs + render.rs).

### Headlines

Criticals:
- Settings route authenticated solely by spoofable From header - settings token
  discarded, unrouted addresses also fall through to settings; forged From
  executes standalone commands as any user. email/inbound.rs:484 (+1111).
- Email-management commands via that path enable account takeover - spoofed
  From can `emails add/confirm/active` an attacker address, redirecting all
  turn emails and game tokens. email/commands.rs:546.

Most notable majors:
- Idempotency marker inserted before processing - any post-marker failure is
  permanently dropped (svix sees 200, never retries); player moves silently
  vanish. email/inbound.rs:456.
- Advertised mailto unsubscribe can never be honored; List-Unsubscribe-Post
  with mailto-only URI violates RFC 8058 (deliverability risk).
  email/inbound.rs:1070 + render.rs:235 (merged).
- From/recipient matching likely breaks on "Name <addr>" forms (UNCERTAIN on
  Resend payload shape) - could silently kill the whole reply-to-play flow.
  email/inbound.rs:37.
- From verification forgeable; tokens are the only real auth - combined with
  web-domain's email_token leak, any authed proposal viewer can forge invite
  accept/decline. email/inbound.rs:378.
- Suppressed turn reminder permanently marked as sent - web-presence
  suppression returns true, sweep marks reminded, never retried. sweep.rs:132.
- FOR UPDATE SKIP LOCKED in candidate query is a no-op (autocommit fetch_all) -
  concurrent replicas can double-send; reads as protected but is not. sweep.rs:68.
- `bot:<name>` opponents unvalidated in email `new` command - wedged game from
  any inbound email, incl. the spoofable path. email/commands.rs:59.
- Concede/undo TOCTOU email paths + near-verbatim duplication of the server fns
  (same db.rs root cause as web-domain; restart was correctly factored, these
  were not). email/commands.rs:891, 966, 886.
- Turnstile widget likely never renders after client-side nav to /login
  (UNCERTAIN) - login stuck until hard refresh when Turnstile enabled.
  app.rs:595.
- GameMeta mutation actions swallow errors (concede/force-delete destructive
  actions give zero failure feedback). components/game.rs:56.

### Unit state

~9.7k LOC / 17 files: full email subtree (inbound webhook, command dispatch,
sweeps, notify, MJML render, outbound) plus Leptos frontend shell/components.
The rendering/threading/escaping layer and frontend hydration discipline are
strong (well-tested, deliberate hazards documented and confirmed); the
dominant problem is inbound-email authentication - From is the only identity
for the settings path, and tokens the only real secret elsewhere - plus
delivery-semantics bugs (mark-before-do in webhook dedupe and every sweep).
Frontend issues are mostly fire-and-forget error swallowing and consistency.

### Theme evidence

- Email auth/parsing weaknesses: both criticals (spoofable From -> command
  execution -> account takeover), forgeable From + token leak combo (major),
  display-name parsing (major), quote-stripping misparse (minor), accept-beats-
  decline ordering (nit), reply-address formats hardcoded x3 (nit).
- Error-swallowing: webhook failures dropped after early marker (major),
  restart internal errors leaked as User and unlogged (major), mrml/markup
  render failures silent (minors), game_log_count -> 0 corrupts threading
  (minor), metric counts failed sends as sent (minor), GameMeta/logout actions
  swallow errors (major + minor).
- NATS/queue delivery semantics (sweep/webhook variant): mark-before-send in
  reminder sweep (major), nudge marked regardless of send outcome (minor),
  expiry notification lost after status update (minor), inline processing past
  svix timeout compounds dedupe marker (minor), processed_webhook_events never
  pruned (minor).
- TOCTOU/concurrency guards missing: concede/undo email paths (majors, shared
  db.rs root cause), SKIP LOCKED no-op (major), ensure_email_token lost-update
  race + unpersisted token for missing row (minors), row lock held across
  Resend send (minor).
- Bot-slot/player-count validation: `bot:` opponents unvalidated (major) -
  same class as web-domain x3.
- Boilerplate duplication: run_concede/run_undo copy server fns (major),
  send_reminder duplicates notify::send_one ~90 lines (minor), RESEND_API_KEY
  block x3 (minor), five copy-pasted sweep spawn loops (nit), inline confirm
  dialogs vs shared helper (minor).
- Preference-gate drift: reminder sweep selects on reminder_emails_enabled but
  sends on turn_emails_enabled (minor); prod-dead Rust predicate drifting from
  SQL (minor).
- Request-reachable panics: essentially absent - only verify_webhook pub-fn
  unwrap on non-header-safe input (nit); frontend/render clean.

### Discussion candidates

- Email From-header authentication redesign (both criticals + forgeable-From
  major): per-user secret settings tokens, drop the None fallthrough, whether
  to require Resend SPF/DKIM verdicts, whether account-security commands belong
  on the email path at all.
- Sweep/webhook delivery semantics: at-least-once (mark after success, return
  5xx for svix retry, claim-then-send in sweeps) vs current at-most-once; also
  sync-vs-enqueue for the webhook (svix 15s timeout).
- Unsubscribe compliance: build an HTTPS one-click endpoint (RFC 8058 /
  Gmail-Yahoo bulk rules) vs mailto-only with the Post header dropped; and
  wiring unsubscribe into the standalone command path.
- concede_core/undo_core extraction: fixing the TOCTOU in db.rs vs also
  unifying the duplicated email/web paths (interacts with web-domain's
  undo-vs-ratings decision).
- Reminder preference semantics: which flag governs reminders
  (reminder_emails_enabled vs turn_emails_enabled) - product call.
- Reserved email verbs vs game move grammars: document the reservation or add
  an escape prefix.
- Turnstile rendering: explicit render() vs forcing full-page load for /login -
  needs prod-config verification either way.

## Cross-unit notes

- Shared root causes: db::undo_game / db::concede_game lack guards (unit 10
  critical/major, unit 11 majors are the email entry points - fix once in db.rs).
- email_token leak (unit 10) x forgeable From (unit 11) compose into forged
  invite responses by any authenticated proposal viewer.
- Unvalidated bot slots appear in 4 entry points across both units (3 web + 1
  email); email command path validated correctly before this class was added
  web-side - one shared choke point recommended.
- before=None diffing: unit 10 nit (was_finished) + unit 11 minor (duplicate
  turn emails in simultaneous-turn games) - same API shape flaw in
  notify_game_emails.
- Fire-and-forget error swallowing is the single most repeated pattern across
  both units (friends, settings, GameMeta, logout, mailers, sweeps).
