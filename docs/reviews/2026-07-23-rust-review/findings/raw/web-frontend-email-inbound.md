# Raw findings: web-frontend-email W1 (email/inbound.rs, email/mod.rs)

Scope: rust/web/src/email/inbound.rs (2015 LOC incl. tests), rust/web/src/email/mod.rs (8 LOC). Supporting reads: email/commands.rs (dispatch surface), email/notify.rs (reply_address), email/render.rs (List-Unsubscribe headers), web/migrations/014_email_play.sql, web/src/main.rs (RESEND_API_KEY wiring).

### Settings route authenticated solely by spoofable From header; s- token ignored
- severity: critical
- category: correctness
- location: web/src/email/inbound.rs:484
- finding: `resend_webhook` sends both `Some(InboundRoute::Settings(_))` and `None` routes to `handle_settings_reply_route`, discarding the settings token entirely. `handle_settings_reply` (line 1111) then resolves the acting user purely from the inbound `From` address via `resolve_user_by_verified_from`. The code never checks SPF/DKIM/authentication-results, and From is trivially forgeable in SMTP. Anyone who knows a user's verified email address can send a forged-From email to `s-anything@brdg.me` - or to ANY address on the domain that lacks a g-/i-/s- prefix, since `None` also lands here - and execute standalone commands as that user (subscribe/unsubscribe, settings changes, bump, per commands.rs). The svix signature only authenticates that Resend delivered the webhook, not the email's sender.
- recommendation: Make the `s-` token a real per-user secret (like game email_tokens) and require it: look up the user by token and additionally require the From match, mirroring `handle_game_reply`. Drop the `None` fallthrough (unrouted addresses should be ignored or bounced, not treated as settings). Ideally also check Resend's SPF/DKIM verdict on the inbound message if the payload exposes it.

### Sender verification is forgeable From matching; token secrecy is the only real auth
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:378
- finding: `from_matches_verified_email` compares the webhook-supplied `From` string against verified addresses; since From is spoofable and no DKIM/SPF result is consulted, the effective secret for game and invite routes is the `g-`/`i-` email token in the recipient address alone. That is acceptable only while those tokens stay secret. The web-domain review already flagged that proposal views leak invitee `email_token`s to any authenticated viewer (not re-flagged here): combined with this spoofable From check, any authenticated user who can view a proposal can forge accept/decline emails for other invitees (`handle_invite_reply` will accept them, potentially starting the game via `start_proposal_tx`). The From check adds no defense once the token is known.
- recommendation: Treat email tokens as bearer secrets (fix the view leak, rotate tokens after the leak fix); consider validating Resend's authentication results for defense in depth, and rotate a game/invite token after each use or on demand.

### Idempotency marker inserted before processing: failures are permanently dropped
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:456
- finding: `mark_event_processed` inserts the dedupe row before any processing, and the handler returns 200 OK regardless of downstream outcome. If anything after the marker fails - JSON parse (line 465), raw-email fetch from Resend (line 529), DB errors during the invite transaction, `start_proposal_tx` failure, outbound reply send - the event is already marked processed, Resend/svix sees a 2xx so it never retries, and every early-return in the handlers just logs and sends the player nothing. A player's move or invite response silently vanishes with no retry and no error email.
- recommendation: Either (a) insert the marker only after successful processing and return 5xx on transient failures so svix retries (keep the pre-check as a read to short-circuit true duplicates), or (b) keep the early marker but delete it / mark failed on error paths and return 5xx. At minimum, send the player an "internal error, please retry" reply on post-auth failures instead of silence.

### Unsubscribe emails are never honored
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:1070
- finding: Outbound emails advertise `List-Unsubscribe: <mailto:unsubscribe@brdg.me?subject=unsubscribe>` (also render.rs:237), but the inbound path cannot honor it: `unsubscribe@brdg.me` has no g-/i-/s- prefix so it falls into the settings route, where the SUBJECT is never read - only the body is parsed for commands. A mail client's unsubscribe action sends an empty-body email with subject "unsubscribe", so the user gets back "I could not find a command in your email." and stays subscribed. Additionally `List-Unsubscribe-Post: List-Unsubscribe=One-Click` is invalid alongside a mailto-only URI (RFC 8058 requires an HTTPS URI), which mailbox providers may score against deliverability (header built here at lines 1064-1075 and in render.rs).
- recommendation: In the settings fallback, detect delivery to `unsubscribe@` (or an `unsubscribe` subject) and run the unsubscribe toggle for the resolved user. Either add an HTTPS one-click endpoint or drop the List-Unsubscribe-Post header.

### From/recipient matching likely breaks on "Display Name <addr>" forms
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:37
- finding: UNCERTAIN (depends on the exact shape Resend puts in `data.from` / `data.to`). `parse_reply_address` naively splits on the first `@` ("Name <g-tok@brdg.me>" yields local part "Name <g-tok" -> no route), and `from_matches_verified_email` / `resolve_user_by_verified_from` compare the raw From string against stored bare addresses with `LOWER(email) = LOWER($2)` - a From of `"Alice <alice@x.com>"` never matches. Nearly all mail clients set a display name, so if Resend passes the raw header form the whole reply-to-play flow silently rejects mail ("no response" info logs). The same raw `from` string is also passed straight to `send_rendered_email` as the recipient.
- recommendation: Parse addresses properly before matching - mail_parser is already a dependency and can parse address headers - extracting the bare addr-spec from both the From value and each recipient entry. Add tests with display-name forms.

### Quote-stripping heuristics misparse common client reply formats
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:8
- finding: `parse_reply_commands` only stops at a single-line `On ...wrote:` attribution or a `--` signature marker. Common real-world cases slip through: Gmail wraps long attributions so `wrote:` lands on the next line (the `On Wed, 22 Jul 2026 at 13:16, brdg.me <mail@brdg.me>` line is then treated as a command); Outlook top-posts an unquoted `-----Original Message-----` / `From: ... Sent: ...` block; localized clients use non-English attributions ("Am ... schrieb:"). These stray lines become commands, and since the loop stops at the first failure the player receives a confusing "Your command failed" report for text they never typed.
- recommendation: Harden the heuristics (multi-line attribution detection, `-----Original Message-----`, `From:`/`Sent:` header blocks) or adopt a dedicated reply-parsing approach; at minimum treat a trailing block of unmatched non-command noise after valid commands more forgivingly.

### Row lock held across outbound email send in invite paths
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:684
- finding: In `handle_invite_reply`, the "invite no longer open" (line 684) and "already responded" (line 710) paths call `send_invite_reply_response` - several DB reads plus an HTTP send to Resend - while `tx` (holding the `lock_proposal_for_update` FOR UPDATE row lock) is still alive; the lock is only released when the function returns and `tx` drops. A slow Resend call blocks all concurrent responders to that proposal for the duration. The success path also runs `start_proposal_tx` (which takes `&state.http_client`, implying an external call) inside the transaction, though that at least needs atomicity.
- recommendation: Drop/rollback the transaction explicitly before sending the response email in the early-exit paths.

### Dead production code: run_commands_in_order, CommandLoopOutcome, error_reply_text
- severity: minor
- category: simplicity
- location: web/src/email/inbound.rs:184
- finding: `run_commands_in_order` / `CommandLoopOutcome` (lines 184-204) and `error_reply_text` (line 227) have no callers anywhere in web/src outside this file's tests - the real loop is `run_game_reply_commands`. They look like superseded scaffolding kept alive only by their own tests.
- recommendation: Delete them and their tests, or wire `error_reply_text` into the actual error path if it was meant to be used.

### RESEND_API_KEY fetch + ResendInbound construction duplicated three times
- severity: minor
- category: simplicity
- location: web/src/email/inbound.rs:518
- finding: The identical block (env var read with empty check, error log, `ResendInbound` construction, `fetch_raw_email`, fetch-failure log) appears in `handle_game_reply` (518-535), `handle_invite_reply` (625-642), and `handle_settings_reply_route` (1088-1107). Also inconsistent with the rest of the app: main.rs reads `RESEND_API_KEY` once at startup into `AppState.resend`, while this file re-reads the env var per inbound email.
- recommendation: Store the inbound source (or the API key) on AppState at startup and extract a `fetch_inbound_text(state, email_id) -> Option<String>` helper.

### All processing happens inline before the webhook responds
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:477
- finding: The handler fetches the raw email from Resend, runs game commands (each an HTTP round-trip to the game service), renders MJML, and sends the reply email - all before returning 200. Svix webhook delivery has a response timeout (~15s); a slow game service or Resend hiccup causes svix to record a failure and retry. The early dedupe marker absorbs the retry (returns OK doing nothing), so the net effect is delivery marked failed plus wasted retries, and combined with the at-most-once finding above, a timed-out-but-still-running first attempt is indistinguishable from success.
- recommendation: Verify + dedupe + enqueue (tokio::spawn or a job row) and return 200 immediately; do the fetch/dispatch/reply work in the background task.

### No pruning of processed_webhook_events
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:408
- finding: `mark_event_processed` inserts one row per webhook delivery forever. Migration 014 creates an index on `processed_at` (clearly intended for pruning) but no code in web/src (including sweep.rs) deletes old rows, so the table grows without bound.
- recommendation: Add a periodic delete of rows older than the svix retry window (e.g. in the existing sweep job).

### Silent return when player row missing from roster
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:704
- finding: In `handle_invite_reply`, if the token's player is not found in `find_proposal_players_tx` results the function returns with no log line, unlike every sibling early-return which logs at info/warn/error.
- recommendation: Add a `tracing::warn!` before returning.

### Invite response subject degrades to " invite" on lookup failure
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:841
- finding: `send_invite_reply_response` folds proposal/game-version/game-type lookup failures into an empty `game_type_name` via `.unwrap_or(None).unwrap_or_default()` and `_ => (String::new(), None)`, producing the subject " invite" (leading space) with errors swallowed unlogged.
- recommendation: Log the failure and use a neutral fallback subject like "Your brdg.me invite".

### Reply-address formats hardcoded and duplicated instead of shared helpers
- severity: nit
- category: consistency
- location: web/src/email/inbound.rs:882
- finding: The invite reply address is built inline as `format!("i-{}@brdg.me", ...)` (882) and the settings one as `format!("s-{user_id}@brdg.me")` (1191), while the game route uses the `crate::email::notify::reply_address` helper. The domain is hardcoded in all three (and in render.rs), with no single source of truth; `parse_reply_address` meanwhile accepts any domain.
- recommendation: Add `invite_reply_address`/`settings_reply_address` helpers next to `reply_address`, with the domain from one constant (or config).

### "accept" wins over "decline" regardless of order in the body
- severity: nit
- category: correctness
- location: web/src/email/inbound.rs:646
- finding: `handle_invite_reply` scans all command lines for both words; if a reply contains both (e.g. "decline" then quoted or corrected "accept" text that survived quote-stripping), `accept` unconditionally wins at line 722 rather than the first/last stated intent.
- recommendation: Use the first line that matches either verb.

### verify_webhook can panic on non-header-safe input
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:144
- finding: `HeaderValue::from_str(...).unwrap()` on the three caller-supplied strings. Safe on the current call path (values originated from a HeaderMap via `to_str()`), but `verify_webhook` is a pub fn; called with a string containing control characters it panics instead of returning an error.
- recommendation: Map `from_str` errors to `VerifyError::Other`/`InvalidSignature` instead of unwrapping.

## Areas reviewed and found clean

- Svix signature verification: correct use of the svix crate (constant-time HMAC verify, timestamp tolerance) with secret sourced from env and empty-secret rejected; missing headers and bad signatures return 401.
- Webhook dedupe semantics: `INSERT ... ON CONFLICT DO NOTHING` on svix-id with rows_affected check is race-free across concurrent deliveries.
- Email token lookup uses parameterized SQL (`email_token = $1`) - no injection; DB equality compare is fine here (index lookup timing is not a practical oracle for high-entropy tokens).
- All SQL in the file is parameterized; email-derived text (commands, From) never interpolated into queries or shell.
- Command bodies are dispatched as opaque strings to the command layer; no injection path into logs beyond plain formatting.
- `parse_reply_commands` core behavior (quoted-line skip, signature cut, blank-line drop) is well tested; `select_route` to/received_for precedence tested.
- Invite accept flow: FOR UPDATE proposal lock + pending-check + response update + conditional start inside one transaction is a sound race guard against double-accept; bot-slot (NULL user_id) tokens rejected; game-token query requires `user_id IS NOT NULL`.
- `run_game_reply_commands` outcome partitioning (applied/failed/not_applied, FullContent short-circuit) is correct and thoroughly tested.
- extract_plain_text delegates MIME parsing to mail_parser (right dependency choice); rules reply threading headers and de-threaded failure report verified by tests.
- mod.rs: trivial re-exports, fine.

Severity tally: 1 critical, 4 major, 6 minor, 5 nit (16 findings).
