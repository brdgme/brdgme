# W5 triage: web-domain + web-frontend-email

## Unit tallies (extracted)

- web-domain: 80 findings extracted (1c/12M/37m/30n). DISCREPANCY: unit doc's own tally says 78 (1c/12M/35m/30n) and claims "80 raw, 2 merged", but the body contains 80 finding headings (the two merged findings each appear once; nothing else was removed). Extraction counts 2 more minors than the stated tally. Rows below follow the body as authoritative.
- web-frontend-email: 63 findings extracted (2c/13M/30m/18n). DISCREPANCY: unit doc's tally says 60 (2c/12M/28m/18n); body headings count 63 (+1 major, +2 minor). Rows below follow the body.

## Rows

web-domain F1 | major | Bot command UserError acked with no re-publish; bot stays on turn, game permanently wedged | web/src/game/mod.rs | D | bot-wedge-recovery (decision: wedge recovery mechanism - republish vs reconciliation sweep)
web-domain F2 | major | MAX_TURN_ATTEMPTS exhaustion logs and acks; wedged game, no durable signal | web/src/game/mod.rs | D | bot-wedge-recovery
web-domain F3 | major | bot.turn publish failure after DB commit warn-only; bot on turn with no stream event | web/src/game/mod.rs | D | bot-wedge-recovery
web-domain F4 | major | bot.command consumer spawned once, never restarted if stream ends or errors | web/src/main.rs, web/src/game/mod.rs | M | bot-pipeline
web-domain F5 | minor | Permanently failing bot.command messages strand after max_deliver; no term/DLQ/metric | web/src/game/mod.rs, web/src/nats.rs | D | bot-wedge-recovery (decision: DLQ vs metric-only)
web-domain F6 | minor | Finished games wipe is_eliminated for previously eliminated players | web/src/game/mod.rs, web/src/db.rs | M | status-fields
web-domain F7 | minor | Export bundle includes private log bodies/hidden state despite paste-into-issues intent | web/src/game/export.rs | D | export-import (decision: export privacy posture / redact mode)
web-domain F8 | nit | before-snapshot find_game_extended errors swallowed via .ok().flatten(), no log | web/src/game/mod.rs, web/src/game/server_fns.rs | M | error-swallowing
web-domain F9 | nit | Conflict re-publish fans out to all bots on turn, not just the conflicting one | web/src/game/mod.rs | M | bot-pipeline
web-domain F10 | nit | BundlePlayer.bot_name holds game_bots.name (seat name), not bot type; naming trap | web/src/game/export.rs, web/src/game/import.rs | M | export-import
web-domain F11 | nit | Bundle timestamps exported but dropped on import | web/src/game/export.rs, web/src/game/import.rs | M | export-import
web-domain F12 | nit | placeholder_user check-then-insert races on username uniqueness | web/src/game/import.rs | M | export-import
web-domain F13 | nit | Imported players get is_turn_at/last_turn_at = NOW() regardless of turn state | web/src/game/import.rs | M | export-import
web-domain F14 | critical | undo_game allows undoing finished game; ratings never rewound, re-rating suppressed | web/src/game/server_fns.rs, web/src/db.rs | D | undo-concede-toctou (decision: undo-vs-ratings semantics)
web-domain F15 | major | undo_game has no updated_at guard; can silently clobber a concurrent move | web/src/game/server_fns.rs, web/src/db.rs | M | undo-concede-toctou
web-domain F16 | major | concede_game TOCTOU: is_finished checked on snapshot; db::concede_game unguarded | web/src/game/server_fns.rs, web/src/db.rs | M | undo-concede-toctou
web-domain F17 | major | get_game_details ignores game_visibility; is_game_visible_to_user never wired up | web/src/game/server_fns.rs, web/src/db.rs | D | visibility-gates (decision: game_visibility scope)
web-domain F18 | minor | Game-service HTTP call made inside open tx holding FOR UPDATE lock (solo restart) | web/src/game/server_fns.rs | M | restart-tx
web-domain F19 | minor | Invite-mailer path silently swallows find_proposal_players errors | web/src/game/server_fns.rs | M | error-swallowing
web-domain F20 | minor | Markup parse failures silently degrade log lines to empty, no log | web/src/game/server_fns.rs | M | error-swallowing
web-domain F21 | minor | N+1 should_hide_add_friend queries per opponent on hottest read path | web/src/game/server_fns.rs, web/src/db.rs | M | query-perf
web-domain F22 | nit | get_game_logs is_new compares created_at but list sorts/displays logged_at | web/src/game/server_fns.rs | M | misc
web-domain F23 | nit | generate_bot_name unauthenticated by omission, not decision | web/src/game/server_fns.rs | M | authz-hygiene
web-domain F24 | nit | p.user_id.unwrap() far from its guarding filter; latent panic path | web/src/game/server_fns.rs | M | misc
web-domain F25 | nit | restart_core does not itself verify caller is a player of the old game | web/src/game/server_fns.rs | M | authz-hygiene
web-domain F26 | major | get_proposal serializes every invitee's email_token to any authenticated user | web/src/proposals.rs | M | email-token-leak
web-domain F27 | major | Client-supplied bot slots stored/used unvalidated at three entry points | web/src/proposals.rs, web/src/game/server_fns.rs, web/src/db.rs | D | bot-slot-validation (decision: validation choke point)
web-domain F28 | major | Auto-decline keyed on proposal created_at, not the player's invite time | web/src/proposals.rs | M | sweep-semantics
web-domain F29 | minor | Owner can decline their own proposal and permanently wedge it | web/src/proposals.rs | M | proposal-state
web-domain F30 | minor | Ownership transferable to a declined (or pending) invitee | web/src/proposals.rs | M | proposal-state
web-domain F31 | minor | cancel_proposal notifies from roster snapshot taken before the lock | web/src/proposals.rs | M | proposal-state
web-domain F32 | minor | notify_owner_decline bypasses invite-email gates every other mailer applies | web/src/proposals.rs | M | invite-mailer
web-domain F33 | minor | Notification emails carry dead Reply-To (i-{proposal_id}) plus reply-inviting footer | web/src/proposals.rs | M | invite-mailer
web-domain F34 | minor | RealInviteMailer tasks swallow DB errors silently; blank-name subjects | web/src/proposals.rs | M | invite-mailer
web-domain F35 | minor | Pre-transaction authz block duplicated verbatim in four server fns | web/src/proposals.rs | M | proposal-dedup
web-domain F36 | minor | RespondOutcome.started/game_id always false/None; client nav path dead | web/src/proposals.rs | M | dead-code
web-domain F37 | minor | Invite emails never trimmed/case-normalized; dup accounts, invite-policy bypass | web/src/proposals.rs, web/src/db.rs | D | email-canonicalization (decision: canonicalization policy)
web-domain F38 | minor | Nudge sweep re-sends invites without re-checking proposal/player state | web/src/proposals.rs, web/src/email/sweep.rs | M | sweep-semantics
web-domain F39 | minor | cancel_proposal_for_expiry swallows follow-up query errors into no notifications | web/src/proposals.rs | M | sweep-semantics
web-domain F40 | nit | Interval built via ($1 || ' seconds')::interval string concat instead of typed bind | web/src/proposals.rs | M | misc
web-domain F41 | nit | reset_accepted_humans_for_roster_change issues one UPDATE per player | web/src/proposals.rs | M | misc
web-domain F42 | nit | count_pending_human_invitees pool variant is dead code | web/src/proposals.rs | M | dead-code
web-domain F43 | nit | Missing player_counts row degrades to garbled error message | web/src/proposals.rs | M | misc
web-domain F44 | nit | find_or_create_user_by_email_tx error labels say create_proposal; 3 fns uninstrumented | web/src/proposals.rs | M | misc
web-domain F45 | major | Stats endpoints bypass game_visibility privacy settings (3 anonymous fns) | web/src/stats/mod.rs, web/src/stats/queries.rs | D | visibility-gates (decision: game_visibility scope)
web-domain F46 | minor | Client-controlled page i64 can overflow offset computation | web/src/stats/mod.rs, web/src/players.rs | M | input-clamp
web-domain F47 | minor | Base rating 1200 hardcoded in rating_series reconstruction | web/src/stats/queries.rs | M | misc
web-domain F48 | minor | get_player_game_type_stats computes stats for every game type to use one row | web/src/stats/mod.rs, web/src/stats/queries.rs | M | query-perf
web-domain F49 | minor | finished_games/rating_series/head_to_head unbounded on public game-type page | web/src/stats/mod.rs | M | query-perf
web-domain F50 | minor | Single-human eligibility predicate duplicated across seven queries, one divergent | web/src/stats/queries.rs | M | stats-sql-dedup
web-domain F51 | minor | game_history runs four correlated subqueries per row | web/src/stats/queries.rs | M | query-perf
web-domain F52 | minor | History game_type filter exact-match while everything else is case-insensitive | web/src/stats/mod.rs, web/src/stats/queries.rs | M | misc
web-domain F53 | nit | Compile-time query! and runtime query_as mixed without cause | web/src/stats/queries.rs | M | stats-sql-dedup
web-domain F54 | nit | SVG viewBox literals duplicate chart dimension constants | web/src/stats/viz.rs | M | misc
web-domain F55 | nit | finished_at DESC ordering puts NULLs first for legacy finished games | web/src/stats/queries.rs | M | misc
web-domain F56 | minor | Concurrent cross friend requests hit friends_pair_key error instead of auto-accept | web/src/db.rs, web/src/friends.rs | M | friends
web-domain F57 | minor | Friends page mutation errors silently swallowed for five actions | web/src/friends.rs | M | fire-and-forget
web-domain F58 | minor | SetInvitePolicy success does not refetch the overview, unlike siblings | web/src/friends.rs | M | fire-and-forget
web-domain F59 | minor | Restart prefill Err silently swallowed; blank default form shown | web/src/new_game.rs | M | fire-and-forget
web-domain F60 | minor | Email slots submitted unvalidated/untrimmed from new-game form | web/src/new_game.rs | D | email-canonicalization
web-domain F61 | nit | block_user does not check target exists; FK error surfaces as generic internal | web/src/friends.rs | M | friends
web-domain F62 | nit | get_friends_overview issues six sequential queries | web/src/friends.rs | M | query-perf
web-domain F63 | nit | Submit is a silent no-op when no version is selected | web/src/new_game.rs | M | newgame-ux
web-domain F64 | nit | Create/restart outcome with both ids None navigates nowhere, no feedback | web/src/new_game.rs | M | newgame-ux
web-domain F65 | nit | Bespoke percent-encoder instead of percent-encoding crate | web/src/players.rs | M | misc
web-domain F66 | nit | Restart prefill can select a player count not offered by the game type | web/src/new_game.rs | M | newgame-ux
web-domain F67 | major | Rules-page version picked by ORDER BY name (oldest + lexicographic), not latest | web/src/game_info/queries.rs | M | rules-page
web-domain F68 | minor | Anonymous game-info page links to auth-gated rules endpoint | web/src/rules.rs, web/src/game_info/mod.rs | D | rules-page (decision: public-content posture)
web-domain F69 | minor | get_rendered_rules ignores is_public/is_deprecated; non-public versions renderable | web/src/rules.rs, web/src/db.rs | M | rules-page
web-domain F70 | minor | Unterminated brdgme fence silently dropped by render_doc | web/src/rules.rs | M | rules-page
web-domain F71 | minor | Two sequential live strategy fetches per rules view; any failure fails whole page | web/src/rules.rs | M | rules-page
web-domain F72 | minor | Email address not trimmed/normalized before add (settings path) | web/src/settings.rs, web/src/auth/server.rs | D | email-canonicalization
web-domain F73 | minor | Fire-and-forget settings mutations swallow server errors; optimistic UI never reverts | web/src/settings.rs | M | fire-and-forget
web-domain F74 | minor | Index page issues O(friends x scan_limit) sequential queries | web/src/index.rs, web/src/db.rs | M | query-perf
web-domain F75 | nit | game_info server fn runs seven sequential queries | web/src/game_info/mod.rs | M | query-perf
web-domain F76 | nit | Redundant glob re-export of ssr queries | web/src/game_info/mod.rs | M | dead-code
web-domain F77 | nit | Stale "email placeholder" module doc in settings.rs | web/src/settings.rs | M | misc
web-domain F78 | nit | GameBot lacks FromRow and timestamps unlike sibling models | web/src/models/game.rs, web/src/db.rs | M | misc
web-domain F79 | nit | Rules markdown raw HTML pass-through into inner_html; trust boundary undocumented | web/src/rules.rs | M | rules-page
web-domain F80 | nit | Mixed Resource strategies between game-info and rules pages | web/src/rules.rs, web/src/game_info/mod.rs | D | rules-page (decision: follows F68 auth-gate outcome)
web-frontend-email F1 | critical | Settings route authed solely by spoofable From; unrouted None addresses fall through | web/src/email/inbound.rs | D | email-from-auth (decision: email From-auth redesign)
web-frontend-email F2 | major | Dedupe marker inserted before processing; failures permanently dropped, always 200 | web/src/email/inbound.rs | D | webhook-delivery (decision: webhook retry/idempotency semantics)
web-frontend-email F3 | major | mailto unsubscribe can never be honored; List-Unsubscribe-Post violates RFC 8058 | web/src/email/inbound.rs, web/src/email/render.rs | D | unsubscribe-rfc8058 (decision: HTTPS one-click endpoint vs drop header)
web-frontend-email F4 | major | From/recipient matching likely breaks on "Display Name <addr>" header forms | web/src/email/inbound.rs | M | address-parsing
web-frontend-email F5 | major | From check forgeable; tokens are sole real auth; composes with proposal token leak | web/src/email/inbound.rs | D | email-from-auth
web-frontend-email F6 | minor | Quote-stripping heuristics misparse Gmail/Outlook/localized reply formats | web/src/email/inbound.rs | M | reply-parsing
web-frontend-email F7 | minor | FOR UPDATE row lock held across outbound email send in invite early-exit paths | web/src/email/inbound.rs | M | lock-scope
web-frontend-email F8 | minor | Dead code: run_commands_in_order, CommandLoopOutcome, error_reply_text | web/src/email/inbound.rs | M | dead-code
web-frontend-email F9 | minor | RESEND_API_KEY fetch + ResendInbound construction duplicated three times vs AppState | web/src/email/inbound.rs | M | email-plumbing
web-frontend-email F10 | minor | All processing inline before webhook responds; svix timeout + early marker compound | web/src/email/inbound.rs | D | webhook-delivery
web-frontend-email F11 | minor | processed_webhook_events never pruned; unbounded growth | web/src/email/inbound.rs | M | sweep-semantics
web-frontend-email F12 | nit | Silent return when player row missing from roster, no log | web/src/email/inbound.rs | M | error-swallowing
web-frontend-email F13 | nit | Invite response subject degrades to " invite" on lookup failure, unlogged | web/src/email/inbound.rs | M | error-swallowing
web-frontend-email F14 | nit | Reply-address formats hardcoded/duplicated; no domain single source of truth | web/src/email/inbound.rs, web/src/email/notify.rs, web/src/email/render.rs | M | email-plumbing
web-frontend-email F15 | nit | "accept" wins over "decline" regardless of order in the body | web/src/email/inbound.rs | M | reply-parsing
web-frontend-email F16 | nit | verify_webhook unwraps HeaderValue::from_str; pub fn can panic on bad input | web/src/email/inbound.rs | M | misc
web-frontend-email F17 | critical | emails add/confirm/active/remove reachable via spoofable From path = account takeover | web/src/email/commands.rs | D | email-from-auth
web-frontend-email F18 | major | bot:<name> opponents not validated against enabled bots; wedged game creatable | web/src/email/commands.rs | D | bot-slot-validation (decision: validation choke point)
web-frontend-email F19 | major | Concede TOCTOU (email path) can overwrite finished game's results | web/src/email/commands.rs, web/src/db.rs | M | undo-concede-toctou
web-frontend-email F20 | major | Undo TOCTOU + finished-game undo permitted (email path); ratings never rewound | web/src/email/commands.rs, web/src/db.rs | D | undo-concede-toctou (decision: undo-vs-ratings semantics)
web-frontend-email F21 | major | run_restart maps internal errors to User; emailed verbatim and unlogged | web/src/email/commands.rs | M | error-classification
web-frontend-email F22 | major | run_concede/run_undo duplicate web server fns near-verbatim; races drift-prone | web/src/email/commands.rs, web/src/game/server_fns.rs | M | undo-concede-toctou
web-frontend-email F23 | minor | emails confirm only matches the most recently added unverified address | web/src/email/commands.rs | M | email-mgmt
web-frontend-email F24 | minor | validate_confirmation_code DB errors masked as "invalid code", never logged | web/src/email/commands.rs | M | error-classification
web-frontend-email F25 | minor | Standalone path rejects subscribe/unsubscribe that help_text advertises | web/src/email/commands.rs | D | unsubscribe-rfc8058
web-frontend-email F26 | minor | Inline SQL in commands.rs instead of db helpers; drift risk with settings path | web/src/email/commands.rs, web/src/db.rs | M | email-plumbing
web-frontend-email F27 | nit | Self-mention in `new` opponents silently dropped | web/src/email/commands.rs | M | misc
web-frontend-email F28 | nit | bump reply does not mention the digest cap | web/src/email/commands.rs | M | misc
web-frontend-email F29 | nit | Game-scoped dispatch reserves verbs that could collide with game move grammars | web/src/email/commands.rs | M | misc
web-frontend-email F30 | major | Suppressed turn reminder permanently marked as sent; never retried once idle | web/src/email/sweep.rs | D | sweep-semantics (decision: sweep delivery semantics)
web-frontend-email F31 | major | FOR UPDATE SKIP LOCKED is a no-op (autocommit); replicas can double-send | web/src/email/sweep.rs | D | sweep-semantics (decision: claim mechanism)
web-frontend-email F32 | minor | Reminder gate checks turn_emails_enabled, not reminder_emails_enabled | web/src/email/sweep.rs, web/src/email/outbound.rs | D | sweep-semantics (decision: which pref governs)
web-frontend-email F33 | minor | Invite nudge marked regardless of fire-and-forget send outcome; no retry | web/src/email/sweep.rs | D | sweep-semantics
web-frontend-email F34 | minor | Auto-decline does not notify the proposal owner, unlike manual decline | web/src/email/sweep.rs, web/src/proposals.rs | M | sweep-semantics
web-frontend-email F35 | minor | Expiry cancellation loses notifications if owner lookup fails after status update | web/src/email/sweep.rs, web/src/proposals.rs | M | sweep-semantics
web-frontend-email F36 | minor | send_reminder duplicates ~90 lines of notify::send_one; gating already drifted | web/src/email/sweep.rs, web/src/email/notify.rs | M | notify-dedup
web-frontend-email F37 | minor | Rust candidate predicate is prod-dead and drifting from the SQL | web/src/email/sweep.rs | M | dead-code
web-frontend-email F38 | nit | Five copy-pasted spawn/interval loops | web/src/email/sweep.rs | M | notify-dedup
web-frontend-email F39 | nit | Reminder threads into the game thread its turn email deliberately opted out of | web/src/email/sweep.rs, web/src/email/notify.rs | M | email-threading
web-frontend-email F40 | nit | Sweep candidate queries unbounded; post-downtime burst | web/src/email/sweep.rs | M | sweep-semantics
web-frontend-email F41 | minor | game_log_count error collapses to 0; threading and first-message flag corrupted | web/src/email/notify.rs | M | email-threading
web-frontend-email F42 | minor | notify_game_emails with before=None re-notifies every player already on turn | web/src/email/notify.rs, web/src/email/commands.rs | M | before-none-diffing
web-frontend-email F43 | minor | Per-recipient game reload and serial sends in notify_game_emails (N+1) | web/src/email/notify.rs | M | query-perf
web-frontend-email F44 | minor | ensure_email_token SELECT-then-UPDATE race can invalidate an emailed reply address | web/src/email/outbound.rs | M | email-token-race
web-frontend-email F45 | minor | ensure_email_token returns unpersisted token for nonexistent game_player_id | web/src/email/outbound.rs | M | email-token-race
web-frontend-email F46 | minor | game_emails_sent_total incremented before send; failures counted as sent | web/src/email/outbound.rs | M | metrics
web-frontend-email F47 | minor | mrml parse/render failure silently falls back with no log/metric | web/src/email/render.rs | M | error-swallowing
web-frontend-email F48 | minor | render_block silently renders malformed markup as empty block | web/src/email/render.rs | M | error-swallowing
web-frontend-email F49 | nit | URLs interpolated into href attributes without escaping | web/src/email/render.rs | M | misc
web-frontend-email F50 | nit | parse_duration lives in outbound.rs but is sweep configuration parsing | web/src/email/outbound.rs, web/src/email/sweep.rs | M | misc
web-frontend-email F51 | nit | random_pref_colors hand-rolls Fisher-Yates instead of SliceRandom | web/src/theme.rs | M | misc
web-frontend-email F52 | major | GameMeta mutation actions swallow errors (concede/force-delete are destructive) | web/src/components/game.rs | M | fire-and-forget
web-frontend-email F53 | major | Turnstile widget likely never renders after client-side navigation to /login | web/src/app.rs | M | turnstile
web-frontend-email F54 | minor | Presence-ping and profile-theme one-shot latches never reset after logout/login | web/src/app.rs | M | spa-latches
web-frontend-email F55 | minor | GamePage error branch leaks raw ServerFnError text to the user | web/src/app.rs | M | frontend-errors
web-frontend-email F56 | minor | GameMeta inlines confirm dialogs instead of shared confirm() helper | web/src/components/game.rs | M | misc
web-frontend-email F57 | minor | friend_request_count resource recreated per navigation; badge flashes, never live | web/src/components/layout.rs | M | spa-latches
web-frontend-email F58 | minor | Bot difficulty select can desync from state when bot_names resolves after render | web/src/components/opponent_slot.rs | M | misc
web-frontend-email F59 | minor | Logout action failure gives no feedback | web/src/components/layout.rs | M | fire-and-forget
web-frontend-email F60 | nit | format_log_time hardcodes en-US locale despite browser-local intent | web/src/components/game.rs | M | misc
web-frontend-email F61 | nit | Click-only anchors without href are keyboard-inaccessible (3 sites) | web/src/components/layout.rs, web/src/app.rs | M | a11y
web-frontend-email F62 | nit | components/mod.rs placeholder comment is stale | web/src/components/mod.rs | M | misc
web-frontend-email F63 | nit | sentry snippet escaping does not cover </script> | web/src/app.rs | M | misc

## Grouping notes

- undo-concede-toctou is the strongest cross-unit package: web-domain F14/F15/F16 (server fns) and web-frontend-email F19/F20/F22 (email commands) share the same db.rs root causes (db::undo_game and db::concede_game lack updated_at/is_finished guards; rating rewind semantics). Fixing in db.rs once covers both entry points; F22's concede_core/undo_core extraction is the natural vehicle. One design decision gates the package: undo-vs-ratings semantics (reject undo on finished vs rewind ratings).
- email-from-auth is one decision spanning both criticals in web-frontend-email (F1 settings route, F17 account takeover) plus F5; it composes with web-domain F26 (email_token leak makes invite replies forgeable). F26 itself is a mechanical one-line-ish fix (drop the field) and should land immediately regardless of the From-auth redesign; token rotation follows the redesign.
- bot-slot-validation: web-domain F27 (three web entry points) and web-frontend-email F18 (email `bot:` path) are the same defect class; decision is where the single validation choke point lives (per-entry vs shared, e.g. start_proposal_tx / create_game_from_service). Consequence severity is amplified by the bot-wedge-recovery gap: an invalid bot wedges a game with no recovery.
- bot-wedge-recovery: web-domain F1/F2/F3/F5 all resolve under one design (reconciliation sweep "bot on turn > N minutes -> republish" vs per-failure republish + term/DLQ). F4/F9 (bot-pipeline) are mechanical and independent.
- sweep-semantics is a large shared package: web-frontend-email F30-F35/F40 plus web-domain F28/F38/F39 and wf F11 (pruning). Core decision: delivery semantics of every sweep (claim-atomically vs mark-after-send vs fire-and-forget), plus which preference governs reminders (F32). web-domain F28 (created_at keying) is mechanical but should ship with the same package since sweeps interlock (auto-decline vs nudge vs expiry).
- unsubscribe-rfc8058: wf F3 (headers, two sites) + wf F25 (standalone dispatch) are one deliverable; decision is HTTPS one-click endpoint vs mailto-only + drop the Post header.
- email-canonicalization: web-domain F37/F60/F72 are three instances of one policy decision (trim+lowercase at server boundaries, storage normalization); fix once at a shared boundary, then the client-side trims.
- fire-and-forget / error-swallowing runs through both units: web-domain F8/F19/F20/F34/F57/F58/F59/F73 and wf F12/F13/F47/F48/F52/F59. Two sub-shapes: (a) frontend ServerAction values never observed (mechanical: shared error-slot pattern already exists in GameCommandInput/UsernameSection); (b) backend .ok()/unwrap_or_default silent drops (mechanical: add tracing::warn). Good candidates for two sweep-style mechanical work packages.
- visibility-gates: web-domain F17 + F45 (and tangentially F69 rules is_public) hinge on one decision: intended scope of game_visibility (which endpoints enforce it, anonymize vs filter). Same predicate (is_game_visible_to_user) should back both.
- rules-page: web-domain F67-F71/F79/F80 form a coherent package; F68's public-posture decision determines F80's fix and interacts with F69.
- webhook-delivery (wf F2/F10) is one redesign: verify + dedupe + enqueue, marker after success, 5xx on transient failure. F11 (pruning) rides along.
- email-token-race (wf F44/F45): both fixed by one atomic UPDATE ... COALESCE ... RETURNING rewrite.
- query-perf items (web-domain F21/F48/F49/F51/F62/F74/F75, wf F43) are independent mechanical batching/join fixes; can be one low-risk perf package.
- email-plumbing (wf F9/F14/F26, and web-domain F33's address helpers) is a small consolidation package: AppState-held inbound client, reply-address helpers with one domain constant, db helper extraction.
- before-none-diffing (wf F42) touches the same notify_game_emails call sites as web-domain F8 (before-snapshot swallow); fix together when making `before` non-optional.
- Tally discrepancies: both findings docs' own severity tallies undercount their body headings (web-domain 78 stated vs 80 present; web-frontend-email 60 stated vs 63 present). Downstream totals should use the row counts here or re-verify with the Lead.

## ID audit

F-numbers are W5-assigned, sequential in order of appearance in each findings doc (the docs themselves do not number findings). By construction there are no gaps or duplicates; verified below.

### web-domain (80 IDs, F1-F80)

F1M F2M F3M F4M F5m F6m F7m F8n F9n F10n F11n F12n F13n F14c F15M F16M F17M F18m F19m F20m F21m F22n F23n F24n F25n F26M F27M F28M F29m F30m F31m F32m F33m F34m F35m F36m F37m F38m F39m F40n F41n F42n F43n F44n F45M F46m F47m F48m F49m F50m F51m F52m F53n F54n F55n F56m F57m F58m F59m F60m F61n F62n F63n F64n F65n F66n F67M F68m F69m F70m F71m F72m F73m F74m F75n F76n F77n F78n F79n F80n

- Gaps: none. Duplicates: none. Highest: F80. Count: 80 (1c/12M/37m/30n).
- Doc's own tally (findings/web-domain.md, "## Severity tally" table):

  > | critical | 1 |
  > | major | 12 |
  > | minor | 35 |
  > | total | 78 |

  (table also lists `| nit | 30 |`), followed by:

  > Raw findings: 80 across W1-W6; 2 merged during curation (unvalidated bot
  > slots: W2 minor + W3 major -> one major; page-number overflow: W4 minor +
  > W5 nit -> one minor). No findings rejected; the bot-slot merge effectively
  > upgraded W2's restart_core instance from minor to major.

  Body headings counted per section: 13 + 12 + 19 + 11 + 11 + 14 = 80. The merged findings each appear once (F27 bot slots, F46 page overflow), so the merge does not reconcile 80 body headings to the stated 78; the stated minor count (35) is 2 short of the 37 present.

### web-frontend-email (63 IDs, F1-F63)

F1c F2M F3M F4M F5M F6m F7m F8m F9m F10m F11m F12n F13n F14n F15n F16n F17c F18M F19M F20M F21M F22M F23m F24m F25m F26m F27n F28n F29n F30M F31M F32m F33m F34m F35m F36m F37m F38n F39n F40n F41m F42m F43m F44m F45m F46m F47m F48m F49n F50n F51n F52M F53M F54m F55m F56m F57m F58m F59m F60n F61n F62n F63n

- Gaps: none. Duplicates: none. Highest: F63. Count: 63 (2c/13M/30m/18n).
- Doc's own tally (findings/web-frontend-email.md, "## Severity tally"):

  > - critical: 2
  > - major: 12
  > - minor: 28
  > - nit: 18
  > - total: 60

  Body headings counted per section: inbound 16 + commands 13 + sweep 11 + notify 3 + render/outbound 7 + theme 1 + components/app 12 = 63. The one merge the doc claims (RFC 8058 / unsubscribe header) appears once in the body (F3), so it does not reconcile 63 to 60; stated tally is short 1 major and 2 minor.
