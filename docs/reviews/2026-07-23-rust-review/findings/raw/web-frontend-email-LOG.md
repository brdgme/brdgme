# web-frontend-email review LOG

Lead session started 2026-07-24. Snapshot: `/home/beefsack/Development/brdgme-review-snapshot` @ `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`.

Unit 11: residual of `rust/web/src` after units 9-10. Domain docs read by
Lead before dispatch: `docs/email.md` (mrml/MJML rendering, the Gmail
`font-size:0` foster-parenting hazard and its fix at render.rs:170 with
regression test at render.rs:329) and `docs/hydration.md` (SSR hydration
rules, mounted-gate idiom, id-skew hazard, nested-Suspense band-aid kept
deliberately in app.rs GamePage / components/game.rs GameMeta).

## Scope (~9,740 LOC, 17 files)

| File | LOC |
|---|---:|
| src/email/commands.rs | 2189 |
| src/email/inbound.rs | 2014 |
| src/email/sweep.rs | 1046 |
| src/email/notify.rs | 679 |
| src/email/render.rs | 553 |
| src/email/outbound.rs | 355 |
| src/email/mod.rs | 8 |
| src/components/game.rs | 660 |
| src/components/opponent_slot.rs | 352 |
| src/components/layout.rs | 316 |
| src/components/form.rs | 75 |
| src/components/mod.rs | 15 |
| src/components/spinner.rs | 14 |
| src/components/confirm.rs | 5 |
| src/app.rs | 924 |
| src/theme.rs | 465 |
| src/lib.rs | 70 |

## Known non-issues (do not re-flag)

- Custom parser combinator + duplicated `impl Parser for CommandSpec` (deliberate).
- DB-backed tests failing in plain local runs (backlog #40).
- Nested `Suspense` wrappers in app.rs GamePage / GameMeta - deliberate remnant, mounted-gate is the real fix (hydration.md case history).
- The `<mj-raw>` + `<tr><td>` + explicit font-size structure in render.rs - deliberate fix for the Gmail collapse hazard; do not flag as odd.

## Cross-unit context to carry into briefs

- web-domain flagged: turn-reminder sweep only targets humans (sweep.rs:34-91 referenced); bot-turn wedge modes; email_token leak via proposals view. Cross-unit patterns go in finding text, not new scope.

## Worker plan (serial, model fable per user override)

- W1: email/inbound.rs + email/mod.rs (~2,022) -> raw `web-frontend-email-inbound.md`
- W2: email/commands.rs (2,189) -> raw `web-frontend-email-commands.md`
- W3: email/sweep.rs + email/notify.rs (~1,725) -> raw `web-frontend-email-sweep-notify.md`
- W4: email/render.rs + email/outbound.rs + theme.rs (~1,373) -> raw `web-frontend-email-render-outbound-theme.md`
- W5: components/ + app.rs + lib.rs (~2,431) -> raw `web-frontend-email-frontend.md`

## Dispatch / return log

### W1 dispatched -> returned (email/inbound.rs, email/mod.rs)
- Raw file: `findings/raw/web-frontend-email-inbound.md` - 16 findings (1 crit / 4 major / 6 minor / 5 nit).
- Headlines: (crit) settings route authenticates purely by forgeable From header - s- token discarded, all unrouted addresses fall into settings route (inbound.rs:484, 1111, resolve at 393); (major) dedupe marker written BEFORE processing + unconditional 200 -> any post-marker failure permanently drops a player's move silently (inbound.rs:456); (major) advertised mailto unsubscribe never honored - subject ignored, unsubscribe@ falls into settings route, invalid RFC 8058 one-click header (inbound.rs:1070); (major UNCERTAIN) address matching may break on "Name <addr>" header forms; (major) From-verification adds nothing once email_tokens leak (cross-ref web-domain proposal token leak). Also: run_commands_in_order/CommandLoopOutcome/error_reply_text dead in production; processed_webhook_events never pruned.
- Lead verification: DONE. Spot-checked snapshot source:
  - VERIFIED `Some(InboundRoute::Settings(_)) | None =>` discards the token and routes to handle_settings_reply_route (inbound.rs:484-486); handle_settings_reply resolves user solely via resolve_user_by_verified_from(from) (1111-1124) and dispatches standalone commands as that user (1141-1154). Critical stands (spoofability depends on Resend-side SPF/DMARC enforcement - note as UNCERTAIN qualifier in curation, severity kept).
  - VERIFIED mark_event_processed inserted before parse/dispatch, Ok(false) -> 200, all downstream handlers return () with errors only logged (inbound.rs:456-489). Major stands.
  - VERIFIED List-Unsubscribe mailto with subject=unsubscribe + List-Unsubscribe-Post One-Click headers (inbound.rs:1068-1075). Major stands.
  - No findings rejected or downgraded at this stage.

### W2 dispatched -> returned (email/commands.rs)
- Raw file: `findings/raw/web-frontend-email-commands.md` - 12 findings (1 crit / 4 major / 4 minor / 3 nit).
- Headlines: (crit) `emails add/confirm/active` reachable via spoofable From standalone path -> account takeover chain (commands.rs:546); (major) classify_opponent accepts any `bot:<anything>` unvalidated -> wedged game on nonexistent bot (commands.rs:59, same class as web-domain bot-slot finding); (major) concede TOCTOU - db::concede_game no is_finished/updated_at guard (commands.rs:891 + db.rs:1284); (major) undo_game no OCC guard + finished-game rating never rewound (commands.rs:966 + db.rs:1407, cross-ref web-domain W2 critical); (major) restart_core errors mapped wholesale to CommandError::User - internal errors emailed verbatim, bypass Internal logging (commands.rs:1044). Concede/undo near-verbatim duplicates of server_fns 731-857 (separate simplicity major). Snapshot predates #47 end verb - no drift flagged.
- Lead verification: DONE. Spot-checked snapshot source:
  - VERIFIED run_settings_emails dispatches add/confirm/active/remove (commands.rs:546-577) on the standalone user_id.
  - VERIFIED classify_opponent `strip_prefix("bot:")` returns Bot(inner) with no validation against bot_names (commands.rs:56-67).
  - VERIFIED restart_core `.map_err(|e| CommandError::User(e.to_string()))` (commands.rs:1033-1044).
  - db.rs concede/undo guards accepted on strength of web-domain W2's verified parallel (db.rs:1407-1449 no updated_at guard, db.rs:1554 idempotency guard) - consistent with prior unit.
  - No findings rejected or downgraded at this stage.

### W3 dispatched -> returned (email/sweep.rs, email/notify.rs)
- Raw file: `findings/raw/web-frontend-email-sweep-notify.md` - 13 findings (0 crit / 2 major / 8 minor / 3 nit).
- Headlines: (major) send_reminder returns true on pref-skip/presence-suppression paths and sweep_once then marks turn_reminder_sent_at - player online at sweep time is permanently marked reminded with no email (sweep.rs:132-137, 207-210); (major) FOR UPDATE SKIP LOCKED via fetch_all(pool) autocommit - locks release at statement end, protects nothing across the send+mark window; multi-replica double-send possible (sweep.rs:58-72); (minor) candidate filter reminder_emails_enabled vs send gate turn_emails_enabled conflation; (minor) game_log_count folds DB errors to 0 -> "-0" subjects collapse de-threading (notify.rs:227); (minor) before=None defaults was_turn false -> commands.rs:426 path re-emails all on-turn players in simultaneous games (notify.rs:480). Confirmed no bot-turn reconciliation sweep exists (cross-ref web-domain wedge findings).
- Lead verification: DONE. Read sweep.rs:55-229 directly:
  - VERIFIED `return true` at :133 (should_email_recipient false) and :137 (presence suppression), and sweep_once marks sent when ok (207-210). Major stands.
  - VERIFIED FOR UPDATE SKIP LOCKED in a plain fetch_all(pool) with no transaction (58-72). Major stands.
  - VERIFIED candidate SQL filters u.reminder_emails_enabled (:67) while gate is should_email_recipient (:132); outbound.rs turn_emails_enabled claim accepted. Minor stands.
  - No findings rejected or downgraded at this stage.

### W4 dispatched -> returned (email/render.rs, email/outbound.rs, theme.rs)
- Raw file: `findings/raw/web-frontend-email-render-outbound-theme.md` - 9 findings (0 crit / 0 major / 6 minor / 3 nit).
- Headlines: ensure_email_token SELECT-then-UPDATE race can overwrite a live token (outbound.rs:110); nonexistent game_player_id returns a never-persisted token -> dead reply address (outbound.rs:116); game_emails_sent_total incremented before the Resend call so failures count as sent (outbound.rs:65); List-Unsubscribe-Post One-Click with mailto-only URI violates RFC 8058 (render.rs:235, cross-ref W1's inbound duplicate); mrml/markup parse failures silently swallowed (render.rs:181, :72). Escaping verified clean (brdgme_markup::html escapes text nodes); no header injection; the deliberate <mj-raw>/<tr><td>/font-size structure correctly not flagged.
- Lead verification: DONE (no crit/major to verify; spot-checked minors anyway):
  - VERIFIED counter!("game_emails_sent_total").increment(1) before resend.emails.send (outbound.rs:65-79).
  - VERIFIED ensure_email_token plain SELECT then UPDATE, no ON CONFLICT/row-lock, and unpersisted-token path when row is None (outbound.rs:110-125).
  - No findings rejected or downgraded at this stage.

### W5 dispatched -> returned (components/, app.rs, lib.rs)
- Raw file: `findings/raw/web-frontend-email-frontend.md` - 11 findings (0 crit / 2 major / 5 minor / 4 nit).
- Headlines: (major) GameMeta undo/concede/bump-bot/force-delete ServerActions swallow all errors - only Some(Ok(())) watched, bump_bot no watcher at all (components/game.rs:56-77); (major UNCERTAIN) Turnstile implicit rendering breaks on client-side navigation to /login - empty token hard-fails login when secret set (app.rs:595, cross-checked auth/server.rs:234-237); (minor) presence-ping / profile-theme one-shot latches never reset across logout/login (app.rs:179, 154); (minor) GamePage error branch leaks raw ServerFnError text contradicting game.rs:567 policy (app.rs:762); (minor) friend_request_count LocalResource recreated per route change (layout.rs:135). Sanctioned hydration idioms verified present and unflagged.
- Lead verification: DONE. Spot-checked snapshot source:
  - VERIFIED undo/concede/force_delete effects match only Some(Ok(())) with no error watcher in view (components/game.rs:50-77). Major stands.
  - VERIFIED implicit cf-turnstile div rendered from reactive closure (app.rs:589-598). Major stands, keeps UNCERTAIN qualifier on api.js scan timing.
  - No findings rejected or downgraded at this stage.

All 5 workers returned. Raw totals: 61 findings (2 crit / 12 major / 29 minor / 18 nit). Proceeding to curation.

## Curation complete (2026-07-24)

- Curated file: `findings/web-frontend-email.md`. Unit 11 web-frontend-email COMPLETE.
- MERGED: W1 "Unsubscribe emails never honored" (major, inbound.rs:1070) + W4 "List-Unsubscribe-Post One-Click without HTTPS URI violates RFC 8058" (minor, render.rs:235) -> single major covering both header sites (same underlying unsubscribe/RFC 8058 defect). This removed one minor.
- No findings rejected. No severity downgrades. Both criticals kept (settings-route spoofable From auth; account-takeover via email-address commands on that path) - the inbound crit carries an explicit UNCERTAIN qualifier that severity assumes Resend does not enforce SPF/DMARC upstream before the webhook fires.
- Cross-unit findings kept in-scope but framed as email entry points to web-domain db.rs root causes (concede/undo TOCTOU), with the forgeable-From/proposal-token-leak interaction and unvalidated bot-slot class noted inline rather than re-scoped.
- Deliberate structures confirmed present and NOT flagged: mrml `<mj-raw>`/`<tr><td>`/font-size (email.md), mounted-gate + nested Suspense remnants (hydration.md), custom parser combinator.
- Curated tally: 2 critical / 12 major / 28 minor / 18 nit = 60 findings.
- LOG is current; all worker dispatch/return/verify entries recorded above.
